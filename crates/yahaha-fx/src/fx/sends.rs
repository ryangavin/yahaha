//! Sends 4-6 (the mixer rework): the send effects the player adds, after the three buses.
//!
//! The control side ([`SendControl`], one per slot in `FxControl::sends`) holds each slot's
//! kind, parameters and return level as atomics; each channel's send to it is
//! `FxControl::strip_send`. The audio side ([`SendFx`], one per slot in `FxBus`) has one
//! of every engine (a [`Reverb`], a [`Chorus`], a [`Delay`], a [`Phaser`]), built in
//! `FxBus::new`, so a kind change never allocates.
//!
//! A kind change while the slot sounds fades its return out over one buffer, then plays
//! the new kind from a return of 0, ramping up over the next buffer, so it doesn't click.
//! A reverb or phaser that still rings when it comes back into use starts again from
//! silence; an engine that is left keeps running on silence, unheard, until its tail has
//! died away, so it is clean the next time it plays.
//!
//! A slot with no kind (or one this build doesn't know) costs nothing: its input isn't
//! even read. A slot with no input whose output has died away is skipped.

use std::sync::atomic::{AtomicU8, AtomicU16, Ordering::Relaxed};

use super::{BUSES, Chorus, ChorusType, Delay, Param, Phaser, RETURN_UNITY, Reverb, ReverbType, SEND_PARAMS, SENDS, SendKind, SendSlot, delay};

/// The sends the player adds (4-6), after the [`BUSES`].
pub const ADDED_SENDS: usize = SENDS - BUSES;

/// A kind as a slot's code: 0 = none (or unknown), else its index in [`SendKind::ALL`] + 1.
pub fn kind_code(k: &SendKind) -> u8 {
    SendKind::ALL.iter().position(|a| a == k).map_or(0, |i| i as u8 + 1)
}

/// A code's kind (None: no kind, or a code this build doesn't know).
pub fn kind_of_code(c: u8) -> Option<SendKind> {
    (c as usize).checked_sub(1).and_then(|i| SendKind::ALL.get(i).cloned())
}

/// One added send's settings on the control side, read by the audio thread once per
/// buffer.
pub struct SendControl {
    /// [`kind_code`]: 0 = no kind.
    pub kind: AtomicU8,
    /// Its parameters, in [`SendKind::params`] order.
    pub params: [AtomicU16; SEND_PARAMS],
    /// 0-127, 64 = 0 dB (`super::return_gain`).
    pub return_level: AtomicU8,
}

impl SendControl {
    /// An empty slot.
    pub fn new() -> SendControl {
        SendControl { kind: AtomicU8::new(0), params: std::array::from_fn(|_| AtomicU16::new(0)), return_level: AtomicU8::new(RETURN_UNITY) }
    }

    /// Play `s` (an unknown kind: nothing). The kind goes last, after its parameters.
    pub fn set(&self, s: &SendSlot) {
        for (a, v) in self.params.iter().zip(s.params) {
            a.store(v, Relaxed);
        }
        self.return_level.store(s.return_level, Relaxed);
        self.kind.store(kind_code(&s.kind), Relaxed);
    }

    /// Empty the slot.
    pub fn clear(&self) {
        self.kind.store(0, Relaxed);
    }

    /// What it plays (None: nothing). Control side: it allocates.
    pub fn slot(&self) -> Option<SendSlot> {
        let kind = kind_of_code(self.kind.load(Relaxed))?;
        Some(SendSlot { kind, params: std::array::from_fn(|i| self.params[i].load(Relaxed)), return_level: self.return_level.load(Relaxed) })
    }
}

impl Default for SendControl {
    fn default() -> Self {
        SendControl::new()
    }
}

/// Which engine plays a kind, and its type there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Engine {
    Reverb(ReverbType),
    Chorus(ChorusType),
    Delay,
    Phaser,
}

impl Engine {
    /// By [`kind_code`] (None: no kind, or unknown). A fixed table, in
    /// [`SendKind::ALL`] order: nothing allocates.
    fn of_code(c: u8) -> Option<Engine> {
        Some(match c {
            1..=4 => Engine::Reverb(ReverbType::from_u8(c - 1)),
            5..=7 => Engine::Chorus(ChorusType::from_u8(c - 5)),
            8..=11 => Engine::Delay,
            12 => Engine::Phaser,
            _ => return None,
        })
    }

    /// Its index in `SendFx::tails`.
    fn index(self) -> usize {
        match self {
            Engine::Reverb(_) => 0,
            Engine::Chorus(_) => 1,
            Engine::Delay => 2,
            Engine::Phaser => 3,
        }
    }
}

/// The reverb's and chorus's parameters, in [`Param::of_block`] order (the delay's are
/// `super::DELAY_PARAMS`).
const REVERB_PARAMS: [Param; 3] = [Param::ReverbTime, Param::PreDelay, Param::ReverbTone];
const CHORUS_PARAMS: [Param; 2] = [Param::ChorusRate, Param::ChorusDepth];
/// The phaser's ranges (the `kinds` PHASER specs): depth, rate (0.01 Hz), feedback (%).
const PHASER_RANGE: [(u16, u16); 3] = [(0, 127), (5, 500), (0, 90)];

/// One engine's idle tracking (as `super::Block`'s).
struct Tail {
    idle: bool,
    quiet: u32,
    hold: u32,
}

impl Tail {
    fn new(hold: f32, rate: f32) -> Tail {
        Tail { idle: true, quiet: 0, hold: (hold * rate) as u32 }
    }

    fn update(&mut self, input: bool, peak: f32, n: usize) {
        self.quiet = if input || peak >= super::IDLE_LEVEL { 0 } else { self.quiet.saturating_add(n as u32) };
        self.idle = self.quiet > self.hold;
    }
}

/// One added send on the audio thread: every engine, preallocated.
pub(super) struct SendFx {
    reverb: Reverb,
    chorus: Chorus,
    delay: Delay,
    phaser: Phaser,
    /// By `Engine::index`.
    tails: [Tail; 4],
    /// The kind playing ([`kind_code`], 0 = none).
    kind: u8,
    gain: f32,
    /// The previous kind faded out last buffer: ramp this one in from 0.
    fade_in: bool,
}

impl SendFx {
    /// Allocates: off the audio thread.
    pub(super) fn new(rate: f32) -> SendFx {
        SendFx {
            reverb: Reverb::new(rate),
            chorus: Chorus::new(rate),
            delay: Delay::new(rate),
            phaser: Phaser::new(rate),
            tails: [Tail::new(0.1, rate), Tail::new(0.05, rate), Tail::new(delay::MAX_SECONDS + 0.1, rate), Tail::new(0.05, rate)],
            kind: 0,
            gain: 0.0,
            fade_in: false,
        }
    }

    #[inline]
    fn tick(&mut self, e: Engine, l: f32, r: f32) -> (f32, f32) {
        match e {
            Engine::Reverb(_) => self.reverb.tick(l, r),
            Engine::Chorus(_) => self.chorus.tick(l, r),
            Engine::Delay => self.delay.tick(l, r),
            Engine::Phaser => self.phaser.tick(l, r),
        }
    }

    /// Play `code` from now on, from a return of 0. An engine coming back into use while
    /// it still rings starts again from silence where it can (the reverb, the phaser).
    fn switch(&mut self, code: u8) {
        let (old, new) = (Engine::of_code(self.kind), Engine::of_code(code));
        self.kind = code;
        self.gain = 0.0;
        let Some(new) = new else { return };
        if old.map(Engine::index) == Some(new.index()) || self.tails[new.index()].idle {
            // The same engine (it changes type itself), or a silent one.
            return;
        }
        match new {
            // A type change clears the tank; one of the two is a change.
            Engine::Reverb(_) => {
                self.reverb.set_type(ReverbType::Hall);
                self.reverb.set_type(ReverbType::Room);
            }
            Engine::Phaser => self.phaser.reset(),
            // No reset: its tail, unheard since it was left, carries on under the fade-in.
            Engine::Chorus(_) | Engine::Delay => {}
        }
    }

    /// Run on `n` frames of its send (`il`, `ir`) and **add** its return into `left` /
    /// `right`. RT-safe.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn process_add(&mut self, il: &[f32], ir: &[f32], n: usize, left: &mut [f32], right: &mut [f32], ctl: &SendControl, bpm: f32) {
        let code = ctl.kind.load(Relaxed);
        let code = if Engine::of_code(code).is_some() { code } else { 0 };
        let active = Engine::of_code(self.kind);
        // Engines left behind run on silence, unheard, until their tails die away.
        for i in 0..self.tails.len() {
            if active.map(Engine::index) != Some(i) && !self.tails[i].idle {
                let e = match i {
                    0 => Engine::Reverb(ReverbType::Hall),
                    1 => Engine::Chorus(ChorusType::Chorus),
                    2 => Engine::Delay,
                    _ => Engine::Phaser,
                };
                let mut peak = 0f32;
                for _ in 0..n {
                    let (wl, wr) = self.tick(e, 0.0, 0.0);
                    peak = peak.max(wl.abs()).max(wr.abs());
                }
                self.tails[i].update(false, peak, n);
            }
        }
        // A kind change: fade the old one out over this buffer if it sounds, else at once.
        let mut fading = false;
        if code != self.kind {
            match active {
                Some(e) if !self.tails[e.index()].idle => fading = true,
                _ => self.switch(code),
            }
        }
        let Some(e) = Engine::of_code(self.kind) else { return };
        let param = |i: usize, p: Param| p.clamp(ctl.params[i].load(Relaxed));
        match e {
            Engine::Reverb(t) => {
                let [a, b, c] = std::array::from_fn(|i| param(i, REVERB_PARAMS[i]) as f32);
                self.reverb.set(t, a / 10.0, b, c * 100.0);
            }
            Engine::Chorus(t) => {
                let [a, b] = std::array::from_fn(|i| param(i, CHORUS_PARAMS[i]) as f32);
                self.chorus.set(t, a / 100.0, b / 10.0);
            }
            Engine::Delay => self.delay.set(delay::Settings::from_params(std::array::from_fn(|i| param(i, super::DELAY_PARAMS[i]))), bpm),
            Engine::Phaser => {
                let [a, b, c] = std::array::from_fn(|i| ctl.params[i].load(Relaxed).clamp(PHASER_RANGE[i].0, PHASER_RANGE[i].1));
                self.phaser.set(a, b, c);
            }
        }
        let n = n.min(il.len()).min(ir.len()).min(left.len()).min(right.len());
        let input = il[..n].iter().chain(&ir[..n]).any(|x| *x != 0.0);
        let target = if fading { 0.0 } else { super::return_gain(ctl.return_level.load(Relaxed)) };
        let t = e.index();
        if self.tails[t].idle {
            // Waking up: no ramp from a stale gain, unless it follows a kind that just
            // faded out (a phaser, dry at once, would jump).
            if !self.fade_in {
                self.gain = target;
            }
            if !input {
                self.gain = target;
                self.fade_in = false;
                return;
            }
        }
        self.fade_in = false;
        let (g0, dg) = (self.gain, (target - self.gain) / n.max(1) as f32);
        self.gain = target;
        let mut peak = 0f32;
        for k in 0..n {
            let (wl, wr) = self.tick(e, il[k], ir[k]);
            peak = peak.max(wl.abs()).max(wr.abs());
            let g = g0 + dg * (k + 1) as f32;
            left[k] += wl * g;
            right[k] += wr * g;
        }
        self.tails[t].update(input, peak, n);
        if fading {
            self.switch(code);
            self.fade_in = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fixed table matches the kinds: each code's engine plays the bus type
    /// [`SendKind::bus`] names, and the phaser's ranges are its specs'.
    #[test]
    fn sends_codes_match_the_kinds() {
        for (i, k) in SendKind::ALL.iter().enumerate() {
            let c = i as u8 + 1;
            assert_eq!(kind_code(k), c);
            assert_eq!(kind_of_code(c).as_ref(), Some(k));
            let want = match k.bus() {
                Some((super::super::REVERB, t)) => Engine::Reverb(ReverbType::from_u8(t)),
                Some((super::super::CHORUS, t)) => Engine::Chorus(ChorusType::from_u8(t)),
                Some(_) => Engine::Delay,
                None => Engine::Phaser,
            };
            assert_eq!(Engine::of_code(c), Some(want), "{k:?}");
            if let Some((b, _)) = k.bus() {
                let order: Vec<Param> = Param::of_block(b).collect();
                let table: &[Param] = match want {
                    Engine::Reverb(_) => &REVERB_PARAMS,
                    Engine::Chorus(_) => &CHORUS_PARAMS,
                    _ => &super::super::DELAY_PARAMS,
                };
                assert_eq!(order, table, "{k:?}");
            }
        }
        let specs = SendKind::Phaser.params();
        assert_eq!(specs.iter().map(|s| (s.min, s.max)).collect::<Vec<_>>(), PHASER_RANGE);
        assert_eq!(kind_code(&SendKind::Unknown("shimmer".into())), 0);
        assert_eq!(Engine::of_code(0), None);
        assert_eq!(Engine::of_code(13), None);
        assert_eq!(kind_of_code(13), None);
    }

    /// A slot round-trips through its control; an unknown kind reads as none.
    #[test]
    fn sends_control_round_trips_a_slot() {
        let c = SendControl::new();
        assert_eq!(c.slot(), None);
        let mut s = SendSlot::of(SendKind::Plate);
        s.set_param(0, 40).unwrap();
        s.return_level = 90;
        c.set(&s);
        assert_eq!(c.slot(), Some(s.clone()));
        let p = SendSlot::of(SendKind::Phaser);
        c.set(&p);
        assert_eq!(c.slot(), Some(p));
        c.clear();
        assert_eq!(c.slot(), None);
        c.set(&SendSlot { kind: SendKind::Unknown("shimmer".into()), params: [1; SEND_PARAMS], return_level: 70 });
        assert_eq!(c.slot(), None, "an unknown kind plays nothing");
    }
}
