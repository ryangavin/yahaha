//! The GM map's auto-fill layer, built from the scanned SoundFonts (docs/sound-browser.md,
//! D3 and D4). There is no default sound set any more: a program no rule covers plays the
//! best-matching preset in the fonts ([`AutoFill`]), and the synth's main font (the one it
//! plays a channel no route covers) is the most GM-complete of them ([`best_font`]).
//!
//! Everything here runs on the control side, when the SoundFont folder changes: the fonts'
//! preset lists are read (their PHDR chunk only), and the session writes the result into
//! the route table off-thread (`SoundLib::write_bank`).
//!
//! `sound-settings.json` in the data folder keeps the sound browser's settings
//! (session/sounds.rs). An old `defaultSoundSet` key in it is ignored.

use super::Control;
use crate::patches::sf2::{self, Preset};
use crate::patches::{best_font, AutoFill};
use std::path::{Path, PathBuf};

/// The file name in the data folder.
pub const FILE_NAME: &str = "sound-settings.json";

/// The fonts in `dir` with their presets. A file that doesn't read has none (it fills
/// nothing and is never the main font while another font reads).
pub(super) fn font_presets(dir: &Path, files: &[String]) -> Vec<(String, Vec<Preset>)> {
    files.iter().map(|f| (f.clone(), sf2::presets(&dir.join(f)).unwrap_or_default())).collect()
}

/// The auto-fill for `files` in `dir`, and the main font (the most GM-complete). With the
/// hidden `--sf2` pin, the pinned font fills every program it has (as it played them
/// before the map), and the others fill only its gaps.
pub(super) fn build(dir: Option<&Path>, files: &[String], pin: Option<&str>) -> (AutoFill, Option<String>) {
    let Some(dir) = dir else { return (AutoFill::default(), None) };
    let fonts = font_presets(dir, files);
    let mut auto = AutoFill::build(&fonts);
    if let Some(own) = fonts.iter().find(|f| Some(f.0.as_str()) == pin) {
        let own = AutoFill::build(std::slice::from_ref(own));
        for (a, o) in auto.programs.iter_mut().zip(own.programs) {
            if o.is_some() {
                *a = o;
            }
        }
        if own.drums.is_some() {
            auto.drums = own.drums;
        }
    }
    (auto, best_font(&fonts))
}

/// Where `sound-settings.json` lives (None: nowhere, e.g. tests without a data folder).
pub(super) fn settings_file(data_dir: Option<&Path>) -> Option<PathBuf> {
    data_dir.map(|d| d.join(FILE_NAME))
}

/// Set one key of `sound-settings.json` at `path` (None: nowhere), keeping the others
/// (a newer yahaha's too).
pub(super) fn write_key(path: Option<&Path>, key: &str, value: serde_json::Value) -> anyhow::Result<()> {
    let Some(path) = path else { return Ok(()) };
    let mut v: serde_json::Value = std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .filter(|v: &serde_json::Value| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}));
    v[key] = value;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(&v)?)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

impl Control {
    /// The font the synth should play as its main one: the hidden `--sf2` pin, else the
    /// most GM-complete font in the folder, else what it plays now.
    pub(super) fn wanted_main_font(&self) -> Option<String> {
        self.sf_pin.clone().or_else(|| self.sound.best.clone()).or_else(|| self.sf_file.clone())
    }

    /// The SoundFont folder changed (or was first listed): build the auto-fill again and
    /// route every channel through it (off the real-time threads).
    pub(super) fn gm_auto_changed(&mut self) {
        let (auto, best) = build(self.sf_dir.as_deref(), &self.sound_fonts, self.sf_pin.as_deref());
        self.sound.auto = auto;
        self.sound.best = best;
        self.gm_routes_changed();
    }

    /// Rewrite the route table for what the map resolves to now (the auto-fill or the main
    /// font moved).
    pub(super) fn gm_routes_changed(&mut self) {
        self.sound.native = self.sf_file.clone();
        let (avail, routes) = (self.avail_fonts(), self.shared.routes.clone());
        self.sound.write_all(&routes, &avail);
        self.sync_channel_routes();
    }
}

#[cfg(test)]
#[path = "gm_auto_tests.rs"]
mod tests;
