//! The arranger engine: section state machine, pattern playback, chord following.
//!
//! The engine is deterministic and has no notion of threads or wall time: callers pass
//! the current time in nanoseconds. It never allocates after construction, so the
//! real-time output thread can drive it directly.
//!
//! Layout: this file holds the types and the `Engine` state; its behaviour is split by
//! concern into `impl Engine` blocks in the modules below. Features plug in through the
//! hooks in hooks.rs; when a queued section change happens is `Engine::change_point`
//! (sections.rs). See docs/architecture.md.

mod chords;
mod hooks;
mod mirror;
mod mixer;
mod playback;
mod prepared;
mod sections;
mod settle;
mod setup;
mod style_change;
mod transport;

use hooks::{Features, Lines};
use mirror::{Mirror, NRPN_BIT, UNSENT};
use sections::Change;
pub use mixer::{Takeover, HW_UNKNOWN};
use prepared::PKind;
pub use prepared::{id_of, slot_of, Msgs, PSection, Prepared, NUM_SLOTS};
pub use settle::{CHORD_SETTLE_DEFAULT_MS, CHORD_SETTLE_MAX_MS};
use settle::{Hold, Unsettled};

use crate::sff::{ChannelRule, Ntr, Ntt, Rtr, SectionId, Style};
use crate::theory::{is_drum_part, plays, transpose_group, Chord, CANCEL, GUITAR_NOISE};

pub trait Sink {
    fn send(&mut self, msg: &[u8]);

    /// Retrigger Rule pitch shift: every note on `ch` now sounds `semis` above the key it
    /// was sent with (the engine has already sent the pitch bend). MIDI sinks ignore it; the
    /// sim listing uses it to show the pitch that sounds.
    fn retune(&mut self, _ch: u8, _semis: i8) {}
}

/// Pitch bend range (RPN 0) a part has before the style sets one: the GM/XG default.
pub const GM_BEND_RANGE: u8 = 2;
/// The pitch shift the engine can bend a part that follows chords by: up to an octave
/// either way, which covers every to-Root move (at most 6) and nearly every Pitch Shift.
/// It is also the smallest bend range such a part gets on the output.
pub const RTR_BEND_RANGE: u8 = 12;
/// The widest pitch bend range a Genos part takes (DL p.98: RPN 0 Pitch Bend Sensitivity
/// 00H-18H, received by the Style parts; MIDI Implementation Chart: 0-24 semi).
pub const MAX_BEND_RANGE: u8 = 24;

/// Parts whose sounding notes a chord change can re-pitch: every accompaniment part but
/// the two rhythm parts.
#[inline]
fn follows_chords(ch: u8) -> bool {
    (8..16).contains(&ch) && !is_drum_part(ch)
}

/// The pitch bend range a part gets on the output: the style's own, raised so that a
/// pitch shift of `RTR_BEND_RANGE` fits on top of the widest bend its patterns make
/// (`pat_max` semitones), but no wider than `MAX_BEND_RANGE`, and never below
/// `RTR_BEND_RANGE`. The pattern's own bends are rescaled to it (see `Engine::send_bend`).
#[inline]
fn out_bend_range(style_range: u8, pat_max: u8) -> u8 {
    style_range.max(RTR_BEND_RANGE).max(pat_max.saturating_add(RTR_BEND_RANGE).min(MAX_BEND_RANGE))
}

/// XG Multi Part parameter address of a part's volume. The part fader owns the volume.
const XG_PART_VOLUME: u8 = 0x0B;

/// GM default channel volume (CC7) for a part the style never sets.
pub const GM_VOLUME: u8 = 100;
/// No RPN selected (MSB and LSB 127).
const RPN_NULL: u16 = 0x3FFF;
/// Pitch bend centre.
const BEND_CENTRE: u16 = 0x2000;
/// Soft takeover: a hardware fader within this distance of the software value picks it up.
pub const PICKUP_RANGE: u8 = 2;

/// The RPN selected on a channel (MSB << 7 | LSB) after controller `cc` = `val`. Selecting
/// an NRPN (CC 98/99) deselects the RPN, so data entry no longer sets it.
#[inline]
fn select_rpn(rpn: u16, cc: u8, val: u8) -> u16 {
    match cc {
        101 => (rpn & 0x7F) | (val as u16) << 7,
        100 => (rpn & !0x7F) | val as u16,
        98 | 99 => RPN_NULL,
        _ => rpn,
    }
}

// ---------------------------------------------------------------------------
// Commands and state
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Intro(u8),
    Main(u8),
    Break,
    Ending(u8),
    StartStop,
    Stop,
    SyncStart,
    SyncStop,
    AutoFill,
    TapTempo,
    TempoUp,
    TempoDown,
    TogglePart(u8),
    StopAcmp,
}


#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Snapshot {
    pub running: bool,
    pub sync_armed: bool,
    pub sync_stop: bool,
    pub auto_fill: bool,
    pub cur: Option<SectionId>,
    pub queued: Option<SectionId>,
    pub pending_intro: Option<u8>,
    pub main: u8,
    pub bar: u32,
    pub beat: u32,
    pub chord: Option<Chord>,
    pub bpm: f64,
    pub parts: u8,
    /// Mixer fader per part (0..=127): the part's volume, sent as its CC7 unchanged.
    pub volumes: [u8; 8],
    /// Parts whose hardware fader is waiting to pick up the software value (soft takeover).
    pub pickup: u8,
    pub stop_acmp: bool,
    /// Keyboard and Master transpose in semitones (-12..=12 each).
    pub transpose: Transpose,
    /// The chord as fingered, before Keyboard transpose (`chord` is what the style follows).
    pub played: Option<Chord>,
    /// The playing position, as an anchor to extrapolate from: at `anchor_ns` the section
    /// had played `anchor_beats` quarter notes; it moves on at `bpm`. Changes only when the
    /// tempo, the section or its loop does (0, 0 when stopped).
    pub anchor_ns: u64,
    pub anchor_beats: f64,
    /// `Prepared::tag` of the style playing.
    pub style_tag: u64,
    /// A style waits for the next bar line to take over (`Engine::change_style`).
    pub style_pending: bool,
    /// Bars in the section playing (0 when stopped).
    pub section_bars: u32,
    /// A style preview playing beside the (stopped) band (`live::EngineLoop`); the engine
    /// itself always reports None.
    pub audition: Option<AuditionPos>,
}

/// Where a style preview is: style `id` (the session's library id), bar `bar` of `bars`
/// (1-based), playing chord `chord` of its progression (0-based).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuditionPos {
    pub id: u32,
    pub bar: u8,
    pub bars: u8,
    pub chord: u8,
}

/// A style waiting for the bar line at tick `at` (of the style playing) to take over.
struct PendingStyle {
    style: Box<Prepared>,
    at: f64,
}

/// Genos TRANSPOSE targets that matter for live play (RM p.42). Keyboard shifts the keys
/// and the chord root sent to the Style; Master shifts everything that sounds, the Style
/// output included, except drum and SFX kits. Song transpose has nothing to act on here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Transpose {
    pub keyboard: i8,
    pub master: i8,
}

impl Transpose {
    pub const RANGE: i8 = 12;

    pub fn new(keyboard: i8, master: i8) -> Transpose {
        Transpose { keyboard: keyboard.clamp(-Self::RANGE, Self::RANGE), master: master.clamp(-Self::RANGE, Self::RANGE) }
    }

    /// Total shift applied to the notes the player plays.
    pub fn keys(self) -> i8 {
        self.keyboard + self.master
    }
}

/// Shift a key by `d` semitones, folding by octaves to stay inside the MIDI range.
#[inline]
pub fn shift_key(key: u8, d: i8) -> u8 {
    let mut k = key as i32 + d as i32;
    while k > 127 {
        k -= 12;
    }
    while k < 0 {
        k += 12;
    }
    k as u8
}

/// A chord moved by `d` semitones (root and on-bass note).
pub fn shift_chord(c: Chord, d: i8) -> Chord {
    let pc = |p: u8| (p as i32 + d as i32).rem_euclid(12) as u8;
    Chord { root: pc(c.root), bass: c.bass.map(pc), ..c }
}

#[derive(Clone, Copy)]
struct Sounding {
    active: bool,
    src: u8,
    src_key: u8,
    dest: u8,
    out: u8,
    vel: u8,
    slot: u8,
    started_ns: u64,
    /// When the note last got an attack: its start, or a retrigger on a chord change.
    attack_ns: u64,
    /// A voice that shares its key with another voice sounding on the part (two voices a
    /// chord folds together): it sends nothing, but keeps its place in the pattern, so a
    /// later chord can part the two again, and it sounds on in the other's place if that
    /// one ends first. There is always an unmuted voice on the same part and key.
    muted: bool,
}

const EMPTY: Sounding =
    Sounding { active: false, src: 0, src_key: 0, dest: 0, out: 0, vel: 0, slot: 0, started_ns: 0, attack_ns: 0, muted: false };
const MAX_SOUNDING: usize = 256;
/// Pseudo source channel for Stop Accompaniment notes.
const STOP_ACMP_SRC: u8 = 255;
/// The Style's Bass part (MIDI channel 11).
const BASS_CH: u8 = 10;
/// Notes that started this recently when the chord changes are corrected outright:
/// the player's chord landed just after the beat.
const LATE_CHORD_NS: u64 = 40_000_000;
/// Notes that end this soon after the chord changes are left to end as they are: the
/// player's chord landed just before the beat, and a new attack would be a blip.
pub(crate) const EARLY_CHORD_NS: u64 = 40_000_000;

/// The chord a channel follows. With no chord yet, or after Chord Cancel ("a state in
/// which no chord is input", OM p.46), only rhythm parts and channels whose CASM
/// autostart bit is set play, as recorded (their source chord); everything else rests.
fn effective_chord(chord: Option<Chord>, rule: &ChannelRule) -> Option<Chord> {
    match chord {
        Some(c) if c.ty != CANCEL => Some(c),
        _ if is_drum_part(rule.dest_ch) || rule.autostart => Some(Chord::new(rule.src_root, rule.src_type)),
        _ => None,
    }
}

#[derive(Clone, Copy)]
struct Queued {
    slot: usize,
    at: f64,
    sec_start: f64,
}

pub struct Engine {
    pub style: Box<Prepared>,
    running: bool,
    sync_armed: bool,
    sync_stop: bool,
    /// Sync Stop is unavailable with the Full Keyboard fingering types.
    sync_stop_allowed: bool,
    auto_fill: bool,
    main: u8,
    pending_intro: Option<u8>,
    cur: usize,
    sec_start: f64,
    /// Section-relative tick this pass of the section started playing from (a Fill or
    /// Break enters mid-bar; nothing before it has sounded).
    entry: f64,
    ev_idx: usize,
    queued: Option<Queued>,
    /// The chord the style follows: `played` moved by Keyboard transpose.
    chord: Option<Chord>,
    played: Option<Chord>,
    transpose: Transpose,
    bpm: f64,
    anchor_ns: u64,
    anchor_tick: f64,
    ns_per_tick: f64,
    parts: u8,
    /// Mixer faders: each part's channel volume (CC7), sent as is. A style load sets them
    /// from the style's own levels.
    mixer: [u8; 8],
    /// Parts whose fader the player has moved since the style loaded: pattern CC7 no
    /// longer overrides them.
    user_set: u8,
    /// Soft takeover state of each part's hardware fader.
    takeover: [Takeover; 8],
    stop_acmp: bool,
    /// Manual Bass (Upper detection mode): the Style's Bass part is muted; the player's
    /// left hand plays the bass instead.
    manual_bass: bool,
    taps: [u64; 4],
    tap_n: usize,
    sounding: [Sounding; MAX_SOUNDING],
    /// Retrigger Rule pitch shift per channel: the semitones every note on the channel is
    /// bent by. A note sent while it is set goes out that much lower so it sounds true
    /// (`Sounding::out` is the key sent). It returns to 0 when the channel falls silent.
    rtr_bend: [i8; 16],
    /// The pattern's own pitch bend per channel (14-bit, in the style's bend range).
    pat_bend: [u16; 16],
    /// The style's pitch bend range per channel (RPN 0, semitones).
    bend_range: [u8; 16],
    /// The RPN a pattern has selected per channel (MSB << 7 | LSB).
    rpn: [u16; 16],
    /// What has been sent on each channel (controllers, voices, (N)RPNs, bends).
    mirror: Box<Mirror>,
    /// Channels where a pattern sent a program change since the part setup last went out
    /// there: the receiver has reset that part's XG parameters and drum setup, even when
    /// the pattern has since gone back to the setup's voice.
    pattern_pc: u16,
    /// A style change queued for the next bar line (`change_style`).
    pending: Option<PendingStyle>,
    /// Styles the engine is done with, for the caller to free off the real-time thread
    /// (`take_retired`).
    retired: [Option<Box<Prepared>>; 4],
    /// The next bar or beat line for the `on_bar`/`on_beat` hooks (hooks.rs).
    lines: Lines,
    /// The chord-settle window (settle.rs), in ns.
    settle_ns: u64,
    /// A chord change the band has not followed yet (settle.rs).
    unsettled: Option<Unsettled>,
    /// Where the pattern's notes held back while the chord settles begin.
    hold: Option<Hold>,
    /// The engine-side state of the features that plug into the hooks (hooks.rs).
    #[allow(dead_code)]
    features: Features,
    /// Pitch bends that did not fit the output range and were clamped.
    #[cfg(test)]
    pub(crate) bend_clamps: std::cell::Cell<u32>,
    /// Notes a chord change retriggered, as (time, channel, key sent).
    #[cfg(test)]
    pub(crate) retriggered: Vec<(u64, u8, u8)>,
    /// The hooks that ran, in order.
    #[cfg(test)]
    pub(crate) hook_log: Vec<hooks::Hook>,
}

impl Engine {
    pub fn new(style: Box<Prepared>) -> Engine {
        let bpm = style.bpm;
        let mixer = style.mix;
        let mut e = Engine {
            style,
            running: false,
            sync_armed: true,
            sync_stop: false,
            sync_stop_allowed: true,
            auto_fill: true,
            main: 0,
            pending_intro: None,
            cur: 4,
            sec_start: 0.0,
            entry: 0.0,
            ev_idx: 0,
            queued: None,
            chord: None,
            played: None,
            transpose: Transpose::default(),
            bpm,
            anchor_ns: 0,
            anchor_tick: 0.0,
            ns_per_tick: 0.0,
            parts: 0xFF,
            mixer,
            user_set: 0,
            takeover: [Takeover::NEW; 8],
            stop_acmp: false,
            manual_bass: false,
            taps: [0; 4],
            tap_n: 0,
            sounding: [EMPTY; MAX_SOUNDING],
            rtr_bend: [0; 16],
            pat_bend: [BEND_CENTRE; 16],
            bend_range: [GM_BEND_RANGE; 16],
            rpn: [RPN_NULL; 16],
            mirror: Box::new(Mirror::NEW),
            pattern_pc: 0,
            pending: None,
            retired: [None, None, None, None],
            lines: Lines::default(),
            settle_ns: 0,
            unsettled: None,
            hold: None,
            features: Features::default(),
            #[cfg(test)]
            bend_clamps: Default::default(),
            #[cfg(test)]
            retriggered: Vec::new(),
            #[cfg(test)]
            hook_log: Vec::new(),
        };
        e.set_bpm_internal(bpm, 0);
        e
    }

    // ----- queries -----

    /// When bar `bar` (0-based, counted from the start) begins: for a style preview, which
    /// plays one section from tick 0 at a steady tempo.
    pub fn ns_at_bar(&self, bar: u32) -> u64 {
        self.ns_at(bar as f64 * self.style.tpb as f64)
    }

    /// A chord would start the band now (Sync Start armed, stopped).
    pub fn starts_on_chord(&self) -> bool {
        self.sync_armed && !self.running
    }


    #[allow(dead_code)]
    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn snapshot(&self, now: u64) -> Snapshot {
        let (anchor_ns, anchor_beats) =
            if self.running { (self.anchor_ns, (self.anchor_tick - self.sec_start) / self.style.ppq as f64) } else { (0, 0.0) };
        let (bar, beat) = if self.running {
            let pos = (self.tick_at(now) - self.sec_start).max(0.0);
            let tpb = self.style.tpb as f64;
            ((pos / tpb) as u32, ((pos % tpb) / self.style.ppq as f64) as u32)
        } else {
            (0, 0)
        };
        Snapshot {
            running: self.running,
            sync_armed: self.sync_armed,
            sync_stop: self.sync_stop,
            auto_fill: self.auto_fill,
            cur: if self.running { Some(id_of(self.cur)) } else { None },
            queued: self.queued.map(|q| id_of(q.slot)),
            pending_intro: self.pending_intro,
            main: self.main,
            bar,
            beat,
            chord: self.chord,
            bpm: self.bpm,
            parts: self.parts,
            volumes: self.mixer,
            pickup: self.pickup_waiting(),
            stop_acmp: self.stop_acmp,
            transpose: self.transpose,
            played: self.played,
            anchor_ns,
            anchor_beats,
            style_tag: self.style.tag,
            style_pending: self.pending.is_some(),
            section_bars: match self.style.sections[self.cur].as_ref() {
                Some(s) if self.running => s.len.div_ceil(self.style.tpb.max(1)),
                _ => 0,
            },
            audition: None,
        }
    }

    /// Time of the next thing the engine needs to do: if running, or a chord change is
    /// waiting to settle (settle.rs).
    pub fn next_deadline(&self) -> Option<u64> {
        if !self.running {
            return self.settle_at();
        }
        let sec = self.style.sections[self.cur].as_ref()?;
        let mut t = self.sec_start + sec.len as f64;
        if let Some(q) = self.queued {
            t = t.min(q.at);
        }
        if let Some(p) = &self.pending {
            t = t.min(p.at);
        }
        if let Some(e) = sec.events.get(self.ev_idx) {
            t = t.min(self.sec_start + e.tick as f64);
        }
        if let Some(h) = self.hook_deadline() {
            t = t.min(h);
        }
        let t = self.ns_at(t);
        Some(self.settle_at().map_or(t, |s| s.min(t)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Data entry after an NRPN select (CC 99/98) is not a pitch bend range; selecting
    /// RPN 0 again makes it one.
    #[test]
    fn nrpn_deselects_the_rpn() {
        let rpn0 = |r: u16| select_rpn(select_rpn(r, 101, 0), 100, 0);
        let r = rpn0(RPN_NULL);
        assert_eq!(r, 0);
        let r = select_rpn(select_rpn(r, 99, 1), 98, 8);
        assert_ne!(r, 0);
        assert_eq!(select_rpn(r, 6, 24), r);
        assert_eq!(rpn0(r), 0);
    }

    /// The output range fits a full octave of pitch shift over the pattern's own widest
    /// bend, up to 24.
    #[test]
    fn output_bend_range_leaves_room_for_pattern_bends() {
        assert_eq!(out_bend_range(GM_BEND_RANGE, 0), 12);
        assert_eq!(out_bend_range(12, 1), 13);
        assert_eq!(out_bend_range(12, 12), 24);
        assert_eq!(out_bend_range(24, 24), 24);
        assert_eq!(out_bend_range(2, 20), 24);
    }
}
