//! A part's envelope controllers (#346 step 4): CC73 attack, CC75 decay and CC72 release
//! (XG, GM2), played in yahaha on upstream rustysynth's primitives, so rustysynth never
//! sees them.
//!
//! Each controller scales the SoundFont's own volume-envelope time for the notes that
//! start after it, 16 steps an octave (64: the SoundFont's; 127: about 15x; 0: 1/16), as
//! the vendored rustysynth did. yahaha can't change a voice's envelope, so it shapes the
//! note's level from outside: with any of them set, each note plays on a MIDI channel of
//! its own (the note-per-channel allocation of voicing.rs), and its channel's expression
//! (CC11/43) carries a gain `m` (0..=1) so that the voice's own envelope times `m` follows
//! the envelope the controllers ask for. rustysynth's gain is (volume x expression)², so
//! the channel gets the player's expression x sqrt(m), and it ramps each change over a
//! synthesizer block; `m` is worked out once a block, from the SoundFont's envelope for the
//! note (read through rustysynth's public SoundFont API when the note starts) and the same
//! envelope with the scaled times.
//!
//! - **Slower attack** (CC73 above 64): the note fades in over the longer attack.
//! - **Shorter decay** (CC75 below 64): the note falls to its sustain level sooner.
//! - **Shorter release** (CC72 below 64): the note-off goes to the synthesizer at once and
//!   the note fades out faster than its own release. The SoundFont's 10 ms floor stays.
//! - **Longer release** (CC72 above 64): the note-off is held back; the note fades out
//!   over the longer release from where it stands, then its channel's sound is cut
//!   (All Sound Off) once it is inaudible.
//!
//! `m` never goes above 1, so a note is never louder than the player's CC7, CC11 and the
//! master volume make it; what would need more is lost (see the PR for #346 step 4): a
//! faster attack, a longer decay, a slower attack or longer release on a sound whose own
//! level falls first (it can only follow that level down), and each layer of a note but the
//! first taking its first layer's shaping. The player's CC11 still works: the note's channel
//! gets it, times sqrt(m). The hold pedal holds a shaped note's release until it comes up,
//! as it does the voice's own.
//!
//! A note is shaped only on a channel it has to itself for its whole life: the drum part
//! (MIDI channel 10, its only one) is never shaped, a note starting where another may still
//! sound (a tail, or past 15 notes at once) plays unshaped (the notes sounding on each
//! channel are counted here from the note-ons and note-offs, each until its longest release
//! has passed after its note-off and the pedal), and a note arriving on a shaped
//! note's channel ends that shaping: the channel goes back to the player's expression and
//! a held-back note-off goes at once. So All Sound Off only ever reaches a channel with its
//! shaped note alone on it. At 64 (or only in the directions that can't be played) nothing
//! is shaped and a note plays exactly as without the controllers. Nothing here allocates,
//! locks or panics.

use super::voicing::To;
use rustysynth::SoundFont;

/// The envelope controllers.
const RELEASE: i32 = 72;
const ATTACK: i32 = 73;
const DECAY: i32 = 75;
/// Steps a controller takes to double (halve) a time.
const STEPS_PER_OCTAVE: f32 = 16.0;
/// rustysynth's level below which a voice ends (`SoundFontMath::NON_AUDIBLE`), its log, and
/// the slope of its decay and release (log of the level per time constant).
const NON_AUDIBLE: f32 = 1e-3;
const LOG_NON_AUDIBLE: f32 = -6.907_755_4;
const SLOPE: f32 = -9.226;
/// rustysynth's shortest release (s).
const MIN_RELEASE: f32 = 0.01;
/// Expression at full (14-bit), as rustysynth resets it.
const FULL: i32 = 127 << 7;
/// How long a note whose envelope isn't known may sound on after its note-off (s): above
/// the longest release a SoundFont can give (8000 timecents, about 101.6 s).
const MAX_TAIL: f32 = 200.0;

/// A note's volume envelope, as rustysynth plays it: times in seconds, sustain linear.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Env {
    pub(super) delay: f32,
    pub(super) attack: f32,
    pub(super) hold: f32,
    pub(super) decay: f32,
    pub(super) sustain: f32,
    /// Before rustysynth's floor (`MIN_RELEASE`), so a scaled release keeps it too.
    pub(super) release: f32,
    /// The longest release of any of the note's layers (s): how long it may sound on
    /// after its note-off.
    pub(super) tail: f32,
}

/// `exp(x)`, 0 below rustysynth's cut-off (`SoundFontMath::exp_cutoff`).
#[inline]
fn exp_cut(x: f32) -> f32 {
    if x < LOG_NON_AUDIBLE { 0.0 } else { x.exp() }
}

impl Env {
    /// The envelope rustysynth gives `key` at `velocity` on preset `bank`:`program` of
    /// `font` (its first region pair that plays them), finding the preset as rustysynth's
    /// note-on does. None if nothing plays them.
    pub(super) fn of(font: &SoundFont, bank: i32, program: i32, key: i32, velocity: i32) -> Option<Env> {
        let presets = font.get_presets();
        let find = |b: i32, p: i32| presets.iter().position(|x| x.get_bank_number() == b && x.get_patch_number() == p);
        let fallback = if bank < 128 { (0, program) } else { (128, 0) };
        let i = find(bank, program).or_else(|| find(fallback.0, fallback.1)).or_else(|| {
            // The preset with the lowest bank and program, as rustysynth's default.
            (0..presets.len()).min_by_key(|&i| (presets[i].get_bank_number(), presets[i].get_patch_number()))
        })?;
        let instruments = font.get_instruments();
        let (mut first, mut tail) = (None, MIN_RELEASE);
        for p in presets[i].get_regions().iter().filter(|p| p.contains(key, velocity)) {
            let Some(inst) = instruments.get(p.get_instrument_id()) else { continue };
            for r in inst.get_regions().iter().filter(|r| r.contains(key, velocity)) {
                tail = tail.max(r.get_release_volume_envelope() * p.get_release_volume_envelope());
                if first.is_some() {
                    continue;
                }
                let by_key = |cents: i32| ((cents * (60 - key)) as f32 / 1200.0).exp2();
                first = Some(Env {
                    delay: r.get_delay_volume_envelope() * p.get_delay_volume_envelope(),
                    attack: r.get_attack_volume_envelope() * p.get_attack_volume_envelope(),
                    hold: r.get_hold_volume_envelope()
                        * p.get_hold_volume_envelope()
                        * by_key(r.get_key_number_to_volume_envelope_hold() + p.get_key_number_to_volume_envelope_hold()),
                    decay: r.get_decay_volume_envelope()
                        * p.get_decay_volume_envelope()
                        * by_key(r.get_key_number_to_volume_envelope_decay() + p.get_key_number_to_volume_envelope_decay()),
                    sustain: 10f32.powf(-0.05 * (r.get_sustain_volume_envelope() + p.get_sustain_volume_envelope())).clamp(0.0, 1.0),
                    release: r.get_release_volume_envelope() * p.get_release_volume_envelope(),
                    tail: 0.0,
                });
            }
        }
        first.map(|e| Env { tail: tail.max(MIN_RELEASE), ..e })
    }

    /// With the attack, decay and release times scaled by `f`.
    fn scaled(self, f: [f32; 3]) -> Env {
        Env { attack: self.attack * f[0], decay: self.decay * f[1], release: self.release * f[2], ..self }
    }

    /// The release time rustysynth plays.
    #[inline]
    fn release_s(&self) -> f32 {
        self.release.max(MIN_RELEASE)
    }

    /// The level `t` seconds into the note while it is held, and whether the voice still
    /// lives (rustysynth ends it once its decay falls below `NON_AUDIBLE`).
    fn held(&self, t: f32) -> (f32, bool) {
        let attack_end = self.delay + self.attack;
        let decay_start = attack_end + self.hold;
        if t < self.delay {
            (0.0, true)
        } else if t < attack_end {
            ((t - self.delay) / self.attack.max(1e-9), true)
        } else if t < decay_start {
            (1.0, true)
        } else {
            let v = exp_cut(SLOPE * (t - decay_start) / self.decay.max(1e-9)).max(self.sustain);
            (v, v > NON_AUDIBLE)
        }
    }
}

/// Where a shaped note is.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Stage {
    /// Its key is down.
    #[default]
    Held,
    /// Its key is up under the hold pedal; its note-off waits here.
    KeyUp,
    /// Released: the synthesizer has its note-off.
    Released,
    /// Released on a longer release than its own: its note-off is held back.
    Withheld,
}

/// A channel's shaped note.
#[derive(Clone, Copy, Debug, Default)]
struct Note {
    /// The lane it plays in, its key.
    lane: u8,
    key: u8,
    /// Frames into the note at its lane's next synthesizer block.
    t: u32,
    /// The SoundFont's envelope for it, and the one the controllers ask for.
    own: Env,
    to: Env,
    stage: Stage,
    /// When it was released (`t`), the voice's own level then, and `m` then.
    rel: u32,
    own_rel: f32,
    m_rel: f32,
    /// The gain on it now (0..=1).
    m: f32,
    /// The expression its channel has (14-bit), on its lane.
    sent: i32,
    /// Inaudible: its channel's sound is cut at the next block.
    closing: bool,
}

impl Note {
    /// The gain `t` seconds into the note, and whether it still sounds.
    fn gain(&self, t: f32, rel_s: f32) -> (f32, bool) {
        match self.stage {
            Stage::Held | Stage::KeyUp => {
                let ((own, alive), (to, to_alive)) = (self.own.held(t), self.to.held(t));
                let m = if own > 1e-9 {
                    to / own
                } else {
                    // Before the voice sounds: where the attacks start from.
                    self.own.attack / self.to.attack.max(1e-9)
                };
                // It ends as its voice would, or as the scaled envelope would have.
                (m.min(1.0), alive && to_alive)
            }
            Stage::Released => {
                // Both release from where they stood, the note's no slower than its own.
                let dt = t - rel_s;
                let (own_r, to_r) = (self.own.release_s(), self.to.release_s());
                let own = self.own_rel * exp_cut(SLOPE * dt / own_r);
                let m = self.m_rel * (SLOPE * dt * (1.0 / to_r - 1.0 / own_r).max(0.0)).exp();
                (m, own * m > NON_AUDIBLE)
            }
            Stage::Withheld => {
                // The voice holds on; the note fades from where it stood.
                let (own, alive) = self.own.held(t);
                let to = self.own_rel * self.m_rel * exp_cut(SLOPE * (t - rel_s) / self.to.release_s());
                let m = if own > 1e-9 { (to / own).min(1.0) } else { 0.0 };
                (m, alive && to > NON_AUDIBLE)
            }
        }
    }
}

/// A part's envelope controllers and its shaped notes (see the module docs).
pub(super) struct PartEnvelope {
    /// CC73, CC75, CC72 (7-bit).
    attack: u8,
    decay: u8,
    release: u8,
    rate: f32,
    /// The synthesizers' block, in frames.
    block: u32,
    /// The player's expression (CC11/43, 14-bit).
    expr: i32,
    /// The synthesizers' hold pedal.
    pedal: bool,
    /// Channels with a shaped note (bit = channel).
    shaping: u16,
    notes: [Note; 16],
    /// The notes sounding on each channel, in any of the part's lanes, shaped or not:
    /// the keys down, the keys up under the pedal, the longest tail among them (s), and
    /// the frame until which released notes may still sound. Erring long, never short.
    held: [u128; 16],
    kept: [u128; 16],
    tail: [f32; 16],
    quiet_at: [u64; 16],
    /// Frames rendered.
    now: u64,
}

impl PartEnvelope {
    pub(super) fn new(sample_rate: f32, block: usize) -> PartEnvelope {
        PartEnvelope {
            attack: 64,
            decay: 64,
            release: 64,
            rate: sample_rate,
            block: block as u32,
            expr: FULL,
            pedal: false,
            shaping: 0,
            notes: [Note::default(); 16],
            held: [0; 16],
            kept: [0; 16],
            tail: [0.0; 16],
            quiet_at: [0; 16],
            now: 0,
        }
    }

    /// `frames` rendered.
    #[inline]
    pub(super) fn tick(&mut self, frames: usize) {
        self.now = self.now.wrapping_add(frames as u64);
    }

    /// Whether a note may still sound on channel `c`.
    #[inline]
    fn busy(&self, c: usize) -> bool {
        self.held[c] | self.kept[c] != 0 || self.now < self.quiet_at[c]
    }

    /// The channels a note may still sound on.
    fn busy_mask(&self) -> u16 {
        (0..16).filter(|&c| self.busy(c)).fold(0, |m, c| m | 1 << c)
    }

    /// Channel `c`'s notes released now: they may sound for their longest tail.
    fn release_at(&mut self, c: usize) {
        let end = self.now + (self.tail[c] as f64 * self.rate as f64).ceil() as u64 + self.block as u64;
        self.quiet_at[c] = self.quiet_at[c].max(end);
    }

    /// Channel `c`'s sound cut: nothing sounds on it.
    fn cut(&mut self, c: usize) {
        (self.held[c], self.kept[c], self.quiet_at[c]) = (0, 0, self.now);
    }

    /// Follow a message to the part. True if it is an envelope controller (the part's
    /// synthesizers don't get it).
    #[inline]
    pub(super) fn follow(&mut self, st: i32, d1: i32, d2: i32) -> bool {
        if st != 0xB0 {
            return false;
        }
        let v = (d2 & 127) as u8;
        match d1 {
            ATTACK => self.attack = v,
            DECAY => self.decay = v,
            RELEASE => self.release = v,
            // Reset All Controllers leaves the sound controllers as they are (GM2, XG).
            _ => return false,
        }
        true
    }

    /// Whether new notes are shaped: a slower attack, a shorter decay or another release.
    #[inline]
    pub(super) fn on(&self) -> bool {
        self.attack > 64 || self.decay < 64 || self.release != 64
    }

    /// Each note on a channel of its own (voicing.rs), away from these channels if it can
    /// be, or None: notes on the part's own channel.
    #[inline]
    pub(super) fn spread(&self) -> Option<u16> {
        (self.on() || self.shaping != 0).then(|| self.shaping | self.busy_mask())
    }

    /// Whether channel `c` has a shaped note.
    #[inline]
    #[cfg(test)]
    pub(super) fn shapes(&self, c: u8) -> bool {
        self.shaping >> (c & 15) & 1 == 1
    }

    /// Whether a shaped note plays in lane `lane` (it renders a block at a time then).
    #[inline]
    pub(super) fn on_lane(&self, lane: usize) -> bool {
        let mut m = self.shaping;
        while m != 0 {
            let c = m.trailing_zeros() as usize;
            m &= m - 1;
            if self.notes[c].lane as usize == lane {
                return true;
            }
        }
        false
    }

    /// The time factors of the attack, decay and release.
    fn factors(&self) -> [f32; 3] {
        [self.attack, self.decay, self.release].map(|v| ((v as f32 - 64.0) / STEPS_PER_OCTAVE).exp2())
    }

    /// The expression for gain `m` on the player's.
    #[inline]
    fn expression_for(&self, m: f32) -> i32 {
        ((self.expr as f32 * m.max(0.0).sqrt()).round() as i32).clamp(0, 16383)
    }

    /// Channel `c`'s expression for its note's gain, if it has changed.
    fn send_gain(&mut self, c: usize, emit: &mut impl FnMut(To, u16, i32, i32, i32)) {
        let v = self.expression_for(self.notes[c].m);
        let n = &mut self.notes[c];
        if v != n.sent {
            n.sent = v;
            emit(To::Lane(n.lane), 1 << c, 0xB0, 11, v >> 7);
            emit(To::Lane(n.lane), 1 << c, 0xB0, 43, v & 127);
        }
    }

    /// Channel `c` unshaped: the player's expression back on its note's lane.
    fn unshape(&mut self, c: usize, emit: &mut impl FnMut(To, u16, i32, i32, i32)) {
        self.shaping &= !(1 << c);
        let n = &mut self.notes[c];
        if n.sent != self.expr {
            emit(To::Lane(n.lane), 1 << c, 0xB0, 11, self.expr >> 7);
            emit(To::Lane(n.lane), 1 << c, 0xB0, 43, self.expr & 127);
        }
    }

    /// Every note-on for `key` on the channels `chans` (lane `lane` plays it), before the
    /// synthesizers get it; `own`: the SoundFont's envelope for it (None: unknown, never
    /// shaped). It is counted as sounding on those channels. A note is shaped only on a
    /// channel it has to itself for its whole life, never on the drum channel (MIDI 10):
    /// where another note may still sound, it plays unshaped. A note shaped on the channel
    /// before ends its shaping now: an inaudible one is cut; a sounding one gets its
    /// held-back note-off at once and the player's expression back, so it is neither cut
    /// nor re-shaped.
    pub(super) fn note_on(&mut self, chans: u16, lane: u8, key: u8, own: Option<Env>, emit: &mut impl FnMut(To, u16, i32, i32, i32)) {
        let mut m = chans & self.shaping;
        while m != 0 {
            let c = m.trailing_zeros() as usize;
            m &= m - 1;
            let old = self.notes[c];
            if old.closing {
                // Inaudible, and alone on its channel: its pending cut goes now.
                emit(To::Lane(old.lane), 1 << c, 0xB0, 120, 0);
                self.cut(c);
            } else if matches!(old.stage, Stage::KeyUp | Stage::Withheld) {
                emit(To::All, 1 << c, 0x80, old.key as i32, 0);
                self.release_at(c);
            }
            self.unshape(c, emit);
        }
        let tail = own.map_or(MAX_TAIL, |e| e.tail);
        let mut busy = false;
        let mut m = chans;
        while m != 0 {
            let c = m.trailing_zeros() as usize;
            m &= m - 1;
            if self.busy(c) {
                busy = true;
            } else {
                self.tail[c] = 0.0;
            }
            self.tail[c] = self.tail[c].max(tail);
            self.held[c] |= 1 << (key & 127);
        }
        let c = chans.trailing_zeros() as usize & 15;
        if busy || chans.count_ones() != 1 || c == 9 {
            return;
        }
        let Some(own) = own.filter(|_| self.on()) else { return };
        let sent = self.expr;
        let mut n = Note { lane, key, own, to: own.scaled(self.factors()), sent, ..Note::default() };
        n.m = n.gain(self.block as f32 / self.rate, 0.0).0;
        self.notes[c] = n;
        self.shaping |= 1 << c;
        self.send_gain(c, emit);
    }

    /// A note-off for `key` on the channels `chans`, before the synthesizers get it. The
    /// channels it should still go to (a shaped note's is played here).
    pub(super) fn note_off(&mut self, chans: u16, key: i32, emit: &mut impl FnMut(To, u16, i32, i32, i32)) -> u16 {
        let bit = 1u128 << (key & 127);
        let mut m = chans;
        while m != 0 {
            let c = m.trailing_zeros() as usize;
            m &= m - 1;
            if self.held[c] & bit != 0 {
                self.held[c] &= !bit;
                if self.pedal {
                    self.kept[c] |= bit;
                } else {
                    self.release_at(c);
                }
            }
        }
        let mut rest = chans;
        let mut m = chans & self.shaping;
        while m != 0 {
            let c = m.trailing_zeros() as usize;
            m &= m - 1;
            if self.notes[c].key as i32 != key & 127 {
                // An older note on the channel: the synthesizer ends it.
                continue;
            }
            rest &= !(1 << c);
            if self.notes[c].stage == Stage::Held {
                if self.pedal {
                    self.notes[c].stage = Stage::KeyUp;
                } else {
                    self.release(c, emit);
                }
            }
        }
        rest
    }

    /// Channel `c`'s note released now: its note-off to the synthesizers, or held back for
    /// a release longer than its own.
    fn release(&mut self, c: usize, emit: &mut impl FnMut(To, u16, i32, i32, i32)) {
        let n = &mut self.notes[c];
        n.rel = n.t;
        n.own_rel = n.own.held(n.t as f32 / self.rate).0;
        n.m_rel = n.m;
        if n.to.release_s() > n.own.release_s() {
            n.stage = Stage::Withheld;
        } else {
            n.stage = Stage::Released;
            emit(To::All, 1 << c, 0x80, n.key as i32, 0);
        }
    }

    /// The synthesizers' hold pedal, after they have it: up, it releases the shaped notes
    /// whose keys are up.
    pub(super) fn pedal(&mut self, down: bool, emit: &mut impl FnMut(To, u16, i32, i32, i32)) {
        self.pedal = down;
        if down {
            return;
        }
        for c in 0..16 {
            if self.kept[c] != 0 {
                self.kept[c] = 0;
                self.release_at(c);
            }
        }
        let mut m = self.shaping;
        while m != 0 {
            let c = m.trailing_zeros() as usize;
            m &= m - 1;
            if self.notes[c].stage == Stage::KeyUp {
                self.release(c, emit);
            }
        }
    }

    /// The player's CC11 or CC43 to the channels `chans`: as it is to the unshaped ones,
    /// times the note's gain on a shaped one.
    pub(super) fn expression(&mut self, chans: u16, d1: i32, d2: i32, emit: &mut impl FnMut(To, u16, i32, i32, i32)) {
        let v = d2 & 127;
        self.expr = if d1 == 11 { (self.expr & 0x7F) | (v << 7) } else { (self.expr & !0x7F) | v };
        let plain = chans & !self.shaping;
        if plain != 0 {
            emit(To::All, plain, 0xB0, d1, d2);
        }
        let mut m = chans & self.shaping;
        while m != 0 {
            let c = m.trailing_zeros() as usize;
            m &= m - 1;
            // The note's other lanes (tails of another font) take the player's.
            emit(To::AllBut(self.notes[c].lane), 1 << c, 0xB0, d1, d2);
            self.send_gain(c, emit);
        }
    }

    /// After the synthesizers have had Reset All Controllers: expression at full and the
    /// pedal up, the shaped notes' gains again.
    pub(super) fn reset(&mut self, emit: &mut impl FnMut(To, u16, i32, i32, i32)) {
        self.expr = FULL;
        let mut m = self.shaping;
        while m != 0 {
            let c = m.trailing_zeros() as usize;
            m &= m - 1;
            self.notes[c].sent = FULL;
            self.send_gain(c, emit);
        }
        self.pedal(false, emit);
    }

    /// After the synthesizers have had All Sound Off on `chans`: nothing left to shape.
    pub(super) fn sound_off(&mut self, chans: u16, emit: &mut impl FnMut(To, u16, i32, i32, i32)) {
        let mut m = chans;
        while m != 0 {
            let c = m.trailing_zeros() as usize;
            m &= m - 1;
            self.cut(c);
        }
        let mut m = chans & self.shaping;
        while m != 0 {
            let c = m.trailing_zeros() as usize;
            m &= m - 1;
            self.unshape(c, emit);
        }
    }

    /// After the synthesizers have had All Notes Off on `chans`: the shaped notes release
    /// on their own release (or a shorter one).
    pub(super) fn notes_off(&mut self, chans: u16) {
        let mut m = chans;
        while m != 0 {
            let c = m.trailing_zeros() as usize;
            m &= m - 1;
            if self.pedal {
                self.kept[c] |= self.held[c];
            } else if self.held[c] != 0 {
                self.release_at(c);
            }
            self.held[c] = 0;
        }
        let mut m = chans & self.shaping;
        while m != 0 {
            let c = m.trailing_zeros() as usize;
            m &= m - 1;
            let n = &mut self.notes[c];
            if n.stage != Stage::Released {
                // The voice releases from where it stands now, at the gain it has.
                n.rel = n.t;
                n.own_rel = n.own.held(n.t as f32 / self.rate).0;
                n.m_rel = n.m;
                n.stage = Stage::Released;
            }
        }
    }

    /// Before lane `lane` renders a block: each shaped note on it a block on
    /// (`emit(channel, status, data1, data2)` to that lane's synthesizer). A note gone
    /// inaudible fades to 0 over the block, then its channel's sound is cut and it has the
    /// player's expression again.
    pub(super) fn block(&mut self, lane: usize, emit: &mut impl FnMut(i32, i32, i32, i32)) {
        let mut m = self.shaping;
        while m != 0 {
            let c = m.trailing_zeros() as usize;
            m &= m - 1;
            if self.notes[c].lane as usize != lane {
                continue;
            }
            if self.notes[c].closing {
                emit(c as i32, 0xB0, 120, 0);
                // Its note was alone on the channel (any other ends the shaping).
                self.cut(c);
                self.unshape(c, &mut |_, _, st, d1, d2| emit(c as i32, st, d1, d2));
                continue;
            }
            let n = &self.notes[c];
            let (g, alive) = n.gain((n.t + self.block) as f32 / self.rate, n.rel as f32 / self.rate);
            let n = &mut self.notes[c];
            n.m = if alive { g } else { 0.0 };
            n.closing = !alive;
            n.t = n.t.saturating_add(self.block);
            self.send_gain(c, &mut |_, _, st, d1, d2| emit(c as i32, st, d1, d2));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48_000.0;
    const BLOCK: usize = 64;

    type Out = (To, u16, i32, i32, i32);

    /// A slow pad: 0.1 s attack, 0.5 s decay to -20 dB, 0.3 s release.
    const PAD: Env = Env { delay: 0.0, attack: 0.1, hold: 0.0, decay: 0.5, sustain: 0.1, release: 0.3, tail: 0.3 };
    /// Strings: 0.1 s attack, held at full.
    const STRINGS: Env = Env { sustain: 1.0, ..PAD };

    fn part(setup: &[[i32; 2]]) -> PartEnvelope {
        let mut e = PartEnvelope::new(RATE, BLOCK);
        for &[cc, v] in setup {
            assert!(e.follow(0xB0, cc, v), "CC{cc} is taken");
        }
        e
    }

    /// The expression a message list leaves on channel `c` (14-bit), if it sets one.
    fn expr_of(out: &[(i32, i32, i32, i32)], c: i32) -> Option<i32> {
        let mut v = None;
        for &(ch, st, d1, d2) in out {
            if ch == c && st == 0xB0 {
                if d1 == 11 {
                    v = Some((v.unwrap_or(FULL) & 0x7F) | (d2 << 7));
                } else if d1 == 43 {
                    v = Some((v.unwrap_or(FULL) & !0x7F) | d2);
                }
            }
        }
        v
    }

    /// Run lane 0's blocks for `secs`: each block's gain on channel `c` (as sqrt of the
    /// expression, squared: the level factor), and every message.
    fn run(e: &mut PartEnvelope, c: i32, secs: f32) -> (Vec<f32>, Vec<(i32, i32, i32, i32)>) {
        let mut gains = Vec::new();
        let mut all = Vec::new();
        let mut last = e.notes[c as usize].sent;
        for _ in 0..(secs * RATE / BLOCK as f32) as usize {
            let mut out = Vec::new();
            e.block(0, &mut |ch, st, a, b| out.push((ch, st, a, b)));
            if let Some(v) = expr_of(&out, c) {
                last = v;
            }
            gains.push((last as f32 / FULL as f32).powi(2));
            all.extend(out);
        }
        (gains, all)
    }

    fn start(e: &mut PartEnvelope, c: u8, env: Env) -> Vec<Out> {
        let mut out = Vec::new();
        e.note_on(1 << c, 0, 60, Some(env), &mut |to, ch, st, a, b| out.push((to, ch, st, a, b)));
        out
    }

    fn off(e: &mut PartEnvelope, chans: u16) -> (u16, Vec<Out>) {
        let mut out = Vec::new();
        let rest = e.note_off(chans, 60, &mut |to, ch, st, a, b| out.push((to, ch, st, a, b)));
        (rest, out)
    }

    /// At 64, and in the directions that can't be played (a faster attack, a longer
    /// decay), nothing is shaped: no channel of its own, nothing sent.
    #[test]
    fn neutral_shapes_nothing() {
        for setup in [&[][..], &[[73, 64], [75, 64], [72, 64]], &[[73, 0], [75, 127]]] {
            let mut e = part(setup);
            assert!(!e.on() && e.spread().is_none(), "{setup:?}");
            assert_eq!(start(&mut e, 0, PAD), [], "{setup:?}");
            assert_eq!(off(&mut e, 1), (1, vec![]), "the note-off goes on: {setup:?}");
        }
        assert!(!PartEnvelope::new(RATE, BLOCK).follow(0xB0, 74, 0), "not an envelope controller");
    }

    /// CC73 127: the note fades in over about 15x its attack, then plays at full.
    #[test]
    fn a_slower_attack_fades_in() {
        let mut e = part(&[[73, 127]]);
        assert_eq!(e.spread(), Some(0));
        let out = start(&mut e, 3, STRINGS);
        let f = (63.0f32 / 16.0).exp2();
        assert!(out.iter().all(|o| o.0 == To::Lane(0) && o.1 == 1 << 3), "{out:?}");
        let (g, _) = run(&mut e, 3, 2.0);
        // Through the note's own attack its level is 1/f of its own; then it climbs to
        // full at f x 0.1 s.
        assert!((g[10] - 1.0 / f).abs() < 0.01, "{}", g[10]);
        let at = |s: f32| g[(s * RATE / BLOCK as f32) as usize];
        assert!(at(0.5) < at(1.0) && at(1.0) < 1.0 && (at(0.75) / (0.75 / (f * 0.1)) - 1.0).abs() < 0.3, "{} {}", at(0.5), at(1.0));
        assert!(g[g.len() - 1] > 0.999, "full after the attack");
        assert!(g.iter().all(|&x| x <= 1.0), "never above the player's");
    }

    /// CC75 0: the note falls to its sustain level 16x sooner.
    #[test]
    fn a_shorter_decay_falls_sooner() {
        let mut e = part(&[[75, 0]]);
        start(&mut e, 0, PAD);
        let (g, _) = run(&mut e, 0, 1.0);
        let at = |s: f32| g[(s * RATE / BLOCK as f32) as usize];
        // At 0.2 s the own envelope is at exp(-9.226 x 0.2) = 0.16; the scaled one is at
        // its sustain (0.1).
        assert!((at(0.2) - 0.1 / (-9.226f32 * 0.1 / 0.5).exp()).abs() < 0.02, "{}", at(0.2));
        assert!(at(0.9) > 0.99, "both at the sustain level");
    }

    /// CC72 0: the note-off goes to the synthesizer at once, and the note fades 16x
    /// faster than its own release; once inaudible, its channel's sound is cut.
    #[test]
    fn a_shorter_release_fades_faster() {
        let mut e = part(&[[72, 0]]);
        start(&mut e, 2, PAD);
        run(&mut e, 2, 1.0);
        let (rest, out) = off(&mut e, 1 << 2);
        assert_eq!((rest, out), (0, vec![(To::All, 1 << 2, 0x80, 60, 0)]));
        let (g, msgs) = run(&mut e, 2, 0.5);
        // The own release: -9.226 a 0.3 s; the note's: a 0.3 / 16 s (above the 10 ms
        // floor). Four blocks in:
        let dt = 4.0 * BLOCK as f32 / RATE;
        let want = (-9.226f32 * dt * (1.0 / 0.01875 - 1.0 / 0.3)).exp();
        assert!((g[3] / want - 1.0).abs() < 0.02, "{} vs {want}", g[3]);
        assert!(msgs.contains(&(2, 0xB0, 120, 0)), "the channel's sound cut once inaudible");
        assert!(!e.shapes(2) && e.spread().is_some(), "done; new notes still spread");
        assert_eq!(expr_of(&msgs, 2), Some(FULL), "the player's expression back");
    }

    /// CC72 127: the note-off is held back while the note fades over 15x its release, then
    /// the channel's sound is cut (no note-off: the voice held on).
    #[test]
    fn a_longer_release_holds_the_note_off() {
        let mut e = part(&[[72, 127]]);
        start(&mut e, 1, PAD);
        run(&mut e, 1, 1.0);
        assert_eq!(off(&mut e, 1 << 1), (0, vec![]), "held back");
        let (g, msgs) = run(&mut e, 1, 1.0);
        let f = (63.0f32 / 16.0).exp2();
        let want = (-9.226f32 * 0.5 / (0.3 * f)).exp();
        let at = (0.5 * RATE / BLOCK as f32) as usize - 1;
        assert!((g[at] / want - 1.0).abs() < 0.05, "{} vs {want}", g[at]);
        assert!(!msgs.iter().any(|m| m.1 == 0x80), "no note-off yet");
        let (_, msgs) = run(&mut e, 1, 10.0);
        assert!(msgs.contains(&(1, 0xB0, 120, 0)) && !e.shapes(1), "cut once inaudible");
    }

    /// The hold pedal holds a shaped note's release until it comes up.
    #[test]
    fn the_pedal_holds_the_release() {
        let mut e = part(&[[72, 0]]);
        start(&mut e, 0, PAD);
        let mut out = Vec::new();
        e.pedal(true, &mut |to, ch, st, a, b| out.push((to, ch, st, a, b)));
        assert_eq!(off(&mut e, 1), (0, vec![]), "held by the pedal");
        let (g, _) = run(&mut e, 0, 0.5);
        assert!(g[g.len() - 1] > 0.99, "sounding on");
        e.pedal(false, &mut |to, ch, st, a, b| out.push((to, ch, st, a, b)));
        assert_eq!(out, [(To::All, 1, 0x80, 60, 0)], "the pedal up releases it");
    }

    /// The player's CC11 still works on a shaped note: its channel gets it times the
    /// note's gain; its other lanes and the unshaped channels get it as it is.
    #[test]
    fn the_players_expression_combines() {
        let mut e = part(&[[73, 127]]);
        start(&mut e, 4, STRINGS);
        run(&mut e, 4, 0.5);
        let m = e.notes[4].m;
        let mut out = Vec::new();
        e.expression(0b1_0001, 11, 64, &mut |to, ch, st, a, b| out.push((to, ch, st, a, b)));
        assert_eq!(out[0], (To::All, 1, 0xB0, 11, 64), "unshaped channel 0");
        assert_eq!(out[1], (To::AllBut(0), 1 << 4, 0xB0, 11, 64), "the note's other lanes");
        assert_eq!((out[2].0, out[2].3, out[3].0, out[3].3), (To::Lane(0), 11, To::Lane(0), 43), "{out:?}");
        let v = (out[2].4 << 7) | out[3].4;
        assert_eq!(v, ((64 << 7) as f32 * m.sqrt()).round() as i32, "the player's times sqrt(m)");
        // Reset All Controllers: full again, times the gain.
        out.clear();
        e.reset(&mut |to, ch, st, a, b| out.push((to, ch, st, a, b)));
        assert_eq!((out[0].4 << 7) | out[1].4, (FULL as f32 * m.sqrt()).round() as i32);
    }

    /// An overflow note on a shaped note's channel: the shaped note's held-back note-off goes
    /// at once, the channel gets the player's expression back, and the new note plays
    /// unshaped; nothing is ever cut. A note starting where another sounds isn't shaped.
    /// Once the old note is inaudible (and alone), a new note takes the channel over.
    #[test]
    fn an_overflow_note_ends_the_shaping() {
        let mut e = part(&[[72, 127]]);
        start(&mut e, 2, PAD);
        run(&mut e, 2, 0.2);
        off(&mut e, 1 << 2);
        run(&mut e, 2, 0.3);
        let mut out = Vec::new();
        e.note_on(1 << 2, 0, 62, Some(PAD), &mut |to, ch, st, a, b| out.push((to, ch, st, a, b)));
        assert_eq!(out[0], (To::All, 1 << 2, 0x80, 60, 0), "{out:?}");
        let rest: Vec<_> = out[1..].iter().map(|o| (2, o.2, o.3, o.4)).collect();
        assert_eq!(expr_of(&rest, 2).unwrap_or(e.notes[2].sent), FULL, "the player's expression back: {out:?}");
        assert!(!e.shapes(2) && !out.iter().any(|o| o.3 == 120), "unshaped, nothing cut");
        // A third note where the others still sound: not shaped either.
        e.note_on(1 << 2, 0, 64, Some(PAD), &mut |_, _, _, _, _| {});
        assert!(!e.shapes(2));
        let (_, msgs) = run(&mut e, 2, 10.0);
        assert!(msgs.is_empty(), "no CC120, no gain: {msgs:?}");
        assert_eq!(e.note_off(1 << 2, 62, &mut |_, _, _, _, _| {}), 1 << 2, "a normal note-off");
        assert_ne!(e.spread().unwrap() & 1 << 2, 0, "still sounding: new notes kept away");
        // Inaudible and alone, then taken over: its cut goes first, the new note is shaped.
        let mut e = part(&[[72, 0]]);
        start(&mut e, 2, PAD);
        off(&mut e, 1 << 2);
        while !e.notes[2].closing {
            run(&mut e, 2, 0.002);
        }
        let out = start(&mut e, 2, PAD);
        assert_eq!(out[0], (To::Lane(0), 1 << 2, 0xB0, 120, 0), "{out:?}");
        assert!(e.shapes(2) && !e.notes[2].closing);
        e.sound_off(0xFFFF, &mut |_, _, _, _, _| {});
        assert!(!e.shapes(2) && e.spread() == Some(0));
    }

    /// Three overlapping drum hits on channel 10 with CC72/73/75 set: the drum part is
    /// never shaped, so each note-off goes at once and no hit is ever cut.
    #[test]
    fn overlapping_drum_hits_are_not_shaped() {
        let mut e = part(&[[72, 127], [73, 127], [75, 0]]);
        let mut out = Vec::new();
        for key in [36, 38, 42] {
            e.note_on(1 << 9, 0, key, Some(PAD), &mut |to, ch, st, a, b| out.push((to, ch, st, a, b)));
            assert!(!e.shapes(9), "drums: not shaped");
        }
        assert!(out.is_empty(), "{out:?}");
        let (_, msgs) = run(&mut e, 9, 0.1);
        for key in [38, 42] {
            assert_eq!(e.note_off(1 << 9, key, &mut |_, _, _, _, _| {}), 1 << 9, "note-off at once");
        }
        let (_, more) = run(&mut e, 9, 10.0);
        assert!(msgs.is_empty() && more.is_empty(), "the first hit is never cut");
        assert_eq!(e.note_off(1 << 9, 36, &mut |_, _, _, _, _| {}), 1 << 9);
    }

    /// A note is counted as sounding from its note-on until its longest release has passed
    /// after its note-off (the pedal holding it): a note starting on its channel before then
    /// isn't shaped; after, it is. A note of unknown envelope counts for the longest tail.
    #[test]
    fn a_tail_keeps_the_channel_shared() {
        let mut e = part(&[[72, 127]]);
        let secs = |s: f32| (s * RATE) as usize;
        let none = &mut |_, _, _, _, _| {};
        // An unshaped note sounding (CC72 was set after it, say): held, then released.
        e.follow(0xB0, 72, 64);
        e.note_on(1 << 3, 0, 50, Some(PAD), none);
        e.follow(0xB0, 72, 127);
        e.pedal(true, none);
        e.note_off(1 << 3, 50, none);
        e.tick(secs(5.0));
        start(&mut e, 3, PAD);
        assert!(!e.shapes(3), "held by the pedal");
        e.note_off(1 << 3, 60, none);
        e.pedal(false, none);
        e.tick(secs(0.29));
        start(&mut e, 3, PAD);
        assert!(!e.shapes(3), "within its release");
        e.note_off(1 << 3, 60, none);
        e.tick(secs(0.32));
        start(&mut e, 3, PAD);
        assert!(e.shapes(3), "its release over: shaped");
        // Unknown envelope (a drum setup's note): the longest tail.
        e.sound_off(0xFFFF, none);
        e.note_on(1 << 4, 0, 50, None, none);
        e.note_off(1 << 4, 50, none);
        e.tick(secs(150.0));
        start(&mut e, 4, PAD);
        assert!(!e.shapes(4));
    }

    /// The envelope rustysynth plays, read from the SoundFont through its public API.
    #[test]
    fn the_soundfonts_envelope() {
        let font = SoundFont::new(&mut &crate::patches::sf2::tiny_sound_font_with(&[(0, 0, "Pad")], &PAD_GENS)[..]).unwrap();
        let env = Env::of(&font, 0, 0, 60, 100).unwrap();
        assert!((env.attack - 0.1).abs() < 1e-3 && (env.decay - 0.5).abs() < 1e-3, "{env:?}");
        assert!((env.sustain - 0.1).abs() < 1e-3 && (env.release - 0.3).abs() < 1e-3, "{env:?}");
        assert_eq!(Env::of(&font, 5, 0, 60, 100), Some(env), "bank 5 falls back to bank 0");
    }

    /// `PAD` as SoundFont instrument generators.
    const PAD_GENS: [(u16, i16); 4] = [(34, -3986), (36, -1200), (37, 200), (38, -2084)];
}
