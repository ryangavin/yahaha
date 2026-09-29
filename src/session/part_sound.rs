//! Putting a stored sound back on a keyboard part: a plugin with its state, a library
//! patch, or the GM voice. Registration recall and rack apply both use these.

use super::{Control, PluginVoice};
use crate::api::{base64_decode, PluginStatus};
use crate::parts;
use crate::patches::SoundTag;

/// What a failed plugin load leaves on the part.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum OnFail {
    /// Back to the GM voice: the plugin is forgotten.
    Clear,
    /// The plugin stays on the part as failed (retryable, saved), so the part's choice
    /// survives; its GM voice plays meanwhile, or nothing if the plugin isn't installed
    /// (docs/racks.md, "Plugins coming and going").
    Keep,
}

impl Control {
    /// Put plugin `id` with `state` (base64; None: its default preset) on part `p`:
    /// nothing when it already plays that plugin with that state (playing or loading);
    /// else load it as `setPartPlugin` does (the part's library patch ends). Ok(true) when
    /// a load started. A plugin that can't play (not installed, no plugin host in this
    /// build) leaves the part on its GM voice, and says so.
    ///
    /// `sound` is the Sound the record names; a record from before sounds had ids names
    /// none, and gets the library's sound with exactly that state, if there is one.
    pub(super) fn recall_part_plugin(&mut self, p: usize, id: &str, name: &str, state: Option<&str>, sound: Option<&SoundTag>, on_fail: OnFail) -> Result<bool, String> {
        let ch = parts::CHANNEL[p];
        let same = !self.part_has_patch_plugin(p)
            && self.part_plugin_voice(p).is_some_and(|(i, s)| i == id && s.as_deref() == state)
            && self.channel_plugin_state(ch).is_some_and(|s| matches!(s.status, PluginStatus::Playing | PluginStatus::Loading));
        if same {
            return Ok(false);
        }
        let name = if name.is_empty() { id } else { name };
        let bytes = match state {
            Some(s) => Some(base64_decode(s).ok_or_else(|| format!("{}: {name}'s stored settings are not readable", parts::NAMES[p]))?),
            None => None,
        };
        self.sound_library_part_plugin(p, true);
        // A preloaded instance is used up (by a Registration or a rack): the pool refills
        // at the next pump.
        self.reg.warm_dirty = true;
        let sound = sound.cloned().or_else(|| self.sound_tag_for_state(id, state.unwrap_or_default()));
        let voice = PluginVoice { id: id.to_string(), state: bytes, preset: None, sound };
        let r = self.assign_channel_plugin(ch, voice.clone());
        let mut missing = false;
        if let Err(e) = &r {
            self.clear_channel_plugin(ch);
            if on_fail == OnFail::Keep {
                // A plugin that isn't installed leaves the part silent (`mark_missing`).
                missing = self.plugin_is_missing(id);
                self.keep_failed_channel_plugin(ch, voice, e.clone());
            }
        }
        self.mark_plugins_dirty();
        r.map(|_| true).map_err(|e| match missing {
            true => {
                // Named as it was last installed, if it was.
                let name = self.channel_plugin_state(ch).map_or_else(|| name.to_string(), |s| s.name);
                format!("{}: {name} is not installed; the part is silent until it is back", parts::NAMES[p])
            }
            false => format!("{}: {name} can't play ({e}); it plays its GM voice", parts::NAMES[p]),
        })
    }

    /// A GM voice (or a library patch) put on part `p`: a plugin picked on the Plugins tab
    /// goes (the part's library patch, and a plugin that patch plays, are the patch's).
    pub(super) fn clear_part_tab_plugin(&mut self, p: usize) {
        let ch = parts::CHANNEL[p];
        if self.channel_plugin_state(ch).is_some() && !self.part_has_patch_plugin(p) {
            self.clear_channel_plugin(ch);
            self.mark_plugins_dirty();
        }
    }

    /// Part `p`'s own library patch: `patch` (its id and name; if it isn't already the
    /// part's), or none (its GM voice). A patch no longer in the sound library leaves the
    /// part on its GM voice, and says so.
    pub(super) fn recall_part_patch(&mut self, p: usize, patch: Option<(&str, &str)>) -> Result<(), String> {
        let now = self.part_patch(p).map(|(id, _)| id);
        match patch {
            Some((id, _)) if now.as_deref() == Some(id) => Ok(()),
            Some((id, _)) if self.has_patch(id) => self.set_part_patch(p, Some(id.to_string())).map_err(|e| e.to_string()),
            Some((_, name)) => {
                self.sound_library_part_voice(p);
                Err(format!("{}: patch {name} is not in the sound library; it plays its GM voice", parts::NAMES[p]))
            }
            None => {
                self.sound_library_part_voice(p);
                Ok(())
            }
        }
    }
}
