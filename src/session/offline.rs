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
    /// it (channel messages, and a fade's Master Volume as `synth::master_volume_msg`):
    /// the band's output, then the keyboard parts'.
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
