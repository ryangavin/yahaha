//! The registrables: each feature's part of a Registration Memory (a "section" of the
//! bank file), how it is captured from the panel and how it is recalled.
//!
//! A feature adds its own `Registrable` (in its own module, or here) and one line in
//! `REGISTRABLES`. Its section is its own serde struct under its own key; a recall skips
//! whatever is not in the groups being recalled (Memorize groups less Freeze), and an old
//! bank file that lacks the section leaves the feature alone. Live Control adds its own
//! when it is wired in.
//!
//! Parameter Lock: a recall that sets an item of a Data List lock group (`LockItem`) asks
//! `c.param_locked(item)` first and leaves the item alone when it is locked.

use super::super::fx::{effects_capture, effects_recall, PadSendsReg};
use super::super::harmony_arp::{harmony_arp_capture, harmony_arp_recall};
use super::super::looper::{looper_capture, looper_recall};
use super::super::style_settings::{style_settings_capture, style_settings_recall};
use super::super::Control;
use crate::api::{gm_name, ChordCmd, LibraryCmd, LockItem, MultiPadCmd, PartsCmd, StopAcmpMode};
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
    // The effect bus's types and return levels (#204): session/fx.rs.
    Registrable { key: "effects", early: false, capture: effects_capture, recall: effects_recall },
    Registrable { key: "parts", early: false, capture: parts_capture, recall: parts_recall },
    Registrable { key: "transpose", early: false, capture: transpose_capture, recall: transpose_recall },
    // Keyboard Harmony/Arpeggio (#32/#33): session/harmony_arp.rs.
    Registrable { key: "harmonyArp", early: false, capture: harmony_arp_capture, recall: harmony_arp_recall },
    // Section Change Timing, Retrigger, Synchro Stop Window, Section Reset, fade times
    // (#107): session/style_settings.rs.
    Registrable { key: "styleSettings", early: false, capture: style_settings_capture, recall: style_settings_recall },
    // The Chord Looper (#201): session/looper.rs. After the style, so a loop armed by the
    // recall follows the recalled style.
    Registrable { key: "chordLooper", early: false, capture: looper_capture, recall: looper_recall },
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
    /// The Multi Pad volume (#196; the Genos's Multi Pad volume offset), 100 = as written.
    /// Missing (a bank from an earlier build): left as it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    level: Option<u8>,
    /// The Multi Pad send scales per effect block (#267). Missing (a bank from before
    /// them): left as they are.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sends: Option<PadSendsReg>,
}

fn multipad_capture(c: &Control, g: Groups) -> Option<Value> {
    if !g.has(Group::MultiPad) {
        return None;
    }
    to_value(&MultiPadReg {
        bank: c.multipad_bank_path().map(|p| p.display().to_string()),
        level: Some(c.shared.parts.volume(parts::PAD_LEVEL)),
        sends: Some(c.pad_sends_capture()),
    })
}

fn multipad_recall(c: &mut Control, v: &Value, g: Groups) -> Result<(), String> {
    if !g.has(Group::MultiPad) {
        return Ok(());
    }
    let r: MultiPadReg = parse("multiPad", v)?;
    if let Some(level) = r.level {
        // Panel fader 6 picks it up; the engine thread scales the pads on its next wake.
        c.shared.parts.set_volume(parts::PAD_LEVEL, level);
        c.wake_engine();
    }
    if let Some(sends) = r.sends {
        c.pad_sends_recall(sends);
    }
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
    /// Left Hold (DL: a Registration item, group Style). Absent in banks from before it:
    /// left as it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    left_hold: Option<bool>,
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
        left_hold: Some(sh.controllers.left_hold()),
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
    if let Some(on) = r.left_hold {
        c.chord_cmd(ChordCmd::SetLeftHold { on }).map_err(e)?;
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
    /// Style Setting > Stop ACMP (Data List: registrable, group Style). Missing (a bank
    /// from an earlier build): `stop_acmp` turns it on or off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stop_acmp_mode: Option<StopAcmpMode>,
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
        stop_acmp_mode: Some(s.stop_acmp_mode.into()),
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
        stop_acmp_mode: r.stop_acmp_mode.map(Into::into),
        parts: None,
        volumes: None,
        player_set: None,
        retrigger: None,
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
    /// The Style volume (#199; the Genos's Style volume offset, DL p.83), 100 = as
    /// written. Missing (a bank from an earlier build): left as it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    level: Option<u8>,
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
        level: Some(c.shared.parts.volume(parts::STYLE_LEVEL)),
    })
}

fn mixer_recall(c: &mut Control, v: &Value, g: Groups) -> Result<(), String> {
    if !g.has(Group::Style) {
        return Ok(());
    }
    let r: MixerReg = parse("styleMixer", v)?;
    if let Some(level) = r.level {
        // Panel fader 5 picks it up; the engine thread scales the parts on its next wake.
        c.shared.parts.set_volume(parts::STYLE_LEVEL, level);
        c.wake_engine();
    }
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
    /// Pan, reverb send and chorus send (CC10, CC91, CC93; #198). Missing (a bank from an
    /// earlier build): the part's are left as they are.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pan: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reverb: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    chorus: Option<u8>,
    /// The variation (delay) send (CC94; #204). Missing: left as it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    variation: Option<u8>,
    /// The part's own sound library patch (#103), played instead of `voice`. None: the
    /// GM voice (and a bank from an earlier build). A patch that is gone from the library
    /// falls back to `voice`, which is the GM voice the part had underneath.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    patch: Option<PatchReg>,
    /// The voice settings an OTS or the panel set (#238): filter, EG, vibrato and
    /// portamento, and the XG part parameters. Missing: the voice's own (the voice recalled
    /// above has already put them back to neutral).
    #[serde(default, skip_serializing_if = "ToneReg::is_empty")]
    tone: ToneReg,
    /// Pitch Bend Range in semitones (RPN 0; Data List: Regist O, not a Voice Set
    /// parameter). Missing (a bank from an earlier build): left as it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bend_range: Option<u8>,
}

/// A part's `parts::TONE_CC` controllers by name, and its XG multi part parameters as
/// `[hh, nn, vv]` (`F0 43 1n 4C hh pp nn vv F7`). None / empty: not set.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToneReg {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cutoff: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    resonance: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    attack: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    decay: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    release: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    vibrato_rate: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    vibrato_depth: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    vibrato_delay: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    portamento: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    portamento_time: Option<u8>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    xg: Vec<[u8; 3]>,
}

impl ToneReg {
    fn is_empty(&self) -> bool {
        *self == ToneReg::default()
    }

    fn capture(kp: &parts::Parts, p: usize) -> ToneReg {
        let t = kp.tone(p);
        ToneReg {
            cutoff: t[parts::CUTOFF],
            resonance: t[parts::RESONANCE],
            attack: t[parts::ATTACK],
            decay: t[parts::DECAY],
            release: t[parts::RELEASE],
            vibrato_rate: t[parts::VIBRATO_RATE],
            vibrato_depth: t[parts::VIBRATO_DEPTH],
            vibrato_delay: t[parts::VIBRATO_DELAY],
            portamento: t[parts::PORTAMENTO],
            portamento_time: t[parts::PORTAMENTO_TIME],
            xg: kp.xg(p).into_iter().map(|(hh, nn, vv)| [hh, nn, vv]).collect(),
        }
    }

    /// By `parts::TONE_CC` index.
    fn controllers(&self) -> [Option<u8>; parts::TONE] {
        let mut t = [None; parts::TONE];
        t[parts::CUTOFF] = self.cutoff;
        t[parts::RESONANCE] = self.resonance;
        t[parts::ATTACK] = self.attack;
        t[parts::DECAY] = self.decay;
        t[parts::RELEASE] = self.release;
        t[parts::VIBRATO_RATE] = self.vibrato_rate;
        t[parts::VIBRATO_DEPTH] = self.vibrato_depth;
        t[parts::VIBRATO_DELAY] = self.vibrato_delay;
        t[parts::PORTAMENTO] = self.portamento;
        t[parts::PORTAMENTO_TIME] = self.portamento_time;
        t
    }
}

/// A library patch in a registration: its id, and its name for Regist Bank Info (and the
/// message when it is gone).
#[derive(Clone, Serialize, Deserialize)]
struct PatchReg {
    id: String,
    name: String,
}

/// Right 1, Right 2, Right 3, Left; None for a part outside the memorized groups.
#[derive(Serialize, Deserialize)]
struct PartsReg {
    parts: [Option<PartReg>; 4],
}

fn lenient_voice<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<VoiceRef>, D::Error> {
    Ok(serde_json::from_value(Value::deserialize(d)?).ok())
}

pub(super) fn part_group(p: usize) -> Group {
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
            voice: Some(c.part_plugin_reg(p).unwrap_or_else(|| VoiceRef::gm(kp.program[p].load(Relaxed)))),
            volume: kp.volume(p),
            octave: kp.octave[p].load(Relaxed).clamp(-2, 2),
            pan: Some(kp.fx(p)[parts::PAN]),
            reverb: Some(kp.fx(p)[parts::REVERB]),
            chorus: Some(kp.fx(p)[parts::CHORUS]),
            variation: Some(kp.fx(p)[parts::VARIATION]),
            patch: c.part_patch(p).map(|(id, name)| PatchReg { id, name }),
            tone: ToneReg::capture(kp, p),
            bend_range: Some(c.shared.controllers.bend_range(p)),
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
        match part.voice.as_ref() {
            Some(v) => {
                kp.set_program(p, v.program().unwrap_or(0));
                let r = match v {
                    VoiceRef::Plugin { id, name, state, .. } => c.recall_part_plugin(p, id, name, state.as_deref()),
                    VoiceRef::Gm { .. } => {
                        // A GM voice: no plugin from the Plugins tab (a library patch's own
                        // plugin is the patch's business, below).
                        c.clear_part_tab_plugin(p);
                        recall_patch(c, p, part.patch.as_ref())
                    }
                };
                if let Err(e) = r {
                    err = Some(e);
                }
            }
            None => err = Some(format!("{}: voice not available", parts::NAMES[p])),
        }
        // After the patch: its defaults give way to the registration's level, octave, pan
        // and sends (the engine thread sends the CCs).
        kp.set_volume(p, part.volume.min(127));
        kp.octave[p].store(part.octave.clamp(-2, 2), Relaxed);
        kp.set_fx(p, [part.pan, part.reverb, part.chorus, part.variation]);
        // The voice settings (#238), after the voice (which put them back to neutral).
        if !part.tone.is_empty() {
            kp.set_tone(p, part.tone.controllers(), part.tone.xg.iter().map(|x| (x[0], x[1], x[2])));
        }
        if let Some(r) = part.bend_range {
            c.shared.controllers.set_bend_range(p, r);
        }
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

/// Part `p`'s own patch, as the registration has it: the patch (if it isn't already the
/// part's), or none (the GM voice just set). A patch no longer in the sound library leaves
/// the part on its GM voice, and says so.
fn recall_patch(c: &mut Control, p: usize, patch: Option<&PatchReg>) -> Result<(), String> {
    let now = c.part_patch(p).map(|(id, _)| id);
    match patch {
        Some(r) if now.as_deref() == Some(r.id.as_str()) => Ok(()),
        Some(r) if c.has_patch(&r.id) => c.set_part_patch(p, Some(r.id.clone())).map_err(|e| e.to_string()),
        Some(r) => {
            c.sound_library_part_voice(p);
            Err(format!("{}: patch {} is not in the sound library; it plays its GM voice", parts::NAMES[p], r.name))
        }
        None => {
            c.sound_library_part_voice(p);
            Ok(())
        }
    }
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
                    Some(p) => match (&p.patch, &p.voice) {
                        (Some(patch), _) => (patch.name.clone(), p.on),
                        (None, Some(VoiceRef::Plugin { id, name, .. })) => (if name.is_empty() { id.clone() } else { name.clone() }, p.on),
                        (None, v) => (v.as_ref().and_then(VoiceRef::program).map_or("?", gm_name).to_string(), p.on),
                    },
                    None => (String::new(), false),
                })
                .collect()
        })
        .unwrap_or_default();
    Info { style, tempo, voices }
}
