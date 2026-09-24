//! The registrables: each feature's part of a Registration Memory (a "section" of the
//! bank file), how it is captured from the panel and how it is recalled.
//!
//! A feature adds its own `Registrable` (in its own module, or here) and one line in
//! `REGISTRABLES`. Its section is its own serde struct under its own key; a recall skips
//! whatever is not in the groups being recalled (Memorize groups less Freeze), and an old
//! bank file that lacks the section leaves the feature alone. Harmony/Arpeggio (#32/#33),
//! Multi Pads (#37), the Chord Looper and Live Control add theirs when they are wired in.

use super::super::Control;
use super::LockItem;
use crate::api::{gm_name, ChordCmd, LibraryCmd, PartsCmd};
use crate::engine::{Button, Transpose};
use crate::fingering::Fingering;
use crate::live::Cmd;
use crate::parts;
use crate::registration::{Group, Groups, Memory, VoiceRef};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering::Relaxed;

/// One feature's part of a registration.
pub(in crate::session) struct Registrable {
    /// Its key in `Memory::sections` (and in the bank file).
    pub key: &'static str,
    /// Recalled before the others, without waiting for the style (the style itself).
    pub early: bool,
    /// Its section for a Memorize of `groups`; None when none of its items are in them.
    pub capture: fn(&Control, Groups) -> Option<Value>,
    /// Recall its section, changing only items in `groups`. An error is reported and the
    /// other sections are still recalled.
    pub recall: fn(&mut Control, &Value, Groups) -> Result<(), String>,
}

/// Every registrable, in recall order: the style first; the chord settings before the
/// parts (Upper/Manual Bass decide what the Left part may do); the section and the Style
/// mixer once the style plays.
pub(in crate::session) const REGISTRABLES: &[Registrable] = &[
    Registrable { key: "style", early: true, capture: style_capture, recall: style_recall },
    Registrable { key: "tempo", early: false, capture: tempo_capture, recall: tempo_recall },
    Registrable { key: "chord", early: false, capture: chord_capture, recall: chord_recall },
    Registrable { key: "styleControl", early: false, capture: control_capture, recall: control_recall },
    Registrable { key: "styleMixer", early: false, capture: mixer_capture, recall: mixer_recall },
    Registrable { key: "parts", early: false, capture: parts_capture, recall: parts_recall },
    Registrable { key: "transpose", early: false, capture: transpose_capture, recall: transpose_recall },
];

fn to_value<T: Serialize>(t: &T) -> Option<Value> {
    serde_json::to_value(t).ok()
}

fn parse<T: DeserializeOwned>(key: &str, v: &Value) -> Result<T, String> {
    serde_json::from_value(v.clone()).map_err(|e| format!("registration {key}: {e}"))
}

// ----- style (group Style) -----

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StyleReg {
    path: String,
    /// For Regist Bank Info, and for finding the file if it moved.
    name: String,
}

fn style_capture(c: &Control, g: Groups) -> Option<Value> {
    if !g.has(Group::Style) {
        return None;
    }
    // The style the player chose: the one waiting for the bar line, else the one playing.
    let info = c.pending_style.as_ref().map_or(&c.info, |p| &p.2);
    to_value(&StyleReg { path: info.path.display().to_string(), name: info.name.clone() })
}

fn style_recall(c: &mut Control, v: &Value, g: Groups) -> Result<(), String> {
    if !g.has(Group::Style) {
        return Ok(());
    }
    let s: StyleReg = parse("style", v)?;
    let path = find_style(c, Path::new(&s.path)).ok_or_else(|| format!("style not found: {}", s.path))?;
    c.library_cmd(LibraryCmd::LoadStylePath { path: path.display().to_string() }).map_err(|e| e.to_string())
}

/// The style file: where it was saved, else a library file of the same name (the folder
/// moved, or another machine).
fn find_style(c: &Control, path: &Path) -> Option<PathBuf> {
    if path.is_file() || c.lib.find(path).is_some() {
        return Some(path.to_path_buf());
    }
    let name = path.file_name()?;
    (0..c.lib.len()).map(|id| &c.lib.entry(id).path).find(|p| p.file_name() == Some(name)).cloned()
}

// ----- tempo (group Tempo) -----

#[derive(Serialize, Deserialize)]
struct TempoReg {
    bpm: f64,
}

fn tempo_capture(c: &Control, g: Groups) -> Option<Value> {
    g.has(Group::Tempo).then(|| to_value(&TempoReg { bpm: c.snap.bpm })).flatten()
}

fn tempo_recall(c: &mut Control, v: &Value, g: Groups) -> Result<(), String> {
    if !g.has(Group::Tempo) {
        return Ok(());
    }
    let t: TempoReg = parse("tempo", v)?;
    c.engine_cmd(Cmd::SetTempo(t.bpm)).map_err(|e| e.to_string())
}

// ----- chord detection and split (group Style; Parameter Lock items) -----

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChordReg {
    fingering: Fingering,
    /// Chord Detection Area = Upper.
    upper: bool,
    manual_bass: bool,
    /// Split point (Style), a MIDI note.
    split: u8,
}

fn chord_capture(c: &Control, g: Groups) -> Option<Value> {
    if !g.has(Group::Style) {
        return None;
    }
    let sh = &c.shared;
    to_value(&ChordReg {
        fingering: Fingering::from_u8(sh.fingering.load(Relaxed)),
        upper: sh.upper.load(Relaxed),
        manual_bass: sh.manual_bass.load(Relaxed),
        split: sh.split.load(Relaxed),
    })
}

fn chord_recall(c: &mut Control, v: &Value, g: Groups) -> Result<(), String> {
    if !g.has(Group::Style) {
        return Ok(());
    }
    let r: ChordReg = parse("chord", v)?;
    let e = |e: crate::api::CmdError| e.to_string();
    if !c.param_locked(LockItem::FingeringType) {
        c.chord_cmd(ChordCmd::SetFingering { fingering: r.fingering }).map_err(e)?;
        c.chord_cmd(ChordCmd::SetUpper { on: r.upper }).map_err(e)?;
        // Manual Bass only changes in Upper (and Upper turns it on).
        c.chord_cmd(ChordCmd::SetManualBass { on: r.manual_bass }).map_err(e)?;
    }
    if !c.param_locked(LockItem::SplitPoint) {
        c.chord_cmd(ChordCmd::SetSplit { note: r.split }).map_err(e)?;
    }
    Ok(())
}

// ----- the section and the Style buttons (group Style) -----

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ControlReg {
    /// Main A-D (0-3).
    main: u8,
    /// The Intro armed to start with (stopped).
    intro: Option<u8>,
    sync_start: bool,
    sync_stop: bool,
    stop_acmp: bool,
    ots_link: bool,
}

fn control_capture(c: &Control, g: Groups) -> Option<Value> {
    if !g.has(Group::Style) {
        return None;
    }
    let s = &c.snap;
    to_value(&ControlReg {
        main: s.main,
        intro: s.pending_intro,
        sync_start: s.sync_armed,
        sync_stop: s.sync_stop,
        stop_acmp: s.stop_acmp,
        ots_link: c.shared.parts.ots_link.load(Relaxed),
    })
}

fn control_recall(c: &mut Control, v: &Value, g: Groups) -> Result<(), String> {
    if !g.has(Group::Style) {
        return Ok(());
    }
    let r: ControlReg = parse("styleControl", v)?;
    let s = c.snap;
    let press = |c: &mut Control, b: Button| c.engine_cmd(Cmd::Button(b)).map_err(|e| e.to_string());
    // OTS Link is part of the registration; it does not fire for the registration's own
    // section (its voices are the registration's).
    c.hold_ots_for(Some(r.main.min(3)));
    c.shared.parts.ots_link.store(r.ots_link, Relaxed);
    if r.main.min(3) != s.main {
        // Playing: the section changes at the next bar line, as a Main press does.
        press(c, Button::Main(r.main.min(3)))?;
    }
    if !s.running {
        match (r.intro, s.pending_intro) {
            (Some(i), cur) if cur != Some(i) => press(c, Button::Intro(i.min(2)))?,
            (None, Some(cur)) => press(c, Button::Intro(cur))?,
            _ => {}
        }
        // Sync Start while playing would stop the band: it's only recalled when stopped.
        if r.sync_start != s.sync_armed {
            press(c, Button::SyncStart)?;
        }
    }
    if r.sync_stop != s.sync_stop {
        press(c, Button::SyncStop)?;
    }
    if r.stop_acmp != s.stop_acmp {
        press(c, Button::StopAcmp)?;
    }
    Ok(())
}

// ----- the Style part mixer (group Style) -----

#[derive(Serialize, Deserialize)]
struct MixerReg {
    /// Rhythm 1 .. Phrase 2: CC7.
    volumes: [u8; 8],
    /// Rhythm 1 .. Phrase 2: not muted.
    on: [bool; 8],
}

fn mixer_capture(c: &Control, g: Groups) -> Option<Value> {
    if !g.has(Group::Style) {
        return None;
    }
    let s = &c.snap;
    to_value(&MixerReg { volumes: s.volumes, on: std::array::from_fn(|p| s.parts & (1 << p) != 0) })
}

fn mixer_recall(c: &mut Control, v: &Value, g: Groups) -> Result<(), String> {
    if !g.has(Group::Style) {
        return Ok(());
    }
    let r: MixerReg = parse("styleMixer", v)?;
    let s = c.snap;
    for p in 0..8u8 {
        let (vol, on) = (r.volumes[p as usize].min(127), r.on[p as usize]);
        if vol != s.volumes[p as usize] {
            c.engine_cmd(Cmd::StyleVolume(p, vol)).map_err(|e| e.to_string())?;
        }
        if on != (s.parts & (1 << p) != 0) {
            c.engine_cmd(Cmd::Button(Button::TogglePart(p))).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

// ----- the keyboard parts (Right 1-3: group Voice; Left: group Style) -----

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PartReg {
    on: bool,
    voice: VoiceRef,
    /// CC7.
    volume: u8,
    /// -2..=2.
    octave: i8,
}

/// Right 1, Right 2, Right 3, Left; None for a part outside the memorized groups.
#[derive(Serialize, Deserialize)]
struct PartsReg {
    parts: [Option<PartReg>; 4],
}

fn part_group(p: usize) -> Group {
    if p == parts::LEFT { Group::Style } else { Group::Voice }
}

fn parts_capture(c: &Control, g: Groups) -> Option<Value> {
    if !g.has(Group::Voice) && !g.has(Group::Style) {
        return None;
    }
    let kp = &c.shared.parts;
    let parts = std::array::from_fn(|p| {
        g.has(part_group(p)).then(|| PartReg {
            on: kp.is_on(p),
            voice: VoiceRef::gm(kp.program[p].load(Relaxed)),
            volume: kp.volume(p),
            octave: kp.octave[p].load(Relaxed).clamp(-2, 2),
        })
    });
    to_value(&PartsReg { parts })
}

fn parts_recall(c: &mut Control, v: &Value, g: Groups) -> Result<(), String> {
    let r: PartsReg = parse("parts", v)?;
    let mut err = None;
    for (p, part) in r.parts.iter().enumerate() {
        let Some(part) = part.as_ref().filter(|_| g.has(part_group(p))) else { continue };
        let kp = c.shared.parts.clone();
        match part.voice.program() {
            Some(prog) => kp.set_program(p, prog),
            None => err = Some(format!("{}: voice not available", parts::NAMES[p])),
        }
        kp.set_volume(p, part.volume.min(127));
        kp.octave[p].store(part.octave.clamp(-2, 2), Relaxed);
        // Left plays the bass under Manual Bass: its switch stays as it is.
        let locked_left = p == parts::LEFT && c.shared.manual_bass();
        if kp.is_on(p) != part.on && !locked_left {
            c.parts_cmd(PartsCmd::SetPartOn { part: p as u8, on: part.on }).map_err(|e| e.to_string())?;
        }
    }
    // The engine sends the new volumes (CC7) on its next wake.
    c.wake_engine();
    err.map_or(Ok(()), Err)
}

// ----- transpose (group Transpose) -----

#[derive(Serialize, Deserialize)]
struct TransposeReg {
    keyboard: i8,
    master: i8,
}

fn transpose_capture(c: &Control, g: Groups) -> Option<Value> {
    g.has(Group::Transpose).then(|| to_value(&TransposeReg { keyboard: c.transpose.keyboard, master: c.transpose.master })).flatten()
}

fn transpose_recall(c: &mut Control, v: &Value, g: Groups) -> Result<(), String> {
    if !g.has(Group::Transpose) {
        return Ok(());
    }
    let t: TransposeReg = parse("transpose", v)?;
    c.set_transpose(Transpose::new(t.keyboard, t.master)).map_err(|e| e.to_string())
}

// ----- Regist Bank Info -----

/// What Regist Bank Info shows for a memory: its style, tempo and voices.
pub(in crate::session) struct Info {
    pub style: Option<String>,
    pub tempo: Option<f64>,
    /// Right 1, Right 2, Right 3, Left (name, on); empty when it stores no parts.
    pub voices: Vec<(String, bool)>,
}

pub(in crate::session) fn info(m: &Memory) -> Info {
    let get = |k: &str| m.sections.get(k).cloned();
    let style = get("style").and_then(|v| serde_json::from_value::<StyleReg>(v).ok()).map(|s| s.name);
    let tempo = get("tempo").and_then(|v| serde_json::from_value::<TempoReg>(v).ok()).map(|t| t.bpm);
    let voices = get("parts")
        .and_then(|v| serde_json::from_value::<PartsReg>(v).ok())
        .map(|r| {
            r.parts
                .iter()
                .map(|p| match p {
                    Some(p) => (p.voice.program().map_or("?", gm_name).to_string(), p.on),
                    None => (String::new(), false),
                })
                .collect()
        })
        .unwrap_or_default();
    Info { style, tempo, voices }
}
