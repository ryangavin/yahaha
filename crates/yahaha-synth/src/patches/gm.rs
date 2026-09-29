//! The GM map's data shape (docs/sound-browser.md, D3, D4, D6): what every GM program and
//! the drum parts resolve to, which layer decided it, and the font preset behind it.
//!
//! Layers, most specific first; each layer asks the style's map, then the global one:
//!
//! 1. **Drums**: the drum rule (drum parts only).
//! 2. **Override**: a per-program rule.
//! 3. **Family**: a rule for the program's GM family (16 families of 8).
//! 4. **Auto**: [`AutoFill`], the best-matching preset in the scanned fonts, most
//!    GM-complete font first.
//!
//! Rules name library patches ([`ProgramMap`]); a patch may be a plugin sound or a font
//! preset added to the library. Auto names a font preset directly. Whatever resolves to a
//! font carries its [`FontPreset`] (file, bank, program), so a `.sf2` writer (D6, not
//! built) can render the map without changing this data.
//!
//! Nothing here runs on a real-time thread: the session resolves the map into
//! `patches::Routes` off-thread (session/sound_library.rs `write_bank`), with the auto-fill
//! built from the scanned fonts (session/gm_auto.rs). There is no default sound set.

use super::sf2::Preset;
use super::{family_of, ProgramMap, RuleKind, SoundId, SoundLibrary};
use super::FontPreset;
use serde::{Deserialize, Serialize};

/// The layer that decided a program (the map page's "decided by").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Layer {
    Drums,
    Override,
    Family,
    /// No rule: the auto-fill's font preset.
    Auto,
    /// No rule and no scanned font has anything for it: silence (or the synth's own).
    None,
}

impl From<RuleKind> for Layer {
    fn from(r: RuleKind) -> Layer {
        match r {
            RuleKind::Drums => Layer::Drums,
            RuleKind::Override => Layer::Override,
            RuleKind::Family => Layer::Family,
            RuleKind::Fallback => Layer::None,
        }
    }
}

/// The auto-fill layer (D4): for each GM program, and for the drum parts, the best
/// matching preset in the scanned fonts. Built off-thread from the fonts' preset lists.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoFill {
    /// By GM program: 128 entries (fewer only in a hand-made value; missing = none).
    pub programs: Vec<Option<FontPreset>>,
    pub drums: Option<FontPreset>,
}

/// How GM-complete a font's presets are: the GM programs it has on bank 0, then whether
/// it has a kit (bank 128). Compare with `>`.
pub fn gm_completeness(presets: &[Preset]) -> (u8, bool) {
    let mut have = [false; 128];
    for p in presets.iter().filter(|p| p.bank == 0) {
        have[p.program as usize & 127] = true;
    }
    (have.iter().filter(|&&h| h).count() as u8, presets.iter().any(|p| p.bank >= 128))
}

/// The most GM-complete of `fonts` (file name, its presets), the first file name on a tie:
/// the font the auto-fill tries first, and the synth's main font (it plays a channel no
/// route covers). None when there are no fonts.
pub fn best_font(fonts: &[(String, Vec<Preset>)]) -> Option<String> {
    fonts.iter().min_by(|a, b| gm_completeness(&b.1).cmp(&gm_completeness(&a.1)).then_with(|| a.0.cmp(&b.0))).map(|f| f.0.clone())
}

impl AutoFill {
    /// Fill every program from `fonts` (file name, its presets): the fonts in order of
    /// [`gm_completeness`] (best first, then file name), each program taking the first
    /// font's bank-0 preset of that program, else the first font with it on any melodic
    /// bank (the lowest bank). Drums take the first font's kit 0, else its lowest kit.
    pub fn build(fonts: &[(String, Vec<Preset>)]) -> AutoFill {
        let mut order: Vec<&(String, Vec<Preset>)> = fonts.iter().collect();
        order.sort_by(|a, b| gm_completeness(&b.1).cmp(&gm_completeness(&a.1)).then_with(|| a.0.cmp(&b.0)));
        let pick = |want: &dyn Fn(&Preset) -> bool| -> Option<FontPreset> {
            order.iter().find_map(|(file, presets)| presets.iter().filter(|p| want(p)).min_by_key(|p| (p.bank, p.program)).map(|p| FontPreset::new(file.clone(), p.bank, p.program)))
        };
        let programs = (0..128u8)
            .map(|prog| pick(&|p: &Preset| p.bank == 0 && p.program == prog).or_else(|| pick(&|p: &Preset| p.bank < 128 && p.program == prog)))
            .collect();
        let drums = pick(&|p: &Preset| p.bank >= 128 && p.program == 0).or_else(|| pick(&|p: &Preset| p.bank >= 128));
        AutoFill { programs, drums }
    }

    /// The auto-fill for a program, or the drum parts.
    pub fn get(&self, drum: bool, program: u8) -> Option<&FontPreset> {
        if drum {
            self.drums.as_ref()
        } else {
            self.programs.get(program as usize & 127).and_then(Option::as_ref)
        }
    }
}

/// What a program resolved to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GmResolution {
    /// The Sound, as a [`SoundId`] string (`saved:<patch>` for a rule, `sf:…` for auto);
    /// None when nothing covers it.
    pub sound: Option<String>,
    pub layer: Layer,
    /// The rule is the style's own (not the global map's).
    pub from_style: bool,
    /// The font preset that plays, when a font does (a font patch's rule, or auto): D6's
    /// provenance. None for a plugin sound, and for nothing.
    pub font: Option<FontPreset>,
}

/// Resolve one program (or the drum parts) through the library's maps: the style map
/// (`style`, a key of `style_maps`) then the global one, layer by layer as
/// [`super::resolve`] has it, then `auto`. A rule naming a patch the library does not
/// have is skipped (it falls through).
pub fn resolve_gm(lib: &SoundLibrary, style: Option<&str>, auto: &AutoFill, drum: bool, program: u8) -> GmResolution {
    let style_map: Option<&ProgramMap> = style.and_then(|k| lib.style_maps.get(k));
    let r = super::resolve(&lib.map, style_map, drum, program);
    if let Some(patch) = r.patch.and_then(|id| lib.patch(id)) {
        return GmResolution {
            sound: Some(SoundId::Library(patch.id.clone()).to_string()),
            layer: r.rule.into(),
            from_style: r.from_style,
            font: patch.source.font_preset(),
        };
    }
    match auto.get(drum, program) {
        Some(f) => GmResolution { sound: Some(SoundId::Font(f.clone()).to_string()), layer: Layer::Auto, from_style: false, font: Some(f.clone()) },
        None => GmResolution { sound: None, layer: Layer::None, from_style: false, font: None },
    }
}

/// One row of the map page: a GM program (or the drums), its family, the rules that
/// apply at each layer (patch ids, the style's where it has one) and what it resolves to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GmMapRow {
    /// The GM program, 0-127; None for the drums row.
    pub program: Option<u8>,
    /// Its GM family, 0-15 (None for the drums row).
    pub family: Option<u8>,
    /// The program's override rule (a patch id), if any.
    pub override_rule: Option<String>,
    /// Its family's rule (a patch id), if any; on the drums row, the drum rule.
    pub family_rule: Option<String>,
    pub resolved: GmResolution,
}

/// The map page's 129 rows: the drums, then programs 0-127 (families in order).
pub fn gm_map_rows(lib: &SoundLibrary, style: Option<&str>, auto: &AutoFill) -> Vec<GmMapRow> {
    let style_map = style.and_then(|k| lib.style_maps.get(k));
    let either = |f: &dyn Fn(&ProgramMap) -> Option<String>| style_map.and_then(f).or_else(|| f(&lib.map));
    let mut rows = vec![GmMapRow {
        program: None,
        family: None,
        override_rule: None,
        family_rule: either(&|m| m.drums.clone()),
        resolved: resolve_gm(lib, style, auto, true, 0),
    }];
    for p in 0..128u8 {
        let fam = family_of(p);
        rows.push(GmMapRow {
            program: Some(p),
            family: Some(fam as u8),
            override_rule: either(&|m| m.override_of(p).map(str::to_string)),
            family_rule: either(&|m| m.families[fam].clone()),
            resolved: resolve_gm(lib, style, auto, false, p),
        });
    }
    rows
}
