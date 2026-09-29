//! The rack commands in the dev mock (docs/app-api.md › Racks), kept in memory: new, load,
//! save, save as, revert, rename, duplicate, delete, the unsaved-changes guard and the
//! sound-names prompt, as the session does them. As mock-racks.ts.

use super::MockSession;
use yahaha::api::*;

/// What a mock rack holds: each part's switch, voice, own patch and mix, the split, the
/// keyboard transpose.
#[derive(Clone, Debug)]
pub(super) struct MockRack {
    id: String,
    name: String,
    parts: Vec<RackPartMix>,
    names: Vec<String>,
    split: u8,
    transpose: i8,
}

#[derive(Clone, Debug)]
struct RackPartMix {
    on: bool,
    program: u8,
    patch: Option<String>,
    volume: u8,
    octave: i8,
    pan: u8,
    reverb: u8,
    chorus: u8,
    variation: u8,
}

#[derive(Default)]
pub(super) struct MockRacks {
    racks: Vec<MockRack>,
    seq: u32,
    /// A rack was just loaded or saved: the next `bump` takes what plays as unmodified.
    pub(super) clean: bool,
}

const NEW_NAME: &str = "New rack";
const DEFAULT_PROGRAMS: [u8; 4] = [0, 48, 61, 48];

impl MockRacks {
    /// The racks list, by name.
    pub(super) fn entries(&self) -> Vec<RackEntry> {
        let mut v: Vec<RackEntry> = self
            .racks
            .iter()
            .map(|r| RackEntry {
                id: r.id.clone(),
                name: r.name.clone(),
                parts: r.names.clone(),
                on: r.parts.iter().map(|p| p.on).collect(),
                needs_attention: false,
            })
            .collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }

    fn find(&self, id: &str) -> Option<&MockRack> {
        self.racks.iter().find(|r| r.id == id)
    }

    fn taken(&self, name: &str) -> bool {
        self.racks.iter().any(|r| r.name == name)
    }

    fn unique(&self, name: &str) -> String {
        let base = if name.trim().is_empty() { NEW_NAME } else { name.trim() };
        (1..).map(|n| if n == 1 { base.to_string() } else { format!("{base} {n}") }).find(|n| !self.taken(n)).expect("a free name")
    }

    fn new_id(&mut self) -> String {
        self.seq += 1;
        format!("r-mock-{}", self.seq)
    }
}

impl MockSession {
    pub(super) fn rack_cmd(&mut self, c: RackCmd) {
        match c {
            RackCmd::NewRack { discard } => self.switch_rack(None, discard),
            RackCmd::LoadRack { id, discard } => match self.racks.find(&id).cloned() {
                Some(r) => self.switch_rack(Some(r), discard),
                None => self.message(format!("no rack {id}"), true),
            },
            RackCmd::SaveRack { sound_names } => self.save_rack(None, &sound_names),
            RackCmd::SaveRackAs { name, sound_names } => self.save_rack(Some(name), &sound_names),
            RackCmd::RevertRack => match self.state.live_rack.id.clone().and_then(|id| self.racks.find(&id).cloned()) {
                Some(r) => self.enter_rack(Some(r)),
                None => self.message(format!("{} has no saved rack to go back to", self.state.live_rack.name), true),
            },
            RackCmd::RenameRack { id, name } => {
                let name = name.trim().to_string();
                let Some(old) = self.racks.find(&id).map(|r| r.name.clone()) else { return self.message(format!("no rack {id}"), true) };
                if name.is_empty() {
                    return self.message("a rack needs a name", true);
                }
                if name != old && self.racks.taken(&name) {
                    return self.message(format!("there is a rack called {name} already"), true);
                }
                if let Some(r) = self.racks.racks.iter_mut().find(|r| r.id == id) {
                    r.name = name.clone();
                }
                if self.state.live_rack.id.as_deref() == Some(id.as_str()) {
                    self.state.live_rack.name = name;
                }
            }
            RackCmd::DuplicateRack { id } => {
                let Some(mut copy) = self.racks.find(&id).cloned() else { return self.message(format!("no rack {id}"), true) };
                copy.name = self.racks.unique(&format!("{} copy", copy.name));
                copy.id = self.racks.new_id();
                self.message(format!("Duplicated as {}", copy.name), false);
                self.racks.racks.push(copy);
            }
            RackCmd::DeleteRack { id } => {
                if self.racks.find(&id).is_none() {
                    return self.message(format!("no rack {id}"), true);
                }
                if self.state.live_rack.id.as_deref() == Some(id.as_str()) {
                    return self.message(format!("{} is loaded: load another rack before deleting it", self.state.live_rack.name), true);
                }
                self.racks.racks.retain(|r| r.id != id);
            }
            RackCmd::DismissRackPrompt => self.state.live_rack.prompt = None,
        }
    }

    fn switch_rack(&mut self, rack: Option<MockRack>, discard: bool) {
        if self.state.live_rack.modified && !discard {
            let then = match &rack {
                Some(r) => RackSwitch::Load { id: r.id.clone(), name: r.name.clone() },
                None => RackSwitch::New,
            };
            self.state.live_rack.prompt = Some(RackPrompt::UnsavedChanges { then });
            return;
        }
        self.enter_rack(rack);
    }

    /// The live rack becomes `rack` (None: a new one), unmodified.
    fn enter_rack(&mut self, rack: Option<MockRack>) {
        for p in 0..4 {
            let mix = rack.as_ref().map(|r| r.parts[p].clone()).unwrap_or(RackPartMix {
                on: p == 0,
                program: DEFAULT_PROGRAMS[p],
                patch: None,
                volume: 100,
                octave: 0,
                pan: 64,
                reverb: 0,
                chorus: 0,
                variation: 0,
            });
            self.cmd(SoundLibraryCmd::SetPartPatch { part: p as u8, id: mix.patch.clone() }.into());
            let k = &mut self.state.keyboard_parts[p];
            (k.on, k.program, k.volume, k.octave, k.pan, k.reverb, k.chorus, k.variation) =
                (mix.on, mix.program, mix.volume, mix.octave, mix.pan, mix.reverb, mix.chorus, mix.variation);
        }
        self.state.chord.split = rack.as_ref().map_or(54, |r| r.split);
        self.state.chord.transpose_keyboard = rack.as_ref().map_or(0, |r| r.transpose);
        let name = rack.as_ref().map_or_else(|| NEW_NAME.to_string(), |r| r.name.clone());
        self.state.live_rack = LiveRackState { name: name.clone(), id: rack.map(|r| r.id), modified: false, prompt: None };
        self.racks.clean = true;
        self.message(format!("Loaded {name}"), false);
    }

    fn save_rack(&mut self, save_as: Option<String>, names: &std::collections::BTreeMap<u8, String>) {
        let own = self.state.live_rack.id.clone().and_then(|id| self.racks.find(&id).map(|r| (r.id.clone(), r.name.clone())));
        let (id, name) = match (&save_as, own) {
            (Some(n), _) => {
                let n = n.trim().to_string();
                if n.is_empty() {
                    return self.message("a rack needs a name", true);
                }
                if self.racks.taken(&n) {
                    return self.message(format!("there is a rack called {n} already"), true);
                }
                (self.racks.new_id(), n)
            }
            (None, Some(own)) => own,
            (None, None) => {
                let n = self.racks.unique(&self.state.live_rack.name);
                (self.racks.new_id(), n)
            }
        };
        // Edited sounds: the user's own saved over, presets need a name.
        let (mut saves, mut ask) = (Vec::new(), Vec::new());
        for (p, k) in self.state.keyboard_parts.iter().enumerate() {
            if !k.sound_edited {
                continue;
            }
            let part = p as u8;
            if k.sound.as_ref().is_some_and(|s| s.id.starts_with("saved:")) {
                saves.push(SoundLibraryCmd::SaveSound { part });
                continue;
            }
            match names.get(&part).map(|n| n.trim()).filter(|n| !n.is_empty()) {
                Some(n) => saves.push(SoundLibraryCmd::SaveSoundAs { part, name: Some(n.to_string()) }),
                None => ask.push(SoundNameAsk { part, suggested: k.sound.as_ref().map_or_else(|| k.voice_name.clone(), |s| s.name.clone()) }),
            }
        }
        if !ask.is_empty() {
            self.state.live_rack.prompt = Some(RackPrompt::SoundNames { parts: ask, save_as: save_as.map(|_| name) });
            return;
        }
        for c in saves {
            self.cmd(c.into());
        }
        let s = &self.state;
        let rack = MockRack {
            id: id.clone(),
            name: name.clone(),
            parts: s
                .keyboard_parts
                .iter()
                .map(|k| RackPartMix {
                    on: k.on,
                    program: k.program,
                    patch: k.patch.clone(),
                    volume: k.volume,
                    octave: k.octave,
                    pan: k.pan,
                    reverb: k.reverb,
                    chorus: k.chorus,
                    variation: k.variation,
                })
                .collect(),
            names: s.keyboard_parts.iter().map(|k| k.sound.as_ref().map_or_else(|| k.voice_name.clone(), |t| t.name.clone())).collect(),
            split: s.chord.split,
            transpose: s.chord.transpose_keyboard,
        };
        self.racks.racks.retain(|r| r.id != id);
        self.racks.racks.push(rack);
        self.state.live_rack = LiveRackState { name: name.clone(), id: Some(id), modified: false, prompt: None };
        self.racks.clean = true;
        self.message(format!("Saved {name}"), false);
    }
}
