//! Tests of `megavoice` moved here from yahaha-core because they need a higher layer (engine, sff, sim).

use crate::engine::Prepared;
use crate::sff::{Ev, Section, SectionId, Style, TimedEv};
use crate::sim::{run, Step};
use crate::theory::Chord;

const NYLON: (u8, u8, u8) = (8, 0, 0);

/// Chord 1 (ch 12) on NylonGuitar and Chord 2 (ch 13) on a GM guitar play the same
/// notes: a strum noise, an open note, a dead note and a slide. Chord 2 plays all four
/// as written; Chord 1 leaves out the noise, plays the dead note as a ghost note and
/// the slide at the open zone's top.
#[test]
fn a_megavoice_part_leaves_out_its_noises_on_other_voices() {
    let at = |tick, ev| TimedEv { tick, ev };
    let mut events = Vec::new();
    for ch in [11, 12] {
        for (tick, key, vel) in [(0, 100, 64), (0, 60, 50), (480, 64, 70), (960, 67, 110)] {
            events.push(at(tick, Ev::NoteOn { ch, key, vel }));
            events.push(at(tick + 400, Ev::NoteOff { ch, key }));
        }
    }
    events.sort_by_key(|e| e.tick);
    let id = SectionId::Main(0);
    let mut init = Vec::new();
    for (ch, (msb, lsb, prog)) in [(11, NYLON), (12, (0, 0, 24))] {
        init.extend([Ev::Cc { ch, cc: 0, val: msb }, Ev::Cc { ch, cc: 32, val: lsb }, Ev::Pc { ch, prog }]);
    }
    let style = Style {
        name: "megavoice".into(),
        format: String::new(),
        ppq: 480,
        tempo_us: 500_000,
        timesig: (4, 4),
        init,
        sections: [(id, Section { id, start: 0, len: 1920, events })].into(),
        casm: vec![],
        ots: vec![],
        opaque_sections: vec![],
        timing_changes: vec![],
        other_chunks: vec![],
    };
    let (_, rec) = run(Box::new(Prepared::new(&style)), &[(0, Step::Chord(Chord::new(0, 0)))], 1_900_000_000);
    let ons = |ch: u8| -> Vec<(u8, u8)> {
        rec.out.iter().filter(|(_, m)| m[0] == 0x90 | ch && m[2] > 0).map(|(_, m)| (m[1], m[2])).collect()
    };
    let gm = ons(12);
    assert_eq!(gm.len(), 4, "Chord 2 plays everything: {gm:?}");
    let (noise, open, dead, slide) = (gm[0], gm[1], gm[2], gm[3]);
    assert_eq!((noise.1, open.1), (64, 50), "{gm:?}");
    assert_eq!(ons(11), vec![open, (dead.0, 30), (slide.0, 60)], "Chord 1 on NylonGuitar");
    // Every note that went out also ended.
    let offs = rec.out.iter().filter(|(_, m)| m[0] & 0xF0 == 0x80 && m[0] & 0x0F == 11 || m[0] == 0x9B && m[2] == 0).count();
    assert_eq!(offs, 3);
}
