//! Mono mode and portamento (#246), played in yahaha's MIDI layer on upstream rustysynth's
//! primitives (#346 step 2): note-on, note-off, the hold pedal, RPN 0 (bend range) and
//! pitch bend. rustysynth itself never sees CC5, CC65, CC126 or CC127.
//!
//! Each part (MIDI channel) has its own synthesizer, so its 16 MIDI channels are free: a
//! part's messages go to all of them (but rustysynth's percussion channel 10), and its
//! notes play on its own channel unless portamento is on (or the part shapes its notes'
//! envelopes, envelope.rs). Then each note takes a channel of its own, the one used longest
//! ago, so each note can have a pitch bend (or an expression) of its own.
//!
//! - **Mono** (CC126/127, or the XG part's Mono/Poly): a held-key stack, last-note
//!   priority. A new note ends the one sounding; letting go of it goes back to the latest
//!   key still held (with its velocity, gliding there with portamento), as the Genos does.
//!   The hold pedal keeps the last note sounding, but never two: in mono the part's hold
//!   pedal is played here (a note-off it holds is sent when the pedal comes up), so a new
//!   note can end the one kept.
//! - **Portamento** (CC65 on, CC5 time above 0): a melodic note glides from the key played
//!   before, at a fixed rate (the XG and Genos default: an octave in 20 ms x 2^(time / 16),
//!   about 30 ms at 8, 0.3 s at 64, 5 s at 127). The glide is a pitch-bend ramp on the
//!   note's channel, a step per synthesizer block, added to the player's own bend; the bend
//!   range (RPN 0) is set wide enough while it glides and put back after.
//!
//! Drum kits (bank 128 and up, and channel 10) play poly with no glide, as in XG. Nothing
//! here allocates, locks or panics: it runs on the audio thread.

/// Which of a part's lanes (synthesizers) a message goes to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum To {
    /// The lane of the font the channel plays now (a note-on).
    Live,
    /// The lanes of that font (bank and program).
    Slot,
    /// Every lane.
    All,
    /// Lane `i` (a glide's, or a shaped note's: envelope.rs).
    Lane(u8),
    /// Every lane but lane `i`.
    AllBut(u8),
}

/// The keys a mono part keeps to go back to (the oldest drop out beyond).
const HELD_KEYS: usize = 16;
/// rustysynth's percussion channel (MIDI channel 10).
const PERCUSSION: u8 = 9;
/// `Voicing::dtype`: the last parameter select, as rustysynth's channel keeps it.
const SELECT_NONE: u8 = 0;
const SELECT_RPN: u8 = 1;
const SELECT_NRPN: u8 = 2;

/// A note's glide on its channel.
#[derive(Clone, Copy, Default)]
struct Glide {
    /// The lane the note plays in.
    lane: u8,
    /// Semitones it sounds away from its key in the next block.
    offset: f32,
    /// Semitones it moves each block.
    step: f32,
    /// The bend range set on the channel for it, in semitones (0: the player's own).
    range: u8,
}

/// A part's mono mode and portamento, and where its notes play (see the module docs).
pub(super) struct Voicing {
    /// The part's own channel.
    home: u8,
    /// The channels its messages go to (bit = channel).
    chans: u16,
    /// A synthesizer block, in seconds (a glide steps once a block).
    block_s: f32,
    mono: bool,
    /// Mono on a melodic voice, as last seen (the hold pedal is played here then).
    mono_now: bool,
    porta: bool,
    porta_time: u8,
    /// Each note on a channel of its own, away from these channels if it can be (the
    /// part's envelope shaping, envelope.rs), or None.
    spread: Option<u16>,
    /// The latest melodic key played (-1: none), where a glide starts.
    last_key: i32,
    /// Mono: the keys held, oldest first, with their velocities.
    held: [(u8, u8); HELD_KEYS],
    n_held: usize,
    /// The player's hold pedal, and what the part's synthesizer channels have.
    pedal: bool,
    synth_pedal: bool,
    /// Mono: keys let go under the hold pedal, their note-offs not sent yet (bit = key).
    deferred: u128,
    /// The keys sounding (bit = key), and the channels each plays on.
    on: u128,
    key_chans: [u16; 128],
    /// When each channel last got a note (for the one used longest ago).
    stamp: [u32; 16],
    clock: u32,
    /// The player's parameter select, bend range and pitch bend, as rustysynth keeps them.
    rpn: i16,
    dtype: u8,
    range: i16,
    bend: u16,
    /// Glides under way (bit = channel).
    glides: [Glide; 16],
    gliding: u16,
}

impl Voicing {
    /// Part `home`'s, on synthesizers with blocks of `block` frames at `sample_rate`.
    pub(super) fn new(home: u8, block: usize, sample_rate: i32) -> Voicing {
        let home = home & 15;
        let chans = if home == PERCUSSION { 1 << PERCUSSION } else { 0xFFFF & !(1 << PERCUSSION) };
        Voicing {
            home,
            chans,
            block_s: block as f32 / sample_rate as f32,
            mono: false,
            mono_now: false,
            porta: false,
            porta_time: 0,
            spread: None,
            last_key: -1,
            held: [(0, 0); HELD_KEYS],
            n_held: 0,
            pedal: false,
            synth_pedal: false,
            deferred: 0,
            on: 0,
            key_chans: [0; 128],
            stamp: [0; 16],
            clock: 0,
            rpn: -1,
            dtype: SELECT_NONE,
            range: 2 << 7,
            bend: 8192,
            glides: [Glide::default(); 16],
            gliding: 0,
        }
    }

    /// The part's own channel.
    #[inline]
    pub(super) fn home(&self) -> u8 {
        self.home
    }

    /// The channels the part's messages go to (bit = channel).
    #[inline]
    pub(super) fn chans(&self) -> u16 {
        self.chans
    }

    /// Each note on a channel of its own, away from the channels in `avoid` if it can be
    /// (Some), or on the part's own channel unless it glides (None).
    #[inline]
    pub(super) fn set_spread(&mut self, spread: Option<u16>) {
        self.spread = spread;
    }

    /// Whether a glide plays in lane `lane` (it renders a block at a time then).
    #[inline]
    pub(super) fn glides_in(&self, lane: usize) -> bool {
        let mut m = self.gliding;
        while m != 0 {
            let c = m.trailing_zeros() as usize;
            m &= m - 1;
            if self.glides[c].lane as usize == lane {
                return true;
            }
        }
        false
    }

    /// A channel message to the part; `melodic`: it plays a melodic voice now, `live`: the
    /// lane of the font it plays. `emit(to, channels, status, data1, data2)` sends to the
    /// synthesizers.
    pub(super) fn process(&mut self, st: i32, d1: i32, d2: i32, melodic: bool, live: u8, emit: &mut impl FnMut(To, u16, i32, i32, i32)) {
        self.refresh(melodic, emit);
        let chans = self.chans;
        let v = d2 & 127;
        match st {
            0x90 if v > 0 => self.note_on(d1 & 127, v, melodic, live, emit),
            0x80 | 0x90 => self.note_off(d1 & 127, melodic, live, emit),
            0xB0 => match d1 {
                0 | 32 => emit(To::Slot, chans, st, d1, d2),
                // Portamento time and switch: played here.
                5 => self.porta_time = v as u8,
                65 => self.porta = v >= 64,
                64 => {
                    self.pedal = v >= 64;
                    if !self.mono_now {
                        self.synth_pedal = self.pedal;
                        emit(To::All, chans, st, d1, d2);
                    } else if !self.pedal {
                        if self.synth_pedal {
                            self.synth_pedal = false;
                            emit(To::All, chans, st, d1, d2);
                        }
                        self.flush(emit);
                    }
                }
                // Mono / Poly Mode On, with All Notes Off.
                126 | 127 => {
                    self.set_mono(d1 == 126, melodic, emit);
                    emit(To::All, chans, 0xB0, 123, 0);
                    self.all_off();
                }
                120 | 123 => {
                    emit(To::All, chans, st, d1, d2);
                    self.all_off();
                }
                // Reset All Controllers: rustysynth's (the pedal up, no select, bend
                // centred), and portamento off (GM2 RP-015, XG).
                121 => {
                    emit(To::All, chans, st, d1, d2);
                    self.porta = false;
                    self.pedal = false;
                    self.synth_pedal = false;
                    self.rpn = -1;
                    self.bend = 8192;
                    self.flush(emit);
                }
                101 => {
                    self.rpn = (self.rpn & 0x7F) | (v << 7) as i16;
                    self.dtype = SELECT_RPN;
                    emit(To::All, chans, st, d1, d2);
                }
                100 => {
                    self.rpn = (((self.rpn as i32) & 0xFF80) | v) as i16;
                    self.dtype = SELECT_RPN;
                    emit(To::All, chans, st, d1, d2);
                }
                98 | 99 => {
                    self.dtype = SELECT_NRPN;
                    emit(To::All, chans, st, d1, d2);
                }
                6 | 38 => {
                    if self.dtype == SELECT_RPN && self.rpn == 0 {
                        self.range = if d1 == 6 { (self.range & 0x7F) | (v << 7) as i16 } else { (((self.range as i32) & 0xFF80) | v) as i16 };
                        // Every channel has the player's range again.
                        for g in &mut self.glides {
                            g.range = 0;
                        }
                    }
                    emit(To::All, chans, st, d1, d2);
                }
                _ => emit(To::All, chans, st, d1, d2),
            },
            0xC0 => emit(To::Slot, chans, st, d1, d2),
            0xE0 => {
                self.bend = ((d1 & 127) | (d2 & 127) << 7) as u16;
                emit(To::All, chans, st, d1, d2);
            }
            _ => emit(To::All, chans, st, d1, d2),
        }
    }

    /// Mono (true) or poly mode, without the All Notes Off of CC126/127 (the XG part's
    /// Mono/Poly): the keys held are forgotten.
    pub(super) fn set_mono(&mut self, mono: bool, melodic: bool, emit: &mut impl FnMut(To, u16, i32, i32, i32)) {
        if self.mono != mono {
            self.mono = mono;
            self.n_held = 0;
        }
        self.refresh(melodic, emit);
    }

    /// Mono on a melodic voice or not: on the way to poly, the synthesizers take the
    /// player's hold pedal again, and the note-offs it held here.
    fn refresh(&mut self, melodic: bool, emit: &mut impl FnMut(To, u16, i32, i32, i32)) {
        let now = self.mono && melodic;
        if now == self.mono_now {
            return;
        }
        self.mono_now = now;
        if !now {
            if self.synth_pedal != self.pedal {
                self.synth_pedal = self.pedal;
                emit(To::All, self.chans, 0xB0, 64, if self.pedal { 127 } else { 0 });
            }
            self.flush(emit);
        }
    }

    fn note_on(&mut self, key: i32, velocity: i32, melodic: bool, live: u8, emit: &mut impl FnMut(To, u16, i32, i32, i32)) {
        if self.mono_now {
            self.unhold(key);
            if self.n_held == HELD_KEYS {
                self.held.copy_within(1.., 0);
                self.n_held -= 1;
            }
            self.held[self.n_held] = (key as u8, velocity as u8);
            self.n_held += 1;
            self.cut(emit);
        }
        self.start(key, velocity, melodic, live, emit);
    }

    fn note_off(&mut self, key: i32, melodic: bool, live: u8, emit: &mut impl FnMut(To, u16, i32, i32, i32)) {
        if self.mono_now {
            // The key sounding let go while others are held: back to the latest of them,
            // gliding from the key let go.
            if self.unhold(key)
                && let Some(&(k, v)) = self.held[..self.n_held].last()
            {
                self.cut(emit);
                self.last_key = key;
                self.start(k as i32, v as i32, melodic, live, emit);
                return;
            }
            // Kept by the hold pedal (played here in mono) until it comes up.
            if self.pedal && !self.synth_pedal && self.key_chans[key as usize] != 0 {
                self.deferred |= 1u128 << key;
                return;
            }
        }
        self.end(key as usize, emit);
    }

    /// Start `key`'s note: on the part's own channel, or with portamento (or envelope
    /// shaping) on a channel of its own, gliding from the key played before.
    fn start(&mut self, key: i32, velocity: i32, melodic: bool, live: u8, emit: &mut impl FnMut(To, u16, i32, i32, i32)) {
        let porta = self.porta && melodic;
        let c = if porta || self.spread.is_some() { self.pick() } else { self.home as usize };
        let bit = 1u16 << c;
        // A glide left on the channel (its note gone) ends first.
        if self.gliding & bit != 0 {
            let lane = self.glides[c].lane;
            self.restore(c, &mut |ch, st, a, b| emit(To::Lane(lane), 1u16 << ch, st, a, b));
            self.gliding &= !bit;
        }
        if porta && self.porta_time > 0 && self.last_key >= 0 && self.last_key != key {
            let octave = 0.02_f32 * (self.porta_time as f32 / 16_f32).exp2();
            let rate = 12_f32 / octave;
            self.glides[c] = Glide { lane: live, offset: (self.last_key - key) as f32, step: rate * self.block_s, range: 0 };
            self.gliding |= bit;
        }
        emit(To::Live, bit, 0x90, key, velocity);
        let k = key as usize & 127;
        self.on |= 1 << k;
        self.key_chans[k] |= bit;
        self.clock = self.clock.wrapping_add(1);
        self.stamp[c] = self.clock;
        if melodic {
            self.last_key = key;
        }
    }

    /// `key`'s note-off, on every channel it plays on.
    fn end(&mut self, key: usize, emit: &mut impl FnMut(To, u16, i32, i32, i32)) {
        let k = key & 127;
        let m = if self.key_chans[k] != 0 { self.key_chans[k] } else { 1 << self.home };
        emit(To::All, m, 0x80, k as i32, 0);
        self.key_chans[k] = 0;
        self.on &= !(1 << k);
        self.deferred &= !(1 << k);
    }

    /// Mono: end every note the part sounds, the hold pedal notwithstanding.
    fn cut(&mut self, emit: &mut impl FnMut(To, u16, i32, i32, i32)) {
        if self.synth_pedal {
            self.synth_pedal = false;
            emit(To::All, self.chans, 0xB0, 64, 0);
        }
        while self.on != 0 {
            let k = self.on.trailing_zeros() as usize;
            self.end(k, emit);
        }
    }

    /// Send the note-offs the hold pedal kept here.
    fn flush(&mut self, emit: &mut impl FnMut(To, u16, i32, i32, i32)) {
        while self.deferred != 0 {
            let k = self.deferred.trailing_zeros() as usize;
            self.end(k, emit);
        }
    }

    /// No note sounding, no key held (All Notes Off, All Sound Off, CC126/127).
    fn all_off(&mut self) {
        self.on = 0;
        self.deferred = 0;
        self.key_chans = [0; 128];
        self.n_held = 0;
    }

    /// Mono: `key` off the held stack; true if it was the latest (the one sounding).
    fn unhold(&mut self, key: i32) -> bool {
        let n = self.n_held;
        match self.held[..n].iter().position(|h| h.0 as i32 == key) {
            Some(i) => {
                self.held.copy_within(i + 1..n, i);
                self.n_held -= 1;
                i + 1 == n
            }
            None => false,
        }
    }

    /// The channel for a note of its own: the one used longest ago, among those with no
    /// note held (and none shaped, `spread`) if there are any.
    fn pick(&self) -> usize {
        let mut busy = 0u16;
        let mut on = self.on;
        while on != 0 {
            let k = on.trailing_zeros() as usize;
            on &= on - 1;
            busy |= self.key_chans[k];
        }
        let quiet = self.chans & !busy & !self.spread.unwrap_or(0);
        let free = if quiet != 0 {
            quiet
        } else if self.chans & !busy != 0 {
            self.chans & !busy
        } else {
            self.chans
        };
        let (mut best, mut age) = (self.home as usize, None);
        let mut m = free;
        while m != 0 {
            let c = m.trailing_zeros() as usize;
            m &= m - 1;
            let a = self.clock.wrapping_sub(self.stamp[c]);
            if age.is_none_or(|x| a > x) {
                (best, age) = (c, Some(a));
            }
        }
        best
    }

    /// The player's bend range, in semitones (rustysynth's reading of RPN 0).
    #[inline]
    fn range_semitones(&self) -> f32 {
        (self.range >> 7) as f32 + 0.01_f32 * (self.range & 0x7F) as f32
    }

    /// Before lane `lane` renders a block: each glide on it one step on (`emit(channel,
    /// status, data1, data2)` to that lane's synthesizer). A glide done puts the player's
    /// bend range and bend back.
    pub(super) fn block(&mut self, lane: usize, emit: &mut impl FnMut(i32, i32, i32, i32)) {
        let mut m = self.gliding;
        while m != 0 {
            let c = m.trailing_zeros() as usize;
            m &= m - 1;
            if self.glides[c].lane as usize != lane {
                continue;
            }
            let g = self.glides[c];
            if g.offset == 0_f32 {
                self.restore(c, emit);
                self.gliding &= !(1 << c);
                continue;
            }
            let user = self.range_semitones();
            let bent = user * ((self.bend as i32 - 8192) as f32 / 8192_f32);
            let need = (user + g.offset.abs()).ceil().clamp(1_f32, 127_f32) as u8;
            if g.range != need {
                self.set_range(c, need as i32, 0, emit);
                self.glides[c].range = need;
            }
            let v = (((bent + g.offset) / need as f32) * 8192_f32).round() as i32 + 8192;
            let v = v.clamp(0, 16383);
            emit(c as i32, 0xE0, v & 127, v >> 7);
            self.glides[c].offset = if g.offset.abs() <= g.step { 0_f32 } else { g.offset - g.step.copysign(g.offset) };
        }
    }

    /// Channel `c`'s bend range and bend back to the player's.
    fn restore(&mut self, c: usize, emit: &mut impl FnMut(i32, i32, i32, i32)) {
        if self.glides[c].range != 0 {
            let r = self.range as i32;
            self.set_range(c, (r >> 7) & 127, r & 127, emit);
            self.glides[c].range = 0;
        }
        emit(c as i32, 0xE0, (self.bend & 127) as i32, (self.bend >> 7) as i32);
    }

    /// Channel `c`'s bend range (RPN 0) to `msb` semitones and `lsb` cents, then the
    /// player's parameter select back.
    fn set_range(&self, c: usize, msb: i32, lsb: i32, emit: &mut impl FnMut(i32, i32, i32, i32)) {
        let c = c as i32;
        for (cc, v) in [(101, 0), (100, 0), (6, msb), (38, lsb)] {
            emit(c, 0xB0, cc, v);
        }
        // A select of none (-1) goes back as 127/127: data entry reaches neither.
        let rpn = self.rpn as i32;
        emit(c, 0xB0, 101, (rpn >> 7) & 127);
        emit(c, 0xB0, 100, rpn & 127);
        if self.dtype == SELECT_NRPN {
            emit(c, 0xB0, 99, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Out = (To, u16, i32, i32, i32);

    /// Part 1 (channel 0), on 64-frame blocks at 48 kHz.
    fn part() -> Voicing {
        Voicing::new(0, 64, 48_000)
    }

    /// `msgs` to `v` (a melodic voice, on lane 0), and what it sends on.
    fn feed(v: &mut Voicing, msgs: &[[i32; 3]]) -> Vec<Out> {
        feed_as(v, msgs, true)
    }

    fn feed_as(v: &mut Voicing, msgs: &[[i32; 3]], melodic: bool) -> Vec<Out> {
        let mut out = Vec::new();
        for &[st, d1, d2] in msgs {
            v.process(st, d1, d2, melodic, 0, &mut |to, c, st, a, b| out.push((to, c, st, a, b)));
        }
        out
    }

    /// The notes sent: (on, key, velocity) for a note-on, (off, key, 0) for a note-off.
    fn notes(out: &[Out]) -> Vec<(bool, i32, i32)> {
        out.iter().filter(|o| o.2 == 0x90 || o.2 == 0x80).map(|o| (o.2 == 0x90, o.3, o.4)).collect()
    }

    /// Lane 0's next block: the messages it sends there.
    fn block(v: &mut Voicing) -> Vec<(i32, i32, i32, i32)> {
        let mut out = Vec::new();
        v.block(0, &mut |c, st, a, b| out.push((c, st, a, b)));
        out
    }

    /// The pitch bend a block sets on channel `c`, in semitones on the range it set.
    fn bent(out: &[(i32, i32, i32, i32)], c: i32, range: f32) -> Option<f32> {
        out.iter().rev().find(|o| o.0 == c && o.1 == 0xE0).map(|o| range * (((o.2 | o.3 << 7) - 8192) as f32 / 8192.0))
    }

    /// The bend range (RPN 0) a block sets on channel `c`, if it sets one.
    fn range_set(out: &[(i32, i32, i32, i32)], c: i32) -> Option<(i32, i32)> {
        let cc: Vec<_> = out.iter().filter(|o| o.0 == c && o.1 == 0xB0).map(|o| (o.2, o.3)).collect();
        let i = cc.windows(4).position(|w| w[0] == (101, 0) && w[1] == (100, 0) && w[2].0 == 6 && w[3].0 == 38)?;
        Some((cc[i + 2].1, cc[i + 3].1))
    }

    /// Mono: last-note priority. A new note ends the one sounding; letting go of it goes
    /// back to the latest key still held, with that key's velocity; letting go of a key
    /// not sounding changes nothing.
    #[test]
    fn mono_plays_the_latest_key_held() {
        let mut v = part();
        let out = feed(&mut v, &[[0xB0, 126, 0]]);
        assert_eq!(out, [(To::All, v.chans(), 0xB0, 123, 0)], "CC126: All Notes Off, and no CC126 to the synthesizer");
        assert_eq!(notes(&feed(&mut v, &[[0x90, 60, 100]])), [(true, 60, 100)]);
        assert_eq!(notes(&feed(&mut v, &[[0x90, 64, 90]])), [(false, 60, 0), (true, 64, 90)]);
        assert_eq!(notes(&feed(&mut v, &[[0x90, 67, 80]])), [(false, 64, 0), (true, 67, 80)]);
        let out = notes(&feed(&mut v, &[[0x80, 64, 0]]));
        assert!(out.iter().all(|n| !n.0), "a key not sounding let go: no note starts {out:?}");
        assert_eq!(notes(&feed(&mut v, &[[0x80, 67, 0]])), [(false, 67, 0), (true, 60, 100)], "back to C4, at its velocity");
        assert_eq!(notes(&feed(&mut v, &[[0x90, 60, 0]])), [(false, 60, 0)], "the last key up: nothing to go back to");
        // Every note on the part's own channel (no portamento).
        assert!(feed(&mut v, &[[0x90, 62, 1]]).iter().all(|o| o.1 == 1));
    }

    /// Mono keeps the latest 16 keys held to go back to; the oldest drops out beyond.
    #[test]
    fn mono_keeps_sixteen_keys() {
        let mut v = part();
        feed(&mut v, &[[0xB0, 126, 0]]);
        for k in 40..57 {
            feed(&mut v, &[[0x90, k, 100]]);
        }
        let mut back = Vec::new();
        for k in (41..57).rev() {
            back.extend(notes(&feed(&mut v, &[[0x80, k, 0]])).into_iter().filter(|n| n.0).map(|n| n.1));
        }
        assert_eq!(back, (41..56).rev().collect::<Vec<_>>(), "back through the keys held, down to the 16th latest");
    }

    /// Mono under the hold pedal: the last note let go keeps sounding, but a new note ends
    /// it; the pedal coming up ends it. The pedal is played here: the synthesizer never
    /// holds a mono note past the next.
    #[test]
    fn mono_hold_pedal_keeps_one_note() {
        let mut v = part();
        feed(&mut v, &[[0xB0, 126, 0]]);
        assert_eq!(feed(&mut v, &[[0xB0, 64, 127]]), [], "the pedal down: played here");
        feed(&mut v, &[[0x90, 60, 100]]);
        assert_eq!(notes(&feed(&mut v, &[[0x80, 60, 0]])), [], "let go under the pedal: it sounds on");
        assert_eq!(notes(&feed(&mut v, &[[0x90, 62, 100]])), [(false, 60, 0), (true, 62, 100)], "the next note ends it");
        feed(&mut v, &[[0x80, 62, 0]]);
        assert_eq!(notes(&feed(&mut v, &[[0xB0, 64, 0]])), [(false, 62, 0)], "the pedal up ends the note kept");

        // The pedal down before mono: the synthesizer has it; the first mono note lifts it.
        let mut v = part();
        feed(&mut v, &[[0xB0, 64, 127], [0x90, 50, 100], [0xB0, 126, 0]]);
        let out = feed(&mut v, &[[0x90, 60, 100]]);
        assert_eq!(out[0], (To::All, v.chans(), 0xB0, 64, 0), "the pedal up on the synthesizer first: {out:?}");
        // Poly again: the synthesizer takes the pedal back, then the note-offs it kept.
        feed(&mut v, &[[0x80, 60, 0]]);
        let out = feed(&mut v, &[[0xB0, 127, 0]]);
        assert_eq!(out[0], (To::All, v.chans(), 0xB0, 64, 127), "{out:?}");
    }

    /// Poly and drum kits play every note; CC5 and CC65 never reach the synthesizer.
    #[test]
    fn poly_and_drums_play_every_note() {
        let mut v = part();
        let out = feed(&mut v, &[[0xB0, 5, 40], [0xB0, 65, 0], [0x90, 60, 100], [0x90, 64, 100]]);
        assert_eq!(notes(&out), [(true, 60, 100), (true, 64, 100)]);
        assert!(out.iter().all(|o| o.2 != 0xB0), "no CC5 or CC65: {out:?}");
        feed_as(&mut v, &[[0xB0, 126, 0]], false);
        let out = feed_as(&mut v, &[[0x90, 36, 100], [0x90, 38, 100]], false);
        assert_eq!(notes(&out), [(true, 36, 100), (true, 38, 100)], "a drum kit plays poly in mono mode");
        // A part's other messages reach all of its channels but rustysynth's percussion one.
        assert_eq!(feed(&mut v, &[[0xB0, 7, 90]]), [(To::All, 0xFFFF & !(1 << 9), 0xB0, 7, 90)]);
        let mut drums = Voicing::new(9, 64, 48_000);
        assert_eq!(feed(&mut drums, &[[0xB0, 7, 90]]), [(To::All, 1 << 9, 0xB0, 7, 90)]);
    }

    /// Portamento: each note on a channel of its own, gliding from the key before as a
    /// pitch-bend ramp, a step a block at the XG rate, on a bend range set wide enough;
    /// then the player's range and bend back, exactly.
    #[test]
    fn portamento_ramps_the_bend_on_the_notes_channel() {
        let mut v = part();
        feed(&mut v, &[[0xB0, 65, 127], [0xB0, 5, 64]]);
        let first = feed(&mut v, &[[0x90, 60, 100]]);
        assert!(!v.glides_in(0), "the first note: nothing to glide from");
        let second = feed(&mut v, &[[0x90, 72, 100]]);
        let (c1, c2) = (first[0].1, second[0].1);
        assert!(c1 != c2 && c1.count_ones() == 1 && c2.count_ones() == 1, "a channel each: {c1:#x} {c2:#x}");
        assert!(v.glides_in(0) && !v.glides_in(1));
        let c = c2.trailing_zeros() as i32;
        // Time 64: an octave in 20 ms x 2^4 = 0.32 s: 12 / 0.32 semitones a second.
        let step = 12.0 / 0.32 * (64.0 / 48_000.0);
        let out = block(&mut v);
        assert_eq!(range_set(&out, c), Some((14, 0)), "the player's 2 plus the 12 to glide");
        assert!((bent(&out, c, 14.0).unwrap() + 12.0).abs() < 14.0 / 16384.0, "starts on C4: {out:?}");
        assert!(out.iter().all(|o| o.0 == c), "only the gliding note's channel");
        let (mut last, mut range, mut blocks) = (-12.0, 14.0, 1);
        loop {
            let out = block(&mut v);
            blocks += 1;
            if let Some((msb, lsb)) = range_set(&out, c) {
                if (msb, lsb) == (2, 0) {
                    assert_eq!(out.last(), Some(&(c, 0xE0, 0, 64)), "the player's bend back: {out:?}");
                    break;
                }
                range = msb as f32;
            }
            let now = bent(&out, c, range).unwrap();
            assert!((now - last - step).abs() < 0.01 || (now.abs() < 0.01 && now > last), "a step a block: {last} -> {now}");
            last = now;
            assert!(blocks < 1000);
        }
        assert!(!v.glides_in(0), "done");
        // About 0.32 s (the octave) of 64-frame blocks.
        assert!((blocks as f32 - 1.0 - 0.32 * 48_000.0 / 64.0).abs() <= 2.0, "{blocks} blocks");
    }

    /// A glide adds to the player's own bend, on the range the player set; the player's
    /// parameter select (here an NRPN) is put back after the range is set.
    #[test]
    fn a_glide_adds_to_the_players_bend() {
        let mut v = part();
        feed(&mut v, &[[0xB0, 101, 0], [0xB0, 100, 0], [0xB0, 6, 12], [0xB0, 38, 0], [0xB0, 99, 1], [0xB0, 98, 2]]);
        feed(&mut v, &[[0xE0, 127, 127], [0xB0, 65, 127], [0xB0, 5, 100], [0x90, 60, 100], [0x90, 65, 100]]);
        let out = block(&mut v);
        let c = out[0].0;
        assert_eq!(range_set(&out, c), Some((17, 0)), "12 of the player's and 5 to glide");
        let want = 12.0 * 8191.0 / 8192.0 - 5.0;
        assert!((bent(&out, c, 17.0).unwrap() - want).abs() < 17.0 / 16384.0, "{out:?}");
        let cc: Vec<_> = out.iter().filter(|o| o.1 == 0xB0).map(|o| o.2).collect();
        assert_eq!(cc.last(), Some(&99), "the NRPN select back: {cc:?}");
    }
}
