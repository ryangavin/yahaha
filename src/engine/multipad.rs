//! Multi Pads in the engine (docs/genos-features.md §7, docs/multipad.md): the bank's
//! player (`multipad::MultiPadPlayer`, built on the control side and swapped in), its own
//! tick clock, and what the band does to it (Synchro Start, Synchro Stop, the next-measure
//! start).
//!
//! **The clock.** Pads run on a clock of their own at a fixed [`PAD_PPQ`], not on the style's
//! ticks: a style's resolution can change under a playing pad (a style change from 480 to
//! 1920 ppq), and pads also play while the band is stopped. The clock is a monotonic tick
//! count that moves at the band's tempo (`Engine::bpm`); a tempo change re-anchors it at the
//! wake that sees it, so a pad keeps its place and only its speed changes.
//!
//! **Timing.** Stopped, a pad starts at once. While the band plays it starts on the style's
//! next bar line (at once exactly on one), mapped to the pad clock; a repeating pad then
//! loops on its own length, so a one-bar pad stays on the bar at a steady tempo.
//!
//! **The chord.** Chord Match follows the chord the style follows (`Engine::chord`: the
//! chord section, after Keyboard transpose). yahaha has no ACMP switch (the chord section
//! is always read), so the Genos case "ACMP off: the LEFT section's chord" is the same chord
//! here.
//!
//! Everything here runs on the engine thread: no allocation (the player never allocates in
//! `process`, and banks come in and go out as `Box`es through `live`'s rings).

use super::*;
use crate::multipad::{sync_fires, MultiPadPlayer, PadState, SyncTrigger, PADS};

/// Ticks per quarter note of the pad clock (the Tyros pad resolution).
pub const PAD_PPQ: u32 = 1920;

/// Multi Pad Synchro Stop (Style Setting, RM p.12): whether repeating pads stop when the
/// Style stops, and when an Ending starts. One-shot pads always play out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SynchroStop {
    pub style_stop: bool,
    pub ending: bool,
}

impl Default for SynchroStop {
    /// Style Stop on (the Owner's Manual: START/STOP "also stops playback of the Multi
    /// Pad(s)"), Style Ending off (repeating pads play on through the Ending until the band
    /// stops). The manual gives no defaults; see docs/multipad.md.
    fn default() -> SynchroStop {
        SynchroStop { style_stop: true, ending: false }
    }
}

/// A Multi Pad command for the engine thread (`live::Cmd::MultiPad`). Pads are 0-based.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PadCmd {
    /// Press a pad: it starts (or restarts) now, or at the next bar line while the band
    /// plays. Pads in Synchro Start standby start with it.
    Trigger(u8),
    /// STOP + pad: stop that pad now.
    Stop(u8),
    /// STOP: every pad stops, and Synchro Start standby is cancelled.
    StopAll,
    /// SELECT + pad: toggle the pad's Synchro Start standby.
    Arm(u8),
    /// Panel override of a pad's Repeat flag.
    Repeat(u8, bool),
    /// Panel override of a pad's Chord Match flag.
    ChordMatch(u8, bool),
    SynchroStop(SynchroStop),
}

/// The pads as the engine last saw them, for the session (`Snapshot::multipad`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PadsSnap {
    /// The bank playing (`live::PadBank::tag`; 0 = none).
    pub tag: u64,
    pub states: [PadState; PADS],
    pub repeat: [bool; PADS],
    pub chord_match: [bool; PADS],
    pub synchro: SynchroStop,
}

impl Default for PadsSnap {
    fn default() -> PadsSnap {
        PadsSnap {
            tag: 0,
            states: [PadState::Empty; PADS],
            repeat: [false; PADS],
            chord_match: [false; PADS],
            synchro: SynchroStop::default(),
        }
    }
}

/// The engine's Multi Pad state (one field of `hooks::Features`).
#[derive(Default)]
pub(super) struct PadDeck {
    player: Option<Box<MultiPadPlayer>>,
    tag: u64,
    synchro: SynchroStop,
    /// The pad clock: at `anchor_ns` it read `anchor_tick`, moving on at `bpm` (0: not
    /// started yet).
    bpm: f64,
    anchor_ns: u64,
    anchor_tick: f64,
    /// Pad ticks played up to (exclusive).
    done: u64,
}

impl PadDeck {
    #[inline]
    fn ns_per_tick(&self) -> f64 {
        60e9 / (self.bpm.max(1.0) * PAD_PPQ as f64)
    }

    /// The pad clock at `now`, fractional.
    #[inline]
    fn tick_f(&self, now: u64) -> f64 {
        self.anchor_tick + now.saturating_sub(self.anchor_ns) as f64 / self.ns_per_tick()
    }

    /// The pad tick `now` falls in (a hair of float error rounds up, so a wake at a
    /// deadline always reaches the tick it was set for).
    #[inline]
    fn tick(&self, now: u64) -> u64 {
        (self.tick_f(now) + 1e-6).floor().max(0.0) as u64
    }

    /// When pad tick `t` comes.
    #[inline]
    fn ns_at(&self, t: u64) -> u64 {
        self.anchor_ns + ((t as f64 - self.anchor_tick).max(0.0) * self.ns_per_tick()).ceil() as u64
    }
}

impl Engine {
    /// The pad clock follows the band's tempo: start it, or re-anchor it after a change.
    fn pad_clock(&mut self, now: u64) {
        let bpm = self.bpm;
        let d = &mut self.features.pads;
        if d.bpm == 0.0 {
            d.bpm = bpm;
            d.anchor_ns = now;
            d.anchor_tick = 0.0;
        } else if d.bpm != bpm {
            d.anchor_tick = d.tick_f(now);
            d.anchor_ns = now;
            d.bpm = bpm;
        }
    }

    /// The pad tick a press at `now` starts on: now when the band is stopped, else the
    /// style's next bar line (now when exactly on one).
    fn pad_start(&self, now: u64) -> u64 {
        let d = &self.features.pads;
        let t = d.tick(now);
        if !self.running {
            return t;
        }
        let tpb = self.style.tpb.max(1) as f64;
        let bars = ((self.tick_at(now) - self.sec_start) / tpb).max(0.0);
        let n = if (bars - bars.round()).abs() < 1e-6 { bars.round() } else { bars.ceil() };
        let line = self.ns_at(self.sec_start + n * tpb);
        (d.tick_f(line).round().max(0.0) as u64).max(t)
    }

    /// A new bank (None: none) from the control side, tagged `tag`: the old one's notes
    /// end, and it comes back to be freed off this thread.
    pub fn load_pads(
        &mut self,
        player: Option<Box<MultiPadPlayer>>,
        tag: u64,
        now: u64,
        sink: &mut impl Sink,
    ) -> Option<Box<MultiPadPlayer>> {
        self.pad_clock(now);
        let d = &mut self.features.pads;
        let mut old = std::mem::replace(&mut d.player, player);
        if let Some(o) = old.as_mut() {
            o.stop_all(sink);
        }
        d.tag = tag;
        old
    }

    /// A Multi Pad command at `now`. What it starts at once plays straight away.
    pub fn pad_cmd(&mut self, c: PadCmd, now: u64, sink: &mut impl Sink) {
        self.pad_clock(now);
        let start = self.pad_start(now);
        let d = &mut self.features.pads;
        if let PadCmd::SynchroStop(s) = c {
            d.synchro = s;
            return;
        }
        let Some(p) = d.player.as_mut() else { return };
        match c {
            PadCmd::Trigger(i) => {
                // Pressing any pad starts every pad in standby with it (OM p.75).
                if p.any_armed() {
                    p.fire_sync(start);
                }
                p.trigger(i as usize, start);
            }
            PadCmd::Stop(i) => p.stop(i as usize, sink),
            PadCmd::StopAll => p.stop_all(sink),
            PadCmd::Arm(i) => {
                p.arm(i as usize);
            }
            PadCmd::Repeat(i, on) => p.set_repeat(i as usize, on),
            PadCmd::ChordMatch(i, on) => p.set_chord_match(i as usize, on),
            PadCmd::SynchroStop(_) => {}
        }
        self.process_pads(now, sink);
    }

    /// Play the pads up to `now`. The engine loop calls it on every wake, the band running
    /// or not.
    pub fn process_pads(&mut self, now: u64, sink: &mut impl Sink) {
        self.pad_clock(now);
        let chord = self.chord;
        let d = &mut self.features.pads;
        let end = d.tick(now) + 1;
        let Some(p) = d.player.as_mut() else { return };
        // Always run it, even with no new tick: a press on the tick already played starts
        // now (the player plays whatever is due before the range's end).
        p.process(d.done.min(end)..end, chord, sink);
        d.done = d.done.max(end);
    }

    /// When the pads next need `process_pads`, if anything plays or waits.
    pub fn pads_deadline(&self) -> Option<u64> {
        let d = &self.features.pads;
        let t = d.player.as_ref()?.next_due()?;
        Some(d.ns_at(t))
    }

    pub(super) fn pads_snapshot(&self) -> PadsSnap {
        let d = &self.features.pads;
        let mut s = PadsSnap { tag: d.tag, synchro: d.synchro, ..PadsSnap::default() };
        if let Some(p) = d.player.as_ref() {
            for i in 0..PADS {
                s.states[i] = p.state(i);
                s.repeat[i] = p.repeat(i);
                s.chord_match[i] = p.chord_match(i);
            }
        }
        s
    }

    // ----- from the hooks -----

    /// The band started: pads in Synchro Start standby start with it.
    pub(super) fn pads_on_start(&mut self, now: u64) {
        self.pads_sync(SyncTrigger::StyleStart, now);
    }

    /// A chord in the chord section releases pads in standby (at the next bar line while
    /// the band plays).
    pub(super) fn pads_on_chord(&mut self, now: u64) {
        if self.chord.is_some_and(|c| c.ty != CANCEL) {
            self.pads_sync(SyncTrigger::Chord, now);
        }
    }

    fn pads_sync(&mut self, trigger: SyncTrigger, now: u64) {
        // No ACMP switch: the chord section is always on (module docs).
        if !sync_fires(trigger, true) || !self.features.pads.player.as_ref().is_some_and(|p| p.any_armed()) {
            return;
        }
        self.pad_clock(now);
        let start = self.pad_start(now);
        if let Some(p) = self.features.pads.player.as_mut() {
            p.fire_sync(start);
        }
    }

    /// The band stopped: Multi Pad Synchro Stop (Style Stop).
    pub(super) fn pads_on_stop(&mut self, sink: &mut impl Sink) {
        let d = &mut self.features.pads;
        if d.synchro.style_stop {
            if let Some(p) = d.player.as_mut() {
                p.stop_repeating(sink);
            }
        }
    }

    /// A section change to slot `to`: an Ending starting is Multi Pad Synchro Stop (Style
    /// Ending).
    pub(super) fn pads_before_section(&mut self, to: usize, sink: &mut impl Sink) {
        let ending = |s: usize| s < NUM_SLOTS && matches!(id_of(s), SectionId::Ending(_));
        if !ending(to) || ending(self.cur) {
            return;
        }
        let d = &mut self.features.pads;
        if d.synchro.ending {
            if let Some(p) = d.player.as_mut() {
                p.stop_repeating(sink);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multipad::{synthetic, PadBank};

    #[derive(Default)]
    struct Rec(Vec<(u64, [u8; 3])>, u64);
    impl Sink for Rec {
        fn send(&mut self, m: &[u8]) {
            let mut b = [0u8; 3];
            let n = m.len().min(3);
            b[..n].copy_from_slice(&m[..n]);
            self.0.push((self.1, b));
        }
    }
    impl Rec {
        /// Note-ons on channel `ch` (0-based): (time, key).
        fn ons(&self, ch: u8) -> Vec<(u64, u8)> {
            self.0.iter().filter(|(_, m)| m[0] == 0x90 | ch && m[2] > 0).map(|&(t, m)| (t, m[1])).collect()
        }
        fn offs(&self, ch: u8) -> usize {
            self.0.iter().filter(|(_, m)| m[0] == 0x80 | ch || (m[0] == 0x90 | ch && m[2] == 0)).count()
        }
    }

    fn engine() -> Option<Engine> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return None;
        }
        Some(Engine::new(Box::new(Prepared::new(&Style::load(&p).unwrap()))))
    }

    fn demo() -> Box<MultiPadPlayer> {
        Box::new(MultiPadPlayer::new(&crate::multipad::file::parse(&synthetic::demo_bank()).unwrap(), PAD_PPQ))
    }

    /// Step the engine (band and pads) from `from` to `to` in 1 ms wakes, as the engine
    /// loop does.
    fn run(e: &mut Engine, rec: &mut Rec, from: u64, to: u64) {
        let mut t = from;
        while t <= to {
            rec.1 = t;
            e.process(t, rec);
            e.process_pads(t, rec);
            t += 1_000_000;
        }
    }

    fn bar_ns(e: &Engine) -> u64 {
        e.ns_at_bar(1) - e.ns_at_bar(0)
    }

    #[test]
    fn stopped_a_pad_starts_at_once_and_a_one_shot_ends() {
        let Some(mut e) = engine() else { return };
        let mut rec = Rec::default();
        assert!(e.load_pads(Some(demo()), 1, 0, &mut rec).is_none());
        e.pad_cmd(PadCmd::Trigger(1), 5_000_000, &mut rec);
        let ons = rec.ons(5);
        assert_eq!(ons.first(), Some(&(0, 60)), "Rise Arp starts at once on channel 6: {ons:?}");
        let bar = 60e9 / e.bpm * 4.0;
        run(&mut e, &mut rec, 5_000_000, 5_000_000 + 2 * bar as u64);
        assert_eq!(rec.ons(5).len(), 8, "one pass of 8 notes, then it ends");
        assert_eq!(rec.offs(5), 8);
        assert_eq!(e.pads_snapshot().states[1], PadState::Ready);
        assert_eq!(e.pads_deadline(), None);
    }

    #[test]
    fn while_the_band_plays_a_pad_waits_for_the_next_bar_line() {
        let Some(mut e) = engine() else { return };
        let mut rec = Rec::default();
        e.load_pads(Some(demo()), 1, 0, &mut rec);
        e.set_chord(crate::parse_chord("C").unwrap(), 0, &mut rec);
        assert!(e.is_running());
        let bar = bar_ns(&e);
        let press = bar + bar / 3;
        run(&mut e, &mut rec, 0, press);
        rec.1 = press;
        e.pad_cmd(PadCmd::Trigger(2), press, &mut rec);
        assert_eq!(e.pads_snapshot().states[2], PadState::Queued);
        run(&mut e, &mut rec, press, 3 * bar + bar / 2);
        let first = rec.ons(6)[0].0;
        let line = e.ns_at_bar(2);
        assert!(first.abs_diff(line) <= 1_000_000, "starts on the bar line: {first} vs {line}");
        // Bass Riff repeats: its next pass starts on the next bar line too.
        let next = e.ns_at_bar(3);
        let second_pass = rec.ons(6).iter().find(|(t, _)| *t + 1_000_000 >= next).unwrap().0;
        assert!(second_pass.abs_diff(next) <= 1_000_000, "{second_pass} vs {next}");
    }

    #[test]
    fn chord_match_follows_the_style_chord() {
        let Some(mut e) = engine() else { return };
        let mut rec = Rec::default();
        e.load_pads(Some(demo()), 1, 0, &mut rec);
        // Stopped, no Sync Start: the chord only goes to Chord Match.
        e.button(Button::SyncStart, 0, &mut rec);
        e.set_chord(crate::parse_chord("F").unwrap(), 0, &mut rec);
        assert!(!e.is_running());
        e.pad_cmd(PadCmd::Trigger(1), 0, &mut rec);
        run(&mut e, &mut rec, 0, 2_000_000_000);
        // The CM7 source's C E G over F: F A C.
        let keys: Vec<u8> = rec.ons(5).iter().take(3).map(|x| x.1 % 12).collect();
        assert_eq!(keys, [5, 9, 0]);
    }

    #[test]
    fn synchro_start_fires_on_a_chord_and_on_style_start() {
        let Some(mut e) = engine() else { return };
        let mut rec = Rec::default();
        e.load_pads(Some(demo()), 1, 0, &mut rec);
        e.pad_cmd(PadCmd::Arm(0), 0, &mut rec);
        e.pad_cmd(PadCmd::Arm(3), 0, &mut rec);
        assert_eq!(e.pads_snapshot().states[0], PadState::Armed);
        // Sync Start is armed on a new engine: the chord starts the band, and the pads with it.
        e.set_chord(crate::parse_chord("C").unwrap(), 10_000_000, &mut rec);
        rec.1 = 10_000_000;
        e.process_pads(10_000_000, &mut rec);
        assert_eq!(e.pads_snapshot().states[0], PadState::Playing);
        assert_eq!(e.pads_snapshot().states[3], PadState::Playing);
        assert!(!rec.ons(4).is_empty() && !rec.ons(7).is_empty());

        // Band stopped, a pad armed, Sync Start off: a chord fires it.
        e.button(Button::StartStop, 20_000_000, &mut rec);
        e.pad_cmd(PadCmd::StopAll, 20_000_000, &mut rec);
        e.pad_cmd(PadCmd::Arm(1), 20_000_000, &mut rec);
        e.set_chord(crate::parse_chord("G").unwrap(), 30_000_000, &mut rec);
        e.process_pads(30_000_000, &mut rec);
        assert!(!e.is_running());
        assert_eq!(e.pads_snapshot().states[1], PadState::Playing);
    }

    #[test]
    fn synchro_stop_on_style_stop_and_on_ending() {
        let Some(mut e) = engine() else { return };
        let mut rec = Rec::default();
        e.load_pads(Some(demo()), 1, 0, &mut rec);
        e.set_chord(crate::parse_chord("C").unwrap(), 0, &mut rec);
        e.pad_cmd(PadCmd::Trigger(0), 0, &mut rec);
        e.pad_cmd(PadCmd::Trigger(2), 0, &mut rec);
        let bar = bar_ns(&e);
        run(&mut e, &mut rec, 0, bar / 2);
        // Default: Style Stop on.
        e.button(Button::StartStop, bar / 2, &mut rec);
        let s = e.pads_snapshot();
        assert_eq!((s.states[0], s.states[2]), (PadState::Ready, PadState::Ready));

        // Style Stop off: the pads play on after the band stops.
        e.pad_cmd(PadCmd::SynchroStop(SynchroStop { style_stop: false, ending: true }), bar, &mut rec);
        e.set_chord(crate::parse_chord("C").unwrap(), bar, &mut rec);
        e.button(Button::SyncStart, bar, &mut rec);
        e.button(Button::StartStop, bar, &mut rec);
        assert!(e.is_running());
        e.pad_cmd(PadCmd::Trigger(0), bar, &mut rec);
        run(&mut e, &mut rec, bar, bar + bar / 2);
        assert_eq!(e.pads_snapshot().states[0], PadState::Playing);
        // Ending on: the Ending stops the repeating pad at its first bar line.
        e.button(Button::Ending(0), bar + bar / 2, &mut rec);
        run(&mut e, &mut rec, bar + bar / 2, 2 * bar + bar / 2);
        assert_eq!(e.pads_snapshot().states[0], PadState::Ready);
    }

    #[test]
    fn a_tempo_change_keeps_the_pad_in_place_and_changes_its_speed() {
        let Some(mut e) = engine() else { return };
        let mut rec = Rec::default();
        e.load_pads(Some(demo()), 1, 0, &mut rec);
        e.pad_cmd(PadCmd::Trigger(1), 0, &mut rec);
        let eighth = (60e9 / e.bpm / 2.0) as u64;
        run(&mut e, &mut rec, 0, eighth + eighth / 2);
        assert_eq!(rec.ons(5).len(), 2);
        for _ in 0..10 {
            e.button(Button::TempoUp, eighth + eighth / 2, &mut rec);
        }
        let faster = (60e9 / e.bpm / 2.0) as u64;
        run(&mut e, &mut rec, eighth + eighth / 2, 2 * eighth + faster);
        let ons = rec.ons(5);
        assert!(ons.len() >= 3, "{ons:?}");
        let expect = eighth + eighth / 2 + faster / 2;
        assert!(ons[2].0.abs_diff(expect) <= 1_500_000, "{} vs {expect}", ons[2].0);
    }

    #[test]
    fn a_new_bank_ends_the_old_ones_notes_and_hands_it_back() {
        let Some(mut e) = engine() else { return };
        let mut rec = Rec::default();
        e.load_pads(Some(demo()), 1, 0, &mut rec);
        e.pad_cmd(PadCmd::Trigger(3), 0, &mut rec);
        assert_eq!(rec.ons(7).len(), 4);
        let old = e.load_pads(None, 2, 1, &mut rec);
        assert!(old.is_some());
        assert_eq!(rec.offs(7), 4);
        assert_eq!(e.pads_snapshot().tag, 2);
        assert_eq!(e.pads_snapshot().states, [PadState::Empty; PADS]);
    }

    #[test]
    fn a_parsed_bank_is_a_pad_bank() {
        let b: PadBank = crate::multipad::file::parse(&synthetic::demo_bank()).unwrap();
        assert_eq!(b.pads.iter().flatten().count(), 4);
    }
}
