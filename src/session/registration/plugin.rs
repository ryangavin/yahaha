//! A keyboard part's plugin in a Registration Memory (#104): the `parts` registrable
//! (`sections.rs`) stores a plugin picked on the Plugins tab as `VoiceRef::Plugin` (its id,
//! name and full state, base64), and recalls it through the path `setPartPlugin` takes
//! (#91's `assign_channel_plugin`: the load runs on a thread of its own, never on the
//! control, engine, MIDI or audio threads). A plugin a part plays for its own library patch
//! is not stored here: the part's `patch` is (#109).
//!
//! **Memorize reads the plugins' states.** A plugin's state is read off the control thread
//! (#125), so Memorize stores the state the part last saved at once, starts a fresh read of
//! each part's playing plugin, and puts what it reads into the button when it lands
//! (`pump_plugin_fill`): the button keeps how the plugin sounded when it was memorized, not
//! the last 30 s autosave.

use super::super::Control;
use super::sections::part_group;
use crate::api::{base64_decode, PluginStatus};
use crate::parts;
use crate::registration::{Groups, VoiceRef};
use crate::session::PluginVoice;
use serde_json::Value;
use std::sync::atomic::Ordering::Relaxed;

/// The largest plugin state a registration stores, as `setPartPlugin` takes it (64 MB).
/// A larger one is stored without it (the plugin's default preset), and Memorize says so.
const MAX_STATE_BYTES: usize = 64 << 20;

/// The base64 length of `MAX_STATE_BYTES`.
const MAX_STATE_B64: usize = MAX_STATE_BYTES.div_ceil(3) * 4;

/// How long a Memorize waits for its plugin state reads at most (a plugin that hangs
/// answering leaves the button with the state saved before).
const FILL_WAIT_NS: u64 = 10_000_000_000;

/// A Memorize waiting for its parts' plugin states.
pub(super) struct PluginFill {
    /// The button memorized.
    button: usize,
    /// (part, plugin id, the state stored at Memorize).
    parts: Vec<(usize, String, Option<String>)>,
    /// When the pump first saw it (0 = not yet).
    since: u64,
}

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

    /// Recall part `p`'s plugin: nothing when it already plays that plugin with that
    /// state (playing or loading); else load it as `setPartPlugin` does (the part's library
    /// patch ends). A plugin that can't play (not installed, no plugin host in this build)
    /// leaves the part on its GM voice, and says so.
    pub(super) fn recall_part_plugin(&mut self, p: usize, id: &str, name: &str, state: Option<&str>) -> Result<(), String> {
        let ch = parts::CHANNEL[p];
        let same = !self.part_has_patch_plugin(p)
            && self.part_plugin_voice(p).is_some_and(|(i, s)| i == id && s.as_deref() == state)
            && self.channel_plugin_state(ch).is_some_and(|s| matches!(s.status, PluginStatus::Playing | PluginStatus::Loading));
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

    /// After a Memorize into `button` of `groups`: read the state of each stored part's
    /// playing plugin (on `plugin-state` threads), for `pump_plugin_fill`.
    pub(super) fn start_plugin_fill(&mut self, button: usize, groups: Groups) {
        let mut fill = Vec::new();
        for p in 0..parts::COUNT {
            if !groups.has(part_group(p)) || self.part_has_patch_plugin(p) {
                continue;
            }
            let ch = parts::CHANNEL[p];
            if !self.channel_plugin_state(ch).is_some_and(|s| s.status == PluginStatus::Playing) {
                continue;
            }
            let Some(VoiceRef::Plugin { id, state, .. }) = self.part_plugin_reg(p) else { continue };
            if self.save_channel_state(ch).is_ok() {
                fill.push((p, id, state));
            }
        }
        self.reg.plugin_fill = (!fill.is_empty()).then_some(PluginFill { button, parts: fill, since: 0 });
    }

    /// Every pump: once a Memorize's plugin state reads are done, their states go into
    /// the button (and the bank file). A part whose plugin changed since, or a button
    /// memorized again, is left as it is.
    pub(super) fn pump_plugin_fill(&mut self, now: u64) {
        let Some(f) = self.reg.plugin_fill.as_mut() else { return };
        if f.since == 0 {
            f.since = now.max(1);
            return;
        }
        let since = f.since;
        if self.plugin_state_reads_pending() && now.saturating_sub(since) < FILL_WAIT_NS {
            return;
        }
        let f = self.reg.plugin_fill.take().unwrap();
        let mut changed = false;
        for (p, id, old) in f.parts {
            let playing = self.channel_plugin_state(parts::CHANNEL[p]).is_some_and(|s| s.status == PluginStatus::Playing);
            let Some((now_id, Some(new))) = self.part_plugin_voice(p) else { continue };
            if !playing || now_id != id || self.part_has_patch_plugin(p) {
                continue;
            }
            if new.len() > MAX_STATE_B64 {
                self.say(format!("{}: the plugin's settings are larger than {} MB: the registration keeps its default preset", parts::NAMES[p], MAX_STATE_BYTES >> 20), true);
                continue;
            }
            let Some(m) = self.reg.bank.memories[f.button].as_mut() else { break };
            let Some(Value::Object(v)) = m.sections.get_mut("parts").and_then(|s| s.pointer_mut(&format!("/parts/{p}/voice"))) else { continue };
            let stored = (v.get("kind"), v.get("id"), v.get("state"));
            if stored.0.and_then(Value::as_str) != Some("plugin") || stored.1.and_then(Value::as_str) != Some(id.as_str()) || stored.2.and_then(Value::as_str) != old.as_deref() {
                continue;
            }
            if old.as_deref() != Some(new.as_str()) {
                v.insert("state".into(), Value::String(new));
                changed = true;
            }
        }
        if changed {
            // A bank file that can't be written says so itself.
            let _ = self.bank_changed();
        }
    }
}

#[cfg(all(test, feature = "plugins"))]
#[path = "plugin_tests.rs"]
mod tests;
