//! A style's XG Drum Setup on the built-in synth (#239).
//!
//! A style's SInt tunes its drum kits note by note with XG Drum Setup SysEx
//! (`F0 43 1n 4C 3s rr pp vv F7`: setup s, note rr, parameter pp). The port passes the SysEx
//! on, so an XG instrument applies it itself. The built-in synth's ring takes 3-byte
//! messages only, so the engine thread's `live::Out` turns each drum setup SysEx into a
//! 3-byte *drum message* ([`encode`]). The audio thread keeps the setups
//! ([`DrumSetups::observe`]) and starts each drum note with that note's settings
//! ([`DrumSetups::note`] → `rustysynth::NoteParams`). They are fixed at the note-on, so a
//! change never moves a note already sounding.
//!
//! Applied:
//! - **Level** (02) as gain, on the CC7 curve (gain = (level / [`DEFAULT_LEVEL`])²).
//! - **Pitch coarse / fine** (00/01) in semitones and cents.
//! - **Pan** (04), in place of the kit's own pan (0 = random per note).
//! - **Reverb / Chorus / Variation send** (05/06/07), scaling the part's send into the
//!   effect bus (value / 127).
//! - **Filter cutoff / resonance** (0B/0C), and **EG attack / decay 1 / decay 2**
//!   (0D/0E/0F: attack, decay and release times).
//!
//! Not applied: Alternate Group, Key Assign and Rcv Note Off/On (no corpus style sets them).
//!
//! The state follows the Data List's rules (DL "MIDI Parameter Change table (DRUM
//! SETUP)"):
//! - A part plays Drum Setup 1 or 2 when its XG Part Mode is DRUMS1 or DRUMS2
//!   (`F0 43 1n 4C 08 pp 07 02|03 F7`). Part 10 is DRUMS1 until told otherwise.
//! - A program change on a part that plays a drum setup initializes that setup.
//! - A Drum Setup Reset, or an XG/GM/GS system reset, initializes the setups.

use rustysynth::NoteParams;

use super::Msg;

/// Drum Setup parameter addresses (DL).
const COARSE: u8 = 0x00;
const FINE: u8 = 0x01;
const LEVEL: u8 = 0x02;
const PAN: u8 = 0x04;
const REVERB: u8 = 0x05;
const CHORUS: u8 = 0x06;
const VARIATION: u8 = 0x07;
const CUTOFF: u8 = 0x0B;
const RESONANCE: u8 = 0x0C;
const ATTACK: u8 = 0x0D;
const DECAY1: u8 = 0x0E;
const DECAY2: u8 = 0x0F;
/// The parameters a drum message carries (00-0F).
const PARAMS: usize = 16;
/// XG Multi Part parameter address of the Part Mode.
const PART_MODE: u8 = 0x07;
/// Part Mode values that select Drum Setup 1 and 2.
const DRUMS1: u8 = 0x02;
const DRUMS2: u8 = 0x03;
/// No drum setup on the part / no value set.
const NONE: u8 = 0xFF;
/// The drum setups a Genos has (DL: "n: Drum Setup Number (0 – 1)").
const SETUPS: usize = 2;

/// Drum messages on the synth ring. Their first byte is below 80H, which no MIDI message
/// starts with: `[PARAM | setup << 4 | parameter, note, value]`, `[PART, part, mode]`,
/// `[SETUP_RESET, setup, 0]` and `[SYSTEM_RESET, 0, 0]`.
const PARAM: u8 = 0x40;
const PART: u8 = 0x60;
const SETUP_RESET: u8 = 0x61;
const SYSTEM_RESET: u8 = 0x62;

/// The Level a note plays at when the setup does not set one. The Data List gives the XG
/// default as "depends on the note", and nothing lists it. Decision: 100. It is the value
/// style authors set most (117 of 1021 corpus Level messages), and their levels spread
/// both ways around it (72–127, mean ≈ 100), as edits of a default in the middle would.
/// 127 is set explicitly 46 times, so it isn't the default for those notes.
pub const DEFAULT_LEVEL: u8 = 100;
/// XG's centre for the signed parameters (pitch, filter, EG): 40H = 0.
const CENTRE: i32 = 0x40;
/// EG rate and filter cutoff steps per octave. Decision: 16, so the ±63 range spans about
/// ±4 octaves (a 16x faster or slower envelope, a cutoff 16x higher or lower). The DL gives
/// only the range; the corpus decays sit at +5..+16 (a slightly shorter drum).
const STEPS_PER_OCTAVE: f32 = 16.0;
/// Decibels of resonance per step. Decision: 0.2, so the range spans about ±12 dB.
const RESONANCE_DB_PER_STEP: f32 = 0.2;

/// The drum message that stands for `m` on the synth ring, if `m` is SysEx that changes
/// a drum setup: a Drum Setup parameter, a Part Mode, a Drum Setup Reset or a system reset.
#[inline]
pub fn encode(m: &[u8]) -> Option<Msg> {
    match *m {
        [0xF0, 0x43, d, 0x4C, s @ (0x30 | 0x31), note, p, v, 0xF7] if d & 0xF0 == 0x10 && note < 0x80 && (p as usize) < PARAMS => {
            Some([PARAM | (s - 0x30) << 4 | p, note, v])
        }
        [0xF0, 0x43, d, 0x4C, 0x08, part, PART_MODE, mode, 0xF7] if d & 0xF0 == 0x10 && part < 16 => Some([PART, part, mode]),
        [0xF0, 0x43, d, 0x4C, 0x00, 0x00, 0x7D, s, ..] if d & 0xF0 == 0x10 && (s as usize) < SETUPS => Some([SETUP_RESET, s, 0]),
        [0xF0, ..] if crate::sff::is_reset(m) => Some([SYSTEM_RESET, 0, 0]),
        _ => None,
    }
}

/// A drum message (`encode`), not a MIDI message.
#[inline]
pub fn is_drum_msg(m: &Msg) -> bool {
    (PARAM..=SYSTEM_RESET).contains(&m[0])
}

/// The drum setups as the built-in synth plays them. No allocation: fixed tables, updated
/// message by message on the audio thread.
#[derive(Clone)]
pub struct DrumSetups {
    /// Each setup's parameters per note (`NONE`: not set).
    params: [[[u8; PARAMS]; 128]; SETUPS],
    /// Notes with any parameter set, per setup (bit per note).
    set: [u128; SETUPS],
    /// The setup each channel's part plays (`NONE`: not a drum-setup part).
    setup_of: [u8; 16],
    /// Random pan (Pan 0): a xorshift state.
    rng: u32,
}

impl Default for DrumSetups {
    fn default() -> Self {
        Self::new()
    }
}

impl DrumSetups {
    pub const fn new() -> DrumSetups {
        let mut setup_of = [NONE; 16];
        // DL Multi Part PART MODE default: part10 = 02 (DRUMS1), other parts 00.
        setup_of[9] = 0;
        DrumSetups { params: [[[NONE; PARAMS]; 128]; SETUPS], set: [0; SETUPS], setup_of, rng: 0x9E37_79B9 }
    }

    fn clear(&mut self, setup: usize) {
        if let Some(t) = self.params.get_mut(setup) {
            *t = [[NONE; PARAMS]; 128];
            self.set[setup] = 0;
        }
    }

    /// Follow a message on the synth ring: a drum message, or a program change that
    /// initializes its part's setup. Any other message is ignored.
    #[inline]
    pub fn observe(&mut self, m: &Msg) {
        match m[0] {
            PARAM..=0x5F => {
                let setup = ((m[0] >> 4) & 1) as usize;
                let note = (m[1] & 0x7F) as usize;
                self.params[setup][note][(m[0] & 0x0F) as usize] = m[2] & 0x7F;
                self.set[setup] |= 1 << note;
            }
            PART => {
                self.setup_of[(m[1] & 0x0F) as usize] = match m[2] {
                    DRUMS1 => 0,
                    DRUMS2 => 1,
                    _ => NONE,
                }
            }
            SETUP_RESET => self.clear(m[1] as usize),
            SYSTEM_RESET => *self = DrumSetups { rng: self.rng, ..DrumSetups::new() },
            st if st & 0xF0 == 0xC0 => self.clear(self.setup_of[(st & 0x0F) as usize] as usize),
            _ => {}
        }
    }

    /// The settings a note-on (`m`) on its channel starts with, if its part plays a drum
    /// setup that sets any for the note.
    #[inline]
    pub fn note(&mut self, m: &Msg) -> Option<NoteParams> {
        if m[0] & 0xF0 != 0x90 || m[2] == 0 {
            return None;
        }
        let setup = self.setup_of[(m[0] & 0x0F) as usize] as usize;
        let key = (m[1] & 0x7F) as usize;
        if setup >= SETUPS || self.set[setup] >> key & 1 == 0 {
            return None;
        }
        let p = self.params[setup][key];
        let get = |a: u8| (p[a as usize] != NONE).then_some(p[a as usize] as i32);
        let signed = |a: u8| get(a).map_or(0, |v| v - CENTRE);
        let octaves = |a: u8| 2f32.powf(signed(a) as f32 / STEPS_PER_OCTAVE);
        let mut n = NoteParams::NEUTRAL;
        if let Some(l) = get(LEVEL) {
            let g = l as f32 / DEFAULT_LEVEL as f32;
            n.gain = g * g;
        }
        n.tune = signed(COARSE) as f32 + signed(FINE) as f32 / 100.0;
        n.pan = get(PAN).map(|v| {
            let v = if v == 0 { self.random_pan() } else { v };
            (v - CENTRE) as f32 * 50.0 / 64.0
        });
        for (i, a) in [REVERB, CHORUS, VARIATION].into_iter().enumerate().take(n.sends.len()) {
            if let Some(v) = get(a) {
                n.sends[i] = v as f32 / 127.0;
            }
        }
        n.cutoff = octaves(CUTOFF);
        n.resonance_db = signed(RESONANCE) as f32 * RESONANCE_DB_PER_STEP;
        // A higher rate is a shorter time.
        n.attack = 1.0 / octaves(ATTACK);
        n.decay = 1.0 / octaves(DECAY1);
        n.release = 1.0 / octaves(DECAY2);
        Some(n)
    }

    /// A pan value 1-127 for XG's random pan (Pan 0).
    fn random_pan(&mut self) -> i32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        1 + (x % 127) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn param(setup: u8, note: u8, p: u8, v: u8) -> Msg {
        encode(&[0xF0, 0x43, 0x10, 0x4C, 0x30 + setup, note, p, v, 0xF7]).unwrap()
    }

    fn part_mode(part: u8, mode: u8) -> Msg {
        encode(&[0xF0, 0x43, 0x10, 0x4C, 0x08, part, PART_MODE, mode, 0xF7]).unwrap()
    }

    fn with(msgs: &[Msg]) -> DrumSetups {
        let mut d = DrumSetups::new();
        for m in msgs {
            assert!(is_drum_msg(m) || m[0] >= 0x80);
            d.observe(m);
        }
        d
    }

    #[test]
    fn a_note_starts_with_its_setups_settings() {
        // The corpus layout: Rhythm 1 (ch 9) on DRUMS2, Rhythm 2 (ch 10) on DRUMS1.
        let mut d = with(&[
            part_mode(8, DRUMS2),
            part_mode(9, DRUMS1),
            param(0, 42, LEVEL, 80),
            param(0, 42, COARSE, 0x42),
            param(0, 42, FINE, 0x40 - 25),
            param(0, 42, PAN, 0x40 + 32),
            param(0, 42, REVERB, 0),
            param(0, 42, CUTOFF, 0x40 - 16),
            param(0, 42, RESONANCE, 0x40 + 10),
            param(0, 42, DECAY1, 0x40 + 16),
            param(0, 42, DECAY2, 0x40 - 16),
            param(0, 42, ATTACK, 0x40),
            param(1, 36, LEVEL, 120),
        ]);
        let n = d.note(&[0x99, 42, 100]).expect("tuned note");
        assert!((n.gain - 0.64).abs() < 1e-6);
        assert!((n.tune - 1.75).abs() < 1e-6);
        assert_eq!(n.pan, Some(25.0));
        assert_eq!(n.sends, [0.0, 1.0, 1.0]);
        assert!((n.cutoff - 0.5).abs() < 1e-6);
        assert!((n.resonance_db - 2.0).abs() < 1e-6);
        assert!((n.decay - 0.5).abs() < 1e-6 && (n.release - 2.0).abs() < 1e-6 && n.attack == 1.0);
        let k = d.note(&[0x98, 36, 100]).expect("setup 2's note");
        assert!((k.gain - 1.44).abs() < 1e-6);
        assert_eq!((k.tune, k.pan, k.sends, k.cutoff), (0.0, None, [1.0; 3], 1.0));
        // Other notes, the same note on the other setup's part, melodic parts, note-offs:
        // nothing.
        assert_eq!(d.note(&[0x99, 36, 100]), None);
        assert_eq!(d.note(&[0x98, 42, 100]), None);
        assert_eq!(d.note(&[0x9B, 42, 100]), None);
        assert_eq!(d.note(&[0x99, 42, 0]), None);
        assert_eq!(d.note(&[0x89, 42, 64]), None);
    }

    #[test]
    fn part_ten_plays_drum_setup_1_by_default() {
        let mut d = with(&[param(0, 38, LEVEL, 50)]);
        assert!(d.note(&[0x99, 38, 100]).is_some());
        assert_eq!(d.note(&[0x98, 38, 100]), None);
        d.observe(&part_mode(9, 0));
        assert_eq!(d.note(&[0x99, 38, 100]), None);
    }

    #[test]
    fn a_program_change_or_a_reset_initializes_the_setup() {
        let mut d = with(&[part_mode(8, DRUMS2), param(0, 38, LEVEL, 50), param(1, 38, LEVEL, 50)]);
        // A program change on ch 9 initializes setup 2 only.
        d.observe(&[0xC8, 25, 0]);
        assert_eq!(d.note(&[0x98, 38, 100]), None);
        assert!(d.note(&[0x99, 38, 100]).is_some());
        // A program change on a part without a setup changes nothing.
        d.observe(&[0xC3, 0, 0]);
        assert!(d.note(&[0x99, 38, 100]).is_some());
        // Drum Setup Reset (setup 1).
        d.observe(&encode(&[0xF0, 0x43, 0x10, 0x4C, 0x00, 0x00, 0x7D, 0x00, 0xF7]).unwrap());
        assert_eq!(d.note(&[0x99, 38, 100]), None);
        // XG System On resets the setups and the part modes.
        d.observe(&param(1, 38, LEVEL, 50));
        d.observe(&encode(&[0xF0, 0x43, 0x10, 0x4C, 0x00, 0x00, 0x7E, 0x00, 0xF7]).unwrap());
        d.observe(&param(1, 38, LEVEL, 50));
        assert_eq!(d.note(&[0x98, 38, 100]), None);
    }

    #[test]
    fn random_pan_stays_in_range() {
        let mut d = with(&[param(0, 38, PAN, 0)]);
        let pans: Vec<f32> = (0..200).map(|_| d.note(&[0x99, 38, 100]).unwrap().pan.unwrap()).collect();
        assert!(pans.iter().all(|p| (-50.0..=50.0).contains(p)));
        assert!(pans.iter().any(|&p| p < -10.0) && pans.iter().any(|&p| p > 10.0));
    }

    #[test]
    fn only_drum_setup_sysex_is_encoded() {
        for m in [
            &[0xF0, 0x43, 0x10, 0x4C, 0x30, 0x80, LEVEL, 10, 0xF7][..],
            &[0xF0, 0x43, 0x10, 0x4C, 0x30, 36, 0x70, 10, 0xF7],
            &[0xF0, 0x43, 0x10, 0x4C, 0x00, 0x00, 0x7D, 9, 0xF7],
            &[0xF0, 0x43, 0x10, 0x4C, 0x02, 0x01, 0x00, 0x01, 0x10, 0xF7],
            &[0xF0, 0x43, 0x10, 0x4C, 0x08, 0x09, 0x0B, 100, 0xF7],
            &[0xF0],
            &[],
            &[0x99, 36, 100],
        ] {
            assert_eq!(encode(m), None, "{m:02X?}");
        }
    }
}
