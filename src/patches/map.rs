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
}

/// A program map: family rules, program overrides and the drum rule, each naming a
/// library patch by id.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramMap {
    /// Per GM family (16), the patch its programs play, if any.
    #[serde(default)]
    pub families: [Option<String>; 16],
    /// Program overrides, sorted by program, at most one per program.
    #[serde(default)]
    pub overrides: Vec<ProgramOverride>,
    /// The drum kit patch for the drum parts.
    #[serde(default)]
    pub drums: Option<String>,
}

impl ProgramMap {
    pub fn is_empty(&self) -> bool {
        self.families.iter().all(Option::is_none) && self.overrides.is_empty() && self.drums.is_none()
    }

    pub fn override_of(&self, program: u8) -> Option<&str> {
        self.overrides.iter().find(|o| o.program == program).map(|o| o.patch.as_str())
    }

    pub fn set_family(&mut self, family: usize, patch: Option<String>) {
        if let Some(f) = self.families.get_mut(family) {
            *f = patch;
        }
    }

    pub fn set_override(&mut self, program: u8, patch: Option<String>) {
        let program = program & 127;
        self.overrides.retain(|o| o.program != program);
        if let Some(patch) = patch {
            self.overrides.push(ProgramOverride { program, patch });
            self.overrides.sort_by_key(|o| o.program);
        }
    }

    /// Forget every rule that names `id` (the patch was deleted).
    pub fn forget(&mut self, id: &str) {
        for f in &mut self.families {
            if f.as_deref() == Some(id) {
                *f = None;
            }
        }
        self.overrides.retain(|o| o.patch != id);
        if self.drums.as_deref() == Some(id) {
            self.drums = None;
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
    /// program, programs in range.
    pub fn normalize(&mut self) {
        for o in &mut self.overrides {
            o.program &= 127;
        }
        self.overrides.sort_by_key(|o| o.program);
        self.overrides.dedup_by_key(|o| o.program);
    }

    /// This map's rule for a program, if it has one.
    fn rule(&self, drum: bool, program: u8) -> Option<(&str, RuleKind)> {
        if drum {
            return self.drums.as_deref().map(|p| (p, RuleKind::Drums));
        }
        if let Some(p) = self.override_of(program) {
            return Some((p, RuleKind::Override));
        }
        self.families[family_of(program)].as_deref().map(|p| (p, RuleKind::Family))
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

/// What a program resolved to: the patch (None: the fallback), by which rule, and whether
/// the rule is the style's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resolution<'a> {
    pub patch: Option<&'a str>,
    pub rule: RuleKind,
    pub from_style: bool,
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
    if let Some((p, rule)) = style.and_then(|m| m.rule(drum, program)) {
        return Resolution { patch: Some(p), rule, from_style: true };
    }
    match global.rule(drum, program) {
        Some((p, rule)) => Resolution { patch: Some(p), rule, from_style: false },
        None => Resolution { patch: None, rule: RuleKind::Fallback, from_style: false },
    }
}
