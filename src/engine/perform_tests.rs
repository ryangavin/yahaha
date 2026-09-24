//! Section Change Timing, Section Reset, Retrigger, ritardando, Fade In/Out and the
//! Synchro Stop Window, played on a corpus style.

use super::*;

#[derive(Default)]
struct Rec {
    msgs: Vec<(u64, Vec<u8>)>,
    now: u64,
}

impl Sink for Rec {
    fn send(&mut self, m: &[u8]) {
        self.msgs.push((self.now, m.to_vec()));
    }
}

impl Rec {
    /// The CC7 levels sent on channel `ch`, in order.
    fn volumes(&self, ch: u8) -> Vec<(u64, u8)> {
        self.msgs.iter().filter(|(_, m)| m.len() == 3 && m[0] == 0xB0 | ch && m[1] == 7).map(|(t, m)| (*t, m[2])).collect()
    }

    /// Anything a fade must never send: a CC7 outside the Style parts (8-15), or a
    /// Master Volume.
    fn outside_style(&self) -> bool {
        self.msgs.iter().any(|(_, m)| m.starts_with(&[0xF0, 0x7F, 0x7F, 0x04, 0x01]) || (m.len() == 3 && m[0] & 0xF0 == 0xB0 && m[1] == 7 && m[0] & 0x0F < 8))
    }
}

/// The Style part with the loudest fader, and its channel.
fn loudest(e: &Engine) -> (usize, u8) {
    let p = (0..8).max_by_key(|&p| e.mixer[p]).unwrap();
    assert!(e.mixer[p] > 20);
    (p, 8 + p as u8)
}

fn engine() -> Option<Engine> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
    if !p.exists() {
        eprintln!("corpus missing; skipping");
        return None;
    }
    Some(Engine::new(Box::new(Prepared::new(&Style::load(&p).unwrap()))))
}

fn chord(s: &str) -> Chord {
    crate::parse_chord(s).unwrap()
}

/// Run the engine to `to` at its deadlines (at most 5 ms apart).
fn play(e: &mut Engine, rec: &mut Rec, from: u64, to: u64) {
    let mut now = from;
    while now < to {
        rec.now = now;
        e.process(now, rec);
        now = e.next_deadline().unwrap_or(to).clamp(now + 1, now + 5_000_000).min(to);
    }
    rec.now = to;
    e.process(to, rec);
}

/// Ticks per beat and per bar, and ns per beat at the style's tempo.
fn grid(e: &Engine) -> (f64, f64, u64) {
    let (ppq, tpb) = (e.style.ppq as f64, e.style.tpb as f64);
    (ppq, tpb, e.ns_at(ppq) - e.ns_at(0.0))
}

fn started(settings: StyleSettings) -> Option<(Engine, Rec)> {
    let mut e = engine()?;
    e.set_style_settings(settings);
    let mut rec = Rec::default();
    e.set_chord(chord("C"), 0, &mut rec);
    assert!(e.running);
    Some((e, rec))
}

#[test]
fn next_bar_changes_at_once_within_the_first_beat() {
    let Some((mut e, _)) = started(StyleSettings::default()) else { return };
    let (ppq, tpb, _) = grid(&e);
    let early = e.ns_at(tpb + 0.5 * ppq);
    assert_eq!(e.change_point(Change::Main, early), (tpb + 0.5 * ppq, tpb));
    let late = e.ns_at(tpb + 1.5 * ppq);
    assert_eq!(e.change_point(Change::Main, late), (2.0 * tpb, 2.0 * tpb));
    assert_eq!(e.change_point(Change::Style, early).1, tpb);
    // The stop an Ending the style lacks always waits for the bar line.
    assert_eq!(e.change_point(Change::Stop, early), (2.0 * tpb, 2.0 * tpb));
    e.set_style_settings(StyleSettings { main_timing: MainTiming::Immediate, ..StyleSettings::default() });
    // Immediate falls back to Next Bar with Auto Fill In on (the default)...
    assert_eq!(e.change_point(Change::Main, late), (2.0 * tpb, 2.0 * tpb));
    // ... and is the next beat with it off, counted from its bar.
    e.button(Button::AutoFill, late, &mut Rec::default());
    assert_eq!(e.change_point(Change::Main, late), (tpb + 2.0 * ppq, tpb));
    // A style change is Immediate whatever Auto Fill In says.
    assert_eq!(e.change_point(Change::Style, late), (tpb + 2.0 * ppq, tpb));
}

#[test]
fn immediate_main_change_carries_on_from_the_beat() {
    let s = StyleSettings { main_timing: MainTiming::Immediate, ..StyleSettings::default() };
    let Some((mut e, mut rec)) = started(s) else { return };
    let (ppq, tpb, _) = grid(&e);
    e.button(Button::AutoFill, 0, &mut rec);
    let at = e.ns_at(tpb + 1.5 * ppq);
    play(&mut e, &mut rec, 0, at);
    e.button(Button::Main(1), at, &mut rec);
    let beat3 = e.ns_at(tpb + 2.0 * ppq);
    play(&mut e, &mut rec, at, beat3 + 1_000);
    let s = e.snapshot(beat3 + 1_000);
    assert_eq!((s.cur, s.bar, s.beat), (Some(SectionId::Main(1)), 0, 2), "Main B from beat 3 of its first bar");
}

#[test]
fn inside_intro_ending_timing() {
    let s = StyleSettings { intro_ending_timing: IntroEndingTiming::EndOfSection, ..StyleSettings::default() };
    let Some(mut e) = engine() else { return };
    e.set_style_settings(s);
    let mut rec = Rec::default();
    let (intro1, intro2) = (slot_of(SectionId::Intro(0)), slot_of(SectionId::Intro(1)));
    let (end1, end2) = (slot_of(SectionId::Ending(0)), slot_of(SectionId::Ending(1)));
    if !(e.style.has(intro1) && e.style.has(intro2) && e.style.has(end2)) {
        return;
    }
    e.button(Button::Intro(0), 0, &mut rec);
    e.set_chord(chord("C"), 0, &mut rec);
    assert_eq!(e.cur, intro1);
    let (ppq, tpb, _) = grid(&e);
    let len = e.style.sections[intro1].as_ref().unwrap().len as f64;
    let now = e.ns_at(1.5 * ppq);
    // Intro to Ending II: the end of the Intro.
    assert_eq!(e.change_point(Change::IntroEnding(end2), now), (len, len));
    // Intro to Intro: always Next Bar (the next bar line, being past the first beat).
    assert_eq!(e.change_point(Change::IntroEnding(intro2), now), (tpb, tpb));
    // Into Ending I: the next bar line, whatever the setting.
    assert_eq!(e.change_point(Change::IntroEnding(end1), now), (tpb, tpb));
    e.set_style_settings(StyleSettings::default());
    assert_eq!(e.change_point(Change::IntroEnding(end2), now), (tpb, tpb));
}

#[test]
fn tap_resets_the_section_or_sets_the_tempo() {
    let Some((mut e, mut rec)) = started(StyleSettings::default()) else { return };
    let (ppq, tpb, _) = grid(&e);
    let t = e.ns_at(tpb + 2.5 * ppq);
    play(&mut e, &mut rec, 0, t);
    let bpm = e.bpm;
    e.button(Button::TapTempo, t, &mut rec);
    let s = e.snapshot(t);
    assert_eq!((s.bar, s.beat, s.bpm), (0, 0, bpm), "back to the top, same tempo");
    // The section plays again from its first events.
    assert_eq!(e.ev_idx, 0);
    let later = t + 10_000_000;
    play(&mut e, &mut rec, t, later);
    assert!(e.ev_idx > 0, "its first events played");
    // Off: taps set the tempo.
    e.set_style_settings(StyleSettings { section_reset: false, ..StyleSettings::default() });
    e.button(Button::TapTempo, later, &mut rec);
    e.button(Button::TapTempo, later + 500_000_000, &mut rec);
    assert!((e.bpm - 120.0).abs() < 0.01, "{}", e.bpm);
    // Section Reset is also its own button, and a queued Main follows the new grid.
    let now = later + 600_000_000;
    play(&mut e, &mut rec, later + 500_000_000, now);
    e.button(Button::AutoFill, now, &mut rec);
    e.button(Button::Main(2), now, &mut rec);
    e.button(Button::SectionReset, now + 1, &mut rec);
    let q = e.queued.expect("Main C queued");
    let t0 = e.sec_start;
    assert_eq!((q.at, q.sec_start), (t0 + tpb, t0 + tpb));
}

#[test]
fn retrigger_loops_the_main_head_from_each_chord() {
    let Some((mut e, mut rec)) = started(StyleSettings { retrigger_rate: 4, ..StyleSettings::default() }) else { return };
    let (ppq, tpb, beat_ns) = grid(&e);
    e.button(Button::Retrigger, 0, &mut rec);
    // On, but nothing loops until a chord is played.
    let t = e.ns_at(tpb + 1.3 * ppq);
    play(&mut e, &mut rec, 0, t);
    assert_eq!(e.snapshot(t).bar, 1);
    e.set_chord(chord("F"), t, &mut rec);
    assert!(e.features.retrigger.looping);
    let start = e.sec_start;
    assert!((start - e.tick_at(t)).abs() < 1e-6, "the Main starts again at the chord");
    // A quarter note loops: two beats later it has wrapped twice, and never got past it.
    let mut now = t;
    while now < t + 2 * beat_ns + beat_ns / 2 {
        now += 7_000_000;
        rec.now = now;
        e.process(now, &mut rec);
        assert!(e.tick_at(now) - e.sec_start <= ppq + 1e-6);
    }
    assert!((e.sec_start - (start + 2.0 * ppq)).abs() < 1e-6, "{} vs {}", e.sec_start, start);
    // Off: the Main plays on past the head.
    e.button(Button::Retrigger, now, &mut rec);
    play(&mut e, &mut rec, now, now + 2 * beat_ns);
    assert!(e.tick_at(now + 2 * beat_ns) - e.sec_start > ppq);
    // On again: armed, waiting for the next chord.
    e.button(Button::Retrigger, now, &mut rec);
    assert!(e.retrigger_on() && !e.features.retrigger.looping);
}

/// Retrigger chords coming faster than the queued change's wait never hold it off: a Main
/// (and a style change, and the Ending) queued for the next bar comes at that bar.
#[test]
fn retrigger_chords_do_not_hold_off_a_queued_change() {
    let Some((mut e, mut rec)) = started(StyleSettings { retrigger_rate: 1, ..StyleSettings::default() }) else { return };
    let (ppq, tpb, beat_ns) = grid(&e);
    e.button(Button::AutoFill, 0, &mut rec); // off: Main B itself, no fill
    e.button(Button::Retrigger, 0, &mut rec);
    let names = ["G", "Am", "Dm", "C"];
    let mut now = e.ns_at(tpb + 1.5 * ppq);
    play(&mut e, &mut rec, 0, now);
    e.button(Button::Main(1), now, &mut rec);
    let main_b = slot_of(SectionId::Main(1));
    let due = e.queued.expect("Main B queued").at;
    e.set_chord(chord("F"), now, &mut rec);
    assert!(e.features.retrigger.looping, "the chord restarted the Main");
    assert!(e.queued.is_some_and(|q| (q.at - due).abs() < 1e-6), "the chord moved the change");
    // A chord every 3 beats.
    for i in 1..=8 {
        let next = now + 3 * beat_ns;
        play(&mut e, &mut rec, now, next);
        now = next;
        if e.cur == main_b {
            break;
        }
        e.set_chord(chord(names[i % 4]), now, &mut rec);
        assert!(e.queued.is_some_and(|q| (q.at - due).abs() < 1e-6), "chord {i} moved the change");
    }
    assert_eq!(e.cur, main_b, "Main B came");
    // And the Ending, the same way: the band stops.
    let end1 = slot_of(SectionId::Ending(0));
    if !e.style.has(end1) {
        return;
    }
    e.button(Button::Ending(0), now, &mut rec);
    for i in 0..16 {
        if !e.running {
            break;
        }
        let next = now + 3 * beat_ns;
        play(&mut e, &mut rec, now, next);
        now = next;
        if e.running && e.cur != end1 {
            e.set_chord(chord(names[i % 4]), now, &mut rec);
        }
    }
    assert!(!e.running, "the Ending came and ended");
}

#[test]
fn pressing_the_ending_again_slows_down_and_the_tempo_comes_back() {
    let Some((mut e, mut rec)) = started(StyleSettings::default()) else { return };
    let (ppq, tpb, _) = grid(&e);
    let end1 = slot_of(SectionId::Ending(0));
    if !e.style.has(end1) {
        return;
    }
    let bpm = e.bpm;
    let t = e.ns_at(tpb + 1.5 * ppq);
    play(&mut e, &mut rec, 0, t);
    e.button(Button::Ending(0), t, &mut rec);
    let bar2 = e.ns_at(2.0 * tpb);
    play(&mut e, &mut rec, t, bar2 + 1_000);
    assert_eq!(e.cur, end1);
    e.button(Button::Ending(0), bar2 + 1_000, &mut rec);
    assert!(e.ritardando() && e.snapshot(bar2).ritardando);
    let mut slowest = bpm;
    let mut now = bar2 + 1_000;
    while e.running {
        let next = e.next_deadline().unwrap().max(now + 1);
        now = next;
        rec.now = now;
        e.process(now, &mut rec);
        if e.running {
            assert!(e.bpm <= slowest + 1e-9, "it only slows");
            slowest = e.bpm;
        }
    }
    assert!(slowest < bpm * 0.75 && slowest >= bpm * RIT_END - 1e-6, "{slowest} of {bpm}");
    assert_eq!(e.bpm, bpm, "the tempo comes back when it stops");
    assert!(!e.ritardando());
}

#[test]
fn fade_in_from_start_and_fade_out_to_a_stop_then_hold() {
    let s = StyleSettings { fade_in_ms: 1000, fade_out_ms: 500, fade_hold_ms: 300, ..StyleSettings::default() };
    let Some(mut e) = engine() else { return };
    e.set_style_settings(s);
    let mut rec = Rec::default();
    let (p, ch) = loudest(&e);
    let full = e.mixer[p];
    e.button(Button::Fade, 0, &mut rec);
    assert_eq!(e.fade_state(), FadeState::Armed);
    assert!(rec.msgs.is_empty(), "arming sends nothing");
    e.set_chord(chord("C"), 0, &mut rec);
    assert_eq!(e.fade_state(), FadeState::FadingIn);
    // Every Style part is at 0 before the first note.
    let first_note = rec.msgs.iter().position(|(_, m)| m[0] & 0xF0 == 0x90 && m[2] > 0).unwrap();
    for c in 8..16u8 {
        let last = rec.msgs[..first_note].iter().rev().find(|(_, m)| m.len() == 3 && m[0] == 0xB0 | c && m[1] == 7);
        assert!(last.is_none_or(|(_, m)| m[2] == 0), "part {c} at {last:?} before the first note");
    }
    play(&mut e, &mut rec, 0, 1_100_000_000);
    let v = rec.volumes(ch);
    let rise = &v[v.iter().position(|x| x.1 == 0).unwrap()..];
    assert!(rise.windows(2).all(|w| w[1].1 >= w[0].1), "rising: {rise:?}");
    assert_eq!(v.last().unwrap().1, full, "back to the fader value");
    assert!(rise.len() > 30, "steps as it goes: {}", rise.len());
    assert_eq!(e.fade_state(), FadeState::Off);
    // The fade is the Style's CC7 alone: no Master Volume, no keyboard part touched, and
    // the faders never moved.
    assert!(!rec.outside_style());
    assert_eq!(e.mixer[p], full);

    rec.msgs.clear();
    let t = 2_000_000_000;
    play(&mut e, &mut rec, 1_100_000_000, t);
    e.button(Button::Fade, t, &mut rec);
    assert_eq!(e.fade_state(), FadeState::FadingOut);
    play(&mut e, &mut rec, t, t + 499_000_000);
    assert!(e.running);
    play(&mut e, &mut rec, t + 499_000_000, t + 510_000_000);
    assert!(!e.running, "the fade out's end stops the band");
    assert_eq!(e.fade_state(), FadeState::Holding);
    assert_eq!(rec.volumes(ch).last().unwrap().1, 0);
    // Stopped, the engine still wakes for the hold's end.
    assert_eq!(e.next_deadline(), Some(t + 800_000_000));
    play(&mut e, &mut rec, t + 510_000_000, t + 800_000_000);
    assert_eq!(e.fade_state(), FadeState::Off);
    assert_eq!(rec.volumes(ch).last(), Some(&(t + 800_000_000, full)));
    assert_eq!(e.next_deadline(), None);
    assert!(!rec.outside_style());
    assert_eq!(e.mixer[p], full);
}

/// A fader moved during a fade keeps its value (the mixer shows it) and goes out scaled;
/// when the fade ends the new value goes out as it is.
#[test]
fn a_fader_moved_mid_fade_is_scaled_then_restored() {
    let s = StyleSettings { fade_out_ms: 1000, ..StyleSettings::default() };
    let Some((mut e, mut rec)) = started(s) else { return };
    let (p, ch) = loudest(&e);
    play(&mut e, &mut rec, 0, 100_000_000);
    e.button(Button::Fade, 100_000_000, &mut rec);
    play(&mut e, &mut rec, 100_000_000, 600_000_000);
    e.set_volume(p as u8, 100, &mut rec);
    assert_eq!(e.mixer[p], 100);
    let sent = rec.volumes(ch).last().unwrap().1;
    assert!((45..=55).contains(&sent), "100 at half way: {sent}");
    e.button(Button::StartStop, 700_000_000, &mut rec);
    assert_eq!(rec.volumes(ch).last().unwrap().1, 100);
}

#[test]
fn stopping_mid_fade_goes_back_to_full() {
    let Some(mut e) = engine() else { return };
    let mut rec = Rec::default();
    let (p, ch) = loudest(&e);
    e.button(Button::Fade, 0, &mut rec);
    e.set_chord(chord("C"), 0, &mut rec);
    play(&mut e, &mut rec, 0, 500_000_000);
    e.button(Button::StartStop, 500_000_000, &mut rec);
    assert_eq!(e.fade_state(), FadeState::Off);
    assert_eq!(rec.volumes(ch).last().unwrap().1, e.mixer[p]);
    // Pressed twice while stopped: armed, then not.
    e.button(Button::Fade, 600_000_000, &mut rec);
    e.button(Button::Fade, 700_000_000, &mut rec);
    assert_eq!(e.fade_state(), FadeState::Off);
}

/// Panic during the hold after a fade out brings the Style's volume back at once.
#[test]
fn panic_ends_the_fade_hold() {
    let s = StyleSettings { fade_out_ms: 100, fade_hold_ms: 5000, ..StyleSettings::default() };
    let Some((mut e, mut rec)) = started(s) else { return };
    let (p, ch) = loudest(&e);
    e.button(Button::Fade, 10_000_000, &mut rec);
    play(&mut e, &mut rec, 10_000_000, 200_000_000);
    assert_eq!(e.fade_state(), FadeState::Holding);
    assert_eq!(rec.volumes(ch).last().unwrap().1, 0);
    e.stop(&mut rec); // already stopped: does nothing
    assert_eq!(e.fade_state(), FadeState::Holding);
    e.fade_cancel(&mut rec);
    assert_eq!(e.fade_state(), FadeState::Off);
    assert_eq!(rec.volumes(ch).last().unwrap().1, e.mixer[p]);
    assert_eq!(e.next_deadline(), None);
}

#[test]
fn synchro_stop_window_cancels_sync_stop_on_a_long_hold() {
    let s = StyleSettings { sync_stop_window_ms: 400, ..StyleSettings::default() };
    let Some(mut e) = engine() else { return };
    e.set_style_settings(s);
    let mut rec = Rec::default();
    e.button(Button::SyncStop, 0, &mut rec);
    // A quick release stops the band.
    e.set_chord(chord("C"), 0, &mut rec);
    play(&mut e, &mut rec, 0, 300_000_000);
    e.chord_released(300_000_000, &mut rec);
    assert!(!e.running && e.sync_stop);
    // A long hold turns Sync Stop off: letting go no longer stops it.
    let t = 1_000_000_000;
    e.set_chord(chord("F"), t, &mut rec);
    assert!(e.running);
    assert!(e.next_deadline().unwrap() <= t + 400_000_000, "the engine wakes for the window");
    play(&mut e, &mut rec, t, t + 401_000_000);
    assert!(!e.sync_stop, "cancelled by the hold");
    e.chord_released(t + 450_000_000, &mut rec);
    assert!(e.running);
    // Off (0): the old behaviour.
    e.set_style_settings(StyleSettings::default());
    e.button(Button::SyncStop, t + 500_000_000, &mut rec);
    e.set_chord(chord("G"), t + 500_000_000, &mut rec);
    play(&mut e, &mut rec, t + 500_000_000, t + 2_000_000_000);
    assert!(e.sync_stop);
    e.chord_released(t + 2_000_000_000, &mut rec);
    assert!(!e.running);
}
