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

use crate::api::*;
use crate::engine::{id_of, Engine, Prepared, Snapshot, Transpose, NUM_SLOTS};
use crate::fingering::Fingering;
use crate::launchkey::{self, Action, Led, Page, Panel};
use crate::library::{self, Info, Library};
use crate::live::{self, Audition, Cmd, EngineLoop, Input, Shared, MAX_KEY_SOURCES, TAG_KEYS, TAG_PADS};
use crate::midi::{self, Client, InputHandler};
use crate::parts::{self, FaderPage};
use crate::rt::{self, PacketSink, Target};
use crate::sff::{Ots, Style};
use crate::synth::{self, SynthControl, SynthInfo};
use crate::theory::Recognizer;
use anyhow::{Context, Result};
use rtrb::{Consumer, Producer, RingBuffer};
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

/// The loaded style, as the panel shows it.
struct Loaded {
    path: PathBuf,
    name: String,
    format: String,
    bpm: f64,
    timesig: (u8, u8),
    /// Quarter notes per bar.
    quarters_per_bar: f64,
    has: [bool; NUM_SLOTS],
    voices: [Option<(u8, u8, u8)>; 16],
    ots: Vec<Ots>,
}

fn load(path: &Path) -> Result<(Box<Prepared>, Loaded)> {
    let style = Style::load(path)?;
    let prep = Box::new(Prepared::new(&style));
    let mut has = [false; NUM_SLOTS];
    for (i, s) in prep.sections.iter().enumerate() {
        has[i] = s.is_some();
    }
    let name = if style.name.is_empty() {
        path.file_stem().unwrap_or_default().to_string_lossy().to_string()
    } else {
        style.name.clone()
    };
    let info = Loaded {
        path: path.to_path_buf(),
        name,
        format: style.format.clone(),
        bpm: prep.bpm,
        timesig: style.timesig,
        quarters_per_bar: prep.tpb as f64 / prep.ppq.max(1) as f64,
        has,
        voices: prep.voices,
        ots: style.ots.clone(),
    };
    Ok((prep, info))
}

/// The Launchkey LEDs: pads, fader buttons, Pad Bank and Track buttons. Sends only what
/// changed.
struct Leds {
    out: PacketSink,
    palette: bool,
    last_leds: [(u8, Option<Led>); 16],
    last_rgb: [Option<(u8, u8, u8)>; 16],
    last_fader_btns: Option<(FaderPage, u8, u8)>,
    last_nav: Option<(Page, bool)>,
    buf: Vec<[u8; 3]>,
}

impl Leds {
    fn new(out: PacketSink, palette: bool) -> Leds {
        Leds { out, palette, last_leds: [(0, None); 16], last_rgb: [None; 16], last_fader_btns: None, last_nav: None, buf: Vec::new() }
    }

    #[allow(clippy::too_many_arguments)]
    fn update(&mut self, s: &Snapshot, has: &[bool], pnl: &Panel, manual_bass: bool, fader_page: FaderPage, styles: bool, beats: f64) {
        if self.palette {
            for (i, (note, led)) in launchkey::pad_leds(s, has, pnl).into_iter().enumerate() {
                if self.last_leds[i] != (note, Some(led)) {
                    self.buf.clear();
                    launchkey::led_msgs(note, led, &mut self.buf);
                    for m in &self.buf {
                        self.out.push(m);
                    }
                    self.last_leds[i] = (note, Some(led));
                }
            }
        } else {
            for (i, (pad, look)) in launchkey::looks(s, has, pnl).iter().enumerate() {
                let rgb = launchkey::rgb_at(look, beats);
                if self.last_rgb[i] != Some(rgb) {
                    self.out.push(&launchkey::rgb_sysex(*pad, rgb));
                    self.last_rgb[i] = Some(rgb);
                }
            }
        }
        let style_on = launchkey::style_lit(s.parts, manual_bass);
        let fb = (fader_page, pnl.parts_on, style_on);
        if self.last_fader_btns != Some(fb) {
            self.buf.clear();
            launchkey::fader_button_msgs(fb.0, fb.1, fb.2, &mut self.buf);
            for m in &self.buf {
                self.out.push(m);
            }
            self.last_fader_btns = Some(fb);
        }
        if self.last_nav != Some((pnl.page, styles)) {
            self.buf.clear();
            launchkey::nav_button_msgs(pnl.page, styles, &mut self.buf);
            for m in &self.buf {
                self.out.push(m);
            }
            self.last_nav = Some((pnl.page, styles));
        }
        self.out.flush();
    }

    /// Palette colours or RGB from now on: every pad is sent again.
    fn set_palette(&mut self, on: bool) {
        if self.palette != on {
            self.palette = on;
            self.last_leds = [(0, None); 16];
            self.last_rgb = [None; 16];
        }
    }

    /// Pads and buttons dark, and the Launchkey back out of DAW mode.
    fn off(&mut self) {
        for n in (96..104).chain(112..120) {
            self.out.push(&[0x90, n, 0]);
        }
        self.buf.clear();
        launchkey::buttons_off_msgs(&mut self.buf);
        for m in &self.buf {
            self.out.push(m);
        }
        self.out.push(&launchkey::EXIT_DAW);
        self.out.flush();
    }
}

/// The synth as the control side sees it.
struct SynthRef {
    info: SynthInfo,
    control: Arc<SynthControl>,
    /// Swapping SoundFonts (None: the synth can't).
    swap: Option<synth::RackSwap>,
}

/// What the SoundFont loader thread sends back.
type RackLoad = Result<Box<synth::Rack>, String>;

/// The chords a style preview plays, one a bar.
pub const AUDITION_CHORDS: [&str; 4] = ["C", "Am", "F", "G7"];

/// A live session's MIDI input: the port, and which sources it listens to.
struct MidiIo {
    port: midi::InputPort,
    /// Keyboard sources by slot (`live::key_tag`; slot 0 is unused live).
    slots: [Option<(midi::Endpoint, String)>; MAX_KEY_SOURCES],
    /// The Launchkey DAW port, when yahaha drives it.
    daw: Option<(midi::Endpoint, String)>,
}

/// The Launchkey's DAW port (pads, buttons, faders), by its source name.
fn is_daw(name: &str) -> bool {
    name.contains("Launchkey") && name.contains("DAW")
}

/// The sources (by index into `sources`) that play the keyboard: every one when `all`,
/// else those whose name contains one of `names`, else (no names) a Launchkey's keys when
/// there is one, else every one. Never yahaha's own port, never a DAW port.
pub fn choose_keys(sources: &[String], all: bool, names: &[String]) -> Vec<usize> {
    let ok: Vec<usize> = (0..sources.len()).filter(|&i| !sources[i].starts_with("yahaha") && !sources[i].contains("DAW")).collect();
    if !all && !names.is_empty() {
        return ok.into_iter().filter(|&i| names.iter().any(|n| !n.is_empty() && sources[i].contains(n.as_str()))).collect();
    }
    let lk: Vec<usize> = ok.iter().copied().filter(|&i| sources[i].contains("Launchkey")).collect();
    if all || lk.is_empty() { ok } else { lk }
}

/// An offline session's engine, input and clock.
struct Offline {
    engine: EngineLoop,
    input: Input,
    now: u64,
    /// What the band and the keyboard parts played, as the synth would get it.
    band: Consumer<[u8; 3]>,
    keys: Consumer<[u8; 3]>,
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

    /// Push the effective Manual Bass state (Upper mode and the setting both on) to the
    /// engine, which mutes the Style's Bass part, and to the keyboard parts, where the Left
    /// part takes the Style's Bass voice and fader.
    fn sync_manual_bass(&mut self) {
        let on = self.shared.manual_bass();
        if self.ui_tx.push(Cmd::ManualBass(on)).is_ok() {
            self.wake_engine();
        }
        self.shared.parts.set_manual_bass(on);
    }

    /// Played notes and the engine must agree, so the key shift only changes once the
    /// engine has the command.
    fn set_transpose(&mut self, t: Transpose) -> Result<(), CmdError> {
        let t = Transpose::new(t.keyboard, t.master);
        self.engine_cmd(Cmd::Transpose(t))?;
        self.shared.key_shift.store(t.keys(), Relaxed);
        self.transpose = t;
        Ok(())
    }

    /// Recall a One Touch Setting into the keyboard parts; the engine thread sends the new
    /// volumes as CC7 on its next wake.
    fn recall_ots(&mut self, index: u8) {
        if let Some(o) = self.info.ots.get(index as usize) {
            self.shared.parts.apply_ots(o, index + 1);
            self.wake_engine();
        }
    }

    fn set_upper(&mut self, on: bool) {
        self.shared.upper.store(on, Relaxed);
        // Selecting Upper turns Manual Bass on, its default there.
        if on {
            self.shared.manual_bass.store(true, Relaxed);
        }
        self.sync_manual_bass();
        self.wake_engine();
    }

    fn set_part_on(&mut self, part: u8, on: bool) -> Result<(), CmdError> {
        let p = (part & 3) as usize;
        if self.shared.parts.is_on(p) != on && !self.shared.parts.toggle(p) {
            return self.fail("Left plays the bass under Manual Bass: turn Manual Bass off [D] to switch Left");
        }
        Ok(())
    }

    /// Load a style and hand it to the engine, playing or stopped: the one path every
    /// style change takes. A file that fails to load is marked as an error row, so
    /// stepping skips it next time.
    ///
    /// Stopped, the engine switches at once; playing, at the next bar line
    /// (`Engine::change_style`). Either way the state shows the new style once the engine
    /// plays it (`promote_style`); until then `preview.queued` names it.
    fn switch_style(&mut self, id: usize) -> Result<(), CmdError> {
        match self.load_entry(id) {
            Ok((mut p, info)) => {
                self.style_seq += 1;
                p.tag = self.style_seq;
                if self.style_tx.push(p).is_err() {
                    return Err(CmdError::Busy);
                }
                self.pending_style = Some((id, self.style_seq, info));
                self.wake_engine();
                self.message = None;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Load library entry `id`. A file that fails to load is marked as an error row, so
    /// stepping skips it next time.
    fn load_entry(&mut self, id: usize) -> Result<(Box<Prepared>, Loaded), CmdError> {
        if id >= self.lib.len() {
            return Err(self.fail(format!("no style {id} in the library")).unwrap_err());
        }
        let path = self.lib.entry(id).path.clone();
        load(&path).map_err(|e| {
            self.lib.set_info(id, Info::Err(format!("{e:#}")));
            self.lib.sort();
            self.lib_rev += 1;
            self.lib_urgent = true;
            self.fail(format!("{}: {e:#}", path.display())).unwrap_err()
        })
    }

    /// The style the player last chose: the one waiting for the bar line, else the one
    /// playing.
    fn target_style(&self) -> usize {
        self.pending_style.as_ref().map_or(self.cur, |p| p.0)
    }

    /// Load `id` unless it is already the style chosen.
    fn choose_style(&mut self, id: usize) -> Result<(), CmdError> {
        if id != self.target_style() {
            return self.switch_style(id);
        }
        Ok(())
    }

    /// The engine plays the style handed to it: it becomes the loaded style.
    fn promote_style(&mut self) {
        let Some((_, tag, _)) = &self.pending_style else { return };
        if self.snap.style_tag != *tag {
            return;
        }
        let (id, _, info) = self.pending_style.take().unwrap();
        self.shared.parts.set_bass_program(synth::style_bass_program(info.voices[10]));
        // No OTS of the new style is recalled yet (OTS Link recalls one on the next pass
        // if it's on).
        self.shared.parts.ots_applied.store(0, Relaxed);
        self.cur = id;
        self.info = info;
    }

    /// A style preview (`AuditionStyle`), for the engine thread.
    fn audition(&mut self, id: usize) -> Result<(), CmdError> {
        if self.snap.running {
            return self.fail("Stop the band to preview a style");
        }
        let (p, _) = self.load_entry(id)?;
        let engine = Engine::new(p);
        let chords = AUDITION_CHORDS.map(|c| crate::parse_chord(c).expect("audition chord"));
        if self.audition_tx.push(Box::new(Audition { engine, id: id as u32, chords })).is_err() {
            return Err(CmdError::Busy);
        }
        self.wake_engine();
        Ok(())
    }

    /// The SoundFonts in the synth's folder.
    fn list_sound_fonts(&mut self) {
        if let Some(dir) = &self.sf_dir {
            self.sound_fonts = library::sound_font_files(dir);
        }
    }

    /// Start loading another SoundFont from the synth's folder, on a thread of its own.
    fn set_sound_font(&mut self, file: String) -> Result<(), CmdError> {
        let Some(sy) = &self.synth else { return self.fail("the synth is off") };
        if sy.swap.is_none() {
            return self.fail("this synth can't change SoundFonts");
        }
        let sample_rate = sy.info.sample_rate;
        self.list_sound_fonts();
        let bad = file.contains('/') || file.contains('\\') || file.starts_with('.') || !file.to_lowercase().ends_with(".sf2");
        let path = self.sf_dir.as_ref().map(|d| d.join(&file));
        let Some(path) = path.filter(|p| !bad && p.is_file()) else {
            return self.fail(format!("no SoundFont {file} in the SoundFont folder"));
        };
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new().name("yahaha-sf2".into()).spawn(move || {
            let _ = tx.send(synth::Rack::load(&path, sample_rate).map_err(|e| format!("{e:#}")));
        });
        if spawned.is_err() {
            return self.fail("couldn't start loading the SoundFont");
        }
        self.sf_load = Some((file, rx));
        Ok(())
    }

    /// The SoundFont loader's result: hand the new rack to the audio thread.
    fn pump_sound_font(&mut self) {
        if let Some((file, rx)) = &self.sf_load {
            match rx.try_recv() {
                Ok(Ok(rack)) => {
                    self.sf_ready = Some((file.clone(), rack));
                    self.sf_load = None;
                }
                Ok(Err(e)) => {
                    let msg = format!("SoundFont {file}: {e}");
                    self.sf_load = None;
                    self.say(msg, true);
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => self.sf_load = None,
            }
        }
        let Some(sy) = self.synth.as_mut() else { return };
        let Some(swap) = sy.swap.as_mut() else { return };
        while swap.old.pop().is_ok() {} // old racks are freed here, off the audio thread
        if let Some((file, rack)) = self.sf_ready.take() {
            match swap.tx.push(rack) {
                Ok(()) => {
                    sy.info.name = Path::new(&file).file_stem().unwrap_or_default().to_string_lossy().to_string();
                    self.sf_file = Some(file);
                }
                Err(rtrb::PushError::Full(rack)) => self.sf_ready = Some((file, rack)),
            }
        }
    }

    /// Start a rescan of the style folders on a thread of its own.
    fn rescan(&mut self) {
        if self.scan_rx.is_some() {
            return;
        }
        let roots = self.roots.clone();
        let (tx, rx) = mpsc::channel();
        if std::thread::Builder::new().name("yahaha-scan".into()).spawn(move || drop(tx.send(Library::scan(&roots)))).is_ok() {
            self.scan_rx = Some(rx);
        }
    }

    /// A finished rescan: merge it, and index what's new.
    fn pump_rescan(&mut self) {
        let Some(rx) = &self.scan_rx else { return };
        match rx.try_recv() {
            Ok(scanned) => {
                self.scan_rx = None;
                let added = self.lib.merge(scanned, &self.roots);
                // Every entry still pending (new, or not reached by the first index).
                self.index_rx = Some(self.lib.spawn_pending_indexer());
                self.lib_rev += 1;
                self.lib_urgent = true;
                let n = self.lib.count();
                self.say(format!("Style folders rescanned: {n} styles, {} new", added.len()), false);
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => self.scan_rx = None,
        }
    }

    /// Connect the keyboard sources `all_inputs`/`input_names` choose and disconnect the
    /// rest; list every source. A source dropped with keys held has them released.
    fn connect_inputs(&mut self) {
        let Some(m) = self.midi.as_mut() else { return };
        let sources = midi::sources();
        let names: Vec<String> = sources.iter().map(|(_, n)| n.clone()).collect();
        let want: Vec<midi::Endpoint> = choose_keys(&names, self.all_inputs, &self.input_names).into_iter().map(|i| sources[i].0).collect();
        let mut dropped = Vec::new();
        for slot in 1..MAX_KEY_SOURCES {
            if let Some((e, _)) = m.slots[slot]
                && !want.contains(&e)
            {
                let _ = m.port.disconnect(e);
                m.slots[slot] = None;
                dropped.push(slot);
            }
        }
        // Queue the releases before a new source can take a freed slot: the input thread
        // applies them before that source's first packet, so they never release its keys.
        for slot in dropped {
            if self.shared.src_held[slot].load(Relaxed) > 0 {
                // The input thread releases the keys (and the chord) on its next message;
                // the notes stop now.
                let _ = self.release_tx.push(slot as u8);
                let _ = self.engine_cmd(Cmd::KeysOff);
            }
        }
        let Some(m) = self.midi.as_mut() else { return };
        for e in want {
            if m.slots.iter().flatten().any(|(x, _)| *x == e) {
                continue;
            }
            let Some(slot) = (1..MAX_KEY_SOURCES).find(|&s| m.slots[s].is_none()) else { break };
            if m.port.connect(e, live::key_tag(slot)).is_ok() {
                let name = sources.iter().find(|(x, _)| *x == e).map_or_else(String::new, |(_, n)| n.clone());
                m.slots[slot] = Some((e, name));
            }
        }
        self.inputs = m.slots.iter().flatten().map(|(_, n)| n.clone()).chain(m.daw.iter().map(|(_, n)| format!("{n} (pads)"))).collect();
        self.sources = sources
            .iter()
            .filter(|(_, n)| !n.starts_with("yahaha"))
            .map(|(e, n)| MidiSource {
                name: n.clone(),
                listening: m.slots.iter().flatten().any(|(x, _)| x == e) || m.daw.as_ref().is_some_and(|(x, _)| x == e),
                pads: is_daw(n),
            })
            .collect();
    }

    fn apply(&mut self, cmd: AppCmd) -> Result<(), CmdError> {
        if let Some(b) = cmd.button() {
            return self.engine_cmd(Cmd::Button(b));
        }
        let parts = self.shared.parts.clone();
        match cmd {
            AppCmd::Mixer(MixerCmd::SetStylePartVolume { part, volume }) => return self.engine_cmd(Cmd::StyleVolume(part & 7, volume.min(127))),
            AppCmd::Chord(ChordCmd::SetFingering { fingering }) => {
                self.shared.fingering.store(fingering.to_u8(), Relaxed);
                self.wake_engine();
            }
            AppCmd::Chord(ChordCmd::NextFingering) => {
                let f = Fingering::from_u8(self.shared.fingering.load(Relaxed)).next();
                self.shared.fingering.store(f.to_u8(), Relaxed);
                self.wake_engine();
            }
            AppCmd::Chord(ChordCmd::SetUpper { on }) => {
                if on != self.shared.upper.load(Relaxed) {
                    self.set_upper(on);
                }
            }
            AppCmd::Chord(ChordCmd::ToggleUpper) => {
                let on = !self.shared.upper.load(Relaxed);
                self.set_upper(on);
            }
            // Manual Bass is only available in Upper mode.
            AppCmd::Chord(ChordCmd::SetManualBass { on }) => {
                if self.shared.upper.load(Relaxed) {
                    self.shared.manual_bass.store(on, Relaxed);
                    self.sync_manual_bass();
                }
            }
            AppCmd::Chord(ChordCmd::ToggleManualBass) => {
                if self.shared.upper.load(Relaxed) {
                    let v = !self.shared.manual_bass.load(Relaxed);
                    self.shared.manual_bass.store(v, Relaxed);
                    self.sync_manual_bass();
                }
            }
            AppCmd::Chord(ChordCmd::SetSplit { note }) => self.shared.split.store(note.clamp(24, 96), Relaxed),
            AppCmd::Chord(ChordCmd::MoveSplit { delta }) => {
                let s = self.shared.split.load(Relaxed) as i16 + delta as i16;
                self.shared.split.store(s.clamp(24, 96) as u8, Relaxed);
            }
            AppCmd::Chord(ChordCmd::SetTranspose { keyboard, master }) => return self.set_transpose(Transpose::new(keyboard, master)),
            AppCmd::Chord(ChordCmd::StepTranspose { keyboard, master }) => {
                let t = self.transpose;
                return self.set_transpose(Transpose::new(t.keyboard.saturating_add(keyboard), t.master.saturating_add(master)));
            }
            AppCmd::Chord(ChordCmd::ResetTranspose) => return self.set_transpose(Transpose::default()),
            AppCmd::Parts(PartsCmd::SetPartOn { part, on }) => return self.set_part_on(part, on),
            AppCmd::Parts(PartsCmd::TogglePart { part }) => {
                let on = !parts.is_on((part & 3) as usize);
                return self.set_part_on(part, on);
            }
            AppCmd::Parts(PartsCmd::SelectPart { part }) => parts.select(part as usize),
            AppCmd::Parts(PartsCmd::SetPartVoice { part, program }) => parts.set_program((part & 3) as usize, program),
            AppCmd::Parts(PartsCmd::StepVoice { delta }) => parts.step_program(delta as i32),
            AppCmd::Parts(PartsCmd::SetPartVolume { part, volume }) => {
                parts.set_volume((part & 3) as usize, volume);
                self.wake_engine();
            }
            AppCmd::Parts(PartsCmd::SetPartOctave { part, octave }) => parts.octave[(part & 3) as usize].store(octave.clamp(-2, 2), Relaxed),
            AppCmd::Mixer(MixerCmd::SetFaderPage { page }) => {
                if parts.fader_page() != page {
                    parts.set_fader_page(page);
                    self.wake_engine();
                }
            }
            AppCmd::Mixer(MixerCmd::ToggleFaderPage) => {
                parts.toggle_fader_page();
                self.wake_engine();
            }
            AppCmd::Pads(PadsCmd::SetPadPage { page }) => self.shared.page.store(page.to_u8(), Relaxed),
            AppCmd::Pads(PadsCmd::CyclePadPage { delta }) => self.shared.step_page(|p| p.cycle(delta)),
            AppCmd::Mixer(MixerCmd::SetMasterVolume { volume }) => match &self.synth {
                Some(s) => {
                    let v = volume.min(127);
                    s.control.master.store(v, Relaxed);
                    // The fader has to reach the new level before it takes over again.
                    let hw = self.shared.master_hw.load(Relaxed);
                    s.control.master_waiting.store(crate::engine::Takeover::at(hw, v).waiting(), Relaxed);
                    self.shared.master_moved.store(true, Relaxed);
                }
                None => return self.fail("the synth is off"),
            },
            AppCmd::Ots(OtsCmd::RecallOts { index }) => self.recall_ots(index),
            AppCmd::Ots(OtsCmd::SetOtsLink { on }) => parts.ots_link.store(on, Relaxed),
            AppCmd::Ots(OtsCmd::ToggleOtsLink) => {
                parts.ots_link.fetch_xor(true, Relaxed);
            }
            // With one style there is nowhere to go, and the loaded one isn't reloaded.
            AppCmd::Library(LibraryCmd::LoadStyle { id }) | AppCmd::Library(LibraryCmd::QueueStyle { id }) => return self.choose_style(id),
            AppCmd::Preview(PreviewCmd::AuditionStyle { id }) => return self.audition(id),
            AppCmd::Preview(PreviewCmd::StopAudition) => return self.engine_cmd(Cmd::StopAudition),
            AppCmd::Library(LibraryCmd::RescanLibrary) => self.rescan(),
            AppCmd::Settings(SettingsCmd::SetSoundFont { file }) => return self.set_sound_font(file),
            AppCmd::Settings(SettingsCmd::SetMidiInputs { all, names }) => {
                self.all_inputs = all;
                self.input_names = names;
                self.connect_inputs();
            }
            AppCmd::Settings(SettingsCmd::SetPaletteLeds { on }) => {
                self.palette_leds = on;
                if let Some(l) = self.leds.as_mut() {
                    l.set_palette(on);
                }
            }
            AppCmd::Library(LibraryCmd::LoadStylePath { path }) => {
                let path = PathBuf::from(path);
                let id = match self.lib.find(&path) {
                    Some(id) => id,
                    None => {
                        let id = self.lib.add_file(path);
                        self.lib_rev += 1;
                        self.lib_urgent = true;
                        id
                    }
                };
                return self.choose_style(id);
            }
            // Folder-then-name order, the browser's unfiltered list.
            AppCmd::Library(LibraryCmd::StepStyle { delta }) => {
                let next = self.lib.step(self.target_style(), delta);
                return self.choose_style(next);
            }
            AppCmd::Mixer(MixerCmd::SetSynthMuted { on }) => {
                if let Some(s) = &self.synth {
                    s.control.muted.store(on, Relaxed);
                }
            }
            AppCmd::Mixer(MixerCmd::ToggleSynthMute) => {
                if let Some(s) = &self.synth {
                    s.control.muted.fetch_xor(true, Relaxed);
                }
            }
            AppCmd::Settings(SettingsCmd::SetAudioOutput { first }) => {
                if let Some(s) = &self.synth {
                    let n = s.info.channels.max(2) as u8;
                    s.control.out_ch.store(first.min(n - 2), Relaxed);
                }
            }
            // Next stereo output pair: 1/2 -> 3/4 -> ... -> back to 1/2.
            AppCmd::Settings(SettingsCmd::NextAudioOutput) => {
                if let Some(s) = &self.synth {
                    let n = s.info.channels.max(2) as u8;
                    let c = s.control.out_ch.load(Relaxed);
                    s.control.out_ch.store(if c + 4 <= n { c + 2 } else { 0 }, Relaxed);
                }
            }
            AppCmd::System(SystemCmd::Panic) => return self.engine_cmd(Cmd::Panic),
            AppCmd::System(SystemCmd::ClearMessage) => self.message = None,
            // Engine buttons were handled above.
            _ => {}
        }
        Ok(())
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
    /// Link, the LEDs, indexing. `now` is the clock the pad flashing follows.
    fn pump(&mut self, now: u64) {
        self.drain_snapshots();
        // Launchkey pad/button actions: the same commands as their keyboard shortcuts.
        while let Ok(a) = self.act_rx.pop() {
            let _ = self.apply(a.into());
        }
        // OTS Link: Main A-D recall One Touch Settings 1-4 (also on style change).
        let s = self.snap;
        let key = (self.cur, s.main);
        let link = self.shared.parts.ots_link.load(Relaxed);
        let due = link && (self.last_ots_key != Some(key) || !self.last_link);
        if due && (s.main as usize) < self.info.ots.len() {
            self.recall_ots(s.main);
        }
        self.last_ots_key = Some(key);
        self.last_link = link;
        while self.old_rx.pop().is_ok() {} // drop old styles here, off the RT thread
        while self.old_audition_rx.pop().is_ok() {}
        self.pump_sound_font();
        self.pump_rescan();
        if self.midi.is_some() && now.saturating_sub(self.sources_ns) >= 2_000_000_000 {
            self.sources_ns = now;
            self.connect_inputs();
        }

        // Free-running beat clock for flashing/pulsing, following the current tempo.
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
        if let Some(rx) = &self.index_rx
            && self.lib.apply(rx) > 0
        {
            self.lib_rev += 1;
        }
    }

    /// The Launchkey beyond the pads, as `live::Input` runs it and `Leds` lights it.
    fn surface(&self, pnl: &Panel, manual_bass_active: bool, now: u64) -> SurfaceState {
        use launchkey::{cc_control, Control as C};
        let shared = &self.shared;
        let kp = &shared.parts;
        let page = pnl.page;
        let styles = self.published.count() > 1;
        let fader_page = kp.fader_page();
        let style_on = launchkey::style_lit(self.snap.parts, manual_bass_active);
        let colours = launchkey::button_colours(page, styles, fader_page, pnl.parts_on, style_on);
        let act = |cc: u8, shift: bool| -> Option<AppCmd> {
            match cc_control(cc, shift)? {
                C::Page(d) => {
                    let to = page.step(d);
                    (to != page).then_some(AppCmd::Pads(PadsCmd::SetPadPage { page: to }))
                }
                C::Act(Action::Style(_)) if !styles => None,
                C::Act(a) => Some(a.into()),
            }
        };
        let mut controls = Vec::new();
        let mut push = |id: String, cc: u8, label: &str, action: Option<AppCmd>, shift: Option<(&str, Option<AppCmd>)>| {
            let label = if action.is_some() { label.to_string() } else { String::new() };
            let (shift_label, shift_action) = match shift {
                Some((l, a)) => (if a.is_some() { l.to_string() } else { String::new() }, a),
                None => (label.clone(), action.clone()),
            };
            let colour = colours.iter().find(|c| c.0 == cc).map(|c| c.1);
            let (rgb, level) = colour.map_or(((0, 0, 0), launchkey::Level::Off), launchkey::palette_colour);
            controls.push(SurfaceControl {
                id,
                cc,
                label,
                action,
                shift_label,
                shift_action,
                rgb: [rgb.0, rgb.1, rgb.2],
                level,
                anim: launchkey::Anim::Solid,
                colour,
            });
        };
        for (id, cc, label, shift_label) in [
            ("padBankUp", launchkey::PAD_UP_CC, "PAGE ▲", "LEFT"),
            ("padBankDown", launchkey::PAD_DOWN_CC, "PAGE ▼", "OTS LINK"),
            ("trackPrev", launchkey::TRACK_LEFT_CC, "◀ STYLE", ""),
            ("trackNext", launchkey::TRACK_RIGHT_CC, "STYLE ▶", ""),
            ("play", launchkey::PLAY_CC, "PLAY", ""),
            ("stop", launchkey::STOP_CC, "STOP", ""),
            ("scene", launchkey::SCENE_CC, "TEMPO +", ""),
            ("function", launchkey::FUNCTION_CC, "TEMPO -", ""),
        ] {
            let (a, sa) = (act(cc, false), act(cc, true));
            let shift = (sa != a).then_some((shift_label, sa));
            push(id.to_string(), cc, label, a, shift);
        }
        // The buttons under faders 1-8: Panel = Right 1-3 and Left on/off (Shift: select),
        // Style = the Style parts' mute.
        for i in 0..8u8 {
            let cc = launchkey::FADER_BTN_CC.start() + i;
            let id = format!("faderButton{}", i + 1);
            match fader_page {
                FaderPage::Panel if (i as usize) < parts::COUNT => {
                    let p = i as usize;
                    let shift = (launchkey::SELECT_LABELS[p], Some(AppCmd::Parts(PartsCmd::SelectPart { part: i })));
                    push(id, cc, launchkey::PART_LABELS[p], Some(AppCmd::Parts(PartsCmd::TogglePart { part: i })), Some(shift));
                }
                FaderPage::Panel => push(id, cc, "", None, None),
                FaderPage::Style => {
                    let name = STYLE_PART_NAMES[i as usize].to_uppercase();
                    push(id, cc, &name, Some(AppCmd::Mixer(MixerCmd::ToggleStylePart { part: i })), None);
                }
            }
        }
        let master = match fader_page {
            FaderPage::Panel => "PANEL",
            FaderPage::Style => "STYLE",
        };
        push("masterButton".into(), *launchkey::FADER_BTN_CC.end(), master, Some(AppCmd::Mixer(MixerCmd::ToggleFaderPage)), None);

        // The faders: the parts they control on this page, and where they physically are.
        let s = &self.snap;
        let mut faders: Vec<SurfaceFader> = (0..8u8)
            .map(|i| {
                let p = i as usize;
                let position = known(kp.fader_hw[p].load(Relaxed));
                match fader_page {
                    FaderPage::Panel if p < parts::COUNT => SurfaceFader {
                        label: launchkey::PART_LABELS[p].to_string(),
                        value: Some(kp.volume(p)),
                        waiting: kp.waiting(p),
                        position,
                        set: Some(AppCmd::Parts(PartsCmd::SetPartVolume { part: i, volume: 0 })),
                    },
                    FaderPage::Panel => SurfaceFader { position, ..SurfaceFader::default() },
                    FaderPage::Style => SurfaceFader {
                        label: STYLE_PART_NAMES[p].to_uppercase(),
                        value: Some(s.volumes[p]),
                        waiting: s.pickup & (1 << p) != 0,
                        position,
                        set: Some(AppCmd::Mixer(MixerCmd::SetStylePartVolume { part: i, volume: 0 })),
                    },
                }
            })
            .collect();
        let master_pos = known(shared.master_hw.load(Relaxed));
        faders.push(match &self.synth {
            Some(sy) => SurfaceFader {
                label: "MASTER".into(),
                value: Some(sy.control.master.load(Relaxed)),
                waiting: sy.control.master_waiting.load(Relaxed),
                position: master_pos,
                set: Some(AppCmd::Mixer(MixerCmd::SetMasterVolume { volume: 0 })),
            },
            None => SurfaceFader { position: master_pos, ..SurfaceFader::default() },
        });

        SurfaceState {
            shift: shared.shift.load(Relaxed),
            controls,
            faders,
            track_prev: neighbour(&self.published, self.cur, -1),
            track_next: neighbour(&self.published, self.cur, 1),
            clock: ClockState {
                at_ms: 0.0,
                running: s.running,
                tempo: s.bpm,
                beats_per_bar: self.info.quarters_per_bar,
                bar: 1,
                beat: 1,
                phase: 0.0,
                section_anchor_ms: ns_to_ms(s.anchor_ns),
                section_anchor_beats: s.anchor_beats,
                led_anchor_ms: ns_to_ms(self.led_ns),
                led_anchor_beats: self.led_beats,
            }
            .at(ns_to_ms(now)),
        }
    }

    fn build_state(&self, now: u64) -> AppState {
        let s = &self.snap;
        let shared = &self.shared;
        let kp = &shared.parts;
        let info = &self.info;
        let pnl = self.panel();
        let manual_bass_active = shared.manual_bass();
        let pads_of = |page: Page| -> Vec<Pad> {
            let p = Panel { page, ..pnl };
            let palette = if self.palette_leds { Some(launchkey::pad_leds(s, &info.has, &p)) } else { None };
            launchkey::looks(s, &info.has, &p)
                .iter()
                .enumerate()
                .map(|(i, (note, look))| Pad {
                    note: *note,
                    label: look.label.to_string(),
                    key: look.key.to_string(),
                    rgb: [look.rgb.0, look.rgb.1, look.rgb.2],
                    level: look.level,
                    anim: look.anim,
                    action: launchkey::pad_action(page, *note).map(AppCmd::from),
                    palette: palette.map(|leds| palette_led(leds[i].1)),
                })
                .collect()
        };
        let fader_hw = kp.fader_hw.each_ref().map(|a| known(a.load(Relaxed)));
        let upper = shared.upper.load(Relaxed);
        let fingering = Fingering::from_u8(shared.fingering.load(Relaxed));
        let split = shared.split.load(Relaxed);
        AppState {
            version: 0,
            style: StyleState {
                id: self.cur,
                path: info.path.display().to_string(),
                name: info.name.clone(),
                format: info.format.clone(),
                tempo: info.bpm,
                time_signature: [info.timesig.0, info.timesig.1],
                sections: (0..NUM_SLOTS).filter(|&i| info.has[i]).map(|i| id_of(i).name()).collect(),
            },
            transport: TransportState {
                running: s.running,
                sync_start: s.sync_armed,
                sync_stop: s.sync_stop,
                sync_stop_available: shared.sync_stop_allowed(),
                auto_fill: s.auto_fill,
                stop_acmp: s.stop_acmp,
                section: s.cur.map(|c| c.name()),
                queued: s.queued.map(|q| q.name()),
                pending_intro: s.pending_intro,
                main: s.main,
                bar: s.bar + 1,
                beat: s.beat + 1,
                beats_per_bar: info.timesig.0,
                section_bars: (s.section_bars > 0).then_some(s.section_bars),
                tempo: s.bpm,
                lamps: pads_of(Page::Sections),
            },
            chord: ChordState {
                name: s.chord.map(|c| c.name()),
                fingered: s.played.map(|c| c.name()),
                fingering,
                fingering_name: fingering.name().to_string(),
                upper,
                manual_bass: shared.manual_bass.load(Relaxed),
                manual_bass_active,
                split,
                split_name: note_name(split),
                transpose_keyboard: s.transpose.keyboard,
                transpose_master: s.transpose.master,
            },
            keyboard_parts: (0..parts::COUNT)
                .map(|p| {
                    let plays_bass = p == parts::LEFT && kp.manual_bass.load(Relaxed);
                    KeyboardPart {
                        name: parts::NAMES[p].to_string(),
                        channel: parts::CHANNEL[p] + 1,
                        on: kp.is_on(p),
                        sounding: if p == parts::LEFT { kp.left_sounds() } else { kp.is_on(p) },
                        selected: kp.selected() == p,
                        volume: kp.volume(p),
                        waiting: kp.waiting(p),
                        program: kp.program[p].load(Relaxed),
                        voice_name: gm_name(kp.channel_program(p)).to_string(),
                        plays_bass,
                        octave: kp.octave[p].load(Relaxed).clamp(-2, 2),
                        fader: fader_hw[p],
                    }
                })
                .collect(),
            mixer: MixerState {
                fader_page: kp.fader_page(),
                style_parts: (0..8u8)
                    .map(|p| {
                        // Manual Bass mutes the Style's Bass part (its voice moves to the left hand).
                        let mb = p == 2 && manual_bass_active;
                        let v = info.voices[8 + p as usize];
                        StylePart {
                            name: STYLE_PART_NAMES[p as usize].to_string(),
                            channel: 9 + p,
                            on: s.parts & (1 << p) != 0 && !mb,
                            muted_by_manual_bass: mb,
                            volume: s.volumes[p as usize],
                            waiting: s.pickup & (1 << p) != 0,
                            fader: fader_hw[p as usize],
                            voice: v.map(|(msb, lsb, program)| Voice {
                                bank_msb: msb,
                                bank_lsb: lsb,
                                program,
                                kit: msb >= 126 || p < 2,
                                label: voice_label(8 + p, v),
                            }),
                        }
                    })
                    .collect(),
                master: self.synth.as_ref().map(|s| s.control.master.load(Relaxed)),
                master_waiting: self.synth.as_ref().is_some_and(|s| s.control.master_waiting.load(Relaxed)),
            },
            pads: PadsState {
                page: pnl.page,
                page_name: pnl.page.name().to_string(),
                page_number: pnl.page.to_u8() + 1,
                page_count: Page::ALL.len() as u8,
                pads: pads_of(pnl.page),
                connected: self.pads_connected,
                palette_leds: self.palette_leds,
            },
            ots: OtsState {
                settings: info
                    .ots
                    .iter()
                    .take(4)
                    .enumerate()
                    .map(|(i, o)| OtsSetting {
                        name: format!("OTS {}", i + 1),
                        parts: o
                            .parts
                            .iter()
                            .map(|q| {
                                let program = q.voice.filter(|v| v.0 < 126).map(|v| v.2);
                                OtsPart {
                                    on: q.on,
                                    program,
                                    voice_name: program.map_or("drum kit", gm_name).to_string(),
                                    volume: q.volume,
                                    octave: q.octave,
                                }
                            })
                            .collect(),
                    })
                    .collect(),
                applied: kp.ots_applied.load(Relaxed),
                link: kp.ots_link.load(Relaxed),
            },
            library: LibraryStatus {
                // The revision `library()` has (published at most every 250 ms while indexing).
                revision: self.lib_published,
                count: self.published.count(),
                position: self.published.position(self.cur),
                pending: self.published.pending(),
                roots: self.roots.iter().map(|p| p.display().to_string()).collect(),
                scanning: self.scan_rx.is_some(),
            },
            surface: self.surface(&pnl, manual_bass_active, now),
            io: IoState {
                output_port: if self.offline.is_some() { String::new() } else { "yahaha".into() },
                inputs: self.inputs.clone(),
                synth: self.synth.as_ref().map(|sy| {
                    let c = sy.control.out_ch.load(Relaxed);
                    SynthState {
                        sound_font: sy.info.name.clone(),
                        device: sy.info.device.clone(),
                        sample_rate: sy.info.sample_rate,
                        buffer_frames: sy.info.buffer,
                        channels: sy.info.channels as u32,
                        output_pair: [c + 1, c + 2],
                        muted: sy.control.muted.load(Relaxed),
                    }
                }),
                engine: EngineStats {
                    realtime: shared.engine_rt.load(Relaxed),
                    wake_p99_us: shared.lateness.percentile_us(0.99) as u32,
                    chord_p99_us: shared.chord_lat.percentile_us(0.99) as u32,
                    midi_in_p99_us: shared.input_lat.percentile_us(0.99) as u32,
                },
                last_control: shared.last_daw.load(Relaxed),
                unmapped: unmapped_text(shared.last_unmapped.load(Relaxed)),
                offline: self.offline.is_some(),
                sources: self.sources.clone(),
                all_inputs: self.all_inputs,
                sound_fonts: self.sound_fonts.clone(),
                sound_font_file: self.synth.as_ref().and(self.sf_file.clone()),
                sound_font_loading: self.sf_load.is_some() || self.sf_ready.is_some(),
            },
            preview: PreviewState {
                audition: s.audition.map(|a| AuditionState {
                    id: a.id as usize,
                    bar: a.bar,
                    bars: a.bars,
                    chord: AUDITION_CHORDS.get(a.chord as usize).map(|c| c.to_string()),
                }),
                queued: self.pending_style.as_ref().map(|p| p.0),
            },
            keyboard: KeyboardState {
                held: (0..128u8)
                    .filter_map(|k| {
                        let v = shared.keys[k as usize].load(Relaxed);
                        (v & live::KEY_HELD != 0).then(|| HeldNote {
                            note: k,
                            zone: if v & live::KEY_RIGHT != 0 { Zone::Right } else { Zone::Left },
                            parts: (0..parts::COUNT as u8).filter(|p| v & (1 << p) != 0).collect(),
                        })
                    })
                    .collect(),
                left_split: split,
                chord_tones: s
                    .played
                    .filter(|c| c.ty != crate::theory::CANCEL)
                    .map(|c| crate::theory::chord_tones(c.ty).iter().map(|t| (c.root + t) % 12).collect())
                    .unwrap_or_default(),
                chord_bass: s.played.filter(|c| c.ty != crate::theory::CANCEL).map(|c| c.bass.unwrap_or(c.root)),
                detection: if !upper && fingering.full_keyboard() {
                    [0, 127]
                } else if upper {
                    [split.saturating_add(1).min(127), 127]
                } else {
                    [0, split]
                },
            },
            message: self.message.clone(),
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

/// A scanned library, its index results to come, and the first style that loads (its id).
type Opened = (Library, mpsc::Receiver<(usize, Info)>, usize, Box<Prepared>, Loaded);

/// Scan the library, start indexing it, and load the first style that loads.
fn open_library(paths: &[PathBuf]) -> Result<Opened> {
    // The folder walk is quick; the index (names, tempos) fills in on a background thread.
    let lib = Library::scan(paths);
    anyhow::ensure!(!lib.is_empty(), "no style files found");
    let index_rx = lib.spawn_indexer();
    for &id in lib.order() {
        match load(&lib.entry(id).path) {
            Ok((prep, info)) => return Ok((lib, index_rx, id, prep, info)),
            Err(e) if lib.len() == 1 => {
                return Err(e).with_context(|| format!("loading {}", lib.entry(id).path.display()));
            }
            Err(_) => {}
        }
    }
    anyhow::bail!("no style file loads")
}

/// Start the synth on a thread of its own, which keeps the audio stream (not `Send`)
/// until told to stop.
fn start_synth(sf2: &Path, consumers: Vec<Consumer<synth::Msg>>, audio_out: Option<u8>, parts: Arc<parts::Parts>) -> Result<(SynthRef, SynthThread)> {
    let (tx, rx) = mpsc::channel();
    let (stop, stop_rx) = mpsc::channel::<()>();
    let sf2 = sf2.to_path_buf();
    let thread = std::thread::Builder::new().name("yahaha-synth".into()).spawn(move || match synth::start(&sf2, consumers, audio_out, parts) {
        Ok(mut s) => {
            let swap = s.swap.take();
            let _ = tx.send(Ok(SynthRef { info: s.info.clone(), control: s.control.clone(), swap }));
            let _ = stop_rx.recv();
            drop(s);
        }
        Err(e) => {
            let _ = tx.send(Err(e));
        }
    })?;
    let r = rx.recv().context("synth thread")??;
    Ok((r, SynthThread { stop, thread }))
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
    let engine = Engine::new(prep);
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
    };
    let mut control = control;
    control.list_sound_fonts();
    Ok((shared, Assembled { control, engine: EngineLoopParts { engine, io: ch.io }, input }))
}

impl Session {
    /// Start a live session: open MIDI, start the synth (if `opts.sf2`), connect the
    /// keyboards and the Launchkey, start the engine and control threads.
    pub fn start(opts: Options) -> Result<Session> {
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
        let sources = midi::sources();
        let lk_daw = sources.iter().find(|(_, n)| is_daw(n));
        let mut io = MidiIo { port, slots: Default::default(), daw: None };
        if let Some((e, n)) = lk_daw.filter(|_| !opts.no_pads) {
            port.connect(*e, TAG_PADS)?;
            io.daw = Some((*e, n.clone()));
            p.control.pads_connected = true;
            if let Some((d, _)) = midi::destinations().into_iter().find(|(_, n)| n.contains("Launchkey") && n.contains("DAW")) {
                let out_port = client.output_port("yahaha leds")?;
                let mut s = PacketSink::new(Target::Port(out_port, d));
                s.push(&launchkey::ENTER_DAW);
                s.flush();
                p.control.leds = Some(Leds::new(s, opts.palette_leds));
            }
        }
        p.control.midi = Some(io);
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
        Ok(Session { inner, live: Mutex::new(Some(Live { client, _port: port, engine: engine_thread, control, synth: synth_thread })) })
    }

    /// An offline session: no MIDI, no audio, no threads. The engine runs on a virtual
    /// clock (starting at 0) that only `advance` moves; `midi_in` plays the keyboard and
    /// the Launchkey. `opts.sf2`, the input and audio options are ignored.
    pub fn offline(opts: Options) -> Result<Session> {
        let (band_tx, band) = RingBuffer::new(1 << 16);
        let (keys_tx, keys) = RingBuffer::new(1 << 12);
        let (shared, mut p) = assemble(
            &opts,
            live::Out::new(PacketSink::new(Target::Null), Some(band_tx)),
            live::Out::new(PacketSink::new(Target::Null), Some(keys_tx)),
            true,
        )?;
        p.control.pads_connected = true;
        p.control.inputs = vec!["(offline)".into()];
        let EngineLoopParts { engine, io } = p.engine;
        let engine = EngineLoop::new(engine, io, shared.clone());
        p.control.offline = Some(Offline { engine, input: p.input, now: 0, band, keys });
        if p.control.set_transpose(opts.transpose).is_err() {
            p.control.transpose = Transpose::default();
        }
        p.control.shared.parts.set_bass_program(synth::style_bass_program(p.control.info.voices[10]));
        p.control.sync_manual_bass();
        let inner = Arc::new(Inner::new(shared, p.control));
        let s = Session { inner, live: Mutex::new(None) };
        s.settle();
        Ok(s)
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

    // ----- offline -----

    /// Offline only: the virtual clock, in ns.
    pub fn now(&self) -> u64 {
        self.inner.lock().offline.as_ref().map_or(0, |o| o.now)
    }

    /// Offline only: move the virtual clock on by `ns`, running the engine at every
    /// deadline on the way, then publish the state.
    pub fn advance(&self, ns: u64) {
        {
            let mut ctl = self.inner.lock();
            let ctl = &mut *ctl;
            let Some(o) = ctl.offline.as_mut() else { return };
            let target = o.now.saturating_add(ns);
            // Every deadline on the way; a deadline that doesn't move is stepped past.
            let mut guard = 0u32;
            while let Some(d) = o.engine.next_deadline() {
                if d > target || guard > 1_000_000 {
                    break;
                }
                o.now = d.max(o.now);
                o.engine.step(o.now);
                while let Ok(s) = ctl.snap_rx.pop() {
                    ctl.snap = s;
                }
                if o.engine.next_deadline() == Some(d) {
                    o.now += 1;
                }
                guard += 1;
            }
            o.now = target;
        }
        self.settle();
    }

    /// Offline only: MIDI arriving on `port` (running status and several messages per
    /// call are fine), handled exactly as the CoreMIDI thread handles it live.
    pub fn midi_in(&self, port: Port, bytes: &[u8]) {
        {
            let mut ctl = self.inner.lock();
            let Some(o) = ctl.offline.as_mut() else { return };
            let tag = match port {
                Port::Keys => TAG_KEYS,
                Port::Pads => TAG_PADS,
            };
            o.input.packet(tag, 0, bytes);
            o.input.end_of_list();
        }
        self.settle();
    }

    /// Offline only: everything played since the last call, as the built-in synth gets
    /// it (channel messages): the band's output, then the keyboard parts'.
    pub fn take_output(&self) -> Vec<[u8; 3]> {
        let mut ctl = self.inner.lock();
        let Some(o) = ctl.offline.as_mut() else { return Vec::new() };
        let mut v: Vec<[u8; 3]> = std::iter::from_fn(|| o.band.pop().ok()).collect();
        v.extend(std::iter::from_fn(|| o.keys.pop().ok()));
        v
    }

    /// Offline only: wait for the library index to finish (it runs on a thread).
    pub fn finish_indexing(&self) {
        let mut ctl = self.inner.lock();
        if let Some(rx) = ctl.index_rx.take() {
            for (id, info) in rx.iter() {
                if id < ctl.lib.len() && !matches!(ctl.lib.entry(id).info, Info::Err(_)) {
                    ctl.lib.set_info(id, info);
                }
            }
            ctl.lib.sort();
            ctl.lib_rev += 1;
        }
        drop(ctl);
        self.settle();
    }

    /// Offline only: show this engine snapshot, as if the engine had sent it (the
    /// `yahaha screen` layout check).
    pub fn show_snapshot(&self, snap: Snapshot) {
        let mut ctl = self.inner.lock();
        ctl.snap = snap;
        let now = ctl.offline.as_ref().map_or(0, |o| o.now);
        self.inner.publish(&mut ctl, now);
    }

    /// Offline: run the engine at the current time, then pump and publish.
    fn settle(&self) {
        let mut ctl = self.inner.lock();
        let ctl = &mut *ctl;
        let Some(o) = ctl.offline.as_mut() else { return };
        let now = o.now;
        o.engine.step(now);
        ctl.drain_snapshots();
        ctl.pump(now);
        // Launchkey actions may have sent the engine commands: run them too.
        if let Some(o) = ctl.offline.as_mut() {
            o.engine.step(now);
        }
        ctl.drain_snapshots();
        self.inner.publish(ctl, now);
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

/// A fader position as last reported (`HW_UNKNOWN` = never).
fn known(hw: u8) -> Option<u8> {
    (hw != crate::engine::HW_UNKNOWN).then_some(hw)
}

/// What a pad shows in palette mode.
fn palette_led(led: Led) -> PaletteLed {
    let look = |c: u8| {
        let (rgb, level) = launchkey::palette_colour(c);
        ([rgb.0, rgb.1, rgb.2], level)
    };
    let (mode, c, flash) = match led {
        Led::Solid(c) => (launchkey::Anim::Solid, c, None),
        Led::Flash(a, b) => (launchkey::Anim::Flash, a, Some(b)),
        Led::Pulse(c) => (launchkey::Anim::Pulse, c, None),
    };
    let (rgb, level) = look(c);
    PaletteLed {
        mode,
        colour: c,
        rgb,
        level,
        flash_colour: flash,
        flash_rgb: flash.map(|f| look(f).0),
        flash_level: flash.map(|f| look(f).1),
    }
}

/// The style `StepStyle { delta }` would load, if it goes anywhere.
fn neighbour(lib: &Library, cur: usize, delta: i8) -> Option<Neighbour> {
    if cur >= lib.len() {
        return None;
    }
    let id = lib.step(cur, delta);
    (id != cur).then(|| {
        let e = lib.entry(id);
        Neighbour { id, name: e.name().to_string(), path: e.path.display().to_string() }
    })
}

/// A library entry as plain data.
pub fn library_entry(lib: &Library, id: usize) -> LibraryEntry {
    let e = lib.entry(id);
    let (status, error, tempo, ts, sections) = match &e.info {
        Info::Pending => ("pending", None, None, None, String::new()),
        Info::Ok(s) => ("ok", None, Some(s.bpm), Some([s.timesig.0, s.timesig.1]), library::sections_text(&s.sections)),
        Info::Err(err) => ("error", Some(err.clone()), None, None, String::new()),
    };
    let format = match &e.info {
        Info::Ok(s) if !s.format.is_empty() => Some(s.format.clone()),
        _ => None,
    };
    LibraryEntry {
        id,
        name: e.name().to_string(),
        folder: e.folder.clone(),
        path: e.path.display().to_string(),
        status: status.into(),
        error,
        tempo,
        time_signature: ts,
        sections,
        format,
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
