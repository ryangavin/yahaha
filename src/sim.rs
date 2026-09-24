//! Offline driver: runs the engine with simulated time and records what it sends.

use crate::engine::{slot_of, Button, Engine, Prepared, Sink, Transpose};
use crate::sff::SectionId;
use crate::sff::Style;
use crate::theory::{Chord, NOTE_NAMES};
use anyhow::{anyhow, bail, Result};

#[derive(Default)]
pub struct Recorder {
    pub now: u64,
    pub out: Vec<(u64, Vec<u8>)>,
    /// Retrigger Rule pitch shifts: (index into `out` they apply from, time, channel, semitones).
    pub retunes: Vec<(usize, u64, u8, i8)>,
}

impl Sink for Recorder {
    fn send(&mut self, msg: &[u8]) {
        self.out.push((self.now, msg.to_vec()));
    }

    fn retune(&mut self, ch: u8, semis: i8) {
        self.retunes.push((self.out.len(), self.now, ch, semis));
    }
}

impl Recorder {
    /// Every note-on as (time, channel, pitch that sounds): the key sent plus the pitch
    /// shift its channel was bent by at the time.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn pitch_ons(&self) -> Vec<(u64, u8, u8)> {
        let mut bend = [0i8; 16];
        let mut r = self.retunes.iter().peekable();
        let mut v = Vec::new();
        for (i, (t, m)) in self.out.iter().enumerate() {
            while let Some(&&(at, _, ch, semis)) = r.peek()
                && at <= i
            {
                r.next();
                bend[ch as usize] = semis;
            }
            if m[0] & 0xF0 == 0x90 && m[2] > 0 {
                let ch = m[0] & 0x0F;
                v.push((*t, ch, (m[1] as i32 + bend[ch as usize] as i32) as u8));
            }
        }
        v
    }

    /// The notes sounding at time `t` as (channel, pitch that sounds), sorted.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn sounding_at(&self, t: u64) -> Vec<(u8, u8)> {
        let mut bend = [0i8; 16];
        let mut held = std::collections::BTreeMap::<(u8, u8), i32>::new();
        let mut r = self.retunes.iter().peekable();
        for (i, (at, m)) in self.out.iter().enumerate() {
            while let Some(&&(idx, when, ch, semis)) = r.peek()
                && idx <= i
                && when <= t
            {
                r.next();
                bend[ch as usize] = semis;
            }
            if *at > t {
                break;
            }
            if m[0] & 0xE0 == 0x80 {
                let n = held.entry((m[0] & 0xF, m[1])).or_default();
                *n = if m[0] & 0xF0 == 0x90 && m[2] > 0 { *n + 1 } else { 0 };
            }
        }
        // Shifts after the last message.
        for &(_, _, ch, semis) in r.filter(|r| r.1 <= t) {
            bend[ch as usize] = semis;
        }
        let mut v: Vec<(u8, u8)> = held
            .into_iter()
            .filter(|&(_, n)| n > 0)
            .map(|((ch, key), _)| (ch, (key as i32 + bend[ch as usize] as i32) as u8))
            .collect();
        v.sort();
        v
    }
}

#[derive(Clone, Copy)]
pub enum Step {
    Chord(Chord),
    /// Every chord-section key let go (Sync Stop listens for this).
    Release,
    Button(Button),
    /// Not in the script grammar yet; only the Manual Bass tests use it.
    #[cfg_attr(not(test), allow(dead_code))]
    ManualBass(bool),
    /// Not in the script grammar yet; only the transpose tests use it.
    #[cfg_attr(not(test), allow(dead_code))]
    Transpose(Transpose),
}

/// Run a script of (time_ns, step) against the engine until `end_ns`.
#[cfg_attr(not(test), allow(dead_code))]
pub fn run(style: Box<Prepared>, script: &[(u64, Step)], end_ns: u64) -> (Engine, Recorder) {
    run_observed(style, script, end_ns, |_, _| {})
}

/// `run`, calling `observe` after every engine wake-up (section changes land on one).
pub fn run_observed(
    style: Box<Prepared>,
    script: &[(u64, Step)],
    end_ns: u64,
    mut observe: impl FnMut(&Engine, u64),
) -> (Engine, Recorder) {
    let mut e = Engine::new(style);
    let mut rec = Recorder::default();
    let mut i = 0;
    let mut now = 0u64;
    loop {
        let next_script = script.get(i).map(|s| s.0);
        let next_engine = e.next_deadline();
        let t = [next_script, next_engine, Some(end_ns)].into_iter().flatten().min().unwrap();
        now = now.max(t);
        rec.now = now;
        // Script steps at this instant happen before the engine plays what is due.
        while let Some((ts, step)) = script.get(i) {
            if *ts > now {
                break;
            }
            match step {
                Step::Chord(c) => e.set_chord(*c, now, &mut rec),
                Step::Release => e.chord_released(now, &mut rec),
                Step::Button(b) => e.button(*b, now, &mut rec),
                Step::ManualBass(on) => e.set_manual_bass(*on, &mut rec),
                Step::Transpose(t) => e.set_transpose(*t, now, &mut rec),
            }
            i += 1;
        }
        e.process(now, &mut rec);
        observe(&e, now);
        if now >= end_ns {
            break;
        }
    }
    (e, rec)
}

// ---------------------------------------------------------------------------
// Scripts and snapshots (`yahaha sim`, tests/golden)
// ---------------------------------------------------------------------------

/// One script step, at a tick counted from the start of bar 1, with the text shown for it.
#[derive(Clone)]
pub struct ScriptStep {
    pub tick: u32,
    pub step: Step,
    pub label: String,
}

/// Parse a performance script (grammar in tests/golden/README.md). `|` separates bars; the
/// chords, `-` holds and `^` releases in a bar share it equally; `[Button]` tokens take no
/// time and press at the next slot. Without any `|`, every chord is its own bar ("C Am F G7").
/// Returns the steps and the number of bars.
pub fn parse_script(text: &str, tpb: u32) -> Result<(Vec<ScriptStep>, u32)> {
    // A token starting with '#' comments out the rest of the line ("F#m7" is a chord).
    let mut tokens: Vec<&str> = Vec::new();
    for (i, l) in text.lines().enumerate() {
        let line: Vec<&str> = l.split_whitespace().take_while(|t| !t.starts_with('#')).collect();
        // "| |" on one line would silently drop a bar and shift every later one. (A line
        // ending in '|' followed by one starting with '|' is the normal layout.)
        if line.windows(2).any(|w| w == ["|", "|"]) {
            bail!("line {}: empty bar \"| |\"; write \"| - |\" to hold the chord for a bar", i + 1);
        }
        tokens.extend(line);
    }
    if !tokens.contains(&"|") {
        tokens = tokens.into_iter().flat_map(|t| if t.starts_with('[') { vec![t] } else { vec![t, "|"] }).collect();
    }
    let mut steps = Vec::new();
    let mut bars = 0u32;
    let mut carried: Vec<&str> = Vec::new();
    for bar in tokens.split(|t| *t == "|") {
        let slots = bar.iter().filter(|t| !t.starts_with('[')).count() as u32;
        if slots == 0 {
            // Buttons between bars ("| [MainB] |") press at the start of the next bar; a bar
            // needs at least one chord or "-".
            carried.extend_from_slice(bar);
            continue;
        }
        let start = bars * tpb;
        let mut slot = 0;
        for &t in carried.drain(..).chain(bar.iter().copied()).collect::<Vec<_>>().iter() {
            // A button after the last slot presses at the start of the next bar.
            let tick = start + slot * tpb / slots;
            if let Some(name) = t.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
                let b = parse_button(name).ok_or_else(|| anyhow!("unknown button [{name}]"))?;
                steps.push(ScriptStep { tick, step: Step::Button(b), label: t.to_string() });
                continue;
            }
            if t == "^" {
                steps.push(ScriptStep { tick, step: Step::Release, label: t.to_string() });
            } else if t != "-" {
                let c = crate::parse_chord(t)?;
                steps.push(ScriptStep { tick, step: Step::Chord(c), label: t.to_string() });
            }
            slot += 1;
        }
        bars += 1;
    }
    if let Some(t) = carried.first() {
        bail!("{t} after the last bar never plays");
    }
    Ok((steps, bars))
}

fn parse_button(name: &str) -> Option<Button> {
    let letter = |s: &str| match s {
        "A" => Some(0),
        "B" => Some(1),
        "C" => Some(2),
        "D" => Some(3),
        _ => None,
    };
    Some(match name {
        "Break" => Button::Break,
        "AutoFill" => Button::AutoFill,
        "StartStop" => Button::StartStop,
        "Stop" => Button::Stop,
        "SyncStart" => Button::SyncStart,
        "SyncStop" => Button::SyncStop,
        "StopAcmp" => Button::StopAcmp,
        _ => {
            if let Some(l) = name.strip_prefix("Intro") {
                Button::Intro(letter(l)?)
            } else if let Some(l) = name.strip_prefix("Main") {
                Button::Main(letter(l)?)
            } else if let Some(l) = name.strip_prefix("Ending") {
                Button::Ending(letter(l)?)
            } else {
                return None;
            }
        }
    })
}

pub const PART_NAMES: [&str; 8] = ["Rhythm1", "Rhythm2", "Bass", "Chord1", "Chord2", "Pad", "Phrase1", "Phrase2"];

/// A note a style part started: tick from the start of bar 1 (the style's ticks), channel
/// (0-based), key, and length in ticks (None: still sounding when the script ends). The key
/// is the one sent; where a Retrigger Rule pitch shift bends the channel it sounds elsewhere
/// (`Take::pitch`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayedNote {
    pub tick: u32,
    pub ch: u8,
    pub key: u8,
    pub len: Option<u32>,
}

/// A Retrigger Rule pitch shift of a sounding note (a bend, no new attack): at `tick`,
/// `Take::notes[note]` goes from pitch `from` to `to`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PitchShift {
    pub tick: u32,
    pub note: usize,
    pub from: u8,
    pub to: u8,
}

/// One script played on one style: the script steps, the section timeline and every note
/// the style parts started. `render` turns it into the listing `yahaha sim` prints.
#[derive(Clone)]
pub struct Take {
    pub name: String,
    pub bpm: f64,
    pub timesig: (u8, u8),
    pub ppq: u32,
    pub tpb: u32,
    pub bars: u32,
    pub steps: Vec<ScriptStep>,
    /// The tick each step acted at (by index). The listing shows a step there, which may be
    /// in the bar before its slot.
    pub acts: Vec<u32>,
    /// (tick, section) at every section change; None = stopped.
    pub sections: Vec<(u32, Option<SectionId>)>,
    /// as_written[slot][part]: see PSection::plays_as_written.
    as_written: Vec<[bool; 8]>,
    pub notes: Vec<PlayedNote>,
    /// The pitch each of `notes` sounds at its start (by index): its key plus the pitch
    /// shift its channel was bent by then. Empty for a take read from a recording, whose
    /// keys sound as sent.
    pub pitch: Vec<u8>,
    /// Every pitch shift of a note still sounding, in order.
    pub shifts: Vec<PitchShift>,
}

/// The tick a script step acts at in `yahaha sim` and the golden snapshots. Buttons press
/// one tick ahead of their slot, as a player would: a press exactly on a beat or bar line
/// would otherwise depend on float rounding (this beat or the next?). Chords act on it.
pub fn golden_act(s: &ScriptStep) -> u32 {
    match s.step {
        Step::Button(_) => s.tick.saturating_sub(1),
        _ => s.tick,
    }
}

/// Play a script on a style and record what every part plays (see [`Take`]).
pub fn perform(style: &Style, script: &str) -> Result<Take> {
    let (steps, bars) = parse_script(script, style.ticks_per_bar())?;
    let acts = steps.iter().map(golden_act).collect();
    Ok(perform_steps(style, steps, bars, acts))
}

/// Play parsed script steps, each at its tick in `acts` (by index), for `bars` bars.
pub fn perform_steps(style: &Style, steps: Vec<ScriptStep>, bars: u32, acts: Vec<u32>) -> Take {
    let prep = Box::new(Prepared::new(style));
    let as_written: Vec<[bool; 8]> = prep
        .sections
        .iter()
        .map(|s| s.as_ref().map_or([false; 8], |s| std::array::from_fn(|p| s.plays_as_written(8 + p as u8))))
        .collect();
    let (ppq, tpb, bpm) = (prep.ppq, prep.tpb, prep.bpm);
    let ns_per_tick = 60e9 / (bpm * ppq as f64);
    let ns = |tick: u32| (tick as f64 * ns_per_tick).ceil() as u64;
    let tick = |ns: u64| (ns as f64 / ns_per_tick).round() as u32;
    let mut timed: Vec<(u64, Step)> = steps.iter().zip(&acts).map(|(s, &t)| (ns(t), s.step)).collect();
    timed.sort_by_key(|s| s.0);

    let mut sections: Vec<(u32, Option<SectionId>)> = Vec::new();
    let mut last = None;
    let (_, rec) = run_observed(prep, &timed, ns(bars * tpb), |e, now| {
        let cur = e.snapshot(now).cur;
        if cur != last {
            sections.push((tick(now), cur));
            last = cur;
        }
    });

    // Each note-off closes the oldest open note of its key. A note sounds `bend[ch]` above
    // its key while a Retrigger Rule pitch shift bends its channel; each shift of an open
    // note is kept.
    let mut notes: Vec<PlayedNote> = Vec::new();
    let mut pitch: Vec<u8> = Vec::new();
    let mut shifts: Vec<PitchShift> = Vec::new();
    let mut bend = [0i8; 16];
    let mut retunes = rec.retunes.iter().peekable();
    for (i, (t, m)) in rec.out.iter().enumerate() {
        while let Some(&&(at, when, ch, semis)) = retunes.peek()
            && at <= i
        {
            retunes.next();
            let old = std::mem::replace(&mut bend[ch as usize], semis);
            for k in (0..notes.len()).filter(|&k| notes[k].ch == ch && notes[k].len.is_none()) {
                let key = notes[k].key as i32;
                shifts.push(PitchShift { tick: tick(when), note: k, from: (key + old as i32) as u8, to: (key + semis as i32) as u8 });
            }
        }
        let (kind, ch) = (m[0] & 0xF0, m[0] & 0x0F);
        if kind == 0x90 && m[2] > 0 {
            notes.push(PlayedNote { tick: tick(*t), ch, key: m[1], len: None });
            pitch.push((m[1] as i32 + bend[ch as usize] as i32) as u8);
        } else if (kind == 0x80 || kind == 0x90)
            && let Some(n) = notes.iter_mut().find(|n| n.ch == ch && n.key == m[1] && n.len.is_none())
        {
            n.len = Some(tick(*t) - n.tick);
        }
    }
    let name = style.name.trim_end_matches(|c: char| c.is_whitespace() || c == '\0').to_string();
    Take { name, bpm, timesig: style.timesig, ppq, tpb, bars, steps, acts, sections, as_written, notes, pitch, shifts }
}

/// A pitch as the listing writes it (Yamaha octaves, C3 = 60).
fn pitch_name(key: u8) -> String {
    format!("{}{}", NOTE_NAMES[key as usize % 12], key as i32 / 12 - 2)
}

impl Take {
    pub fn section_at(&self, t: u32) -> Option<SectionId> {
        self.sections.iter().rev().find(|(s, _)| *s <= t).and_then(|(_, id)| *id)
    }

    /// Whether the part on `ch` plays as written (whatever the chord) at tick `t`.
    pub fn as_written(&self, t: u32, ch: u8) -> bool {
        (8..16).contains(&ch) && self.section_at(t).is_some_and(|id| self.as_written[slot_of(id)][ch as usize - 8])
    }

    /// `beat.tick` within its bar.
    pub fn pos(&self, t: u32) -> String {
        let width = (self.ppq - 1).to_string().len();
        format!("{}.{:0width$}", (t % self.tpb) / self.ppq + 1, t % self.ppq)
    }

    /// A note as the listing writes it: `beat.tick Note~length`.
    pub fn note_item(&self, n: &PlayedNote) -> String {
        self.item(n, n.key)
    }

    /// `note_item` with the note sounding `pitch`.
    fn item(&self, n: &PlayedNote, pitch: u8) -> String {
        let len = n.len.map_or("…".to_string(), |l| l.to_string());
        format!("{} {}~{len}", self.pos(n.tick), pitch_name(pitch))
    }

    /// The listing's `bar` line: the section at the bar start, the section changes inside
    /// the bar, and the script steps in it.
    pub fn bar_header(&self, bar: u32) -> String {
        let name = |id: Option<SectionId>| id.map_or("stopped".into(), |c| c.name());
        let (lo, hi) = (bar * self.tpb, (bar + 1) * self.tpb);
        let mut out = format!("bar {}  {}", bar + 1, name(self.section_at(lo)));
        for (t, s) in self.sections.iter().filter(|(t, _)| *t > lo && *t < hi) {
            out += &format!(" > {}@{}", name(*s), self.pos(*t));
        }
        out + &self.steps_in(bar)
    }

    /// The script steps acting in a bar, as the listing's bar line shows them.
    fn steps_in(&self, bar: u32) -> String {
        let (lo, hi) = (bar * self.tpb, (bar + 1) * self.tpb);
        let mut out = String::new();
        for (s, &t) in self.steps.iter().zip(&self.acts).filter(|(_, t)| (lo..hi).contains(*t)) {
            out += &format!("  {}@{}", s.label, self.pos(t));
        }
        out
    }

    /// The listing's first lines: style, tempo, time signature, ppq and bar count.
    fn head(&self) -> String {
        let (num, den) = self.timesig;
        format!("style  {}  {:.0} bpm  {num}/{den}  ppq {}\nbars   {}\n", self.name, self.bpm, self.ppq, self.bars)
    }

    /// What the part on `ch` starts in a bar: the listed notes, and the number of notes it
    /// played as written (those are only counted).
    pub fn part_items(&self, bar: u32, ch: u8) -> (Vec<String>, usize) {
        let (items, written) = self.listed_notes(bar, ch, false);
        (items.into_iter().map(|i| i.1).collect(), written)
    }

    /// `part_items`, each with its tick; with `sounding`, at the pitch each note sounds at
    /// its start rather than the key sent.
    fn listed_notes(&self, bar: u32, ch: u8, sounding: bool) -> (Vec<(u32, String)>, usize) {
        let (lo, hi) = (bar * self.tpb, (bar + 1) * self.tpb);
        let (mut items, mut written) = (Vec::new(), 0);
        for (i, n) in self.notes.iter().enumerate().filter(|(_, n)| n.ch == ch && n.tick >= lo && n.tick < hi) {
            if self.as_written(n.tick, ch) {
                written += 1;
            } else {
                let pitch = if sounding { self.pitch.get(i).copied().unwrap_or(n.key) } else { n.key };
                items.push((n.tick, self.item(n, pitch)));
            }
        }
        (items, written)
    }

    /// The pitch shifts of the part on `ch` in a bar, as `beat.tick From>To`. A note that
    /// ends on the tick it is bent never sounds at the new pitch, so it is left out.
    fn shift_items(&self, bar: u32, ch: u8) -> impl Iterator<Item = (u32, String)> + '_ {
        let (lo, hi) = (bar * self.tpb, (bar + 1) * self.tpb);
        self.shifts
            .iter()
            .filter(move |s| self.notes[s.note].ch == ch && (lo..hi).contains(&s.tick))
            .filter(|s| self.notes[s.note].len.is_none_or(|l| self.notes[s.note].tick + l > s.tick))
            .map(|s| (s.tick, format!("{} {}>{}", self.pos(s.tick), pitch_name(s.from), pitch_name(s.to))))
    }

    /// The listing's line for one part in one bar (None when the part is silent there): the
    /// notes it starts, at the pitch they sound, and the pitch shifts of its notes, in time
    /// order.
    pub fn part_line(&self, bar: u32, ch: u8) -> Option<String> {
        let (mut items, written) = self.listed_notes(bar, ch, true);
        items.extend(self.shift_items(bar, ch));
        items.sort_by_key(|i| i.0);
        let mut items: Vec<String> = items.into_iter().map(|i| i.1).collect();
        if written > 0 {
            items.push(format!("{written} as written"));
        }
        (!items.is_empty()).then(|| format!("  ch{} {:<8}{}", ch + 1, PART_NAMES[ch as usize - 8], items.join("  ")))
    }

    /// The listing (format in tests/golden/README.md).
    pub fn render(&self) -> String {
        let mut out = self.head();
        for bar in 0..self.bars {
            out.push('\n');
            out += &self.bar_header(bar);
            out.push('\n');
            for ch in 8..16u8 {
                if let Some(line) = self.part_line(bar, ch) {
                    out += &line;
                    out.push('\n');
                }
            }
        }
        out
    }

    /// The listing reference captures are compared in (tests/reference). Unlike `render` it
    /// holds nothing yahaha decides: the bar line has only the script steps, because the
    /// instrument does not send its section changes and ours would be a guess, and every part
    /// lists all its notes, drums included, because which notes play as written depends on
    /// the section. Only its digest is ever kept.
    pub fn render_reference(&self) -> String {
        let mut out = self.head();
        for bar in 0..self.bars {
            let (lo, hi) = (bar * self.tpb, (bar + 1) * self.tpb);
            out += &format!("\nbar {}{}\n", bar + 1, self.steps_in(bar));
            for ch in 8..16u8 {
                let items: Vec<String> = self.notes.iter().filter(|n| n.ch == ch && n.tick >= lo && n.tick < hi).map(|n| self.note_item(n)).collect();
                if !items.is_empty() {
                    out += &format!("  ch{} {:<8}{}\n", ch + 1, PART_NAMES[ch as usize - 8], items.join("  "));
                }
            }
        }
        out
    }
}

/// Play a script on a style and list, bar by bar, the section playing, the script steps and
/// every note each part starts, as `beat.tick Note~length` (Yamaha octaves, C3 = 60; `~…` =
/// still sounding when the script ends). Positions and lengths are in the style's ticks.
/// Parts that play as written whatever the chord (drums, Root Fixed + Bypass) only get a
/// per-bar note count: listing them would copy the style's own pattern and says nothing
/// about chord following. The listing depends only on the style and the script, so it can
/// be diffed.
pub fn snapshot(style: &Style, script: &str) -> Result<String> {
    Ok(perform(style, script)?.render())
}

/// 64-bit FNV-1a. Fixed and dependency-free, so a digest means the same on every machine and
/// toolchain (std's `DefaultHasher` makes no such promise).
pub fn fnv1a(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3))
}

/// A part line's key: channel and part name (`ch11 Bass`).
pub fn part_key(line: &str) -> String {
    line.split_whitespace().take(2).collect::<Vec<_>>().join(" ")
}

/// The committed form of a listing. Lines before the first bar (style name, tempo, bar count)
/// and every `bar` header stay as they are: they come from our script. Each part line becomes
/// its key and the hash of the whole line, so the notes cannot be read back from it.
pub fn digest(listing: &str) -> String {
    let mut out = String::new();
    for line in listing.lines() {
        if line.starts_with("  ch") {
            out += &format!("  {:<12} {:016x}\n", part_key(line), fnv1a(line.trim()));
        } else {
            out += line;
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sff::Style;

    pub(super) fn corpus() -> Vec<std::path::PathBuf> {
        crate::library::corpus_styles()
    }


    /// Stop Accompaniment: chords sound on bass + pad while stopped, and clear on start.
    #[test]
    fn stop_accompaniment() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/FunkyFinger.S930.STY");
        if !path.exists() {
            return;
        }
        let mut e = Engine::new(Box::new(Prepared::new(&Style::load(&path).unwrap())));
        let mut rec = Recorder::default();
        e.button(Button::SyncStart, 0, &mut rec); // disarm sync start
        e.button(Button::StopAcmp, 0, &mut rec);
        e.set_chord(Chord::new(9, 8), 1, &mut rec); // Am
        let ons: Vec<(u8, u8)> = rec.out.iter().filter(|(_, m)| m[0] & 0xF0 == 0x90).map(|(_, m)| (m[0] & 0xF, m[1] % 12)).collect();
        assert!(ons.contains(&(10, 9)), "bass A: {ons:?}");
        for pc in [9, 0, 4] {
            assert!(ons.contains(&(13, pc)), "pad {pc}: {ons:?}");
        }
        assert!(!e.is_running());
        rec.out.clear();
        e.set_chord(Chord::new(5, 0), 2, &mut rec); // F: old notes off, new on
        assert!(rec.out.iter().any(|(_, m)| m[0] == 0x8A && m[1] % 12 == 9));
        assert!(rec.out.iter().any(|(_, m)| m[0] == 0x9A && m[1] % 12 == 5));
        rec.out.clear();
        e.button(Button::StartStop, 3, &mut rec);
        assert!(rec.out.iter().any(|(_, m)| m[0] == 0x8A && m[1] % 12 == 5), "released on start");
    }

    /// Every style under every chord type, including 1+8, Cancel and the display-only
    /// Data List types, which must play exactly like the CASM type they map to.
    #[test]
    fn corpus_every_chord_type() {
        use crate::theory::{casm_type, CANCEL, TYPE_NAMES};
        let files = corpus();
        if files.is_empty() {
            eprintln!("no corpus; skipping");
            return;
        }
        for f in files {
            let style = Style::load(&f).unwrap();
            let prep = Prepared::new(&style);
            let bar = (60e9 / prep.bpm * (prep.tpb as f64 / prep.ppq as f64)) as u64;
            let play = |ty: u8| {
                let script = [(0, Step::Chord(Chord { root: 2, ty, bass: Some(9) }))];
                run(Box::new(Prepared::new(&style)), &script, bar * 2).1.out
            };
            for ty in 0..TYPE_NAMES.len() as u8 {
                let out = play(ty);
                // Cancel does not sync-start the style.
                assert!(ty == CANCEL || out.iter().any(|(_, m)| m[0] & 0xF0 == 0x90), "{}: silent under {ty}", f.display());
                // Regression guard, not a fidelity check: plays/transpose map through
                // casm() on entry, so this holds by construction unless that is bypassed.
                if casm_type(ty) != ty {
                    assert_eq!(out, play(casm_type(ty)), "{}: type {}", f.display(), TYPE_NAMES[ty as usize]);
                }
            }
        }
    }

    fn bar_ns(prep: &Prepared) -> u64 {
        (60e9 / prep.bpm * (prep.tpb as f64 / prep.ppq as f64)) as u64
    }

    /// Note-ons in [from, to) as (channel, key).
    fn ons(rec: &Recorder, from: u64, to: u64) -> Vec<(u8, u8)> {
        rec.pitch_ons().into_iter().filter(|&(t, _, _)| t >= from && t < to).map(|(_, ch, p)| (ch, p)).collect()
    }

    /// Chord Cancel is the no-chord state: every part except rhythm (and CASM autostart
    /// channels) goes quiet at once, and the band comes back on the next chord.
    #[test]
    fn chord_cancel_leaves_only_rhythm() {
        use crate::theory::{is_drum_part, CANCEL};
        let files = corpus();
        if files.is_empty() {
            eprintln!("no corpus; skipping");
            return;
        }
        for f in files {
            let style = Style::load(&f).unwrap();
            let name = f.file_name().unwrap().to_string_lossy().to_string();
            let autostart: Vec<u8> =
                style.casm.iter().flat_map(|s| &s.rules).filter(|r| r.autostart).map(|r| r.dest_ch).collect();
            let rests = |ch: u8| !is_drum_part(ch) && !autostart.contains(&ch);
            let prep = Box::new(Prepared::new(&style));
            let bar = bar_ns(&prep);
            let (t_cancel, t_back) = (2 * bar + bar / 3, 4 * bar + bar / 3);
            let script = [(0, Step::Chord(Chord::new(0, 0))), (t_cancel, Step::Chord(Chord::new(0, CANCEL))),
                          (t_back, Step::Chord(Chord::new(5, 0)))];
            let (_, rec) = run(prep, &script, 6 * bar);
            // By the end of the Cancel instant every resting part has been released.
            let mut held = std::collections::HashMap::<(u8, u8), i32>::new();
            for (_, m) in rec.out.iter().filter(|(t, m)| *t <= t_cancel && rests(m[0] & 0xF)) {
                match m[0] & 0xF0 {
                    0x90 if m[2] > 0 => *held.entry((m[0] & 0xF, m[1])).or_default() += 1,
                    0x80 | 0x90 => *held.entry((m[0] & 0xF, m[1])).or_default() -= 1,
                    _ => {}
                }
            }
            assert!(held.values().all(|&n| n == 0), "{name}: still sounding after Cancel: {held:?}");
            let before = ons(&rec, 0, t_cancel);
            let during = ons(&rec, t_cancel, t_back);
            let after = ons(&rec, t_back, 6 * bar);
            assert!(during.iter().all(|&(ch, _)| !rests(ch)), "{name}: plays under Cancel: {during:?}");
            if before.iter().any(|&(ch, _)| is_drum_part(ch)) {
                assert!(during.iter().any(|&(ch, _)| is_drum_part(ch)), "{name}: rhythm stopped under Cancel");
            }
            if before.iter().any(|&(ch, _)| rests(ch)) {
                assert!(after.iter().any(|&(ch, _)| rests(ch)), "{name}: band did not come back after Cancel");
            }
        }
    }

    /// Notes sounding once everything at time `t` has been sent, as sorted (channel, key).
    fn held_at(rec: &Recorder, t: u64) -> Vec<(u8, u8)> {
        rec.sounding_at(t)
    }

    /// A chord that ends the no-chord state (Chord Cancel, or Start with no chord yet)
    /// a little after the barline still gets the downbeat: once it lands, the same notes
    /// sound as if it had come exactly on the beat. (A note with less of it left than it
    /// has missed is not started, as it would be a blip, so keys the on-beat run strikes or
    /// releases within as long again are not compared.)
    #[test]
    fn late_chord_after_no_chord_keeps_the_downbeat() {
        use crate::theory::CANCEL;
        let files = corpus();
        if files.is_empty() {
            eprintln!("no corpus; skipping");
            return;
        }
        let late = 20_000_000;
        for f in files {
            let style = Style::load(&f).unwrap();
            let name = f.file_name().unwrap().to_string_lossy().to_string();
            let prep = Prepared::new(&style);
            let bar = bar_ns(&prep);
            let t_f = 4 * bar + late;
            let intros: [Vec<(u64, Step)>; 2] = [
                vec![(0, Step::Chord(Chord::new(0, 0))), (2 * bar + bar / 3, Step::Chord(Chord::new(0, CANCEL)))],
                vec![(0, Step::Button(Button::StartStop))],
            ];
            // The notes sounding at `t` in `rec`, but for keys the on-beat run `on` strikes or
            // releases within `late` after it.
            let settled = |on: &Recorder, rec: &Recorder, t: u64, late: u64| {
                let moving = |n: &(u8, u8)| {
                    on.out.iter().any(|(at, m)| *at > t && *at <= t + late && m[0] & 0xE0 == 0x80 && (m[0] & 0xF, m[1]) == *n)
                };
                held_at(rec, t).into_iter().filter(|n| !moving(n)).collect::<Vec<_>>()
            };
            for (what, intro) in ["Cancel", "no chord"].iter().zip(intros) {
                let mut recs = Vec::new();
                for at in [4 * bar, t_f] {
                    let mut script = intro.clone();
                    script.push((at, Step::Chord(Chord::new(5, 0))));
                    recs.push(run(Box::new(Prepared::new(&style)), &script, t_f + late + 1).1);
                }
                let held = [settled(&recs[0], &recs[0], t_f, late), settled(&recs[0], &recs[1], t_f, late)];
                assert_eq!(held[1], held[0], "{name}: F 20 ms late after {what}");
            }
            // Break pressed under Cancel enters mid-bar on the next beat. A chord just after
            // that entry must not start Break notes from before it, which never sounded.
            let beat = bar * prep.ppq as u64 / prep.tpb as u64;
            let press = 3 * bar + beat + beat / 2;
            let entry = 3 * bar + 2 * beat;
            for late in [10_000_000, 20_000_000, 35_000_000] {
                let t_f = entry + late;
                let mut recs = Vec::new();
                for at in [entry, t_f] {
                    let script = [(0, Step::Chord(Chord::new(0, 0))), (2 * bar + bar / 3, Step::Chord(Chord::new(0, CANCEL))),
                                  (press, Step::Button(Button::Break)), (at, Step::Chord(Chord::new(5, 0)))];
                    recs.push(run(Box::new(Prepared::new(&style)), &script, t_f + late + 1).1);
                }
                let held = [settled(&recs[0], &recs[0], t_f, late), settled(&recs[0], &recs[1], t_f, late)];
                assert_eq!(held[1], held[0], "{name}: F {} ms after a Break entry under Cancel", late / 1_000_000);
            }
        }
    }

    /// Cancel is not a chord, so it does not trigger Sync Start.
    #[test]
    fn chord_cancel_does_not_sync_start() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/FunkyFinger.S930.STY");
        if !path.exists() {
            return;
        }
        let prep = Box::new(Prepared::new(&Style::load(&path).unwrap()));
        let bar = bar_ns(&prep);
        let (e, rec) = run(prep, &[(0, Step::Chord(Chord::new(0, crate::theory::CANCEL)))], bar);
        assert!(!e.is_running());
        assert!(ons(&rec, 0, bar).is_empty());
    }

    /// Channels whose every rule is a Guitar (other parts may retrigger a unison).
    fn guitar_channels(style: &Style) -> Vec<u8> {
        use crate::sff::Ntr;
        let rules: Vec<_> = style.casm.iter().flat_map(|s| &s.rules).collect();
        let is_guitar = |r: &&crate::sff::ChannelRule| (0..128).all(|k| r.zone_for(k).ntr == Ntr::Guitar);
        (8..16u8)
            .filter(|&ch| rules.iter().any(|r| r.dest_ch == ch && is_guitar(r)))
            .filter(|&ch| rules.iter().all(|r| r.dest_ch != ch || is_guitar(r)))
            .collect()
    }

    /// A chord just after the beat corrects the strum outright: the strings the previous
    /// chord left out (strings Stroke muted below its bass) come in too, and unisons are
    /// re-parted, so every corpus Guitar part rings the strings of that strum it rings
    /// when the chord lands on the beat. (Strings of earlier strums follow their
    /// retrigger rule in both cases.)
    #[test]
    fn corpus_late_chord_restores_guitar_strings() {
        let mut styles = 0;
        for f in corpus() {
            let style = Style::load(&f).unwrap();
            let guitar = guitar_channels(&style);
            if guitar.is_empty() {
                continue;
            }
            styles += 1;
            let name = f.file_name().unwrap().to_string_lossy().to_string();
            let bar = bar_ns(&Prepared::new(&style));
            for (before, after) in [(Chord::new(0, 30), Chord::new(0, 0)), (Chord::new(7, 31), Chord::new(7, 1)),
                                    (Chord::new(2, 0), Chord::new(0, 0)), (Chord::new(9, 10), Chord::new(4, 2))] {
                let t_f = 2 * bar + 20_000_000;
                let mut held = Vec::new();
                for at in [2 * bar, t_f] {
                    let script = [(0, Step::Chord(before)), (at, Step::Chord(after))];
                    let (e, _) = run(Box::new(Prepared::new(&style)), &script, t_f + 1);
                    held.push(guitar.iter().map(|&ch| e.guitar_strings(ch, 2 * bar)).collect::<Vec<_>>());
                }
                assert_eq!(held[1], held[0], "{name}: {after:?} 20 ms late after {before:?}");
            }
        }
        if !corpus().is_empty() {
            assert!(styles > 0, "corpus has no Guitar parts");
        }
    }

    /// A guitar strikes each pitch once. Over chords that fold strings together (1+8, 1+5,
    /// 7#9) and through chord changes that re-pitch ringing strings, no Guitar part of any
    /// corpus style re-strikes a key another of its strings is ringing, and no two of its
    /// strings sound on one key.
    #[test]
    fn corpus_guitar_strikes_each_pitch_once() {
        let mut styles = 0;
        for f in corpus() {
            let style = Style::load(&f).unwrap();
            let guitar = guitar_channels(&style);
            if guitar.is_empty() {
                continue;
            }
            styles += 1;
            let prep = Box::new(Prepared::new(&style));
            let bar = bar_ns(&prep);
            let chords = [Chord::new(0, 0), Chord::new(7, 30), Chord::new(0, 27), Chord::new(7, 31), Chord::new(2, 0),
                          Chord { root: 0, ty: 0, bass: Some(7) }, Chord::new(9, 10), Chord::new(6, 0), Chord::new(4, 30),
                          Chord::new(0, 2), Chord::new(5, 27), Chord::new(9, 31)];
            // Half-bar changes (ringing strings are re-pitched) and changes just after the
            // downbeat (the notes that just started are re-voiced).
            let script: Vec<(u64, Step)> = chords
                .iter()
                .enumerate()
                .map(|(i, &c)| (i as u64 * bar / 2 + if i % 3 == 2 { 10_000_000 } else { 0 }, Step::Chord(c)))
                .collect();
            let mut unisons = 0;
            let (_, rec) = run_observed(prep, &script, chords.len() as u64 * bar / 2 + bar, |e, _| unisons += e.guitar_unisons());
            let name = f.file_name().unwrap().to_string_lossy().to_string();
            assert_eq!(unisons, 0, "{name}: two guitar strings on one key");
            // What the synth hears: a note-on for a key the channel already sounds either
            // re-strikes a ringing string or is cut by the other string's later note-off.
            let mut on = [[false; 128]; 16];
            for (t, m) in rec.out.iter().filter(|(_, m)| m[0] & 0xE0 == 0x80) {
                let (ch, key) = (m[0] & 0xF, m[1] as usize);
                let strike = m[0] & 0xF0 == 0x90 && m[2] > 0;
                assert!(!(strike && on[ch as usize][key] && guitar.contains(&ch)), "{name} ch{}: key {key} struck while sounding at {t} ns", ch + 1);
                on[ch as usize][key] = strike;
            }
        }
        if !corpus().is_empty() {
            assert!(styles > 0, "corpus has no Guitar parts");
        }
    }

    /// 1+8 and 1+5 over every corpus style: parts that follow the chord (NTT other than
    /// Bypass) play only the root for 1+8, and only root, 5th, 2nd and 4th for 1+5 (Guitar
    /// noise keys, which are not pitches, aside). The CASM chord-mute routing in the corpus
    /// always gives the bass something to play.
    #[test]
    fn corpus_one_plus_eight_and_one_plus_five() {
        use crate::sff::Ntt;
        use crate::theory::is_drum_part;
        let files = corpus();
        if files.is_empty() {
            eprintln!("no corpus; skipping");
            return;
        }
        for f in files {
            let style = Style::load(&f).unwrap();
            let name = f.file_name().unwrap().to_string_lossy().to_string();
            // Channels written to play as recorded are exempt.
            let as_written: Vec<u8> = style
                .casm
                .iter()
                .filter(|s| s.sections.iter().any(|n| n == "Main A"))
                .flat_map(|s| &s.rules)
                .filter(|r| r.zones.iter().any(|z| z.ntt == Ntt::Bypass))
                .map(|r| r.dest_ch)
                .collect();
            let follows = |ch: u8| !is_drum_part(ch) && !as_written.contains(&ch);
            // Guitar noise keys (MegaVoice strum and fret noises) are not pitches: they play
            // untouched on every chord. A held one sounds with the part's bend (at most an
            // octave), so on a Guitar channel anything within an octave below them is let be.
            let guitar: Vec<u8> = style
                .casm
                .iter()
                .filter(|s| s.sections.iter().any(|n| n == "Main A"))
                .flat_map(|s| &s.rules)
                .filter(|r| r.zones.iter().any(|z| z.ntr == crate::sff::Ntr::Guitar))
                .map(|r| r.dest_ch)
                .collect();
            let pitched = |ch: u8, key: u8| !(guitar.contains(&ch) && key + 12 >= crate::theory::GUITAR_NOISE);
            let prep = Box::new(Prepared::new(&style));
            let bar = bar_ns(&prep);
            let script = [(0, Step::Chord(Chord::new(7, 30))), (2 * bar, Step::Chord(Chord::new(2, 31))),
                          (4 * bar, Step::Chord(Chord::new(0, 0)))];
            let (_, rec) = run(prep, &script, 6 * bar);
            // Notes started in the window, and notes held into it (a pitch shift bends them).
            let heard = |from: u64, to: u64| {
                let mut v = ons(&rec, from, to);
                v.extend(rec.sounding_at(from + 1));
                v
            };
            let g8 = heard(0, 2 * bar);
            let d5 = heard(2 * bar, 4 * bar);
            let c = heard(4 * bar, 6 * bar);
            for &(ch, key) in g8.iter().filter(|&&(ch, key)| follows(ch) && pitched(ch, key)) {
                assert_eq!(key % 12, 7, "{name}: ch{} key {key} under G1+8", ch + 1);
            }
            for &(ch, key) in d5.iter().filter(|&&(ch, key)| follows(ch) && pitched(ch, key)) {
                assert!(matches!(key % 12, 2 | 4 | 7 | 9), "{name}: ch{} key {key} under D1+5", ch + 1);
            }
            if c.iter().any(|&(ch, _)| ch == 10) {
                assert!(g8.iter().any(|&(ch, _)| ch == 10), "{name}: bass silent under 1+8");
                assert!(d5.iter().any(|&(ch, _)| ch == 10), "{name}: bass silent under 1+5");
            }
            // The pitch checks above pass trivially on a silent channel, so a part whose
            // Main A rules all allow the chord (CASM chord-mute bit 30 / 31 and the root)
            // and that plays over the same bars under G / D major must also play under
            // G1+8 / D1+5.
            let script = [(0, Step::Chord(Chord::new(7, 0))), (2 * bar, Step::Chord(Chord::new(2, 0)))];
            let (_, reference) = run(Box::new(Prepared::new(&style)), &script, 4 * bar);
            let main_a: Vec<_> =
                style.casm.iter().filter(|s| s.sections.iter().any(|n| n == "Main A")).flat_map(|s| &s.rules).collect();
            for (ty, root, got, from) in [(30u8, 7u8, &g8, 0), (31, 2, &d5, 2 * bar)] {
                let major = ons(&reference, from, from + 2 * bar);
                for ch in 8..16u8 {
                    let rules: Vec<_> = main_a.iter().filter(|r| r.dest_ch == ch).collect();
                    let allowed = !rules.is_empty()
                        && rules.iter().all(|r| r.chord_mute & (1 << ty) != 0 && r.note_mute & (1 << root) != 0);
                    if allowed && !is_drum_part(ch) && major.iter().any(|&(x, _)| x == ch) {
                        assert!(got.iter().any(|&(x, _)| x == ch), "{name}: ch{} silent under type {ty}", ch + 1);
                    }
                }
            }
        }
    }

    /// Every style: play through intro, mains, fills, break, chord changes and ending.
    /// Afterwards the engine must be stopped with no sounding notes, and every note-on
    /// must have a matching note-off.
    #[test]
    fn corpus_full_performance_no_stuck_notes() {
        let files = corpus();
        if files.is_empty() {
            eprintln!("no corpus; skipping");
            return;
        }
        let chords = [Chord::new(0, 0), Chord::new(9, 10), Chord::new(5, 2), Chord::new(7, 19),
                      Chord::new(2, 8), Chord { root: 0, ty: 0, bass: Some(4) }, Chord::new(10, 22), Chord::new(6, 11)];
        for f in files {
            let style = Style::load(&f).unwrap();
            let prep = Box::new(Prepared::new(&style));
            let bar = (60e9 / prep.bpm * (prep.tpb as f64 / prep.ppq as f64)) as u64;
            let mut script = vec![(0, Step::Button(Button::Intro(0))), (1_000, Step::Chord(chords[0]))];
            let mut t = bar / 3;
            for (i, b) in [Button::Main(1), Button::Main(1), Button::Main(2), Button::Break, Button::Main(3),
                           Button::Main(0), Button::AutoFill, Button::Main(2), Button::Intro(1)].iter().enumerate() {
                script.push((t, Step::Button(*b)));
                script.push((t + bar / 7, Step::Chord(chords[(i + 1) % chords.len()])));
                t += bar + bar / 5;
            }
            // Transpose changes while notes sound (Keyboard moves the chord, Master the output).
            script.push((bar * 2 + bar / 2, Step::Transpose(Transpose::new(3, 0))));
            script.push((bar * 4 + bar / 3, Step::Transpose(Transpose::new(3, -5))));
            script.push((bar * 6 + bar / 5, Step::Transpose(Transpose::new(-12, 12))));
            script.push((t, Step::Button(Button::Ending(0))));
            script.sort_by_key(|s| s.0);
            let (e, rec) = run(prep, &script, t + bar * 12);
            let name = f.file_name().unwrap().to_string_lossy().to_string();
            assert!(!e.is_running(), "{name}: still running after ending");
            let mut on = std::collections::HashMap::<(u8, u8), i32>::new();
            let mut notes = 0;
            for (_, m) in &rec.out {
                match m[0] & 0xF0 {
                    0x90 => { *on.entry((m[0] & 0xF, m[1])).or_default() += 1; notes += 1; }
                    0x80 => { *on.entry((m[0] & 0xF, m[1])).or_default() -= 1; }
                    _ => {}
                }
            }
            assert!(notes > 20, "{name}: only {notes} notes");
            for ((ch, key), n) in on {
                assert_eq!(n, 0, "{name}: ch{} key {key} unbalanced by {n}", ch + 1);
            }
            // No part may be left bent after it stops.
            for ch in 8..16u8 {
                if let Some((_, m)) = rec.out.iter().rev().find(|(_, m)| m[0] == 0xE0 | ch) {
                    assert_eq!((m[1], m[2]), (0, 0x40), "{name}: ch{} left bent", ch + 1);
                }
            }
            // The Style never speaks on a keyboard part's channel: its setup (SInt, re-sent
            // on every start and section change) and its patterns stay on ch 9-16, so a
            // part's volume is only ever its own CC7 and its voice its own program.
            for (_, m) in &rec.out {
                let part = if m[0] == 0xF0 {
                    // XG Multi Part parameter (43 1n 4C 08 pp ..) on a part's channel.
                    (m.len() > 5 && m[1] == 0x43 && m[3] == 0x4C && m[4] == 0x08).then(|| m[5])
                } else {
                    Some(m[0] & 0x0F)
                };
                assert!(part.and_then(crate::parts::part_of_channel).is_none(), "{name}: {m:02X?} on a keyboard part");
            }
        }
    }
}

/// Retrigger Rule on chord changes: Pitch Shift bends, and revoicing never leaves blips,
/// doubled keys or stuck notes.
#[cfg(test)]
mod rtr {
    use super::*;
    use crate::engine::EARLY_CHORD_NS;
    use crate::sff::{Ev, Rtr, SectionId, Style};

    fn bar_ns(p: &Prepared) -> u64 {
        (60e9 / p.bpm * (p.tpb as f64 / p.ppq as f64)) as u64
    }

    /// Chord changes every half bar through Main A-D, alternately on the beat and a
    /// little after it (or, with `early`, all that long before the beat), over slash
    /// chords and, with `fold`, chords that fold voices together (1+8, 1+5; without it,
    /// major chords in their place).
    fn script(bar: u64, fold: bool, early: u64) -> (Vec<(u64, Step)>, Vec<u64>, u64) {
        let (one8, one5) = if fold { (30, 31) } else { (0, 0) };
        let chords = [Chord::new(0, 0), Chord::new(9, 10), Chord::new(7, one8), Chord::new(5, 2), Chord::new(2, one5),
                      Chord { root: 0, ty: 0, bass: Some(4) }, Chord::new(7, 19), Chord::new(10, 22), Chord::new(4, 8),
                      Chord::new(6, 11), Chord::new(1, 0), Chord::new(11, one8)];
        let mut s = vec![(0, Step::Chord(chords[0]))];
        let mut changes = Vec::new();
        for i in 1..32u64 {
            let t = match early {
                0 => i * bar / 2 + if i % 2 == 1 { bar / 11 } else { 0 },
                e => i * bar / 2 - e,
            };
            if i % 8 == 0 {
                s.push((t - bar / 4, Step::Button(Button::Main((i / 8) as u8 % 4))));
            }
            s.push((t, Step::Chord(chords[i as usize % chords.len()])));
            changes.push(t);
        }
        s.push((16 * bar + bar / 3, Step::Button(Button::Stop)));
        s.sort_by_key(|x| x.0);
        (s, changes, 17 * bar)
    }

    /// A busy performance: a chord change every quarter bar, at an uneven moment, to any
    /// root and chord type (slash chords among them), through Main A-D, then Stop.
    fn busy_script(bar: u64, seed: u64) -> (Vec<(u64, Step)>, u64) {
        let mut x = seed;
        let mut rand = |n: u64| {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (x >> 33) % n
        };
        let mut s = vec![(0, Step::Chord(Chord::new(0, 0)))];
        for i in 1..64u64 {
            let t = i * bar / 4 + rand(bar / 8);
            if i % 12 == 0 {
                s.push((t - bar / 8, Step::Button(Button::Main((i / 12) as u8 % 4))));
            }
            let ty = rand(34) as u8;
            let bass = (rand(4) == 0).then(|| rand(12) as u8);
            s.push((t, Step::Chord(Chord { root: rand(12) as u8, ty, bass })));
        }
        s.push((16 * bar + bar / 3, Step::Button(Button::Stop)));
        s.sort_by_key(|x| x.0);
        (s, 17 * bar)
    }

    /// On the parts that follow chords, no note starts on a key its channel is already
    /// sounding (#46) and none lasts zero time (#49). On every part, no note is left
    /// sounding and no part is left bent. (Rhythm parts play as written, doubled hits too.)
    #[test]
    fn corpus_chord_changes_leave_no_blips_or_doubled_keys() {
        let files = tests::corpus();
        if files.is_empty() {
            eprintln!("no corpus; skipping");
            return;
        }
        for f in files {
            let style = Style::load(&f).unwrap();
            let name = f.file_name().unwrap().to_string_lossy().to_string();
            let prep = Box::new(Prepared::new(&style));
            let (script, _, end) = script(bar_ns(&prep), true, 0);
            let (e, rec) = run(prep, &script, end);
            assert!(!e.is_running(), "{name}: still running");
            let mut on = std::collections::HashMap::<(u8, u8), u64>::new();
            for (t, m) in rec.out.iter().filter(|(_, m)| !crate::theory::is_drum_part(m[0] & 0x0F)) {
                let key = (m[0] & 0x0F, m[1]);
                match m[0] & 0xF0 {
                    0x90 if m[2] > 0 => {
                        assert!(on.insert(key, *t).is_none(), "{name}: ch{} key {} started twice at {t}", key.0 + 1, key.1);
                    }
                    0x80 | 0x90 => {
                        let start = on.remove(&key);
                        assert!(start.is_some_and(|s| s < *t), "{name}: ch{} key {} zero-length or unmatched at {t}", key.0 + 1, key.1);
                    }
                    _ => {}
                }
            }
            assert!(on.is_empty(), "{name}: stuck notes {on:?}");
            let (ons, offs) = rec.out.iter().fold((0, 0), |(a, b), (_, m)| match m[0] & 0xF0 {
                0x90 if m[2] > 0 => (a + 1, b),
                0x80 | 0x90 => (a, b + 1),
                _ => (a, b),
            });
            assert_eq!(ons, offs, "{name}: note-ons and note-offs unbalanced");
            assert!(rec.retunes.last().is_none_or(|r| r.3 == 0), "{name}: left pitch-shifted");
            // Through every section change, a part that follows chords keeps a bend range
            // of at least 12 (the SInt's and the patterns' own RPN 0 go out raised).
            let mut rpn = [[127u8; 2]; 16];
            for (t, m) in rec.out.iter().filter(|(_, m)| m.len() == 3 && m[0] & 0xF0 == 0xB0) {
                let ch = (m[0] & 0x0F) as usize;
                match m[1] {
                    101 => rpn[ch][0] = m[2],
                    100 => rpn[ch][1] = m[2],
                    98 | 99 => rpn[ch] = [127; 2],
                    6 if rpn[ch] == [0, 0] && (8..16).contains(&ch) && !crate::theory::is_drum_part(ch as u8) => {
                        assert!(m[2] >= 12, "{name}: ch{} bend range {} at {t}", ch + 1, m[2]);
                    }
                    _ => {}
                }
            }
            for ch in 8..16u8 {
                if let Some((_, m)) = rec.out.iter().rev().find(|(_, m)| m[0] == 0xE0 | ch) {
                    assert_eq!((m[1], m[2]), (0, 0x40), "{name}: ch{} left bent", ch + 1);
                }
            }
        }
    }

    /// A pitch shift sounds exactly the notes a retrigger would. Every style plays as
    /// written and again with Pitch Shift (to Root) turned into Retrigger (to Root):
    /// - from a first chord to a second, the pitches sounding just after the change agree
    ///   (but for notes about to end or be struck again: those are bent, or left as they
    ///   are);
    /// - through a whole performance, every note started between chord changes sounds the
    ///   same pitch (on a part a pitch shift keeps bent, it is sent compensated).
    ///
    /// (Later on, the two runs may differ in which of two voices that met on one key kept
    /// sounding, and so in when it ends.)
    #[test]
    fn corpus_pitch_shift_sounds_what_retrigger_would() {
        let files = tests::corpus();
        if files.is_empty() {
            eprintln!("no corpus; skipping");
            return;
        }
        let mut bends = 0;
        for f in files {
            let style = Style::load(&f).unwrap();
            let name = f.file_name().unwrap().to_string_lossy().to_string();
            let mut retrig = Style::load(&f).unwrap();
            for z in retrig.casm.iter_mut().flat_map(|s| s.rules.iter_mut()).flat_map(|r| r.zones.iter_mut()) {
                z.rtr = match z.rtr {
                    Rtr::PitchShift => Rtr::Retrigger,
                    Rtr::PitchShiftToRoot => Rtr::RetriggerToRoot,
                    r => r,
                };
            }
            // Guitar noise keys (from theory::GUITAR_NOISE up in a Guitar zone) are not
            // pitches: they go out as written, so on a bent part they sound bent (at most an
            // octave). Leave them out on those parts.
            let noise_parts: Vec<u8> = style
                .casm
                .iter()
                .flat_map(|s| &s.rules)
                .filter(|r| (crate::theory::GUITAR_NOISE..=127).any(|k| r.zone_for(k).ntr == crate::sff::Ntr::Guitar))
                .map(|r| r.dest_ch)
                .collect();
            let noise = |ch: u8, pitch: u8| noise_parts.contains(&ch) && pitch + 12 >= crate::theory::GUITAR_NOISE;
            let both = |script: &[(u64, Step)], end: u64| {
                let (_, a) = run(Box::new(Prepared::new(&style)), script, end);
                let (_, b) = run(Box::new(Prepared::new(&retrig)), script, end);
                (a, b)
            };
            let bar = bar_ns(&Prepared::new(&style));
            let (script, changes, end) = script(bar, true, 0);
            let chords: Vec<Chord> = script.iter().filter_map(|s| if let Step::Chord(c) = s.1 { Some(c) } else { None }).collect();
            for (i, pair) in chords.windows(2).enumerate() {
                let t = if i % 2 == 0 { bar + bar / 3 + bar / 11 } else { 2 * bar };
                let s = [(0, Step::Button(Button::Main(i as u8 % 4))), (1, Step::Chord(pair[0])), (t, Step::Chord(pair[1]))];
                let (a, b) = both(&s, t + EARLY_CHORD_NS + 1);
                bends += a.retunes.len();
                // A note about to end, or to be struck again, is bent under Pitch Shift but
                // left as it is (or stopped) under Retrigger: leave those pitches out.
                let mut moving = Vec::new();
                for r in [&a, &b] {
                    let later = r.sounding_at(t + EARLY_CHORD_NS);
                    moving.extend(r.sounding_at(t + 1).into_iter().filter(|n| !later.contains(n)));
                    moving.extend(r.pitch_ons().into_iter().filter(|n| n.0 > t).map(|n| (n.1, n.2)));
                }
                let settled =
                    |r: &Recorder| r.sounding_at(t + 1).into_iter().filter(|n| !moving.contains(n) && !noise(n.0, n.1)).collect::<Vec<_>>();
                assert_eq!(settled(&a), settled(&b), "{name}: {:?} to {:?} at {t}", pair[0], pair[1]);
            }
            let (a, b) = both(&script, end);
            bends += a.retunes.len();
            let pitched = |p: Vec<(u64, u8, u8)>| {
                p.into_iter().filter(|n| !noise(n.1, n.2)).collect::<Vec<_>>()
            };
            let (pa, pb) = (pitched(a.pitch_ons()), pitched(b.pitch_ons()));
            for w in changes.windows(2) {
                let started = |p: &[(u64, u8, u8)]| p.iter().filter(|n| n.0 > w[0] && n.0 < w[1]).copied().collect::<Vec<_>>();
                assert_eq!(started(&pa), started(&pb), "{name}: notes between {} and {}", w[0], w[1]);
            }
        }
        assert!(bends > 100, "only {bends} pitch shifts across the corpus");
    }

    /// A chord played a little before the beat (#49): a note the pattern ends or strikes
    /// again on the beat is left to end, not retriggered for a moment. Every note a chord
    /// change retriggers lasts at least `EARLY_CHORD_NS`. (A note it starts because its
    /// part comes in keeps at least as much of its written length as it has missed.)
    #[test]
    fn corpus_early_chords_start_no_blips() {
        let files = tests::corpus();
        if files.is_empty() {
            eprintln!("no corpus; skipping");
            return;
        }
        let mut started = 0;
        for f in files {
            let style = Style::load(&f).unwrap();
            let name = f.file_name().unwrap().to_string_lossy().to_string();
            let bar = bar_ns(&Prepared::new(&style));
            // Odd offsets, so no pattern event falls on a chord change by chance.
            for early in [1_000_003, 3_000_017, 20_000_029, EARLY_CHORD_NS - 1_000_003] {
                let (script, changes, end) = script(bar, true, early);
                let (e, rec) = run(Box::new(Prepared::new(&style)), &script, end);
                let mut on = std::collections::HashMap::<(u8, u8), u64>::new();
                for (t, m) in rec.out.iter().filter(|(_, m)| !crate::theory::is_drum_part(m[0] & 0x0F)) {
                    let key = (m[0] & 0x0F, m[1]);
                    match m[0] & 0xF0 {
                        0x90 if m[2] > 0 => {
                            on.insert(key, *t);
                        }
                        0x80 | 0x90 => {
                            let retriggered = |s: &u64| e.retriggered.contains(&(*s, key.0, key.1));
                            let Some(s) = on.remove(&key).filter(|s| changes.binary_search(s).is_ok() && retriggered(s)) else { continue };
                            started += 1;
                            assert!(t - s >= EARLY_CHORD_NS, "{name}: ch{} key {} started {early} ns before the beat at {s} lasts {} ns",
                                    key.0 + 1, key.1, t - s);
                        }
                        _ => {}
                    }
                }
            }
        }
        assert!(started > 1000, "only {started} notes started on chord changes");
    }

    /// Merengue's Main A at the bar line, with the bar length truncated to whole ns so the
    /// chord lands a fraction of a tick before the beat (review repro): ch12's notes that
    /// end on the beat used to be retriggered for 1 ns.
    #[test]
    fn chord_a_hair_before_the_beat_retriggers_nothing_that_ends_on_it() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/T5Style/Merengue.T158.prs");
        if !p.exists() {
            return;
        }
        let style = Style::load(&p).unwrap();
        let bar = bar_ns(&Prepared::new(&style));
        let (script, ..) = script(bar, true, 0);
        let chords: Vec<Chord> = script.iter().filter_map(|s| if let Step::Chord(c) = s.1 { Some(c) } else { None }).collect();
        for pair in chords.windows(2) {
            for t in [2 * bar, 2 * bar - 1, 2 * bar - 1_000_000] {
                let s = [(0, Step::Button(Button::Main(0))), (1, Step::Chord(pair[0])), (t, Step::Chord(pair[1]))];
                let (_, rec) = run(Box::new(Prepared::new(&style)), &s, t + bar);
                for (ch, key) in rec.pitch_ons().into_iter().filter(|n| n.0 == t).map(|n| (n.1, n.2)) {
                    let sounding = |at: u64| rec.sounding_at(at).contains(&(ch, key));
                    assert!(sounding(t + EARLY_CHORD_NS - 1), "{:?} to {:?} at {t}: ch{} key {key} is a blip", pair[0], pair[1], ch + 1);
                }
            }
        }
    }

    /// A part's pitch shift never pushes its pattern's own bends past the output range
    /// (review: TickingAway's bass slides a full octave down while shifted, and stopped at
    /// the end of the range). The part gets a wider range, up to 24, and never shifts by
    /// more than that range leaves over its patterns' widest bend; so no bend is clamped.
    #[test]
    fn corpus_pitch_shift_leaves_room_for_pattern_bends() {
        let files = tests::corpus();
        if files.is_empty() {
            eprintln!("no corpus; skipping");
            return;
        }
        let mut runs = 0;
        for f in files {
            let style = Style::load(&f).unwrap();
            let name = f.file_name().unwrap().to_string_lossy().to_string();
            let bar = bar_ns(&Prepared::new(&style));
            for seed in 1..4 {
                let (script, end) = busy_script(bar, seed);
                let (e, _) = run(Box::new(Prepared::new(&style)), &script, end);
                assert_eq!(e.bend_clamps.get(), 0, "{name}: pitch bends clamped");
                runs += 1;
            }
        }
        assert!(runs > 100);
        // AnalogBallad's bass slides a full octave (its patterns set a range of 12): it
        // goes out with 24, room for a 12-semitone shift on top.
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/T5Style/AnalogBallad.T160.prs");
        if !p.exists() {
            return;
        }
        let prep = Box::new(Prepared::new(&Style::load(&p).unwrap()));
        assert_eq!(prep.pat_bend_max[10], 12);
        let (_, rec) = run(prep, &[(0, Step::Chord(Chord::new(0, 0)))], 1);
        let rpn: Vec<u8> = rec.out.iter().filter(|(_, m)| m[0] == 0xB0 | 10 && [101, 100, 6].contains(&m[1])).map(|(_, m)| m[2]).collect();
        assert!(rpn.windows(3).any(|w| w == [0, 0, 24]), "bass bend range not set to 24: {rpn:?}");
    }

    /// Voices a 1+8 chord folds onto one key part again on the next chord (#46). With
    /// every zone set to Retrigger, from C1+8 to C the notes held through the change sound
    /// what they would had C been played all along, E and G among them. (Styles whose
    /// chord parts start their C1+8 voices together; where they come in one by one, the
    /// later one takes the key from the earlier, which then ends.)
    #[test]
    fn folded_voices_part_again() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/T5Style");
        let mut ran = 0;
        for name in ["ChoralSymphony.T150.prs", "OrchestralBallad.T153.prs", "RomanticBallet.T150.prs", "AnalogBallad.T160.prs"] {
            let p = dir.join(name);
            if !p.exists() {
                continue;
            }
            ran += 1;
            let mut style = Style::load(&p).unwrap();
            for z in style.casm.iter_mut().flat_map(|s| s.rules.iter_mut()).flat_map(|r| r.zones.iter_mut()) {
                z.rtr = Rtr::Retrigger;
            }
            let bar = bar_ns(&Prepared::new(&style));
            let t = bar + bar / 3;
            let end = t + EARLY_CHORD_NS + 1;
            let (_, a) = run(Box::new(Prepared::new(&style)), &[(0, Step::Chord(Chord::new(0, 30))), (t, Step::Chord(Chord::new(0, 0)))], end);
            let (_, b) = run(Box::new(Prepared::new(&style)), &[(0, Step::Chord(Chord::new(0, 0)))], end);
            // Notes about to end or be struck again are left out: the change leaves those be.
            let mut moving = Vec::new();
            for r in [&a, &b] {
                let later = r.sounding_at(end - 1);
                moving.extend(r.sounding_at(t + 1).into_iter().filter(|n| !later.contains(n)));
                moving.extend(r.pitch_ons().into_iter().filter(|n| n.0 > t).map(|n| (n.1, n.2)));
            }
            let settled = |r: &Recorder| r.sounding_at(t + 1).into_iter().filter(|n| !moving.contains(n)).collect::<Vec<_>>();
            let held = settled(&a);
            assert_eq!(held, settled(&b), "{name}");
            let parted = held.iter().filter(|n| !crate::theory::is_drum_part(n.0) && n.1 % 12 != 0).count();
            assert!(parted >= 2, "{name}: {held:?}");
        }
        assert!(ran > 0 || tests::corpus().is_empty(), "corpus present but none of the folded-voice styles found");
    }

    /// Part 6 (ch 14, Root Trans, Pitch Shift) in a one-bar style at 120 bpm. Its SInt sets
    /// a bend range of 2 and bends up 1 semitone (0x3000). Main A and Fill A set the range
    /// to 4 on their first beat (the same bend is then 2 semitones). Main A and Main B hold
    /// C3 through the bar; Fill A plays it on beat 4. The part goes out with a range of 14:
    /// the widest bend is 2 semitones, with a 12-semitone shift on top.
    fn bend_style() -> Style {
        use crate::sff::{Section, TimedEv};
        let at = |tick, ev| TimedEv { tick, ev };
        let range4 = || [101, 100, 6].map(|cc| at(0, Ev::Cc { ch: 13, cc, val: if cc == 6 { 4 } else { 0 } }));
        let bar = |id, on: u32, rpn: bool| {
            let mut events = vec![at(on, Ev::NoteOn { ch: 13, key: 60, vel: 100 }), at(1900, Ev::NoteOff { ch: 13, key: 60 })];
            if rpn {
                events.splice(0..0, range4());
            }
            (id, Section { id, start: 0, len: 1920, events })
        };
        Style {
            name: "bend".into(),
            format: String::new(),
            ppq: 480,
            tempo_us: 500_000,
            timesig: (4, 4),
            init: vec![
                Ev::Cc { ch: 13, cc: 101, val: 0 },
                Ev::Cc { ch: 13, cc: 100, val: 0 },
                Ev::Cc { ch: 13, cc: 6, val: 2 },
                Ev::Bend { ch: 13, val: 0x3000 },
            ],
            sections: [bar(SectionId::Main(0), 0, true), bar(SectionId::Main(1), 0, false), bar(SectionId::Fill(0), 1440, true)].into(),
            casm: vec![],
            ots: vec![],
            other_chunks: vec![],
        }
    }

    /// Ch 14's bend range data entries, and its pitch bends as (time, 14-bit value).
    fn ch14_bends(rec: &Recorder) -> (Vec<u8>, Vec<(u64, u16)>) {
        let ranges = rec.out.iter().filter(|(_, m)| m[0] == 0xBD && m[1] == 6).map(|(_, m)| m[2]).collect();
        let bends = rec.out.iter().filter(|(_, m)| m[0] == 0xED).map(|(t, m)| (*t, (m[2] as u16) << 7 | m[1] as u16)).collect();
        (ranges, bends)
    }

    /// The SInt's bend `semis` in the output range of 14, plus a pitch shift of `shift`.
    fn out_bend(semis: f64, shift: f64) -> u16 {
        (8192.0 + (semis + shift) * 8192.0 / 14.0).round() as u16
    }

    /// A section change sends the SInt again (#15): the part's bend range goes back to 14
    /// (not the style's 2), the pattern's RPN 0 of 4 is forgotten, and the SInt's bend goes
    /// out rescaled (1 semitone, not 7). A Pitch Shift then bends by exactly 5 on top.
    #[test]
    fn section_change_keeps_the_output_bend_range() {
        let prep = Box::new(Prepared::new(&bend_style()));
        assert_eq!(prep.pat_bend_max[13], 2);
        let bar = bar_ns(&prep);
        let script = [(0, Step::Chord(Chord::new(0, 0))), (bar / 2, Step::Button(Button::Main(1))),
                      (bar + bar / 2, Step::Chord(Chord::new(5, 0)))];
        let (_, rec) = run(prep, &script, bar + bar / 2 + 1);
        let (ranges, bends) = ch14_bends(&rec);
        assert!(ranges.len() >= 3 && ranges.iter().all(|&r| r == 14), "bend ranges sent: {ranges:?}");
        let last_before = |t: u64| bends.iter().rev().find(|b| b.0 < t).unwrap().1;
        assert_eq!(last_before(bar / 2), out_bend(2.0, 0.0), "Main A: 0x3000 in its range of 4");
        assert_eq!(last_before(bar + bar / 2), out_bend(1.0, 0.0), "Main B: the SInt's range of 2 again");
        assert_eq!(bends.last().unwrap(), &(bar + bar / 2, out_bend(1.0, 5.0)), "C to F: bent up 5");
        assert!(!rec.out.iter().any(|(t, m)| *t == bar + bar / 2 && m[0] & 0xEF == 0x8D), "retriggered");
        assert!(rec.sounding_at(bar + bar / 2 + 1).contains(&(13, 65)));
    }

    /// A Fill entered mid-bar gets the SInt again, then its own first-beat controllers
    /// (chased): its RPN 0 of 4 sets the range the SInt's bend is read in, and still goes
    /// out as 14. A Pitch Shift during the Fill bends by exactly 5 on top.
    #[test]
    fn mid_bar_fill_keeps_the_output_bend_range() {
        let prep = Box::new(Prepared::new(&bend_style()));
        let bar = bar_ns(&prep);
        let change = bar * 4 / 5;
        let script = [(0, Step::Chord(Chord::new(0, 0))), (bar / 2 + 1_000_000, Step::Button(Button::Main(0))),
                      (change, Step::Chord(Chord::new(5, 0)))];
        let (e, rec) = run(prep, &script, change + 1);
        assert_eq!(e.snapshot(change).cur, Some(SectionId::Fill(0)));
        let (ranges, bends) = ch14_bends(&rec);
        assert!(ranges.len() >= 4 && ranges.iter().all(|&r| r == 14), "bend ranges sent: {ranges:?}");
        let entry = 3 * bar / 4;
        assert!(bends.iter().any(|b| b.0 == entry && b.1 == out_bend(1.0, 0.0)), "SInt at the entry");
        assert_eq!(bends.iter().rev().find(|b| b.0 < change).unwrap(), &(entry, out_bend(2.0, 0.0)), "then the Fill's range");
        assert_eq!(bends.last().unwrap(), &(change, out_bend(2.0, 5.0)), "C to F: bent up 5");
        assert!(rec.sounding_at(change + 1).contains(&(13, 65)));
    }

    /// The pitch shift a part can take leaves room for its patterns' bends in the
    /// narrowest range it can have: with a channel setup of 24 and a pattern that sets 2,
    /// the part may go out with 12, so it shifts by 12 at most, not 24.
    #[test]
    fn shift_room_fits_the_narrowest_range() {
        let mut style = bend_style();
        style.init = vec![Ev::Cc { ch: 13, cc: 101, val: 0 }, Ev::Cc { ch: 13, cc: 100, val: 0 }, Ev::Cc { ch: 13, cc: 6, val: 24 }];
        let p = Prepared::new(&style);
        assert_eq!((p.bend_range[13], p.pat_bend_max[13], p.shift_room[13]), (24, 0, 12));
        let p = Prepared::new(&bend_style());
        assert_eq!((p.bend_range[13], p.pat_bend_max[13], p.shift_room[13]), (2, 2, 12));
    }

    /// A key two voices share (a muted twin) votes once for the part's bend. Ch 13 (Root
    /// Fixed, Chord, Pitch Shift) holds C3 twice, E3 and E4; from C to Cm, both Es go down
    /// a semitone and C stays. Counted per voice it would be a 2-2 tie, won by no bend; per
    /// key, the Es carry it: the part bends down 1, and only C is struck again.
    #[test]
    fn muted_twin_votes_once() {
        use crate::sff::{Section, TimedEv};
        let at = |tick, ev| TimedEv { tick, ev };
        let mut events = Vec::new();
        for key in [60, 60, 64, 76] {
            events.push(at(0, Ev::NoteOn { ch: 12, key, vel: 100 }));
            events.push(at(1900, Ev::NoteOff { ch: 12, key }));
        }
        let id = SectionId::Main(0);
        let style = Style {
            name: "twins".into(),
            format: String::new(),
            ppq: 480,
            tempo_us: 500_000,
            timesig: (4, 4),
            init: vec![],
            sections: [(id, Section { id, start: 0, len: 1920, events })].into(),
            casm: vec![],
            ots: vec![],
            other_chunks: vec![],
        };
        let prep = Box::new(Prepared::new(&style));
        let bar = bar_ns(&prep);
        let script = [(0, Step::Chord(Chord::new(0, 0))), (bar / 2, Step::Chord(Chord::new(0, 8)))];
        let (e, rec) = run(prep, &script, bar / 2 + 1);
        assert_eq!(rec.retunes.last().map(|r| (r.1, r.2, r.3)), Some((bar / 2, 12, -1)));
        let struck: Vec<u8> = e.retriggered.iter().filter(|r| r.0 == bar / 2).map(|r| r.2).collect();
        assert_eq!(struck, vec![61], "C3 sent a semitone up under the bend");
        let mut sounding = rec.sounding_at(bar / 2 + 1);
        sounding.sort();
        assert_eq!(sounding, vec![(12, 60), (12, 63), (12, 75)]);
    }

    fn ticking_away() -> Option<Box<Prepared>> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/TickingAway.T162.sty");
        p.exists().then(|| Box::new(Prepared::new(&Style::load(&p).unwrap())))
    }

    /// TickingAway's bass (Root Trans, Pitch Shift to Root) holds one note through four
    /// bars. From G1+8 to D1+5 it bends down a fourth, with no new attack, in a 12-semitone
    /// bend range; the next note after the bend is sent at its true pitch, straight.
    #[test]
    fn pitch_shift_bends_the_held_bass() {
        let Some(prep) = ticking_away() else { return };
        let bar = bar_ns(&prep);
        let script = [(0, Step::Chord(Chord::new(7, 30))), (2 * bar, Step::Chord(Chord::new(2, 31))),
                      (4 * bar, Step::Chord(Chord::new(0, 0)))];
        let (_, rec) = run(prep, &script, 5 * bar);
        const BASS: u8 = 10;
        let rpn: Vec<u8> = rec.out.iter().filter(|(_, m)| m[0] == 0xB0 | BASS && [101, 100, 6].contains(&m[1])).map(|(_, m)| m[2]).collect();
        assert!(rpn.windows(3).any(|w| w == [0, 0, 12]), "bass bend range not set to 12: {rpn:?}");
        let at_change: Vec<&Vec<u8>> = rec.out.iter().filter(|(t, m)| *t == 2 * bar && m[0] & 0x0F == BASS).map(|(_, m)| m).collect();
        assert!(!at_change.iter().any(|m| m[0] & 0xE0 == 0x80), "bass retriggered: {at_change:?}");
        // G1 (43) down 5 to D1 (38): centre - 5/12 of the half range.
        let v = 0x2000 - (5.0f64 * 8192.0 / 12.0).round() as u16;
        assert!(at_change.iter().any(|m| **m == [0xE0 | BASS, (v & 0x7F) as u8, (v >> 7) as u8]), "{at_change:?}");
        assert!(rec.sounding_at(2 * bar + 1).contains(&(BASS, 38)));
        // Under C the next bass note starts straight, at its own pitch.
        let first_c = rec.pitch_ons().into_iter().find(|&(t, ch, _)| t >= 4 * bar && ch == BASS).unwrap();
        assert_eq!(first_c.2 % 12, 0);
        let last_bend = rec.out.iter().rev().find(|(t, m)| *t <= first_c.0 && m[0] == 0xE0 | BASS).unwrap();
        assert_eq!(last_bend.1, vec![0xE0 | BASS, 0, 0x40]);
    }
}

#[cfg(test)]
mod transpose {
    use super::*;
    use crate::engine::{shift_key, Snapshot};
    use crate::sff::Style;

    fn funky() -> Option<Box<Prepared>> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/FunkyFinger.S930.STY");
        path.exists().then(|| Box::new(Prepared::new(&Style::load(&path).unwrap())))
    }

    fn bar_ns(p: &Prepared) -> u64 {
        (60e9 / p.bpm * (p.tpb as f64 / p.ppq as f64)) as u64
    }

    fn snap(e: &Engine) -> Snapshot {
        e.snapshot(0)
    }

    /// Keyboard transpose +2 with C fingered plays exactly what fingering D plays.
    #[test]
    fn keyboard_transpose_moves_chord_root() {
        let Some(p) = funky() else { return };
        let end = bar_ns(&p) * 4;
        let c7_over_e = Chord { root: 0, ty: 19, bass: Some(4) };
        let (e, a) = run(p, &[(0, Step::Transpose(Transpose::new(2, 0))), (0, Step::Chord(c7_over_e)), (end / 2, Step::Chord(Chord::new(9, 8)))], end);
        let s = snap(&e);
        assert_eq!(s.chord, Some(Chord::new(11, 8)), "Am + 2 = Bm");
        assert_eq!(s.played, Some(Chord::new(9, 8)));
        let (_, b) = run(funky().unwrap(), &[(0, Step::Chord(Chord { root: 2, ty: 19, bass: Some(6) })), (end / 2, Step::Chord(Chord::new(11, 8)))], end);
        assert!(a.out.len() > 50);
        assert_eq!(a.out, b.out);
    }

    /// Changing Keyboard transpose with a chord held is the same as playing the moved chord then.
    #[test]
    fn keyboard_transpose_change_moves_held_chord() {
        let Some(p) = funky() else { return };
        let bar = bar_ns(&p);
        let (_, a) = run(p, &[(0, Step::Chord(Chord::new(0, 0))), (bar + bar / 3, Step::Transpose(Transpose::new(5, 0)))], bar * 3);
        let (_, b) = run(funky().unwrap(), &[(0, Step::Chord(Chord::new(0, 0))), (bar + bar / 3, Step::Chord(Chord::new(5, 0)))], bar * 3);
        assert!(a.out.len() > 50);
        assert_eq!(a.out, b.out);
    }

    /// Master transpose shifts every pitched style note and leaves drum parts alone. The chord
    /// the style follows (and shows) is unchanged.
    #[test]
    fn master_transpose_shifts_output_not_drums() {
        let Some(p) = funky() else { return };
        let kit = p.kit;
        let end = bar_ns(&p) * 4;
        let script = |m: i8| vec![(0, Step::Transpose(Transpose::new(0, m))), (0, Step::Chord(Chord::new(9, 8))), (end / 2, Step::Chord(Chord::new(5, 0)))];
        let (e, a) = run(p, &script(-3), end);
        assert_eq!(snap(&e).chord, Some(Chord::new(5, 0)));
        let (_, b) = run(funky().unwrap(), &script(0), end);
        assert_eq!(a.out.len(), b.out.len());
        let (mut drums, mut pitched) = (0, 0);
        for ((ta, ma), (tb, mb)) in a.out.iter().zip(&b.out) {
            assert_eq!(ta, tb);
            let ch = mb[0] & 0x0F;
            if matches!(mb[0] & 0xF0, 0x80 | 0x90) && !kit[ch as usize] {
                assert_eq!((ma[0], ma[1], ma[2]), (mb[0], shift_key(mb[1], -3), mb[2]));
                pitched += 1;
            } else {
                assert_eq!(ma, mb);
                drums += matches!(mb[0] & 0xF0, 0x90) as u32;
            }
        }
        assert!(drums > 10 && pitched > 10, "drums {drums} pitched {pitched}");
    }

    /// A Master change mid-note: sounding notes keep their pitch and are released at it.
    #[test]
    fn master_change_mid_note_releases_old_pitch() {
        let Some(p) = funky() else { return };
        let bar = bar_ns(&p);
        let script = [(0, Step::Chord(Chord::new(0, 0))), (bar / 3, Step::Transpose(Transpose::new(0, 7))), (bar * 2 + bar / 5, Step::Transpose(Transpose::new(-4, -12)))];
        let (mut e, mut rec) = run(p, &script, bar * 3);
        e.stop(&mut rec);
        let mut on = std::collections::HashMap::<(u8, u8), i32>::new();
        for (_, m) in &rec.out {
            match m[0] & 0xF0 {
                0x90 => *on.entry((m[0] & 0xF, m[1])).or_default() += 1,
                0x80 => *on.entry((m[0] & 0xF, m[1])).or_default() -= 1,
                _ => {}
            }
        }
        assert!(on.values().all(|&n| n == 0), "{on:?}");
    }

    /// Stop Accompaniment follows a Keyboard transpose change with the band stopped.
    #[test]
    fn stop_accompaniment_follows_keyboard_transpose() {
        let Some(p) = funky() else { return };
        let mut e = Engine::new(p);
        let mut rec = Recorder::default();
        e.button(Button::SyncStart, 0, &mut rec);
        e.button(Button::StopAcmp, 0, &mut rec);
        e.set_chord(Chord::new(0, 0), 1, &mut rec);
        rec.out.clear();
        e.set_transpose(Transpose::new(-1, 2), 2, &mut rec);
        // Chord is now B; the bass sounds B + 2 = C#.
        assert!(rec.out.iter().any(|(_, m)| m[0] == 0x9A && m[1] % 12 == 1), "{:?}", rec.out);
        assert_eq!(snap(&e).chord, Some(Chord::new(11, 0)));
    }

    /// With the band stopped and the Stop Accompaniment notes already silenced, a Keyboard
    /// change moves the remembered chord but does not sound it again.
    #[test]
    fn stop_accompaniment_silent_stays_silent() {
        let Some(p) = funky() else { return };
        let mut e = Engine::new(p);
        let mut rec = Recorder::default();
        e.button(Button::SyncStart, 0, &mut rec);
        e.button(Button::StopAcmp, 0, &mut rec);
        e.set_chord(Chord::new(0, 0), 1, &mut rec);
        e.button(Button::StartStop, 2, &mut rec);
        e.button(Button::StartStop, 3, &mut rec);
        rec.out.clear();
        e.set_transpose(Transpose::new(4, 0), 4, &mut rec);
        assert!(!rec.out.iter().any(|(_, m)| m[0] & 0xF0 == 0x90), "{:?}", rec.out);
        assert_eq!(snap(&e).chord, Some(Chord::new(4, 0)));
    }

    /// A part whose voice is a drum/SFX kit (bank MSB 126/127) outside the drum channels is
    /// left alone by Master transpose, like the drum parts.
    #[test]
    fn master_transpose_skips_kit_voice_on_any_part() {
        use crate::sff::{Ev, Section, SectionId, TimedEv};
        let note = |tick, ch, on| TimedEv { tick, ev: if on { Ev::NoteOn { ch, key: 60, vel: 100 } } else { Ev::NoteOff { ch, key: 60 } } };
        let style = Style {
            name: "kit".into(),
            format: String::new(),
            ppq: 480,
            tempo_us: 500_000,
            timesig: (4, 4),
            init: vec![
                Ev::Cc { ch: 12, cc: 0, val: 0 },
                Ev::Pc { ch: 12, prog: 0 },
                Ev::Cc { ch: 13, cc: 0, val: 126 },
                Ev::Pc { ch: 13, prog: 0 },
            ],
            sections: [(SectionId::Main(0), Section {
                id: SectionId::Main(0),
                start: 0,
                len: 1920,
                events: vec![note(0, 12, true), note(0, 13, true), note(480, 12, false), note(480, 13, false)],
            })]
            .into(),
            casm: vec![],
            ots: vec![],
            other_chunks: vec![],
        };
        let p = Prepared::new(&style);
        let kits: Vec<usize> = (0..16).filter(|&d| p.kit[d]).collect();
        assert_eq!(kits, vec![8, 9, 13]);
        let ons = |m: i8| {
            let (_, rec) = run(Box::new(Prepared::new(&style)), &[(0, Step::Transpose(Transpose::new(0, m))), (0, Step::Chord(Chord::new(0, 0)))], 500_000_000);
            rec.out.iter().filter(|(_, m)| m[0] & 0xF0 == 0x90).map(|(_, m)| (m[0] & 0xF, m[1])).collect::<Vec<_>>()
        };
        let (plain, moved) = (ons(0), ons(5));
        assert_eq!(plain.len(), 2);
        let key = |v: &[(u8, u8)], ch| v.iter().find(|n| n.0 == ch).unwrap().1;
        assert_eq!(key(&moved, 12), key(&plain, 12) + 5);
        assert_eq!(key(&moved, 13), key(&plain, 13));
    }
}

#[cfg(test)]
mod style_switch {
    use super::*;
    use crate::sff::Style;

    /// Switching style mid-bend must re-centre the bend (TenorToTheMAX bends Chord 2 constantly).
    #[test]
    fn style_change_recentres_bends() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2");
        let (a, b) = (dir.join("TenorToTheMAX.S930.STY"), dir.join("FunkyFinger.S930.STY"));
        if !a.exists() || !b.exists() {
            return;
        }
        let pa = Box::new(Prepared::new(&Style::load(&a).unwrap()));
        let pb = Box::new(Prepared::new(&Style::load(&b).unwrap()));
        let bar = (60e9 / pa.bpm * (pa.tpb as f64 / pa.ppq as f64)) as u64;
        // Stop right after the first bend away from centre.
        let (_, full) = run(pa, &[(0, Step::Chord(Chord::new(0, 0)))], bar * 8);
        let (t_bend, status) = full
            .out
            .iter()
            .find(|(_, m)| m[0] & 0xF0 == 0xE0 && (m[1], m[2]) != (0, 0x40))
            .map(|(t, m)| (*t, m[0]))
            .expect("expected the style to bend a part");
        let pa = Box::new(Prepared::new(&Style::load(&a).unwrap()));
        let (mut e, mut rec) = run(pa, &[(0, Step::Chord(Chord::new(0, 0)))], t_bend);
        rec.out.clear();
        rec.now = t_bend + 1;
        let _old = e.load(pb, t_bend + 1, &mut rec);
        let last = rec.out.iter().rev().find(|(_, m)| m[0] == status).map(|(_, m)| (m[1], m[2]));
        assert_eq!(last, Some((0, 0x40)));
    }
}

#[cfg(test)]
mod manual_bass {
    use super::*;
    use crate::sff::Style;

    fn style() -> Option<Box<Prepared>> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return None;
        }
        Some(Box::new(Prepared::new(&Style::load(&p).unwrap())))
    }

    fn bass_ons(rec: &Recorder, from: u64, to: u64) -> usize {
        rec.out.iter().filter(|(t, m)| *t >= from && *t < to && m[0] == 0x9A && m[2] > 0).count()
    }

    /// Manual Bass mutes the Style's Bass part (ch 11) from the moment it is turned on,
    /// cuts the bass note that is sounding, and gives the part back when turned off.
    #[test]
    fn mutes_the_style_bass_part() {
        let Some(prep) = style() else { return };
        let bar = (60e9 / prep.bpm * (prep.tpb as f64 / prep.ppq as f64)) as u64;
        let (_, plain) = run(prep, &[(0, Step::Chord(Chord::new(0, 0)))], 4 * bar);
        assert!(bass_ons(&plain, 0, 4 * bar) > 0, "Main A should play the Bass part");

        let script = [
            (0, Step::Chord(Chord::new(0, 0))),
            (bar + bar / 2, Step::ManualBass(true)),
            (3 * bar, Step::ManualBass(false)),
        ];
        let (_, rec) = run(style().unwrap(), &script, 4 * bar);
        assert_eq!(bass_ons(&rec, 0, bar), bass_ons(&plain, 0, bar));
        assert_eq!(bass_ons(&rec, bar + bar / 2, 3 * bar), 0, "Bass part must be silent under Manual Bass");
        assert!(bass_ons(&rec, 3 * bar, 4 * bar) > 0, "Bass part must come back");
        // Every bass note started before the switch was released by it.
        let ons = bass_ons(&rec, 0, bar + bar / 2);
        let offs = rec.out.iter().filter(|(t, m)| *t <= bar + bar / 2 && (m[0] == 0x8A || (m[0] == 0x9A && m[2] == 0))).count();
        assert_eq!(ons, offs);
        // The other parts are untouched.
        let others = |r: &Recorder| r.out.iter().filter(|(_, m)| m[0] & 0xF0 == 0x90 && m[0] != 0x9A && m[2] > 0).count();
        assert_eq!(others(&rec), others(&plain));
    }

    /// Stop Accompaniment's bass note goes quiet too; its chord still sounds.
    #[test]
    fn mutes_stop_accompaniment_bass() {
        let Some(prep) = style() else { return };
        let script = [
            (0, Step::Button(Button::SyncStart)),
            (0, Step::Button(Button::StopAcmp)),
            (0, Step::ManualBass(true)),
            (1_000, Step::Chord(Chord::new(0, 0))),
        ];
        let (_, rec) = run(prep, &script, 2_000);
        assert_eq!(bass_ons(&rec, 0, 2_000), 0);
        assert!(rec.out.iter().any(|(_, m)| m[0] == 0x9D && m[2] > 0), "Pad still sounds the chord");
    }
}

#[cfg(test)]
mod mixer {
    use super::*;
    use crate::engine::GM_VOLUME;
    use crate::sff::Style;

    fn prep(name: &str) -> Option<Box<Prepared>> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2").join(name);
        p.exists().then(|| Box::new(Prepared::new(&Style::load(&p).unwrap())))
    }

    /// The last CC7 the style's init (SInt) sets on each part, or the GM default (for a
    /// style whose parts all keep their own channel).
    fn init_levels(s: &Style) -> [u8; 8] {
        let mut v = [GM_VOLUME; 8];
        for ev in &s.init {
            if let crate::sff::Ev::Cc { ch: ch @ 8..=15, cc: 7, val } = *ev {
                v[ch as usize - 8] = val;
            }
        }
        v
    }

    /// The last CC7 sent on each part channel.
    fn sent_levels(rec: &Recorder) -> [Option<u8>; 8] {
        let mut v = [None; 8];
        for (_, m) in &rec.out {
            if m.len() == 3 && m[0] & 0xF0 == 0xB0 && m[1] == 7 && m[0] & 0x0F >= 8 {
                v[(m[0] & 0x0F) as usize - 8] = Some(m[2]);
            }
        }
        v
    }

    fn cc7_count(rec: &Recorder, part: u8) -> usize {
        rec.out.iter().filter(|(_, m)| m.len() == 3 && m[0] == 0xB8 + part && m[1] == 7).count()
    }

    /// Play from `from` to `to` (ns) in 1 ms steps.
    fn play(e: &mut Engine, rec: &mut Recorder, from: u64, to: u64) {
        let mut t = from;
        while t <= to {
            rec.now = t;
            e.process(t, rec);
            t += 1_000_000;
        }
    }

    fn bar_ns(p: &Prepared) -> u64 {
        (60e9 / p.bpm * (p.tpb as f64 / p.ppq as f64)) as u64
    }

    /// Loading a style sets each part fader to the style's own CC7 (GM 100 where it has
    /// none), and the init sends exactly those values.
    #[test]
    fn style_load_sets_faders_from_style_cc7() {
        // AustinCityBlues leaves some parts without a CC7.
        let Some(p) = prep("AustinCityBlues.S930.STY") else { return };
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/AustinCityBlues.S930.STY");
        let want = init_levels(&Style::load(&path).unwrap());
        assert!(want.contains(&GM_VOLUME) && want.iter().any(|&v| v != GM_VOLUME), "{want:?}");
        assert_eq!(p.mix, want);
        let mut e = Engine::new(p);
        assert_eq!(e.snapshot(0).volumes, want);
        let mut rec = Recorder::default();
        e.send_init(&mut rec);
        assert_eq!(sent_levels(&rec), want.map(Some), "every part gets its level, the defaults too");
        for part in 0..8 {
            assert_eq!(cc7_count(&rec, part), 1, "one CC7 per part, not the style's then ours");
        }
    }

    /// A fader value goes out as that part's CC7, unchanged: no scaling by the style level.
    #[test]
    fn fader_sends_its_value_as_cc7() {
        let Some(p) = prep("FunkyFinger.S930.STY") else { return };
        let mut e = Engine::new(p);
        let mut rec = Recorder::default();
        for v in [0u8, 1, 64, 100, 127] {
            e.set_volume(2, v, &mut rec); // part 3 = Bass (ch 11)
            assert_eq!(rec.out.last().unwrap().1, vec![0xBA, 7, v]);
            assert_eq!(e.snapshot(0).volumes[2], v);
        }
        // A restart re-sends the channel setup: the mixer's value wins over the style's CC7.
        rec.out.clear();
        e.send_init(&mut rec);
        assert_eq!(sent_levels(&rec)[2], Some(127));
        assert_eq!(cc7_count(&rec, 2), 1);
    }

    /// A section change keeps a fader the player moved, even where the new section carries
    /// its own CC7 for that part; parts the player left alone follow the pattern's CC7.
    #[test]
    fn section_change_keeps_user_fader() {
        // SmoothItOver: every section sets part levels at its start, some unlike the init.
        let Some(p) = prep("SmoothItOver.S930.STY") else { return };
        let bar = bar_ns(&p);
        let init = p.mix;
        let mut e = Engine::new(p);
        let mut rec = Recorder::default();
        e.set_chord(Chord::new(0, 0), 0, &mut rec);
        // Walk every Main and fill; note which parts the patterns set.
        let mut t = 0;
        let mut moved = None;
        for m in [0u8, 1, 2, 3, 3, 0, 1, 2] {
            e.button(Button::Main(m), t, &mut rec);
            play(&mut e, &mut rec, t, t + 3 * bar);
            t += 3 * bar;
            let s = e.snapshot(t);
            moved = moved.or((0..8).find(|&i| s.volumes[i] != init[i]));
        }
        let part = moved.expect("expected a pattern CC7 to move a fader");
        // The fader shows what the channel was last sent.
        assert_eq!(sent_levels(&rec)[part], Some(e.snapshot(t).volumes[part]));

        // Now the player sets that part, and every section plays again.
        e.set_volume(part as u8, 40, &mut rec);
        rec.out.clear();
        for m in [0u8, 1, 2, 3, 3, 0, 1, 2] {
            e.button(Button::Main(m), t, &mut rec);
            play(&mut e, &mut rec, t, t + 3 * bar);
            t += 3 * bar;
        }
        assert_eq!(e.snapshot(t).volumes[part], 40);
        assert_eq!(cc7_count(&rec, part as u8), 0, "no section may resend its own level");
        // Stop and start again: the init goes out with the mixer's level.
        e.button(Button::StartStop, t, &mut rec);
        rec.out.clear();
        e.button(Button::StartStop, t, &mut rec);
        assert_eq!(sent_levels(&rec)[part], Some(40));
    }

    /// A start re-plays the style's channel setup: a part the player has not moved goes back
    /// to the style's own level instead of keeping a level the last Intro/Ending pattern set.
    #[test]
    fn restart_restores_untouched_style_levels() {
        // TickingAway: Intro B and Ending C set part 2 (ch 10) to 76 against an init of 90,
        // and no other section sets it.
        let Some(p) = prep("TickingAway.T162.sty") else { return };
        let bar = bar_ns(&p);
        let init = p.mix[1];
        let mut e = Engine::new(p);
        let mut rec = Recorder::default();
        e.set_chord(Chord::new(0, 0), 0, &mut rec);
        e.set_volume(4, 33, &mut rec); // a part the player moved keeps its value
        // Main A, then Ending C to the end: nothing after the Ending puts part 2 back.
        play(&mut e, &mut rec, 0, bar / 2);
        e.button(Button::Ending(2), bar / 2, &mut rec);
        let t = 8 * bar;
        play(&mut e, &mut rec, bar / 2, t);
        let s = e.snapshot(t);
        assert!(!s.running);
        assert_ne!(s.volumes[1], init, "the Ending should have moved part 2");
        rec.out.clear();
        e.button(Button::Intro(0), t, &mut rec);
        e.button(Button::StartStop, t, &mut rec);
        assert_eq!(sent_levels(&rec)[1], Some(init));
        assert_eq!(sent_levels(&rec)[4], Some(33));
        assert_eq!(e.snapshot(t).volumes[1], init);
        assert_eq!(e.snapshot(t).volumes[4], 33);
    }

    /// Changing style resets every fader to the new style's levels.
    #[test]
    fn style_change_resets_faders() {
        let (Some(a), Some(b)) = (prep("FunkyFinger.S930.STY"), prep("SlowWalker.T552.sty")) else { return };
        let want = b.mix;
        let mut e = Engine::new(a);
        let mut rec = Recorder::default();
        for p in 0..8 {
            e.set_volume(p, 11, &mut rec);
        }
        e.set_chord(Chord::new(0, 0), 0, &mut rec);
        rec.out.clear();
        let _old = e.load(b, 1_000_000, &mut rec);
        assert_eq!(e.snapshot(1_000_000).volumes, want);
        assert_eq!(sent_levels(&rec), want.map(Some));
    }

    /// A one-bar Main A and Main B at 120 bpm whose SInt sets part 5 (ch 13): bank,
    /// program 5, CC7 90, pan, an XG part volume and dry level, and part 6 (ch 14) CC7 80,
    /// with GM and XG System On and an XG reverb type. Main A changes part 5's voice and
    /// level half way through the bar.
    fn sint_style() -> Style {
        use crate::sff::{Ev, Section, SectionId, TimedEv};
        let at = |tick, ev| TimedEv { tick, ev };
        let bar = |id, extra: Vec<TimedEv>| {
            let mut events = vec![
                at(0, Ev::NoteOn { ch: 12, key: 60, vel: 100 }),
                at(480, Ev::NoteOff { ch: 12, key: 60 }),
            ];
            events.extend(extra);
            events.sort_by_key(|e| e.tick);
            (id, Section { id, start: 0, len: 1920, events })
        };
        Style {
            name: "sint".into(),
            format: String::new(),
            ppq: 480,
            tempo_us: 500_000,
            timesig: (4, 4),
            init: vec![
                Ev::Sysex(vec![0xF0, 0x7E, 0x7F, 0x09, 0x01, 0xF7]),
                Ev::Sysex(vec![0xF0, 0x43, 0x10, 0x4C, 0x00, 0x00, 0x7E, 0x00, 0xF7]),
                Ev::Sysex(vec![0xF0, 0x43, 0x10, 0x4C, 0x02, 0x01, 0x00, 0x01, 0x10, 0xF7]),
                Ev::Sysex(vec![0xF0, 0x43, 0x10, 0x4C, 0x08, 0x0C, 0x0B, 0x20, 0xF7]),
                Ev::Sysex(vec![0xF0, 0x43, 0x10, 0x4C, 0x08, 0x0C, 0x11, 0x7F, 0xF7]),
                Ev::Cc { ch: 12, cc: 32, val: 112 },
                Ev::Cc { ch: 12, cc: 0, val: 0 },
                Ev::Pc { ch: 12, prog: 5 },
                Ev::Cc { ch: 12, cc: 7, val: 90 },
                Ev::Cc { ch: 12, cc: 10, val: 40 },
                Ev::Cc { ch: 13, cc: 7, val: 80 },
            ],
            sections: [
                bar(SectionId::Main(0), vec![
                    at(960, Ev::Pc { ch: 12, prog: 9 }),
                    at(960, Ev::Cc { ch: 12, cc: 7, val: 50 }),
                ]),
                bar(SectionId::Main(1), vec![]),
            ]
            .into(),
            casm: vec![],
            ots: vec![],
            other_chunks: vec![],
        }
    }

    /// A bend range (RPN 0) message: the engine sets one on every part that follows chords.
    fn is_rpn(m: &[u8]) -> bool {
        m.len() == 3 && m[0] & 0xF0 == 0xB0 && matches!(m[1], 6 | 38 | 100 | 101)
    }

    /// The SInt is sent structured: no system resets, bank before program, the part's XG
    /// parameters after its program change, the effect SysEx last, and no part volume but
    /// the mixer's (neither CC7 nor the XG part volume). Each part that follows chords gets
    /// its bend range set with the part setup, before the effects, so a section change
    /// sends it again.
    #[test]
    fn init_is_structured_and_mixer_owns_volume() {
        let p = Prepared::new(&sint_style());
        let all: Vec<Vec<u8>> = p.init.iter().map(|m| m.to_vec()).collect();
        let (rpn, msgs): (Vec<_>, Vec<_>) = all.iter().cloned().partition(|m| is_rpn(m));
        assert_eq!(rpn.len(), 6 * 6, "RPN 0 on ch 11-16");
        assert!(all.iter().rposition(|m| is_rpn(m)).unwrap() < p.init_resend);
        assert_eq!(msgs, vec![
            vec![0xBC, 0, 0],
            vec![0xBC, 32, 112],
            vec![0xCC, 5],
            vec![0xBC, 10, 40],
            vec![0xF0, 0x43, 0x10, 0x4C, 0x08, 0x0C, 0x11, 0x7F, 0xF7],
            vec![0xF0, 0x43, 0x10, 0x4C, 0x02, 0x01, 0x00, 0x01, 0x10, 0xF7],
        ]);
        assert_eq!(p.voices[12], Some((0, 112, 5)));
        assert_eq!(p.mix, [100, 100, 100, 100, 90, 80, 100, 100]);
    }

    /// Every section change plays the SInt again: a voice or level the last section's
    /// pattern changed goes back to the style's, except a fader the player moved, which
    /// keeps its level and gets no CC7. A section repeating itself is not a change.
    #[test]
    fn section_change_reapplies_init() {
        let p = Box::new(Prepared::new(&sint_style()));
        let bar = bar_ns(&p);
        let mut e = Engine::new(p);
        let mut rec = Recorder::default();
        e.set_chord(Chord::new(0, 0), 0, &mut rec);
        e.set_volume(5, 33, &mut rec); // part 6 (ch 14)
        // Main A plays twice: the repeat is not a section change.
        play(&mut e, &mut rec, 0, bar + bar / 2);
        let sent = |rec: &Recorder, from: u64, m: &[u8]| rec.out.iter().filter(|(t, x)| *t >= from && x == m).count();
        assert_eq!(sent(&rec, 1, &[0xCC, 5]), 0, "no SInt at the Main A repeat");
        assert_eq!(e.snapshot(bar + bar / 2).volumes[4], 50, "the pattern's CC7 moves the untouched fader");
        // Main B at the next bar.
        e.button(Button::Main(1), bar + bar / 2, &mut rec);
        play(&mut e, &mut rec, bar + bar / 2, 2 * bar + bar / 4);
        let from = 2 * bar - 1_000_000;
        assert_eq!(sent(&rec, from, &[0xCC, 5]), 1, "Main B gets the style's voice back");
        assert_eq!(sent(&rec, from, &[0xBC, 7, 90]), 1, "and the style's level on the untouched part");
        assert_eq!(sent(&rec, from, &[0xF0, 0x43, 0x10, 0x4C, 0x02, 0x01, 0x00, 0x01, 0x10, 0xF7]), 0,
            "the effect setup is not re-sent: no pattern changes it");
        assert_eq!(sent(&rec, from, &[0xF0, 0x43, 0x10, 0x4C, 0x08, 0x0C, 0x11, 0x7F, 0xF7]), 1, "the part's are");
        assert_eq!(cc7_count(&Recorder { out: rec.out.iter().filter(|(t, _)| *t >= from).cloned().collect(), ..Default::default() }, 5), 0,
            "the player's fader is not re-sent");
        let s = e.snapshot(2 * bar + bar / 4);
        assert_eq!((s.volumes[4], s.volumes[5]), (90, 33));
        // Never a system reset, never the XG part volume.
        assert!(!rec.out.iter().any(|(_, m)| m[0] == 0xF0 && (m[1] == 0x7E || m[4] == 0x00 || m[6] == 0x0B)));
        // The SInt goes out before Main B's first note.
        let pc = rec.out.iter().position(|(t, m)| *t >= from && m[..] == [0xCC, 5]).unwrap();
        let note = rec.out.iter().position(|(t, m)| *t >= from && m[0] == 0x9C).unwrap();
        assert!(pc < note);
    }

    /// A program change on a part that uses a drum setup resets that drum setup (Data List,
    /// Drum Setup note), so a section change sends the SInt's drum setup SysEx again, after
    /// the parts' program changes, as Start does. The effect SysEx is still not re-sent.
    #[test]
    fn section_change_resends_drum_setup_after_program_changes() {
        use crate::sff::Ev;
        let mut s = sint_style();
        let drum_mode = vec![0xF0, 0x43, 0x10, 0x4C, 0x08, 0x0C, 0x07, 0x02, 0xF7];
        let reset = vec![0xF0, 0x43, 0x10, 0x4C, 0x00, 0x00, 0x7D, 0x00, 0xF7];
        let level = vec![0xF0, 0x43, 0x10, 0x4C, 0x30, 0x24, 0x02, 0x50, 0xF7];
        let reverb = vec![0xF0, 0x43, 0x10, 0x4C, 0x02, 0x01, 0x00, 0x01, 0x10, 0xF7];
        // In the file the drum setup comes before the part's own setup.
        s.init.insert(2, Ev::Sysex(reset.clone()));
        s.init.insert(3, Ev::Sysex(level.clone()));
        s.init.push(Ev::Sysex(drum_mode.clone()));
        let p = Box::new(Prepared::new(&s));
        let order = |msgs: &[Vec<u8>]| -> Vec<usize> {
            [&[0xCC, 5][..], &drum_mode, &reset, &level, &reverb]
                .iter()
                .map(|w| msgs.iter().position(|m| m == w).unwrap_or(usize::MAX))
                .collect()
        };
        let init: Vec<Vec<u8>> = p.init.iter().map(|m| m.to_vec()).filter(|m| !is_rpn(m)).collect();
        assert_eq!(order(&init), vec![2, 5, 6, 7, 8], "PC, part mode, drum setup, then effects");

        let bar = bar_ns(&p);
        let mut e = Engine::new(p);
        let mut rec = Recorder::default();
        e.set_chord(Chord::new(0, 0), 0, &mut rec);
        play(&mut e, &mut rec, 0, bar + bar / 2);
        e.button(Button::Main(1), bar + bar / 2, &mut rec);
        play(&mut e, &mut rec, bar + bar / 2, 2 * bar + bar / 4);
        let from = 2 * bar - 1_000_000;
        let change: Vec<Vec<u8>> = rec.out.iter().filter(|(t, _)| *t >= from).map(|(_, m)| m.clone()).collect();
        let o = order(&change);
        assert!(o[0] < o[1] && o[1] < o[2] && o[2] < o[3], "PC, part mode, then the drum setup: {o:?}");
        assert_eq!(o[4], usize::MAX, "no effect SysEx at a section change");
        assert_eq!(change.iter().filter(|m| **m == level).count(), 1);
    }

    /// AustinCityBlues sets up a drum kit's notes: Start and every section change send
    /// that drum setup after the program changes that would reset it.
    #[test]
    fn corpus_drum_setup_survives_section_change() {
        let Some(p) = prep("AustinCityBlues.S930.STY") else { return };
        let drum: Vec<Vec<u8>> = p.init.iter().filter(|m| crate::sff::is_drum_setup(m)).map(|m| m.to_vec()).collect();
        assert!(!drum.is_empty());
        let pcs = p.init.iter().filter(|m| m.len() == 2 && m[0] & 0xF0 == 0xC0).count();
        let bar = bar_ns(&p);
        let mut e = Engine::new(p);
        let mut rec = Recorder::default();
        e.set_chord(Chord::new(0, 0), 0, &mut rec);
        let mut t = 0;
        for m in [1u8, 2, 0] {
            play(&mut e, &mut rec, t, t + bar / 2);
            let from = rec.out.len();
            e.button(Button::Main(m), t + bar / 2, &mut rec);
            play(&mut e, &mut rec, t + bar / 2, t + 3 * bar);
            t += 3 * bar;
            // The drum setup goes out whole, after every SInt program change.
            let out = &rec.out[from..];
            let first = out.iter().position(|(_, m)| crate::sff::is_drum_setup(m)).expect("drum setup re-sent");
            let block: Vec<Vec<u8>> = out[first..].iter().take(drum.len()).map(|(_, m)| m.clone()).collect();
            assert_eq!(block, drum);
            let is_pc = |m: &[u8]| m.len() == 2 && m[0] & 0xF0 == 0xC0;
            assert_eq!(out[..first].iter().filter(|(_, m)| is_pc(m)).count(), pcs);
        }
    }

    /// TickingAway's Intro B sets part 2 (ch 10) to 76 against the SInt's 90, and Main A
    /// sets no level: Main A starts from the SInt's 90 again.
    #[test]
    fn section_change_restores_untouched_style_levels() {
        let Some(p) = prep("TickingAway.T162.sty") else { return };
        let bar = bar_ns(&p);
        let init = p.mix[1];
        assert_eq!(init, 90);
        let mut e = Engine::new(p);
        let mut rec = Recorder::default();
        e.button(Button::Intro(1), 0, &mut rec);
        e.set_chord(Chord::new(0, 0), 0, &mut rec);
        play(&mut e, &mut rec, 0, bar / 2);
        assert_eq!(e.snapshot(bar / 2).volumes[1], 76, "Intro B's own level");
        play(&mut e, &mut rec, bar / 2, bar + bar / 2);
        assert_eq!(e.snapshot(bar + bar / 2).cur, Some(crate::sff::SectionId::Main(0)));
        assert_eq!(e.snapshot(bar + bar / 2).volumes[1], init);
        assert_eq!(sent_levels(&rec)[1], Some(init));
    }

    fn t5(name: &str) -> Option<Style> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/T5Style").join(name);
        p.exists().then(|| Style::load(&p).unwrap())
    }

    /// A receiver's voice on `ch` after `msgs`: (bank MSB, LSB, program).
    fn voice_after<'a>(msgs: impl IntoIterator<Item = &'a [u8]>, ch: u8) -> (u8, u8, u8) {
        let (mut msb, mut lsb, mut voice) = (0, 0, (0, 0, 0));
        for m in msgs {
            match *m {
                [s, 0, v] if s == 0xB0 | ch => msb = v,
                [s, 32, v] if s == 0xB0 | ch => lsb = v,
                [s, p] if s == 0xC0 | ch => voice = (msb, lsb, p),
                _ => {}
            }
        }
        voice
    }

    fn effect_parts(p: &Prepared) -> Vec<u8> {
        p.init.iter().filter(|m| crate::sff::xg_effect_part(m).is_some()).map(|m| m[7]).collect()
    }

    /// An insertion or variation effect assigned to a part follows the part to its
    /// destination channel. CountryTwoStep routes source ch 15 to 14 and 16 to 13 (MIDI
    /// numbering); a part the style never routes gets its insertion switched off.
    #[test]
    fn effect_part_assignments_follow_the_routing() {
        use crate::sff::{ChannelRule, Cseg, Ev};
        let mut s = sint_style();
        let rule = |src, dest| ChannelRule { dest_ch: dest, ..ChannelRule::default_for(src) };
        s.casm = vec![Cseg {
            sections: vec!["Main A".into(), "Main B".into()],
            rules: vec![rule(12, 12), rule(14, 13), rule(15, 14)],
        }];
        let ins = |block, part| Ev::Sysex(vec![0xF0, 0x43, 0x10, 0x4C, 0x03, block, 0x0C, part, 0xF7]);
        s.init.extend([
            ins(0, 0x0E),
            ins(1, 0x0F),
            ins(2, 0x0D), // ch 14's own destination is taken by ch 15: unrouted
            ins(3, 0x7F),
            Ev::Sysex(vec![0xF0, 0x43, 0x10, 0x4C, 0x02, 0x01, 0x5B, 0x0F, 0xF7]),
        ]);
        assert_eq!(effect_parts(&Prepared::new(&s)), vec![0x0D, 0x0E, 0x7F, 0x7F, 0x0E]);

        let Some(s) = t5("CountryTwoStep.T151.prs") else { return };
        assert_eq!(effect_parts(&Prepared::new(&s)), vec![0x0D, 0x0C]);
        let Some(s) = t5("JazzWaltzFast.T157.prs") else { return };
        assert_eq!(effect_parts(&Prepared::new(&s)), vec![0x7F], "source ch 12 plays nowhere");
    }

    /// A bank select after the SInt's program change selects no voice. ChartPop1's ch 15
    /// sends CC0 8, CC32 2, PC 3, CC0 104: the voice is 8/2/3, with 104 left pending.
    #[test]
    fn sint_voice_keeps_the_bank_of_its_program_change() {
        let Some(s) = t5("ChartPop1.T160.prs") else { return };
        let p = Prepared::new(&s);
        assert_eq!(p.voices[14], Some((8, 2, 3)));
        let file: Vec<Vec<u8>> = s
            .init
            .iter()
            .filter_map(|e| match *e {
                crate::sff::Ev::Cc { ch: 14, cc, val } => Some(vec![0xBE, cc, val]),
                crate::sff::Ev::Pc { ch: 14, prog } => Some(vec![0xCE, prog]),
                _ => None,
            })
            .collect();
        assert_eq!(voice_after(p.init.iter(), 14), voice_after(file.iter().map(|m| &m[..]), 14));
        assert_eq!(voice_after(p.init.iter(), 14), (8, 2, 3));
        let Some(s) = t5("Let'sFunk.T161.prs") else { return };
        assert_eq!(Prepared::new(&s).voices[14], Some((104, 5, 0)));
    }

    /// sint_style with a Fill A that changes part 5's voice on its first beat and plays a
    /// note on beat 4.
    fn sint_style_with_fill() -> Style {
        use crate::sff::{Ev, Section, SectionId, TimedEv};
        let mut s = sint_style();
        let id = SectionId::Fill(0);
        let events = vec![
            TimedEv { tick: 0, ev: Ev::Cc { ch: 12, cc: 0, val: 8 } },
            TimedEv { tick: 0, ev: Ev::Pc { ch: 12, prog: 20 } },
            TimedEv { tick: 1440, ev: Ev::NoteOn { ch: 12, key: 64, vel: 100 } },
            TimedEv { tick: 1900, ev: Ev::NoteOff { ch: 12, key: 64 } },
        ];
        s.sections.insert(id, Section { id, start: 0, len: 1920, events });
        s
    }

    /// A Fill entering mid-bar skips its first beat's notes but not its voice: the SInt
    /// goes out at the entry, then the Fill's own bank and program change from before the
    /// entry, so its note plays on the Fill's voice, not the SInt's.
    #[test]
    fn mid_bar_fill_plays_its_own_voice() {
        let p = Box::new(Prepared::new(&sint_style_with_fill()));
        let bar = bar_ns(&p);
        let mut e = Engine::new(p);
        let mut rec = Recorder::default();
        e.set_chord(Chord::new(0, 0), 0, &mut rec);
        play(&mut e, &mut rec, 0, bar / 2 + 1_000_000);
        // Just after beat 2: the Fill comes in on beat 3.
        e.button(Button::Main(0), bar / 2 + 1_000_000, &mut rec);
        play(&mut e, &mut rec, bar / 2 + 1_000_000, bar);
        let entry = 3 * bar / 4;
        let at = |want: &dyn Fn(&[u8]) -> bool| rec.out.iter().position(|(t, m)| *t >= entry && want(m));
        let init = at(&|m| m == [0xCC, 5]).expect("the SInt");
        let pc = at(&|m| m == [0xCC, 20]).expect("the Fill's program change");
        let note = at(&|m| m[0] == 0x9C && m[2] > 0).expect("the Fill's note");
        assert!(init < pc && pc < note, "{init} {pc} {note}");
        assert_eq!(rec.out[note].1, vec![0x9C, 64, 100]);
        assert_eq!(voice_after(rec.out[..note].iter().map(|(_, m)| &m[..]), 12), (8, 112, 20));
    }

    /// ChartPop1's Fill A from beat 3: its part on ch 15 plays Fill A's first-beat voice
    /// (104/8/4), not the SInt's.
    #[test]
    fn mid_bar_fill_voice_in_corpus() {
        let Some(s) = t5("ChartPop1.T160.prs") else { return };
        let p = Box::new(Prepared::new(&s));
        let bar = bar_ns(&p);
        let mut e = Engine::new(p);
        let mut rec = Recorder::default();
        e.set_chord(Chord::new(0, 0), 0, &mut rec); // Sync Start
        play(&mut e, &mut rec, 0, bar / 2 + 1_000_000);
        e.button(Button::Main(0), bar / 2 + 1_000_000, &mut rec);
        play(&mut e, &mut rec, bar / 2 + 1_000_000, bar - 1_000_000);
        assert_eq!(e.snapshot(bar - 1_000_000).cur, Some(crate::sff::SectionId::Fill(0)));
        assert_eq!(voice_after(rec.out.iter().map(|(_, m)| &m[..]), 14), (104, 8, 4));
    }

    /// The takeover rule on its own (the master fader uses it directly).
    #[test]
    fn takeover_rule() {
        use crate::engine::Takeover;
        let mut t = Takeover::NEW;
        assert!(!t.waiting());
        assert!(!t.hardware(100, 20), "never reported and far: wait");
        assert!(t.waiting());
        assert!(!t.hardware(100, 97), "3 away: still waiting");
        assert!(t.hardware(100, 98), "within 2: picked up");
        assert!(t.hardware(100, 10), "then it follows");
        t.software_moved(60);
        assert!(t.waiting());
        assert!(!t.hardware(60, 20));
        // Moves in between lost (full ring): a jump across the value still picks up.
        assert!(t.hardware(60, 90));
        t.software_moved(91);
        assert!(!t.waiting(), "the fader is already within 2 of the new value");
        let mut u = Takeover::NEW;
        assert!(u.hardware(100, 101), "first report within 2 picks up at once");
    }

    /// Back from the Panel fader page, each Style fader picks its part up only once it
    /// reaches the part's level: a fader moved elsewhere meanwhile must not jump the part.
    #[test]
    fn style_faders_rebind_after_a_page_switch() {
        use crate::engine::HW_UNKNOWN;
        let Some(p) = prep("TickingAway.T162.sty") else { return };
        let mut e = Engine::new(p);
        let mut rec = Recorder::default();
        e.hw_fader(0, e.snapshot(0).volumes[0], &mut rec);
        e.hw_fader(0, 90, &mut rec);
        e.hw_fader(1, e.snapshot(0).volumes[1], &mut rec);
        let v1 = e.snapshot(0).volumes[1];
        assert_eq!((e.snapshot(0).volumes[0], e.snapshot(0).pickup), (90, 0));
        // On the Panel page fader 1 went down to 10; fader 2 stayed at its part's level.
        let mut hw = [HW_UNKNOWN; 8];
        hw[0] = 10;
        hw[1] = v1;
        e.faders_at(hw);
        assert_eq!(e.snapshot(0).pickup, 0b01, "fader 1 waits, fader 2 still holds its part");
        e.hw_fader(0, 12, &mut rec);
        assert_eq!(e.snapshot(0).volumes[0], 90, "no jump to the fader");
        e.hw_fader(0, 91, &mut rec);
        assert_eq!(e.snapshot(0).volumes[0], 91);
        e.hw_fader(1, v1.saturating_sub(1), &mut rec);
        assert_eq!(e.snapshot(0).volumes[1], v1.saturating_sub(1));
    }

    /// A start keeps a fader the player moved, and its hardware fader stays in control.
    #[test]
    fn start_keeps_moved_fader_under_hardware_control() {
        let Some(p) = prep("TickingAway.T162.sty") else { return };
        let mix = p.mix;
        let mut e = Engine::new(p);
        let mut rec = Recorder::default();
        let part = (0..8).find(|&i| mix[i] > 20).unwrap();
        e.hw_fader(part as u8, mix[part], &mut rec); // picked up, but the player moved nothing
        e.set_volume(part as u8, mix[part], &mut rec);
        assert_eq!(e.snapshot(0).pickup, 0);
        e.hw_fader(part as u8, 5, &mut rec); // player pulls it down
        e.button(Button::StartStop, 0, &mut rec);
        assert_eq!(e.snapshot(0).volumes[part], 5, "a moved fader survives a start");
        assert_eq!(e.snapshot(0).pickup, 0);
    }

    /// Soft takeover: after software moved a fader, the hardware fader does nothing until
    /// it comes within 2 of the value or crosses it; then it follows.
    #[test]
    fn hardware_fader_soft_takeover() {
        let Some(p) = prep("FunkyFinger.S930.STY") else { return };
        let bass = p.mix[2];
        assert!(bass > 20 && bass < 120, "{bass}");
        let mut e = Engine::new(p);
        let mut rec = Recorder::default();
        assert_eq!(e.snapshot(0).pickup, 0, "no marker before the hardware has reported");

        // Hardware far below: ignored, and marked as waiting.
        e.hw_fader(2, 5, &mut rec);
        assert!(rec.out.is_empty());
        assert_eq!(e.snapshot(0).volumes[2], bass);
        assert_eq!(e.snapshot(0).pickup, 1 << 2);
        e.hw_fader(2, bass - 10, &mut rec);
        assert!(rec.out.is_empty(), "still below, not within 2");
        // Crossing the value picks it up, at the hardware position.
        e.hw_fader(2, bass + 5, &mut rec);
        assert_eq!(rec.out.last().unwrap().1, vec![0xBA, 7, bass + 5]);
        assert_eq!(e.snapshot(0).pickup, 0);
        // From then on it follows directly.
        e.hw_fader(2, 3, &mut rec);
        assert_eq!(rec.out.last().unwrap().1, vec![0xBA, 7, 3]);

        // Unknown hardware position (never moved): a first report within 2 picks up at once.
        rec.out.clear();
        let other = e.snapshot(0).volumes[3];
        e.hw_fader(3, other.saturating_sub(2), &mut rec);
        assert_eq!(rec.out.last().unwrap().1, vec![0xBB, 7, other.saturating_sub(2)]);

        // A style load moves the software fader away from the hardware: wait again.
        let Some(q) = prep("CoolRevibed.T552.sty") else { return };
        let new_bass = q.mix[2];
        assert!(new_bass.abs_diff(3) > 2);
        let _old = e.load(q, 0, &mut rec);
        assert_eq!(e.snapshot(0).pickup & (1 << 2), 1 << 2);
        rec.out.clear();
        e.hw_fader(2, 4, &mut rec);
        assert!(rec.out.is_empty(), "hardware must not jump the new level");
        assert_eq!(e.snapshot(0).volumes[2], new_bass);
        e.hw_fader(2, new_bass - 1, &mut rec);
        assert_eq!(rec.out.last().unwrap().1, vec![0xBA, 7, new_bass - 1]);
    }
}

#[cfg(test)]
/// Retrigger Rule pitch shift under a chaotic live performance, checked through a model of
/// the receiver (its RPN 0 bend range and pitch bend per channel), not the engine's own
/// bookkeeping.
mod rtr_chaos {
    use super::*;
    use crate::theory::is_drum_part;

    fn bar_ns(p: &Prepared) -> u64 {
        (60e9 / p.bpm * (p.tpb as f64 / p.ppq as f64)) as u64
    }

    fn follows(ch: u8) -> bool {
        (8..16).contains(&ch) && !is_drum_part(ch)
    }

    enum Act {
        S(Step),
        /// Load the other style (as the live engine thread does between wake-ups).
        Load,
    }

    /// 80 beats of anything a player might do, at uneven moments (on the beat, up to 40 ms
    /// either side of it, or anywhere in the beat): chords of every type with slash chords,
    /// Chord Cancel and rolled chords (a second change 3 ms later), sections, Break, Intro,
    /// Ending, Auto Fill, part toggles, Manual Bass, transposes, tempo, Sync Start/Stop,
    /// Stop Accompaniment and chord release. Also, always: stop and restart mid-pattern,
    /// and a style change while running, 2 ms after a chord change. Then Stop.
    fn chaos(bar: u64, seed: u64) -> (Vec<(u64, Act)>, u64) {
        let mut x = seed;
        let mut rand = |n: u64| {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (x >> 33) % n
        };
        let mut s = vec![(0, Act::S(Step::Chord(Chord::new(0, 0))))];
        let beat = bar / 4;
        for i in 1..80u64 {
            let base = i * beat;
            let jitter = match rand(4) { 0 => 0, 1 => rand(40_000_000), 2 => 0u64.wrapping_sub(rand(40_000_000)), _ => rand(beat) };
            let t = base.wrapping_add(jitter);
            let act = match rand(34) {
                0 => Act::S(Step::Button(Button::Main(rand(4) as u8))),
                1 => Act::S(Step::Button(Button::Break)),
                2 => Act::S(Step::Button(Button::TogglePart(rand(8) as u8))),
                3 => Act::S(Step::ManualBass(rand(2) == 0)),
                4 => Act::S(Step::Transpose(Transpose::new(rand(7) as i8 - 3, rand(7) as i8 - 3))),
                5 => Act::S(Step::Button(Button::Stop)),
                6 => Act::S(Step::Button(Button::StartStop)),
                8 => Act::S(Step::Chord(Chord::new(rand(12) as u8, crate::theory::CANCEL))),
                24 => Act::S(Step::Button(Button::StopAcmp)),
                25 => Act::S(Step::Button(Button::SyncStop)),
                26 if i % 30 == 26 => Act::S(Step::Button(Button::Ending(rand(3) as u8))),
                27 => Act::S(Step::Button(Button::Intro(rand(3) as u8))),
                28 => Act::S(Step::Button(Button::AutoFill)),
                29 => Act::S(Step::Release),
                30 => Act::S(Step::Button(Button::SyncStart)),
                31 => Act::S(Step::Button(if rand(2) == 0 { Button::TempoUp } else { Button::TempoDown })),
                _ => {
                    let c = Chord { root: rand(12) as u8, ty: rand(34) as u8, bass: (rand(4) == 0).then(|| rand(12) as u8) };
                    // Sometimes a second change a few ms later (a rolled chord).
                    if rand(5) == 0 {
                        s.push((t + 3_000_000, Act::S(Step::Chord(Chord::new(rand(12) as u8, rand(34) as u8)))));
                    }
                    Act::S(Step::Chord(c))
                }
            };
            s.push((t, act));
        }
        // Fixed: stop and restart mid-pattern, a style change while running (mid-bar, a few
        // ms after a chord change), and a style change while stopped.
        s.push((5 * bar + bar / 3, Act::S(Step::Button(Button::Stop))));
        s.push((6 * bar + bar / 5, Act::S(Step::Button(Button::StartStop))));
        s.push((10 * bar + bar / 2 + 2_000_000, Act::Load));
        s.push((10 * bar + bar / 2, Act::S(Step::Chord(Chord::new(5, 0)))));
        s.push((21 * bar, Act::S(Step::Button(Button::Stop))));
        s.sort_by_key(|x| x.0);
        // The live engine thread takes at most one chord per wake-up (one atomic word), and
        // wakes for each input as it comes: nothing else lands on a chord's very instant.
        // (A chord and a command in one wake-up can still restrike and cut a note at once;
        // that predates the pitch shift and is not what this test is about.)
        let mut last_chord = u64::MAX;
        s.retain(|(t, a)| !matches!(a, Act::S(Step::Chord(_))) || std::mem::replace(&mut last_chord, *t) != *t);
        let chords: Vec<u64> = s.iter().filter(|a| matches!(a.1, Act::S(Step::Chord(_)))).map(|a| a.0).collect();
        for (t, a) in s.iter_mut() {
            if !matches!(a, Act::S(Step::Chord(_))) && chords.contains(t) {
                *t += 1_000_000;
            }
        }
        s.sort_by_key(|x| x.0);
        (s, 22 * bar)
    }

    /// Every style, 4 seeds each, with a change to another style mid-song. On the parts that
    /// follow chords, at every note-on the receiver's pitch offset (bend x range) is the
    /// pitch shift the engine applied, give or take no more than the part's own pattern
    /// bends: no note starts detuned by a range left behind (a section change resending
    /// the SInt, a Fill's chased RPN, a style change) or by a pattern bend sent unscaled.
    /// The bend range is at least 12 at every note-on; no bend is clamped; no key starts
    /// twice or lasts zero time; nothing is stuck; no part is left bent after Stop; and the
    /// Style never sends on a keyboard part's channel.
    #[test]
    fn corpus_chaos_leaves_no_note_detuned() {
        let files = tests::corpus();
        if files.is_empty() {
            eprintln!("no corpus; skipping");
            return;
        }
        let mut worst = 0.0f64;
        let mut runs = 0;
        let mut fails: Vec<String> = Vec::new();
        let mut notes = 0usize;
        let mut bends = 0usize;
        for (fi, f) in files.iter().enumerate() {
            let style = Style::load(f).unwrap();
            let other = Style::load(&files[(fi + 7) % files.len()]).unwrap();
            let name = f.file_name().unwrap().to_string_lossy().to_string();
            let p0 = Prepared::new(&style);
            let p1 = Prepared::new(&other);
            let bar = bar_ns(&p0);
            for seed in 1..5 {
                let (script, end) = chaos(bar, seed * 7919 + fi as u64);
                let mut e = Engine::new(Box::new(Prepared::new(&style)));
                let mut rec = Recorder::default();
                let mut pat_max = p0.pat_bend_max;
                // The other style's pattern bend headroom applies from this index of `out`.
                let mut loads: Vec<(usize, [u8; 16])> = Vec::new();
                let mut i = 0;
                let mut now = 0u64;
                loop {
                    let t = [script.get(i).map(|s| s.0), e.next_deadline(), Some(end)].into_iter().flatten().min().unwrap();
                    now = now.max(t);
                    rec.now = now;
                    while let Some((ts, a)) = script.get(i) {
                        if *ts > now {
                            break;
                        }
                        match a {
                            Act::S(Step::Chord(c)) => e.set_chord(*c, now, &mut rec),
                            Act::S(Step::Release) => e.chord_released(now, &mut rec),
                            Act::S(Step::Button(b)) => e.button(*b, now, &mut rec),
                            Act::S(Step::ManualBass(on)) => e.set_manual_bass(*on, &mut rec),
                            Act::S(Step::Transpose(tr)) => e.set_transpose(*tr, now, &mut rec),
                            Act::Load => {
                                loads.push((rec.out.len(), p1.pat_bend_max));
                                let _ = e.load(Box::new(Prepared::new(&other)), now, &mut rec);
                            }
                        }
                        i += 1;
                    }
                    e.process(now, &mut rec);
                    if now >= end {
                        break;
                    }
                }
                // Stop Accompaniment holds its chord while stopped, by design: switch it off.
                if e.snapshot(now).stop_acmp {
                    e.button(Button::StopAcmp, now + 1, &mut rec);
                }
                runs += 1;
                if e.bend_clamps.get() != 0 {
                    fails.push(format!("{name}/{seed}: {} clamps", e.bend_clamps.get()));
                }
                // Replay through the receiver model.
                let mut rpn = [0x3FFFu16; 16];
                let mut range = [2.0f64; 16];
                let mut bend = [8192u16; 16];
                let mut shift = [0i8; 16];
                let mut on = std::collections::HashMap::<(u8, u8), u64>::new();
                let mut r = rec.retunes.iter().peekable();
                let mut l = loads.iter().peekable();
                for (idx, (t, m)) in rec.out.iter().enumerate() {
                    while let Some(&&(at, _, ch, s)) = r.peek() && at <= idx {
                        r.next();
                        shift[ch as usize] = s;
                    }
                    while let Some(&&(at, pm)) = l.peek() && at <= idx {
                        l.next();
                        pat_max = pm;
                    }
                    let ch = m[0] & 0x0F;
                    let c = ch as usize;
                    if m[0] != 0xF0 && crate::parts::part_of_channel(ch).is_some() {
                        fails.push(format!("{name}/{seed}: {m:02X?} on keyboard part at {t}"));
                    }
                    match m[0] & 0xF0 {
                        0xB0 => match m[1] {
                            101 => rpn[c] = (rpn[c] & 0x7F) | (m[2] as u16) << 7,
                            100 => rpn[c] = (rpn[c] & !0x7F) | m[2] as u16,
                            98 | 99 => rpn[c] = 0x3FFF,
                            6 if rpn[c] == 0 => range[c] = m[2] as f64,
                            _ => {}
                        },
                        0xE0 => {
                            bend[c] = (m[2] as u16) << 7 | m[1] as u16;
                            bends += (follows(ch) && bend[c] != 8192) as usize;
                        }
                        0x90 if m[2] > 0 => {
                            if follows(ch) {
                                notes += 1;
                                if on.insert((ch, m[1]), *t).is_some() {
                                    fails.push(format!("{name}/{seed}: ch{} key {} doubled at {t}", c + 1, m[1]));
                                }
                                let off = (bend[c] as f64 - 8192.0) / 8192.0 * range[c];
                                let dev = (off - shift[c] as f64).abs();
                                worst = worst.max(dev - pat_max[c] as f64);
                                if dev > pat_max[c] as f64 + 0.02 {
                                    fails.push(format!("{name}/{seed}: ch{} note-on at {t}: device offset {off:.3} vs shift {} (pat max {})", c + 1, shift[c], pat_max[c]));
                                }
                                if range[c] < 12.0 {
                                    fails.push(format!("{name}/{seed}: ch{} range {} at note-on {t}", c + 1, range[c]));
                                }
                            }
                        }
                        0x80 | 0x90 if follows(ch) => match on.remove(&(ch, m[1])) {
                            Some(s) if s < *t => {}
                            s => fails.push(format!("{name}/{seed}: ch{} key {} zero-length/unmatched {s:?} at {t}", c + 1, m[1])),
                        },
                        _ => {}
                    }
                }
                if !on.is_empty() {
                    fails.push(format!("{name}/{seed}: stuck {on:?}"));
                }
                // After Stop only the style's own setup may bend a part: a style loaded while
                // stopped sends its SInt, bends and all.
                let last_stop = rec.out.iter().rposition(|(_, m)| *m == [0xE8, 0, 0x40]).unwrap_or(0);
                for ch in 8..16u8 {
                    let last = rec.out.iter().rposition(|(_, m)| m[0] == 0xE0 | ch);
                    let from_setup = last.is_some_and(|i| i > last_stop && loads.iter().any(|l| l.0 <= i));
                    if bend[ch as usize] != 8192 && !from_setup {
                        fails.push(format!("{name}/{seed}: ch{} left bent {}", ch + 1, bend[ch as usize]));
                    }
                }
            }
        }
        eprintln!("{runs} runs, {notes} notes, {bends} off-centre bends, worst excess {worst:.4} semitones");
        for f in fails.iter().take(40) {
            eprintln!("{f}");
        }
        assert!(runs > 300 && bends > 1000, "{runs} runs, {bends} bends");
        assert!(fails.is_empty(), "{} failures", fails.len());
    }
}
