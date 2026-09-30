//! Putting a stored sound back on a keyboard part: a plugin with its state, a library
//! patch, or the GM voice. Rack apply uses these.

use super::sound_library::{at_number, number_of};
use super::{Control, PluginVoice};
use crate::api::{base64_decode, PluginStatus};
use crate::parts;
use crate::patches::SoundTag;

impl Control {
    /// `swapSound` (swap mode, docs/eyes-free.md): step keyboard part `part`'s sound by
    /// `step` sound numbers (`sound_library::number_of` / `at_number`), live, keeping its
    /// mix as `replacePartSound` does. The dial stops at the first and last number (it
    /// doesn't wrap), so turning hard left always lands on 1. A part playing no numbered
    /// sound (its GM voice, or a plugin picked on its own) dials from before 1: a step up
    /// lands on 1. A plugin sound loads as a patch's plugin does; the part's GM voice plays
    /// until it is ready. Steps aren't recents: dialling through sounds would flood them.
    pub(super) fn swap_sound(&mut self, part: u8, step: i32) -> Result<(), crate::api::CmdError> {
        if part as usize >= parts::COUNT {
            return self.fail(format!("no keyboard part {part}"));
        }
        let p = part as usize;
        let lib = &self.sound.lib;
        let last = lib.patches.len() as i64;
        if last == 0 {
            return self.fail(format!("{}: no sounds in the library to swap to", parts::NAMES[p]));
        }
        let now = self.part_patch(p).and_then(|(id, _)| number_of(lib, &id)).unwrap_or(0);
        let to = (now as i64 + step as i64).clamp(1, last) as u32;
        if to == now {
            return Ok(());
        }
        let Some(id) = at_number(lib, to).map(|s| s.id.clone()) else {
            return self.fail(format!("no sound number {to}"));
        };
        let mix = self.capture_rack_part(p, false);
        self.set_part_patch(p, Some(id))?;
        if let Err(e) = self.apply_rack_mix(p, &mix) {
            return self.fail(e);
        }
        self.wake_engine();
        Ok(())
    }

    /// Put plugin `id` with `state` (base64; None: its default preset) on part `p`:
    /// nothing when it already plays that plugin with that state (playing or loading);
    /// else load it as `setPartPlugin` does (the part's library patch ends). Ok(true) when
    /// a load started. A plugin that can't play (not installed, no plugin host in this
    /// build) stays on the part as failed (retryable, saved), so the part's choice
    /// survives; its GM voice plays meanwhile, or nothing if the plugin isn't installed
    /// (docs/racks.md, "Plugins coming and going"), and it says so.
    ///
    /// `sound` is the Sound the record names; a record from before sounds had ids names
    /// none, and gets the library's sound with exactly that state, if there is one.
    pub(super) fn recall_part_plugin(&mut self, p: usize, id: &str, name: &str, state: Option<&str>, sound: Option<&SoundTag>) -> Result<bool, String> {
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
        let sound = sound.cloned().or_else(|| self.sound_tag_for_state(id, state.unwrap_or_default()));
        let voice = PluginVoice { id: id.to_string(), state: bytes, preset: None, sound };
        let r = self.assign_channel_plugin(ch, voice.clone());
        let mut missing = false;
        if let Err(e) = &r {
            self.clear_channel_plugin(ch);
            // A plugin that isn't installed leaves the part silent (`mark_missing`).
            missing = self.plugin_is_missing(id);
            self.keep_failed_channel_plugin(ch, voice, e.clone());
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

#[cfg(test)]
pub(super) mod tests {
    use crate::api::*;
    use crate::launchkey;
    use crate::patches::Category;
    use crate::session::testing::session;
    use crate::session::{Port, Session};

    /// A session with library sounds named `names`; the ones in `favourites` are
    /// favourites (so the sound numbers put them first once they count).
    pub(in crate::session) fn with_sounds(names: &[&str], favourites: &[&str]) -> Session {
        let s = session();
        for (i, name) in names.iter().enumerate() {
            let patch = PatchFields {
                name: name.to_string(),
                category: Category::guess(0, i as u8),
                tags: vec![],
                favourite: favourites.contains(name),
                source: PatchSource::SoundFont { file: "Test.sf2".into(), bank: 0, program: i as u8 },
            };
            s.send(SoundLibraryCmd::CreatePatch { patch }).unwrap();
        }
        s
    }

    /// The number of the sound keyboard part `part` plays (0: none), and its name.
    pub(in crate::session) fn part_number(s: &Session, part: usize) -> (u32, String) {
        let st = s.state();
        let Some(id) = st.keyboard_parts[part].patch.clone() else { return (0, String::new()) };
        let p = st.sound_library.patches.iter().find(|p| p.patch.id == id).unwrap();
        (p.number, p.patch.name.clone())
    }

    /// The name of the sound numbered `n`.
    fn named(s: &Session, n: u32) -> String {
        s.state().sound_library.patches.iter().find(|p| p.number == n).unwrap().patch.name.clone()
    }

    /// Hold Panel fader button `part + 1`.
    pub(in crate::session) fn hold(s: &Session, part: u8, down: bool) {
        s.midi_in(Port::Pads, &[0xB0, launchkey::FADER_BTN_CC.start() + part, if down { 127 } else { 0 }]);
    }

    /// Turn knob `knob` (0-7) by `delta` encoder steps.
    pub(in crate::session) fn turn(s: &Session, knob: u8, delta: i8) {
        s.midi_in(Port::Pads, &[launchkey::ENCODER_STATUS, launchkey::ENCODER_CC.start() + knob, (64 + delta) as u8]);
    }

    /// `swapSound` steps by sound number, keeping the mix, and stops at the first and last
    /// number rather than wrapping. A part playing no numbered sound dials from before 1.
    #[test]
    fn swap_sound_steps_by_number_and_stops_at_the_ends() {
        let s = with_sounds(&["Grand", "Rhodes Soft", "Strings", "Nylon"], &["Strings"]);
        s.send(PartsCmd::SetPartVolume { part: 1, volume: 55 }).unwrap();
        s.send(PartsCmd::SetPartOctave { part: 1, octave: -1 }).unwrap();
        s.send(PartsCmd::SetPartPan { part: 1, pan: 20 }).unwrap();
        assert_eq!(part_number(&s, 1).0, 0);
        s.send(PartsCmd::SwapSound { part: 1, step: 1 }).unwrap();
        assert_eq!(part_number(&s, 1), (1, named(&s, 1)));
        s.send(PartsCmd::SwapSound { part: 1, step: 2 }).unwrap();
        assert_eq!(part_number(&s, 1), (3, named(&s, 3)));
        let st = s.state();
        let r2 = &st.keyboard_parts[1];
        assert_eq!((r2.volume, r2.octave, r2.pan), (55, -1, 20), "the mix stays");
        assert_eq!(r2.voice_name, named(&s, 3), "it plays the sound at once");
        // Past the last: the last. Past the first: the first.
        s.send(PartsCmd::SwapSound { part: 1, step: 100 }).unwrap();
        assert_eq!(part_number(&s, 1).0, 4);
        s.send(PartsCmd::SwapSound { part: 1, step: 1 }).unwrap();
        assert_eq!(part_number(&s, 1).0, 4);
        s.send(PartsCmd::SwapSound { part: 1, step: -100 }).unwrap();
        assert_eq!(part_number(&s, 1).0, 1);
        s.send(PartsCmd::SwapSound { part: 1, step: -1 }).unwrap();
        assert_eq!(part_number(&s, 1).0, 1);
        // Other parts are left alone; a bad part is refused.
        assert_eq!(part_number(&s, 0).0, 0);
        assert!(s.send(PartsCmd::SwapSound { part: 4, step: 1 }).is_err());
    }

    /// Every step lands on the sound with the next number, whatever order the numbers
    /// give the library (favourites first), across the favourites' end.
    #[test]
    fn each_step_lands_on_the_next_number() {
        let names = ["Grand", "Rhodes Soft", "Strings", "Nylon", "Brass"];
        let s = with_sounds(&names, &["Nylon", "Strings"]);
        let mut seen = Vec::new();
        for n in 1..=names.len() as u32 {
            s.send(PartsCmd::SwapSound { part: 3, step: 1 }).unwrap();
            let (number, name) = part_number(&s, 3);
            assert_eq!(number, n);
            assert_eq!(name, named(&s, n));
            seen.push(name);
        }
        seen.sort();
        let mut all = names.map(String::from).to_vec();
        all.sort();
        assert_eq!(seen, all, "every sound once");
        for n in (1..names.len() as u32).rev() {
            s.send(PartsCmd::SwapSound { part: 3, step: -1 }).unwrap();
            assert_eq!(part_number(&s, 3).0, n);
        }
    }

    /// With no sounds in the library there's nothing to swap to: refused, and it says so.
    #[test]
    fn an_empty_library_refuses() {
        let s = session();
        assert!(s.send(PartsCmd::SwapSound { part: 0, step: 1 }).is_err());
        assert!(s.state().message.as_ref().is_some_and(|m| m.error));
    }

    /// On the Launchkey: hold Right 2's button and turn knob 1. Each step sounds at once;
    /// the release changes nothing more; a tap with no turn still toggles the part.
    #[test]
    fn hold_and_knob_1_swaps_live_and_release_commits() {
        let s = with_sounds(&["Grand", "Rhodes Soft", "Strings"], &[]);
        let on = s.state().keyboard_parts[1].on;
        hold(&s, 1, true);
        turn(&s, 0, 1);
        assert_eq!(s.state().surface.layer, launchkey::Layer::Swap { part: 1 });
        assert_eq!(part_number(&s, 1).0, 1);
        turn(&s, 0, 2);
        assert_eq!(part_number(&s, 1).0, 3);
        turn(&s, 0, -1);
        assert_eq!(part_number(&s, 1).0, 2);
        hold(&s, 1, false);
        let st = s.state();
        assert_eq!(st.surface.layer, launchkey::Layer::None);
        assert_eq!(part_number(&s, 1).0, 2, "the release keeps the sound");
        assert_eq!(st.keyboard_parts[1].on, on, "a swap is not a tap");
        hold(&s, 1, true);
        hold(&s, 1, false);
        assert_eq!(s.state().keyboard_parts[1].on, !on);
        assert_eq!(part_number(&s, 1).0, 2);
    }
}
