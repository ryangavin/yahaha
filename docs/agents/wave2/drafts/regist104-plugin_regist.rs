//! A keyboard part's plugin in a Registration Memory (#104): the `parts` registrable
//! (`registration/sections.rs`) stores a plugin picked on the Plugins tab as
//! `VoiceRef::Plugin` (its id, name and full state, base64), and recalls it through the
//! path `setPartPlugin` takes (#91's `assign_channel_plugin`). A plugin a part plays for
//! its own library patch is not stored here: the part's `patch` is (#109).

use super::{Control, PluginVoice};
use crate::api::{base64_decode, PluginStatus};
use crate::parts;
use crate::registration::VoiceRef;
use std::sync::atomic::Ordering::Relaxed;

/// The largest plugin state a registration stores, as `setPartPlugin` takes it (64 MB).
/// A larger one is stored without it (the plugin's default preset), and Memorize says so.
pub(super) const MAX_STATE_BYTES: usize = 64 << 20;

/// The base64 length of `MAX_STATE_BYTES`.
const MAX_STATE_B64: usize = MAX_STATE_BYTES.div_ceil(3) * 4;

impl Control {
    /// Part `p`'s plugin from the Plugins tab, as a registration stores it: None when it
    /// plays none (or its library patch's).
    pub(super) fn part_plugin_reg(&self, p: usize) -> Option<VoiceRef> {
        if self.part_has_patch_plugin(p) {
            return None;
        }
        let (id, state) = self.part_plugin_voice(p)?;
        let name = self.channel_plugin_state(parts::CHANNEL[p]).map(|s| s.name).unwrap_or_default();
        let state = state.filter(|s| s.len() <= MAX_STATE_B64);
        Some(VoiceRef::Plugin { id, name, state, program: self.shared.parts.program[p].load(Relaxed) & 127 })
    }

    /// Before a Memorize: read the state of each part's playing plugin (as the editor left
    /// it), so the registration stores how it sounds now rather than the last autosave.
    pub(super) fn refresh_part_plugin_states(&mut self) {
        for p in 0..parts::COUNT {
            let ch = parts::CHANNEL[p];
            let playing = self.channel_plugin_state(ch).is_some_and(|s| s.status == PluginStatus::Playing);
            if !playing || self.part_has_patch_plugin(p) || self.save_channel_state(ch).is_err() {
                continue;
            }
            if self.part_plugin_voice(p).and_then(|v| v.1).is_some_and(|s| s.len() > MAX_STATE_B64) {
                self.say(format!("{}: the plugin's settings are larger than {} MB: the registration keeps its default preset", parts::NAMES[p], MAX_STATE_BYTES >> 20), true);
            }
        }
    }

    /// Recall part `p`'s plugin: nothing when it already plays that plugin with that
    /// state (playing or loading); else load it as `setPartPlugin` does (the part's library
    /// patch ends). A plugin that can't play (not installed, no plugin host in this build)
    /// leaves the part on its GM voice, and says so.
    pub(super) fn recall_part_plugin(&mut self, p: usize, id: &str, name: &str, state: Option<&str>) -> Result<(), String> {
        let ch = parts::CHANNEL[p];
        let same = self.part_plugin_voice(p).is_some_and(|(i, s)| i == id && s.as_deref() == state)
            && self.channel_plugin_state(ch).is_some_and(|s| matches!(s.status, PluginStatus::Playing | PluginStatus::Loading))
            && !self.part_has_patch_plugin(p);
        if same {
            return Ok(());
        }
        let name = if name.is_empty() { id } else { name };
        let bytes = match state {
            Some(s) => Some(base64_decode(s).ok_or_else(|| format!("{}: {name}'s stored settings are not readable", parts::NAMES[p]))?),
            None => None,
        };
        self.sound_library_part_plugin(p, true);
        let r = self.assign_channel_plugin(ch, PluginVoice { id: id.to_string(), state: bytes });
        if r.is_err() {
            self.clear_channel_plugin(ch);
        }
        self.mark_plugins_dirty();
        r.map_err(|e| format!("{}: {name} can't play ({e}); it plays its GM voice", parts::NAMES[p]))
    }

    /// A GM voice recalled on part `p`: a plugin picked on the Plugins tab goes (the part's
    /// library patch, and a plugin that patch plays, are the `patch` recall's).
    pub(super) fn clear_part_tab_plugin(&mut self, p: usize) {
        let ch = parts::CHANNEL[p];
        if self.channel_plugin_state(ch).is_some() && !self.part_has_patch_plugin(p) {
            self.clear_channel_plugin(ch);
            self.mark_plugins_dirty();
        }
    }
}
