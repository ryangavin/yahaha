//! MegaVoices on voices that are not MegaVoices (#223).
//!
//! stub

/// What a SoundFont or plugin voice plays for a pattern note written for the Style voice
/// `voice` (bank MSB, LSB, program): the velocity to play it at, or None to leave it out.
pub fn playable(_voice: Option<(u8, u8, u8)>, _key: u8, vel: u8) -> Option<u8> {
    Some(vel)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Prepared;
    use crate::sff::{Ev, Section, SectionId, Style, TimedEv};
    use crate::sim::{run, Step};
    use crate::theory::Chord;

    const NYLON: (u8, u8, u8) = (8, 0, 0);

    #[test]
    fn noise_keys_and_dead_notes_are_left_out() {
        assert_eq!(playable(Some(NYLON), 100, 64), None, "strum noise");
        assert_eq!(playable(Some(NYLON), 121, 64), None, "fret noise");
        assert_eq!(playable(Some(NYLON), 60, 50), Some(50), "open");
        assert_eq!(playable(Some(NYLON), 60, 70), None, "dead");
        assert_eq!(playable(Some(NYLON), 60, 85), Some(60), "mute: played open, at the open zone's top");
        assert_eq!(playable(Some(NYLON), 60, 110), Some(60), "slide");
        assert_eq!(playable(Some(NYLON), 95, 127), Some(60), "harmonics, just under the noise keys");
    }

    #[test]
    fn other_voices_play_as_written() {
        for voice in [None, Some((0, 0, 24)), Some((0, 115, 0)), Some((8, 34, 0)), Some((104, 0, 0))] {
            for (key, vel) in [(100, 64), (60, 70), (60, 110)] {
                assert_eq!(playable(voice, key, vel), Some(vel), "{voice:?} {key} {vel}");
            }
        }
        // A MegaVoice without noise keys (SeattleStrings): a high note is a pitch, and its
        // velocity zones are all pitched, so it plays as written.
        assert_eq!(playable(Some((8, 1, 49)), 100, 110), Some(110));
    }

    #[test]
    fn basses_and_the_odd_layouts() {
        let electric = Some((8, 0, 17));
        assert_eq!(playable(electric, 40, 70), Some(70), "open hard");
        assert_eq!(playable(electric, 40, 100), None, "dead");
        assert_eq!(playable(electric, 40, 125), Some(80), "slap");
        assert_eq!(playable(electric, 100, 50), None, "SE");
        let pick = Some((8, 0, 18));
        assert_eq!(playable(pick, 40, 60), Some(40), "mute");
        assert_eq!(playable(pick, 40, 90), None, "dead");
        let flamenco = Some((8, 3, 0));
        assert_eq!(playable(flamenco, 60, 75), Some(75), "finger");
        assert_eq!(playable(flamenco, 60, 125), None, "key off noise");
        assert_eq!(playable(Some((8, 0, 2)), 60, 120), Some(120), "HiStringGuitar: soft and hard only");
        assert_eq!(playable(Some((8, 0, 4)), 60, 100), Some(55), "OverdriveGuitar: mute");
        assert_eq!(playable(Some((8, 0, 6)), 60, 85), None, "JazzGuitar: dead hard");
        assert_eq!(playable(Some((8, 0, 100)), 100, 64), None, "PopHaa: vocal articulations and breath");
    }

    /// Chord 1 (ch 12) on NylonGuitar and Chord 2 (ch 13) on a GM guitar play the same
    /// notes: a strum noise, an open note, a dead note and a slide. Chord 2 plays all four
    /// as written; Chord 1 leaves out the noise and the dead note and plays the slide at
    /// the open zone's top.
    #[test]
    fn a_megavoice_part_leaves_out_its_noises() {
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
            other_chunks: vec![],
        };
        let (_, rec) = run(Box::new(Prepared::new(&style)), &[(0, Step::Chord(Chord::new(0, 0)))], 1_900_000_000);
        let ons = |ch: u8| -> Vec<(u8, u8)> {
            rec.out.iter().filter(|(_, m)| m[0] == 0x90 | ch && m[2] > 0).map(|(_, m)| (m[1], m[2])).collect()
        };
        let gm = ons(12);
        assert_eq!(gm.len(), 4, "Chord 2 plays everything: {gm:?}");
        let (noise, open, _dead, slide) = (gm[0], gm[1], gm[2], gm[3]);
        assert_eq!((noise.1, open.1), (64, 50), "{gm:?}");
        assert_eq!(ons(11), vec![open, (slide.0, 60)], "Chord 1 on NylonGuitar");
        // Every note that went out also ended.
        let offs = rec.out.iter().filter(|(_, m)| m[0] & 0xF0 == 0x80 && m[0] & 0x0F == 11 || m[0] == 0x9B && m[2] == 0).count();
        assert_eq!(offs, 2);
    }
}
