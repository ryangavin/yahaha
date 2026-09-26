//! A style's own System Effects (#237): the XG Effect 1 SysEx in its SInt (`F0 43 1n 4C
//! 02 01 aa dd... F7`) picks the Reverb, Chorus and Variation types (and their
//! parameters) the style was written for. The Genos Data List's Parameter Chart makes
//! them Style Data. This reads them and maps each onto the nearest type yahaha's bus has:
//!
//! - Reverb: by the type's MSB, its family: 1 Hall, 2 Room, 3 Stage, 4 Plate (and the
//!   special spaces: 16 White Room a Room, 17-19 Tunnel/Canyon/Basement a Hall).
//! - Chorus: 65/66 Chorus (66/0 is Celeste 1), 67 Flanger, 87 Ensemble Detune as Celeste.
//!   The rest (tempo delays and cross delays in the chorus block, phaser, tremolo, auto
//!   pan, pitch change) have no match here.
//! - Variation, only when connected as a System effect (`5A` = 1; XG's default is
//!   Insertion, one part's own effect): the tempo delays (21/x Tempo Delay and Echo, 22/x
//!   Tempo Cross, which is our ping-pong) with their delay time (Data List Table#5),
//!   feedback and high damp; the free-time delays (5/x Delay LCR, 6/x Delay LR) as the
//!   dotted 1/8. Other variation types (reverbs, distortion, wah...) have no match: the
//!   delay then stays at its default, and the band doesn't reach it anyway (#236).
//!
//! No match leaves the block at its own default type. Only the control side uses this (at
//! style load); nothing here runs on the audio thread.

use super::Param;

/// XG Effect 1 (System Effects) parameter addresses within `02 01`.
const REVERB_TYPE: usize = 0x00;
const CHORUS_TYPE: usize = 0x20;
const VARIATION_TYPE: usize = 0x40;
/// Variation parameters 1-10: two bytes each (MSB, LSB), from 0x42.
const VARIATION_PARAM: usize = 0x42;
const VARIATION_CONNECTION: usize = 0x5A;

/// A block's type as the style chose it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    /// The XG type number.
    pub msb: u8,
    pub lsb: u8,
    /// The type's name ("Real Medium Hall"), or "XG msb/lsb" for one not in the table.
    pub name: String,
    /// The nearest type yahaha has, as its number in the block (`ReverbType as u8` etc.),
    /// or None: no match.
    pub kind: Option<u8>,
    /// Parameters the style sets on top of that type's own (`Param`, value).
    pub params: Vec<(Param, u16)>,
}

/// The style's choices: Reverb, Chorus, Variation (None: the style doesn't set it, or its
/// Variation is an Insertion effect).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StyleFx {
    pub blocks: [Option<Choice>; 3],
}

impl StyleFx {
    /// Read the style's SInt SysEx (`sff::SInt::sysex`, in file order: a later message
    /// overrides an earlier one).
    pub fn parse(sysex: &[Vec<u8>]) -> StyleFx {
        // The Effect 1 address block, as the messages leave it.
        let mut mem = [None::<u8>; 0x80];
        for m in sysex {
            // F0 43 1n 4C 02 01 aa data... F7
            if m.len() < 9 || m[0] != 0xF0 || m[1] != 0x43 || m[2] & 0xF0 != 0x10 || m[3] != 0x4C || m[4] != 0x02 || m[5] != 0x01 {
                continue;
            }
            let data = &m[7..m.len() - 1];
            for (i, &d) in data.iter().enumerate() {
                if let Some(slot) = mem.get_mut(m[6] as usize + i) {
                    *slot = Some(d & 0x7F);
                }
            }
        }
        let pair = |a: usize| Some((mem[a]?, mem[a + 1]?));
        let reverb = pair(REVERB_TYPE).filter(|t| t.0 != 0).map(|(msb, lsb)| Choice {
            msb,
            lsb,
            name: name(REVERB_NAMES, msb, lsb),
            kind: reverb_kind(msb),
            params: Vec::new(),
        });
        let chorus = pair(CHORUS_TYPE).filter(|t| t.0 != 0).map(|(msb, lsb)| Choice {
            msb,
            lsb,
            name: name(CHORUS_NAMES, msb, lsb),
            kind: chorus_kind(msb, lsb),
            params: Vec::new(),
        });
        let system = mem[VARIATION_CONNECTION] == Some(1);
        let variation = pair(VARIATION_TYPE).filter(|t| t.0 != 0 && system).map(|(msb, lsb)| {
            let param = |n: usize| pair(VARIATION_PARAM + 2 * (n - 1)).map(|(a, b)| (a as u16) << 7 | b as u16);
            let (kind, params) = delay(msb, param);
            Choice { msb, lsb, name: name(VARIATION_NAMES, msb, lsb), kind, params }
        });
        StyleFx { blocks: [reverb, chorus, variation] }
    }
}

fn reverb_kind(msb: u8) -> Option<u8> {
    use super::ReverbType::*;
    Some(match msb {
        1 | 17..=19 => Hall,
        2 | 16 => Room,
        3 => Stage,
        4 => Plate,
        _ => return None,
    } as u8)
}

fn chorus_kind(msb: u8, lsb: u8) -> Option<u8> {
    use super::ChorusType::*;
    Some(match (msb, lsb) {
        (66, 0) | (87, _) => Celeste,
        (65 | 66, _) => Chorus,
        (67, _) => Flanger,
        _ => return None,
    } as u8)
}

/// Data List Table#5 (tempo delay times): value -> beats, for the values up to 4thx6.
const TABLE5_BEATS: [f32; 20] = [
    1.0 / 12.0, // 32nd/3
    0.1875,     // 64th.
    0.125,      // 32nd
    1.0 / 6.0,  // 16th/3
    0.1875,     // 32nd.
    0.25,       // 16th
    1.0 / 3.0,  // 8th/3
    0.375,      // 16th.
    0.5,        // 8th
    2.0 / 3.0,  // 4th/3
    0.75,       // 8th.
    1.0,        // 4th
    4.0 / 3.0,  // 2nd/3
    1.5,        // 4th.
    2.0,        // 2nd
    8.0 / 3.0,  // Whole/3
    3.0,        // 2nd.
    4.0,        // 4thx4
    5.0,        // 4thx5
    6.0,        // 4thx6
];

/// The `NOTES` index nearest `beats` (by ratio).
fn nearest_note(beats: f32) -> u16 {
    let mut best = (0, f32::MAX);
    for (i, (b, _)) in super::NOTES.iter().enumerate() {
        let d = (beats / b).ln().abs();
        if d < best.1 {
            best = (i, d);
        }
    }
    best.0 as u16
}

/// A delay-type variation: our delay type (`DelayType as u8`) and its parameters.
fn delay(msb: u8, param: impl Fn(usize) -> Option<u16>) -> (Option<u8>, Vec<(Param, u16)>) {
    use super::DelayType;
    let mut params = Vec::new();
    let note = |v: Option<u16>| v.and_then(|v| TABLE5_BEATS.get(v as usize)).map(|&b| (Param::DelayNote, nearest_note(b)));
    // XG feedback: 1-127 = -63..+63; our feedback is its size, in %.
    let feedback = |v: Option<u16>| v.map(|v| (Param::DelayFeedback, ((v as i32 - 64).unsigned_abs() * 100 / 64).min(90) as u16));
    // XG high damp: 1-10 = 0.1-1.0 of the full band: our tone, 100 Hz steps (1.0 = 20 kHz).
    let damp = |v: Option<u16>| v.filter(|v| (1..=10).contains(v)).map(|v| (Param::DelayTone, v * 20));
    let kind = match msb {
        // Tempo Delay / Tempo Echo: 1 time, 2 feedback, 3 high damp.
        21 => {
            params.extend([note(param(1)), feedback(param(2)), damp(param(3))].into_iter().flatten());
            DelayType::DottedEighth
        }
        // Tempo Cross: 1 time L>R, 2 time R>L, 3 feedback, 5 high damp.
        22 => {
            params.extend([note(param(1)), feedback(param(3)), damp(param(5))].into_iter().flatten());
            DelayType::PingPong
        }
        5 | 6 => DelayType::DottedEighth,
        _ => return (None, params),
    };
    // Whatever the type's own ping-pong switch, the style's family decides it.
    (Some(kind as u8), params)
}

/// Names of the XG types seen in styles (Genos Data List Effect Type List); a type not
/// here reads "XG msb/lsb".
const REVERB_NAMES: &[(u8, u8, &str)] = &[
    (1, 0, "Hall 1"),
    (1, 1, "Hall 5"),
    (1, 16, "Hall 2"),
    (1, 17, "Hall 3"),
    (1, 18, "Hall 4"),
    (1, 19, "Ballad Hall"),
    (1, 20, "Piano Hall"),
    (1, 21, "Basic Hall"),
    (1, 22, "Light Hall"),
    (1, 32, "Real Large Hall"),
    (1, 33, "Real Medium Hall"),
    (1, 34, "Real Bright Hall"),
    (1, 35, "Real Large Hall +"),
    (1, 36, "Real Medium Hall +"),
    (1, 37, "Real Small Hall +"),
    (2, 19, "Room 4"),
    (2, 20, "Acoustic Room"),
    (2, 21, "Drums Room"),
    (2, 22, "Percussion Room"),
    (2, 32, "Real Room"),
    (2, 34, "Real Room +"),
    (3, 16, "Stage 1"),
    (4, 0, "Plate 3"),
    (4, 16, "Plate 1"),
    (4, 32, "Real Large Plate"),
    (4, 33, "Real Medium Plate"),
];
const CHORUS_NAMES: &[(u8, u8, &str)] = &[
    (21, 0, "Tempo Delay 1"),
    (21, 16, "Tempo Delay 2"),
    (22, 0, "Tempo Cross 1"),
    (22, 16, "Tempo Cross 2"),
    (65, 2, "Chorus 5"),
    (65, 5, "GM Chorus 3"),
    (65, 8, "Chorus 8"),
    (65, 17, "Chorus Lite"),
    (66, 0, "Celeste 1"),
    (66, 8, "Chorus 2"),
    (66, 16, "Chorus 3"),
    (66, 17, "Chorus 1"),
    (67, 8, "Flanger 1"),
    (67, 17, "Flanger 3"),
    (87, 0, "Ensemble Detune 1"),
    (108, 0, "Tempo Phaser 1"),
];
const VARIATION_NAMES: &[(u8, u8, &str)] = &[
    (5, 16, "Delay LCR 1"),
    (6, 0, "Delay LR"),
    (21, 0, "Tempo Delay 1"),
    (21, 8, "Tempo Echo"),
    (21, 16, "Tempo Delay 2"),
    (22, 0, "Tempo Cross 1"),
    (22, 16, "Tempo Cross 2"),
    (22, 17, "Tempo Cross 3"),
    (22, 18, "Tempo Cross 4"),
    (1, 0, "Hall 1"),
    (1, 17, "Hall 3"),
    (1, 18, "Hall 4"),
    (2, 20, "Acoustic Room"),
    (4, 16, "Plate 1"),
];

fn name(table: &[(u8, u8, &str)], msb: u8, lsb: u8) -> String {
    table.iter().find(|t| t.0 == msb && t.1 == lsb).map_or_else(|| format!("XG {msb}/{lsb}"), |t| t.2.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fx::{ChorusType, DelayType, ReverbType};

    fn msg(addr: u8, data: &[u8]) -> Vec<u8> {
        let mut m = vec![0xF0, 0x43, 0x10, 0x4C, 0x02, 0x01, addr];
        m.extend_from_slice(data);
        m.push(0xF7);
        m
    }

    #[test]
    fn the_types_map_to_the_nearest_block_type() {
        let fx = StyleFx::parse(&[msg(0x00, &[1, 33]), msg(0x20, &[66, 8]), msg(0x40, &[22, 0]), msg(0x5A, &[1])]);
        let [r, c, v] = &fx.blocks;
        let r = r.as_ref().unwrap();
        assert_eq!((r.name.as_str(), r.kind), ("Real Medium Hall", Some(ReverbType::Hall as u8)));
        assert_eq!(c.as_ref().unwrap().kind, Some(ChorusType::Chorus as u8));
        assert_eq!(v.as_ref().unwrap().kind, Some(DelayType::PingPong as u8));
        let kind = |m: u8, l: u8| StyleFx::parse(&[msg(0x00, &[m, l]), msg(0x20, &[m, l])]).blocks;
        assert_eq!(kind(4, 32)[0].as_ref().unwrap().kind, Some(ReverbType::Plate as u8));
        assert_eq!(kind(2, 22)[0].as_ref().unwrap().kind, Some(ReverbType::Room as u8));
        assert_eq!(kind(66, 0)[1].as_ref().unwrap().kind, Some(ChorusType::Celeste as u8));
        assert_eq!(kind(67, 8)[1].as_ref().unwrap().kind, Some(ChorusType::Flanger as u8));
        let phaser = &kind(108, 0)[1].as_ref().unwrap().clone();
        assert_eq!((phaser.name.as_str(), phaser.kind), ("Tempo Phaser 1", None), "no match");
        assert_eq!(kind(77, 0)[1].as_ref().unwrap().name, "XG 77/0");
    }

    /// The variation counts only as a System effect (connection 1); XG's default is
    /// Insertion.
    #[test]
    fn an_insertion_variation_is_not_the_styles_system_effect() {
        assert_eq!(StyleFx::parse(&[msg(0x40, &[21, 0])]).blocks[2], None);
        assert_eq!(StyleFx::parse(&[msg(0x40, &[21, 0]), msg(0x5A, &[0])]).blocks[2], None);
        assert!(StyleFx::parse(&[msg(0x40, &[21, 0]), msg(0x5A, &[1])]).blocks[2].is_some());
        // A distortion as the system variation: named, no match.
        let v = StyleFx::parse(&[msg(0x40, &[96, 0]), msg(0x5A, &[1])]).blocks[2].clone().unwrap();
        assert_eq!(v.kind, None);
    }

    /// A tempo delay's time (Table#5), feedback and high damp become our note value,
    /// feedback and tone; a multi-byte message fills consecutive addresses.
    #[test]
    fn tempo_delay_parameters() {
        // Tempo Delay 1, time 8th. (10), feedback +32 (96), high damp 0.5 (5).
        let v = StyleFx::parse(&[msg(0x40, &[21, 0, 0, 10, 0, 96, 0, 5]), msg(0x5A, &[1])]).blocks[2].clone().unwrap();
        assert_eq!(v.kind, Some(DelayType::DottedEighth as u8));
        assert_eq!(v.params, vec![(Param::DelayNote, 4), (Param::DelayFeedback, 50), (Param::DelayTone, 100)]);
        // Tempo Cross 1 with an 8th: our ping-pong at 1/8.
        let v = StyleFx::parse(&[msg(0x40, &[22, 0]), msg(0x42, &[0, 8]), msg(0x5A, &[1])]).blocks[2].clone().unwrap();
        assert_eq!((v.kind, v.params.clone()), (Some(DelayType::PingPong as u8), vec![(Param::DelayNote, 2)]));
        // Beyond our notes: the nearest (4thx6 is six beats: our 1/2).
        let v = StyleFx::parse(&[msg(0x40, &[21, 0, 0, 19]), msg(0x5A, &[1])]).blocks[2].clone().unwrap();
        assert_eq!(v.params, vec![(Param::DelayNote, 7)]);
    }

    #[test]
    fn nothing_set_is_no_choice() {
        assert_eq!(StyleFx::parse(&[]), StyleFx::default());
        assert_eq!(StyleFx::parse(&[msg(0x00, &[0, 0])]).blocks[0], None, "No Effect");
        // Another device's or model's SysEx is not read.
        assert_eq!(StyleFx::parse(&[vec![0xF0, 0x43, 0x10, 0x4C, 0x08, 0x01, 0x00, 1, 33, 0xF7]]), StyleFx::default());
    }

    /// Every corpus style: its effects read, and the reverb always matches (they are all
    /// halls, rooms and plates). Prints how often each type occurs.
    #[test]
    fn corpus_styles_effects() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
        if !dir.exists() {
            eprintln!("corpus missing; skipping");
            return;
        }
        let mut files = Vec::new();
        let mut stack = vec![dir];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| matches!(x.to_ascii_lowercase().to_str(), Some("sty" | "prs" | "sst"))) {
                    files.push(p);
                }
            }
        }
        let mut counts = std::collections::BTreeMap::<(usize, String, Option<u8>), usize>::new();
        let mut n = 0;
        for f in &files {
            let Ok(s) = crate::sff::Style::load(f) else { continue };
            n += 1;
            let fx = StyleFx::parse(&s.sint().sysex);
            if let Some(r) = &fx.blocks[0] {
                assert!(r.kind.is_some(), "{f:?}: reverb {}", r.name);
            }
            for (b, c) in fx.blocks.iter().enumerate() {
                let key = c.as_ref().map_or((b, "(none)".to_string(), None), |c| (b, c.name.clone(), c.kind));
                *counts.entry(key).or_default() += 1;
            }
        }
        assert!(n > 100, "{n} styles");
        for ((b, name, kind), c) in counts {
            eprintln!("{} {name:<22} -> {kind:?}: {c}", ["reverb", "chorus", "variation"][b]);
        }
    }
}
