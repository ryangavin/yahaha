//! The default sound set (#117): the SoundFont that plays whatever no rule of the program
//! map covers (Style parts' GM voices, the keyboard parts' GM voices). There is no
//! canonical SoundFont any more: every `.sf2` in the SoundFont folder is a source of
//! sounds, and this setting picks the one the band falls back to.
//!
//! The choice is a file name in the folder, or Auto: the most GM-complete font there
//! ([`gm_score`]). It is saved in `sound-settings.json` in the data folder. `--sf2` (the
//! hidden override) wins at start, without changing the saved choice.

use super::Control;
use crate::api::CmdError;
use crate::patches::sf2::{self, Preset};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The file name in the data folder.
pub const FILE_NAME: &str = "sound-settings.json";

/// What `sound-settings.json` holds. Unknown fields are kept out of the way, so a newer
/// yahaha's additions don't stop this one reading the file.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundSettings {
    /// The default sound set, a file name in the SoundFont folder (None: Auto).
    #[serde(default)]
    pub default_sound_set: Option<String>,
}

/// How GM-complete a SoundFont is: the GM programs it has on bank 0, and whether it has a
/// drum kit (bank 128). Compare scores with `>`: programs first, then the kit.
pub fn gm_score(presets: &[Preset]) -> (u8, bool) {
    let mut have = [false; 128];
    for p in presets.iter().filter(|p| p.bank == 0) {
        have[p.program as usize & 127] = true;
    }
    (have.iter().filter(|&&h| h).count() as u8, presets.iter().any(|p| p.bank == 128))
}

/// The Auto choice among `files` (in `dir`): the best [`gm_score`], the first file name on
/// a tie. A file that doesn't read scores nothing. None when there are no files.
pub fn auto_pick(dir: &Path, files: &[String]) -> Option<String> {
    let mut best: Option<(&String, (u8, bool))> = None;
    for f in files {
        let score = sf2::presets(&dir.join(f)).map(|p| gm_score(&p)).unwrap_or_default();
        if best.is_none_or(|(_, b)| score > b) {
            best = Some((f, score));
        }
    }
    best.map(|(f, _)| f.clone())
}

/// The setting as the control side keeps it.
#[derive(Debug, Default)]
pub(super) struct SoundSet {
    /// The saved choice (None: Auto).
    pub(super) choice: Option<String>,
    /// What Auto picks from the folder as last listed.
    pub(super) auto: Option<String>,
    /// Where it is saved (None: nowhere, e.g. tests without a data folder).
    file: Option<PathBuf>,
}

impl SoundSet {
    /// Read the saved choice (a missing or unreadable file is Auto) and work out Auto.
    pub(super) fn open(data_dir: Option<&Path>, sf_dir: Option<&Path>, fonts: &[String]) -> SoundSet {
        let file = data_dir.map(|d| d.join(FILE_NAME));
        let choice = file
            .as_deref()
            .and_then(|f| std::fs::read_to_string(f).ok())
            .and_then(|t| serde_json::from_str::<SoundSettings>(&t).ok())
            .and_then(|s| s.default_sound_set);
        let mut s = SoundSet { choice, auto: None, file };
        s.refresh(sf_dir, fonts);
        s
    }

    /// Work Auto out again for the folder's `fonts`.
    pub(super) fn refresh(&mut self, sf_dir: Option<&Path>, fonts: &[String]) {
        self.auto = sf_dir.and_then(|d| auto_pick(d, fonts));
    }

    /// The font the setting names: the choice while it is in the folder, else Auto.
    pub(super) fn resolve(&self, fonts: &[String]) -> Option<String> {
        self.choice.clone().filter(|c| fonts.contains(c)).or_else(|| self.auto.clone())
    }

    fn save(&self) -> anyhow::Result<()> {
        write_key(self.file.as_deref(), "defaultSoundSet", serde_json::to_value(&self.choice)?)
    }

    /// Where the settings are saved (None: nowhere).
    pub(super) fn file(&self) -> Option<&Path> {
        self.file.as_deref()
    }
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
    /// `SetDefaultSoundSet`: save the choice, and load the font it names when the synth
    /// plays another one.
    pub(super) fn set_default_sound_set(&mut self, file: Option<String>) -> Result<(), CmdError> {
        self.list_sound_fonts();
        if let Some(f) = &file
            && !self.sound_fonts.contains(f)
        {
            return self.fail(format!("no SoundFont {f} in the SoundFont folder"));
        }
        self.sound_set.choice = file;
        if let Err(e) = self.sound_set.save() {
            self.say(format!("The default sound set was not saved: {e:#}"), true);
        }
        let Some(want) = self.sound_set.resolve(&self.sound_fonts) else { return Ok(()) };
        let loading = self.sf_load.as_ref().map(|(f, _)| f).or(self.sf_ready.as_ref().map(|(f, _)| f));
        let playing = loading.or(self.sf_file.as_ref());
        if self.synth.is_none() {
            // No synth (offline, `--no-synth`): nothing to load; the next start plays it.
            return Ok(());
        }
        if playing != Some(&want) {
            return self.load_sound_font(want);
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "sound_set_tests.rs"]
mod tests;
