//! MegaVoice parts on voices that are not MegaVoices (#223).
//!
//! A Genos MegaVoice (bank MSB 8, Data List "MegaVoice Map") is not played by pitch alone:
//! - **Velocity zones** pick the articulation below C6 (MIDI 96). A guitar sounds open
//!   notes at 1-60, dead notes at 61-75, then mutes, hammer-ons, slides and harmonics.
//! - **Noise keys**: from C6 (Yamaha numbering, MIDI 96) up, keys are strum, fret, body,
//!   valve or breath noises, or vocal ad libs, not pitches.
//!
//! Style parts are written for those voices. A SoundFont or plugin voice plays every key
//! as a pitch and every velocity as a level, so a guitar's strum noises come out as high
//! chromatic notes and its dead notes as loud open ones. yahaha never sounds a real
//! MegaVoice, so on a part whose Style voice is a MegaVoice:
//! - noise keys are left out, and so is a velocity zone that is a noise (FlamencoGuitar's
//!   key-off noise);
//! - notes in the pitched articulation zones (mute, hammer-on, slide, harmonics, slap) play
//!   as plain notes, their velocity capped at the top of the voice's plain zone, so they
//!   sound like a hard plain note rather than a loud accent;
//! - dead notes play as ghost notes, at half the plain zone's top.
//!
//! Decision: dead notes are kept as ghost notes, not left out. They carry the groove (a
//! seventh of the MegaVoice bass notes and one in 25 guitar notes in the corpus
//! are dead notes), and on a Guitar part the NTT has already made them chord tones, so a
//! quiet pitch is the closest a plain voice gets to a muted thump.
//!
//! Decision: the bowed, blown and sung MegaVoices (strings, brass, trumpet, sax, choirs)
//! play their velocities as written. All their zones are pitched (legato, spiccato,
//! scoops, falls) and roughly rise in intensity. Their noise keys are still left out.
//!
//! The note's written key and velocity pick its articulation, as on the Genos. The pattern
//! writes a noise key where the author heard one, so it is the source key that counts:
//! a note transposed up past C6 on another chord is a pitch.

/// The MegaVoice bank.
pub const MSB: u8 = 8;
/// The first noise key: C6 in Yamaha numbering (the Data List's "above C6").
pub const NOISE_KEY: u8 = 96;

/// What a velocity zone plays on a voice that is not a MegaVoice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Zone {
    /// The voice's plain notes: as written.
    Plain,
    /// A pitched articulation (mute, hammer-on, slide, harmonics): a plain note, at most
    /// as loud as the top of the plain zone.
    Capped,
    /// A dead note (a muted string, struck): a ghost note, at half the plain zone's top.
    Dead,
    /// A noise (a key-off noise): left out.
    Noise,
}
use Zone::*;

/// A voice's velocity zones below C6, as (last velocity, zone), rising to 127.
type Zones = &'static [(u8, Zone)];

/// 1-60 open, 61-75 dead, 76-90 mute, 91-105 hammer, 106-120 slide, 121-127 harmonics.
/// Also the Twin guitars (both elements switch at the same velocities) and ActiveBassSlap
/// (thumb and pop, dead, then thumb/pop combinations and harmonics).
const GUITAR: Zones = &[(60, Plain), (75, Dead), (127, Capped)];
/// JazzGuitar: dead soft (61-75) and dead hard (76-90).
const JAZZ: Zones = &[(60, Plain), (90, Dead), (127, Capped)];
/// FlamencoGuitar: finger 1-80, mute, hammer, slides, then a key-off noise at 121-127.
const FLAMENCO: Zones = &[(80, Plain), (120, Capped), (127, Noise)];
/// OverdriveGuitar, DistortionGuitar: open 1-55, mute, pick harmonics.
const DRIVE: Zones = &[(55, Plain), (127, Capped)];
/// Open soft, open hard, dead, harmonics (or slap) at 121-127.
const BASS: Zones = &[(80, Plain), (120, Dead), (127, Capped)];
/// PickBass, VintagePick, ActiveBassPick: open 1-40, mute 41-80, dead, harmonics.
const BASS_PICK: Zones = &[(40, Plain), (80, Capped), (120, Dead), (127, Capped)];
/// ActiveBassFingHmrOn, ActiveBassPickHmrOn: open, mute, dead 81-90, hammer-ons, harmonics.
const BASS_HAMMER: Zones = &[(40, Plain), (80, Capped), (90, Dead), (127, Capped)];
/// Every zone plays as written: HiStringGuitar and 12StringGuitar (soft, hard), and the
/// bowed, blown and sung voices (see the module's Decision).
const AS_WRITTEN: Zones = &[(127, Plain)];

/// A MegaVoice: (LSB, program 0-127, zones, has noise keys from C6 up).
struct MegaVoice(u8, u8, Zones, bool);

/// The Genos Data List's MegaVoice Map. Programs here are 0-based (the Data List's PC# - 1).
#[rustfmt::skip]
const VOICES: [MegaVoice; 82] = [
    // Guitars (A.Guitar, E.Guitar): strum, pick or fret noises from C6 up.
    MegaVoice(0, 0, GUITAR, true),       // NylonGuitar
    MegaVoice(3, 0, FLAMENCO, true),     // FlamencoGuitar
    MegaVoice(4, 0, GUITAR, true),       // SpanishMedium
    MegaVoice(5, 0, GUITAR, true),       // SpanishHard
    MegaVoice(0, 1, GUITAR, true),       // SteelGuitar
    MegaVoice(1, 1, GUITAR, true),       // SteelAcousticPick
    MegaVoice(2, 1, GUITAR, true),       // SteelAcousticSlap
    MegaVoice(5, 1, GUITAR, true),       // SteelGuitarTwin1
    MegaVoice(6, 1, GUITAR, true),       // SteelGuitarTwin2
    MegaVoice(7, 1, GUITAR, true),       // 12StringPickTwin1
    MegaVoice(8, 1, GUITAR, true),       // 12StringPickTwin2
    MegaVoice(10, 1, GUITAR, true),      // D-FolkGuitar
    MegaVoice(11, 1, GUITAR, true),      // SteelAcousticFinger
    MegaVoice(12, 1, GUITAR, true),      // SteelAcThumbPick
    MegaVoice(13, 1, GUITAR, true),      // D&HardFolkTwin1
    MegaVoice(14, 1, GUITAR, true),      // D&WarmFolkTwin1
    MegaVoice(15, 1, GUITAR, true),      // D&HardFolkTwin2
    MegaVoice(16, 1, GUITAR, true),      // D&WarmFolkTwin2
    MegaVoice(0, 2, AS_WRITTEN, true),   // HiStringGuitar
    MegaVoice(1, 2, AS_WRITTEN, true),   // 12StringGuitar
    MegaVoice(0, 3, GUITAR, true),       // CleanGuitar
    MegaVoice(1, 3, GUITAR, true),       // SolidGuitar1
    MegaVoice(2, 3, GUITAR, true),       // SolidGuitar2
    MegaVoice(3, 3, GUITAR, true),       // SingleCoilGuitar
    MegaVoice(4, 3, GUITAR, true),       // 50sVintageFinger
    MegaVoice(5, 3, GUITAR, true),       // 50sVintageFingerSlap
    MegaVoice(6, 3, GUITAR, true),       // 50sVintagePick
    MegaVoice(7, 3, GUITAR, true),       // 50sVintageSlap
    MegaVoice(8, 3, GUITAR, true),       // SlapAmpGuitar
    MegaVoice(10, 3, GUITAR, true),      // 60sVintage
    MegaVoice(11, 3, GUITAR, true),      // 60sVintageSlap
    MegaVoice(0, 4, DRIVE, true),        // OverdriveGuitar
    MegaVoice(0, 5, DRIVE, true),        // DistortionGuitar
    MegaVoice(0, 6, JAZZ, true),         // JazzGuitar
    MegaVoice(4, 12, GUITAR, true),      // Mandolin
    MegaVoice(0, 13, GUITAR, true),      // UkleleThumbDown
    // Basses: sound effects from C6 up.
    MegaVoice(0, 16, BASS, true),        // AcousticBass
    MegaVoice(0, 17, BASS, true),        // ElectricBass
    MegaVoice(1, 17, BASS, true),        // VintageRound
    MegaVoice(2, 17, BASS, true),        // VintageFlat
    MegaVoice(3, 17, BASS_HAMMER, true), // ActiveBassFingHmrOn
    MegaVoice(4, 17, BASS, true),        // ActiveBassFingHarm
    MegaVoice(5, 17, BASS, true),        // ActiveBassFingSlap
    MegaVoice(0, 18, BASS_PICK, true),   // PickBass
    MegaVoice(1, 18, BASS_PICK, true),   // VintagePick
    MegaVoice(2, 18, BASS_HAMMER, true), // ActiveBassPickHmrOn
    MegaVoice(3, 18, BASS_PICK, true),   // ActiveBassPick
    MegaVoice(4, 18, BASS, true),        // ActiveBassPickOpen
    MegaVoice(5, 18, BASS, true),        // ActiveBassPickMute (mute soft / hard)
    MegaVoice(0, 19, BASS, true),        // FretlessBass (open 1-80)
    MegaVoice(0, 20, GUITAR, true),      // ActiveBassSlap
    // Strings: velocity zones only.
    MegaVoice(0, 48, AS_WRITTEN, false), // SmallStrings
    MegaVoice(1, 48, AS_WRITTEN, false), // ClassicalStrings
    MegaVoice(3, 48, AS_WRITTEN, false), // KinoSmall
    MegaVoice(4, 48, AS_WRITTEN, false), // KinoSmallComp
    MegaVoice(5, 48, AS_WRITTEN, false), // KinoSmallCompOctCb
    MegaVoice(6, 48, AS_WRITTEN, false), // KinoSmallAmbi
    MegaVoice(0, 49, AS_WRITTEN, false), // LargeStrings
    MegaVoice(1, 49, AS_WRITTEN, false), // SeattleStrings
    MegaVoice(2, 49, AS_WRITTEN, false), // KinoLarge
    MegaVoice(3, 49, AS_WRITTEN, false), // KinoLargeComp
    MegaVoice(4, 49, AS_WRITTEN, false), // KinoLargeOctCb
    MegaVoice(5, 49, AS_WRITTEN, false), // KinoLargeAmbiOctCb
    MegaVoice(6, 49, AS_WRITTEN, false), // KinoLargeAmbi
    // Choirs, brass and woodwind.
    MegaVoice(0, 51, AS_WRITTEN, false), // MaleVoiceChoir
    MegaVoice(0, 54, AS_WRITTEN, true),  // GospelChoir: ad libs, sound effects
    MegaVoice(0, 56, AS_WRITTEN, false), // Brass
    MegaVoice(1, 56, AS_WRITTEN, false), // PopHorns1
    MegaVoice(2, 56, AS_WRITTEN, false), // PopHorns2
    // Valve, key and breath noises, and the pop choirs' vocal articulations, from C6 up.
    MegaVoice(0, 64, AS_WRITTEN, true),  // Trumpet
    MegaVoice(0, 82, AS_WRITTEN, true),  // TenorSax
    MegaVoice(0, 100, AS_WRITTEN, true), // PopHaa
    MegaVoice(0, 101, AS_WRITTEN, true), // PopDaa
    MegaVoice(0, 102, AS_WRITTEN, true), // PopBaa
    MegaVoice(0, 103, AS_WRITTEN, true), // PopShoo
    MegaVoice(0, 105, AS_WRITTEN, true), // PopHoo
    MegaVoice(0, 106, AS_WRITTEN, true), // PopDoo
    MegaVoice(0, 107, AS_WRITTEN, true), // PopBee
    MegaVoice(0, 110, AS_WRITTEN, true), // PopHee
    MegaVoice(0, 115, AS_WRITTEN, true), // PopHaaLegato2
    MegaVoice(0, 120, AS_WRITTEN, true), // PopHooLegato2
    MegaVoice(0, 125, AS_WRITTEN, true), // PopHeeLegato2
];

fn lookup(msb: u8, lsb: u8, program: u8) -> Option<&'static MegaVoice> {
    if msb != MSB {
        return None;
    }
    VOICES.iter().find(|v| v.0 == lsb && v.1 == program)
}

/// What a SoundFont or plugin voice plays for a pattern note written for the Style voice
/// `voice` (bank MSB, LSB, program): the velocity to play it at, or None to leave it out.
/// A note for any other voice plays as written. Real-time safe.
pub fn playable(voice: Option<(u8, u8, u8)>, key: u8, vel: u8) -> Option<u8> {
    let Some(&MegaVoice(_, _, zones, noise)) = voice.and_then(|(m, l, p)| lookup(m, l, p)) else {
        return Some(vel);
    };
    if key >= NOISE_KEY {
        return if noise { None } else { Some(vel) };
    }
    let zone = zones.iter().find(|z| vel <= z.0).map_or(Plain, |z| z.1);
    match zone {
        Plain => Some(vel),
        Capped => Some(vel.min(zones[0].0)),
        Dead => Some(zones[0].0 / 2),
        Noise => None,
    }
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
    fn the_table_is_well_formed() {
        for (i, v) in VOICES.iter().enumerate() {
            assert!(!VOICES[..i].iter().any(|w| (w.0, w.1) == (v.0, v.1)), "{}/{} twice", v.0, v.1);
            assert!(v.2.windows(2).all(|z| z[0].0 < z[1].0) && v.2.last().unwrap().0 == 127, "{}/{}", v.0, v.1);
            assert_eq!(v.2[0].1, Zone::Plain, "{}/{}: a voice's first zone is its plain notes", v.0, v.1);
        }
    }

    #[test]
    fn noise_keys_are_left_out_and_articulations_play_plain() {
        assert_eq!(playable(Some(NYLON), 100, 64), None, "strum noise");
        assert_eq!(playable(Some(NYLON), 121, 64), None, "fret noise");
        assert_eq!(playable(Some(NYLON), 60, 50), Some(50), "open");
        assert_eq!(playable(Some(NYLON), 60, 70), Some(30), "dead: a ghost note");
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
        assert_eq!(playable(electric, 40, 100), Some(40), "dead");
        assert_eq!(playable(electric, 40, 125), Some(80), "slap");
        assert_eq!(playable(electric, 100, 50), None, "SE");
        let pick = Some((8, 0, 18));
        assert_eq!(playable(pick, 40, 60), Some(40), "mute");
        assert_eq!(playable(pick, 40, 90), Some(20), "dead");
        let flamenco = Some((8, 3, 0));
        assert_eq!(playable(flamenco, 60, 75), Some(75), "finger");
        assert_eq!(playable(flamenco, 60, 125), None, "key off noise");
        assert_eq!(playable(Some((8, 0, 2)), 60, 120), Some(120), "HiStringGuitar: soft and hard only");
        assert_eq!(playable(Some((8, 0, 4)), 60, 100), Some(55), "OverdriveGuitar: mute");
        assert_eq!(playable(Some((8, 0, 6)), 60, 85), Some(30), "JazzGuitar: dead hard");
        assert_eq!(playable(Some((8, 0, 100)), 100, 64), None, "PopHaa: vocal articulations and breath");
    }

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
}
