//! Settings: audio output, SoundFont, MIDI inputs, Launchkey LED mode; the synth and MIDI
//! as the state shows them.

use super::{Control, SynthThread};
use crate::api::{unmapped_text, CmdError, EngineStats, IoState, MidiSource, SettingsCmd, SynthState};
use crate::library;
use crate::live::{self, Cmd, MAX_KEY_SOURCES};
use crate::midi;
use crate::parts;
use crate::synth::{self, SynthControl, SynthInfo};
use anyhow::{Context, Result};
use rtrb::Consumer;
use std::path::Path;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{mpsc, Arc};

/// The synth as the control side sees it.
pub(super) struct SynthRef {
    pub(super) info: SynthInfo,
    pub(super) control: Arc<SynthControl>,
    /// Swapping SoundFonts (None: the synth can't).
    pub(super) swap: Option<synth::RackSwap>,
}

/// What the SoundFont loader thread sends back.
pub(super) type RackLoad = Result<Box<synth::Rack>, String>;

/// A live session's MIDI input: the port, and which sources it listens to.
pub(super) struct MidiIo {
    pub(super) port: midi::InputPort,
    /// Keyboard sources by slot (`live::key_tag`; slot 0 is unused live).
    pub(super) slots: [Option<(midi::Endpoint, String)>; MAX_KEY_SOURCES],
    /// The Launchkey DAW port, when yahaha drives it.
    pub(super) daw: Option<(midi::Endpoint, String)>,
    /// The output port the Launchkey LEDs go out through (None with `--no-pads`).
    pub(super) leds_port: Option<midi::OutPort>,
    /// `--no-pads`: leave the Launchkey DAW port alone.
    pub(super) no_pads: bool,
    /// The MIDI setup generation last followed (`midi::setup_generation`,
    /// session/devices.rs).
    pub(super) setup_gen: u64,
}

/// The Launchkey's DAW port (pads, buttons, faders), by its source name.
pub(super) fn is_daw(name: &str) -> bool {
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

/// Start the synth on a thread of its own, which keeps the audio stream (not `Send`)
/// until told to stop.
pub(super) fn start_synth(sf2: &Path, consumers: Vec<Consumer<synth::Msg>>, audio_out: Option<u8>, parts: Arc<parts::Parts>) -> Result<(SynthRef, SynthThread)> {
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

impl Control {
    pub(super) fn settings_cmd(&mut self, c: SettingsCmd) -> Result<(), CmdError> {
        match c {
            SettingsCmd::SetSoundFont { file } => return self.set_sound_font(file),
            SettingsCmd::SetMidiInputs { all, names } => {
                self.all_inputs = all;
                self.input_names = names;
                self.connect_inputs();
            }
            SettingsCmd::SetPaletteLeds { on } => {
                self.palette_leds = on;
                if let Some(l) = self.leds.as_mut() {
                    l.set_palette(on);
                }
            }
            SettingsCmd::SetAudioOutput { first } => {
                if let Some(s) = &self.synth {
                    let n = s.info.channels.max(2) as u8;
                    s.control.out_ch.store(first.min(n - 2), Relaxed);
                }
            }
            // Next stereo output pair: 1/2 -> 3/4 -> ... -> back to 1/2.
            SettingsCmd::NextAudioOutput => {
                if let Some(s) = &self.synth {
                    let n = s.info.channels.max(2) as u8;
                    let c = s.control.out_ch.load(Relaxed);
                    s.control.out_ch.store(if c + 4 <= n { c + 2 } else { 0 }, Relaxed);
                }
            }
        }
        Ok(())
    }

    /// The SoundFonts in the synth's folder.
    pub(super) fn list_sound_fonts(&mut self) {
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
    pub(super) fn pump_sound_font(&mut self) {
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

    /// Connect the keyboard sources `all_inputs`/`input_names` choose and disconnect the
    /// rest (and those that went offline); list every source online. A source dropped
    /// with keys held has them released.
    pub(super) fn connect_inputs(&mut self) {
        let Some(m) = self.midi.as_mut() else { return };
        let sources = midi::online_sources();
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
            // A source that moved a pedal or wheel is reset too: its release never comes.
            if self.shared.src_held[slot].load(Relaxed) > 0 || self.shared.controllers.touched(slot) {
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

    pub(super) fn io_state(&self) -> IoState {
        let shared = &self.shared;
        IoState {
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
        }
    }
}
