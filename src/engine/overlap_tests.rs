//! A pattern note struck again on its own key before it ends (#127): each note-off ends
//! the note struck first, so the new note sounds its full length.

use super::prepared::PEvent;
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

/// SlowWalker with Main A's notes replaced by `notes` (tick, source channel, key, velocity;
/// velocity 0 is a note-off).
fn engine_with(notes: &[(u32, u8, u8, u8)]) -> Option<Engine> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
    if !p.exists() {
        eprintln!("corpus missing; skipping");
        return None;
    }
    let mut prep = Prepared::new(&Style::load(&p).unwrap());
    let sec = prep.sections[slot_of(SectionId::Main(0))].as_mut().unwrap();
    sec.events.retain(|e| !matches!(e.kind, PKind::On { .. } | PKind::Off { .. }));
    for &(tick, src, key, vel) in notes {
        let kind = if vel > 0 { PKind::On { key, vel } } else { PKind::Off { key } };
        sec.events.push(PEvent { tick, src, kind });
    }
    sec.events.sort_by_key(|e| (e.tick, !matches!(e.kind, PKind::Off { .. })));
    Some(Engine::new(Box::new(prep)))
}

/// The source channel Main A plays on part `dest`.
fn src_for(e: &Engine, dest: u8) -> u8 {
    let sec = e.style.sections[slot_of(SectionId::Main(0))].as_ref().unwrap();
    (0..16u8).find(|&s| sec.rules[s as usize].as_ref().is_some_and(|r| r.dest_ch == dest)).unwrap()
}

/// Play the first 1100 ticks on a C chord; the notes on `ch` as (tick, key, velocity; 0 = off).
fn notes_on(e: &mut Engine, ch: u8) -> Vec<(u32, u8, u8)> {
    let mut rec = Rec::default();
    e.set_chord(Chord::new(0, 0), 0, &mut rec);
    assert!(e.running);
    let end = e.ns_at(1100.0);
    let mut now = 0;
    while now < end {
        rec.now = now;
        e.process(now, &mut rec);
        now = e.next_deadline().unwrap_or(end).clamp(now + 1, end);
    }
    let tick = |t: u64| ((t - e.ns_at(0.0)) as f64 / (e.ns_at(1000.0) - e.ns_at(0.0)) as f64 * 1000.0).round() as u32;
    rec.msgs
        .iter()
        .filter(|(_, m)| m.len() == 3 && m[0] & 0x0F == ch && matches!(m[0] & 0xF0, 0x80 | 0x90))
        .map(|(t, m)| (tick(*t), m[1], if m[0] & 0xF0 == 0x90 { m[2] } else { 0 }))
        .collect()
}

#[test]
fn a_note_struck_again_before_it_ends_sounds_its_full_length() {
    let Some(e) = engine_with(&[]) else { return };
    let src = src_for(&e, BASS_CH);
    // A legato bass: the second C is struck at 480 while the first still sounds (it ends
    // at 540); the second ends at 1000.
    let mut e = engine_with(&[(0, src, 36, 99), (480, src, 36, 70), (540, src, 36, 0), (1000, src, 36, 0)]).unwrap();
    let got = notes_on(&mut e, BASS_CH);
    let k = got[0].1;
    assert_eq!(got, vec![(0, k, 99), (480, k, 0), (480, k, 70), (1000, k, 0)], "the second note must not end at the first one's note-off");
}

#[test]
fn a_drum_hit_doubled_on_one_tick_ends_at_the_last_note_off() {
    let Some(e) = engine_with(&[]) else { return };
    let src = src_for(&e, 9);
    let mut e = engine_with(&[(0, src, 38, 90), (0, src, 38, 60), (100, src, 38, 0), (300, src, 38, 0)]).unwrap();
    let got = notes_on(&mut e, 9);
    assert_eq!(got, vec![(0, 38, 90), (0, 38, 0), (0, 38, 60), (300, 38, 0)]);
}
