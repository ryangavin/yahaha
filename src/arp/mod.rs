//! Arpeggio engine (#33): turns the held right-hand notes into a tempo-synced pattern.
//!
//! Pure and deterministic, like the arranger engine: no threads, no wall time. Callers
//! pass ticks on the style clock (`ppq` ticks per quarter note) and pull events with
//! [`Arp::process`] one tick range at a time. Nothing allocates after [`Arp::new`] and
//! [`Arp::set_pattern`]: all state lives in fixed arrays.
//!
//! Note-offs always balance note-ons. Every note the arp starts is tracked per pitch
//! until its off is sent, a retriggered pitch is released before it sounds again, and
//! pattern changes, [`Arp::stop`] and [`Arp::all_off`] cut whatever is sounding.
//!
//! See docs/arpeggio.md for the pattern format and the settings.

pub mod library;
pub mod pattern;
#[cfg(test)]
mod tests;

pub use pattern::{len, Category, Motion, Pattern, Sel, Sort, Step, PATTERN_PPQ};

use std::ops::Range;

/// Where [`Arp::process`] sends its notes.
pub trait ArpSink {
    fn note_on(&mut self, tick: u64, note: u8, vel: u8);
    fn note_off(&mut self, tick: u64, note: u8);
}

/// A note event, for sinks that collect them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArpEvent {
    On { tick: u64, note: u8, vel: u8 },
    Off { tick: u64, note: u8 },
}

impl ArpEvent {
    pub fn tick(&self) -> u64 {
        match *self {
            ArpEvent::On { tick, .. } | ArpEvent::Off { tick, .. } => tick,
        }
    }
}

impl ArpSink for Vec<ArpEvent> {
    fn note_on(&mut self, tick: u64, note: u8, vel: u8) {
        self.push(ArpEvent::On { tick, note, vel });
    }
    fn note_off(&mut self, tick: u64, note: u8) {
        self.push(ArpEvent::Off { tick, note });
    }
}

/// Arpeggio Quantize (RM p.41): the grid a pattern starts on, so it lines up with Style
/// or Song playback. A key pressed a little late or early starts the pattern on the
/// nearest grid line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Quantize {
    /// Start the moment the first key goes down.
    #[default]
    Off,
    Eighth,
    Sixteenth,
}

impl Quantize {
    /// The grid in ticks at `ppq`, or `None` when off.
    pub fn grid(self, ppq: u32) -> Option<u64> {
        match self {
            Quantize::Off => None,
            Quantize::Eighth => Some(ppq as u64 / 2),
            Quantize::Sixteenth => Some(ppq as u64 / 4),
        }
    }
}

/// Where each arp note's velocity comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Velocity {
    /// The pattern's own step velocities (its accents).
    #[default]
    Original,
    /// The velocity the source key was played with.
    Thru,
    /// Every note at this velocity.
    Fixed(u8),
}

/// Arpeggio settings. The three percentages are the Genos Live Control values ArpVel,
/// ArpGateT and ArpUnitM (RM p.147): a percent of the pattern's own value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub quantize: Quantize,
    /// Arpeggio Hold (latch): the pattern keeps playing after the keys are released. A
    /// new chord played after a full release replaces the latched one.
    pub hold: bool,
    pub velocity: Velocity,
    /// Velocity scale, percent (ArpVel). Clamped to 0..=200.
    pub vel_scale: u16,
    /// Gate time scale, percent (ArpGateT). Clamped to 1..=400.
    pub gate_scale: u16,
    /// Step length scale, percent (ArpUnitM): 200 plays at half speed, 50 at double.
    /// Clamped to 25..=400.
    pub unit_multiply: u16,
    /// Keep Key On: the pattern clock keeps running through a full release, so the next
    /// chord picks up where the phrase is instead of restarting it from step 1.
    pub keep_key_on: bool,
    /// The sustain pedal holds released notes in the arpeggio until it comes up.
    pub sustain_holds: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            quantize: Quantize::Off,
            hold: false,
            velocity: Velocity::Original,
            vel_scale: 100,
            gate_scale: 100,
            unit_multiply: 100,
            keep_key_on: false,
            sustain_holds: false,
        }
    }
}

/// The most notes the arp holds at once (further keys are ignored).
pub const MAX_KEYS: usize = 16;
/// The most note-ons waiting to start (strummed notes after their step).
const MAX_PENDING: usize = 64;
const NONE: u64 = u64::MAX;

#[derive(Debug, Clone, Copy, Default)]
struct Key {
    note: u8,
    vel: u8,
    /// Physically down (as opposed to latched or pedalled).
    held: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct Pend {
    tick: u64,
    off: u64,
    note: u8,
    vel: u8,
}

/// Denominator of the step length: `step_len * ppq * unit_multiply / STEP_DEN` ticks.
const STEP_DEN: u64 = PATTERN_PPQ as u64 * 100;

pub struct Arp {
    ppq: u32,
    pattern: Pattern,
    settings: Settings,
    /// Notes the pattern plays from, in the order they were pressed.
    pool: [Key; MAX_KEYS],
    npool: usize,
    /// Indices into `pool`, lowest pitch first.
    by_pitch: [u8; MAX_KEYS],
    pedal: bool,
    running: bool,
    /// Step `anchor_step` falls at `anchor` (before swing).
    anchor: u64,
    anchor_step: u64,
    next_step: u64,
    walk: u32,
    last_pick: u32,
    rng: u32,
    /// Off tick per sounding pitch, `NONE` when silent.
    off_at: [u64; 128],
    nsounding: usize,
    pend: [Pend; MAX_PENDING],
    npend: usize,
    /// End of the last processed range, `NONE` before the first.
    last_end: u64,
}

impl Arp {
    pub fn new(ppq: u32, pattern: Pattern) -> Arp {
        assert!(ppq > 0, "ppq must be positive");
        Arp {
            ppq,
            rng: seed(pattern.seed),
            pattern,
            settings: Settings::default(),
            pool: [Key::default(); MAX_KEYS],
            npool: 0,
            by_pitch: [0; MAX_KEYS],
            pedal: false,
            running: false,
            anchor: 0,
            anchor_step: 0,
            next_step: 0,
            walk: 0,
            last_pick: u32::MAX,
            off_at: [NONE; 128],
            nsounding: 0,
            pend: [Pend::default(); MAX_PENDING],
            npend: 0,
            last_end: NONE,
        }
    }

    pub fn pattern(&self) -> &Pattern {
        &self.pattern
    }
    pub fn settings(&self) -> &Settings {
        &self.settings
    }
    /// Whether the pattern clock is running (notes held, latched or kept on).
    pub fn is_running(&self) -> bool {
        self.running
    }
    /// How many arp notes are sounding (started and not yet released).
    pub fn sounding(&self) -> usize {
        self.nsounding
    }
    /// The notes the pattern is playing from, in the order they were pressed.
    pub fn notes(&self) -> impl Iterator<Item = u8> + '_ {
        self.pool[..self.npool].iter().map(|k| k.note)
    }

    /// A key goes down at `tick`. Velocity 0 is a note-off.
    pub fn note_on(&mut self, note: u8, vel: u8, tick: u64) {
        if vel == 0 {
            return self.note_off(note, tick);
        }
        let note = note.min(127);
        let any_held = self.pool[..self.npool].iter().any(|k| k.held);
        // Hold: the first key after a full release starts a new chord.
        let mut restart = false;
        if self.settings.hold && !any_held && self.npool > 0 {
            self.npool = 0;
            restart = !self.settings.keep_key_on;
        }
        if let Some(k) = self.pool[..self.npool].iter_mut().find(|k| k.note == note) {
            k.vel = vel;
            k.held = true;
        } else if self.npool < MAX_KEYS {
            self.pool[self.npool] = Key { note, vel, held: true };
            self.npool += 1;
        } else {
            return;
        }
        self.sort();
        if !self.running || restart {
            self.start(tick);
        }
    }

    /// A key comes up at `tick`.
    pub fn note_off(&mut self, note: u8, _tick: u64) {
        let Some(i) = self.pool[..self.npool].iter().position(|k| k.note == note) else { return };
        self.pool[i].held = false;
        if !self.sustaining() {
            self.remove(i);
            if self.npool == 0 {
                self.emptied();
            }
        }
    }

    /// The sustain pedal goes down or up. It only affects the arp when
    /// [`Settings::sustain_holds`] is on.
    pub fn set_sustain(&mut self, down: bool, _tick: u64) {
        self.pedal = down;
        if !down {
            self.drop_released();
        }
    }

    /// Changes the settings at `tick`. A new Unit Multiply takes effect from the next
    /// step; turning Hold (or Keep Key On) off releases what only it was keeping.
    pub fn set_settings(&mut self, s: Settings, _tick: u64) {
        let s = Settings {
            vel_scale: s.vel_scale.min(200),
            gate_scale: s.gate_scale.clamp(1, 400),
            unit_multiply: s.unit_multiply.clamp(25, 400),
            velocity: match s.velocity {
                Velocity::Fixed(v) => Velocity::Fixed(v.clamp(1, 127)),
                v => v,
            },
            ..s
        };
        if s.unit_multiply != self.settings.unit_multiply && self.running {
            // Rebase so the next step keeps its time and the ones after it take the new length.
            self.anchor = self.base(self.next_step);
            self.anchor_step = self.next_step;
        }
        self.settings = s;
        self.drop_released();
        if self.npool == 0 {
            self.emptied();
        }
    }

    /// Arpeggio Hold on or off (the pedal function).
    pub fn set_hold(&mut self, on: bool, tick: u64) {
        let s = Settings { hold: on, ..self.settings };
        self.set_settings(s, tick);
    }

    /// Switches pattern at `tick`. Sounding notes are cut there, and a running arp starts
    /// the new pattern from its first step: at `tick`, or on the next grid line when
    /// Quantize is on.
    pub fn set_pattern(&mut self, pattern: Pattern, tick: u64) {
        self.cut(tick);
        self.npend = 0;
        self.pattern = pattern;
        self.reset_walk();
        if self.running {
            self.anchor = match self.settings.quantize.grid(self.ppq) {
                Some(g) => tick.div_ceil(g) * g,
                None => tick,
            };
            self.anchor_step = 0;
            self.next_step = 0;
        }
    }

    /// Stops the arp at `tick` (the HARMONY/ARPEGGIO button going off): forgets the held
    /// and latched notes and cuts whatever is sounding. The offs go out on the next
    /// [`Arp::process`].
    pub fn stop(&mut self, tick: u64) {
        self.running = false;
        self.npool = 0;
        self.npend = 0;
        self.cut(tick);
    }

    /// [`Arp::stop`], sending the note-offs to `sink` now.
    pub fn all_off(&mut self, tick: u64, sink: &mut impl ArpSink) {
        self.stop(tick);
        for n in 0..128u8 {
            if self.off_at[n as usize] != NONE {
                self.off_at[n as usize] = NONE;
                sink.note_off(tick, n);
            }
        }
        self.nsounding = 0;
    }

    /// The notes whose key is physically down (not the latched or pedalled ones), in the
    /// order pressed. The live wiring checks them against the keys really held, so a
    /// lost key-up can never leave a note in the pattern.
    pub fn keys_down(&self) -> impl Iterator<Item = u8> + '_ {
        self.pool[..self.npool].iter().filter(|k| k.held).map(|k| k.note)
    }

    /// The pattern's tick resolution.
    pub fn ppq(&self) -> u32 {
        self.ppq
    }

    /// The earliest tick something is due (a note-off, a queued note-on or the next
    /// step), if anything is. The engine wakes for it.
    pub fn next_due(&self) -> Option<u64> {
        let step = if self.running { self.step_tick(self.next_step) } else { NONE };
        let t = self.next_off().0.min(self.next_pending().0).min(step);
        (t != NONE).then_some(t)
    }

    /// The clock jumped to `tick` (the style restarted at 0, another style took over): as
    /// a `process` range starting before the last one ended, but at once, for a caller
    /// that knows. Every sounding note is cut at `tick` (the offs go out on the next
    /// `process`), queued note-ons are dropped, and a running pattern starts again from
    /// step 1 there (on the next grid line with Quantize on). The next range starts at
    /// `tick`.
    pub fn jump(&mut self, tick: u64) {
        self.clock_reset(tick);
        self.last_end = tick;
    }

    /// A new clock resolution at `tick` (a style with another PPQ took over): every
    /// sounding note is cut now through `sink` (its off tick would mean something else
    /// on the new clock), queued note-ons are dropped, and a running pattern starts
    /// again from its first step at `tick` (the next grid line with Quantize on). The
    /// held and latched notes stay. A ppq of 0 is ignored.
    pub fn set_ppq(&mut self, ppq: u32, tick: u64, sink: &mut impl ArpSink) {
        if ppq == 0 || ppq == self.ppq {
            return;
        }
        for n in 0..128u8 {
            if self.off_at[n as usize] != NONE {
                self.off_at[n as usize] = NONE;
                sink.note_off(tick, n);
            }
        }
        self.nsounding = 0;
        self.npend = 0;
        self.ppq = ppq;
        self.last_end = NONE;
        if self.running {
            self.anchor = match self.settings.quantize.grid(self.ppq) {
                Some(g) => tick.div_ceil(g) * g,
                None => tick,
            };
            self.anchor_step = 0;
            self.next_step = 0;
            self.reset_walk();
        }
    }

    /// Sends every event in `range` to `sink`, in time order. Events due before
    /// `range.start` (a late start, or a cut at an earlier tick) go out at
    /// `range.start`. Call it with contiguous ranges.
    ///
    /// The clock may jump. A range that starts before the previous one ended (the style
    /// restarting at tick 0) cuts every sounding note at `range.start` and, if the
    /// pattern is running, starts it again from step 1 there (on the next grid line with
    /// Quantize on). A range that starts after the previous one ended skips the steps
    /// in the gap: only the latest one plays, at `range.start`. An empty range does
    /// nothing.
    pub fn process(&mut self, range: Range<u64>, sink: &mut impl ArpSink) {
        let (from, to) = (range.start, range.end);
        if from >= to {
            return;
        }
        if self.last_end != NONE && from < self.last_end {
            self.clock_reset(from);
        }
        self.last_end = to;
        loop {
            let (off_t, off_n) = self.next_off();
            let (pend_t, pend_i) = self.next_pending();
            let step_t = if self.running { self.step_tick(self.next_step) } else { NONE };
            let t = off_t.min(pend_t).min(step_t);
            if t >= to {
                break;
            }
            let at = t.max(from);
            if off_t == t {
                self.off_at[off_n as usize] = NONE;
                self.nsounding -= 1;
                sink.note_off(at, off_n);
            } else if pend_t == t {
                let p = self.pend[pend_i];
                // Keep queue order, so notes due together start in the order queued.
                self.pend.copy_within(pend_i + 1..self.npend, pend_i);
                self.npend -= 1;
                self.start_note(p, at, sink);
            } else {
                if t < from {
                    self.skip_late_steps(from);
                }
                let t = self.step_tick(self.next_step);
                self.play_step(t);
            }
        }
    }

    /// The clock went back to `tick`: cut what is sounding, drop queued notes and start
    /// a running pattern over.
    fn clock_reset(&mut self, tick: u64) {
        for t in self.off_at.iter_mut() {
            if *t != NONE {
                *t = tick;
            }
        }
        self.npend = 0;
        if self.running {
            self.anchor = match self.settings.quantize.grid(self.ppq) {
                Some(g) => tick.div_ceil(g) * g,
                None => tick,
            };
            self.anchor_step = 0;
            self.next_step = 0;
            self.reset_walk();
        }
    }

    /// Moves `next_step` to the latest step due at or before `from`, so a late start
    /// or a gap in the clock plays one step instead of a burst of every missed one.
    fn skip_late_steps(&mut self, from: u64) {
        let num = self.step_num();
        let base = self.anchor_step + (from - self.anchor) * STEP_DEN / num;
        let mut k = base.max(self.next_step);
        while k > self.next_step && self.step_tick(k) > from {
            k -= 1;
        }
        while self.step_tick(k + 1) <= from {
            k += 1;
        }
        self.next_step = k;
    }

    // --- held notes ---------------------------------------------------------------

    fn sustaining(&self) -> bool {
        self.settings.hold || (self.settings.sustain_holds && self.pedal)
    }

    fn remove(&mut self, i: usize) {
        self.pool.copy_within(i + 1..self.npool, i);
        self.npool -= 1;
        self.sort();
    }

    /// Drops the released notes nothing keeps any more.
    fn drop_released(&mut self) {
        if self.sustaining() {
            return;
        }
        let before = self.npool;
        let mut j = 0;
        for i in 0..self.npool {
            if self.pool[i].held {
                self.pool[j] = self.pool[i];
                j += 1;
            }
        }
        self.npool = j;
        if j != before {
            self.sort();
            if j == 0 {
                self.emptied();
            }
        }
    }

    /// The last note left. Without Keep Key On the pattern stops (sounding notes finish
    /// their gate); with it the clock runs on silently.
    fn emptied(&mut self) {
        if !self.settings.keep_key_on {
            self.running = false;
            self.npend = 0;
        }
    }

    fn sort(&mut self) {
        let n = self.npool;
        for i in 0..n {
            self.by_pitch[i] = i as u8;
        }
        for i in 1..n {
            let mut j = i;
            while j > 0 && self.pool[self.by_pitch[j - 1] as usize].note > self.pool[self.by_pitch[j] as usize].note {
                self.by_pitch.swap(j - 1, j);
                j -= 1;
            }
        }
    }

    /// The i-th held note in `sort` order.
    fn nth(&self, sort: Sort, i: usize) -> Key {
        match sort {
            Sort::Pitch => self.pool[self.by_pitch[i] as usize],
            Sort::Played => self.pool[i],
        }
    }

    // --- timing -------------------------------------------------------------------

    fn start(&mut self, tick: u64) {
        self.running = true;
        self.npend = 0;
        self.anchor = match self.settings.quantize.grid(self.ppq) {
            // The nearest grid line: a slightly late key still starts on the beat.
            Some(g) => (tick + g / 2) / g * g,
            None => tick,
        };
        self.anchor_step = 0;
        self.next_step = 0;
        self.reset_walk();
    }

    fn reset_walk(&mut self) {
        self.walk = 0;
        self.last_pick = u32::MAX;
        self.rng = seed(self.pattern.seed);
    }

    /// Step length numerator (over `STEP_DEN`).
    /// A zero step length (an unvalidated pattern) counts as one tick at
    /// [`PATTERN_PPQ`], so the clock always moves forward.
    fn step_num(&self) -> u64 {
        self.pattern.step_len.max(1) as u64 * self.ppq as u64 * self.settings.unit_multiply as u64
    }

    /// Unswung time of step `k`.
    fn base(&self, k: u64) -> u64 {
        self.anchor + (k - self.anchor_step) * self.step_num() / STEP_DEN
    }

    fn step_tick(&self, k: u64) -> u64 {
        let swing = self.pattern.swing.clamp(50, 75) as u64 - 50;
        let delay = if k % 2 == 1 { self.step_num() * 2 * swing / (STEP_DEN * 100) } else { 0 };
        self.base(k) + delay
    }

    fn next_off(&self) -> (u64, u8) {
        if self.nsounding == 0 {
            return (NONE, 0);
        }
        let mut best = (NONE, 0);
        for (n, &t) in self.off_at.iter().enumerate() {
            if t < best.0 {
                best = (t, n as u8);
            }
        }
        best
    }

    fn next_pending(&self) -> (u64, usize) {
        let mut best = (NONE, 0);
        for (i, p) in self.pend[..self.npend].iter().enumerate() {
            if p.tick < best.0 {
                best = (p.tick, i);
            }
        }
        best
    }

    /// Cuts every sounding note at `tick` (sent on the next `process`).
    fn cut(&mut self, tick: u64) {
        for t in self.off_at.iter_mut() {
            if *t != NONE && *t > tick {
                *t = tick;
            }
        }
    }

    // --- playback -----------------------------------------------------------------

    fn start_note(&mut self, p: Pend, at: u64, sink: &mut impl ArpSink) {
        let n = p.note as usize;
        if self.off_at[n] != NONE {
            sink.note_off(at, p.note);
        } else {
            self.nsounding += 1;
        }
        sink.note_on(at, p.note, p.vel);
        self.off_at[n] = p.off.max(at + 1);
    }

    fn play_step(&mut self, t: u64) {
        let k = self.next_step;
        self.next_step += 1;
        if self.npool == 0 {
            return;
        }
        let steps = self.pattern.steps.len();
        if steps == 0 {
            // An unvalidated pattern with no steps plays nothing.
            return;
        }
        let step = self.pattern.steps[(k % steps as u64) as usize];
        let gate = (self.step_num() * step.gate as u64 * self.settings.gate_scale as u64 / (STEP_DEN * 10_000)).max(1);
        let n = self.npool;
        let sort = self.pattern.sort;
        match step.sel {
            Sel::Rest => {}
            Sel::Walk => {
                let (key, oct) = self.walk_next();
                self.queue(key, oct + step.oct as i32, &step, t, gate);
            }
            Sel::Idx(i) => {
                let i = i as usize;
                let key = self.nth(sort, i % n);
                self.queue(key, (i / n) as i32 + step.oct as i32, &step, t, gate);
            }
            Sel::Top(i) => {
                let i = i as usize;
                let key = self.nth(sort, n - 1 - i % n);
                self.queue(key, step.oct as i32 - (i / n) as i32, &step, t, gate);
            }
            Sel::All => {
                for j in 0..n {
                    let key = self.nth(Sort::Pitch, j);
                    self.queue(key, step.oct as i32, &step, t, gate);
                }
            }
            Sel::Strum { up, spread } => {
                let spread = spread as u64 * self.ppq as u64 / PATTERN_PPQ as u64;
                for j in 0..n {
                    let key = self.nth(Sort::Pitch, if up { n - 1 - j } else { j });
                    let at = t + j as u64 * spread;
                    self.queue(key, step.oct as i32, &step, at, gate);
                }
            }
        }
    }

    /// The next note of the pattern's walk, and its octave shift.
    fn walk_next(&mut self) -> (Key, i32) {
        let n = self.npool as u32;
        let l = n * self.pattern.octaves.clamp(1, 4) as u32;
        let pos = self.walk;
        self.walk = self.walk.wrapping_add(1);
        let updown = |pos: u32| {
            let period = if l > 1 { 2 * l - 2 } else { 1 };
            let q = pos % period;
            if q < l { q } else { period - q }
        };
        let e = match self.pattern.motion {
            Motion::Up | Motion::AsPlayed => pos % l,
            Motion::Down => l - 1 - pos % l,
            Motion::UpDown => updown(pos),
            Motion::DownUp => l - 1 - updown(pos),
            Motion::Random => {
                if l == 1 {
                    0
                } else {
                    self.rng = xorshift(self.rng);
                    let mut r = self.rng % l;
                    if r == self.last_pick {
                        r = (r + 1) % l;
                    }
                    r
                }
            }
        };
        self.last_pick = e;
        let sort = if self.pattern.motion == Motion::AsPlayed { Sort::Played } else { Sort::Pitch };
        (self.nth(sort, (e % n) as usize), (e / n) as i32)
    }

    fn queue(&mut self, key: Key, oct: i32, step: &Step, tick: u64, gate: u64) {
        let note = key.note as i32 + 12 * oct;
        if !(0..=127).contains(&note) || self.npend == MAX_PENDING {
            return;
        }
        let vel = match self.settings.velocity {
            Velocity::Original => step.vel,
            Velocity::Thru => key.vel,
            Velocity::Fixed(v) => v,
        };
        let vel = (vel as u32 * self.settings.vel_scale as u32 / 100).clamp(1, 127) as u8;
        self.pend[self.npend] = Pend { tick, off: tick + gate, note: note as u8, vel };
        self.npend += 1;
    }
}

fn seed(s: u32) -> u32 {
    if s == 0 { 0x6D2B_79F5 } else { s }
}

fn xorshift(mut x: u32) -> u32 {
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    x
}
