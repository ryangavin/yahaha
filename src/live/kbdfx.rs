//! Keyboard Harmony / Arpeggio (#32, #33) on the real-time threads: the settings word both
//! threads read, the key messages from the input thread, and the engine-thread driver that
//! plays what happens later than the key press.
//!
//! The pieces and where they run:
//!
//! - [`FxConfig`]: the HARMONY/ARPEGGIO switch, the type and the detail settings, packed
//!   into one `u64` ([`Shared::kbd_fx`]). The session's control side writes it; the input
//!   thread reads it on each key, the engine thread on each wake. One atomic word, so a
//!   change can never be lost or half-seen, and neither side waits.
//! - The input thread's processor stage (`pipeline.rs`, [`Processor`]): the Harmony
//!   category (Duet, Trio, Block, ...) sounds with the melody key right there, and Multi
//!   Assign routes each key to one Right part. For the Echo category and the Arpeggio it
//!   swallows the right-hand keys and sends them here as [`FxKey`]s (an SPSC ring), with
//!   the keys it holds in [`Shared::fx_held`]. Strum's late notes come here too.
//! - [`KbdFx`] (engine thread, `EngineLoop::step`): `harmony::EchoGen` on engine
//!   nanoseconds, `arp::Arp` on style ticks, and the Strum queue. `EngineLoop::next_deadline`
//!   wakes the engine for whatever of them is due.
//!
//! # Time domains
//!
//! - **Echo, Tremolo, Trill** count in engine nanoseconds from the key press (the key is
//!   applied at the engine's `now` when it drains the ring; the input thread signals the
//!   engine at once, so that is well under a millisecond later). A tempo change reaches
//!   the generator on the next wake (`EchoGen::set_tempo`).
//! - **Arpeggio** counts in ticks of the style clock (`Engine::style_tick`), so with the
//!   band running it is in phase with the accompaniment and Quantize snaps to the style's
//!   grid. The clock restarts at 0 on every START (the arp's clock-reset rule cuts and
//!   restarts the pattern there) and re-times at a style change (a style with another PPQ:
//!   `Arp::set_ppq` at that point). With the band stopped the engine's clock runs on from
//!   its last anchor at the current tempo: that is the arp's own free clock, and it goes
//!   on from where the band stopped, so an arpeggio that was playing keeps its phase.
//!   The arp counts in eighths of a style tick ([`SUB`]) and plays what is due before the
//!   current one: at most 1/8 tick late, never early.
//! - **Strum** notes are 15 ms apart: the input thread sounds the melody and the first
//!   note; the later ones start here at `now + delay` and end with the melody key.
//!
//! # No stuck notes
//!
//! Every note this driver starts is counted per (channel, note) (`Voices`), and a note-off
//! goes out when the count falls back to 0, so two generator keys landing on one pitch
//! never cut each other short. The count is this thread's only: a note-off from the
//! input thread (a plain key, a left-hand key on a Right part) can end a note sounding
//! here on the same channel and pitch, and the other way round (see
//! `live::pipeline`'s module docs). Nothing sticks either way.
//!
//! Counting means a pitch struck again while it sounds (a second holder, or the sustain
//! pedal holding the first) sends a second note-on and, when the last holder lets go, one
//! note-off: the convention `live::Keys` already has. A synth whose note-off releases every
//! voice on that note (the built-in synth, GM/XG modules) is fine; one that stacks a voice
//! per note-on and releases one per note-off would keep the earlier voice until its own
//! release or the sustain pedal comes up. Where each generator key sounded (channels, transposed
//! pitches) is remembered, so its note-off goes there whatever changed since. A key-up
//! lost to a full ring cannot leave a key in a generator: every wake compares the keys
//! the generators hold with [`Shared::fx_held`] and releases the ones no longer down.
//! Switching the type or turning the effect off stops the old generator (`Arp::all_off`,
//! `EchoGen::all_off`); PANIC and shutdown stop everything.
//!
//! Nothing here allocates: the generators are fixed-size, built on the control side
//! (`channels`) and rebuilt in place (library patterns are borrowed, so `set_pattern`
//! frees nothing). `tests/engine_no_alloc.rs` covers it.

use super::*;
use crate::arp::{self, library::PATTERNS, Arp, ArpSink, Quantize, Velocity};
use crate::harmony::{self, Assign, Category, EchoEvent, EchoGen, EchoSpeed, HarmonySettings, HarmonyType, PartMask, RightParts, ALL_TYPES};

/// No ACMP switch in yahaha: the chord section is always read, so the Harmony chord is
/// the Style's (spec §6, ACMP on). `harmony::harmony_chord` keeps the other rows for when
/// there is one.
pub const ACMP: bool = true;

/// HARMONY/ARPEGGIO type category: one switch, one type, as on the Genos.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FxMode {
    /// A Keyboard Harmony type (the Harmony and Echo categories, Multi Assign).
    #[default]
    Harmony,
    /// An arpeggio pattern.
    Arpeggio,
}

/// The Harmony/Arpeggio settings: the switch, the type and the detail settings. `Copy`,
/// and packed into one `u64` for the real-time threads ([`FxConfig::pack`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FxConfig {
    /// The HARMONY/ARPEGGIO switch.
    pub on: bool,
    pub mode: FxMode,
    /// Harmony type and details. Volume and Assign also apply to the arpeggio (RM p.46:
    /// the settings marked * in spec §6).
    pub harmony: HarmonySettings,
    /// Arpeggio pattern: an index into `arp::library::PATTERNS`.
    pub pattern: u8,
    pub quantize: Quantize,
    pub hold: bool,
    pub velocity: Velocity,
    pub keep_key_on: bool,
}

impl Default for FxConfig {
    fn default() -> FxConfig {
        FxConfig {
            on: false,
            mode: FxMode::Harmony,
            harmony: HarmonySettings::default(),
            pattern: 0,
            quantize: Quantize::Off,
            hold: false,
            velocity: Velocity::Original,
            keep_key_on: false,
        }
    }
}

pub const ASSIGNS: [Assign; 5] = [Assign::Auto, Assign::Multi, Assign::Right1, Assign::Right2, Assign::Right3];

/// A Harmony type's place in the Data List order (`harmony::ALL_TYPES`).
pub fn type_index(t: HarmonyType) -> u8 {
    ALL_TYPES.iter().position(|&x| x == t).unwrap_or(0) as u8
}

fn index_of<T: PartialEq + Copy>(all: &[T], v: T) -> u64 {
    all.iter().position(|&x| x == v).unwrap_or(0) as u64
}

// Bit layout of the packed word.
const ON: u32 = 0;
const MODE: u32 = 1;
const TYPE: u32 = 2; // 5 bits
const PATTERN: u32 = 7; // 5 bits
const VOLUME: u32 = 12; // 7 bits
const SPEED: u32 = 19; // 3 bits
const ASSIGN: u32 = 22; // 3 bits
const CHORD_NOTE_ONLY: u32 = 25;
const MIN_VEL: u32 = 26; // 7 bits
const QUANTIZE: u32 = 33; // 2 bits
const HOLD: u32 = 35;
const VEL_MODE: u32 = 36; // 2 bits
const FIXED_VEL: u32 = 38; // 7 bits
const KEEP_KEY_ON: u32 = 45;
/// Set in every packed word, so 0 (never written) reads as "not set yet".
const VALID: u32 = 63;
// TYPE and PATTERN are 5-bit fields: a longer list would wrap round in the word.
const _: () = assert!(ALL_TYPES.len() <= 32 && PATTERNS.len() <= 32, "widen TYPE/PATTERN in FxConfig's word");

impl FxConfig {
    /// The word for [`Shared::kbd_fx`].
    pub fn pack(&self) -> u64 {
        let h = &self.harmony;
        let (vmode, fixed) = match self.velocity {
            Velocity::Original => (0u64, 100u64),
            Velocity::Thru => (1, 100),
            Velocity::Fixed(v) => (2, v.clamp(1, 127) as u64),
        };
        let quantize = match self.quantize {
            Quantize::Off => 0u64,
            Quantize::Eighth => 1,
            Quantize::Sixteenth => 2,
        };
        (self.on as u64) << ON
            | ((self.mode == FxMode::Arpeggio) as u64) << MODE
            | (type_index(h.ty) as u64 & 31) << TYPE
            | (self.pattern as u64 & 31) << PATTERN
            | (h.volume.min(127) as u64) << VOLUME
            | index_of(&EchoSpeed::ALL, h.speed) << SPEED
            | index_of(&ASSIGNS, h.assign) << ASSIGN
            | (h.chord_note_only as u64) << CHORD_NOTE_ONLY
            | (h.min_velocity.clamp(1, 127) as u64) << MIN_VEL
            | quantize << QUANTIZE
            | (self.hold as u64) << HOLD
            | vmode << VEL_MODE
            | fixed << FIXED_VEL
            | (self.keep_key_on as u64) << KEEP_KEY_ON
            | 1 << VALID
    }

    /// The settings in a packed word (the default for 0, never written).
    pub fn unpack(w: u64) -> FxConfig {
        if w >> VALID & 1 == 0 {
            return FxConfig::default();
        }
        let f = |at: u32, bits: u32| (w >> at & ((1 << bits) - 1)) as usize;
        let ty = ALL_TYPES[f(TYPE, 5).min(ALL_TYPES.len() - 1)];
        let fixed = f(FIXED_VEL, 7).clamp(1, 127) as u8;
        FxConfig {
            on: f(ON, 1) == 1,
            mode: if f(MODE, 1) == 1 { FxMode::Arpeggio } else { FxMode::Harmony },
            harmony: HarmonySettings {
                ty,
                volume: f(VOLUME, 7) as u8,
                speed: EchoSpeed::ALL[f(SPEED, 3).min(EchoSpeed::ALL.len() - 1)],
                assign: ASSIGNS[f(ASSIGN, 3).min(ASSIGNS.len() - 1)],
                chord_note_only: f(CHORD_NOTE_ONLY, 1) == 1,
                min_velocity: (f(MIN_VEL, 7) as u8).max(1),
            },
            pattern: (f(PATTERN, 5).min(PATTERNS.len().saturating_sub(1))) as u8,
            quantize: match f(QUANTIZE, 2) {
                1 => Quantize::Eighth,
                2 => Quantize::Sixteenth,
                _ => Quantize::Off,
            },
            hold: f(HOLD, 1) == 1,
            velocity: match f(VEL_MODE, 2) {
                1 => Velocity::Thru,
                2 => Velocity::Fixed(fixed),
                _ => Velocity::Original,
            },
            keep_key_on: f(KEEP_KEY_ON, 1) == 1,
        }
    }

    /// What the input thread's processor slot does with these settings.
    pub fn processor(&self) -> Processor {
        if !self.on {
            return Processor::Off;
        }
        match self.mode {
            FxMode::Arpeggio => Processor::Arp,
            FxMode::Harmony => match self.harmony.ty {
                HarmonyType::MultiAssign => Processor::MultiAssign,
                t if t.category() == Category::Echo => Processor::Echo,
                _ => Processor::Harmony(self.harmony),
            },
        }
    }

    /// The generator the engine thread runs.
    fn generator(&self) -> Gen {
        match self.processor() {
            Processor::Arp => Gen::Arp,
            Processor::Echo => Gen::Echo(self.harmony.ty),
            _ => Gen::Off,
        }
    }

    /// The arpeggio's settings (the Live Control percentages stay at 100).
    pub fn arp_settings(&self) -> arp::Settings {
        arp::Settings {
            quantize: self.quantize,
            hold: self.hold,
            velocity: self.velocity,
            keep_key_on: self.keep_key_on,
            ..arp::Settings::default()
        }
    }
}

/// Which generator the engine thread runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Gen {
    Off,
    Echo(HarmonyType),
    Arp,
}

/// A key message from the input thread's processor to the engine thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FxKey {
    /// A right-hand key for the Echo category or the arpeggio went down (the key as
    /// fingered: transpose and octaves apply when its notes sound).
    On { key: u8, vel: u8 },
    Off { key: u8 },
    /// Strum: a harmony note to start `delay_ms` after its melody key went down, on
    /// channel `ch` at pitch `note` (already transposed), until melody key `melody` goes up.
    Strum { melody: u8, ch: u8, note: u8, vel: u8, delay_ms: u16 },
    /// Melody key `melody` went up: its Strum notes end.
    StrumOff { melody: u8 },
}

/// Room in the ring for key messages (a burst of a full keyboard, twice, with strums).
pub const FX_RING: usize = 512;

/// The Right parts as the harmony sees them. yahaha's parts are all Poly.
/// The Right parts an arpeggio sounds on for `assign` (RM p.46). Auto: every Right part
/// that is on, as the keys would play; Right 1-3: that part. Multi is only offered for the
/// Harmony and Echo categories (RM p.46), so the app hides it for an arpeggio; a Multi left
/// over from a Harmony type plays as Auto.
pub fn arp_mask(assign: Assign, on: RightParts) -> PartMask {
    let assign = if assign == Assign::Multi { Assign::Auto } else { assign };
    let r = harmony::route(assign, Category::Echo, on, 1);
    if assign == Assign::Auto { r.melody } else { r.effect[0] }
}

pub fn right_parts(parts: &Parts) -> RightParts {
    RightParts { on: [parts.is_on(parts::RIGHT1), parts.is_on(parts::RIGHT2), parts.is_on(parts::RIGHT3)], mono: [false; 3] }
}

/// What this driver sounds, counted per (channel, note), and where each generator key's
/// notes went.
struct Voices {
    count: [[u8; 128]; 16],
    /// By generator key (the arp's pitch, EchoGen's key): the channels and pitches its
    /// sounding note went out on.
    sent: [Sounded; 128],
}

impl Voices {
    fn hold(&mut self, ch: u8, note: u8, vel: u8, out: &mut Out) {
        let n = &mut self.count[ch as usize & 15][note as usize & 127];
        *n = n.saturating_add(1);
        out.push(&[0x90 | ch, note, vel]);
    }

    fn unhold(&mut self, ch: u8, note: u8, out: &mut Out) {
        let n = &mut self.count[ch as usize & 15][note as usize & 127];
        if *n <= 1 {
            *n = 0;
            out.push(&[0x80 | ch, note, 0]);
        } else {
            *n -= 1;
        }
    }

    /// Generator key `key` sounds as `note` at `vel` on the Right parts in `mask` that are
    /// on, each at its octave, with the transpose `shift`.
    #[allow(clippy::too_many_arguments)]
    fn start(&mut self, key: u8, note: u8, vel: u8, mask: PartMask, parts: &Parts, shift: i8, out: &mut Out) {
        self.stop(key, out);
        let mut s = Sounded::default();
        for p in [parts::RIGHT1, parts::RIGHT2, parts::RIGHT3] {
            if mask & (1 << p) != 0 && parts.is_on(p) {
                let ch = parts::CHANNEL[p];
                let n = shift_key(note, shift + 12 * parts.octave_of(p));
                s.push(ch, n);
                self.hold(ch, n, vel, out);
            }
        }
        self.sent[key as usize & 127] = s;
    }

    /// Generator key `key`'s note ends, where it sounded.
    fn stop(&mut self, key: u8, out: &mut Out) {
        let s = std::mem::take(&mut self.sent[key as usize & 127]);
        for (ch, n) in s.iter() {
            self.unhold(ch, n, out);
        }
    }

    /// Everything this driver sounds, off.
    fn all_off(&mut self, out: &mut Out) {
        for ch in 0..16u8 {
            for n in 0..128u8 {
                if self.count[ch as usize][n as usize] > 0 {
                    self.count[ch as usize][n as usize] = 0;
                    out.push(&[0x80 | ch, n, 0]);
                }
            }
        }
        self.sent = [Sounded::default(); 128];
    }

    fn sounding(&self) -> usize {
        self.count.iter().flatten().filter(|&&n| n > 0).count()
    }
}

/// The arp's `ArpSink`: its notes to the Right parts, now.
struct ArpOut<'a> {
    v: &'a mut Voices,
    out: &'a mut Out,
    parts: &'a Parts,
    shift: i8,
    mask: PartMask,
    volume: u8,
}

impl ArpSink for ArpOut<'_> {
    fn note_on(&mut self, _tick: u64, note: u8, vel: u8) {
        let vel = harmony::effect_velocity(vel, self.volume);
        if vel > 0 {
            self.v.start(note, note, vel, self.mask, self.parts, self.shift, self.out);
        }
    }

    fn note_off(&mut self, _tick: u64, note: u8) {
        self.v.stop(note, self.out);
    }
}

/// A Strum note waiting for its time, or sounding until its melody key goes up.
#[derive(Clone, Copy, Default)]
struct Strum {
    active: bool,
    started: bool,
    melody: u8,
    ch: u8,
    note: u8,
    vel: u8,
    due: u64,
}

const MAX_STRUM: usize = 64;
const NO_TICK: u64 = u64::MAX;
/// The arp counts in eighths of a style tick (its ppq is the style's × `SUB`): it plays
/// what is due before the current sub-tick, so a note is at most 1/8 tick late (0.13 ms at
/// 480 PPQ and 120 BPM), never early, and a key pressed now starts on the next sub-tick.
/// The grid lines (Quantize, the bar) stay where the style's are.
pub const SUB: u32 = 8;

/// The engine thread's Harmony/Arpeggio driver. See the module docs.
pub struct KbdFx {
    rx: Consumer<FxKey>,
    word: u64,
    cfg: FxConfig,
    running: Gen,
    echo: EchoGen,
    arp: Arp,
    v: Voices,
    strum: [Strum; MAX_STRUM],
    /// The tempo EchoGen was last given.
    bpm: f64,
    /// End (exclusive) of the last sub-tick range the arp processed; `NO_TICK` before the
    /// first, and after a new resolution.
    arp_to: u64,
}

impl KbdFx {
    /// Built on the control side (`channels`): nothing is allocated later.
    pub fn new(rx: Consumer<FxKey>) -> KbdFx {
        let cfg = FxConfig::default();
        KbdFx {
            rx,
            word: cfg.pack(),
            cfg,
            running: Gen::Off,
            echo: EchoGen::new(&cfg.harmony, 120.0),
            arp: Arp::new(480 * SUB, PATTERNS[0].clone()),
            v: Voices { count: [[0; 128]; 16], sent: [Sounded::default(); 128] },
            strum: [Strum::default(); MAX_STRUM],
            bpm: 120.0,
            arp_to: NO_TICK,
        }
    }

    /// Key messages are waiting (the engine's spin checks it).
    #[inline]
    pub fn pending(&self) -> bool {
        self.rx.slots() > 0
    }

    /// How many (channel, note)s this driver has sounding (tests).
    pub fn sounding(&self) -> usize {
        self.v.sounding()
    }

    /// The arp's clock at `now`, in sub-ticks: the style clock × `SUB` (see the module docs).
    fn tick(engine: &Engine, now: u64) -> u64 {
        let t = engine.style_tick(now) * SUB as f64;
        // The epsilon covers `ns_at_tick` rounding (it rounds up to the ns).
        if t.is_finite() && t > 0.0 { (t + 1e-3).floor() as u64 } else { 0 }
    }

    /// The arp's resolution for the style playing.
    fn arp_ppq(engine: &Engine) -> u32 {
        engine.ppq().saturating_mul(SUB)
    }

    /// When the engine must wake next for this driver, if anything is due.
    pub fn next_deadline(&self, engine: &Engine) -> Option<u64> {
        let mut t = self.strum.iter().filter(|s| s.active && !s.started).map(|s| s.due).min();
        let mut at = |d: u64| t = Some(t.map_or(d, |x: u64| x.min(d)));
        match self.running {
            Gen::Echo(_) => {
                if let Some(d) = self.echo.next_due() {
                    at(d);
                }
            }
            Gen::Arp => {
                // Played once the clock is past its sub-tick.
                if let Some(d) = self.arp.next_due() {
                    at(engine.ns_at_tick(d.saturating_add(1) as f64 / SUB as f64));
                }
            }
            Gen::Off => {}
        }
        t
    }

    /// One engine wake at `now`: settings, key messages, then whatever is due.
    pub fn step(&mut self, now: u64, engine: &Engine, shared: &Shared, out: &mut Out) {
        let parts = &*shared.parts;
        let shift = shared.key_shift.load(Relaxed);
        let tick = Self::tick(engine, now);
        // A style with another resolution: the arp re-times first.
        if self.running == Gen::Arp && Self::arp_ppq(engine) != self.arp.ppq() {
            let (mask, volume) = self.arp_route(parts);
            let mut sink = ArpOut { v: &mut self.v, out, parts, shift, mask, volume };
            self.arp.set_ppq(Self::arp_ppq(engine), tick, &mut sink);
            self.arp_to = NO_TICK;
        }
        let w = shared.kbd_fx.load(Acquire);
        if w != self.word {
            self.configure(w, now, engine, tick, parts, shift, out);
        }
        if engine.bpm() != self.bpm {
            self.bpm = engine.bpm();
            self.echo.set_tempo(self.bpm);
        }
        while let Ok(k) = self.rx.pop() {
            self.key(k, now, tick, out);
        }
        self.reconcile(shared, now, tick, out);
        self.play(now, tick, parts, shift, out);
    }

    /// New settings: a new generator stops the old one first.
    #[allow(clippy::too_many_arguments)]
    fn configure(&mut self, w: u64, now: u64, engine: &Engine, tick: u64, parts: &Parts, shift: i8, out: &mut Out) {
        let cfg = FxConfig::unpack(w);
        let next = cfg.generator();
        if next != self.running {
            self.stop_generator(now, tick, parts, shift, out);
            match next {
                Gen::Echo(_) => self.echo = EchoGen::new(&cfg.harmony, engine.bpm()),
                Gen::Arp => {
                    // Stopped (`stop_generator`): nothing sounds, so a new resolution cuts nothing.
                    let (mask, volume) = self.arp_route(parts);
                    let mut sink = ArpOut { v: &mut self.v, out, parts, shift, mask, volume };
                    self.arp.set_ppq(Self::arp_ppq(engine), tick, &mut sink);
                    self.arp.set_pattern(PATTERNS[cfg.pattern as usize].clone(), tick);
                    self.arp.set_settings(cfg.arp_settings(), tick);
                    self.arp_to = NO_TICK;
                }
                Gen::Off => {}
            }
            self.bpm = engine.bpm();
        } else {
            match next {
                Gen::Echo(_) => self.echo.set_settings(&cfg.harmony),
                Gen::Arp => {
                    if cfg.pattern != self.cfg.pattern {
                        self.arp.set_pattern(PATTERNS[cfg.pattern as usize].clone(), tick);
                    }
                    self.arp.set_settings(cfg.arp_settings(), tick);
                }
                Gen::Off => {}
            }
        }
        self.cfg = cfg;
        self.running = next;
        self.word = w;
    }

    /// Stop the generator running: its notes off now, its keys forgotten.
    fn stop_generator(&mut self, now: u64, tick: u64, parts: &Parts, shift: i8, out: &mut Out) {
        match self.running {
            Gen::Echo(_) => {
                self.echo.all_off(now);
                self.drain_echo(now, parts, shift, out);
            }
            Gen::Arp => {
                let (mask, volume) = self.arp_route(parts);
                let mut sink = ArpOut { v: &mut self.v, out, parts, shift, mask, volume };
                self.arp.all_off(tick, &mut sink);
            }
            Gen::Off => {}
        }
    }

    fn key(&mut self, k: FxKey, now: u64, tick: u64, out: &mut Out) {
        match k {
            FxKey::On { key, vel } => match self.running {
                Gen::Echo(_) => self.echo.note_on(key, vel, now),
                Gen::Arp => self.arp.note_on(key, vel, tick),
                Gen::Off => {}
            },
            FxKey::Off { key } => match self.running {
                Gen::Echo(_) => self.echo.note_off(key, now),
                Gen::Arp => self.arp.note_off(key, tick),
                Gen::Off => {}
            },
            FxKey::Strum { melody, ch, note, vel, delay_ms } => {
                if let Some(s) = self.strum.iter_mut().find(|s| !s.active) {
                    *s = Strum { active: true, started: false, melody, ch, note, vel, due: now.saturating_add(delay_ms as u64 * 1_000_000) };
                }
            }
            FxKey::StrumOff { melody } => self.strum_off(out, |s| s.melody == melody),
        }
    }

    /// End the Strum notes `f` picks: the started ones stop, the waiting ones never start.
    fn strum_off(&mut self, out: &mut Out, f: impl Fn(&Strum) -> bool) {
        for i in 0..MAX_STRUM {
            let s = self.strum[i];
            if s.active && f(&s) {
                self.strum[i].active = false;
                if s.started {
                    self.v.unhold(s.ch, s.note, out);
                }
            }
        }
    }

    /// Release the generator keys that are no longer down (a key-up the ring lost), and
    /// Strum notes of melody keys that went up.
    fn reconcile(&mut self, shared: &Shared, now: u64, tick: u64, out: &mut Out) {
        let held = |k: u8| shared.fx_held[(k >> 6) as usize].load(Acquire) & (1u64 << (k & 63)) != 0;
        let mut gone = [0u8; 16];
        let mut n = 0;
        match self.running {
            Gen::Echo(_) => {
                for k in self.echo.keys_down() {
                    if !held(k) && n < gone.len() {
                        gone[n] = k;
                        n += 1;
                    }
                }
                for &k in &gone[..n] {
                    self.echo.note_off(k, now);
                }
            }
            Gen::Arp => {
                for k in self.arp.keys_down() {
                    if !held(k) && n < gone.len() {
                        gone[n] = k;
                        n += 1;
                    }
                }
                for &k in &gone[..n] {
                    self.arp.note_off(k, tick);
                }
            }
            Gen::Off => {}
        }
        self.strum_off(out, |s| !held(s.melody));
    }

    /// The Right parts the arpeggio sounds on, and its level: Assign and Volume (RM p.46,
    /// HrmArpVol).
    fn arp_route(&self, parts: &Parts) -> (PartMask, u8) {
        (arp_mask(self.cfg.harmony.assign, right_parts(parts)), self.cfg.harmony.volume)
    }

    fn drain_echo(&mut self, now: u64, parts: &Parts, shift: i8, out: &mut Out) {
        let r = harmony::route(self.cfg.harmony.assign, Category::Echo, right_parts(parts), 1);
        let mut buf = [EchoEvent::default(); 16];
        loop {
            let n = self.echo.next_events(now, &mut buf);
            for e in &buf[..n] {
                if e.vel == 0 {
                    self.v.stop(e.key, out);
                } else {
                    let mask = if e.effect { r.effect[0] } else { r.melody };
                    self.v.start(e.key, e.key, e.vel, mask, parts, shift, out);
                }
            }
            if n < buf.len() {
                break;
            }
        }
    }

    /// Everything due at `now`.
    fn play(&mut self, now: u64, tick: u64, parts: &Parts, shift: i8, out: &mut Out) {
        match self.running {
            Gen::Echo(_) => self.drain_echo(now, parts, shift, out),
            Gen::Arp => {
                // Everything due before the current sub-tick.
                match self.arp_to {
                    // The first range, or the clock went back (a START, a style change):
                    // the arp restarts its pattern there.
                    t if t == NO_TICK || t > tick => self.arp.jump(tick),
                    t if t < tick => {
                        let (mask, volume) = self.arp_route(parts);
                        let mut sink = ArpOut { v: &mut self.v, out, parts, shift, mask, volume };
                        self.arp.process(t..tick, &mut sink);
                    }
                    _ => {}
                }
                self.arp_to = tick;
            }
            Gen::Off => {}
        }
        for s in self.strum.iter_mut() {
            if !s.active {
                continue;
            }
            if !s.started && s.due <= now {
                s.started = true;
                self.v.hold(s.ch, s.note, s.vel, out);
            }
        }
    }

    /// Everything off now (PANIC, shutdown): the generator stops, Strum notes end, and
    /// whatever this driver still counts as sounding gets its note-off.
    pub fn all_off(&mut self, now: u64, engine: &Engine, shared: &Shared, out: &mut Out) {
        let parts = &*shared.parts;
        let shift = shared.key_shift.load(Relaxed);
        let tick = Self::tick(engine, now);
        self.stop_generator(now, tick, parts, shift, out);
        self.strum = [Strum::default(); MAX_STRUM];
        self.v.all_off(out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_through_the_word() {
        assert_eq!(FxConfig::unpack(0), FxConfig::default());
        for (i, &ty) in ALL_TYPES.iter().enumerate() {
            for (j, &assign) in ASSIGNS.iter().enumerate() {
                let c = FxConfig {
                    on: i % 2 == 0,
                    mode: if j % 2 == 0 { FxMode::Harmony } else { FxMode::Arpeggio },
                    harmony: HarmonySettings {
                        ty,
                        volume: (i * 5) as u8,
                        speed: EchoSpeed::ALL[i % 6],
                        assign,
                        chord_note_only: j % 2 == 1,
                        min_velocity: 1 + (i * 7 % 127) as u8,
                    },
                    pattern: (i % PATTERNS.len()) as u8,
                    quantize: [Quantize::Off, Quantize::Eighth, Quantize::Sixteenth][j % 3],
                    hold: i % 3 == 0,
                    velocity: [Velocity::Original, Velocity::Thru, Velocity::Fixed(1 + i as u8)][(i + j) % 3],
                    keep_key_on: j % 3 == 1,
                };
                assert_eq!(FxConfig::unpack(c.pack()), c);
            }
        }
    }

    /// A key-up the ring lost (it was full): the engine sees the key is no longer down
    /// (`Shared::fx_held`) and releases it from the arpeggio and from Echo, so nothing
    /// plays on.
    #[test]
    fn a_lost_key_up_is_released_from_the_held_mask() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return;
        }
        let style = crate::sff::Style::load(&p).unwrap();
        for cfg in [FxConfig { on: true, mode: FxMode::Arpeggio, ..FxConfig::default() }, {
            let mut c = FxConfig { on: true, ..FxConfig::default() };
            c.harmony.ty = HarmonyType::Tremolo;
            c
        }] {
            let shared = Arc::new(Shared::new(54));
            let (synth, mut heard) = RingBuffer::new(1 << 14);
            let mut ch = channels(Out::new(crate::rt::PacketSink::new(crate::rt::Target::Null), Some(synth)));
            let mut l = EngineLoop::new(Engine::new(Box::new(Prepared::new(&style))), ch.io, shared.clone());
            shared.kbd_fx.store(cfg.pack(), Release);
            shared.fx_held[0].store(1 << 60, Release);
            ch.fx_tx.push(FxKey::On { key: 60, vel: 100 }).unwrap();
            let mut now = 1_000;
            let mut ons = 0;
            let mut run = |l: &mut EngineLoop, now: &mut u64, ns: u64, ons: &mut usize| {
                let end = *now + ns;
                while *now < end {
                    l.step(*now);
                    *now = l.next_deadline().unwrap_or(*now + 5_000_000).clamp(*now + 1, end);
                    while let Ok(m) = heard.pop() {
                        if m[0] & 0xF0 == 0x90 && m[2] > 0 && m[0] & 0x0F == 0 {
                            *ons += 1;
                        }
                    }
                }
            };
            run(&mut l, &mut now, 1_000_000_000, &mut ons);
            assert!(ons > 2, "{cfg:?}: playing");
            // The key goes up; its message is lost.
            shared.fx_held[0].store(0, Release);
            run(&mut l, &mut now, 500_000_000, &mut ons);
            let before = ons;
            run(&mut l, &mut now, 1_000_000_000, &mut ons);
            assert_eq!(ons, before, "{cfg:?}: stopped");
            assert_eq!(l.io.fx.sounding(), 0, "{cfg:?}");
        }
    }

    /// Assign for an arpeggio: Auto is every Right part that is on (not only the first);
    /// Multi, not offered for an arpeggio (RM p.46), plays as Auto.
    #[test]
    fn arp_assign_picks_the_parts() {
        let r1_r3 = RightParts { on: [true, false, true], mono: [false; 3] };
        assert_eq!(arp_mask(Assign::Auto, r1_r3), 0b101);
        assert_eq!(arp_mask(Assign::Multi, r1_r3), 0b101);
        assert_eq!(arp_mask(Assign::Right3, r1_r3), 0b100);
        assert_eq!(arp_mask(Assign::Auto, RightParts::only(parts::RIGHT2)), 0b010);
    }

    #[test]
    fn the_type_picks_the_processor() {
        let mut c = FxConfig { on: true, ..FxConfig::default() };
        assert_eq!(c.processor(), Processor::Harmony(c.harmony));
        c.harmony.ty = HarmonyType::MultiAssign;
        assert_eq!(c.processor(), Processor::MultiAssign);
        c.harmony.ty = HarmonyType::Trill;
        assert_eq!(c.processor(), Processor::Echo);
        c.mode = FxMode::Arpeggio;
        assert_eq!(c.processor(), Processor::Arp);
        c.on = false;
        assert_eq!(c.processor(), Processor::Off);
    }
}
