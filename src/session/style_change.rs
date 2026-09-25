//! Style Setting > Change Behavior: the control side keeps the settings (for the state)
//! and hands the engine a copy, which applies them at the next style change.

use super::Control;
use crate::api::{ChangeRuleMode, CmdError, StyleChangeCmd};
use crate::live::Cmd;

impl Control {
    pub(super) fn style_change_cmd(&mut self, c: StyleChangeCmd) -> Result<(), CmdError> {
        let mut s = self.style_change;
        let flip = |cur: ChangeRuleMode, to: ChangeRuleMode| if cur == ChangeRuleMode::Reset { to } else { ChangeRuleMode::Reset };
        match c {
            StyleChangeCmd::SetTempoChange { rule } => s.tempo = rule,
            StyleChangeCmd::SetPartsChange { rule } => s.parts = rule,
            StyleChangeCmd::SetSectionSet { section } => s.section_set = section.map(|m| m.min(3)),
            StyleChangeCmd::ToggleStyleTempoLock => s.tempo = flip(s.tempo, ChangeRuleMode::Lock),
            StyleChangeCmd::ToggleStyleTempoHold => s.tempo = flip(s.tempo, ChangeRuleMode::Hold),
        }
        self.engine_cmd(Cmd::ChangeRules(s.into()))?;
        self.style_change = s;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::api::*;
    use crate::session::{Options, Port, Session};
    use std::path::{Path, PathBuf};

    const MS: u64 = 1_000_000;

    fn corpus(name: &str) -> Option<PathBuf> {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2").join(name);
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return None;
        }
        Some(p)
    }

    fn offline(name: &str) -> Option<Session> {
        Some(Session::offline(Options { paths: vec![corpus(name)?], ..Options::default() }).unwrap())
    }

    fn chord_c(s: &Session) {
        for n in [36, 40, 43] {
            s.midi_in(Port::Keys, &[0x90, n, 100]);
        }
    }

    /// Release C and play F: a chord the session has not sent yet (it never re-sends an
    /// unchanged chord, so replaying C could not start the band even with Sync Start on).
    fn chord_f(s: &Session) {
        for n in [36, 40, 43] {
            s.midi_in(Port::Keys, &[0x80, n, 0]);
        }
        for n in [41, 45, 48] {
            s.midi_in(Port::Keys, &[0x90, n, 100]);
        }
    }

    /// SlowWalker, OTS Link on at At Main Section Change, band playing Main A, Main B
    /// pressed and the band stopped before B plays: B waits.
    fn stopped_with_main_b_waiting() -> Option<Session> {
        let s = offline("SlowWalker.T552.sty")?;
        s.send(OtsCmd::SetOtsLinkTiming { timing: OtsLinkTiming::MainChange }).unwrap();
        s.send(OtsCmd::SetOtsLink { on: true }).unwrap();
        chord_c(&s);
        s.advance(500 * MS);
        s.send(TransportCmd::Main { index: 1 }).unwrap();
        s.advance(10 * MS);
        s.send(TransportCmd::StartStop).unwrap();
        s.advance(10 * MS);
        let st = s.state();
        assert_eq!((st.transport.running, st.transport.main, st.ots.applied, st.transport.sync_start), (false, 1, 1, false));
        Some(s)
    }

    /// Advance in 10 ms steps until `f` holds (at most `max_ms`).
    fn until(s: &Session, max_ms: u64, f: impl Fn(&AppState) -> bool) -> bool {
        for _ in 0..max_ms / 10 {
            if f(&s.state()) {
                return true;
            }
            s.advance(10 * MS);
        }
        false
    }

    #[test]
    fn ots_recall_turns_sync_start_on() {
        let Some(s) = offline("SlowWalker.T552.sty") else { return };
        s.send(TransportCmd::ToggleSyncStart).unwrap();
        assert!(!s.state().transport.sync_start);
        s.send(OtsCmd::RecallOts { index: 1 }).unwrap();
        assert!(s.state().transport.sync_start, "OTS turns Sync Start on");
        chord_c(&s);
        s.advance(50 * MS);
        assert!(s.state().transport.running, "the next chord starts the style");
        s.send(OtsCmd::RecallOts { index: 0 }).unwrap();
        s.advance(10 * MS);
        let st = s.state();
        assert!(st.transport.running && !st.transport.sync_start, "playing: the recall does not stop the band");
    }

    #[test]
    fn ots_link_timing_at_main_section_change() {
        let Some(s) = offline("SlowWalker.T552.sty") else { return };
        assert_eq!(s.state().ots.link_timing, OtsLinkTiming::MainChange, "the default");
        s.send(OtsCmd::SetOtsLink { on: true }).unwrap();
        assert_eq!(s.state().ots.link_timing, OtsLinkTiming::MainChange);
        assert_eq!(s.state().ots.applied, 1, "stopped: Main A's OTS at once");
        chord_c(&s);
        s.advance(500 * MS);
        s.send(TransportCmd::Main { index: 1 }).unwrap();
        s.advance(10 * MS);
        let st = s.state();
        assert_eq!((st.transport.main, st.ots.applied), (1, 1), "pressed: not yet");
        // Auto Fill: B's fill first, still OTS 1; OTS 2 when Main B starts.
        assert!(until(&s, 4000, |st| st.transport.section.as_deref() == Some("Fill In BB")), "the fill");
        assert_eq!(s.state().ots.applied, 1);
        assert!(until(&s, 4000, |st| st.transport.section.as_deref() == Some("Main B")), "Main B");
        assert_eq!(s.state().ots.applied, 2, "at the Main section change");
        // Immediate: as the Main is pressed.
        s.send(OtsCmd::SetOtsLinkTiming { timing: OtsLinkTiming::Immediate }).unwrap();
        s.send(TransportCmd::Main { index: 2 }).unwrap();
        s.advance(10 * MS);
        let st = s.state();
        assert_eq!(st.ots.applied, 3);
        assert_ne!(st.transport.section.as_deref(), Some("Main C"));
    }

    /// Review #92: stopping the band before a pressed Main plays must not recall that
    /// Main's OTS (the recall arms Sync Start, and the next chord would restart the band).
    #[test]
    fn ots_link_at_main_change_stop_is_not_a_change() {
        let Some(s) = offline("SlowWalker.T552.sty") else { return };
        s.send(OtsCmd::SetOtsLinkTiming { timing: OtsLinkTiming::MainChange }).unwrap();
        s.send(OtsCmd::SetOtsLink { on: true }).unwrap();
        assert_eq!(s.state().ots.applied, 1);
        chord_c(&s);
        s.advance(500 * MS);
        assert!(s.state().transport.running);
        s.send(TransportCmd::Main { index: 1 }).unwrap();
        s.advance(10 * MS);
        s.send(TransportCmd::StartStop).unwrap();
        s.advance(10 * MS);
        let st = s.state();
        assert!(!st.transport.running);
        assert_eq!((st.transport.main, st.ots.applied, st.transport.sync_start), (1, 1, false), "the stop recalls nothing");
        chord_f(&s);
        s.advance(50 * MS);
        assert!(!s.state().transport.running, "the next chord does not restart the band");
        // Starting it plays Main B: its OTS then.
        s.send(TransportCmd::StartStop).unwrap();
        assert!(until(&s, 4000, |st| st.transport.section.as_deref() == Some("Main B")), "Main B");
        assert_eq!(s.state().ots.applied, 2, "Main B starts");
        // Stopped, a press still recalls at once (and arms Sync Start, as any recall does).
        s.send(TransportCmd::Main { index: 2 }).unwrap();
        s.advance(10 * MS);
        s.send(TransportCmd::StartStop).unwrap();
        s.advance(10 * MS);
        assert_eq!(s.state().ots.applied, 2);
        s.send(TransportCmd::Main { index: 3 }).unwrap();
        s.advance(10 * MS);
        let st = s.state();
        assert_eq!((st.ots.applied, st.transport.sync_start), (4, true), "stopped: follows the press");
    }

    /// Review #92 round 2: switching the timing to Immediate while a Main waits is a
    /// settings change, not a Main press: no recall and no Sync Start, and the band stays
    /// stopped on the next (new) chord. The waiting Main's OTS comes when the band starts.
    #[test]
    fn ots_link_timing_switch_while_stopped_recalls_nothing() {
        let Some(s) = stopped_with_main_b_waiting() else { return };
        s.send(OtsCmd::SetOtsLinkTiming { timing: OtsLinkTiming::Immediate }).unwrap();
        s.advance(10 * MS);
        let st = s.state();
        assert_eq!((st.ots.applied, st.transport.sync_start), (1, false), "the switch recalls nothing");
        chord_f(&s);
        s.advance(50 * MS);
        assert!(!s.state().transport.running, "the next chord does not start the band");
        s.send(OtsCmd::SetOtsLinkTiming { timing: OtsLinkTiming::MainChange }).unwrap();
        s.send(OtsCmd::SetOtsLinkTiming { timing: OtsLinkTiming::Immediate }).unwrap();
        s.advance(10 * MS);
        assert_eq!(s.state().ots.applied, 1, "back and forth: still nothing");
        s.send(TransportCmd::StartStop).unwrap();
        s.advance(10 * MS);
        let st = s.state();
        assert!(st.transport.running);
        assert_eq!(st.ots.applied, 2, "Immediate: Main B's OTS as the band starts on it");
    }

    /// Review #92 round 2: pressing the waiting Main again while stopped is a press, so
    /// both timings follow it (recall, Sync Start on), as for any other Main.
    #[test]
    fn ots_link_same_main_pressed_again_while_stopped_recalls() {
        let Some(s) = stopped_with_main_b_waiting() else { return };
        s.send(TransportCmd::Main { index: 1 }).unwrap();
        s.advance(10 * MS);
        let st = s.state();
        assert_eq!((st.transport.main, st.ots.applied, st.transport.sync_start), (1, 2, true), "the press recalls");
        chord_f(&s);
        s.advance(50 * MS);
        assert!(s.state().transport.running, "Sync Start: the next chord starts the band");
        // Fill Self on the waiting Main is a press of it too.
        let Some(s) = stopped_with_main_b_waiting() else { return };
        s.send(TransportCmd::FillSelf).unwrap();
        s.advance(10 * MS);
        assert_eq!(s.state().ots.applied, 2, "Fill Self presses the selected Main");
    }

    /// Immediate and At Main Section Change recall the same OTS for a Main the style
    /// lacks: the pressed button's.
    #[test]
    fn ots_link_timings_agree_on_a_missing_main() {
        let Some((path, missing)) = corpus_lacking_a_main() else {
            eprintln!("no corpus style lacks a Main; skipping");
            return;
        };
        for timing in [OtsLinkTiming::Immediate, OtsLinkTiming::MainChange] {
            let s = Session::offline(Options { paths: vec![path.clone()], ..Options::default() }).unwrap();
            s.send(OtsCmd::SetOtsLinkTiming { timing }).unwrap();
            s.send(OtsCmd::SetOtsLink { on: true }).unwrap();
            chord_c(&s);
            s.advance(500 * MS);
            s.send(TransportCmd::Main { index: missing }).unwrap();
            s.advance(8000 * MS);
            let st = s.state();
            assert!(st.transport.running, "{timing:?}");
            assert_eq!(st.ots.applied, missing + 1, "{timing:?}: the pressed button's OTS");
        }
    }

    /// A corpus style with four OTS that lacks a Main (and has Main A), and that Main.
    fn corpus_lacking_a_main() -> Option<(PathBuf, u8)> {
        use crate::sff::{SectionId, Style};
        let mut paths = Vec::new();
        for dir in ["corpus/MOX_v2", "corpus/SX900Style for Genos", "corpus/T5Style"] {
            let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
            paths.extend(std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()));
        }
        paths.sort();
        paths.into_iter().find_map(|p| {
            let s = Style::load(&p).ok()?;
            let has = |m| s.sections.contains_key(&SectionId::Main(m));
            let missing = (1..4).find(|&m| !has(m))?;
            (has(0) && s.ots.len() == 4).then_some((p, missing))
        })
    }

    #[test]
    fn change_behavior_commands_and_state() {
        let (Some(s), Some(other)) = (offline("SlowWalker.T552.sty"), corpus("AustinCityBlues.S930.STY")) else { return };
        assert_eq!(
            s.state().style_change,
            StyleChangeState { tempo: ChangeRuleMode::Hold, parts: ChangeRuleMode::Hold, section_set: None }
        );
        s.send(StyleChangeCmd::ToggleStyleTempoLock).unwrap();
        assert_eq!(s.state().style_change.tempo, ChangeRuleMode::Reset, "Hold -> Reset");
        s.send(StyleChangeCmd::ToggleStyleTempoLock).unwrap();
        assert_eq!(s.state().style_change.tempo, ChangeRuleMode::Lock);
        s.send(StyleChangeCmd::ToggleStyleTempoHold).unwrap();
        assert_eq!(s.state().style_change.tempo, ChangeRuleMode::Reset);
        s.send(StyleChangeCmd::ToggleStyleTempoHold).unwrap();
        assert_eq!(s.state().style_change.tempo, ChangeRuleMode::Hold);
        s.send(StyleChangeCmd::SetTempoChange { rule: ChangeRuleMode::Lock }).unwrap();
        s.send(StyleChangeCmd::SetPartsChange { rule: ChangeRuleMode::Lock }).unwrap();
        s.send(StyleChangeCmd::SetSectionSet { section: Some(7) }).unwrap();
        assert_eq!(s.state().style_change.section_set, Some(3), "clamped to Main D");
        // Locked: another style keeps the tempo and the muted part; Section Set picks D.
        s.send(TransportCmd::TempoUp).unwrap();
        s.send(MixerCmd::ToggleStylePart { part: 2 }).unwrap();
        let tempo = s.state().transport.tempo;
        s.send(LibraryCmd::LoadStylePath { path: other.display().to_string() }).unwrap();
        s.advance(10 * MS);
        let st = s.state();
        assert!(st.style.path.contains("Austin"), "{}", st.style.path);
        assert_eq!(st.transport.tempo, tempo);
        assert!(!st.mixer.style_parts[2].on, "Part On/Off Lock keeps the mute");
        assert_eq!(st.transport.main, 3);
    }
    // ----- OTS Link timing at the change point (owner requirement) -----

    /// What a keyboard part plays: on, program, volume, octave.
    type PartSound = Vec<(bool, u8, u8, i8)>;

    fn sounds(st: &AppState) -> PartSound {
        st.keyboard_parts.iter().map(|p| (p.on, p.program, p.volume, p.octave)).collect()
    }

    /// Two styles (SlowWalker first, BubblyDub), OTS Link on at the default timing.
    fn two_styles() -> Option<(Session, usize)> {
        let (a, b) = (corpus("SlowWalker.T552.sty")?, corpus("BubblyDub.T552.sty")?);
        let s = Session::offline(Options { paths: vec![a, b], ..Options::default() }).unwrap();
        s.finish_indexing();
        let id = |name: &str| s.library_list().entries.iter().find(|e| e.path.contains(name)).unwrap().id;
        let (walker, other) = (id("SlowWalker"), id("BubblyDub"));
        s.send(LibraryCmd::LoadStyle { id: walker }).unwrap();
        s.send(OtsCmd::SetOtsLink { on: true }).unwrap();
        s.advance(10 * MS);
        assert_eq!(s.state().style.id, walker);
        Some((s, other))
    }

    /// The keyboard parts as OTS `i` of style `id` sets them (a session of its own).
    fn ots_sounds(path: &str, i: u8) -> Option<PartSound> {
        let s = offline(path)?;
        s.send(OtsCmd::RecallOts { index: i }).unwrap();
        s.advance(10 * MS);
        Some(sounds(&s.state()))
    }

    /// Step 5 ms at a time until `done`; at every step before it, the keyboard parts must
    /// still sound as `before` (no OTS reaches them early). Returns the state at `done`.
    fn nothing_until(s: &Session, before: &PartSound, max_ms: u64, done: impl Fn(&AppState) -> bool) -> std::sync::Arc<AppState> {
        for _ in 0..max_ms / 5 {
            let st = s.state();
            if done(&st) {
                return st;
            }
            assert_eq!(&sounds(&st), before, "an OTS reached the keyboard parts before the change point ({:?} bar {} beat {})", st.transport.section, st.transport.bar, st.transport.beat);
            s.advance(5 * MS);
        }
        panic!("the change never came");
    }

    /// Playing Main A of SlowWalker (Auto Fill as given), OTS 1 recalled, on beat 3 of
    /// bar 1 (mid-bar: not the first beat, where Next Bar changes at once).
    fn playing_main_a(auto_fill: bool) -> Option<(Session, usize)> {
        let (s, other) = two_styles()?;
        if s.state().transport.auto_fill != auto_fill {
            s.send(TransportCmd::ToggleAutoFill).unwrap();
        }
        chord_c(&s);
        assert!(until(&s, 4000, |st| st.transport.running && st.transport.beat == 3), "beat 3");
        let st = s.state();
        assert_eq!((st.transport.section.as_deref(), st.ots.applied), (Some("Main A"), 1));
        Some((s, other))
    }

    #[test]
    fn ots_link_timing_defaults_to_at_main_section_change() {
        let Some(s) = offline("SlowWalker.T552.sty") else { return };
        assert_eq!(s.state().ots.link_timing, OtsLinkTiming::MainChange);
    }

    /// A Main pressed mid-bar (Auto Fill off): its OTS comes exactly as that Main starts,
    /// never while Main A still plays.
    #[test]
    fn ots_at_change_main_pressed_mid_bar() {
        let Some((s, _)) = playing_main_a(false) else { return };
        let want = ots_sounds("SlowWalker.T552.sty", 1).unwrap();
        let before = sounds(&s.state());
        assert_ne!(before, want, "OTS 1 and 2 differ, so the test can see the change");
        s.send(TransportCmd::Main { index: 1 }).unwrap();
        let st = nothing_until(&s, &before, 4000, |st| st.transport.section.as_deref() == Some("Main B"));
        assert_eq!((st.ots.applied, sounds(&st)), (2, want), "at the switch");
    }

    /// Fill -> Main (Auto Fill on): nothing during the fill; OTS 2 when Main B starts.
    #[test]
    fn ots_at_change_fill_then_main() {
        let Some((s, _)) = playing_main_a(true) else { return };
        let before = sounds(&s.state());
        s.send(TransportCmd::Main { index: 1 }).unwrap();
        let st = nothing_until(&s, &before, 4000, |st| st.transport.section.as_deref() == Some("Fill In BB"));
        assert_eq!(st.ots.applied, 1, "the fill plays: still OTS 1");
        let st = nothing_until(&s, &before, 4000, |st| st.transport.section.as_deref() == Some("Main B"));
        assert_eq!(st.ots.applied, 2, "Main B starts: OTS 2");
    }

    /// A style queued at the next bar: its OTS reaches the keyboard parts as it takes over,
    /// not on selection.
    #[test]
    fn ots_at_change_style_queued_at_next_bar() {
        let Some((s, other)) = playing_main_a(false) else { return };
        let want = ots_sounds("BubblyDub.T552.sty", 0).unwrap();
        let before = sounds(&s.state());
        assert_ne!(before, want, "the two styles' OTS 1 differ");
        s.send(LibraryCmd::QueueStyle { id: other }).unwrap();
        assert_eq!(s.state().preview.queued, Some(other), "it waits for the bar line");
        let st = nothing_until(&s, &before, 4000, |st| st.style.id == other);
        assert!(st.transport.running);
        assert_eq!((st.ots.applied, sounds(&st)), (1, want), "the new style's OTS 1 as it takes over");
    }

    /// A style chosen while an Ending plays waits for the Ending (#94), and so does its OTS.
    #[test]
    fn ots_at_change_style_queued_during_an_ending() {
        let Some((s, other)) = playing_main_a(false) else { return };
        let want = ots_sounds("BubblyDub.T552.sty", 0).unwrap();
        s.send(TransportCmd::Ending { index: 0 }).unwrap();
        assert!(until(&s, 4000, |st| st.transport.section.as_deref().is_some_and(|n| n.starts_with("Ending"))), "the Ending");
        s.advance(50 * MS);
        let before = sounds(&s.state());
        s.send(LibraryCmd::QueueStyle { id: other }).unwrap();
        let st = nothing_until(&s, &before, 20_000, |st| st.style.id == other);
        assert!(!st.transport.running, "the Ending played out first: {:?} bar {} beat {}", st.transport.section, st.transport.bar, st.transport.beat);
        assert_eq!(sounds(&st), want, "the new style's OTS 1 once it takes over");
    }

    /// #111: a style that takes over during a Fill (At Main Section Change): the fill plays
    /// on in the new style, and the keyboard parts change only when Main B starts, to the
    /// new style's OTS 2. Nothing reaches them at the swap, mid-fill.
    #[test]
    fn ots_at_change_style_swapping_in_during_a_fill() {
        use crate::engine::MainTiming;
        let Some((s, other)) = playing_main_a(true) else { return };
        // To Main Immediate: the style comes in at the next beat, with the fill.
        s.send(StyleSettingsCmd::SetMainTiming { timing: MainTiming::Immediate }).unwrap();
        let want = ots_sounds("BubblyDub.T552.sty", 1).unwrap();
        let before = sounds(&s.state());
        assert_ne!(before, want, "the test can see the change");
        // Mid-beat, so both wait for the same next beat. (From a playing Fill a style waits
        // for the bar line, where the Main starts: the fill's first beat is the only way
        // in. At the beat line itself the style would take over during Main A, and recall
        // its OTS 1 there.)
        s.advance(50 * MS);
        s.send(LibraryCmd::QueueStyle { id: other }).unwrap();
        s.send(TransportCmd::Main { index: 1 }).unwrap();
        let st = nothing_until(&s, &before, 4000, |st| st.style.id == other);
        let fill = st.transport.section.clone();
        assert!(fill.as_deref().is_some_and(|n| n.starts_with("Fill")), "the new style came in with the fill: {fill:?}");
        assert_ne!(st.ots.applied, 2, "no OTS 2 at the swap");
        let st = nothing_until(&s, &before, 4000, |st| st.transport.section.as_deref() == Some("Main B"));
        assert_eq!((st.ots.applied, sounds(&st)), (2, want), "Main B starts: the new style's OTS 2");
    }

    /// #111: a style that takes over during an Intro (At Main Section Change): its OTS
    /// comes when the Main starts, not at the swap.
    #[test]
    fn ots_at_change_style_swapping_in_during_an_intro() {
        let Some((s, other)) = two_styles() else { return };
        let want = ots_sounds("BubblyDub.T552.sty", 0).unwrap();
        s.send(TransportCmd::Intro { index: 2 }).unwrap();
        chord_c(&s);
        assert!(until(&s, 4000, |st| st.transport.running), "started");
        let before = sounds(&s.state());
        assert_ne!(before, want);
        s.send(LibraryCmd::QueueStyle { id: other }).unwrap();
        let st = nothing_until(&s, &before, 8000, |st| st.style.id == other);
        if !st.transport.section.as_deref().is_some_and(|n| n.starts_with("Intro")) {
            // The Intro ended at the swap: nothing to test here.
            return;
        }
        let st = nothing_until(&s, &before, 20_000, |st| st.transport.section.as_deref().is_some_and(|n| n.starts_with("Main")));
        assert_eq!(sounds(&st), want, "the Main starts: the new style's OTS 1");
    }

    /// Real Time (Immediate) is still there: the pressed Main's OTS at once, while Main A
    /// plays on.
    #[test]
    fn ots_real_time_still_immediate() {
        let Some((s, _)) = playing_main_a(false) else { return };
        s.send(OtsCmd::SetOtsLinkTiming { timing: OtsLinkTiming::Immediate }).unwrap();
        s.send(TransportCmd::Main { index: 1 }).unwrap();
        s.advance(5 * MS);
        let st = s.state();
        assert_eq!((st.transport.section.as_deref(), st.ots.applied), (Some("Main A"), 2));
        assert_eq!(sounds(&st), ots_sounds("SlowWalker.T552.sty", 1).unwrap());
    }
}
