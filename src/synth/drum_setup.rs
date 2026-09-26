//! A style's XG Drum Setup on the built-in synth (#239).
//!
//! A style's SInt tunes its drum kits note by note with XG Drum Setup SysEx
//! (`F0 43 1n 4C 3s rr pp vv F7`: setup s, note rr, parameter pp). The port passes the SysEx
//! on, so an XG instrument applies it itself. The built-in synth takes channel messages
//! only, so this module follows the SysEx on the way to it and applies what a note-on can
//! carry: each note's **Level**, as a velocity scale on the rhythm parts' note-ons.
//!
//! The level follows the same curve as the part's CC7 (the SoundFont's velocity and CC7
//! both follow the GM 40·log curve), relative to [`DEFAULT_LEVEL`]. So a level of 80 on a
//! note played at velocity 100 goes out at velocity 80. A level above the default can't
//! lift a note past velocity 127.
//!
//! Pitch, pan, the effect sends, the filter and the EG would need per-voice parameters in
//! the synthesizer. Channel controllers can't do it, because they move every note already
//! sounding on the part. They are left to an XG instrument on the port (see
//! docs/genos-features.md, "Drum Setup").
//!
//! The state follows the Data List's rules (DL "MIDI Parameter Change table (DRUM
//! SETUP)"):
//! - A part plays Drum Setup 1 or 2 when its XG Part Mode is DRUMS1 or DRUMS2
//!   (`F0 43 1n 4C 08 pp 07 02|03 F7`). Part 10 is DRUMS1 until told otherwise.
//! - A program change on a part that plays a drum setup initializes that setup.
//! - A Drum Setup Reset, or an XG/GM/GS system reset, initializes the setups.

/// The parameter address of a drum note's Level.
const LEVEL: u8 = 0x02;
/// XG Multi Part parameter address of the Part Mode.
const PART_MODE: u8 = 0x07;
/// Part Mode values that select Drum Setup 1 and 2.
const DRUMS1: u8 = 0x02;
const DRUMS2: u8 = 0x03;
/// No drum setup on the part / no level set on the note.
const NONE: u8 = 0xFF;
/// The drum setups a Genos has (DL: "n: Drum Setup Number (0 – 1)").
const SETUPS: usize = 2;

/// The Level a note plays at when the setup does not set one: the velocity it arrives
/// with stands. The Data List gives the XG default as "depends on the note", and nothing
/// lists it. Decision: 100. It is the value style authors set most (117 of 1021 corpus
/// Level messages), and their levels spread both ways around it (72–127, mean ≈ 100), as
/// edits of a default in the middle would. 127 as the default would pull every tuned
/// note down by 4 dB on average.
pub const DEFAULT_LEVEL: u8 = 100;

/// The drum setups as the built-in synth should hear them. No allocation: fixed tables,
/// updated message by message on the thread that feeds the synth.
#[derive(Clone)]
pub struct DrumSetups {
    /// Each setup's Level per note (`NONE`: not set).
    level: [[u8; 128]; SETUPS],
    /// The setup each channel's part plays (`NONE`: not a drum-setup part).
    setup_of: [u8; 16],
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
        DrumSetups { level: [[NONE; 128]; SETUPS], setup_of }
    }

    /// Follow one message on its way to the synth (any message: those that don't change a
    /// drum setup are ignored).
    #[inline]
    pub fn observe(&mut self, m: &[u8]) {
        match *m {
            [0xF0, 0x43, d, 0x4C, s @ (0x30 | 0x31), note, LEVEL, v, 0xF7] if d & 0xF0 == 0x10 && note < 128 => {
                self.level[(s - 0x30) as usize][note as usize] = v;
            }
            [0xF0, 0x43, d, 0x4C, 0x08, part, PART_MODE, mode, 0xF7] if d & 0xF0 == 0x10 && part < 16 => {
                self.setup_of[part as usize] = match mode {
                    DRUMS1 => 0,
                    DRUMS2 => 1,
                    _ => NONE,
                };
            }
            [0xF0, 0x43, d, 0x4C, 0x00, 0x00, 0x7D, s, ..] if d & 0xF0 == 0x10 => {
                if let Some(t) = self.level.get_mut(s as usize) {
                    *t = [NONE; 128];
                }
            }
            [0xF0, ..] if crate::sff::is_reset(m) => *self = DrumSetups::new(),
            [st, ..] if st & 0xF0 == 0xC0 => {
                if let Some(t) = self.level.get_mut(self.setup_of[(st & 0x0F) as usize] as usize) {
                    *t = [NONE; 128];
                }
            }
            _ => {}
        }
    }

    /// The velocity a note-on (`m`: status, key, velocity) should reach the synth with:
    /// scaled by its drum setup Level, if its part plays a setup that sets one. Any other
    /// message is unchanged.
    #[inline]
    pub fn apply(&self, m: &mut [u8; 3]) {
        if m[0] & 0xF0 != 0x90 || m[2] == 0 {
            return;
        }
        let Some(t) = self.level.get(self.setup_of[(m[0] & 0x0F) as usize] as usize) else { return };
        let level = t[(m[1] & 0x7F) as usize];
        if level == NONE {
            return;
        }
        let v = (m[2] as u32 * level as u32 + DEFAULT_LEVEL as u32 / 2) / DEFAULT_LEVEL as u32;
        m[2] = v.clamp(1, 127) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(setup: u8, note: u8, v: u8) -> [u8; 9] {
        [0xF0, 0x43, 0x10, 0x4C, 0x30 + setup, note, LEVEL, v, 0xF7]
    }

    fn part_mode(part: u8, mode: u8) -> [u8; 9] {
        [0xF0, 0x43, 0x10, 0x4C, 0x08, part, PART_MODE, mode, 0xF7]
    }

    fn vel(d: &DrumSetups, ch: u8, key: u8, v: u8) -> u8 {
        let mut m = [0x90 | ch, key, v];
        d.apply(&mut m);
        m[2]
    }

    #[test]
    fn level_scales_the_notes_velocity_on_its_setups_part() {
        let mut d = DrumSetups::new();
        // The corpus layout: Rhythm 1 (ch 9) on DRUMS2, Rhythm 2 (ch 10) on DRUMS1.
        d.observe(&part_mode(8, DRUMS2));
        d.observe(&part_mode(9, DRUMS1));
        d.observe(&level(0, 42, 80));
        d.observe(&level(1, 36, 120));
        assert_eq!(vel(&d, 9, 42, 100), 80);
        assert_eq!(vel(&d, 9, 42, 50), 40);
        // Other notes, and the same note on another setup's part, play as sent.
        assert_eq!(vel(&d, 9, 36, 100), 100);
        assert_eq!(vel(&d, 8, 42, 100), 100);
        // Above the default: louder, but never past 127 and never silenced.
        assert_eq!(vel(&d, 8, 36, 100), 120);
        assert_eq!(vel(&d, 8, 36, 127), 127);
        d.observe(&level(0, 42, 0));
        assert_eq!(vel(&d, 9, 42, 100), 1);
        // A melodic part, note-offs and other messages are untouched.
        assert_eq!(vel(&d, 11, 42, 100), 100);
        let mut off = [0x99, 42, 0];
        d.apply(&mut off);
        assert_eq!(off, [0x99, 42, 0]);
        let mut cc = [0xB9, 7, 100];
        d.apply(&mut cc);
        assert_eq!(cc, [0xB9, 7, 100]);
    }

    #[test]
    fn part_ten_plays_drum_setup_1_by_default() {
        let mut d = DrumSetups::new();
        d.observe(&level(0, 38, 50));
        assert_eq!(vel(&d, 9, 38, 100), 50);
        assert_eq!(vel(&d, 8, 38, 100), 100);
        // Part Mode NORMAL takes it off.
        d.observe(&part_mode(9, 0));
        assert_eq!(vel(&d, 9, 38, 100), 100);
    }

    #[test]
    fn a_program_change_or_a_reset_initializes_the_setup() {
        let mut d = DrumSetups::new();
        d.observe(&part_mode(8, DRUMS2));
        d.observe(&level(0, 38, 50));
        d.observe(&level(1, 38, 50));
        // A program change on ch 9 initializes setup 2 only.
        d.observe(&[0xC8, 25]);
        assert_eq!(vel(&d, 8, 38, 100), 100);
        assert_eq!(vel(&d, 9, 38, 100), 50);
        // A program change on a part without a setup changes nothing.
        d.observe(&[0xC3, 0]);
        assert_eq!(vel(&d, 9, 38, 100), 50);
        // Drum Setup Reset (setup 1).
        d.observe(&[0xF0, 0x43, 0x10, 0x4C, 0x00, 0x00, 0x7D, 0x00, 0xF7]);
        assert_eq!(vel(&d, 9, 38, 100), 100);
        // XG System On resets the setups and the part modes.
        d.observe(&level(1, 38, 50));
        d.observe(&[0xF0, 0x43, 0x10, 0x4C, 0x00, 0x00, 0x7E, 0x00, 0xF7]);
        d.observe(&level(1, 38, 50));
        assert_eq!(vel(&d, 8, 38, 100), 100);
    }

    #[test]
    fn malformed_messages_are_ignored() {
        let mut d = DrumSetups::new();
        for m in [&[0xF0, 0x43, 0x10, 0x4C, 0x30, 0x80, LEVEL, 10, 0xF7][..], &[0xF0, 0x43, 0x10, 0x4C, 0x00, 0x00, 0x7D, 9, 0xF7], &[0xF0], &[]] {
            d.observe(m);
        }
        assert_eq!(vel(&d, 9, 0, 100), 100);
        let mut m = [0x99, 0xFF, 100];
        d.apply(&mut m);
        assert_eq!(m[2], 100);
    }
}
