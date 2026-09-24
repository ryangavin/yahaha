use super::*;
use std::collections::HashSet;

const PPQ: u32 = 480;
const C: u8 = 60;
const E: u8 = 64;
const G: u8 = 67;

fn arp(name: &str) -> Arp {
    Arp::new(PPQ, library::find(name).unwrap_or_else(|| panic!("no pattern {name}")).clone())
}

fn with(name: &str, s: Settings) -> Arp {
    let mut a = arp(name);
    a.set_settings(s, 0);
    a
}

/// Processes `from..to` in 37-tick chunks.
fn run(a: &mut Arp, from: u64, to: u64) -> Vec<ArpEvent> {
    let mut ev = Vec::new();
    let mut t = from;
    while t < to {
        let e = (t + 37).min(to);
        a.process(t..e, &mut ev);
        t = e;
    }
    ev
}

fn ons(ev: &[ArpEvent]) -> Vec<(u64, u8, u8)> {
    ev.iter().filter_map(|e| match *e {
        ArpEvent::On { tick, note, vel } => Some((tick, note, vel)),
        _ => None,
    }).collect()
}

fn notes(ev: &[ArpEvent]) -> Vec<u8> {
    ons(ev).iter().map(|x| x.1).collect()
}

fn ticks(ev: &[ArpEvent]) -> Vec<u64> {
    ons(ev).iter().map(|x| x.0).collect()
}

fn offs(ev: &[ArpEvent]) -> Vec<(u64, u8)> {
    ev.iter().filter_map(|e| match *e {
        ArpEvent::Off { tick, note } => Some((tick, note)),
        _ => None,
    }).collect()
}

/// Checks the stream is in time order, never starts a sounding pitch or stops a silent
/// one, and returns how many pitches are left sounding.
fn check(ev: &[ArpEvent]) -> usize {
    let mut on = [false; 128];
    let mut last = 0;
    for (i, e) in ev.iter().enumerate() {
        assert!(e.tick() >= last, "event {i} {e:?} goes back in time (after {last})");
        last = e.tick();
        match *e {
            ArpEvent::On { note, vel, .. } => {
                assert!(!on[note as usize], "event {i}: note {note} started twice");
                assert!((1..=127).contains(&vel), "event {i}: velocity {vel}");
                on[note as usize] = true;
            }
            ArpEvent::Off { note, .. } => {
                assert!(on[note as usize], "event {i}: off for silent note {note}");
                on[note as usize] = false;
            }
        }
    }
    on.iter().filter(|x| **x).count()
}

fn chord(a: &mut Arp, keys: &[u8], tick: u64) {
    for &k in keys {
        a.note_on(k, 100, tick);
    }
}

fn release(a: &mut Arp, keys: &[u8], tick: u64) {
    for &k in keys {
        a.note_off(k, tick);
    }
}

// --- library -------------------------------------------------------------------

#[test]
fn library_is_valid_and_covers_every_category() {
    assert!(library::PATTERNS.len() >= 20);
    let mut names = HashSet::new();
    for p in library::PATTERNS {
        p.validate().unwrap();
        assert!(names.insert(p.name.to_lowercase()), "duplicate {}", p.name);
    }
    for c in [Category::UpDown, Category::Random, Category::AsPlayed, Category::ChordStab,
              Category::BrokenChord, Category::Guitar, Category::Sequence] {
        assert!(library::PATTERNS.iter().any(|p| p.category == c), "{c:?} empty");
    }
}

#[test]
fn every_pattern_plays_balanced_with_one_to_six_notes() {
    for p in library::PATTERNS {
        for n in 1..=6u8 {
            let mut a = Arp::new(PPQ, p.clone());
            let keys: Vec<u8> = (0..n).map(|i| 48 + i * 5).collect();
            chord(&mut a, &keys, 0);
            let mut ev = run(&mut a, 0, 4 * 1920);
            assert!(!ons(&ev).is_empty(), "{} with {n} notes is silent", p.name);
            release(&mut a, &keys, 4 * 1920);
            ev.extend(run(&mut a, 4 * 1920, 6 * 1920));
            assert_eq!(check(&ev), 0, "{} with {n} notes", p.name);
            assert_eq!(a.sounding(), 0);
        }
    }
}

// --- categories ------------------------------------------------------------------

#[test]
fn up_walks_the_chord_with_accents() {
    let mut a = arp("Climb 16");
    chord(&mut a, &[G, C, E], 0);
    let ev = run(&mut a, 0, 960);
    assert_eq!(notes(&ev), [C, E, G, C, E, G, C, E]);
    assert_eq!(ticks(&ev), [0, 120, 240, 360, 480, 600, 720, 840]);
    let vels: Vec<u8> = ons(&ev).iter().map(|x| x.2).collect();
    assert_eq!(vels, [100, 64, 80, 64, 100, 64, 80, 64]);
    // Gate 80% of a 16th, then 60%.
    assert_eq!(offs(&ev)[..2], [(96, C), (192, E)]);
}

#[test]
fn down_walks_from_the_top() {
    let mut a = arp("Fall 16");
    chord(&mut a, &[C, E, G], 0);
    assert_eq!(notes(&run(&mut a, 0, 720)), [G, E, C, G, E, C]);
}

#[test]
fn up_down_spans_octaves_without_repeating_the_ends() {
    let mut a = arp("Peak 8");
    chord(&mut a, &[C, E, G], 0);
    let n = notes(&run(&mut a, 0, 12 * 240));
    assert_eq!(n, [60, 64, 67, 72, 76, 79, 76, 72, 67, 64, 60, 64]);
}

#[test]
fn down_up_in_triplets() {
    let mut a = arp("Valley Triplet");
    chord(&mut a, &[C, E, G], 0);
    let ev = run(&mut a, 0, 960);
    assert_eq!(notes(&ev), [G, E, C, E, G, E]);
    assert_eq!(ticks(&ev), [0, 160, 320, 480, 640, 800]);
}

#[test]
fn random_is_seeded_repeatable_and_never_repeats_a_note() {
    let play = || {
        let mut a = arp("Dice 16");
        chord(&mut a, &[C, E, G, 71], 0);
        notes(&run(&mut a, 0, 64 * 120))
    };
    let first = play();
    assert_eq!(first, play());
    assert_eq!(first.len(), 64);
    assert!(first.windows(2).all(|w| w[0] != w[1]));
    assert!(first.iter().all(|n| [C, E, G, 71].contains(n)));
    let distinct: HashSet<_> = first.iter().collect();
    assert_eq!(distinct.len(), 4);

    // Restarting the pattern restarts the sequence.
    let mut a = arp("Dice 16");
    chord(&mut a, &[C, E, G, 71], 0);
    let one = notes(&run(&mut a, 0, 16 * 120));
    release(&mut a, &[C, E, G, 71], 16 * 120);
    chord(&mut a, &[C, E, G, 71], 3000);
    let two = notes(&run(&mut a, 16 * 120, 3000 + 16 * 120));
    assert_eq!(one, two);
}

#[test]
fn random_octaves_stays_in_range() {
    let mut a = arp("Scatter Octaves");
    chord(&mut a, &[C, E, G], 0);
    let n = notes(&run(&mut a, 0, 100 * 240));
    assert!(n.iter().all(|&x| [C, E, G, C + 12, E + 12, G + 12].contains(&x)));
    assert!(n.iter().any(|&x| x > G));
}

#[test]
fn as_played_keeps_press_order() {
    let mut a = arp("Echo Order 8");
    chord(&mut a, &[G, C, E], 0);
    assert_eq!(notes(&run(&mut a, 0, 1440)), [G, C, E, G, C, E]);
}

#[test]
fn chord_stabs_play_the_whole_chord() {
    let mut a = arp("Four Stabs");
    chord(&mut a, &[E, C, G], 0);
    let ev = run(&mut a, 0, 960);
    assert_eq!(ons(&ev), [(0, C, 110), (0, E, 110), (0, G, 110), (480, C, 90), (480, E, 90), (480, G, 90)]);
    // Gate 40% of a quarter.
    assert!(offs(&ev).iter().take(3).all(|&(t, _)| t == 192));

    let mut a = arp("Offbeat Pump");
    chord(&mut a, &[C, E, G], 0);
    let t: HashSet<_> = ticks(&run(&mut a, 0, 960)).into_iter().collect();
    assert_eq!(t, HashSet::from([240, 720]));
}

#[test]
fn broken_chord_figures() {
    let mut a = arp("Alberti 16");
    chord(&mut a, &[C, E, G], 0);
    assert_eq!(notes(&run(&mut a, 0, 480)), [C, G, E, G]);

    // Indexes past the chord wrap up an octave.
    let mut a = arp("Rolling Eights");
    chord(&mut a, &[C, E, G], 0);
    assert_eq!(notes(&run(&mut a, 0, 1920)), [60, 64, 67, 72, 76, 72, 67, 64]);

    // Waltz: bass an octave down, then two chords.
    let mut a = arp("Waltz Broken");
    chord(&mut a, &[C, E, G], 0);
    assert_eq!(notes(&run(&mut a, 0, 1440)), [48, C, E, G, C, E, G]);
}

#[test]
fn guitar_strums_roll_across_the_chord() {
    let mut a = arp("Strum Quarters");
    chord(&mut a, &[C, E, G], 0);
    let ev = run(&mut a, 0, 480);
    assert_eq!(ons(&ev), [(0, C, 110), (20, E, 110), (40, G, 110)]);

    // An upstroke goes high to low: step 3 of Campfire Strum, an 8th pattern.
    let mut a = arp("Campfire Strum");
    chord(&mut a, &[C, E, G], 0);
    let ev = run(&mut a, 0, 960);
    let up: Vec<_> = ons(&ev).into_iter().filter(|x| (720..960).contains(&x.0)).map(|x| (x.0, x.1)).collect();
    assert_eq!(up, [(720, G), (734, E), (748, C)]);
    assert_eq!(check(&ev), a.sounding());
}

#[test]
fn sequences_jump_octaves() {
    let mut a = arp("Octave Pulse");
    chord(&mut a, &[C, E, G], 0);
    assert_eq!(notes(&run(&mut a, 0, 960)), [60, 72, 60, 60, 72, 60, 72, 60]);
}

#[test]
fn played_sort_counts_in_press_order() {
    let mut p = library::find("Alberti 16").unwrap().clone();
    p.sort = Sort::Played;
    let mut a = Arp::new(PPQ, p);
    chord(&mut a, &[G, C, E], 0);
    // Idx 0 = first pressed (G), Top 0 = last pressed (E), Idx 1 = C.
    assert_eq!(notes(&run(&mut a, 0, 480)), [G, E, C, E]);
}

// --- timing ---------------------------------------------------------------------

#[test]
fn ticks_do_not_drift() {
    // 1920 PPQ: a 16th is exactly 480.
    let mut a = Arp::new(1920, library::find("Climb 16").unwrap().clone());
    chord(&mut a, &[C, E, G], 1000);
    let t = ticks(&run(&mut a, 1000, 1000 + 1000 * 480));
    assert_eq!(t.len(), 1000);
    assert!(t.iter().enumerate().all(|(k, &x)| x == 1000 + k as u64 * 480));

    // Unit Multiply 133% of a 16th at 480 PPQ is 159.6 ticks: rounded per step, never accumulated.
    let mut a = with("Climb 16", Settings { unit_multiply: 133, ..Settings::default() });
    chord(&mut a, &[C, E, G], 0);
    let t = ticks(&run(&mut a, 0, 1000 * 160));
    assert_eq!(t[999], 999 * 1596 / 10);
    assert_eq!(t[1000], 159_600);
}

#[test]
fn chunking_does_not_change_the_output() {
    let play = |chunk: u64| {
        let mut a = arp("Campfire Strum");
        chord(&mut a, &[C, E, G], 5);
        let mut ev = Vec::new();
        let mut t = 5;
        while t < 5000 {
            a.process(t..t + chunk, &mut ev);
            t += chunk;
        }
        ev
    };
    let one = play(1);
    assert_eq!(one, play(7));
    assert_eq!(one, play(4995));
}

#[test]
fn unit_multiply_changes_speed_from_the_next_step() {
    let mut a = arp("Climb 16");
    chord(&mut a, &[C, E, G], 0);
    let mut ev = run(&mut a, 0, 250); // steps at 0, 120, 240
    a.set_settings(Settings { unit_multiply: 200, ..Settings::default() }, 250);
    ev.extend(run(&mut a, 250, 1000));
    assert_eq!(ticks(&ev), [0, 120, 240, 360, 600, 840]);
}

#[test]
fn gate_scale_and_minimum_gate() {
    let mut a = with("Four Stabs", Settings { gate_scale: 50, ..Settings::default() });
    chord(&mut a, &[C], 0);
    let ev = run(&mut a, 0, 480);
    assert_eq!(offs(&ev), [(96, C)]);
}

#[test]
fn swing_delays_every_second_step() {
    // Shuffle Order 16 swings 62%: odd 16ths move 2 * 12% of a 16th = 28.8 ticks later.
    let mut a = arp("Shuffle Order 16");
    chord(&mut a, &[C, E], 0);
    assert_eq!(ticks(&run(&mut a, 0, 480)), [0, 148, 240, 388]);

    let mut p = library::find("Climb 16").unwrap().clone();
    p.swing = 75;
    let mut a = Arp::new(PPQ, p);
    chord(&mut a, &[C, E], 0);
    assert_eq!(ticks(&run(&mut a, 0, 480)), [0, 180, 240, 420]);
}

#[test]
fn quantize_starts_on_the_nearest_grid_line() {
    let q = Settings { quantize: Quantize::Sixteenth, ..Settings::default() };
    // Early: 190 snaps forward to 240.
    let mut a = with("Climb 16", q);
    chord(&mut a, &[C, E, G], 190);
    assert_eq!(ticks(&run(&mut a, 190, 600)), [240, 360, 480]);
    // Late: 130 snaps back to 120, so the first note plays now and the rest are on the grid.
    let mut a = with("Climb 16", q);
    chord(&mut a, &[C, E, G], 130);
    assert_eq!(ticks(&run(&mut a, 130, 500)), [130, 240, 360, 480]);
    // Eighths.
    let mut a = with("Climb 16", Settings { quantize: Quantize::Eighth, ..Settings::default() });
    chord(&mut a, &[C], 1100);
    assert_eq!(ticks(&run(&mut a, 1100, 1300)), [1200]);
    // Off: straight away.
    let mut a = arp("Climb 16");
    chord(&mut a, &[C], 1001);
    assert_eq!(ticks(&run(&mut a, 1001, 1300))[0], 1001);
}

// --- held notes -----------------------------------------------------------------

#[test]
fn release_stops_and_the_next_press_restarts() {
    let mut a = arp("Climb 16");
    chord(&mut a, &[C, E, G], 0);
    let mut ev = run(&mut a, 0, 250);
    release(&mut a, &[C, E, G], 250);
    assert!(!a.is_running());
    ev.extend(run(&mut a, 250, 600));
    assert_eq!(notes(&ev), [C, E, G]);
    assert_eq!(check(&ev), 0);
    chord(&mut a, &[C, E, G], 600);
    let ev2 = run(&mut a, 600, 700);
    assert_eq!(ons(&ev2), [(600, C, 100)]);
}

#[test]
fn notes_join_and_leave_while_playing() {
    let mut a = arp("Climb 16");
    chord(&mut a, &[C, G], 0);
    let mut ev = run(&mut a, 0, 230); // C, G
    a.note_on(E, 90, 230);
    ev.extend(run(&mut a, 230, 600)); // walk pos 2 of C E G = G, then C, E
    a.note_off(G, 600);
    ev.extend(run(&mut a, 600, 840)); // pos 5 of C E = E, then C
    assert_eq!(notes(&ev), [C, G, G, C, E, E, C]);
}

#[test]
fn hold_latches_until_a_new_chord() {
    let mut a = with("Climb 16", Settings { hold: true, ..Settings::default() });
    chord(&mut a, &[C, E, G], 0);
    let mut ev = run(&mut a, 0, 100);
    release(&mut a, &[C, E, G], 100);
    ev.extend(run(&mut a, 100, 480));
    assert_eq!(notes(&ev), [C, E, G, C]);
    assert!(a.is_running());

    // A new chord after a full release replaces the latched one and restarts the pattern.
    chord(&mut a, &[62, 65, 69], 500);
    let ev2 = run(&mut a, 480, 870);
    assert_eq!(notes(&ev2), [62, 65, 69, 62]);
    assert_eq!(ticks(&ev2)[0], 500);

    // Keys added while others are held join the chord.
    a.note_on(72, 100, 900);
    assert_eq!(a.notes().collect::<Vec<_>>(), [62, 65, 69, 72]);
    release(&mut a, &[62, 65, 69, 72], 950);

    // Hold off releases the latched chord; no notes are left hanging.
    a.set_hold(false, 1000);
    assert!(!a.is_running());
    let mut all = ev;
    all.extend(ev2);
    all.extend(run(&mut a, 870, 2000));
    assert_eq!(check(&all), 0);
}

#[test]
fn hold_off_keeps_keys_still_down() {
    let mut a = with("Climb 16", Settings { hold: true, ..Settings::default() });
    chord(&mut a, &[C, E, G], 0);
    a.note_off(E, 50);
    a.set_hold(false, 60);
    assert_eq!(a.notes().collect::<Vec<_>>(), [C, G]);
    assert!(a.is_running());
}

#[test]
fn sustain_pedal_holds_released_notes_when_enabled() {
    let mut a = with("Climb 16", Settings { sustain_holds: true, ..Settings::default() });
    chord(&mut a, &[C, E], 0);
    a.set_sustain(true, 10);
    release(&mut a, &[C, E], 20);
    a.note_on(G, 100, 30); // joins, like a sustained piano note
    let mut ev = run(&mut a, 0, 480);
    assert_eq!(notes(&ev), [C, E, G, C]);
    a.note_off(G, 480);
    a.set_sustain(false, 480);
    assert!(!a.is_running());
    ev.extend(run(&mut a, 480, 1000));
    assert_eq!(check(&ev), 0);

    // Without the option the pedal is ignored.
    let mut a = arp("Climb 16");
    chord(&mut a, &[C], 0);
    a.set_sustain(true, 10);
    a.note_off(C, 20);
    assert!(!a.is_running());
}

#[test]
fn keep_key_on_continues_the_phrase() {
    let mut a = with("Climb 16", Settings { keep_key_on: true, ..Settings::default() });
    chord(&mut a, &[C, E, G], 0);
    let mut ev = run(&mut a, 0, 250); // C E G at 0 120 240
    release(&mut a, &[C, E, G], 250);
    assert!(a.is_running());
    ev.extend(run(&mut a, 250, 500)); // silent steps at 360, 480
    chord(&mut a, &[C, E, G], 500);
    ev.extend(run(&mut a, 500, 700)); // picks up on the grid at 600, walk position 3
    assert_eq!(ons(&ev), [(0, C, 100), (120, E, 64), (240, G, 80), (600, C, 64)]);
    a.stop(700);
    ev.extend(run(&mut a, 700, 800));
    assert_eq!(check(&ev), 0);
}

// --- velocity -------------------------------------------------------------------

#[test]
fn velocity_modes() {
    let vels = |s: Settings| {
        let mut a = with("Climb 16", s);
        a.note_on(C, 30, 0);
        a.note_on(E, 90, 0);
        ons(&run(&mut a, 0, 480)).iter().map(|x| x.2).collect::<Vec<_>>()
    };
    assert_eq!(vels(Settings::default()), [100, 64, 80, 64]);
    assert_eq!(vels(Settings { velocity: Velocity::Thru, ..Settings::default() }), [30, 90, 30, 90]);
    assert_eq!(vels(Settings { velocity: Velocity::Fixed(77), ..Settings::default() }), [77; 4]);
    assert_eq!(vels(Settings { vel_scale: 150, ..Settings::default() }), [127, 96, 120, 96]);
    assert_eq!(vels(Settings { vel_scale: 0, ..Settings::default() }), [1; 4]);
}

// --- stuck notes ----------------------------------------------------------------

#[test]
fn pattern_change_cuts_sounding_notes() {
    let mut a = arp("Four Stabs"); // gate 192
    chord(&mut a, &[C, E, G], 0);
    let mut ev = run(&mut a, 0, 100);
    assert_eq!(a.sounding(), 3);
    a.set_pattern(library::find("Climb 16").unwrap().clone(), 100);
    ev.extend(run(&mut a, 100, 250));
    let o = offs(&ev);
    assert_eq!(o[..3], [(100, C), (100, E), (100, G)]);
    assert_eq!(notes(&ev), [C, E, G, C, E]);
    assert_eq!(ticks(&ev)[3..], [100, 220]);
}

#[test]
fn legato_gates_retrigger_cleanly() {
    let mut p = library::find("Octave Pulse").unwrap().clone();
    p.steps = std::borrow::Cow::Owned(vec![Step::new(Sel::Idx(0), 0, 100, 250)]);
    let mut a = Arp::new(PPQ, p);
    chord(&mut a, &[C], 0);
    let mut ev = run(&mut a, 0, 1000);
    a.all_off(1000, &mut ev);
    assert_eq!(check(&ev), 0);
    assert_eq!(a.sounding(), 0);
    // Each retrigger releases the same pitch first.
    assert_eq!(ev[1], ArpEvent::Off { tick: 120, note: C });
}

#[test]
fn stop_cuts_everything() {
    let mut a = with("Strum Quarters", Settings { hold: true, ..Settings::default() });
    chord(&mut a, &[C, E, G, 71, 74], 0);
    let mut ev = run(&mut a, 0, 50); // strum half way through
    a.stop(50);
    ev.extend(run(&mut a, 50, 2000));
    assert_eq!(check(&ev), 0);
    assert!(ev.iter().all(|e| e.tick() <= 50));
    assert!(!a.is_running());
}

// --- clock jumps and bad input ---------------------------------------------------

#[test]
fn clock_going_back_cuts_and_restarts() {
    // The style restarts at tick 0 while arp notes sound at a high tick.
    let mut a = arp("Four Stabs"); // gate 192
    chord(&mut a, &[C, E, G], 100_000);
    let mut ev = run(&mut a, 100_000, 100_010);
    assert_eq!(a.sounding(), 3);
    let back = run(&mut a, 0, 1000);
    // The old notes are released at the new tick, and the pattern starts over there.
    assert_eq!(offs(&back)[..3], [(0, C), (0, E), (0, G)]);
    assert_eq!(ticks(&back), [0, 0, 0, 480, 480, 480, 960, 960, 960]);
    ev.extend(back);
    a.all_off(1000, &mut ev);
    assert_eq!(a.sounding(), 0);
    assert_eq!(ons(&ev).len(), offs(&ev).len());
    // With Quantize on, the restart waits for the grid.
    let mut a = with("Climb 16", Settings { quantize: Quantize::Sixteenth, ..Settings::default() });
    chord(&mut a, &[C], 5_000);
    run(&mut a, 5_000, 5_050);
    assert_eq!(ticks(&run(&mut a, 30, 300)), [120, 240]);
}

#[test]
fn clock_gap_plays_one_step_not_a_burst() {
    let mut a = arp("Sky Ladder 32");
    chord(&mut a, &[C, E, G], 0);
    let mut ev = run(&mut a, 0, 100);
    let n = ev.len();
    a.process(96_000..96_010, &mut ev);
    assert_eq!(ons(&ev[n..]).len(), 1);
    assert_eq!(check(&ev), 1);
    // A late quantized start (snapped back several steps) also plays just one note now.
    let mut a = with("Sky Ladder 32", Settings { quantize: Quantize::Eighth, ..Settings::default() });
    chord(&mut a, &[C], 1070); // snaps back to 960: the 32nds at 960 and 1020 are late
    assert_eq!(ticks(&run(&mut a, 1070, 1150)), [1070, 1080, 1140]);
}

#[test]
fn empty_ranges_do_nothing() {
    let mut a = arp("Climb 16");
    chord(&mut a, &[C, E], 0);
    let mut ev = Vec::new();
    a.process(0..0, &mut ev);
    let (from, to) = (500, 100);
    a.process(from..to, &mut ev);
    assert!(ev.is_empty());
}

#[test]
fn unvalidated_patterns_do_not_panic_or_hang() {
    let mut p = library::find("Climb 16").unwrap().clone();
    p.steps = std::borrow::Cow::Owned(vec![]);
    let mut a = Arp::new(PPQ, p);
    chord(&mut a, &[C], 0);
    assert!(run(&mut a, 0, 480).is_empty());

    let mut p = library::find("Climb 16").unwrap().clone();
    p.step_len = 0;
    let mut a = Arp::new(PPQ, p);
    chord(&mut a, &[C], 0);
    let mut ev = run(&mut a, 0, 480);
    a.all_off(480, &mut ev);
    assert_eq!(check(&ev), 0);
}

struct Rng(u32);
impl Rng {
    fn next(&mut self, n: u32) -> u32 {
        self.0 = xorshift(self.0);
        self.0 % n
    }
}

#[test]
fn fuzz_note_offs_always_balance() {
    for seed_ in 1..=40u32 {
        let mut r = Rng(seed_ * 7919);
        let pats = library::PATTERNS;
        let mut a = Arp::new(if seed_ % 2 == 0 { 480 } else { 1920 }, pats[r.next(pats.len() as u32) as usize].clone());
        let mut ev = Vec::new();
        let mut t = 0u64;
        let mut down: Vec<u8> = Vec::new();
        for _ in 0..3000 {
            let dt = r.next(200) as u64;
            a.process(t..t + dt, &mut ev);
            t += dt;
            match r.next(20) {
                0..=6 => {
                    let n = 36 + r.next(60) as u8;
                    a.note_on(n, 1 + r.next(127) as u8, t);
                    if !down.contains(&n) {
                        down.push(n);
                    }
                }
                7..=12 => {
                    if !down.is_empty() {
                        let n = down.swap_remove(r.next(down.len() as u32) as usize);
                        a.note_off(n, t);
                    }
                }
                13 => a.set_hold(r.next(2) == 0, t),
                14 => a.set_sustain(r.next(2) == 0, t),
                15 => a.set_pattern(pats[r.next(pats.len() as u32) as usize].clone(), t),
                16 => {
                    let s = Settings {
                        quantize: [Quantize::Off, Quantize::Eighth, Quantize::Sixteenth][r.next(3) as usize],
                        hold: r.next(2) == 0,
                        velocity: [Velocity::Original, Velocity::Thru, Velocity::Fixed(r.next(140) as u8)][r.next(3) as usize],
                        vel_scale: r.next(250) as u16,
                        gate_scale: r.next(500) as u16,
                        unit_multiply: r.next(500) as u16,
                        keep_key_on: r.next(2) == 0,
                        sustain_holds: r.next(2) == 0,
                    };
                    a.set_settings(s, t);
                }
                17 => {
                    for n in down.drain(..) {
                        a.note_off(n, t);
                    }
                }
                18 => {
                    if r.next(4) == 0 {
                        a.stop(t);
                        down.clear();
                    }
                }
                _ => {
                    if r.next(8) == 0 {
                        a.all_off(t, &mut ev);
                        down.clear();
                    }
                }
            }
        }
        a.stop(t);
        a.process(t..t + 1, &mut ev);
        assert_eq!(check(&ev), 0, "seed {seed_}");
        assert_eq!(a.sounding(), 0, "seed {seed_}");
        assert!(ons(&ev).len() > 100, "seed {seed_} played too little");
    }
}

#[test]
fn releasing_everything_never_leaves_a_note_hanging() {
    for p in library::PATTERNS {
        let mut a = Arp::new(PPQ, p.clone());
        chord(&mut a, &[C, E, G, 71], 0);
        let mut ev = run(&mut a, 0, 1000);
        release(&mut a, &[C, E, G, 71], 1000);
        // Longest gate in the library is under a bar, so everything is off after two.
        ev.extend(run(&mut a, 1000, 1000 + 2 * 1920));
        assert_eq!(check(&ev), 0, "{}", p.name);
    }
}

// --- wiring helpers (set_ppq, next_due, keys_down) ----------------------------------

/// A style with another resolution takes over: the notes sounding are cut at once (their
/// off ticks were on the old clock) and the pattern starts again on the new one, keeping
/// the held notes.
#[test]
fn set_ppq_cuts_and_restarts_on_the_new_clock() {
    let mut a = arp("Climb 16");
    chord(&mut a, &[C, E, G], 0);
    let mut ev = run(&mut a, 0, 130);
    assert!(a.sounding() > 0);
    a.set_ppq(960, 1000, &mut ev);
    assert_eq!(a.sounding(), 0);
    assert_eq!(a.ppq(), 960);
    assert_eq!(check(&ev), 0, "every note cut");
    let more = run(&mut a, 1000, 1000 + 960);
    assert_eq!(notes(&more)[..3], [C, E, G], "the pattern starts again from step 1");
    // A 16th at 960 PPQ is 240 ticks.
    assert_eq!(ticks(&more)[..3], [1000, 1240, 1480]);
    assert_eq!(a.keys_down().collect::<Vec<_>>(), [C, E, G]);
    a.set_ppq(0, 5000, &mut Vec::new());
    assert_eq!(a.ppq(), 960, "a zero ppq changes nothing");
}

/// `next_due` is the earliest off, queued note or step, so the engine can sleep until it;
/// whatever is due goes out when its tick is processed.
#[test]
fn next_due_is_the_next_event() {
    let mut a = arp("Climb 16");
    assert_eq!(a.next_due(), None);
    chord(&mut a, &[C, E], 10);
    assert_eq!(a.next_due(), Some(10));
    let mut t = 10;
    while t < 2000 {
        let d = a.next_due().unwrap();
        assert!(d >= t);
        let mut ev = Vec::new();
        a.process(t..d + 1, &mut ev);
        assert!(!ev.is_empty() && ev.iter().all(|e| e.tick() == d), "{ev:?} at {d}");
        t = d + 1;
    }
    release(&mut a, &[C, E], t);
    let mut ev = Vec::new();
    a.process(t..t + 1000, &mut ev);
    assert_eq!(a.next_due(), None);
}

/// Hold keeps the notes after release; `keys_down` lists only the keys still down.
#[test]
fn keys_down_leaves_out_latched_notes() {
    let mut a = with("Climb 16", Settings { hold: true, ..Settings::default() });
    chord(&mut a, &[C, E, G], 0);
    release(&mut a, &[E], 10);
    assert_eq!(a.keys_down().collect::<Vec<_>>(), [C, G]);
    assert_eq!(a.notes().count(), 3);
}
