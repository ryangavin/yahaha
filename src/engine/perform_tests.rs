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

/// TAP TEMPO while the band plays, with the default settings (#128): the taps set the
/// tempo (from the second tap, as the Genos does for a Song, OM p.46), averaging up to four
/// taps, and the band plays on from where it is: no Section Reset.
#[test]
#[ignore = "#128 pending: ending/tap-tempo removes this"]
fn tap_while_playing_sets_the_tempo_by_default() {
    let Some((mut e, mut rec)) = started(StyleSettings::default()) else { return };
    let (ppq, tpb, _) = grid(&e);
    let t = e.ns_at(tpb + 2.5 * ppq);
    play(&mut e, &mut rec, 0, t);
    let (bpm, idx) = (e.bpm, e.ev_idx);
    assert!(idx > 0);
    e.button(Button::TapTempo, t, &mut rec);
    assert_eq!(e.bpm, bpm, "one tap alone keeps the tempo");
    assert_eq!(e.ev_idx, idx, "and does not rewind the section");
    let s = e.snapshot(t);
    assert_eq!((s.bar, s.beat), (1, 2), "still in bar 2, beat 3");
    // Taps 400 ms apart: 150 BPM from the second tap on.
    let mut now = t;
    for _ in 0..3 {
        play(&mut e, &mut rec, now, now + 400_000_000);
        now += 400_000_000;
        let tick = e.tick_at(now);
        e.button(Button::TapTempo, now, &mut rec);
        assert!((e.bpm - 150.0).abs() < 0.01, "{}", e.bpm);
        assert!((e.tick_at(now) - tick).abs() < 1.0, "the band plays on from where it is");
    }
    // A fifth tap 500 ms later averages the last four taps (1.2 s over 3 beats).
    play(&mut e, &mut rec, now, now + 500_000_000);
    e.button(Button::TapTempo, now + 500_000_000, &mut rec);
    assert!((e.bpm - 60.0 / 1.3 * 3.0).abs() < 0.01, "{}", e.bpm);
    assert!(e.running);
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

/// Note-ons sent (velocity > 0) at or after `from` in the recording.
fn note_ons_since(rec: &Rec, from: usize) -> usize {
    rec.msgs[from..].iter().filter(|(_, m)| m.len() == 3 && m[0] & 0xF0 == 0x90 && m[2] > 0).count()
}

/// Shortening the Retrigger length while the head loops (a whole note -> a 32nd, 3 beats
/// into the loop): the loop's new end is already past. The head starts again at the last
/// whole loop before now; the loops in between are skipped, not played back to back in
/// one wake. Before the fix: 24 restarts and 119 note-ons at once.
#[test]
fn shortening_the_retrigger_length_mid_loop_does_not_replay_missed_loops() {
    let Some((mut e, mut rec)) = started(StyleSettings { retrigger_rate: 1, ..StyleSettings::default() }) else { return };
    let (ppq, tpb, beat_ns) = grid(&e);
    e.button(Button::Retrigger, 0, &mut rec);
    let t = e.ns_at(tpb);
    play(&mut e, &mut rec, 0, t);
    e.set_chord(chord("F"), t, &mut rec);
    assert!(e.features.retrigger.looping);
    let start = e.sec_start;
    let now = t + 3 * beat_ns;
    play(&mut e, &mut rec, t, now);
    assert!((e.sec_start - start).abs() < 1e-6, "still in the first whole-note loop");
    // The length goes to a 32nd: one wake.
    e.set_style_settings(StyleSettings { retrigger_rate: 32, ..e.style_settings() });
    let l = 4.0 * ppq / 32.0;
    let from = rec.msgs.len();
    let wake = now + 1_000_000;
    rec.now = wake;
    e.process(wake, &mut rec);
    let pos = e.tick_at(wake) - e.sec_start;
    assert!((-1e-6..l + 1e-6).contains(&pos), "the head is within one loop of now: {pos} ticks in, loop {l}");
    let loops = (e.sec_start - start) / l;
    assert!((loops - loops.round()).abs() < 1e-6, "on the loop grid from the chord");
    // At most the head's note-ons once (plus one restart at the wake's own loop).
    let head = e.style.sections[e.cur].as_ref().unwrap().events.iter().filter(|ev| (ev.tick as f64) < l && matches!(ev.kind, PKind::On { vel, .. } if vel > 0)).count();
    let got = note_ons_since(&rec, from);
    assert!(got <= 2 * head.max(1), "{got} note-ons in one wake (head has {head})");
    // It carries on looping the short head.
    let mut n = wake;
    for _ in 0..40 {
        n += 3_000_000;
        rec.now = n;
        e.process(n, &mut rec);
        assert!(e.tick_at(n) - e.sec_start <= l + 1e-6);
    }
}

/// The same shortening with a Main queued inside the skipped loops: the change still
/// comes at its bar line, not skipped with them.
#[test]
fn shortening_the_retrigger_length_keeps_a_queued_change() {
    let Some((mut e, mut rec)) = started(StyleSettings { retrigger_rate: 1, ..StyleSettings::default() }) else { return };
    let (ppq, tpb, beat_ns) = grid(&e);
    e.button(Button::AutoFill, 0, &mut rec); // off: Main B itself, no fill
    e.button(Button::Retrigger, 0, &mut rec);
    let t = e.ns_at(tpb + 1.5 * ppq);
    play(&mut e, &mut rec, 0, t);
    e.set_chord(chord("F"), t, &mut rec);
    e.button(Button::Main(1), t, &mut rec);
    let due = e.queued.expect("Main B queued").at;
    // Past the bar line Main B waits for, with the length shortened in the same wake.
    let now = e.ns_at(due) + beat_ns / 2;
    e.set_style_settings(StyleSettings { retrigger_rate: 32, ..e.style_settings() });
    rec.now = now;
    e.process(now, &mut rec);
    assert_eq!(e.cur, slot_of(SectionId::Main(1)), "Main B came");
    assert!((e.sec_start - due).abs() < 1e-6, "at its bar line");
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

fn other_style() -> Option<Box<Prepared>> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/TickingAway.T162.sty");
    p.exists().then(|| Box::new(Prepared::new(&Style::load(&p).unwrap())))
}

/// Into Ending I at bar 2, then pressed again at its first beat: the ritardando runs.
fn in_ending_rit() -> Option<(Engine, Rec, u64)> {
    let (mut e, mut rec) = started(StyleSettings::default())?;
    let (ppq, tpb, _) = grid(&e);
    let end1 = slot_of(SectionId::Ending(0));
    if !e.style.has(end1) {
        return None;
    }
    let t = e.ns_at(tpb + 1.5 * ppq);
    play(&mut e, &mut rec, 0, t);
    e.button(Button::Ending(0), t, &mut rec);
    let bar2 = e.ns_at(2.0 * tpb) + 1_000;
    play(&mut e, &mut rec, t, bar2);
    assert_eq!(e.cur, end1);
    e.button(Button::Ending(0), bar2, &mut rec);
    assert!(e.ritardando());
    Some((e, rec, bar2))
}

/// A style chosen while an Ending plays waits for the Ending to end, even in the Ending's
/// first beat (owner-confirmed Genos behaviour); a Main's first beat still changes at once.
#[test]
fn a_style_change_in_an_endings_first_beat_waits_for_its_end() {
    for timing in [MainTiming::NextBar, MainTiming::Immediate] {
        let Some((mut e, mut rec)) = started(StyleSettings { main_timing: timing, ..StyleSettings::default() }) else { return };
        let Some(other) = other_style() else { return };
        let (ppq, tpb, _) = grid(&e);
        let end1 = slot_of(SectionId::Ending(0));
        if !e.style.has(end1) {
            return;
        }
        let t = e.ns_at(tpb + 1.5 * ppq);
        play(&mut e, &mut rec, 0, t);
        e.button(Button::Ending(0), t, &mut rec);
        let bar2 = e.ns_at(2.0 * tpb) + 1_000;
        play(&mut e, &mut rec, t, bar2);
        assert_eq!(e.cur, end1);
        let (old_bpm, new_bpm) = (e.style.bpm, other.bpm);
        assert_ne!(old_bpm, new_bpm);
        e.change_style(other, bar2, &mut rec);
        assert!(e.style_pending(), "{timing:?}: waits");
        let mut now = bar2;
        while e.running {
            let next = e.next_deadline().unwrap_or(now + 5_000_000).clamp(now + 1, now + 5_000_000);
            play(&mut e, &mut rec, now, next);
            now = next;
            if e.running {
                assert_eq!(e.cur, end1, "{timing:?}: the Ending plays to its end in the old style");
                assert_eq!(e.style.bpm, old_bpm);
            }
            assert!(now < bar2 + 60_000_000_000, "the Ending never ended");
        }
        assert!(!e.style_pending(), "{timing:?}: the new style took over at the Ending's end");
        assert_eq!(e.style.bpm, new_bpm);
    }
}

/// A style chosen while an Ending is queued but not playing yet (#111) waits for that
/// Ending's end too: the Ending plays in the old style, and the new style is loaded at the
/// stop (owner rule: a style change waits for the Ending to finish).
#[test]
fn a_style_chosen_while_an_ending_is_queued_waits_for_its_end() {
    for timing in [MainTiming::NextBar, MainTiming::Immediate] {
        let Some((mut e, mut rec)) = started(StyleSettings { main_timing: timing, ..StyleSettings::default() }) else { return };
        let Some(other) = other_style() else { return };
        let (ppq, tpb, _) = grid(&e);
        let end1 = slot_of(SectionId::Ending(0));
        if !e.style.has(end1) {
            return;
        }
        let t = e.ns_at(tpb + 1.5 * ppq);
        play(&mut e, &mut rec, 0, t);
        e.button(Button::Ending(0), t, &mut rec);
        assert!(matches!(id_of(e.cur), SectionId::Main(_)), "the Ending is only queued");
        let (old_bpm, new_bpm) = (e.style.bpm, other.bpm);
        assert_ne!(old_bpm, new_bpm);
        e.change_style(other, t + 1_000, &mut rec);
        let at = e.pending.as_ref().map(|p| p.at).expect("the style waits");
        let len = e.style.sections[end1].as_ref().unwrap().len as f64;
        assert!((at - (2.0 * tpb + len)).abs() < 1e-6, "{timing:?}: for the queued Ending's end, not the bar line: {at}");
        let mut now = t + 1_000;
        let mut heard_ending = false;
        while e.running {
            let next = e.next_deadline().unwrap_or(now + 5_000_000).clamp(now + 1, now + 5_000_000);
            play(&mut e, &mut rec, now, next);
            now = next;
            if e.running {
                assert_eq!(e.style.bpm, old_bpm, "{timing:?}: the old style plays on");
                heard_ending |= e.cur == end1;
            }
            assert!(now < t + 60_000_000_000, "the Ending never ended");
        }
        assert!(heard_ending, "{timing:?}: the Ending played, in the old style");
        assert!(!e.style_pending(), "{timing:?}: the new style took over at the Ending's end");
        assert_eq!(e.style.bpm, new_bpm);
    }
}

/// TAP TEMPO during a ritardando (Style Section Reset off): the tapped tempo is the one the
/// band slows from and comes back to at the stop.
#[test]
fn tap_tempo_during_a_ritardando_is_the_tempo_it_comes_back_to() {
    let Some((mut e, mut rec, t)) = in_ending_rit() else { return };
    e.set_style_settings(StyleSettings { section_reset: false, ..e.style_settings() });
    play(&mut e, &mut rec, t, t + 200_000_000);
    let t1 = t + 200_000_000;
    e.button(Button::TapTempo, t1, &mut rec);
    e.button(Button::TapTempo, t1 + 500_000_000, &mut rec);
    assert!(e.ritardando(), "still slowing");
    assert!((e.features.rit.base - 120.0).abs() < 1e-6, "tapped 120: base {}", e.features.rit.base);
    assert!(e.bpm < 120.0 && e.bpm > 120.0 * RIT_END, "slows on from the tapped tempo: {}", e.bpm);
    let mut now = t1 + 500_000_000;
    let mut last = e.bpm;
    while e.running {
        now = e.next_deadline().unwrap().max(now + 1).min(now + 5_000_000);
        rec.now = now;
        e.process(now, &mut rec);
        if e.running {
            assert!(e.bpm <= last + 1e-9, "only slows: {} after {last}", e.bpm);
            last = e.bpm;
        }
    }
    assert!((e.bpm - 120.0).abs() < 1e-6, "back to the tapped tempo: {}", e.bpm);
}

/// Stopped during a ritardando with a style change waiting: the new style comes in at its
/// own tempo, not the one the ritardando started from.
#[test]
fn a_style_waiting_through_a_ritardando_keeps_its_own_tempo() {
    let (Some((mut e, mut rec, t)), Some(other)) = (in_ending_rit(), other_style()) else { return };
    let want = other.bpm;
    play(&mut e, &mut rec, t, t + 300_000_000);
    e.change_style(other, t + 300_000_000, &mut rec);
    e.button(Button::StartStop, t + 310_000_000, &mut rec);
    assert!(!e.running && !e.ritardando());
    assert_eq!(e.bpm, want);
}

/// Section Reset during an Ending's ritardando: the Ending starts over at the tempo the
/// ritardando started from.
#[test]
fn section_reset_ends_the_ritardando() {
    let Some((mut e, mut rec, t)) = in_ending_rit() else { return };
    let base = e.features.rit.base;
    play(&mut e, &mut rec, t, t + 800_000_000);
    assert!(e.bpm < base);
    e.button(Button::SectionReset, t + 800_000_000, &mut rec);
    assert!(!e.ritardando());
    assert_eq!(e.bpm, base);
    play(&mut e, &mut rec, t + 800_000_000, t + 1_600_000_000);
    assert_eq!(e.bpm, base, "no stale ritardando");
}

/// A style change while the Ending slows waits for the Ending's end (owner-confirmed):
/// the Ending slows on to its end in the old style, and the new style comes in at the
/// stop at its own tempo, not the slowed one.
#[test]
fn a_style_change_mid_ritardando_waits_for_the_endings_end() {
    let (Some((mut e, mut rec, t)), Some(other)) = (in_ending_rit(), other_style()) else { return };
    let base = e.features.rit.base;
    let (old_bpm, new_bpm) = (e.style.bpm, other.bpm);
    e.change_style(other, t + 10_000_000, &mut rec);
    let at = e.pending.as_ref().map(|p| p.at).expect("the style waits");
    assert!((at - e.section_end().0).abs() < 1e-6, "for the Ending's end");
    let mut now = t + 10_000_000;
    let mut last = e.bpm;
    while e.running {
        now = e.next_deadline().unwrap().max(now + 1).min(now + 5_000_000);
        rec.now = now;
        e.process(now, &mut rec);
        if !e.running {
            break;
        }
        assert!(e.pending.is_some() && e.style.bpm == old_bpm, "the old style plays to the end");
        assert!(e.ritardando(), "still slowing");
        assert!(e.bpm <= last + 1e-9, "only slows: {} after {last}", e.bpm);
        last = e.bpm;
    }
    assert!(last < base * 0.8 && last >= base * RIT_END - 1e-6, "{last} of {base}");
    assert!(e.pending.is_none() && e.style.bpm == new_bpm, "the new style is in");
    assert!((e.bpm - new_bpm).abs() < 0.5, "at its own tempo: {} vs {new_bpm}", e.bpm);
}

/// A new style while the Main's head loops (review #94 r3): the new style's Main plays on
/// from where it comes in; nothing loops until a chord is played in it.
#[test]
fn a_style_change_ends_the_retrigger_loop() {
    let settings = StyleSettings { retrigger_rate: 4, ..StyleSettings::default() };
    let (Some((mut e, mut rec)), Some(other)) = (started(settings), other_style()) else { return };
    let (ppq, tpb, beat_ns) = grid(&e);
    e.button(Button::Retrigger, 0, &mut rec);
    let t = e.ns_at(tpb + 0.5 * ppq);
    play(&mut e, &mut rec, 0, t);
    e.set_chord(chord("F"), t, &mut rec);
    assert!(e.features.retrigger.looping);
    e.change_style(other, t + 1_000_000, &mut rec);
    let mut now = t + 1_000_000;
    while e.pending.is_some() {
        let next = now + 5_000_000;
        play(&mut e, &mut rec, now, next);
        now = next;
        assert!(now < t + 20 * beat_ns, "the style never came in");
    }
    assert!(e.retrigger_on(), "Retrigger stays on");
    assert!(!e.features.retrigger.looping, "the new style's Main does not loop its head");
    let (ppq, _, beat_ns) = grid(&e);
    let from = e.sec_start;
    play(&mut e, &mut rec, now, now + 2 * beat_ns);
    assert!(e.tick_at(now + 2 * beat_ns) - from > ppq + 1e-6 && (e.sec_start - from).abs() < 1e-6, "it plays past the head");
}

/// Retrigger with the chord-settle window (#101): the restart is at the chord, and the
/// head's chord-part notes wait for the settle and start once, on the settled chord. No
/// note is struck and cut again within the window (#47, #65), for a new chord or the same
/// one struck again.
#[test]
fn a_retrigger_chord_settles_once() {
    let settings = StyleSettings { retrigger_rate: 4, ..StyleSettings::default() };
    let Some((mut e, mut rec)) = started(settings) else { return };
    let win = 10_000_000;
    e.set_chord_settle(win);
    let (_, _, beat) = grid(&e);
    e.button(Button::Retrigger, 0, &mut rec);
    let mut now = 0;
    for (k, name) in ["F", "F", "G7", "G7", "C"].into_iter().enumerate() {
        let t = beat * (5 + 7 * k as u64) / 3;
        play(&mut e, &mut rec, now, t);
        now = t;
        let from = rec.msgs.len();
        e.set_chord(chord(name), now, &mut rec);
        assert!(e.features.retrigger.looping);
        play(&mut e, &mut rec, now, now + 4 * win);
        now += 4 * win;
        let sent = &rec.msgs[from..];
        assert!(sent.iter().any(|(_, m)| m[0] & 0xF0 == 0x90 && m[2] > 0 && (9..16).contains(&(m[0] & 0x0F))), "{name}: the head plays");
        for (i, (t_on, m)) in sent.iter().enumerate() {
            if m[0] & 0xF0 != 0x90 || m[2] == 0 {
                continue;
            }
            let cut = sent[i + 1..].iter().find(|(_, o)| (o[0] & 0xF0 == 0x80 || o[0] & 0xF0 == 0x90 && o[2] == 0) && o[0] & 0x0F == m[0] & 0x0F && o[1] == m[1]);
            if let Some((t_off, _)) = cut {
                assert!(t_off - t_on >= win, "{name}: ch{} note {} struck at {t_on} and cut at {t_off}", m[0] & 0x0F, m[1]);
            }
        }
    }
}

/// The same chord struck again (the input thread publishes it since review #94 r3) with
/// Retrigger off is no chord change: when it settles, the Retrigger Rules move nothing,
/// so the band plays exactly as if it had not been struck. On BluesOrganTrio the walking
/// bass jumped to the root (and on Thrust a part bent) at every re-strike.
#[test]
fn a_restruck_chord_moves_no_note() {
    for name in ["BluesOrganTrio.S930.STY", "Thrust.S930.STY"] {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2").join(name);
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return;
        }
        let style = Style::load(&p).unwrap();
        // `a` has the chord struck again and again; `b` only its changes. A settle window
        // of 0 settles in the same wake, so nothing is held back and the two can match.
        let (mut a, mut b) = (Engine::new(Box::new(Prepared::new(&style))), Engine::new(Box::new(Prepared::new(&style))));
        let (mut ra, mut rb) = (Rec::default(), Rec::default());
        for (e, r) in [(&mut a, &mut ra), (&mut b, &mut rb)] {
            e.set_chord_settle(0);
            e.set_chord(chord("C"), 0, r);
        }
        let (_, _, beat) = grid(&a);
        let mut now = 0;
        for i in 1..40u64 {
            let t = i * beat * 3 / 7;
            play(&mut a, &mut ra, now, t);
            play(&mut b, &mut rb, now, t);
            now = t;
            let c = chord(if i < 20 { "C" } else { "Am7" });
            a.set_chord(c, now, &mut ra);
            if i == 20 {
                b.set_chord(c, now, &mut rb);
            }
        }
        play(&mut a, &mut ra, now, now + beat);
        play(&mut b, &mut rb, now, now + beat);
        let first = ra.msgs.iter().zip(&rb.msgs).position(|(x, y)| x != y);
        assert!(first.is_none() && ra.msgs.len() == rb.msgs.len(), "{name}: re-struck differs at {first:?}: {:?} vs {:?}", first.map(|i| &ra.msgs[i]), first.map(|i| &rb.msgs[i]));
    }
}

