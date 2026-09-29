//! The program map: which library patch a program change plays.
//!
//! Rules, most specific first, within one map:
//!
//! 1. **Drums**: a drum part (MIDI channel 9 or 10 on the band, Rhythm 1 and 2) or a
//!    voice on a Yamaha drum/SFX kit bank (bank MSB 126/127) plays the drum rule's patch.
//!    Kit pieces are not mapped one by one.
//! 2. **Program override**: one GM program (0-127) to a patch.
//! 3. **Family rule**: a GM family (programs in groups of 8: Piano 0-7, ..., Sound FX
//!    120-127) to a patch.
//!
//! Bank variations collapse: the XG/GS bank select is ignored, so a rule for program 33
//! covers every Finger Bass variation. A Yamaha voice outside the GM banks is first
//! brought to its GM program the way the synth already does (`synth::gm_fallback`).
//!
//! A style may have its own map on top of the global one: its rules (drums, override,
//! family) win; what it leaves unset falls through to the global map. Anything neither
//! maps plays what it played before the library existed (the SoundFont's GM voice).

use serde::{Deserialize, Serialize};

/// The GM families, 8 programs each.
pub const FAMILY_NAMES: [&str; 16] = [
    "Piano",
    "Chromatic Perc.",
    "Organ",
    "Guitar",
    "Bass",
    "Strings",
    "Ensemble",
    "Brass",
    "Reed",
    "Pipe",
    "Synth Lead",
    "Synth Pad",
    "Synth FX",
    "Ethnic",
    "Percussive",
    "Sound FX",
];

pub fn family_of(program: u8) -> usize {
    (program as usize & 127) / 8
}

/// One program override.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramOverride {
    /// GM program, 0-127.
    pub program: u8,
    pub patch: String,
    /// The rule's level: see [`ProgramMap::family_volumes`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<u8>,
}

/// A program map: family rules, program overrides and the drum rule, each naming a
/// library patch by id, and each with an optional level.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramMap {
    /// Per GM family (16), the patch its programs play, if any.
    #[serde(default)]
    pub families: [Option<String>; 16],
    /// Per GM family, its rule's level: the CC7 a Style part that resolves by the rule
    /// takes when the style sets none of its own (the mixer shows it). Sounds carry no mix
    /// (docs/racks.md); a version 2 library's sound volumes moved here (store.rs).
    #[serde(default, skip_serializing_if = "no_volumes")]
    pub family_volumes: [Option<u8>; 16],
    /// Program overrides, sorted by program, at most one per program.
    #[serde(default)]
    pub overrides: Vec<ProgramOverride>,
    /// The drum kit patch for the drum parts.
    #[serde(default)]
    pub drums: Option<String>,
    /// The drum rule's level: see `family_volumes`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drums_volume: Option<u8>,
}

fn no_volumes(v: &[Option<u8>; 16]) -> bool {
    v.iter().all(Option::is_none)
}

impl ProgramMap {
    pub fn is_empty(&self) -> bool {
        self.families.iter().all(Option::is_none) && self.overrides.is_empty() && self.drums.is_none()
    }

    pub fn override_of(&self, program: u8) -> Option<&str> {
        self.overrides.iter().find(|o| o.program == program).map(|o| o.patch.as_str())
    }

    /// A family's rule. A rule given another patch loses its level (it was the old
    /// patch's); the same patch keeps it.
    pub fn set_family(&mut self, family: usize, patch: Option<String>) {
        if let Some(f) = self.families.get_mut(family) {
            if *f != patch {
                self.family_volumes[family] = None;
            }
            *f = patch;
        }
    }

    /// A program's override, its level kept as `set_family` keeps one.
    pub fn set_override(&mut self, program: u8, patch: Option<String>) {
        let program = program & 127;
        let volume = self.overrides.iter().find(|o| o.program == program && Some(&o.patch) == patch.as_ref()).and_then(|o| o.volume);
        self.set_override_rule(program, patch.map(|patch| ProgramOverride { program, patch, volume }));
    }

    /// A program's override as given, level and all (None: no override).
    pub fn set_override_rule(&mut self, program: u8, rule: Option<ProgramOverride>) {
        let program = program & 127;
        self.overrides.retain(|o| o.program != program);
        if let Some(mut o) = rule {
            o.program = program;
            self.overrides.push(o);
            self.overrides.sort_by_key(|o| o.program);
        }
    }

    /// The drum rule, its level kept as `set_family` keeps one.
    pub fn set_drums(&mut self, patch: Option<String>) {
        if self.drums != patch {
            self.drums_volume = None;
        }
        self.drums = patch;
    }

    /// Every rule naming patch `id` takes level `volume` (the version 2 migration moves a
    /// sound's volume onto its rules).
    pub fn set_volume_of(&mut self, id: &str, volume: u8) {
        for (f, p) in self.families.iter().enumerate() {
            if p.as_deref() == Some(id) {
                self.family_volumes[f] = Some(volume);
            }
        }
        for o in self.overrides.iter_mut().filter(|o| o.patch == id) {
            o.volume = Some(volume);
        }
        if self.drums.as_deref() == Some(id) {
            self.drums_volume = Some(volume);
        }
    }

    /// Forget every rule that names `id` (the patch was deleted).
    pub fn forget(&mut self, id: &str) {
        for (f, v) in self.families.iter_mut().zip(&mut self.family_volumes) {
            if f.as_deref() == Some(id) {
                *f = None;
                *v = None;
            }
        }
        self.overrides.retain(|o| o.patch != id);
        if self.drums.as_deref() == Some(id) {
            self.drums = None;
            self.drums_volume = None;
        }
    }

    /// Rules naming `from` name `to` instead.
    pub fn rename(&mut self, from: &str, to: &str) {
        for f in self.families.iter_mut().flatten() {
            if f == from {
                *f = to.to_string();
            }
        }
        for o in &mut self.overrides {
            if o.patch == from {
                o.patch = to.to_string();
            }
        }
        if let Some(d) = self.drums.as_mut().filter(|d| *d == from) {
            *d = to.to_string();
        }
    }

    /// Fix what a hand-edited or imported file may have: overrides sorted, one per
    /// program, programs in range, levels in range and only on rules.
    pub fn normalize(&mut self) {
        for o in &mut self.overrides {
            o.program &= 127;
            o.volume = o.volume.map(|v| v.min(127));
        }
        self.overrides.sort_by_key(|o| o.program);
        self.overrides.dedup_by_key(|o| o.program);
        for (f, v) in self.families.iter().zip(&mut self.family_volumes) {
            *v = v.filter(|_| f.is_some()).map(|v| v.min(127));
        }
        self.drums_volume = self.drums_volume.filter(|_| self.drums.is_some()).map(|v| v.min(127));
    }

    /// This map's rule for a program, if it has one: its patch, kind and level.
    fn rule(&self, drum: bool, program: u8) -> Option<(&str, RuleKind, Option<u8>)> {
        if drum {
            return self.drums.as_deref().map(|p| (p, RuleKind::Drums, self.drums_volume));
        }
        if let Some(o) = self.overrides.iter().find(|o| o.program == program) {
            return Some((o.patch.as_str(), RuleKind::Override, o.volume));
        }
        let f = family_of(program);
        self.families[f].as_deref().map(|p| (p, RuleKind::Family, self.family_volumes[f]))
    }
}

/// Which rule a program resolved by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuleKind {
    Drums,
    Override,
    Family,
    /// No rule: the SoundFont's own voice, as before.
    Fallback,
}

/// What a program resolved to: the patch (None: the fallback), by which rule, whether
/// the rule is the style's own, and the rule's level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resolution<'a> {
    pub patch: Option<&'a str>,
    pub rule: RuleKind,
    pub from_style: bool,
    pub volume: Option<u8>,
}

/// A part is a drum part: the band's Rhythm 1 and 2 (MIDI channels 9 and 10, `ch` 8 and 9
/// 0-based) or any part on a Yamaha drum/SFX kit bank (MSB 126/127).
pub fn is_drum(ch: u8, msb: u8) -> bool {
    ch == 8 || ch == 9 || msb >= 126
}

/// The GM program the map looks up for a Yamaha voice on band channel `ch` (0-based):
/// the program itself on the GM/XG banks, else what the synth plays for it.
pub fn map_program(ch: u8, msb: u8, program: u8) -> u8 {
    crate::synth::gm_fallback(ch, msb, program & 127)
}

/// Resolve a program: the style's map first, then the global one, else the fallback.
pub fn resolve<'a>(global: &'a ProgramMap, style: Option<&'a ProgramMap>, drum: bool, program: u8) -> Resolution<'a> {
    if let Some((p, rule, volume)) = style.and_then(|m| m.rule(drum, program)) {
        return Resolution { patch: Some(p), rule, from_style: true, volume };
    }
    match global.rule(drum, program) {
        Some((p, rule, volume)) => Resolution { patch: Some(p), rule, from_style: false, volume },
        None => Resolution { patch: None, rule: RuleKind::Fallback, from_style: false, volume: None },
    }
}
