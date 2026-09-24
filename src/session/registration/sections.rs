//! The registrables: each feature's part of a Registration Memory (a "section" of the
//! bank file), how it is captured from the panel and how it is recalled.
//!
//! A feature adds its own `Registrable` (in its own module, or here) and one line in
//! `REGISTRABLES`. Its section is its own serde struct under its own key; a recall skips
//! whatever is not in the groups being recalled (Memorize groups less Freeze), and an old
//! bank file that lacks the section leaves the feature alone. Harmony/Arpeggio (#32/#33),
//! the Chord Looper and Live Control add theirs when they are wired in.

use super::super::Control;
use super::LockItem;
use crate::api::{gm_name, ChordCmd, LibraryCmd, MultiPadCmd, PartsCmd};
use crate::engine::{Button, StyleControls, Transpose};
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
    // The Multi Pad bank doesn't depend on the style: it loads with it, not after it.
    Registrable { key: "multiPad", early: true, capture: multipad_capture, recall: multipad_recall },
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

// ----- the Multi Pad bank (group Multi Pad) -----

/// Data List Regist items: "Multi Pad File". The pads' Synchro Start standby is not
/// stored (a standby waits for a chord, and the bank it was armed on may not be loaded
/// yet); nor are the Multi Pad part offsets, which yahaha doesn't have.
#[derive(Serialize, Deserialize)]
struct MultiPadReg {
    /// The bank file; None: no bank.
    bank: Option<String>,
}

fn multipad_capture(c: &Control, g: Groups) -> Option<Value> {
    if !g.has(Group::MultiPad) {
        return None;
    }
    to_value(&MultiPadReg { bank: c.multipad_bank_path().map(|p| p.display().to_string()) })
}

fn multipad_recall(c: &mut Control, v: &Value, g: Groups) -> Result<(), String> {
    if !g.has(Group::MultiPad) {
        return Ok(());
    }
    let r: MultiPadReg = parse("multiPad", v)?;
    let now = c.multipad_bank_path().map(Path::to_path_buf);
    let cmd = match r.bank {
        // Already chosen: pads playing from it carry on.
        Some(p) if now.as_deref() == Some(Path::new(&p)) => return Ok(()),
        Some(p) if !Path::new(&p).is_file() => return Err(format!("Multi Pad bank not found: {p}")),
        Some(path) => MultiPadCmd::LoadMultiPadPath { path },
        None if now.is_none() => return Ok(()),
        None => MultiPadCmd::ClearMultiPad,
    };
    c.multipad_cmd(cmd).map_err(|e| e.to_string())
}

// ----- tempo (group Tempo) -----

#[derive(Serialize, Deserialize)]
struct TempoReg {
    bpm: f64,
}

fn tempo_capture(c: &Control, g: Groups) -> Option<Value> {
    g.has(Group::Tempo).then(|| to_value(&TempoReg { bpm: c.snap.bpm.round() })).flatten()
}

fn tempo_recall(c: &mut Control, v: &Value, g: Groups) -> Result<(), String> {
    if !g.has(Group::Tempo) {
        return Ok(());
    }
    let t: TempoReg = parse("tempo", v)?;
    c.engine_cmd(tempo_cmd(t.bpm)).map_err(|e| e.to_string())
}

/// The engine command for a recalled tempo: the panel's SET TEMPO, whole BPM as on the
/// Genos (the engine clamps it to its range).
pub(in crate::session) fn tempo_cmd(bpm: f64) -> Cmd {
    let bpm = if bpm.is_finite() { bpm.round().clamp(0.0, u16::MAX as f64) as u16 } else { 0 };
    Cmd::Button(Button::SetTempo(bpm))
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
    // OTS Link is part of the registration; it does not fire for the registration's own
    // section (its voices are the registration's).
    c.hold_ots_for(Some(r.main.min(3)));
    c.shared.parts.ots_link.store(r.ots_link, Relaxed);
    // States, not presses: the engine compares them with its own (a snapshot here may be
    // behind an earlier recall's changes). Playing, the section changes at the next bar
    // line; Intro and Sync Start only change while stopped.
    let set = StyleControls {
        main: Some(r.main),
        intro: Some(r.intro),
        sync_start: Some(r.sync_start),
        sync_stop: Some(r.sync_stop),
        stop_acmp: Some(r.stop_acmp),
        parts: None,
        volumes: None,
        player_set: None,
    };
    c.engine_cmd(Cmd::StyleControls(set)).map_err(|e| e.to_string())
}

// ----- the Style part mixer (group Style) -----

#[derive(Serialize, Deserialize)]
struct MixerReg {
    /// Rhythm 1 .. Phrase 2: CC7 (as the mixer showed them).
    volumes: [u8; 8],
    /// Rhythm 1 .. Phrase 2: not muted.
    on: [bool; 8],
    /// Rhythm 1 .. Phrase 2: the player had set the level (the Genos's Volume(Style)
    /// offset). Only these levels are recalled; the others are the style's, which its
    /// patterns' CC7 move (Intro, Main, Ending levels). Missing (a bank from an earlier
    /// build): a level is set where it differs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    set: Option<[bool; 8]>,
}

fn mixer_capture(c: &Control, g: Groups) -> Option<Value> {
    if !g.has(Group::Style) {
        return None;
    }
    let s = &c.snap;
    to_value(&MixerReg {
        volumes: s.volumes,
        on: std::array::from_fn(|p| s.parts & (1 << p) != 0),
        set: Some(std::array::from_fn(|p| s.user_set & (1 << p) != 0)),
    })
}

fn mixer_recall(c: &mut Control, v: &Value, g: Groups) -> Result<(), String> {
    if !g.has(Group::Style) {
        return Ok(());
    }
    let r: MixerReg = parse("styleMixer", v)?;
    // Absolute levels and states only: the engine compares them with its own (the
    // snapshot may be behind an earlier recall's changes). It sets the player's levels and
    // hands every other part back to the style, so the patterns' CC7 move it as usual.
    let mask = |b: [bool; 8]| (0..8).filter(|&p| b[p]).fold(0u8, |m, p| m | 1 << p);
    let set = StyleControls {
        parts: Some(mask(r.on)),
        volumes: Some(r.volumes),
        player_set: r.set.map(mask),
        ..StyleControls::default()
    };
    c.engine_cmd(Cmd::StyleControls(set)).map_err(|e| e.to_string())
}

// ----- the keyboard parts (Right 1-3: group Voice; Left: group Style) -----

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PartReg {
    on: bool,
    /// None: a voice this build can't read (a newer `kind`, e.g. a plugin instrument). The
    /// part's other settings still recall, and so do the other parts; the file keeps it.
    #[serde(deserialize_with = "lenient_voice")]
    voice: Option<VoiceRef>,
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

fn lenient_voice<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<VoiceRef>, D::Error> {
    Ok(serde_json::from_value(Value::deserialize(d)?).ok())
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
            voice: Some(VoiceRef::gm(kp.program[p].load(Relaxed))),
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
        match part.voice.as_ref().and_then(VoiceRef::program) {
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
                    Some(p) => (p.voice.as_ref().and_then(VoiceRef::program).map_or("?", gm_name).to_string(), p.on),
                    None => (String::new(), false),
                })
                .collect()
        })
        .unwrap_or_default();
    Info { style, tempo, voices }
}
