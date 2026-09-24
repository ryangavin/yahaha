//! Keyboard Harmony: our own implementation of the documented Genos harmony types.
//!
//! Pure and real-time safe: nothing here allocates, locks or panics, so the input thread
//! can call it per note. It is not wired into the engine yet; see docs/harmony.md for
//! every type's rule and which parts of it are guesses.
//!
//! The pieces:
//! - [`chord_zone`] / [`harmony_chord`]: which keys and which chord drive the harmony in
//!   each ACMP/LEFT combination (spec §6).
//! - [`voice`]: the harmony keys for one melody key and chord (the voicing rules).
//! - [`harmonize`]: [`voice`] plus the detail settings (Chord Note Only, Minimum
//!   Velocity, Volume, Assign) for one melody note-on.
//! - [`HarmonyTracker`]: remembers what each melody key added, so its note-off can stop it.
//! - [`MultiAssign`]: the Multi Assign type (right-hand chord notes spread over R1/R2/R3).
//! - [`EchoGen`]: the Echo category (Echo, Tremolo, Trill) as a timing generator the engine
//!   drives with `next_events(now)`.
//!
//! Keys are MIDI note numbers; times are nanoseconds on the engine's clock.

use crate::parts::{RIGHT1, RIGHT2, RIGHT3};
use crate::theory::{chord_tones, Chord, CANCEL};

/// Most notes one melody key can add (Full Chord adds five).
pub const MAX_NOTES: usize = 6;

// ---------------------------------------------------------------------------
// Types and settings
// ---------------------------------------------------------------------------

/// The Data List (p.74) Keyboard Harmony types, in list order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HarmonyType {
    StandardDuet1,
    StandardDuet2,
    StandardTrio,
    FullChord,
    RockDuet,
    CountryDuet1,
    CountryDuet2,
    CountryTrio,
    Block,
    FourWayClose1,
    FourWayClose2,
    FourWayClose3,
    FourWayClose4,
    FourWayOpen1,
    FourWayOpen2,
    FourWayOpen3,
    OnePlusFive,
    Octave,
    Strum,
    MultiAssign,
    Echo,
    Tremolo,
    Trill,
}

use HarmonyType as T;

/// Every type, in Data List order.
pub const ALL_TYPES: [HarmonyType; 23] = [
    T::StandardDuet1, T::StandardDuet2, T::StandardTrio, T::FullChord, T::RockDuet,
    T::CountryDuet1, T::CountryDuet2, T::CountryTrio, T::Block, T::FourWayClose1,
    T::FourWayClose2, T::FourWayClose3, T::FourWayClose4, T::FourWayOpen1, T::FourWayOpen2,
    T::FourWayOpen3, T::OnePlusFive, T::Octave, T::Strum, T::MultiAssign, T::Echo,
    T::Tremolo, T::Trill,
];

/// The two Keyboard Harmony categories (Arpeggio is separate and not here).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    /// Standard Duet 1 .. Multi Assign.
    Harmony,
    /// Echo, Tremolo, Trill.
    Echo,
}

/// Where a type puts its added notes relative to the melody key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Below,
    Above,
    /// Country Trio: one note above, one below.
    Both,
    /// Multi Assign and the Echo category add no pitches.
    None,
}

impl HarmonyType {
    pub const fn name(self) -> &'static str {
        match self {
            T::StandardDuet1 => "Standard Duet 1",
            T::StandardDuet2 => "Standard Duet 2",
            T::StandardTrio => "Standard Trio",
            T::FullChord => "Full Chord",
            T::RockDuet => "Rock Duet",
            T::CountryDuet1 => "Country Duet 1",
            T::CountryDuet2 => "Country Duet 2",
            T::CountryTrio => "Country Trio",
            T::Block => "Block",
            T::FourWayClose1 => "4-Way Close 1",
            T::FourWayClose2 => "4-Way Close 2",
            T::FourWayClose3 => "4-Way Close 3",
            T::FourWayClose4 => "4-Way Close 4",
            T::FourWayOpen1 => "4-Way Open 1",
            T::FourWayOpen2 => "4-Way Open 2",
            T::FourWayOpen3 => "4-Way Open 3",
            T::OnePlusFive => "1+5",
            T::Octave => "Octave",
            T::Strum => "Strum",
            T::MultiAssign => "Multi Assign",
            T::Echo => "Echo",
            T::Tremolo => "Tremolo",
            T::Trill => "Trill",
        }
    }

    pub const fn category(self) -> Category {
        match self {
            T::Echo | T::Tremolo | T::Trill => Category::Echo,
            _ => Category::Harmony,
        }
    }

    /// Whether the type follows the chord. "1+5" and "Octave" ignore it (OM p.56), and
    /// Multi Assign and the Echo category are independent of ACMP and LEFT (OM p.57).
    pub const fn uses_chord(self) -> bool {
        !matches!(self, T::OnePlusFive | T::Octave | T::MultiAssign | T::Echo | T::Tremolo | T::Trill)
    }

    /// Whether the detail settings apply. Multi Assign has none of them (RM p.46).
    pub const fn has_details(self) -> bool {
        !matches!(self, T::MultiAssign)
    }

    pub const fn direction(self) -> Direction {
        match self {
            T::CountryDuet1 | T::OnePlusFive => Direction::Above,
            T::CountryTrio => Direction::Both,
            T::MultiAssign | T::Echo | T::Tremolo | T::Trill => Direction::None,
            _ => Direction::Below,
        }
    }
}

/// Assign (RM p.46): which Right parts sound the effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assign {
    /// The Right parts that are on, prioritised Right 1, Right 2, Right 3.
    Auto,
    /// The played note on Right 1, the effect notes divided over Right 1 and the others.
    Multi,
    Right1,
    Right2,
    Right3,
}

/// Echo-category Speed as a note value: `Quarter` is 1/4, `QuarterTriplet` 1/6, and so on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EchoSpeed {
    Quarter,
    QuarterTriplet,
    Eighth,
    EighthTriplet,
    Sixteenth,
    ThirtySecond,
}

impl EchoSpeed {
    pub const ALL: [EchoSpeed; 6] = [
        EchoSpeed::Quarter, EchoSpeed::QuarterTriplet, EchoSpeed::Eighth,
        EchoSpeed::EighthTriplet, EchoSpeed::Sixteenth, EchoSpeed::ThirtySecond,
    ];

    /// Repeats per whole note (the denominator of the note value).
    pub const fn per_whole(self) -> u32 {
        match self {
            EchoSpeed::Quarter => 4,
            EchoSpeed::QuarterTriplet => 6,
            EchoSpeed::Eighth => 8,
            EchoSpeed::EighthTriplet => 12,
            EchoSpeed::Sixteenth => 16,
            EchoSpeed::ThirtySecond => 32,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            EchoSpeed::Quarter => "1/4",
            EchoSpeed::QuarterTriplet => "1/6",
            EchoSpeed::Eighth => "1/8",
            EchoSpeed::EighthTriplet => "1/12",
            EchoSpeed::Sixteenth => "1/16",
            EchoSpeed::ThirtySecond => "1/32",
        }
    }

    /// The repeat period in nanoseconds at `bpm` quarter notes per minute. A non-finite or
    /// non-positive tempo reads as 120 BPM; the result is at least 1 ms.
    pub fn period_ns(self, bpm: f64) -> u64 {
        let bpm = if bpm.is_finite() && bpm > 0.0 { bpm } else { 120.0 };
        let quarter = 60e9 / bpm;
        let p = quarter * 4.0 / self.per_whole() as f64;
        (p as u64).max(1_000_000)
    }
}

/// The detail settings (RM p.46-47).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HarmonySettings {
    pub ty: HarmonyType,
    /// Level of the generated notes, 0-127. It scales their velocity: 127 plays them at the
    /// melody's velocity, 0 silences them.
    pub volume: u8,
    /// Echo category only.
    pub speed: EchoSpeed,
    pub assign: Assign,
    /// Harmony category only: harmonise only melody notes that belong to the chord.
    pub chord_note_only: bool,
    /// The effect sounds only when the key velocity is at least this (1-127).
    pub min_velocity: u8,
}

impl Default for HarmonySettings {
    fn default() -> Self {
        HarmonySettings {
            ty: T::StandardDuet1,
            volume: 100,
            speed: EchoSpeed::Eighth,
            assign: Assign::Auto,
            chord_note_only: false,
            min_velocity: 1,
        }
    }
}

/// The Right parts' state as the harmony sees it: which are on and which are Mono (Mono,
/// Legato or Crossfade). Index 0-2 is Right 1-3 ([`crate::parts::RIGHT1`] ..).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RightParts {
    pub on: [bool; 3],
    pub mono: [bool; 3],
}

impl RightParts {
    pub const fn all_on() -> Self {
        RightParts { on: [true; 3], mono: [false; 3] }
    }

    pub const fn only(part: usize) -> Self {
        RightParts { on: [part == RIGHT1, part == RIGHT2, part == RIGHT3], mono: [false; 3] }
    }

    fn mask(&self) -> u8 {
        (self.on[0] as u8) | (self.on[1] as u8) << 1 | (self.on[2] as u8) << 2
    }

    /// The parts an effect of `category` may sound on: on, and for the Harmony category not
    /// Mono (RM p.46: a Mono/Legato/Crossfade part is regarded as off).
    fn eligible(&self, category: Category) -> u8 {
        let mut m = 0;
        for i in 0..3 {
            if self.on[i] && !(category == Category::Harmony && self.mono[i]) {
                m |= 1 << i;
            }
        }
        m
    }
}

/// Bit `i` = Right part `i` (Right 1 = bit 0).
pub type PartMask = u8;

// ---------------------------------------------------------------------------
// Chord source rules (spec §6, OM p.56-57)
// ---------------------------------------------------------------------------

/// Where the harmony chord comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChordSource {
    /// The Style's chord section (left of Split Point (Style)), shared with the Style.
    Style,
    /// The LEFT part's section (left of Split Point (Left)).
    Left,
    /// ACMP and LEFT both off: no chord, so chord-following types add nothing.
    None,
}

/// The keyboard layout the harmony sees for one ACMP/LEFT combination. Split keys belong
/// to the section below them (F#2 and below is the left side).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChordZone {
    pub source: ChordSource,
    /// Keys at or below this form the harmony chord; `None` when there is no chord.
    pub chord_top: Option<u8>,
    /// Keys the LEFT voice plays (inclusive), when LEFT has its own section.
    pub left_voice: Option<(u8, u8)>,
    /// The lowest key the Right parts (and so the melody) play; 128 when a split at 127
    /// leaves them none.
    pub right_bottom: u8,
}

/// Which notes count as the chord in each ACMP/LEFT combination.
///
/// | ACMP | LEFT | chord source | chord keys | LEFT voice | Right parts |
/// |---|---|---|---|---|---|
/// | on | off | Style | ≤ style split | - | > style split |
/// | off | on | Left | ≤ left split | ≤ left split | > left split |
/// | on | on | Style | ≤ style split | style < k ≤ left | > left split |
/// | off | off | none | - | - | whole keyboard |
///
/// `left_split` below `style_split` is read as equal to it (the Genos keeps Left ≥ Style).
pub fn chord_zone(acmp: bool, left: bool, style_split: u8, left_split: u8) -> ChordZone {
    let left_split = left_split.max(style_split);
    let above = |k: u8| k.saturating_add(1);
    match (acmp, left) {
        (true, false) => ChordZone {
            source: ChordSource::Style,
            chord_top: Some(style_split),
            left_voice: None,
            right_bottom: above(style_split),
        },
        (false, true) => ChordZone {
            source: ChordSource::Left,
            chord_top: Some(left_split),
            left_voice: Some((0, left_split)),
            right_bottom: above(left_split),
        },
        (true, true) => ChordZone {
            source: ChordSource::Style,
            chord_top: Some(style_split),
            left_voice: (left_split > style_split).then(|| (above(style_split), left_split)),
            right_bottom: above(left_split),
        },
        (false, false) => ChordZone { source: ChordSource::None, chord_top: None, left_voice: None, right_bottom: 0 },
    }
}

/// The chord the harmony follows: the Style's recognised chord when ACMP is on, the chord
/// read from the LEFT section when only LEFT is on, none when both are off. Types that
/// ignore the chord get `None`; so does a Cancel (N.C.) chord.
pub fn harmony_chord(ty: HarmonyType, acmp: bool, left: bool, style_chord: Option<Chord>, left_chord: Option<Chord>) -> Option<Chord> {
    if !ty.uses_chord() {
        return None;
    }
    let c = match chord_zone(acmp, left, 0, 0).source {
        ChordSource::Style => style_chord,
        ChordSource::Left => left_chord,
        ChordSource::None => None,
    };
    c.filter(|c| c.ty != CANCEL)
}

// ---------------------------------------------------------------------------
// Voicing
// ---------------------------------------------------------------------------

/// A fixed-capacity list of harmony keys, highest first, with a per-note strum delay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Voicing {
    keys: [u8; MAX_NOTES],
    delay_ms: [u16; MAX_NOTES],
    len: u8,
}

impl Voicing {
    const EMPTY: Voicing = Voicing { keys: [0; MAX_NOTES], delay_ms: [0; MAX_NOTES], len: 0 };

    pub fn keys(&self) -> &[u8] {
        &self.keys[..(self.len as usize).min(MAX_NOTES)]
    }

    pub fn delays_ms(&self) -> &[u16] {
        &self.delay_ms[..(self.len as usize).min(MAX_NOTES)]
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn push(&mut self, key: i16) {
        let i = self.len as usize;
        if i < MAX_NOTES && (0..=127).contains(&key) && !self.keys().contains(&(key as u8)) {
            self.keys[i] = key as u8;
            self.len += 1;
        }
    }

    /// Keep the keys highest first (stable for the delays, which are set afterwards).
    fn sort_desc(&mut self) {
        let n = (self.len as usize).min(MAX_NOTES);
        // Insertion sort: at most six keys, no allocation.
        for i in 1..n {
            let mut j = i;
            while j > 0 && self.keys[j - 1] < self.keys[j] {
                self.keys.swap(j - 1, j);
                j -= 1;
            }
        }
    }
}

/// The semitone below which the first harmony note must sit in the duet/trio types, so a
/// two-voice harmony never makes a second against the melody.
const DUET_GAP: i16 = 3;
/// The close-voicing gap: a whole step is fine in a 4-way block (A over G in C6), a
/// semitone never is.
const CLOSE_GAP: i16 = 2;
/// Strum: each added note starts this much after the previous one.
const STRUM_STEP_MS: u16 = 15;

fn pc(k: i16) -> u8 {
    k.rem_euclid(12) as u8
}

/// The chord's pitch classes as a 12-bit mask (bit 0 = C). Only the chord's own tones; an
/// on-bass note that is not one of them does not count.
pub fn chord_mask(chord: Chord) -> u16 {
    rel_mask_at(chord_tones(chord.ty), chord.root)
}

fn rel_mask_at(rel: &[u8], root: u8) -> u16 {
    rel.iter().fold(0u16, |m, &t| m | 1 << ((t as u16 + root as u16) % 12))
}

fn has(mask: u16, p: u8) -> bool {
    mask & (1 << (p % 12)) != 0
}

/// Whether `key` belongs to `chord` (Chord Note Only).
pub fn is_chord_note(key: u8, chord: Chord) -> bool {
    has(chord_mask(chord), key % 12)
}

/// Chord keys below `melody`, nearest first, one per pitch class, never the melody's own
/// pitch class nor the one a semitone under it (it would make a minor 9th / major 7th
/// against the melody). The first is at least `gap` semitones down.
fn below(melody: i16, mask: u16, count: usize, gap: i16, out: &mut Voicing) {
    let m = pc(melody);
    let mut used: u16 = 1 << m | 1 << ((m + 11) % 12);
    let mut n = 0;
    for d in gap..=gap + 11 {
        if n >= count {
            break;
        }
        let k = melody - d;
        let p = pc(k);
        if has(mask, p) && !has(used, p) {
            used |= 1 << p;
            out.push(k);
            n += 1;
        }
    }
}

/// The nearest chord key above `melody`, at least `gap` up, not the melody's pitch class
/// nor the one a semitone above it.
fn nearest_above(melody: i16, mask: u16, gap: i16) -> Option<i16> {
    let m = pc(melody);
    let skip: u16 = 1 << m | 1 << ((m + 1) % 12);
    (gap..=gap + 11).map(|d| melody + d).find(|&k| has(mask, pc(k)) && !has(skip, pc(k)))
}

/// The four-part pitch-class set a 4-way / Block voicing uses, as a mask.
///
/// - `sixth`: triads gain the 6th (major, minor) or diminished 7th; otherwise the 7th
///   (maj7 on major, m7 on minor). Augmented and sus chords gain the b7 either way.
/// - `ninth`: the 9th replaces the root (not on diminished chords, whose 9th clashes).
/// - Five-note chords drop the 5th; two- and one-note chords (1+5, 1+8) stay as they are.
/// - When the melody sits a semitone above a set tone (C over B in Cmaj7) that tone becomes
///   the one a minor 3rd under the melody (A), as a 6th chord would voice it.
fn four_part(chord: Chord, melody: u8, sixth: bool, ninth: bool) -> u16 {
    let rel = rel_mask_at(chord_tones(chord.ty), 0);
    let r = |i: u8| has(rel, i);
    let mut set = rel;
    let count = rel.count_ones();
    if count == 3 {
        let third = if r(4) { Some(4) } else if r(3) { Some(3) } else { None };
        let add = match (third, r(7), r(8), r(6)) {
            (Some(3), _, _, true) => 9,                  // dim -> dim7
            (Some(_), _, true, _) => 10,                 // aug -> aug7
            (Some(4), true, _, _) if sixth => 9,         // M -> 6
            (Some(4), true, _, _) => 11,                 // M -> M7
            (Some(3), true, _, _) if sixth => 9,         // m -> m6
            (Some(3), true, _, _) => 10,                 // m -> m7
            _ => 10,                                     // sus, (b5): add b7
        };
        set |= 1 << add;
    } else if count >= 5 {
        if r(7) {
            set &= !(1 << 7);
        } else {
            set &= !1;
        }
    }
    let dim = r(3) && r(6) && !r(7);
    // A b9 or #9 (a minor and a major 3rd together) already is the chord's 9th: the
    // rootless voicing keeps it (3-5-b7-b9, 3-5-b7-#9) instead of adding a natural 9th.
    let altered_ninth = r(1) || (r(3) && r(4));
    if ninth && altered_ninth {
        set = rel & !1;
    } else if ninth && !dim && set.count_ones() >= 4 {
        set = (set & !1) | 1 << 2;
    }
    let mut abs = rel_mask_at_mask(set, chord.root);
    let m = melody % 12;
    let under = (m + 11) % 12;
    if has(abs, m) && has(abs, under) {
        abs = (abs & !(1 << under)) | 1 << ((m + 9) % 12);
    }
    abs
}

fn rel_mask_at_mask(rel: u16, root: u8) -> u16 {
    let by = root % 12;
    ((rel << by) | (rel >> ((12 - by) % 12))) & 0x0FFF
}

/// The chord's own tones reduced to at most four (five-note chords drop the 5th).
fn own_four(chord: Chord) -> u16 {
    let rel = rel_mask_at(chord_tones(chord.ty), 0);
    let set = if rel.count_ones() >= 5 { rel & !(1 << 7) } else { rel };
    rel_mask_at_mask(set, chord.root)
}

/// `count` notes of `set` in close position below the melody. A melody on a chord tone
/// (of the set or of the chord itself) may have a whole step under it; a passing melody
/// note keeps a minor 3rd clear, so the block reads as a reharmonised tension.
fn close(melody: i16, set: u16, chord: u16, count: usize, out: &mut Voicing) {
    let gap = if has(set | chord, pc(melody)) { CLOSE_GAP } else { DUET_GAP };
    below(melody, set, count, gap, out);
}

/// Drop the `drops` voices (1-based from the top, melody = 1) of a close voicing an octave.
/// A drop that would put a minor 9th against another voice (the melody included) is
/// skipped, so a close semitone such as B-C in Cmaj7 never opens into a b9.
fn drop_voices(v: &mut Voicing, melody: i16, drops: &[usize]) {
    let n = (v.len as usize).min(MAX_NOTES);
    let close = v.keys;
    for &d in drops {
        // Harmony index d-2 is voice d (voice 1 is the melody).
        let Some(i) = d.checked_sub(2).filter(|&i| i < n) else { continue };
        let Some(k) = (close[i] as i16).checked_sub(12).filter(|&k| k >= 0) else { continue };
        let b9 = |o: i16| (o - k).abs() == 13;
        if b9(melody) || (0..n).any(|j| j != i && b9(v.keys[j] as i16)) {
            continue;
        }
        v.keys[i] = k as u8;
    }
    v.sort_desc();
}

/// The harmony keys for `melody` under `chord`, highest first. Pure and deterministic.
/// Types that need a chord return nothing without one; Multi Assign and the Echo category
/// add no pitches (see [`MultiAssign`] and [`EchoGen`]).
pub fn voice(ty: HarmonyType, melody: u8, chord: Option<Chord>) -> Voicing {
    let mut v = Voicing::EMPTY;
    let m = melody as i16;
    match ty {
        T::OnePlusFive => {
            v.push(m + 7);
            return v;
        }
        T::Octave => {
            v.push(m - 12);
            return v;
        }
        T::MultiAssign | T::Echo | T::Tremolo | T::Trill => return v,
        _ => {}
    }
    let Some(chord) = chord.filter(|c| c.ty != CANCEL) else { return v };
    let mask = chord_mask(chord);
    match ty {
        T::StandardDuet1 => below(m, mask, 1, DUET_GAP, &mut v),
        T::StandardDuet2 => {
            let mut two = Voicing::EMPTY;
            below(m, mask, 2, DUET_GAP, &mut two);
            if let Some(&k) = two.keys().get(1).or(two.keys().first()) {
                v.push(k as i16);
            }
        }
        T::StandardTrio => below(m, mask, 2, DUET_GAP, &mut v),
        T::FullChord => {
            let set = own_four(chord);
            close(m, set, mask, 4, &mut v);
            // A root under the lowest note for body.
            if let Some(&low) = v.keys().last() {
                let root = (1..=12).map(|d| low as i16 - d).find(|&k| pc(k) == chord.root % 12);
                if let Some(k) = root {
                    v.push(k);
                }
            }
        }
        T::RockDuet => {
            // Root or fifth only: a power-chord harmony.
            let rel = chord_tones(chord.ty);
            let fifth = [7u8, 6, 8].into_iter().find(|f| rel.contains(f));
            let mut set = 1u16 << (chord.root % 12);
            if let Some(f) = fifth {
                set |= 1 << ((chord.root % 12 + f) % 12);
            }
            below(m, set, 1, DUET_GAP, &mut v);
        }
        T::CountryDuet1 => {
            if let Some(k) = nearest_above(m, mask, DUET_GAP) {
                v.push(k);
            }
        }
        T::CountryDuet2 => {
            // The tenor line an octave down (baritone register), else the nearest below.
            match nearest_above(m, mask, DUET_GAP).map(|k| k - 12).filter(|&k| m - k >= DUET_GAP) {
                Some(k) => v.push(k),
                None => below(m, mask, 1, DUET_GAP, &mut v),
            }
        }
        T::CountryTrio => {
            let tenor = nearest_above(m, mask, DUET_GAP);
            let mut rest = mask;
            if let Some(t) = tenor {
                v.push(t);
                rest &= !(1 << pc(t));
            }
            below(m, rest, 1, DUET_GAP, &mut v);
        }
        T::Block => {
            close(m, four_part(chord, melody, true, false), mask, 3, &mut v);
            v.push(m - 12);
        }
        T::FourWayClose1 => close(m, four_part(chord, melody, true, false), mask, 3, &mut v),
        T::FourWayClose2 => close(m, four_part(chord, melody, false, false), mask, 3, &mut v),
        T::FourWayClose3 => close(m, four_part(chord, melody, true, true), mask, 3, &mut v),
        T::FourWayClose4 => {
            close(m, four_part(chord, melody, false, false), mask, 3, &mut v);
            v.push(m - 12);
        }
        T::FourWayOpen1 => {
            close(m, four_part(chord, melody, true, false), mask, 3, &mut v);
            drop_voices(&mut v, m, &[2]);
        }
        T::FourWayOpen2 => {
            close(m, four_part(chord, melody, true, false), mask, 3, &mut v);
            drop_voices(&mut v, m, &[3]);
        }
        T::FourWayOpen3 => {
            close(m, four_part(chord, melody, true, false), mask, 3, &mut v);
            drop_voices(&mut v, m, &[2, 4]);
        }
        T::Strum => {
            close(m, own_four(chord), mask, 3, &mut v);
            v.sort_desc();
            let n = (v.len as usize).min(MAX_NOTES);
            for i in 0..n {
                v.delay_ms[i] = STRUM_STEP_MS * (i as u16 + 1);
            }
            return v;
        }
        T::OnePlusFive | T::Octave | T::MultiAssign | T::Echo | T::Tremolo | T::Trill => {}
    }
    v.sort_desc();
    v
}

// ---------------------------------------------------------------------------
// Detail settings and routing
// ---------------------------------------------------------------------------

/// One added note: key, velocity and the Right parts it sounds on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HarmonyNote {
    pub key: u8,
    pub vel: u8,
    pub parts: PartMask,
    /// Start this long after the melody note (Strum).
    pub delay_ms: u16,
}

/// What one melody note-on produces: where the melody sounds and the added notes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Harmony {
    /// The Right parts the melody key itself sounds on.
    pub melody_parts: PartMask,
    notes: [HarmonyNote; MAX_NOTES],
    len: u8,
}

impl Harmony {
    pub const NONE: Harmony = Harmony { melody_parts: 0, notes: [HarmonyNote { key: 0, vel: 0, parts: 0, delay_ms: 0 }; MAX_NOTES], len: 0 };

    pub fn notes(&self) -> &[HarmonyNote] {
        &self.notes[..(self.len as usize).min(MAX_NOTES)]
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// Velocity of an effect note: the key velocity scaled by Volume (127 = unchanged). Zero
/// means silent.
pub fn effect_velocity(vel: u8, volume: u8) -> u8 {
    if vel == 0 || volume == 0 {
        return 0;
    }
    let v = (vel.min(127) as u32 * volume.min(127) as u32 + 63) / 127;
    v.clamp(1, 127) as u8
}

/// Routing for one note-on: the melody's parts and the part mask of each effect note.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Routing {
    pub melody: PartMask,
    pub effect: [PartMask; MAX_NOTES],
}

/// Which Right parts the melody and each of `n` effect notes sound on (RM p.46).
///
/// - Auto: the melody on every Right part that is on; the effect on the first eligible
///   part in the order Right 1, 2, 3.
/// - Multi: with two or more eligible parts, the melody on the first; effect note `i` on
///   the other eligible parts in turn, then back round to the first. With one part, all of
///   it there.
/// - Right1/2/3: the effect on that part, if it is on and eligible.
///
/// Eligible = on, and in the Harmony category not Mono.
pub fn route(assign: Assign, category: Category, parts: RightParts, n: usize) -> Routing {
    let on = parts.mask();
    let ok = parts.eligible(category);
    let first = ok & ok.wrapping_neg();
    let mut r = Routing { melody: on, effect: [0; MAX_NOTES] };
    match assign {
        Assign::Auto => r.effect = [first; MAX_NOTES],
        Assign::Multi => {
            if ok.count_ones() >= 2 {
                r.melody = first | (on & !ok);
                let mut order = [0u8; 3];
                let mut len = 0;
                for i in 0..3 {
                    let b = 1u8 << i;
                    if ok & b != 0 && b != first {
                        order[len] = b;
                        len += 1;
                    }
                }
                order[len] = first;
                len += 1;
                for (i, e) in r.effect.iter_mut().enumerate().take(n.min(MAX_NOTES)) {
                    *e = order[i % len];
                }
            } else {
                r.effect = [ok; MAX_NOTES];
            }
        }
        Assign::Right1 | Assign::Right2 | Assign::Right3 => {
            let b = match assign {
                Assign::Right1 => 1 << RIGHT1,
                Assign::Right2 => 1 << RIGHT2,
                _ => 1 << RIGHT3,
            };
            r.effect = [ok & b; MAX_NOTES];
        }
    }
    r
}

/// The Harmony-category effect for one melody note-on: the voicing, filtered by Chord Note
/// Only and Minimum Velocity, scaled by Volume and routed by Assign. `chord` is the one
/// [`harmony_chord`] picked. The Echo category and Multi Assign return no notes here.
pub fn harmonize(melody: u8, vel: u8, chord: Option<Chord>, s: &HarmonySettings, parts: RightParts) -> Harmony {
    let mut h = Harmony { melody_parts: parts.mask(), ..Harmony::NONE };
    let ty = s.ty;
    if ty.category() != Category::Harmony || !ty.has_details() || melody > 127 {
        return h;
    }
    if vel == 0 || vel < s.min_velocity {
        return h;
    }
    let chord = chord.filter(|c| c.ty != CANCEL);
    if s.chord_note_only && ty.uses_chord() {
        match chord {
            Some(c) if is_chord_note(melody, c) => {}
            _ => return h,
        }
    }
    let v = voice(ty, melody, chord);
    let hv = effect_velocity(vel, s.volume);
    if v.is_empty() || hv == 0 {
        return h;
    }
    let r = route(s.assign, Category::Harmony, parts, v.keys().len());
    h.melody_parts = r.melody;
    for (i, (&k, &d)) in v.keys().iter().zip(v.delays_ms()).enumerate() {
        if r.effect[i] == 0 {
            continue;
        }
        let j = h.len as usize;
        if j < MAX_NOTES {
            h.notes[j] = HarmonyNote { key: k, vel: hv, parts: r.effect[i], delay_ms: d };
            h.len += 1;
        }
    }
    h
}

/// The melody key when several right-hand keys are down: the highest one.
pub fn melody_of(held: &[u8]) -> Option<u8> {
    held.iter().copied().max()
}

// ---------------------------------------------------------------------------
// Note-off bookkeeping
// ---------------------------------------------------------------------------

/// What each held melody key added, so its release stops exactly those notes even if the
/// chord or settings changed meanwhile. Fixed size, one slot per MIDI key.
pub struct HarmonyTracker {
    slots: [Harmony; 128],
}

impl Default for HarmonyTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl HarmonyTracker {
    pub const fn new() -> Self {
        HarmonyTracker { slots: [Harmony::NONE; 128] }
    }

    /// Record `h` for `melody`. Returns what the key held before (a re-press without a
    /// release), which the caller should stop first.
    pub fn press(&mut self, melody: u8, h: Harmony) -> Option<Harmony> {
        let slot = self.slots.get_mut(melody as usize)?;
        let old = std::mem::replace(slot, h);
        (!old.is_empty()).then_some(old)
    }

    /// Forget `melody` and return what it added.
    pub fn release(&mut self, melody: u8) -> Option<Harmony> {
        let slot = self.slots.get_mut(melody as usize)?;
        let old = std::mem::replace(slot, Harmony::NONE);
        (!old.is_empty()).then_some(old)
    }

    /// Everything still sounding, for an all-notes-off; clears the tracker.
    pub fn release_all(&mut self, mut f: impl FnMut(u8, &Harmony)) {
        for (k, slot) in self.slots.iter_mut().enumerate() {
            if !slot.is_empty() {
                f(k as u8, slot);
                *slot = Harmony::NONE;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Multi Assign
// ---------------------------------------------------------------------------

/// Multi Assign (OM p.57): right-hand keys go to Right 1, Right 2, Right 3 in the order
/// pressed. Each new key takes the first part (in R1, R2, R3 order, among the parts that
/// are on) that no held key is using; when all are busy it wraps by the number held.
pub struct MultiAssign {
    part_of: [u8; 128],
    busy: [u8; 3],
}

const NO_PART: u8 = 0xFF;

impl Default for MultiAssign {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiAssign {
    pub const fn new() -> Self {
        MultiAssign { part_of: [NO_PART; 128], busy: [0; 3] }
    }

    /// The part mask `key` sounds on; 0 when no Right part is on.
    pub fn press(&mut self, key: u8, parts: RightParts) -> PartMask {
        let Some(&prev) = self.part_of.get(key as usize) else { return 0 };
        if prev != NO_PART {
            self.release(key);
        }
        let mut on = [0usize; 3];
        let mut n = 0;
        for i in 0..3 {
            if parts.on[i] {
                on[n] = i;
                n += 1;
            }
        }
        if n == 0 {
            return 0;
        }
        let held: u32 = self.busy.iter().map(|&b| b as u32).sum();
        let part = on[..n].iter().copied().find(|&p| self.busy[p] == 0).unwrap_or(on[held as usize % n]);
        self.busy[part] = self.busy[part].saturating_add(1);
        self.part_of[key as usize] = part as u8;
        1 << part
    }

    /// The part mask `key` was sounding on (0 if it was not held).
    pub fn release(&mut self, key: u8) -> PartMask {
        let Some(slot) = self.part_of.get_mut(key as usize) else { return 0 };
        let p = std::mem::replace(slot, NO_PART);
        match self.busy.get_mut(p as usize) {
            Some(b) => {
                *b = b.saturating_sub(1);
                1 << p
            }
            None => 0,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

// ---------------------------------------------------------------------------
// Echo category: timing generator
// ---------------------------------------------------------------------------

/// Held keys the generator tracks at once; a further key replaces the oldest.
pub const MAX_ECHO_VOICES: usize = 16;
const PENDING: usize = 64;
const NEVER: u64 = u64::MAX;
/// Each echo repeat is this fraction of the one before (Echo only).
const ECHO_DECAY: (u32, u32) = (3, 4);
/// Echo repeats stop once they would fall below this velocity.
const ECHO_FLOOR: u8 = 4;
/// Note length as a fraction of the repeat period.
const PULSE_GATE: (u64, u64) = (3, 4);
const TRILL_GATE: (u64, u64) = (9, 10);

/// One note event from [`EchoGen`]. `vel == 0` is a note-off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EchoEvent {
    pub at: u64,
    pub key: u8,
    pub vel: u8,
    /// False for the note the player struck, true for a generated repeat. The engine routes
    /// them with [`route`]: with Assign = Multi the struck note goes to the melody parts
    /// and the repeats to the effect parts.
    pub effect: bool,
}

#[derive(Debug, Clone, Copy)]
struct Voice {
    key: u8,
    vel: u8,
    active: bool,
    seq: u32,
    next_on: u64,
    off_at: u64,
    sounding: bool,
    count: u32,
    level: u8,
}

const IDLE_VOICE: Voice = Voice {
    key: 0, vel: 0, active: false, seq: 0, next_on: NEVER, off_at: NEVER,
    sounding: false, count: 0, level: 0,
};

#[derive(Debug, Clone, Copy)]
struct TrillState {
    /// Voice indices: `a` the older key, `b` the newer.
    a: usize,
    b: usize,
    next_on: u64,
    off_at: u64,
    /// The key sounding now, if any.
    sounding: Option<u8>,
    /// Next pulse plays `b` when true.
    to_b: bool,
    count: u32,
}

/// The Echo, Tremolo and Trill effects as a timing generator (OM p.57). The generator owns
/// every note of the right-hand keys it is given, the struck ones included: feed it key
/// presses and releases, and poll [`EchoGen::next_events`] at least by [`EchoGen::next_due`].
///
/// - **Echo:** the key sounds at once, then repeats every Speed note value while held, each
///   repeat quieter (×3/4, starting from Volume), until it fades out.
/// - **Tremolo:** the same at a steady level (Volume).
/// - **Trill:** with two or more keys held, the last two alternate every Speed note value,
///   starting with the newer key; other held keys fall silent. A lone key just sounds.
/// - Keys below Minimum Velocity sound plainly with no repeats.
///
/// Repeats run from the key press, not from the bar. No allocation after construction.
pub struct EchoGen {
    ty: HarmonyType,
    speed: EchoSpeed,
    volume: u8,
    min_velocity: u8,
    bpm: f64,
    period: u64,
    voices: [Voice; MAX_ECHO_VOICES],
    seq: u32,
    trill: Option<TrillState>,
    pending: [EchoEvent; PENDING],
    p_head: usize,
    p_len: usize,
}

impl EchoGen {
    /// A generator for `s.ty` (Echo, Tremolo or Trill; anything else reads as Echo) at `bpm`.
    pub fn new(s: &HarmonySettings, bpm: f64) -> Self {
        let ty = if s.ty.category() == Category::Echo { s.ty } else { T::Echo };
        EchoGen {
            ty,
            speed: s.speed,
            volume: s.volume,
            min_velocity: s.min_velocity,
            bpm,
            period: s.speed.period_ns(bpm),
            voices: [IDLE_VOICE; MAX_ECHO_VOICES],
            seq: 0,
            trill: None,
            pending: [EchoEvent::default(); PENDING],
            p_head: 0,
            p_len: 0,
        }
    }

    pub fn ty(&self) -> HarmonyType {
        self.ty
    }

    /// The keys the generator holds (pressed and not yet released). The live wiring checks
    /// them against the keys really down, so a lost key-up can never leave one repeating.
    pub fn keys_down(&self) -> impl Iterator<Item = u8> + '_ {
        self.voices.iter().filter(|v| v.active).map(|v| v.key)
    }

    /// The current repeat period in nanoseconds.
    pub fn period(&self) -> u64 {
        self.period
    }

    /// Change tempo. Repeats already scheduled keep their time; the ones after use the new
    /// period.
    pub fn set_tempo(&mut self, bpm: f64) {
        self.bpm = bpm;
        self.period = self.speed.period_ns(bpm);
    }

    /// Change Speed, Volume and Minimum Velocity (the type stays; make a new generator for
    /// a new type, after [`EchoGen::all_off`]).
    pub fn set_settings(&mut self, s: &HarmonySettings) {
        self.speed = s.speed;
        self.volume = s.volume;
        self.min_velocity = s.min_velocity;
        self.period = s.speed.period_ns(self.bpm);
    }

    fn emit(&mut self, e: EchoEvent) {
        // A note-off whose note-on has not been handed out yet cancels it, so a burst of
        // presses and releases between two polls cannot fill the queue and lose note-offs.
        // What remains is bounded by the voices (≤ 17 note-ons plus ≤ 17 note-offs).
        if e.vel == 0 {
            for back in (0..self.p_len).rev() {
                let i = (self.p_head + back) % PENDING;
                let q = self.pending[i];
                if q.key != e.key {
                    continue;
                }
                if q.vel > 0 {
                    for j in back..self.p_len - 1 {
                        let (a, b) = ((self.p_head + j) % PENDING, (self.p_head + j + 1) % PENDING);
                        self.pending[a] = self.pending[b];
                    }
                    self.p_len -= 1;
                    return;
                }
                break;
            }
        }
        if self.p_len < PENDING {
            let i = (self.p_head + self.p_len) % PENDING;
            self.pending[i] = e;
            self.p_len += 1;
        }
    }

    fn pop(&mut self) -> Option<EchoEvent> {
        if self.p_len == 0 {
            return None;
        }
        let e = self.pending[self.p_head];
        self.p_head = (self.p_head + 1) % PENDING;
        self.p_len -= 1;
        Some(e)
    }

    fn find(&self, key: u8) -> Option<usize> {
        self.voices.iter().position(|v| v.active && v.key == key)
    }

    fn silence_voice(&mut self, i: usize, now: u64) {
        let Some(v) = self.voices.get_mut(i) else { return };
        let (key, was) = (v.key, v.sounding);
        v.sounding = false;
        v.off_at = NEVER;
        v.next_on = NEVER;
        if was {
            let effect = v.count > 1;
            self.emit(EchoEvent { at: now, key, vel: 0, effect });
        }
    }

    fn stop_trill(&mut self, now: u64) {
        if let Some(t) = self.trill.take()
            && let Some(k) = t.sounding
        {
            // Same flag as the note-on it ends (the trill's first note is the struck one).
            self.emit(EchoEvent { at: now, key: k, vel: 0, effect: t.count > 1 });
        }
    }

    /// Recompute the trill after a press or release: the last two held (eligible) keys
    /// alternate; a lone key sounds plainly.
    fn retrill(&mut self, now: u64, newest_first: bool) {
        let mut top: [Option<(u32, usize)>; 2] = [None, None];
        for (i, v) in self.voices.iter().enumerate() {
            if !v.active || v.vel < self.min_velocity {
                continue;
            }
            let e = Some((v.seq, i));
            if top[0].is_none_or(|t| v.seq > t.0) {
                top[1] = top[0];
                top[0] = e;
            } else if top[1].is_none_or(|t| v.seq > t.0) {
                top[1] = e;
            }
        }
        let pair = match (top[0], top[1]) {
            (Some((_, b)), Some((_, a))) => Some((a, b)),
            _ => None,
        };
        if let (Some((a, b)), Some(t)) = (pair, self.trill)
            && t.a == a
            && t.b == b
        {
            return;
        }
        self.stop_trill(now);
        match pair {
            Some((a, b)) => {
                // Everything else held falls silent while the trill runs.
                for i in 0..MAX_ECHO_VOICES {
                    if self.voices[i].active {
                        self.silence_voice(i, now);
                    }
                }
                self.trill = Some(TrillState { a, b, next_on: now, off_at: NEVER, sounding: None, to_b: newest_first, count: 0 });
            }
            None => {
                if let Some((_, i)) = top[0] {
                    let v = &mut self.voices[i];
                    if !v.sounding {
                        v.sounding = true;
                        let (key, vel) = (v.key, v.vel);
                        self.emit(EchoEvent { at: now, key, vel, effect: false });
                    }
                }
            }
        }
    }

    /// A right-hand key went down.
    pub fn note_on(&mut self, key: u8, vel: u8, now: u64) {
        if key > 127 {
            return;
        }
        if vel == 0 {
            self.note_off(key, now);
            return;
        }
        if self.find(key).is_some() {
            self.note_off(key, now);
        }
        let slot = match self.voices.iter().position(|v| !v.active) {
            Some(i) => i,
            None => {
                // Full: drop the oldest key.
                let oldest = self.voices.iter().enumerate().min_by_key(|(_, v)| v.seq).map(|(i, _)| i).unwrap_or(0);
                let k = self.voices[oldest].key;
                self.note_off(k, now);
                oldest
            }
        };
        self.seq = self.seq.wrapping_add(1);
        let plain = vel < self.min_velocity;
        self.voices[slot] = Voice {
            key, vel, active: true, seq: self.seq, next_on: NEVER, off_at: NEVER,
            sounding: false, count: 0, level: vel,
        };
        if self.ty == T::Trill {
            if plain {
                self.voices[slot].sounding = true;
                self.emit(EchoEvent { at: now, key, vel, effect: false });
            } else {
                self.retrill(now, true);
            }
            return;
        }
        let v = &mut self.voices[slot];
        if plain {
            v.sounding = true;
            self.emit(EchoEvent { at: now, key, vel, effect: false });
        } else {
            v.next_on = now;
        }
    }

    /// A right-hand key went up.
    pub fn note_off(&mut self, key: u8, now: u64) {
        let Some(i) = self.find(key) else { return };
        self.silence_voice(i, now);
        self.voices[i] = IDLE_VOICE;
        if self.ty == T::Trill {
            let in_pair = self.trill.is_some_and(|t| t.a == i || t.b == i);
            if in_pair {
                self.stop_trill(now);
                self.retrill(now, true);
            }
        }
    }

    /// Stop everything now (note-offs for whatever sounds) and forget the held keys.
    pub fn all_off(&mut self, now: u64) {
        self.stop_trill(now);
        for i in 0..MAX_ECHO_VOICES {
            if self.voices[i].active {
                self.silence_voice(i, now);
            }
            self.voices[i] = IDLE_VOICE;
        }
    }

    /// The earliest time something is due, if anything is.
    pub fn next_due(&self) -> Option<u64> {
        if self.p_len > 0 {
            return Some(self.pending[self.p_head].at);
        }
        let mut t = NEVER;
        for v in self.voices.iter().filter(|v| v.active) {
            t = t.min(v.next_on).min(v.off_at);
        }
        if let Some(tr) = &self.trill {
            t = t.min(tr.next_on).min(tr.off_at);
        }
        (t != NEVER).then_some(t)
    }

    /// Write the events due at or before `now` into `out`, in time order (a note-off
    /// before a note-on at the same time), and return how many. Anything that does not fit
    /// stays for the next call.
    pub fn next_events(&mut self, now: u64, out: &mut [EchoEvent]) -> usize {
        let mut n = 0;
        while n < out.len() {
            if let Some(e) = self.pop() {
                out[n] = e;
                n += 1;
                continue;
            }
            match self.step(now) {
                Some(e) => {
                    out[n] = e;
                    n += 1;
                }
                None => break,
            }
        }
        n
    }

    /// The single earliest due event, applied to the state.
    fn step(&mut self, now: u64) -> Option<EchoEvent> {
        // Offs first: (time, is_on, which) with which = voice index or MAX for the trill.
        let mut best: Option<(u64, bool, usize)> = None;
        let mut consider = |t: u64, on: bool, w: usize| {
            if t <= now && best.is_none_or(|b| (t, on) < (b.0, b.1)) {
                best = Some((t, on, w));
            }
        };
        for (i, v) in self.voices.iter().enumerate().filter(|(_, v)| v.active) {
            if v.sounding {
                consider(v.off_at.min(v.next_on), false, i);
            }
            consider(v.next_on, true, i);
        }
        if let Some(t) = &self.trill {
            if t.sounding.is_some() {
                consider(t.off_at.min(t.next_on), false, usize::MAX);
            }
            consider(t.next_on, true, usize::MAX);
        }
        let (at, on, w) = best?;
        if w == usize::MAX {
            let t = self.trill.as_mut()?;
            if !on {
                let key = t.sounding.take()?;
                t.off_at = NEVER;
                return Some(EchoEvent { at, key, vel: 0, effect: t.count > 1 });
            }
            let idx = if t.to_b { t.b } else { t.a };
            let v = self.voices.get(idx)?;
            let effect = t.count > 0;
            let vel = if effect { effect_velocity(v.vel, self.volume).max(1) } else { v.vel };
            t.sounding = Some(v.key);
            t.to_b = !t.to_b;
            t.count = t.count.saturating_add(1);
            t.off_at = at.saturating_add(self.period / TRILL_GATE.1 * TRILL_GATE.0);
            t.next_on = at.saturating_add(self.period);
            return Some(EchoEvent { at, key: v.key, vel, effect });
        }
        let period = self.period;
        let (ty, volume) = (self.ty, self.volume);
        let v = self.voices.get_mut(w)?;
        if !on {
            v.sounding = false;
            v.off_at = NEVER;
            return Some(EchoEvent { at, key: v.key, vel: 0, effect: v.count > 1 });
        }
        let effect = v.count > 0;
        let vel = pulse_velocity(ty, v.vel, v.level, v.count, volume);
        v.count = v.count.saturating_add(1);
        v.level = vel;
        v.sounding = true;
        v.off_at = at.saturating_add(period / PULSE_GATE.1 * PULSE_GATE.0);
        // Schedule the next repeat only if it will be heard (Echo fades out; Volume 0
        // leaves just the struck note). The key stays held, silent, until released.
        v.next_on = if pulse_velocity(ty, v.vel, vel, v.count, volume) == 0 { NEVER } else { at.saturating_add(period) };
        Some(EchoEvent { at, key: v.key, vel, effect })
    }
}

/// The velocity of pulse number `count` (0 = the struck note) of a key struck at `vel`,
/// the previous pulse having been `prev`. Zero means silent: Volume 0, or an Echo repeat
/// that has faded below [`ECHO_FLOOR`].
fn pulse_velocity(ty: HarmonyType, vel: u8, prev: u8, count: u32, volume: u8) -> u8 {
    match count {
        0 => vel,
        1 => effect_velocity(vel, volume),
        _ if ty != T::Echo => effect_velocity(vel, volume),
        _ => {
            let v = (prev as u32 * ECHO_DECAY.0 / ECHO_DECAY.1) as u8;
            if v < ECHO_FLOOR { 0 } else { v }
        }
    }
}

#[cfg(test)]
mod tests;
