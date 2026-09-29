//! Racks (docs/racks.md) and the running session: capture what's under the player's hands
//! into a `racks::Rack`, and apply one back so the parts sound and mix as captured. The
//! model and its file are `crate::racks`.
//!
//! A part's sound is captured by reference: its own library patch, else the library sound
//! its plugin plays (with the plugin's state as `editedState` when it differs from the
//! sound's), else a plugin with no sound (its state kept), else its GM voice as a preset
//! of the main SoundFont. Applying sets the sound first and the mix after it (a sound
//! carries no mix, so the part's level, pan, sends and octave are the rack's). A sound that
//! can't play (a plugin not installed, a sound no longer in the library) is reported and
//! the rest of the rack is still applied, that part's mix included. A part whose plugin
//! isn't installed is silent (not on its GM voice) and keeps the rack's reference, so the
//! plugin plays it again once it is back (session/plugins.rs, `mark_missing`).
//!
//! Each part's channel strip (`Control::strips`) is captured and applied with it: its
//! compressor, both insert slots (insert 1's kind, on/off and amount are the part's
//! `insert`) and its levels to sends 4-6 (sends 1-3 are its `reverb`, `chorus` and
//! `variation`). So are the rack's send effects (`Rack::sends`): the added sends 4-6, and
//! sends 1-3 where the rack overrides the style's (applying a rack without an override
//! gives the send back to the style). A rack saved before the strips plays its keyboard
//! parts flat (compressor off, insert 2 empty, sends 4-6 at 0) with no added sends and no
//! override, and leaves the Style parts' strips as they are.

use super::{Control, Session};
use crate::api::{ChordCmd, LockItem, PartsCmd};
use crate::engine::Transpose;
use crate::parts;
use crate::patches::{PatchSource, SoundId};
use crate::racks::{self, Rack, RackPart, RackSends, SoundRef, StripReg, ToneReg};
use std::sync::atomic::Ordering::Relaxed;

impl Session {
    /// The rack playing now, as `name` (with a new id).
    pub fn capture_rack(&self, name: &str) -> Rack {
        self.inner.lock().capture_rack(name)
    }

    /// Apply `rack`: the keyboard parts' sounds, mix and strips, the send effects, the
    /// split point, Harmony/Arpeggio, the keyboard transpose and the controller map.
    /// Returns what could not be applied (a part whose sound can't play keeps its mix;
    /// everything else still applies).
    pub fn apply_rack(&self, rack: &Rack) -> Vec<String> {
        let mut ctl = self.inner.lock();
        let problems = ctl.apply_rack(rack);
        if ctl.offline.is_some() {
            drop(ctl);
            self.settle();
        } else {
            // Pump and publish now, as `send` does, so `state()` straight after shows the
            // rack, follow-ups included. The control thread publishes again only if the
            // engine's snapshot then changes (the engine wakes it itself).
            let now = crate::rt::now_ns();
            ctl.pump(now);
            self.inner.publish(&mut ctl, now);
        }
        problems
    }
}

impl Control {
    pub(super) fn capture_rack(&self, name: &str) -> Rack {
        Rack { id: racks::new_id(), name: name.to_string(), ..self.capture_rack_with(true) }
    }

    /// The rack playing now, with no id or name. Without `states`, no plugin states are
    /// read or encoded: an edited sound's `edited_state` is an empty string, and a bare
    /// plugin's is none (the live rack's change check, which runs at every pump).
    pub(super) fn capture_rack_with(&self, states: bool) -> Rack {
        let sh = &self.shared;
        Rack {
            format: racks::FORMAT.into(),
            version: racks::VERSION,
            id: String::new(),
            name: String::new(),
            parts: std::array::from_fn(|p| self.capture_rack_part(p, states)),
            split: sh.split.load(Relaxed),
            harmony_arp: self.harmony_arp_reg(),
            transpose: self.transpose.keyboard,
            controls: self.rack_controls.clone(),
            sends: RackSends {
                added: self.strips.borrow().added.clone(),
                override_: std::array::from_fn(|i| self.fx.rack_override[i].then(|| self.style_send_slot(i))),
            },
            other: Default::default(),
        }
    }

    pub(super) fn capture_rack_part(&self, p: usize, states: bool) -> RackPart {
        let kp = &self.shared.parts;
        let ch = parts::CHANNEL[p];
        let program = kp.program[p].load(Relaxed) & 127;
        // An edited sound's state: the plugin's, or (no states) a marker that it is edited.
        let edit = |edited: bool, state: Option<String>| if states { state.filter(|_| edited) } else { edited.then(String::new) };
        let (sound, edited_state, fallback_program) = if let Some((id, _)) = self.part_patch(p) {
            // Its own library patch; a plugin patch whose plugin was edited keeps the edit.
            let edited = self.channel_sound(ch).is_some_and(|(_, e)| e);
            let state = self.part_plugin_voice_with(p, states).and_then(|(_, s)| s);
            (SoundRef::Library { id }, edit(edited, state), Some(program))
        } else if let Some((component, state)) = self.part_plugin_voice_with(p, states) {
            let (tag, edited) = self.channel_sound(ch).unwrap_or_default();
            match tag.and_then(|t| SoundId::parse(&t.id)) {
                Some(SoundId::Library(id)) => (SoundRef::Library { id }, edit(edited, state), Some(program)),
                _ => (SoundRef::Plugin { component }, state, Some(program)),
            }
        } else {
            (SoundRef::Font { file: self.sf_file.clone().unwrap_or_default(), bank: 0, program }, None, None)
        };
        let fx = kp.fx(p);
        let strip = {
            let strips = self.strips.borrow();
            let cell = &strips.strips[p];
            let n = strips.sends();
            let mut sends = [0; racks::SENDS];
            sends[..racks::STYLE_SENDS].copy_from_slice(&[fx[parts::REVERB], fx[parts::CHORUS], fx[parts::VARIATION]]);
            for (i, s) in sends.iter_mut().enumerate().take(n).skip(racks::STYLE_SENDS) {
                *s = cell.sends[i];
            }
            StripReg {
                comp: Some(cell.comp).filter(|c| !c.is_default()),
                inserts: [self.keyboard_insert1(p), cell.inserts[1].clone()],
                sends,
            }
        };
        RackPart {
            on: kp.is_on(p),
            sound,
            edited_state,
            fallback_program,
            volume: kp.volume(p),
            pan: fx[parts::PAN],
            reverb: fx[parts::REVERB],
            chorus: fx[parts::CHORUS],
            variation: fx[parts::VARIATION],
            octave: kp.octave[p].load(Relaxed).clamp(-2, 2),
            tone: ToneReg::capture(kp, p),
            bend_range: self.shared.controllers.bend_range(p),
            eq: kp.eq(p),
            insert: kp.insert(p),
            strip,
            other: Default::default(),
        }
    }

    pub(super) fn apply_rack(&mut self, r: &Rack) -> Vec<String> {
        let mut problems = Vec::new();
        for (p, part) in r.parts.iter().enumerate() {
            // `live-rack.json` embeds a rack read without `Rack::from_json`: normalize here.
            let mut part = part.clone();
            part.normalize();
            let part = &part;
            match self.apply_rack_sound(p, part) {
                Err(e) => problems.push(e),
                // A library sound whose plugin isn't installed: the part is silent.
                Ok(()) if self.channel_plugin_state(parts::CHANNEL[p]).is_some_and(|s| s.missing) => {
                    let name = self.channel_plugin_state(parts::CHANNEL[p]).map(|s| s.name).unwrap_or_default();
                    problems.push(format!("{}: {name} is not installed; the part is silent until it is back", parts::NAMES[p]));
                }
                Ok(()) => {}
            }
            if let Err(e) = self.apply_rack_mix(p, part) {
                problems.push(e);
            }
            self.apply_rack_strip(p, &part.strip);
        }
        problems.extend(self.apply_rack_sends(&r.sends));
        // The engine sends the new volumes, pans and sends (CCs) on its next wake.
        self.wake_engine();
        if !self.param_locked(LockItem::SplitPoint)
            && let Err(e) = self.chord_cmd(ChordCmd::SetSplit { note: r.split.min(127) })
        {
            problems.push(e.to_string());
        }
        if let Err(e) = self.apply_harmony_arp_reg(&r.harmony_arp) {
            problems.push(e);
        }
        if let Err(e) = self.set_transpose(Transpose::new(r.transpose, self.transpose.master)) {
            problems.push(e.to_string());
        }
        self.set_rack_controls(r.controls.clone());
        problems
    }

    /// What part `p` plays. Err: it can't (it plays its GM voice, or its plugin shows as
    /// failed).
    fn apply_rack_sound(&mut self, p: usize, part: &RackPart) -> Result<(), String> {
        let kp = self.shared.parts.clone();
        if let Some(program) = part.fallback_program {
            kp.set_program(p, program);
        }
        match &part.sound {
            SoundRef::Font { file, bank: 0, program } if self.is_gm_font(file) => {
                kp.set_program(p, *program);
                self.clear_part_tab_plugin(p);
                self.recall_part_patch(p, None)
            }
            SoundRef::Font { file, bank, program } => {
                let id = SoundId::Font(crate::patches::FontPreset::new(file.clone(), *bank, *program)).to_string();
                match self.patch_for_sound(&id) {
                    Ok(patch) => {
                        self.clear_part_tab_plugin(p);
                        self.recall_part_patch(p, Some((&patch, &patch)))
                    }
                    Err(e) => Err(format!("{}: {file} {bank}:{program} can't play ({e})", parts::NAMES[p])),
                }
            }
            SoundRef::Library { id } => {
                let patch = self.sound_patches().iter().find(|q| q.id == *id).map(|q| (q.source.clone(), q.name.clone(), q.tag()));
                match (patch, part.edited_state.as_deref()) {
                    // An edited plugin sound: the plugin with the edit, still named as the sound.
                    (Some((PatchSource::Plugin { component_id, .. }, name, tag)), Some(state)) => {
                        self.recall_part_plugin(p, &component_id, &name, Some(state), Some(&tag)).map(|_| ())
                    }
                    (Some((_, name, _)), _) => {
                        self.clear_part_tab_plugin(p);
                        self.recall_part_patch(p, Some((id, &name)))
                    }
                    (None, _) => {
                        self.clear_part_tab_plugin(p);
                        self.recall_part_patch(p, Some((id, id)))
                    }
                }
            }
            SoundRef::Plugin { component } => {
                self.recall_part_plugin(p, component, "", part.edited_state.as_deref(), None).map(|_| ())
            }
        }
    }

    /// A bank 0 preset of `file` is a GM voice: `file` is the main SoundFont, or one that
    /// isn't in the folder any more (the main font plays it instead).
    fn is_gm_font(&self, file: &str) -> bool {
        self.sf_file.as_deref().unwrap_or_default() == file || !self.sound_fonts.iter().any(|f| f == file)
    }

    /// Part `p`'s mix, after its sound: the rack's level, octave, pan and sends, and its
    /// voice settings over the sound's neutral ones.
    pub(super) fn apply_rack_mix(&mut self, p: usize, part: &RackPart) -> Result<(), String> {
        let kp = self.shared.parts.clone();
        kp.set_volume(p, part.volume.min(127));
        kp.octave[p].store(part.octave.clamp(-2, 2), Relaxed);
        kp.set_fx(p, [part.pan, part.reverb, part.chorus, part.variation].map(Some));
        kp.voice_changed(p);
        kp.set_tone(p, part.tone.controllers(), part.tone.xg.iter().map(|x| (x[0], x[1], x[2])));
        // The rack's EQ, not the XG part EQ in its voice settings (#247): a rack saved
        // before the part EQ plays flat, as it did.
        kp.set_eq(p, part.eq);
        // Its insert slot: a rack saved before it plays with none, as it did.
        kp.set_insert(p, part.insert);
        self.shared.controllers.set_bend_range(p, part.bend_range);
        // Left plays the bass under Manual Bass: its switch stays as it is.
        let locked_left = p == parts::LEFT && self.shared.manual_bass();
        if kp.is_on(p) != part.on && !locked_left {
            return self.parts_cmd(PartsCmd::SetPartOn { part: p as u8, on: part.on }).map_err(|e| e.to_string());
        }
        Ok(())
    }

    /// Part `p`'s channel strip beyond what `apply_rack_mix` sets, from a normalized
    /// `strip`: its compressor (none: off), insert 1's settings past the first (its kind,
    /// on/off and amount are the part's `insert`, already set), insert 2 and its levels to
    /// sends 4-6. A rack saved before the strip plays flat: compressor off, insert 2
    /// empty, sends 4-6 at 0.
    fn apply_rack_strip(&mut self, p: usize, strip: &StripReg) {
        let cell = &mut self.strips.get_mut().strips[p];
        cell.comp = strip.comp.unwrap_or_default();
        cell.inserts = strip.inserts.clone();
        cell.sends[racks::STYLE_SENDS..].copy_from_slice(&strip.sends[racks::STYLE_SENDS..]);
    }

    /// The rack's send effects: sends 4-6 as it adds them (every keyboard strip's level to
    /// a send it hasn't goes to 0; the Style strips keep theirs), and sends 1-3 as it overrides them, or the style's where it
    /// doesn't. Returns what could not be applied.
    fn apply_rack_sends(&mut self, sends: &RackSends) -> Vec<String> {
        let mut problems = Vec::new();
        let strips = self.strips.get_mut();
        strips.added = sends.added.iter().take(racks::ADDED_SENDS).cloned().collect();
        let n = strips.sends();
        // A rack carries only the keyboard strips: the Style parts' levels stay.
        for s in &mut strips.strips[..crate::api::KEYBOARD_STRIPS] {
            s.sends[n..].fill(0);
        }
        for (i, o) in sends.override_.iter().enumerate() {
            match o {
                Some(slot) => {
                    if let Err(e) = self.set_style_send(i, slot) {
                        problems.push(e);
                    }
                    self.set_send_override(i, true);
                }
                None => self.set_send_override(i, false),
            }
        }
        problems
    }
}

#[cfg(test)]
#[path = "racks_tests.rs"]
mod tests;
