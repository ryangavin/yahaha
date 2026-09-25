//! An offline session: no MIDI, no audio, no threads. The engine runs on a virtual clock
//! that `Session::advance` moves; `Session::midi_in` plays the keyboard and the Launchkey.
//! Tests and the app's dev mode use it.

use super::{assemble, EngineLoopParts, Inner, Options, Port, Session};
use crate::engine::{Snapshot, Transpose};
use crate::library::Info;
use crate::live::{self, EngineLoop, Input, TAG_KEYS, TAG_PADS};
use crate::midi::InputHandler;
use crate::rt::{PacketSink, Target};
use crate::synth;
use anyhow::Result;
use rtrb::{Consumer, RingBuffer};
use std::sync::{Arc, Mutex};

/// An offline session's engine, input and clock.
pub(super) struct Offline {
    pub(super) engine: EngineLoop,
    pub(super) input: Input,
    pub(super) now: u64,
    /// What the band and the keyboard parts played, as the synth would get it.
    pub(super) band: Consumer<[u8; 3]>,
    pub(super) keys: Consumer<[u8; 3]>,
    /// The synth, rendered by `Session::render` (`offline_audio`).
    pub(super) audio: Option<Box<synth::AudioCore>>,
}

impl Session {
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
        p.control.offline = Some(Offline { engine, input: p.input, now: 0, band, keys, audio: None });
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

    /// Offline only: [`Session::take_output`] kept apart: (what the engine thread sent, what
    /// the input thread sent). Within one `midi_in`, the input thread's messages came
    /// first; an `advance` or a `send` has only the engine's.
    pub fn take_output_split(&self) -> (Vec<[u8; 3]>, Vec<[u8; 3]>) {
        let mut ctl = self.inner.lock();
        let Some(o) = ctl.offline.as_mut() else { return Default::default() };
        (std::iter::from_fn(|| o.band.pop().ok()).collect(), std::iter::from_fn(|| o.keys.pop().ok()).collect())
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

    /// Offline only: give the session the built-in synth, rendered by [`Session::render`]
    /// instead of an audio device (the same `synth::AudioCore` the device runs). `sf2`
    /// None: no SoundFont (plugin parts only). From here on the band and the keys play into
    /// it, so `take_output` returns nothing. `meters` and plugin parts work as live.
    pub fn offline_audio(&self, sf2: Option<&std::path::Path>, sample_rate: u32) -> Result<()> {
        {
            let mut ctl = self.inner.lock();
            let ctl = &mut *ctl;
            let Some(o) = ctl.offline.as_mut() else { anyhow::bail!("not an offline session") };
            // The rack plays the sound library's program map (#103), as a live synth's.
            let main = sf2.and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string());
            let font_id = main.as_deref().and_then(|m| ctl.sound.font_id(m)).unwrap_or(0);
            let rack = match sf2 {
                Some(p) => {
                    let font = Arc::new(rustysynth::SoundFont::new(&mut std::fs::File::open(p)?).map_err(|e| anyhow::anyhow!("{e:?}"))?);
                    Some(Box::new(synth::Rack::with_fonts(&[(font_id, font)], sample_rate as i32)?))
                }
                None => None,
            };
            let band = std::mem::replace(&mut o.band, RingBuffer::new(1).1);
            let keys = std::mem::replace(&mut o.keys, RingBuffer::new(1).1);
            // The control side's ring (library auditions), as a live synth has it.
            let (audition_tx, auditions) = RingBuffer::new(256);
            ctl.sound.audition_tx = Some(audition_tx);
            let control = Arc::new(synth::SynthControl::new(0));
            let (mut core, swap, plugins) = synth::AudioCore::new(rack, vec![band, keys, auditions], ctl.shared.parts.clone(), control.clone(), sample_rate, 2);
            core.set_routes(ctl.shared.routes.clone());
            if let Some(m) = &main {
                ctl.sound.synth_started(m);
            }
            o.audio = Some(Box::new(core));
            o.input.set_synth(Some(control.clone()));
            let name = sf2.and_then(|p| p.file_stem()).map_or_else(String::new, |n| n.to_string_lossy().to_string());
            ctl.synth = Some(super::SynthRef {
                info: synth::SynthInfo { name, sample_rate, buffer: None, device: "(offline)".into(), channels: 2 },
                control,
                swap: Some(swap),
                plugins,
            });
        }
        self.settle();
        Ok(())
    }

    /// Offline only, after [`Session::offline_audio`]: render `frames` of stereo (in
    /// buffers of 64, as a device would), then pump and publish. Empty without a synth.
    pub fn render(&self, frames: usize) -> (Vec<f32>, Vec<f32>) {
        let (mut l, mut r) = (Vec::with_capacity(frames), Vec::with_capacity(frames));
        {
            let mut ctl = self.inner.lock();
            let Some(core) = ctl.offline.as_mut().and_then(|o| o.audio.as_mut()) else { return (l, r) };
            let mut buf = [0f32; 128];
            let mut done = 0;
            while done < frames {
                let n = (frames - done).min(64);
                core.process(&mut buf[..n * 2]);
                for f in buf[..n * 2].chunks(2) {
                    l.push(f[0]);
                    r.push(f[1]);
                }
                done += n;
            }
        }
        self.settle();
        (l, r)
    }

    /// Offline: run the engine at the current time, then pump and publish.
    pub(super) fn settle(&self) {
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
