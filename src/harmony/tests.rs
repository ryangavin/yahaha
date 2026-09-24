use super::*;
use crate::theory::{Chord, NUM_TYPES};

const C: Chord = Chord::new(0, 0);
const CM: Chord = Chord::new(0, 8);
const C7: Chord = Chord::new(0, 19);
const CDIM: Chord = Chord::new(0, 17);
const CAUG: Chord = Chord::new(0, 7);
const CHORDS: [Chord; 5] = [C, CM, C7, CDIM, CAUG];

/// C5 (72) is a tone of all five test chords; D5 (74) of none.
const ON: u8 = 72;
const OFF: u8 = 74;

type Row = (HarmonyType, u8, [&'static [u8]; 5]);

/// Expected keys, highest first, for each type, melody and chord (C, Cm, C7, Cdim, Caug).
#[rustfmt::skip]
const TABLE: &[Row] = &[
    (T::StandardDuet1, ON,  [&[67], &[67], &[67], &[66], &[68]]),
    (T::StandardDuet1, OFF, [&[67], &[67], &[70], &[66], &[68]]),
    (T::StandardDuet2, ON,  [&[64], &[63], &[64], &[63], &[64]]),
    (T::StandardDuet2, OFF, [&[64], &[63], &[67], &[63], &[64]]),
    (T::StandardTrio,  ON,  [&[67, 64], &[67, 63], &[67, 64], &[66, 63], &[68, 64]]),
    (T::StandardTrio,  OFF, [&[67, 64], &[67, 63], &[70, 67], &[66, 63], &[68, 64]]),
    (T::FullChord,     ON,  [&[67, 64, 60], &[67, 63, 60], &[70, 67, 64, 60], &[66, 63, 60], &[68, 64, 60]]),
    (T::FullChord,     OFF, [&[67, 64, 60, 48], &[67, 63, 60, 48], &[70, 67, 64, 60, 48], &[66, 63, 60, 48], &[68, 64, 60, 48]]),
    (T::RockDuet,      ON,  [&[67], &[67], &[67], &[66], &[68]]),
    (T::RockDuet,      OFF, [&[67], &[67], &[67], &[66], &[68]]),
    (T::CountryDuet1,  ON,  [&[76], &[75], &[76], &[75], &[76]]),
    (T::CountryDuet1,  OFF, [&[79], &[79], &[79], &[78], &[80]]),
    (T::CountryDuet2,  ON,  [&[64], &[63], &[64], &[63], &[64]]),
    (T::CountryDuet2,  OFF, [&[67], &[67], &[67], &[66], &[68]]),
    (T::CountryTrio,   ON,  [&[76, 67], &[75, 67], &[76, 67], &[75, 66], &[76, 68]]),
    (T::CountryTrio,   OFF, [&[79, 64], &[79, 63], &[79, 70], &[78, 63], &[80, 64]]),
    (T::Block,         ON,  [&[69, 67, 64, 60], &[69, 67, 63, 60], &[70, 67, 64, 60], &[69, 66, 63, 60], &[70, 68, 64, 60]]),
    (T::Block,         OFF, [&[69, 67, 64, 62], &[69, 67, 63, 62], &[70, 67, 64, 62], &[69, 66, 63, 62], &[70, 68, 64, 62]]),
    (T::FourWayClose1, ON,  [&[69, 67, 64], &[69, 67, 63], &[70, 67, 64], &[69, 66, 63], &[70, 68, 64]]),
    (T::FourWayClose1, OFF, [&[69, 67, 64], &[69, 67, 63], &[70, 67, 64], &[69, 66, 63], &[70, 68, 64]]),
    // Close 2 on C: Cmaj7 with C on top trades the B (a minor 9th) for the 6th.
    (T::FourWayClose2, ON,  [&[69, 67, 64], &[70, 67, 63], &[70, 67, 64], &[69, 66, 63], &[70, 68, 64]]),
    (T::FourWayClose2, OFF, [&[71, 67, 64], &[70, 67, 63], &[70, 67, 64], &[69, 66, 63], &[70, 68, 64]]),
    (T::FourWayClose3, ON,  [&[69, 67, 64], &[69, 67, 63], &[70, 67, 64], &[69, 66, 63], &[70, 68, 64]]),
    (T::FourWayClose3, OFF, [&[69, 67, 64], &[69, 67, 63], &[70, 67, 64], &[69, 66, 63], &[70, 68, 64]]),
    (T::FourWayClose4, ON,  [&[69, 67, 64, 60], &[70, 67, 63, 60], &[70, 67, 64, 60], &[69, 66, 63, 60], &[70, 68, 64, 60]]),
    (T::FourWayClose4, OFF, [&[71, 67, 64, 62], &[70, 67, 63, 62], &[70, 67, 64, 62], &[69, 66, 63, 62], &[70, 68, 64, 62]]),
    (T::FourWayOpen1,  ON,  [&[67, 64, 57], &[67, 63, 57], &[67, 64, 58], &[66, 63, 57], &[68, 64, 58]]),
    (T::FourWayOpen1,  OFF, [&[67, 64, 57], &[67, 63, 57], &[67, 64, 58], &[66, 63, 57], &[68, 64, 58]]),
    (T::FourWayOpen2,  ON,  [&[69, 64, 55], &[69, 63, 55], &[70, 64, 55], &[69, 63, 54], &[70, 64, 56]]),
    (T::FourWayOpen2,  OFF, [&[69, 64, 55], &[69, 63, 55], &[70, 64, 55], &[69, 63, 54], &[70, 64, 56]]),
    (T::FourWayOpen3,  ON,  [&[67, 57, 52], &[67, 57, 51], &[67, 58, 52], &[66, 57, 51], &[68, 58, 52]]),
    (T::FourWayOpen3,  OFF, [&[67, 57, 52], &[67, 57, 51], &[67, 58, 52], &[66, 57, 51], &[68, 58, 52]]),
    (T::OnePlusFive,   ON,  [&[79], &[79], &[79], &[79], &[79]]),
    (T::OnePlusFive,   OFF, [&[81], &[81], &[81], &[81], &[81]]),
    (T::Octave,        ON,  [&[60], &[60], &[60], &[60], &[60]]),
    (T::Octave,        OFF, [&[62], &[62], &[62], &[62], &[62]]),
    (T::Strum,         ON,  [&[67, 64], &[67, 63], &[70, 67, 64], &[66, 63], &[68, 64]]),
    (T::Strum,         OFF, [&[67, 64, 60], &[67, 63, 60], &[70, 67, 64], &[66, 63, 60], &[68, 64, 60]]),
    (T::MultiAssign,   ON,  [&[], &[], &[], &[], &[]]),
    (T::MultiAssign,   OFF, [&[], &[], &[], &[], &[]]),
    (T::Echo,          ON,  [&[], &[], &[], &[], &[]]),
    (T::Tremolo,       ON,  [&[], &[], &[], &[], &[]]),
    (T::Trill,         ON,  [&[], &[], &[], &[], &[]]),
];

#[test]
fn table_every_type_on_and_off_chord() {
    for ty in ALL_TYPES {
        assert!(TABLE.iter().any(|r| r.0 == ty), "{} missing from the table", ty.name());
    }
    for &(ty, melody, expect) in TABLE {
        for (chord, want) in CHORDS.iter().zip(expect) {
            let got = voice(ty, melody, Some(*chord));
            assert_eq!(got.keys(), want, "{} melody {melody} over {}", ty.name(), chord.name());
        }
    }
}

#[test]
fn table_transposes_with_the_chord() {
    // Every root gives the same shape, shifted.
    for &(ty, melody, expect) in TABLE {
        for (chord, want) in CHORDS.iter().zip(expect) {
            for r in 1..12u8 {
                let c = Chord::new((chord.root + r) % 12, chord.ty);
                let got = voice(ty, melody + r, Some(c));
                let shifted: Vec<u8> = want.iter().map(|k| k + r).collect();
                assert_eq!(got.keys(), &shifted[..], "{} root {r}", ty.name());
            }
        }
    }
}

#[test]
fn close3_uses_the_ninth() {
    // E over C: Close 1 = C A G; Close 3 swaps the root for D.
    assert_eq!(voice(T::FourWayClose1, 76, Some(C)).keys(), &[72, 69, 67]);
    assert_eq!(voice(T::FourWayClose3, 76, Some(C)).keys(), &[74, 69, 67]);
}

#[test]
fn strum_delays_step_down_from_the_melody() {
    let v = voice(T::Strum, ON, Some(C7));
    assert_eq!(v.keys(), &[70, 67, 64]);
    assert_eq!(v.delays_ms(), &[15, 30, 45]);
    assert!(voice(T::StandardTrio, ON, Some(C)).delays_ms().iter().all(|&d| d == 0));
}

fn chord_following_tones_only(ty: HarmonyType) -> bool {
    matches!(ty, T::StandardDuet1 | T::StandardDuet2 | T::StandardTrio | T::FullChord | T::RockDuet
        | T::CountryDuet1 | T::CountryDuet2 | T::CountryTrio | T::Strum)
}

/// Invariants over every root, chord type and a wide melody range.
#[test]
fn invariants_every_chord_every_melody() {
    for ty in ALL_TYPES {
        for root in 0..12u8 {
            for cty in 0..38u8 {
                let chord = Chord::new(root, cty);
                for melody in 0..=127u8 {
                    let v = voice(ty, melody, Some(chord));
                    assert_eq!(v, voice(ty, melody, Some(chord)), "deterministic");
                    let keys = v.keys();
                    assert!(keys.len() <= MAX_NOTES);
                    assert!(keys.windows(2).all(|w| w[0] > w[1]), "{} {keys:?} not strictly descending", ty.name());
                    let m = melody as i16;
                    for &k in keys {
                        let k = k as i16;
                        assert_ne!(k, m);
                        assert_ne!((k - m).abs(), 1, "{} semitone against melody", ty.name());
                        match ty.direction() {
                            Direction::Below => assert!(k < m, "{} {k} not below {m}", ty.name()),
                            Direction::Above => assert!(k > m, "{} {k} not above {m}", ty.name()),
                            _ => {}
                        }
                    }
                    if ty == T::CountryTrio && keys.len() == 2 {
                        assert!(keys[0] > melody && keys[1] < melody);
                    }
                    if chord_following_tones_only(ty) && cty != CANCEL {
                        for &k in keys {
                            assert!(is_chord_note(k, chord), "{} {k} not in {}", ty.name(), chord.name());
                        }
                    }
                    if cty == CANCEL && ty.uses_chord() {
                        assert!(keys.is_empty());
                    }
                }
            }
        }
    }
}

#[test]
fn no_panic_on_hostile_input() {
    for ty in ALL_TYPES {
        for root in [0u8, 11, 12, 15, 255] {
            for cty in [0u8, 33, 34, 37, 38, 63, 255] {
                for melody in [0u8, 1, 11, 12, 126, 127, 128, 255] {
                    let c = Chord { root, ty: cty, bass: Some(root) };
                    let _ = voice(ty, melody, Some(c));
                    let s = HarmonySettings { ty, ..Default::default() };
                    let _ = harmonize(melody, 127, Some(c), &s, RightParts::all_on());
                }
            }
        }
    }
}

#[test]
fn chordless_types_ignore_the_chord() {
    for ty in [T::OnePlusFive, T::Octave] {
        let a = voice(ty, 70, None);
        assert!(!a.is_empty());
        for c in CHORDS {
            assert_eq!(voice(ty, 70, Some(c)), a);
        }
    }
    // Chord-following types need a chord.
    assert!(voice(T::StandardDuet1, 72, None).is_empty());
    assert!(voice(T::StandardDuet1, 72, Some(Chord::new(0, CANCEL))).is_empty());
}

#[test]
fn edges_of_the_keyboard_drop_notes_rather_than_wrap() {
    assert!(voice(T::Octave, 5, None).is_empty());
    assert!(voice(T::OnePlusFive, 125, None).is_empty());
    assert!(voice(T::StandardDuet1, 1, Some(C)).is_empty());
}

// ---------------------------------------------------------------------------
// Chord source
// ---------------------------------------------------------------------------

#[test]
fn chord_zone_table() {
    let (st, lt) = (54u8, 59u8); // F#2 style split, B2 left split
    let z = chord_zone(true, false, st, lt);
    assert_eq!((z.source, z.chord_top, z.left_voice, z.right_bottom), (ChordSource::Style, Some(54), None, 55));
    let z = chord_zone(false, true, st, lt);
    assert_eq!((z.source, z.chord_top, z.left_voice, z.right_bottom), (ChordSource::Left, Some(59), Some((0, 59)), 60));
    let z = chord_zone(true, true, st, lt);
    assert_eq!((z.source, z.chord_top, z.left_voice, z.right_bottom), (ChordSource::Style, Some(54), Some((55, 59)), 60));
    let z = chord_zone(false, false, st, lt);
    assert_eq!((z.source, z.chord_top, z.left_voice, z.right_bottom), (ChordSource::None, None, None, 0));
    // "Style + Left": one shared area, no separate LEFT-only range.
    let z = chord_zone(true, true, 54, 54);
    assert_eq!((z.chord_top, z.left_voice, z.right_bottom), (Some(54), None, 55));
    // Left below Style reads as equal; the top key never overflows.
    assert_eq!(chord_zone(false, true, 60, 50).chord_top, Some(60));
    assert_eq!(chord_zone(true, false, 127, 127).right_bottom, 128);
}

#[test]
fn harmony_chord_picks_the_source() {
    let (s, l) = (Some(C), Some(CM));
    let ty = T::StandardDuet1;
    assert_eq!(harmony_chord(ty, true, false, s, l), s);
    assert_eq!(harmony_chord(ty, false, true, s, l), l);
    assert_eq!(harmony_chord(ty, true, true, s, l), s);
    assert_eq!(harmony_chord(ty, false, false, s, l), None);
    assert_eq!(harmony_chord(ty, true, false, Some(Chord::new(0, CANCEL)), l), None);
    for ty in [T::OnePlusFive, T::Octave, T::MultiAssign, T::Echo, T::Tremolo, T::Trill] {
        assert_eq!(harmony_chord(ty, true, true, s, l), None, "{}", ty.name());
    }
}

// ---------------------------------------------------------------------------
// Detail settings
// ---------------------------------------------------------------------------

fn set(ty: HarmonyType) -> HarmonySettings {
    HarmonySettings { ty, volume: 127, ..Default::default() }
}

fn keys(h: &Harmony) -> Vec<u8> {
    h.notes().iter().map(|n| n.key).collect()
}

#[test]
fn chord_note_only() {
    let mut s = set(T::StandardTrio);
    s.chord_note_only = true;
    let p = RightParts::only(RIGHT1);
    assert_eq!(keys(&harmonize(ON, 100, Some(C), &s, p)), [67, 64]);
    assert!(harmonize(OFF, 100, Some(C), &s, p).is_empty());
    assert!(harmonize(ON, 100, None, &s, p).is_empty());
    // Off: the passing note is harmonised too.
    s.chord_note_only = false;
    assert_eq!(keys(&harmonize(OFF, 100, Some(C), &s, p)), [67, 64]);
    // The chordless types are not affected.
    let mut o = set(T::Octave);
    o.chord_note_only = true;
    assert_eq!(keys(&harmonize(OFF, 100, Some(C), &o, p)), [62]);
}

#[test]
fn minimum_velocity_is_inclusive() {
    let mut s = set(T::StandardDuet1);
    s.min_velocity = 90;
    let p = RightParts::only(RIGHT1);
    assert!(harmonize(ON, 89, Some(C), &s, p).is_empty());
    assert_eq!(keys(&harmonize(ON, 90, Some(C), &s, p)), [67]);
    assert!(harmonize(ON, 0, Some(C), &s, p).is_empty());
}

#[test]
fn volume_scales_effect_velocity() {
    assert_eq!(effect_velocity(100, 127), 100);
    assert_eq!(effect_velocity(100, 100), 79);
    assert_eq!(effect_velocity(127, 64), 64);
    assert_eq!(effect_velocity(1, 1), 1);
    assert_eq!(effect_velocity(100, 0), 0);
    assert_eq!(effect_velocity(255, 255), 127);
    let mut s = set(T::StandardDuet1);
    s.volume = 64;
    let h = harmonize(ON, 127, Some(C), &s, RightParts::only(RIGHT1));
    assert_eq!(h.notes()[0].vel, 64);
    s.volume = 0;
    assert!(harmonize(ON, 127, Some(C), &s, RightParts::only(RIGHT1)).is_empty());
}

#[test]
fn assign_routing() {
    let all = RightParts::all_on();
    let (r1, r2, r3) = (1u8, 2u8, 4u8);
    // Auto: melody layered on all, effect on the first part in R1, R2, R3 order.
    let r = route(Assign::Auto, Category::Harmony, all, 3);
    assert_eq!((r.melody, &r.effect[..3]), (7, &[r1, r1, r1][..]));
    let r = route(Assign::Auto, Category::Harmony, RightParts { on: [false, true, true], mono: [false; 3] }, 1);
    assert_eq!((r.melody, r.effect[0]), (6, r2));
    // Multi: melody on R1 only, effect over the others, then back to R1.
    let r = route(Assign::Multi, Category::Harmony, all, 4);
    assert_eq!((r.melody, &r.effect[..4]), (r1, &[r2, r3, r1, r2][..]));
    // Multi with one part: everything there.
    let r = route(Assign::Multi, Category::Harmony, RightParts::only(RIGHT2), 2);
    assert_eq!((r.melody, &r.effect[..2]), (r2, &[r2, r2][..]));
    // Fixed part, on or off.
    assert_eq!(route(Assign::Right3, Category::Harmony, all, 1).effect[0], r3);
    assert_eq!(route(Assign::Right3, Category::Harmony, RightParts::only(RIGHT1), 1).effect[0], 0);
    // Mono parts are off for the Harmony category (RM p.46 example), not for Echo.
    let mono_r1 = RightParts { on: [true, true, false], mono: [true, false, false] };
    let r = route(Assign::Auto, Category::Harmony, mono_r1, 1);
    assert_eq!((r.melody, r.effect[0]), (3, r2));
    assert_eq!(route(Assign::Auto, Category::Echo, mono_r1, 1).effect[0], r1);
    // Multi keeps a Mono part's melody.
    let r = route(Assign::Multi, Category::Harmony, RightParts { on: [true; 3], mono: [true, false, false] }, 2);
    assert_eq!((r.melody, &r.effect[..2]), (r1 | r2, &[r3, r2][..]));
}

#[test]
fn harmonize_routes_notes() {
    let mut s = set(T::StandardTrio);
    s.assign = Assign::Multi;
    let h = harmonize(ON, 100, Some(C), &s, RightParts::all_on());
    assert_eq!(h.melody_parts, 1);
    let parts: Vec<u8> = h.notes().iter().map(|n| n.parts).collect();
    assert_eq!(parts, [2, 4]);
    // No eligible part: no notes.
    s.assign = Assign::Auto;
    let h = harmonize(ON, 100, Some(C), &s, RightParts { on: [true, false, false], mono: [true, false, false] });
    assert!(h.is_empty());
    assert_eq!(h.melody_parts, 1);
}

#[test]
fn non_harmony_types_return_nothing_from_harmonize() {
    for ty in [T::MultiAssign, T::Echo, T::Tremolo, T::Trill] {
        assert!(harmonize(ON, 100, Some(C), &set(ty), RightParts::all_on()).is_empty());
    }
}

#[test]
fn melody_is_the_top_key() {
    assert_eq!(melody_of(&[60, 72, 67]), Some(72));
    assert_eq!(melody_of(&[]), None);
}

#[test]
fn tracker_returns_what_a_key_added() {
    let mut t = HarmonyTracker::new();
    let s = set(T::StandardTrio);
    let a = harmonize(ON, 100, Some(C), &s, RightParts::all_on());
    assert_eq!(t.press(ON, a), None);
    // The chord changes while held: release still returns the original notes.
    assert_eq!(t.release(ON), Some(a));
    assert_eq!(t.release(ON), None);
    assert_eq!(t.press(ON, a), None);
    assert_eq!(t.press(ON, Harmony::NONE), Some(a));
    t.press(60, a);
    t.press(64, a);
    let mut n = 0;
    t.release_all(|_, _| n += 1);
    assert_eq!(n, 2);
    assert_eq!(t.release(60), None);
    assert_eq!(t.press(200, a), None);
}

#[test]
fn multi_assign_press_order() {
    let mut m = MultiAssign::new();
    let all = RightParts::all_on();
    // Press order, not pitch: G, C, E -> R1, R2, R3.
    assert_eq!(m.press(67, all), 1);
    assert_eq!(m.press(60, all), 2);
    assert_eq!(m.press(64, all), 4);
    // A fourth key wraps round.
    assert_eq!(m.press(72, all), 1);
    // Release frees the part for the next key.
    assert_eq!(m.release(60), 2);
    assert_eq!(m.press(62, all), 2);
    assert_eq!(m.release(99), 0);
    m.reset();
    // Only the parts that are on.
    let p = RightParts { on: [false, true, true], mono: [false; 3] };
    assert_eq!(m.press(60, p), 2);
    assert_eq!(m.press(64, p), 4);
    assert_eq!(m.press(67, RightParts::default()), 0);
    assert_eq!(m.press(200, all), 0);
}

// ---------------------------------------------------------------------------
// Echo, Tremolo, Trill timing
// ---------------------------------------------------------------------------

const MS: u64 = 1_000_000;

fn echo(ty: HarmonyType, speed: EchoSpeed, volume: u8) -> EchoGen {
    EchoGen::new(&HarmonySettings { ty, speed, volume, ..Default::default() }, 120.0)
}

/// Every event up to `until`, polling at each due time like the engine would.
fn run(g: &mut EchoGen, until: u64) -> Vec<EchoEvent> {
    let mut out = Vec::new();
    let mut buf = [EchoEvent::default(); 4];
    while let Some(t) = g.next_due() {
        if t > until {
            break;
        }
        let n = g.next_events(t, &mut buf);
        assert!(n > 0, "due at {t} but nothing came");
        for e in &buf[..n] {
            assert!(e.at <= t);
        }
        out.extend_from_slice(&buf[..n]);
    }
    out
}

fn ev(at_ms: u64, key: u8, vel: u8, effect: bool) -> EchoEvent {
    EchoEvent { at: at_ms * MS, key, vel, effect }
}

#[test]
fn speed_periods() {
    let want = [500.0, 333.333, 250.0, 166.667, 125.0, 62.5];
    for (s, w) in EchoSpeed::ALL.iter().zip(want) {
        let p = s.period_ns(120.0) as f64 / MS as f64;
        assert!((p - w).abs() < 0.01, "{} {p}", s.name());
    }
    assert_eq!(EchoSpeed::Eighth.period_ns(0.0), EchoSpeed::Eighth.period_ns(120.0));
    assert_eq!(EchoSpeed::Eighth.period_ns(f64::NAN), EchoSpeed::Eighth.period_ns(120.0));
    assert_eq!(EchoSpeed::ThirtySecond.period_ns(1e9), MS);
}

#[test]
fn tremolo_repeats_at_the_note_value() {
    let mut g = echo(T::Tremolo, EchoSpeed::Eighth, 100);
    g.note_on(60, 100, 0);
    // Nothing is due early.
    let mut buf = [EchoEvent::default(); 8];
    assert_eq!(g.next_due(), Some(0));
    let evs = run(&mut g, 600 * MS);
    let expect = [
        ev(0, 60, 100, false),
        ev(187, 60, 0, false),
        ev(250, 60, 79, true),
        ev(437, 60, 0, true),
        ev(500, 60, 79, true),
    ];
    // 187.5 ms gates.
    let mut e = expect;
    e[1].at = 187_500_000;
    e[3].at = 437_500_000;
    assert_eq!(evs, e);
    assert_eq!(g.next_events(600 * MS, &mut buf), 0);
    // Release mid-note: an immediate off, then silence.
    g.note_off(60, 600 * MS);
    assert_eq!(run(&mut g, 10_000 * MS), [ev(600, 60, 0, true)]);
    assert_eq!(g.next_due(), None);
}

#[test]
fn echo_decays_then_stops() {
    let mut g = echo(T::Echo, EchoSpeed::Quarter, 127);
    g.note_on(72, 100, 0);
    let ons: Vec<(u64, u8)> = run(&mut g, 20_000 * MS).iter().filter(|e| e.vel > 0).map(|e| (e.at / MS, e.vel)).collect();
    assert_eq!(ons, [(0, 100), (500, 100), (1000, 75), (1500, 56), (2000, 42), (2500, 31), (3000, 23), (3500, 17), (4000, 12), (4500, 9), (5000, 6), (5500, 4)]);
    // Faded out but still held: nothing more is due.
    assert_eq!(g.next_due(), None);
    g.note_off(72, 30_000 * MS);
    assert_eq!(run(&mut g, u64::MAX - 1), []);
}

#[test]
fn trill_alternates_the_last_two_keys() {
    let mut g = echo(T::Trill, EchoSpeed::Eighth, 127);
    g.note_on(60, 100, 0);
    // One key: it just sounds.
    assert_eq!(run(&mut g, 99 * MS), [ev(0, 60, 100, false)]);
    g.note_on(62, 90, 100 * MS);
    let evs = run(&mut g, 700 * MS);
    let at = |e: &EchoEvent| e.at / MS;
    let ons: Vec<(u64, u8)> = evs.iter().filter(|e| e.vel > 0).map(|e| (at(e), e.key)).collect();
    // Newer key first, then alternating every 250 ms.
    assert_eq!(ons, [(100, 62), (350, 60), (600, 62)]);
    // The lone key was stopped when the trill started, before the trill's first note.
    assert_eq!(evs[0], ev(100, 60, 0, false));
    // Each trill note ends (225 ms gate) before the next starts.
    let offs: Vec<u64> = evs.iter().skip(1).filter(|e| e.vel == 0).map(|e| e.at).collect();
    assert_eq!(offs, [325 * MS, 575 * MS]);
    // A third key: trill the last two (62, 64), newest first.
    g.note_on(64, 100, 700 * MS);
    let evs = run(&mut g, 1000 * MS);
    assert_eq!(evs[0], ev(700, 62, 0, true));
    let ons: Vec<(u64, u8)> = evs.iter().filter(|e| e.vel > 0).map(|e| (at(e), e.key)).collect();
    assert_eq!(ons, [(700, 64), (950, 62)]);
    // Releasing one of the pair: the trill falls back to the last two still held (60, 62).
    g.note_off(64, 1000 * MS);
    let evs = run(&mut g, 1000 * MS);
    assert_eq!(evs.iter().filter(|e| e.vel > 0).map(|e| e.key).collect::<Vec<_>>(), [62]);
    // Down to one key: it sounds plainly until released.
    g.note_off(60, 1100 * MS);
    let evs = run(&mut g, 5000 * MS);
    assert_eq!(evs.last(), Some(&ev(1100, 62, 90, false)));
    g.note_off(62, 5000 * MS);
    assert_eq!(run(&mut g, 9000 * MS), [ev(5000, 62, 0, false)]);
    assert_eq!(g.next_due(), None);
}

#[test]
fn below_minimum_velocity_sounds_plainly() {
    let mut g = EchoGen::new(&HarmonySettings { ty: T::Tremolo, min_velocity: 64, ..Default::default() }, 120.0);
    g.note_on(60, 40, 0);
    assert_eq!(run(&mut g, 5000 * MS), [ev(0, 60, 40, false)]);
    g.note_off(60, 5000 * MS);
    assert_eq!(run(&mut g, 9000 * MS), [ev(5000, 60, 0, false)]);
}

#[test]
fn tempo_change_applies_to_later_repeats() {
    let mut g = echo(T::Tremolo, EchoSpeed::Quarter, 127);
    g.note_on(60, 100, 0);
    run(&mut g, 0);
    // 500 ms repeat already scheduled; afterwards the period is 250 ms (240 BPM).
    g.set_tempo(240.0);
    let ons: Vec<u64> = run(&mut g, 1000 * MS).iter().filter(|e| e.vel > 0).map(|e| e.at / MS).collect();
    assert_eq!(ons, [500, 750, 1000]);
}

#[test]
fn next_events_respects_buffer_size_and_time_order() {
    let mut g = echo(T::Tremolo, EchoSpeed::Sixteenth, 127);
    for k in 60..70 {
        g.note_on(k, 100, 0);
    }
    // Poll late: everything due by 1 s comes out in time order across calls.
    let mut all = Vec::new();
    let mut buf = [EchoEvent::default(); 3];
    loop {
        let n = g.next_events(1000 * MS, &mut buf);
        if n == 0 {
            break;
        }
        all.extend_from_slice(&buf[..n]);
    }
    assert!(all.windows(2).all(|w| (w[0].at, w[0].vel > 0) <= (w[1].at, w[1].vel > 0)));
    // 10 keys, repeats at 0, 125 .. 1000 ms: 9 ons and 8 offs each.
    assert_eq!(all.iter().filter(|e| e.vel > 0).count(), 90);
    assert_eq!(all.iter().filter(|e| e.vel == 0).count(), 80);
}

#[test]
fn all_off_silences_everything() {
    let mut g = echo(T::Trill, EchoSpeed::Eighth, 127);
    g.note_on(60, 100, 0);
    g.note_on(62, 100, 0);
    run(&mut g, 10 * MS);
    g.all_off(20 * MS);
    let evs = run(&mut g, 10_000 * MS);
    // 62 is the trill's first (struck) note, so its off carries the same flag as its on.
    assert_eq!(evs, [ev(20, 62, 0, false)]);
    assert_eq!(g.next_due(), None);
}

#[test]
fn voice_overflow_drops_the_oldest() {
    let mut g = echo(T::Tremolo, EchoSpeed::Eighth, 127);
    for k in 0..MAX_ECHO_VOICES as u8 + 1 {
        g.note_on(40 + k, 100, 0);
    }
    let evs = run(&mut g, 0);
    // The first key's queued strike never sounds; the 17th key does.
    assert!(evs.iter().any(|e| e.key == 40 + MAX_ECHO_VOICES as u8 && e.vel > 0));
    assert!(!evs.iter().any(|e| e.key == 40 && e.vel > 0));
    // Hostile input is ignored.
    g.note_on(200, 100, 0);
    g.note_off(200, 0);
}

#[test]
fn every_type_is_named_and_categorised() {
    assert_eq!(ALL_TYPES.len(), 23);
    let echo: Vec<_> = ALL_TYPES.iter().filter(|t| t.category() == Category::Echo).collect();
    assert_eq!(echo, [&T::Echo, &T::Tremolo, &T::Trill]);
    for t in ALL_TYPES {
        assert!(!t.name().is_empty());
    }
    let _ = NUM_TYPES;
}

// ---------------------------------------------------------------------------
// Review fixes (#76)
// ---------------------------------------------------------------------------

#[test]
fn rock_duet_takes_any_root_byte() {
    // Root bytes are not range-checked upstream; the fifth must not overflow.
    for root in [11u8, 250, 255] {
        let _ = voice(T::RockDuet, 72, Some(Chord::new(root, 0)));
    }
}

#[test]
fn close3_keeps_an_altered_ninth() {
    // C7(b9): the rootless 3-5-b7-b9, never a natural 9th against the b9.
    // (A b9 can still stand in for the #9 a semitone under an E melody: both are in the
    // altered scale.)
    for ty in [25u8, 27] {
        let c = Chord::new(0, ty);
        for melody in 48..=96u8 {
            let v = voice(T::FourWayClose3, melody, Some(c));
            assert!(!v.keys().iter().any(|k| k % 12 == 2), "type {ty} melody {melody}: {:?}", v.keys());
            if ty == 25 {
                assert!(v.keys().iter().all(|k| is_chord_note(*k, c)), "melody {melody}: {:?}", v.keys());
            }
        }
    }
    // E on top of C7(b9): Db Bb G under it.
    assert_eq!(voice(T::FourWayClose3, 76, Some(Chord::new(0, 25))).keys(), [73, 70, 67]);
}

#[test]
fn drop_voicings_never_make_a_minor_ninth() {
    for ty in [T::FourWayOpen1, T::FourWayOpen2, T::FourWayOpen3] {
        for root in 0..12u8 {
            for cty in 0..NUM_TYPES as u8 {
                for melody in 24..=120u8 {
                    let v = voice(ty, melody, Some(Chord::new(root, cty)));
                    let mut all = v.keys().to_vec();
                    all.push(melody);
                    for (i, &a) in all.iter().enumerate() {
                        for &b in &all[i + 1..] {
                            let d = (a as i16 - b as i16).abs();
                            // 7(b9) has its b9 against the root by definition.
                            assert!(d != 13 || cty == 25, "{} root {root} type {cty} melody {melody}: {:?}", ty.name(), v.keys());
                        }
                    }
                }
            }
        }
    }
    // Cmaj7 with E on top: the drop-3 B would sit a b9 under C, so it stays put.
    assert_eq!(voice(T::FourWayOpen2, 64, Some(Chord::new(0, 2))).keys(), [60, 59, 55]);
}

#[test]
fn a_burst_of_plain_notes_loses_no_note_off() {
    // Presses and releases faster than the engine polls must still balance.
    let mut g = EchoGen::new(&HarmonySettings { ty: T::Echo, min_velocity: 127, ..Default::default() }, 120.0);
    for k in 0..40u8 {
        g.note_on(40 + k, 64, 0);
    }
    for k in 0..40u8 {
        g.note_off(40 + k, 0);
    }
    let mut sounding = [0i32; 128];
    for e in run(&mut g, 10 * MS) {
        sounding[e.key as usize] += if e.vel > 0 { 1 } else { -1 };
    }
    assert!(sounding.iter().all(|&s| s == 0), "{sounding:?}");
}

#[test]
fn trill_release_before_the_second_pulse_balances_per_route() {
    // With Assign = Multi the flag picks the part, so each off must match its on's flag.
    let mut g = echo(T::Trill, EchoSpeed::Eighth, 127);
    g.note_on(60, 100, 0);
    g.note_on(62, 100, MS);
    let mut evs = run(&mut g, MS);
    g.note_off(62, 2 * MS);
    g.all_off(3 * MS);
    evs.extend(run(&mut g, 10_000 * MS));
    let mut open = std::collections::HashMap::new();
    for e in evs {
        *open.entry((e.key, e.effect)).or_insert(0i32) += if e.vel > 0 { 1 } else { -1 };
    }
    assert!(open.values().all(|&n| n == 0), "{open:?}");
}

#[test]
fn absurd_tempo_does_not_overflow() {
    let mut g = EchoGen::new(&HarmonySettings { ty: T::Tremolo, ..Default::default() }, 1e-300);
    g.note_on(60, 100, 0);
    let mut buf = [EchoEvent::default(); 8];
    g.next_events(0, &mut buf);
}
