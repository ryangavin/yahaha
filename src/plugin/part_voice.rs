//! A plugin part's XG voice settings, played in front of the plugin (#247). A plugin gets
//! 3-byte channel messages only, so the XG multi part SysEx an OTS sets never reaches it;
//! the settings with a clean equivalent in yahaha are played here instead, on the notes
//! the plugin gets:
//!
//! - **Mono/Poly** (XG 08 pp 05): in mono, a held-key stack with last-note priority, as
//!   the SoundFont parts' mono (`synth::voicing`, #350) and the Genos: a new note ends the
//!   one sounding; letting go of it goes back to the latest key still held, at that key's
//!   velocity. The hold pedal is played here while mono (the plugin never sees CC64 then),
//!   so it keeps the last note sounding but never two: a note-off it holds is sent when the
//!   pedal comes up, and a new note still ends the kept one. Back to poly, the plugin gets
//!   the player's pedal, then the note-off that was being held.
//! - **Velocity sense depth and offset** (XG 08 pp 0C / 0D): each note-on's velocity goes
//!   through the XG velocity curve, `velocity x depth / 64 + (offset - 64)`, kept to 1-127
//!   (a note-on never turns into a note-off). At the XG defaults (64, 64) it is unchanged.
//!
//! Nothing here allocates, locks or panics: it runs on the audio thread.

/// The keys a mono part keeps to go back to (the oldest drop out beyond), as voicing.rs.
const HELD_KEYS: usize = 16;
/// No key.
const NONE: u8 = 0xFF;
/// The XG default velocity sense depth and offset.
pub const VELOCITY_NEUTRAL: u8 = 64;

/// One part's mono mode and velocity curve, and the keys it holds. See the module docs.
#[derive(Clone, Copy, Debug)]
pub struct PartVoice {
    mono: bool,
    depth: u8,
    offset: u8,
    /// Keys held down, oldest first, with the velocity they sounded at.
    held: [(u8, u8); HELD_KEYS],
    n: u8,
    /// The key sounding on the plugin while mono (`NONE`: none).
    sounding: u8,
    /// The player's hold pedal is down (tracked in poly too, for a switch to mono).
    pedal: bool,
}

impl Default for PartVoice {
    fn default() -> PartVoice {
        PartVoice::new()
    }
}

impl PartVoice {
    pub const fn new() -> PartVoice {
        PartVoice { mono: false, depth: VELOCITY_NEUTRAL, offset: VELOCITY_NEUTRAL, held: [(0, 0); HELD_KEYS], n: 0, sounding: NONE, pedal: false }
    }

    pub fn mono(&self) -> bool {
        self.mono
    }

    /// The XG velocity sense depth (`offset` false) or offset.
    pub fn set_velocity_sense(&mut self, offset: bool, v: u8) {
        if offset {
            self.offset = v & 0x7F;
        } else {
            self.depth = v & 0x7F;
        }
    }

    /// A note-on velocity through the part's curve (1-127).
    #[inline]
    pub fn velocity(&self, v: u8) -> u8 {
        if self.depth == VELOCITY_NEUTRAL && self.offset == VELOCITY_NEUTRAL {
            return v;
        }
        let x = v as i32 * self.depth as i32 / 64 + (self.offset as i32 - 64);
        x.clamp(1, 127) as u8
    }

    /// Switch mono or poly on channel `ch`, sending the plugin what the switch needs (the
    /// pedal changing hands, a held note-off). Notes sounding keep sounding.
    pub fn set_mono(&mut self, ch: u8, mono: bool, out: &mut impl FnMut([u8; 3])) {
        if mono == self.mono {
            return;
        }
        let ch = ch & 0x0F;
        if mono {
            // The pedal is played here now: the plugin's goes up (a note it holds ends).
            if self.pedal {
                out([0xB0 | ch, 64, 0]);
            }
        } else {
            if self.pedal {
                out([0xB0 | ch, 64, 127]);
            }
            // A note kept by the pedal here: the plugin's pedal keeps it now.
            if self.sounding != NONE && !self.is_held(self.sounding) {
                out([0x80 | ch, self.sounding, 0]);
            }
        }
        self.mono = mono;
        self.n = 0;
        self.sounding = NONE;
    }

    /// Play `m` (a channel message for this part) into `out`, which gets what the plugin
    /// plays: the message as it is, or the notes mono and the velocity curve make of it.
    #[inline]
    pub fn play(&mut self, m: [u8; 3], out: &mut impl FnMut([u8; 3])) {
        let ch = m[0] & 0x0F;
        match m[0] & 0xF0 {
            0x90 if m[2] > 0 => {
                let v = self.velocity(m[2]);
                if !self.mono {
                    out([m[0], m[1], v]);
                    return;
                }
                let key = m[1] & 0x7F;
                if self.sounding != NONE {
                    out([0x80 | ch, self.sounding, 0]);
                }
                self.push(key, v);
                out([0x90 | ch, key, v]);
                self.sounding = key;
            }
            0x80 | 0x90 => {
                if !self.mono {
                    out(m);
                    return;
                }
                let key = m[1] & 0x7F;
                self.remove(key);
                if key != self.sounding {
                    return;
                }
                if self.n > 0 {
                    // Back to the latest key still held, at its own velocity.
                    let (k, v) = self.held[self.n as usize - 1];
                    out([0x80 | ch, key, 0]);
                    out([0x90 | ch, k, v]);
                    self.sounding = k;
                } else if !self.pedal {
                    out([0x80 | ch, key, 0]);
                    self.sounding = NONE;
                }
            }
            0xB0 if m[1] == 64 => {
                self.pedal = m[2] >= 64;
                if !self.mono {
                    out(m);
                    return;
                }
                if !self.pedal && self.sounding != NONE && !self.is_held(self.sounding) {
                    out([0x80 | ch, self.sounding, 0]);
                    self.sounding = NONE;
                }
            }
            0xB0 if matches!(m[1], 120 | 123..=127) => {
                // All Sound/Notes Off and the mode messages end every note.
                self.n = 0;
                self.sounding = NONE;
                out(m);
            }
            0xB0 if m[1] == 121 => {
                self.pedal = false;
                out(m);
            }
            _ => out(m),
        }
    }

    fn is_held(&self, key: u8) -> bool {
        self.held[..self.n as usize].iter().any(|h| h.0 == key)
    }

    fn remove(&mut self, key: u8) {
        let n = self.n as usize;
        if let Some(i) = self.held[..n].iter().position(|h| h.0 == key) {
            self.held.copy_within(i + 1..n, i);
            self.n -= 1;
        }
    }

    fn push(&mut self, key: u8, v: u8) {
        self.remove(key);
        if self.n as usize == HELD_KEYS {
            self.held.copy_within(1.., 0);
            self.n -= 1;
        }
        self.held[self.n as usize] = (key, v);
        self.n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(v: &mut PartVoice, msgs: &[[u8; 3]]) -> Vec<[u8; 3]> {
        let mut out = Vec::new();
        for &m in msgs {
            v.play(m, &mut |x| out.push(x));
        }
        out
    }

    #[test]
    fn poly_at_the_xg_defaults_passes_everything_as_it_is() {
        let mut v = PartVoice::new();
        let msgs = [[0x92, 60, 100], [0x92, 64, 1], [0xB2, 64, 127], [0x82, 60, 0], [0x92, 64, 0], [0xE2, 0, 70], [0xB2, 74, 30]];
        assert_eq!(run(&mut v, &msgs), msgs);
    }

    #[test]
    fn velocity_sense_curves_the_note_ons_only() {
        let mut v = PartVoice::new();
        v.set_velocity_sense(false, 32);
        assert_eq!(run(&mut v, &[[0x90, 60, 100], [0x80, 60, 100], [0x90, 60, 0]]), [[0x90, 60, 50], [0x80, 60, 100], [0x90, 60, 0]]);
        v.set_velocity_sense(false, 64);
        v.set_velocity_sense(true, 84);
        assert_eq!(run(&mut v, &[[0x90, 60, 100], [0x90, 61, 120]]), [[0x90, 60, 120], [0x90, 61, 127]], "+20, kept to 127");
        v.set_velocity_sense(true, 0);
        assert_eq!(run(&mut v, &[[0x90, 60, 30]]), [[0x90, 60, 1]], "never a note-off");
        // Depth 127: about twice as steep.
        v.set_velocity_sense(true, 64);
        v.set_velocity_sense(false, 127);
        assert_eq!(v.velocity(40), 79);
    }

    #[test]
    fn mono_plays_one_note_last_note_priority() {
        let mut v = PartVoice::new();
        v.set_mono(1, true, &mut |_| panic!("nothing to send"));
        let out = run(&mut v, &[[0x91, 60, 100], [0x91, 64, 90], [0x91, 67, 80]]);
        assert_eq!(out, [[0x91, 60, 100], [0x81, 60, 0], [0x91, 64, 90], [0x81, 64, 0], [0x91, 67, 80]]);
        // Letting go of the sounding key goes back to the latest held, at its velocity.
        assert_eq!(run(&mut v, &[[0x81, 67, 0]]), [[0x81, 67, 0], [0x91, 64, 90]]);
        // Letting go of a key not sounding sends nothing.
        assert_eq!(run(&mut v, &[[0x91, 60, 0]]), Vec::<[u8; 3]>::new());
        assert_eq!(run(&mut v, &[[0x81, 64, 0]]), [[0x81, 64, 0]]);
    }

    #[test]
    fn mono_plays_the_hold_pedal_itself() {
        let mut v = PartVoice::new();
        v.set_mono(0, true, &mut |_| {});
        let out = run(&mut v, &[[0xB0, 64, 127], [0x90, 60, 100], [0x80, 60, 0]]);
        assert_eq!(out, [[0x90, 60, 100]], "the plugin never sees the pedal; the note is kept");
        // A new note still ends the kept one.
        assert_eq!(run(&mut v, &[[0x90, 62, 100], [0x80, 62, 0]]), [[0x80, 60, 0], [0x90, 62, 100]]);
        assert_eq!(run(&mut v, &[[0xB0, 64, 0]]), [[0x80, 62, 0]], "the pedal up ends it");
        // Back to poly under the pedal: the plugin gets the pedal, then the held note-off.
        run(&mut v, &[[0xB0, 64, 127], [0x90, 65, 100], [0x80, 65, 0]]);
        let mut out = Vec::new();
        v.set_mono(0, false, &mut |x| out.push(x));
        assert_eq!(out, [[0xB0, 64, 127], [0x80, 65, 0]]);
        assert_eq!(run(&mut v, &[[0x90, 60, 100], [0x90, 64, 100]]), [[0x90, 60, 100], [0x90, 64, 100]], "poly again");
        // To mono under the pedal: the plugin's pedal goes up.
        let mut out = Vec::new();
        v.set_mono(0, true, &mut |x| out.push(x));
        assert_eq!(out, [[0xB0, 64, 0]]);
    }

    #[test]
    fn the_held_keys_stay_bounded() {
        let mut v = PartVoice::new();
        v.set_mono(0, true, &mut |_| {});
        for k in 0..40u8 {
            run(&mut v, &[[0x90, k, 100]]);
        }
        assert_eq!(v.n as usize, HELD_KEYS);
        // All Notes Off forgets them.
        assert_eq!(run(&mut v, &[[0xB0, 123, 0]]), [[0xB0, 123, 0]]);
        assert_eq!(run(&mut v, &[[0x80, 39, 0]]), Vec::<[u8; 3]>::new());
    }
}
