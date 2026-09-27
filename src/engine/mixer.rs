//! The Style parts' levels: the mixer faders, soft takeover, Manual Bass.

use super::*;

/// Soft takeover for an absolute, non-motorised hardware fader controlling a value that
/// software can also move (the Launchkey part and master faders). After software moves
/// the value, the fader is ignored until it comes within `PICKUP_RANGE` of it or crosses
/// it; then it follows again. A fader that has never reported must also pick up first.
#[derive(Clone, Copy, Debug)]
pub struct Takeover {
    /// Last position the fader reported (`HW_UNKNOWN` until it moves).
    hw: u8,
    /// The fader tracks the value.
    picked: bool,
}

/// `Takeover::hw`: the fader has not reported a position yet.
pub const HW_UNKNOWN: u8 = 255;

impl Takeover {
    pub const NEW: Takeover = Takeover { hw: HW_UNKNOWN, picked: false };

    /// The physical fader, last reported at `hw` (`HW_UNKNOWN` if never), is handed the
    /// value `cur` (a fader page switch): it controls it only if it is already there.
    pub fn at(hw: u8, cur: u8) -> Takeover {
        let mut t = Takeover { hw, picked: false };
        t.software_moved(cur);
        t
    }

    /// State kept elsewhere (atomics shared between threads), rebuilt for one report.
    pub fn resume(hw: u8, picked: bool) -> Takeover {
        Takeover { hw, picked }
    }

    pub fn picked(&self) -> bool {
        self.picked
    }

    /// Software set the value to `v`: the fader keeps control only if it is already there.
    pub fn software_moved(&mut self, v: u8) {
        self.picked = self.hw != HW_UNKNOWN && self.hw.abs_diff(v) <= PICKUP_RANGE;
    }

    /// The fader reported `v` while the value is `cur`. True: the fader controls the value
    /// and `v` applies. Crossing is judged from the last report, so a move lost on the way
    /// (a full command ring) still counts as a crossing on the next one.
    pub fn hardware(&mut self, cur: u8, v: u8) -> bool {
        let prev = std::mem::replace(&mut self.hw, v);
        if !self.picked {
            let (c, a, b) = (cur as i16, prev as i16, v as i16);
            let near = v.abs_diff(cur) <= PICKUP_RANGE;
            let crossed = prev != HW_UNKNOWN && (a - c).signum() != (b - c).signum();
            self.picked = near || crossed;
        }
        self.picked
    }

    /// The fader has reported a position but does not control the value yet.
    pub fn waiting(&self) -> bool {
        self.hw != HW_UNKNOWN && !self.picked
    }
}

impl Engine {
    /// Parts the player has not moved go to the style's own level (the SInt CC7) as the
    /// section playing (`self.cur`) routes it (#64: a section that routes another source
    /// channel to a part brings that source's level with its voice). A level already there
    /// is left alone, so its hardware fader keeps control.
    pub(super) fn restore_untouched_levels(&mut self) {
        for p in 0..8 {
            let v = self.style.setup(self.cur).mix[p];
            if self.user_set & (1 << p) == 0 && self.mixer[p] != v {
                self.set_mixer(p, v);
            }
        }
    }

    /// Set a part's fader from software (style load, pattern CC7). A hardware fader that
    /// is not already there has to pick the new value up before it takes control again.
    pub(super) fn set_mixer(&mut self, p: usize, v: u8) {
        self.mixer[p] = v;
        self.takeover[p].software_moved(v);
    }

    /// A CC7 from the style's pattern on `ch`. It is the part's fader value, so it moves the
    /// fader, unless the player has moved that fader since the style loaded.
    pub(super) fn pattern_volume(&mut self, ch: u8, val: u8, sink: &mut impl Sink) {
        if !(8..16).contains(&ch) {
            self.mirror.send(sink, &[0xB0 | ch, 7, val]);
            return;
        }
        let p = (ch - 8) as usize;
        if self.user_set & (1 << p) != 0 {
            return;
        }
        self.set_mixer(p, val);
        let v = self.faded(val);
        self.mirror.send(sink, &[0xB0 | ch, 7, v]);
    }

    /// Style part `part`'s (0-7) own send to bus `b` (0-2: CC91/93/94), 0-127; `UNSENT`
    /// (255): back to the style's (#268). It goes out now, and every CC the style sends
    /// there goes out at it (`Mirror::send`), through section and style changes.
    pub fn set_style_send(&mut self, part: u8, b: u8, value: u8, sink: &mut impl Sink) {
        let (p, b) = ((part & 7) as usize, (b as usize).min(2));
        let v = if value == UNSENT { UNSENT } else { value.min(127) };
        if self.mirror.send_own[p][b] == v {
            return;
        }
        self.mirror.send_own[p][b] = v;
        sink.send(&[0xB8 + p as u8, crate::fx::SEND_CC[b], self.mirror.send_in_effect(p, b)]);
    }

    /// A Style-page fader in a send layer moved from `prev` to `v`: part `part`'s send to
    /// bus `b` follows once the fader has picked it up, as the volume faders do. A new
    /// `generation` (the layer or page changed) binds the fader afresh.
    pub fn style_send_fader(&mut self, part: u8, b: u8, prev: u8, v: u8, generation: u8, sink: &mut impl Sink) {
        let (p, b) = ((part & 7) as usize, b.min(2));
        let cur = self.mirror.send_in_effect(p, b as usize);
        if self.send_bound[p] != generation {
            self.send_take[p] = Takeover::at(prev, cur);
            self.send_bound[p] = generation;
            self.send_last[p] = cur;
        } else if self.send_last[p] != cur {
            self.send_take[p].software_moved(cur);
            self.send_last[p] = cur;
        }
        if self.send_take[p].hardware(cur, v) {
            self.set_style_send(p as u8, b, v, sink);
            self.send_last[p] = self.mirror.send_in_effect(p, b as usize);
        }
    }

    /// Every Style part's own sends at once (`[part][bus]`, `UNSENT` = the style's): a
    /// Registration recall, or Reset to the style.
    pub fn set_style_sends(&mut self, sends: [[u8; 3]; 8], sink: &mut impl Sink) {
        for (p, s) in sends.iter().enumerate() {
            for (b, &v) in s.iter().enumerate() {
                self.set_style_send(p as u8, b as u8, v, sink);
            }
        }
    }

    /// The own sends go out again (after a setup that may have left the style's).
    pub(super) fn send_own_sends(&mut self, sink: &mut impl Sink) {
        for p in 0..8 {
            for b in 0..3 {
                let v = self.mirror.send_own[p][b];
                if v != UNSENT {
                    sink.send(&[0xB8 + p as u8, crate::fx::SEND_CC[b], v]);
                }
            }
        }
    }

    /// Each Style part's send to each bus as it plays (#268), and its own (`UNSENT`: the
    /// style's).
    pub(super) fn style_sends(&self) -> ([[u8; 3]; 8], [[u8; 3]; 8]) {
        (std::array::from_fn(|p| std::array::from_fn(|b| self.mirror.send_in_effect(p, b))), self.mirror.send_own)
    }

    /// A part fader (0..8) moved to `value`: sent as that part's CC7, unchanged (scaled only
    /// while a Fade In/Out runs: fade.rs).
    pub fn set_volume(&mut self, part: u8, value: u8, sink: &mut impl Sink) {
        let p = (part & 7) as usize;
        let v = value.min(127);
        self.mixer[p] = v;
        self.user_set |= 1 << p;
        let v = self.faded(v);
        self.mirror.send(sink, &[0xB0 | (8 + p as u8), 7, v]);
    }

    /// A part's volume set from software (the app's mixer): as a fader move, but the
    /// hardware fader has to pick the new value up before it takes control again.
    pub fn set_volume_from_software(&mut self, part: u8, value: u8, sink: &mut impl Sink) {
        let p = (part & 7) as usize;
        self.set_volume(part, value, sink);
        self.takeover[p].software_moved(self.mixer[p]);
    }

    /// A hardware fader (absolute, not motorised) reported `value` for part 0..8. Soft
    /// takeover: after the software value moved on its own, the fader is ignored until it
    /// comes within `PICKUP_RANGE` of that value or crosses it; then it follows again.
    pub fn hw_fader(&mut self, part: u8, value: u8, sink: &mut impl Sink) {
        let p = (part & 7) as usize;
        let v = value.min(127);
        if self.takeover[p].hardware(self.mixer[p], v) {
            self.set_volume(part, v, sink);
        }
    }

    /// The Launchkey faders now control the Style parts again (fader page switch), from
    /// their physical positions `hw` (`HW_UNKNOWN` = never moved): each picks its part up
    /// only once it reaches the part's level, as after any software move.
    pub fn faders_at(&mut self, hw: [u8; 8]) {
        for ((t, &h), &v) in self.takeover.iter_mut().zip(&hw).zip(&self.mixer) {
            *t = Takeover::at(h, v);
        }
    }

    /// Parts whose hardware fader has reported a position but not yet picked up.
    pub(super) fn pickup_waiting(&self) -> u8 {
        let mut m = 0;
        for (p, t) in self.takeover.iter().enumerate() {
            if t.waiting() {
                m |= 1 << p;
            }
        }
        m
    }

    /// The Style parts that sound (bit = part 0-7): the soloed part alone, whatever its
    /// on/off switch says, else the parts switched on.
    #[inline]
    pub(super) fn audible(&self) -> u8 {
        match self.features.solo {
            Some(p) => 1 << (p & 7),
            None => self.parts,
        }
    }

    /// End the notes of the Style parts that no longer sound.
    pub(super) fn silence_inaudible(&mut self, sink: &mut impl Sink) {
        let audible = self.audible();
        self.off_where(sink, |n| (8..16).contains(&n.dest) && audible & (1 << (n.dest - 8)) == 0);
    }

    /// Solo a Style part (0-7): only it sounds, even if it is switched off; None ends the
    /// solo. The parts' on/off switches are left as they are.
    pub fn set_style_solo(&mut self, part: Option<u8>, sink: &mut impl Sink) {
        self.features.solo = part.map(|p| p & 7);
        self.silence_inaudible(sink);
    }

    /// Switch the Style parts on/off at once (bit = part 0-7): Style Track Mute.
    pub fn set_style_parts(&mut self, mask: u8, sink: &mut impl Sink) {
        self.parts = mask;
        self.silence_inaudible(sink);
    }

    /// Manual Bass on/off: mutes the Style's Bass part (and its Stop Accompaniment note).
    pub fn set_manual_bass(&mut self, on: bool, sink: &mut impl Sink) {
        self.manual_bass = on;
        if on {
            self.off_where(sink, |n| n.dest == BASS_CH);
        }
    }
}
