//! The rack commands (docs/racks.md, "Saving"): new, load, save, save as, revert, rename,
//! duplicate and delete the user's racks (`<data>/Racks/*.rack.json`), with the switching
//! guard, and the racks list the app shows.
//!
//! - **The guard.** `loadRack` and `newRack` with unsaved changes (`liveRack.modified`)
//!   change nothing unless sent with `discard`: they return `CmdError::UnsavedChanges` and
//!   `liveRack.prompt` holds the switch, so the app can offer Save first, Discard and
//!   switch, or Keep editing (`dismissRackPrompt`). The hardware, which has no dialog,
//!   switches anyway and keeps the unsaved rack as "Recovered: <name>" in the user's racks
//!   ([`Session::load_rack_from_hardware`]).
//! - **Saving** writes the live rack and its edited sounds in one step. A part whose
//!   plugin was edited (`soundEdited`) and plays the user's own sound saves over that
//!   sound; one playing a factory preset, an `.aupreset` file or someone else's sound makes
//!   a new sound of the user's, which needs a name (`soundNames`; without one nothing is
//!   saved, `CmdError::NeedsSoundNames`, and the prompt lists those parts). Each sound is
//!   one record; the rack then names it, with no edit of its own.
//! - Loading or saving leaves the live rack that rack, unmodified, and autosaves it.

use super::{Control, Session};
use crate::api::{CmdError, PluginsState, RackCmd, RackEntry, RackPrompt, RackSwitch, SoundNameAsk};
use crate::parts;
use crate::patches::FontPreset;
use crate::racks::{self, Rack, RackPart, SoundRef, ToneReg};
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

/// What a recovered rack's name starts with.
pub const RECOVERED: &str = "Recovered: ";

impl Control {
    pub(super) fn rack_cmd(&mut self, c: RackCmd) -> Result<(), CmdError> {
        match c {
            RackCmd::NewRack { discard } => self.switch_rack(RackSwitch::New, discard),
            RackCmd::LoadRack { id, discard } => {
                let r = self.read_rack(&id)?;
                self.switch_rack(RackSwitch::Load { id, name: r.name }, discard)
            }
            RackCmd::SaveRack { sound_names } => self.save_rack_holding(None, &sound_names),
            RackCmd::SaveRackAs { name, sound_names } => self.save_rack_holding(Some(name), &sound_names),
            RackCmd::RevertRack => {
                let Some(id) = self.live_rack.id.clone().filter(|id| self.rack_file(id).is_some()) else {
                    return self.fail(format!("{} has no saved rack to go back to", self.live_rack.name));
                };
                let r = self.read_rack(&id)?;
                self.enter_rack(Some(r));
                Ok(())
            }
            RackCmd::RenameRack { id, name } => self.rename_rack(&id, &name),
            RackCmd::DuplicateRack { id } => {
                let dir = self.racks_dir()?;
                let mut r = self.read_rack(&id)?;
                r.name = unique_name(&dir, &format!("{} copy", r.name));
                r.id = racks::new_id();
                self.write_rack(&r, &racks::path_for(&dir, &r.name))?;
                self.say(format!("Duplicated as {}", r.name), false);
                Ok(())
            }
            RackCmd::DeleteRack { id } => {
                let Some(path) = self.rack_file(&id) else { return self.fail(format!("no rack {id}")) };
                if self.live_rack.id.as_deref() == Some(id.as_str()) {
                    return self.fail(format!("{} is loaded: load another rack before deleting it", self.live_rack.name));
                }
                if let Err(e) = std::fs::remove_file(&path) {
                    return self.fail(format!("The rack was not deleted: {e}"));
                }
                self.presence.refresh_racks(true);
                Ok(())
            }
            RackCmd::DismissRackPrompt => {
                self.live_rack.prompt = None;
                self.live_rack.held = None;
                Ok(())
            }
            RackCmd::SetRackControl { control, index, target } => self.set_rack_control(control, index, target),
            RackCmd::MoveRackFader { fader, volume } => self.move_rack_fader(fader, volume),
        }
    }

    /// The racks folder. Err: there is no data folder, so racks can't be kept.
    fn racks_dir(&mut self) -> Result<PathBuf, CmdError> {
        match self.presence.racks_dir() {
            Some(d) => Ok(d.to_path_buf()),
            None => self.refuse("racks can't be saved: there is no data folder"),
        }
    }

    /// `fail`, for a command part way through that returns a value.
    fn refuse<T>(&mut self, text: impl Into<String>) -> Result<T, CmdError> {
        let t = text.into();
        self.say(t.clone(), true);
        Err(CmdError::Failed(t))
    }

    /// Rack `id`'s file, from the folder as read now.
    fn rack_file(&mut self, id: &str) -> Option<PathBuf> {
        self.presence.refresh_racks(false);
        self.presence.racks().iter().find(|r| r.id == id).map(|r| r.path.clone())
    }

    fn read_rack(&mut self, id: &str) -> Result<Rack, CmdError> {
        let Some(path) = self.rack_file(id) else { return self.refuse(format!("no rack {id}")) };
        Rack::load(&path).or_else(|e| self.refuse(format!("{e:#}")))
    }

    fn write_rack(&mut self, r: &Rack, path: &std::path::Path) -> Result<(), CmdError> {
        let written = r.save(path);
        self.presence.refresh_racks(true);
        written.or_else(|e| self.fail(format!("The rack was not saved: {e:#}")))
    }

    /// `loadRack` / `newRack`: switch, unless that would lose unsaved changes without
    /// `discard` (then the prompt asks).
    fn switch_rack(&mut self, to: RackSwitch, discard: bool) -> Result<(), CmdError> {
        let rack = match &to {
            RackSwitch::Load { id, .. } => Some(self.read_rack(id)?),
            RackSwitch::New => None,
        };
        if self.live_rack.modified && !discard {
            self.live_rack.prompt = Some(RackPrompt::UnsavedChanges { then: to });
            return Err(CmdError::UnsavedChanges);
        }
        self.enter_rack(rack);
        Ok(())
    }

    /// The live rack becomes `rack` (None: a new one), unmodified, and is saved soon. What
    /// could not be applied (a sound that can't play) is reported; the rest still is.
    fn enter_rack(&mut self, rack: Option<Rack>) {
        self.live_rack.held = None;
        let (r, id) = match rack {
            Some(r) => {
                let id = Some(r.id.clone());
                (r, id)
            }
            None => (self.blank_rack(), None),
        };
        let problems = self.apply_rack(&r);
        self.live_rack_clean(&r.name, id);
        self.say(format!("Loaded {}", r.name), false);
        for p in problems {
            self.say(p, true);
        }
    }

    /// A new rack: every part on its default GM voice with a neutral mix (Right 1 on, the
    /// others off), the default split, no transpose, Harmony/Arpeggio off (its settings as
    /// they are) and the default controller map.
    fn blank_rack(&self) -> Rack {
        let now = self.capture_rack_with(false);
        let file = self.sf_file.clone().unwrap_or_default();
        let part = |p: usize| RackPart {
            on: p == parts::RIGHT1,
            sound: SoundRef::Font { file: file.clone(), bank: 0, program: parts::DEFAULT_PROGRAMS[p] },
            edited_state: None,
            fallback_program: None,
            volume: 100,
            pan: parts::FX_DEFAULT[p][parts::PAN],
            reverb: parts::FX_DEFAULT[p][parts::REVERB],
            chorus: parts::FX_DEFAULT[p][parts::CHORUS],
            variation: parts::FX_DEFAULT[p][parts::VARIATION],
            octave: 0,
            tone: ToneReg::default(),
            bend_range: crate::controllers::DEFAULT_BEND_RANGE,
            eq: Default::default(),
            insert: Default::default(),
            other: Default::default(),
        };
        Rack {
            name: super::live_rack::NEW_NAME.into(),
            parts: std::array::from_fn(part),
            split: super::Options::default().split,
            harmony_arp: racks::HarmonyArpReg { on: false, ..now.harmony_arp },
            transpose: 0,
            controls: Default::default(),
            ..now
        }
    }

    /// A save, with "Save first": sent while the unsaved-changes prompt is up, the switch it
    /// holds waits for the save and the engine makes it once saved (the app never resends
    /// it). A save that fails (other than asking for sound names) drops the held switch, so
    /// it can't happen later by surprise.
    fn save_rack_holding(&mut self, save_as: Option<String>, names: &BTreeMap<u8, String>) -> Result<(), CmdError> {
        if let Some(RackPrompt::UnsavedChanges { then }) = self.live_rack.prompt.take() {
            self.live_rack.held = Some(then);
        }
        let r = self.save_rack(save_as, names);
        if matches!(r, Err(ref e) if !matches!(e, CmdError::NeedsSoundNames)) {
            self.live_rack.held = None;
        }
        r
    }

    /// `saveRack` (`save_as` None: over the live rack's own rack, or as a new one under its
    /// name if it has none) and `saveRackAs`, with the edited sounds.
    fn save_rack(&mut self, save_as: Option<String>, names: &BTreeMap<u8, String>) -> Result<(), CmdError> {
        let dir = self.racks_dir()?;
        let own = self.live_rack.id.clone().and_then(|id| self.rack_file(&id).map(|p| (id, p)));
        let (id, name, path) = match (&save_as, own) {
            (Some(n), _) => {
                let n = n.trim().to_string();
                if n.is_empty() {
                    return self.fail("a rack needs a name");
                }
                let path = racks::path_for(&dir, &n);
                if path.exists() {
                    return self.fail(format!("there is a rack called {n} already"));
                }
                (racks::new_id(), n, path)
            }
            (None, Some((id, path))) => (id, self.live_rack.name.clone(), path),
            (None, None) => {
                let n = unique_name(&dir, &self.live_rack.name);
                let path = racks::path_for(&dir, &n);
                (racks::new_id(), n, path)
            }
        };
        // The edited sounds: the user's own are saved over; presets need a name first.
        let (mut over, mut new, mut ask) = (Vec::new(), Vec::new(), Vec::new());
        for p in 0..racks::PARTS {
            let Some((tag, true)) = self.channel_sound(parts::CHANNEL[p]) else { continue };
            if self.part_own_sound(p).is_some() {
                over.push(p);
                continue;
            }
            match names.get(&(p as u8)).map(|n| n.trim()).filter(|n| !n.is_empty()) {
                Some(n) => new.push((p, n.to_string())),
                None => ask.push(SoundNameAsk { part: p as u8, suggested: tag.map(|t| t.name).unwrap_or_default() }),
            }
        }
        if !ask.is_empty() {
            self.live_rack.prompt = Some(RackPrompt::SoundNames { parts: ask, save_as });
            return Err(CmdError::NeedsSoundNames);
        }
        for p in over {
            self.save_part_sound(p)?;
        }
        for (p, n) in new {
            self.save_part_sound_as(p, Some(n))?;
        }
        let rack = Rack { id: id.clone(), name: name.clone(), ..self.capture_rack_with(true) };
        self.write_rack(&rack, &path)?;
        self.live_rack_clean(&name, Some(id));
        self.say(format!("Saved {name}"), false);
        // The switch that asked to save first now goes ahead.
        match self.live_rack.held.take() {
            Some(then) => self.switch_rack(then, false),
            None => Ok(()),
        }
    }

    fn rename_rack(&mut self, id: &str, name: &str) -> Result<(), CmdError> {
        let name = name.trim();
        if name.is_empty() {
            return self.fail("a rack needs a name");
        }
        let dir = self.racks_dir()?;
        let Some(old) = self.rack_file(id) else { return self.fail(format!("no rack {id}")) };
        let mut r = self.read_rack(id)?;
        let path = racks::path_for(&dir, name);
        // A name that differs only in case is this rack's own file on a case-insensitive
        // filesystem: a rename of itself, not a taken name.
        let itself = path == old || same_file(&path, &old);
        if !itself && path.exists() {
            return self.fail(format!("there is a rack called {name} already"));
        }
        r.name = name.to_string();
        if path != old && itself {
            // Case only: move the file to its new spelling, then write it there.
            if let Err(e) = std::fs::rename(&old, &path) {
                self.presence.refresh_racks(true);
                return self.fail(format!("The rack was not renamed: {e}"));
            }
            self.write_rack(&r, &path)?;
        } else if path == old {
            self.write_rack(&r, &path)?;
        } else {
            // Exactly one file per id: write the new one, then remove the old; if the old
            // one stays, the new one goes again.
            let moved = replace_file(&old, &path, |p| r.save(p));
            self.presence.refresh_racks(true);
            if let Err(e) = moved {
                return self.fail(format!("The rack was not renamed: {e}"));
            }
        }
        if self.live_rack.id.as_deref() == Some(id) {
            // The loaded rack's new name; its changes (if any) stay unsaved.
            self.live_rack.name = name.to_string();
            self.live_rack_touched(self.clock_ns);
        }
        Ok(())
    }

    /// The hardware (Launchkey, pedals) switches rack: there is no dialog, so unsaved
    /// changes are kept as a rack of the user's, "Recovered: <name>", and the switch goes
    /// ahead. If that rack can't be written, nothing changes.
    pub(super) fn switch_rack_unattended(&mut self, id: Option<&str>) -> Result<(), CmdError> {
        let rack = match id {
            Some(id) => Some(self.read_rack(id)?),
            None => None,
        };
        if self.live_rack.modified {
            let dir = self.racks_dir()?;
            let name = unique_name(&dir, &format!("{RECOVERED}{}", self.live_rack.name));
            // As it plays: edited plugin states stay the parts' edits; no sound is saved.
            let kept = Rack { id: racks::new_id(), name: name.clone(), ..self.capture_rack_with(true) };
            self.write_rack(&kept, &racks::path_for(&dir, &name))?;
        }
        self.enter_rack(rack);
        Ok(())
    }

    /// The user's racks, by name, for the app.
    pub(super) fn rack_entries(&self, plugins: &PluginsState) -> Vec<RackEntry> {
        let attention: HashSet<&str> = plugins.needs_attention.iter().map(|r| r.id.as_str()).collect();
        self.presence
            .racks()
            .iter()
            .map(|r| RackEntry {
                id: r.id.clone(),
                name: r.name.clone(),
                parts: r.sounds.iter().map(|s| self.rack_sound_name(s, plugins)).collect(),
                on: r.on.clone(),
                needs_attention: attention.contains(r.id.as_str()),
            })
            .collect()
    }

    /// A rack part's sound by name: the library's, the SoundFont preset's (a GM voice's),
    /// or the plugin's.
    fn rack_sound_name(&self, s: &SoundRef, plugins: &PluginsState) -> String {
        match s {
            SoundRef::Library { id } => {
                self.sound_patches().iter().find(|p| p.id == *id).map_or_else(|| id.clone(), |p| p.name.clone())
            }
            SoundRef::Font { file, bank, program } => {
                self.font_preset_name(&FontPreset::new(file.clone(), *bank, *program)).unwrap_or_else(|| {
                    if *bank == 0 { crate::api::gm_name(*program).to_string() } else { format!("{file} {bank}:{}", program + 1) }
                })
            }
            SoundRef::Plugin { component } => plugins
                .list
                .iter()
                .map(|p| (&p.id, &p.name))
                .chain(plugins.missing.iter().map(|p| (&p.id, &p.name)))
                .find(|(id, _)| *id == component)
                .map_or_else(|| component.clone(), |(_, n)| n.clone()),
        }
    }
}

/// `a` and `b` are the same existing file (as on a case-insensitive filesystem, where
/// `Ballad` and `ballad` name one file).
fn same_file(a: &std::path::Path, b: &std::path::Path) -> bool {
    match (std::fs::metadata(a), std::fs::metadata(b)) {
        #[cfg(unix)]
        (Ok(x), Ok(y)) => {
            use std::os::unix::fs::MetadataExt;
            (x.dev(), x.ino()) == (y.dev(), y.ino())
        }
        #[cfg(not(unix))]
        (Ok(_), Ok(_)) => a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase(),
        _ => false,
    }
}

/// Moves a file from `old` to `new` by writing `new` (with `write`) and then removing
/// `old`. If `old` can't be removed, `new` is removed again and it's an error, so exactly
/// one of the two is left.
fn replace_file(
    old: &std::path::Path,
    new: &std::path::Path,
    write: impl FnOnce(&std::path::Path) -> anyhow::Result<()>,
) -> Result<(), String> {
    write(new).map_err(|e| format!("{e:#}"))?;
    if let Err(e) = std::fs::remove_file(old) {
        let _ = std::fs::remove_file(new);
        return Err(format!("the old file could not be removed: {e}"));
    }
    Ok(())
}

/// `name`, or `name 2`, `name 3`… : the first no rack file in `dir` has.
fn unique_name(dir: &std::path::Path, name: &str) -> String {
    let name = name.trim();
    let name = if name.is_empty() { super::live_rack::NEW_NAME } else { name };
    (1..)
        .map(|n| if n == 1 { name.to_string() } else { format!("{name} {n}") })
        .find(|n| !racks::path_for(dir, n).exists())
        .expect("a free name")
}

impl Session {
    /// The Launchkey or a pedal loads rack `id` (None: a new rack). With no dialog to ask,
    /// unsaved changes are kept as "Recovered: <name>" in the user's racks and the switch
    /// goes ahead (docs/racks.md, "Saving").
    pub fn load_rack_from_hardware(&self, id: Option<&str>) -> Result<(), CmdError> {
        let mut ctl = self.inner.lock();
        let r = ctl.switch_rack_unattended(id);
        if ctl.offline.is_some() {
            drop(ctl);
            self.settle();
        } else {
            self.inner.publish(&mut ctl, crate::rt::now_ns());
            drop(ctl);
            self.inner.shared.ctl_wake.signal();
        }
        r
    }
}

#[cfg(test)]
#[path = "rack_cmds_tests.rs"]
mod tests;
