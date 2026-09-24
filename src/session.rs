//! The session: one running arranger and the only way clients reach it.
//!
//! A [`Session`] owns the runtime: MIDI in and out, the engine thread, the built-in synth,
//! the Launchkey (pads, buttons, faders and LEDs) and the style library. Clients send
//! [`AppCmd`]s and read [`AppState`] snapshots; they never touch the engine, the
//! real-time atomics or the synth controls. docs/app-api.md is the contract.
//!
//! Threads (live):
//!
//!   CoreMIDI thread  keys, Launchkey pads/buttons/faders (`live::Input`)
//!   MIDI run loop    CoreMIDI's setup-change notifications (`midi::init`; devices.rs)
//!   engine thread    real-time playback (`live::run_engine`)
//!   control thread   runs Launchkey actions as `AppCmd`s, OTS Link, Launchkey LEDs,
//!                    library indexing, and republishes `AppState` when it changes
//!   audio thread     the synth (cpal)
//!   client threads   `Session::send` runs the command right there, under the control lock
//!
//! Only the control thread and client threads take locks. The CoreMIDI and engine threads
//! talk to the control side through SPSC rings, atomics and a semaphore, as before.
//!
//! An offline session ([`Session::offline`]) has no MIDI, audio or threads: the engine
//! runs on a virtual clock that [`Session::advance`] moves, and [`Session::midi_in`] plays
//! the part of the keyboard and the Launchkey. Tests and the app's dev mode use it.
//!
//! Layout: this file is the frame (the `Session` API, the control side's state, command
//! dispatch, the pump, publishing). Each feature is a module of `impl Control` blocks
//! with its command handler (`transport_cmd`, ...), its part of the state
//! (`transport_state`, ...) and its pump step if it has one; `apply`, `pump` and
//! `build_state` below call them in a fixed order (docs/architecture.md).

mod chord;
mod controllers;
mod devices;
mod keyboard;
mod leds;
mod library;
mod looper;
mod metronome;
mod mixer;
mod multipad;
mod offline;
mod ots;
mod pads;
mod parts;
mod preview;
mod settings;
mod surface;
mod system;
mod transport;

pub use library::library_entry;
pub use preview::AUDITION_CHORDS;
pub use settings::choose_keys;

use crate::api::*;
use crate::engine::{Engine, Prepared, Snapshot, Transpose};
use crate::fingering::Fingering;
use crate::launchkey::{Action, Page, Panel};
use crate::library::{Info, Library};
use crate::live::{self, Audition, Cmd, Input, Shared, MAX_KEY_SOURCES};
use crate::midi::{self, Client};
use crate::rt::{self, PacketSink, Target};
use crate::synth;
use crate::theory::Recognizer;
use anyhow::Result;
use leds::Leds;
use library::{open_library, Loaded};
use offline::Offline;
use rtrb::{Consumer, Producer, RingBuffer};
use settings::{is_daw, start_synth, MidiIo, RackLoad, SynthRef};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::{mpsc, Arc, Mutex, MutexGuard};

/// How a session starts.
#[derive(Clone, Debug)]
pub struct Options {
    /// Style files and folders (folders are scanned recursively) for the library.
    pub paths: Vec<PathBuf>,
    /// Split point, a MIDI note.
    pub split: u8,
    /// Connect every MIDI source as a keyboard (else a Launchkey's keys when there is one).
    pub all_inputs: bool,
    /// Only connect sources whose name contains one of these.
    pub inputs: Vec<String>,
    /// Leave the Launchkey DAW port alone (no pads, buttons, faders or LEDs).
    pub no_pads: bool,
    /// SoundFont for the built-in synth; None = no synth.
    pub sf2: Option<PathBuf>,
    /// Use Novation palette colours (and hardware flashing) instead of RGB SysEx.
    pub palette_leds: bool,
    /// 1-based left output channel for the synth (None = auto).
    pub audio_out: Option<u8>,
    /// Chord fingering type at startup.
    pub fingering: Fingering,
    /// Chord Detection Area = Upper.
    pub upper: bool,
    /// The Manual Bass setting (takes effect in Upper mode only).
    pub manual_bass: bool,
    /// Initial Keyboard / Master transpose.
    pub transpose: Transpose,
    /// The chord-settle window, in ms (`ChordCmd::SetChordSettle`).
    pub chord_settle_ms: u32,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            paths: Vec::new(),
            split: 54, // F#2 in Yamaha octave numbering (C3 = 60), the Genos default
            all_inputs: false,
            inputs: Vec::new(),
            no_pads: false,
            sf2: None,
            palette_leds: false,
            audio_out: None,
            fingering: Fingering::FingeredOnBass,
            upper: false,
            manual_bass: true,
            transpose: Transpose::default(),
            chord_settle_ms: crate::engine::CHORD_SETTLE_DEFAULT_MS,
        }
    }
}

/// Which input an offline `midi_in` message arrives on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Port {
    /// A keyboard.
    Keys,
    /// The Launchkey DAW port (pads, buttons, faders).
    Pads,
}

/// A running arranger. `Send + Sync`: share it (an `Arc`, or Tauri managed state) between
/// the threads that send commands and read state. Stops when dropped.
pub struct Session {
    inner: Arc<Inner>,
    live: Mutex<Option<Live>>,
}

struct Inner {
    shared: Arc<Shared>,
    ctl: Mutex<Control>,
    state: Mutex<Arc<AppState>>,
    /// The published library and its revision, swapped together so a `LibraryList` never
    /// carries a revision its entries are not.
    library: Mutex<(Arc<Library>, u64)>,
    version: AtomicU64,
    subscribers: Mutex<Vec<mpsc::Sender<Event>>>,
    stop: AtomicBool,
}

/// What only a live session has.
struct Live {
    client: Client,
    /// Its handler runs on CoreMIDI's thread until the client is disposed.
    _port: midi::InputPort,
    engine: std::thread::JoinHandle<()>,
    control: std::thread::JoinHandle<()>,
    /// Dropping it stops the synth thread (and its audio stream).
    synth: Option<SynthThread>,
}

struct SynthThread {
    stop: mpsc::Sender<()>,
    thread: std::thread::JoinHandle<()>,
}

/// The control side: everything a command can change that isn't real-time, and the
/// producer ends of the rings into the engine.
struct Control {
    shared: Arc<Shared>,
    lib: Library,
    index_rx: Option<mpsc::Receiver<(usize, Info)>>,
    lib_rev: u64,
    lib_published: u64,
    lib_published_ns: u64,
    /// The library as last published (`Session::library`): the state's library status
    /// describes this one, so it always matches what a client fetches.
    published: Arc<Library>,
    /// A change a client made (a file added, a style that failed to load): publish it at
    /// once rather than on the indexing cadence.
    lib_urgent: bool,
    cur: usize,
    info: Loaded,
    snap: Snapshot,
    ui_tx: Producer<Cmd>,
    style_tx: Producer<Box<Prepared>>,
    old_rx: Consumer<Box<Prepared>>,
    snap_rx: Consumer<Snapshot>,
    act_rx: Consumer<Action>,
    transpose: Transpose,
    /// The chord-settle window, in ms (session/chord.rs).
    chord_settle_ms: u32,
    message: Option<Message>,
    msg_seq: u64,
    last_ots_key: Option<(usize, u8)>,
    last_link: bool,
    leds: Option<Leds>,
    synth: Option<SynthRef>,
    inputs: Vec<String>,
    pads_connected: bool,
    /// The free-running beat clock the pad flashing follows: `led_beats` at `led_ns`,
    /// moving on at `led_bpm` (re-anchored when the tempo changes).
    led_beats: f64,
    led_ns: u64,
    led_bpm: f64,
    /// The time of the last pump.
    clock_ns: u64,
    /// The pads use palette colours (`Options::palette_leds`).
    palette_leds: bool,
    offline: Option<Offline>,
    /// Style previews to the engine thread, and finished ones back to free here.
    audition_tx: Producer<Box<Audition>>,
    old_audition_rx: Consumer<Box<Audition>>,
    /// The last `Prepared::tag` handed out.
    style_seq: u64,
    /// A style handed to the engine that it hasn't switched to yet: (id, tag, what it is).
    /// It becomes `cur`/`info` when a snapshot shows the engine playing it.
    pending_style: Option<(usize, u64, Loaded)>,
    /// The style folders and files the library scans.
    roots: Vec<PathBuf>,
    /// A rescan running (`RescanLibrary`).
    scan_rx: Option<mpsc::Receiver<Library>>,
    /// The SoundFont folder, the file the synth plays, and the `.sf2` files there.
    sf_dir: Option<PathBuf>,
    sf_file: Option<String>,
    sound_fonts: Vec<String>,
    /// A SoundFont loading (`SetSoundFont`): its file and the loader's result.
    sf_load: Option<(String, mpsc::Receiver<RackLoad>)>,
    /// A loaded rack waiting for room in the swap ring.
    sf_ready: Option<(String, Box<synth::Rack>)>,
    /// Keyboard sources: every one, or those named (`SetMidiInputs`).
    all_inputs: bool,
    input_names: Vec<String>,
    /// Every MIDI source, as last listed.
    sources: Vec<MidiSource>,
    /// Live MIDI input (None offline).
    midi: Option<MidiIo>,
    /// Source slots disconnected, for the input thread to release their keys.
    release_tx: Producer<u8>,
    /// When the sources were last listed (live: every 2 s, for hot-plugged keyboards).
    sources_ns: u64,
    /// Chord Looper memories and the rings to the engine's looper.
    looper: looper::LooperCtl,
    /// Metronome settings.
    metronome: metronome::MetronomeCtl,
    /// Multi Pad banks to the engine thread, and replaced players back to free here.
    pad_tx: Producer<live::PadBank>,
    old_pad_rx: Consumer<Box<crate::multipad::MultiPadPlayer>>,
    /// Multi Pads: the bank list and the bank loaded.
    multipad: multipad::Pads,
}

/// What several parts of the state read, read once per `build_state` so they all agree.
struct View {
    pnl: Panel,
    manual_bass_active: bool,
    /// Where each Launchkey fader 1-8 physically is.
    fader_hw: [Option<u8>; 8],
    upper: bool,
    fingering: Fingering,
    split: u8,
}

impl Control {
    fn wake_engine(&self) {
        self.shared.wake.signal();
    }

    /// A command for the engine thread. Busy if its ring is full.
    fn engine_cmd(&mut self, c: Cmd) -> Result<(), CmdError> {
        self.ui_tx.push(c).map_err(|_| CmdError::Busy)?;
        self.wake_engine();
        Ok(())
    }

    fn say(&mut self, text: impl Into<String>, error: bool) {
        self.msg_seq += 1;
        self.message = Some(Message { seq: self.msg_seq, text: text.into(), error });
    }

    fn fail(&mut self, text: impl Into<String>) -> Result<(), CmdError> {
        let t = text.into();
        self.say(t.clone(), true);
        Err(CmdError::Failed(t))
    }

    /// Run a command: each group goes to its feature's handler.
    fn apply(&mut self, cmd: AppCmd) -> Result<(), CmdError> {
        match cmd {
            AppCmd::Transport(c) => self.transport_cmd(c),
            AppCmd::Mixer(c) => self.mixer_cmd(c),
            AppCmd::Chord(c) => self.chord_cmd(c),
            AppCmd::Parts(c) => self.parts_cmd(c),
            AppCmd::Pads(c) => self.pads_cmd(c),
            AppCmd::Ots(c) => self.ots_cmd(c),
            AppCmd::Library(c) => self.library_cmd(c),
            AppCmd::Preview(c) => self.preview_cmd(c),
            AppCmd::Settings(c) => self.settings_cmd(c),
            AppCmd::System(c) => self.system_cmd(c),
            AppCmd::Looper(c) => self.looper_cmd(c),
            AppCmd::Metronome(c) => self.metronome_cmd(c),
            AppCmd::MultiPad(c) => self.multipad_cmd(c),
            AppCmd::Controllers(c) => self.controllers_cmd(c),
        }
    }

    fn panel(&self) -> Panel {
        let parts = &self.shared.parts;
        Panel {
            page: Page::from_u8(self.shared.page.load(Relaxed)),
            fingering: Fingering::from_u8(self.shared.fingering.load(Relaxed)),
            upper: self.shared.upper.load(Relaxed),
            manual_bass: self.shared.manual_bass.load(Relaxed),
            ots_count: self.info.ots.len().min(4) as u8,
            ots_applied: parts.ots_applied.load(Relaxed),
            ots_link: parts.ots_link.load(Relaxed),
            parts_on: parts.sounding_mask(),
            selected: parts.selected() as u8,
        }
    }

    /// The pad flash clock at `t`.
    fn led_beats_at(&self, t: u64) -> f64 {
        self.led_beats + (t as f64 - self.led_ns as f64) / 1e9 * self.led_bpm / 60.0
    }

    fn drain_snapshots(&mut self) {
        while let Ok(s) = self.snap_rx.pop() {
            self.snap = s;
        }
        self.promote_style();
    }

    /// Everything that happens between commands: new snapshots, Launchkey actions, OTS
    /// Link, the LEDs, indexing. `now` is the clock the pad flashing follows. The steps
    /// run in this order; a feature that follows the engine (a snapshot) or a background
    /// job adds its `pump_*` step here.
    fn pump(&mut self, now: u64) {
        self.drain_snapshots();
        // Launchkey pad/button actions: the same commands as their keyboard shortcuts.
        while let Ok(a) = self.act_rx.pop() {
            let _ = self.apply(a.into());
        }
        self.pump_ots_link();
        while self.old_rx.pop().is_ok() {} // drop old styles here, off the RT thread
        while self.old_audition_rx.pop().is_ok() {}
        self.pump_sound_font();
        self.pump_rescan();
        self.pump_devices(now);
        self.pump_looper();
        self.pump_metronome();

        // Free-running beat clock for flashing/pulsing, following the current tempo.
        let s = self.snap;
        if s.bpm != self.led_bpm {
            self.led_beats = self.led_beats_at(now);
            self.led_ns = now;
            self.led_bpm = s.bpm;
        }
        self.clock_ns = now;
        let pnl = self.panel();
        let beats = self.led_beats_at(now);
        if let Some(leds) = self.leds.as_mut() {
            let styles = self.lib.count() > 1;
            leds.update(&s, &self.info.has, &pnl, self.shared.manual_bass(), self.shared.parts.fader_page(), styles, beats);
        }
        self.pump_index();
        self.pump_multipad();
    }

    /// The state: each feature builds its part, in `AppState`'s order.
    fn build_state(&self, now: u64) -> AppState {
        let shared = &self.shared;
        let pnl = self.panel();
        let manual_bass_active = shared.manual_bass();
        let v = View {
            pnl,
            manual_bass_active,
            fader_hw: shared.parts.fader_hw.each_ref().map(|a| surface::known(a.load(Relaxed))),
            upper: shared.upper.load(Relaxed),
            fingering: Fingering::from_u8(shared.fingering.load(Relaxed)),
            split: shared.split.load(Relaxed),
        };
        AppState {
            version: 0,
            style: self.style_state(),
            transport: self.transport_state(&v),
            chord: self.chord_state(&v),
            keyboard_parts: self.keyboard_parts_state(&v),
            mixer: self.mixer_state(&v),
            pads: self.pads_state(&v),
            ots: self.ots_state(),
            library: self.library_status(),
            surface: self.surface(&v.pnl, v.manual_bass_active, now),
            io: self.io_state(),
            preview: self.preview_state(),
            keyboard: self.keyboard_state(&v),
            multi_pad: self.multipad_state(),
            controllers: self.controllers_state(),
            message: self.message.clone(),
            looper: self.looper_state(),
            metronome: self.metronome_state(),
        }
    }
}

impl Inner {
    fn lock(&self) -> MutexGuard<'_, Control> {
        self.ctl.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Rebuild the state from `ctl`; if anything changed, publish it with a new version
    /// and tell the subscribers. Also republishes the library when it changed (at most
    /// every 250 ms while indexing).
    fn publish(&self, ctl: &mut Control, now: u64) {
        let mut events = Vec::new();
        // The library first, so the state's `library.revision` is always the revision
        // `library()` returns.
        if ctl.lib_rev != ctl.lib_published
            && (ctl.lib_urgent || ctl.lib.pending() == 0 || now.saturating_sub(ctl.lib_published_ns) >= 250_000_000)
        {
            ctl.published = Arc::new(ctl.lib.clone());
            *self.library.lock().unwrap_or_else(|e| e.into_inner()) = (ctl.published.clone(), ctl.lib_rev);
            ctl.lib_urgent = false;
            ctl.lib_published = ctl.lib_rev;
            ctl.lib_published_ns = now;
            events.push(Event::LibraryChanged { revision: ctl.lib_rev });
        }
        let mut st = ctl.build_state(now);
        {
            let mut cur = self.state.lock().unwrap_or_else(|e| e.into_inner());
            st.version = cur.version;
            // Time passing alone is not a change: the clock is read at `nowNs`, and moves
            // on from its anchors.
            let fresh = st.surface.clock.clone();
            let old = &cur.surface.clock;
            st.surface.clock = ClockState { at_ms: old.at_ms, bar: old.bar, beat: old.beat, phase: old.phase, ..fresh.clone() };
            if **cur != st {
                st.surface.clock = fresh;
                st.version = self.version.fetch_add(1, Relaxed) + 1;
                events.push(Event::StateChanged { version: st.version });
                *cur = Arc::new(st);
            }
        }
        if !events.is_empty() {
            self.notify(&events);
        }
    }

    fn notify(&self, events: &[Event]) {
        let mut subs = self.subscribers.lock().unwrap_or_else(|e| e.into_inner());
        subs.retain(|tx| events.iter().all(|e| tx.send(*e).is_ok()));
    }

    /// The control thread: wake on Launchkey actions and new snapshots, or every 10 ms
    /// for the pad animation.
    fn control_loop(&self) {
        while !self.stop.load(Relaxed) {
            self.shared.ctl_wake.wait(10_000_000);
            if self.stop.load(Relaxed) {
                break;
            }
            let now = rt::now_ns();
            let mut ctl = self.lock();
            ctl.pump(now);
            self.publish(&mut ctl, now);
        }
    }
}

struct Assembled {
    control: Control,
    engine: EngineLoopParts,
    input: Input,
}

/// The engine and its rings, before a thread (or the offline clock) runs them.
struct EngineLoopParts {
    engine: Engine,
    io: live::EngineIo,
}

/// Build the shared state, the rings, the input handler and the control side.
fn assemble(opts: &Options, engine_out: live::Out, input_out: live::Out, offline: bool) -> Result<(Arc<Shared>, Assembled)> {
    let (lib, index_rx, cur, prep, info) = open_library(&opts.paths)?;
    let shared = Arc::new(Shared::new(opts.split));
    shared.fingering.store(opts.fingering.to_u8(), Relaxed);
    shared.upper.store(opts.upper, Relaxed);
    shared.manual_bass.store(opts.manual_bass, Relaxed);

    let ch = live::channels(engine_out);
    let mut input = Input::new(shared.clone(), Recognizer::new(), ch.input_tx, input_out);
    // Launchkey pads and buttons that run on the control side, as `AppCmd`s.
    let (act_tx, act_rx) = RingBuffer::<Action>::new(64);
    input.set_actions(act_tx);
    let (release_tx, release_rx) = RingBuffer::<u8>::new(MAX_KEY_SOURCES);
    input.set_release(release_rx);
    let mut engine = Engine::new(prep);
    let chord_settle_ms = opts.chord_settle_ms.min(crate::engine::CHORD_SETTLE_MAX_MS);
    engine.set_chord_settle(chord_settle_ms as u64 * 1_000_000);
    let snap = engine.snapshot(0);
    let published = Arc::new(lib.clone());
    let control = Control {
        shared: shared.clone(),
        lib,
        index_rx: Some(index_rx),
        lib_rev: 1,
        lib_published: 0,
        lib_published_ns: 0,
        published,
        lib_urgent: false,
        cur,
        info,
        snap,
        ui_tx: ch.ui_tx,
        style_tx: ch.style_tx,
        old_rx: ch.old_rx,
        snap_rx: ch.snap_rx,
        act_rx,
        transpose: Transpose::default(),
        chord_settle_ms,
        message: None,
        msg_seq: 0,
        last_ots_key: None,
        last_link: false,
        leds: None,
        synth: None,
        inputs: Vec::new(),
        pads_connected: false,
        led_beats: 0.0,
        led_ns: if offline { 0 } else { rt::now_ns() },
        led_bpm: snap.bpm,
        clock_ns: if offline { 0 } else { rt::now_ns() },
        palette_leds: opts.palette_leds,
        offline: None,
        audition_tx: ch.audition_tx,
        old_audition_rx: ch.old_audition_rx,
        style_seq: 0,
        pending_style: None,
        roots: opts.paths.clone(),
        scan_rx: None,
        sf_dir: opts.sf2.as_ref().and_then(|p| p.parent()).map(|d| if d.as_os_str().is_empty() { Path::new(".") } else { d }.to_path_buf()),
        sf_file: opts.sf2.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()),
        sound_fonts: Vec::new(),
        sf_load: None,
        sf_ready: None,
        all_inputs: opts.all_inputs,
        input_names: opts.inputs.clone(),
        sources: Vec::new(),
        midi: None,
        release_tx,
        sources_ns: 0,
        looper: looper::LooperCtl::new(ch.looper_tx, ch.recorded_rx),
        metronome: Default::default(),
        pad_tx: ch.pad_tx,
        old_pad_rx: ch.old_pad_rx,
        multipad: multipad::Pads::scan(&opts.paths),
    };
    let mut control = control;
    control.list_sound_fonts();
    Ok((shared, Assembled { control, engine: EngineLoopParts { engine, io: ch.io }, input }))
}

impl Session {
    /// Start a live session: open MIDI, start the synth (if `opts.sf2`), connect the
    /// keyboards and the Launchkey, start the engine and control threads.
    pub fn start(opts: Options) -> Result<Session> {
        // (`midi::init` makes the process's first CoreMIDI call on a run-loop thread of
        // its own, so the session hears of devices coming and going: session/devices.rs.)
        let client = Client::new("yahaha")?;
        let out_src = client.virtual_source("yahaha")?;

        // The synth reads the keyboard parts, so they exist before it starts.
        let mut feeds = synth::feeds();
        let (shared, mut p) = {
            // Rings to the synth are only used when it runs; built here so the Outs have them.
            let (engine_feed, input_feed) = (feeds.engine.take(), feeds.input.take());
            let (shared, mut p) = assemble(
                &opts,
                live::Out::new(PacketSink::new(Target::Virtual(out_src)), None),
                live::Out::new(PacketSink::new(Target::Virtual(out_src)), None),
                false,
            )?;
            p.engine.io.out.synth = engine_feed;
            p.input.set_out_synth(input_feed);
            (shared, p)
        };
        let mut synth_thread = None;
        if let Some(sf2) = &opts.sf2 {
            match start_synth(sf2, std::mem::take(&mut feeds.consumers), opts.audio_out, shared.parts.clone()) {
                Ok((r, t)) => {
                    p.input.set_synth(Some(r.control.clone()));
                    p.control.synth = Some(r);
                    synth_thread = Some(t);
                }
                Err(e) => p.control.say(format!("synth off: {e:#}"), true),
            }
        }
        if p.control.synth.is_none() {
            p.engine.io.out.synth = None;
            p.input.set_out_synth(None);
        }

        let port = client.input_port("yahaha in", p.input)?;
        let leds_port = if opts.no_pads { None } else { Some(client.output_port("yahaha leds")?) };
        p.control.midi = Some(MidiIo { port, slots: Default::default(), daw: None, leds_port, leds_dest: None, no_pads: opts.no_pads, setup_gen: midi::setup_generation() });
        p.control.connect_pads();
        p.control.connect_inputs();
        p.control.sources_ns = rt::now_ns();

        if p.control.set_transpose(opts.transpose).is_err() {
            p.control.transpose = Transpose::default();
        }

        let sh = shared.clone();
        let EngineLoopParts { engine, io } = p.engine;
        let engine_thread =
            std::thread::Builder::new().name("yahaha-engine".into()).spawn(move || live::run_engine(engine, io, sh))?;
        p.control.shared.parts.set_bass_program(synth::style_bass_program(p.control.info.voices[10]));
        p.control.sync_manual_bass();

        let inner = Arc::new(Inner::new(shared, p.control));
        let i2 = inner.clone();
        let control = std::thread::Builder::new().name("yahaha-control".into()).spawn(move || i2.control_loop())?;
        Ok(Session {
            inner,
            live: Mutex::new(Some(Live { client, _port: port, engine: engine_thread, control, synth: synth_thread })),
        })
    }

    /// Run a command. Returns once it has been applied on the control side; what it does
    /// in the engine (sections, tempo, mixer) shows in the state a moment later (live) or
    /// at once (offline). The Launchkey sends the same commands.
    pub fn send(&self, cmd: impl Into<AppCmd>) -> Result<(), CmdError> {
        let mut ctl = self.inner.lock();
        let r = ctl.apply(cmd.into());
        if ctl.offline.is_some() {
            drop(ctl);
            self.settle();
        } else {
            // Publish now, so `state()` straight after `send` shows what the control side
            // applied. The control thread republishes once the engine has run its part.
            self.inner.publish(&mut ctl, rt::now_ns());
            drop(ctl);
            self.inner.shared.ctl_wake.signal();
        }
        r
    }

    /// The latest state. Cheap: an `Arc` clone.
    pub fn state(&self) -> Arc<AppState> {
        self.inner.state.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// The latest state's version (changes whenever the state does).
    pub fn version(&self) -> u64 {
        self.inner.version.load(Relaxed)
    }

    /// The style library (indexed in the background; `AppState::library.revision` and
    /// `Event::LibraryChanged` say when it changed). Cheap: an `Arc` clone.
    pub fn library(&self) -> Arc<Library> {
        self.inner.library.lock().unwrap_or_else(|e| e.into_inner()).0.clone()
    }

    /// The library as plain data, in display order (folder, then name).
    pub fn library_list(&self) -> LibraryList {
        let (lib, revision) = self.inner.library.lock().unwrap_or_else(|e| e.into_inner()).clone();
        LibraryList { revision, entries: lib.order().iter().map(|&id| library_entry(&lib, id)).collect(), voices: voice_options() }
    }

    /// Notifications: a `StateChanged` whenever the state's version moves, a
    /// `LibraryChanged` when the library does, `Stopped` at the end. Unread events queue up;
    /// drop the receiver to unsubscribe.
    pub fn subscribe(&self) -> mpsc::Receiver<Event> {
        let (tx, rx) = mpsc::channel();
        self.inner.subscribers.lock().unwrap_or_else(|e| e.into_inner()).push(tx);
        rx
    }

    /// The free-running beat clock the Launchkey pads flash and pulse on (fractional
    /// beats, following the tempo): draw with it to flash in step with the hardware.
    pub fn beats(&self) -> f64 {
        let ctl = self.inner.lock();
        let now = match &ctl.offline {
            Some(o) => o.now,
            None => rt::now_ns(),
        };
        ctl.led_beats_at(now)
    }

    /// The session clock, in ns (monotonic; the virtual clock offline): the time base of
    /// `AppState::clock`.
    pub fn now_ns(&self) -> u64 {
        match &self.inner.lock().offline {
            Some(o) => o.now,
            None => rt::now_ns(),
        }
    }

    /// The output meters: each part's peak and the master's since the last call, and the
    /// clip count (`Meters`). Cheap; meant for one reader polling at display rate (the
    /// app's meter bridge), which applies its own decay and peak hold. Without the synth
    /// (offline, or no SoundFont), no channels and zero levels.
    pub fn meters(&self) -> Meters {
        let ctl = self.inner.lock();
        let now = ctl.offline.as_ref().map_or_else(rt::now_ns, |o| o.now);
        let Some(sy) = &ctl.synth else { return Meters { at_ms: ns_to_ms(now), ..Meters::default() } };
        let (peaks, master, clips) = synth::take_meters(&sy.control);
        Meters {
            at_ms: ns_to_ms(now),
            channels: synth::RACK_CHANNELS.iter().map(|&c| ChannelMeter { channel: c + 1, peak: peaks[c as usize] }).collect(),
            master,
            clips,
        }
    }

    /// The latest state with its clock read now (`surface.clock`: `atMs`, `bar`, `beat`,
    /// `phase`): what a client that animates from the clock should fetch.
    pub fn state_now(&self) -> AppState {
        let mut st = (*self.state()).clone();
        st.surface.clock = st.surface.clock.at(ns_to_ms(self.now_ns()));
        st
    }

    /// Stop: the band stops, the Launchkey leaves DAW mode, audio and MIDI close.
    /// Idempotent; `Drop` calls it.
    pub fn stop(&self) {
        // Claim the stop before taking the threads: a second, concurrent `stop` must not
        // take them and then return without joining them.
        if self.inner.stop.swap(true, Relaxed) {
            return;
        }
        let live = self.live.lock().unwrap_or_else(|e| e.into_inner()).take();
        let shared = &self.inner.shared;
        shared.quit.store(true, Relaxed);
        shared.wake.signal();
        shared.ctl_wake.signal();
        if let Some(live) = live {
            let _ = live.engine.join();
            let _ = live.control.join();
            if let Some(leds) = self.inner.lock().leds.as_mut() {
                leds.off();
            }
            if let Some(s) = live.synth {
                let _ = s.stop.send(());
                let _ = s.thread.join();
            }
            live.client.dispose();
        } else {
            let mut ctl = self.inner.lock();
            if let Some(o) = ctl.offline.as_mut() {
                o.engine.stop();
            }
        }
        self.inner.notify(&[Event::Stopped]);
    }
}

impl Inner {
    fn new(shared: Arc<Shared>, control: Control) -> Inner {
        let mut control = control;
        control.published = Arc::new(control.lib.clone());
        let lib = control.published.clone();
        control.lib_published = control.lib_rev;
        let rev = control.lib_rev;
        let inner = Inner {
            shared,
            ctl: Mutex::new(control),
            state: Mutex::new(Arc::new(AppState::default())),
            library: Mutex::new((lib, rev)),
            version: AtomicU64::new(0),
            subscribers: Mutex::new(Vec::new()),
            stop: AtomicBool::new(false),
        };
        {
            let mut ctl = inner.lock();
            let now = ctl.clock_ns;
            ctl.drain_snapshots();
            inner.publish(&mut ctl, now);
        }
        inner
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
    }
}

// A session is shared between client threads (and Tauri's managed state needs both).
const _: fn() = || {
    fn is_send_sync<T: Send + Sync>() {}
    is_send_sync::<Session>();
};

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
