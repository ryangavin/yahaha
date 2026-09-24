//! Chord following: a new chord re-voices the notes sounding (the Retrigger Rules) and
//! brings in parts it lets play; Stop Accompaniment; transpose.

use super::*;

/// What a chord change does to one sounding note (`Engine::revoice_part`). Pitches are
/// what sounds (key sent plus the channel's `rtr_bend`).
#[derive(Clone, Copy, PartialEq)]
enum Revoice {
    /// Untouched: its pattern note-off is due right now.
    Leave,
    /// Keeps sounding at its pitch.
    Hold,
    Cut,
    /// Bend to this pitch, no new attack (Pitch Shift, Pitch Shift to Root).
    Shift(u8),
    /// New attack at this pitch.
    Retrigger(u8),
}

impl Engine {
    // ----- input -----

    /// A chord as fingered (before Keyboard transpose). Starts playback when sync start is armed.
    pub fn set_chord(&mut self, played: Chord, now: u64, sink: &mut impl Sink) {
        self.played = Some(played);
        let chord = shift_chord(played, self.transpose.keyboard);
        let prev = self.chord;
        self.chord = Some(chord);
        self.follow_chord(prev, chord, now, sink);
        self.on_chord(prev, now, sink);
    }

    /// The band follows a new chord: a Sync Start chord starts it, a playing band
    /// re-voices, a stopped one with Stop ACMP on sounds it.
    fn follow_chord(&mut self, prev: Option<Chord>, chord: Chord, now: u64, sink: &mut impl Sink) {
        if self.sync_armed && !self.running && chord.ty != CANCEL {
            self.start(now, sink);
            return;
        }
        if self.running {
            if prev.is_some() {
                self.revoice(chord, now, sink);
            }
            self.catch_up(prev, chord, now, sink);
        }
        if !self.running && self.stop_acmp != StopAcmp::Off {
            self.sound_stop_acmp(chord, now, sink);
        }
    }

    /// New transpose settings. They apply to notes started from now on; sounding notes keep
    /// their pitch until they end or the next chord change revoices them. A Keyboard change
    /// moves the held chord at once, so the band follows as if the same keys had been played
    /// in the new key. Stop Accompaniment notes move only if they are still sounding.
    pub fn set_transpose(&mut self, t: Transpose, now: u64, sink: &mut impl Sink) {
        let old = self.transpose;
        self.transpose = Transpose::new(t.keyboard, t.master);
        if self.transpose.keyboard == old.keyboard {
            return;
        }
        let Some(played) = self.played else { return };
        let chord = shift_chord(played, self.transpose.keyboard);
        let prev = self.chord;
        self.chord = Some(chord);
        if self.running {
            self.revoice(chord, now, sink);
            self.catch_up(prev, chord, now, sink);
        } else if self.stop_acmp != StopAcmp::Off && self.sounding.iter().any(|n| n.active && n.src == STOP_ACMP_SRC) {
            self.sound_stop_acmp(chord, now, sink);
        }
        self.on_chord(prev, now, sink);
    }

    /// Master transpose for a note on `dest` (never on drum/SFX kits).
    #[inline]
    pub(super) fn master(&self, dest: u8, key: u8) -> u8 {
        if self.style.kit[dest as usize & 15] {
            key
        } else {
            shift_key(key, self.transpose.master)
        }
    }

    /// Stop Accompaniment: with the band stopped, the held chord sounds on the Bass (root /
    /// on-bass note) and Pad (chord tones) channels, with the style's voices (Style) or
    /// fixed ones (Fixed, `stop_acmp_voices`).
    pub(super) fn sound_stop_acmp(&mut self, chord: Chord, now: u64, sink: &mut impl Sink) {
        self.off_where(sink, |n| n.src == STOP_ACMP_SRC);
        if chord.ty == CANCEL {
            return;
        }
        self.stop_acmp_voices(sink);
        let bass = 36 + chord.bass.unwrap_or(chord.root);
        self.note_on(STOP_ACMP_SRC, 0, 10, bass, 90, 4, now, sink);
        for (i, &t) in crate::theory::chord_tones(chord.ty).iter().enumerate() {
            let pc = (chord.root + t) % 12;
            let key = 55 + (pc + 12 - 7) % 12; // G3..F#4
            self.note_on(STOP_ACMP_SRC, 1 + i as u8, 13, key, 70, 4, now, sink);
        }
    }

    /// Pattern events due at `now`, or within `EARLY_CHORD_NS` of it, that `process` has
    /// not played yet: they run from `ev_idx` to the index returned. None when the section
    /// itself ends by then: the all-off at its boundary cuts every note.
    pub(super) fn due_now(&self, now: u64) -> Option<usize> {
        self.due_within(now, EARLY_CHORD_NS)
    }

    /// `due_now` with a window of `window` ns.
    pub(super) fn due_within(&self, now: u64, window: u64) -> Option<usize> {
        let sec = self.style.sections[self.cur].as_ref()?;
        let target = self.tick_at(now + window) + 1e-6;
        let sec_end = self.sec_start + sec.len as f64;
        let (boundary, inclusive, _) = self.boundary(sec_end);
        if boundary <= target {
            return None;
        }
        let mut k = self.ev_idx;
        while let Some(e) = sec.events.get(k) {
            let t = self.sec_start + e.tick as f64;
            let before = if inclusive { t <= boundary + 1e-6 } else { t < boundary - 1e-6 };
            if !before || t > target {
                break;
            }
            k += 1;
        }
        Some(k)
    }

    /// The pattern releases source key `key` on `src` among the events due now (`due`
    /// from `due_now`): a note started or retriggered for it now would be a blip.
    pub(super) fn ends_now(&self, src: u8, key: u8, due: usize) -> bool {
        let Some(sec) = self.style.sections[self.cur].as_ref() else { return true };
        let due = due.min(sec.events.len());
        sec.events[self.ev_idx.min(due)..due]
            .iter()
            .any(|e| e.src == src && matches!(e.kind, PKind::Off { key: k } if k == key))
    }

    /// The pattern strikes `pitch` on part `dest` under `chord` among the events due now
    /// (`due` from `due_now`): a note started or retriggered at that pitch now would be cut
    /// short by that attack.
    pub(super) fn struck_now(&self, dest: u8, pitch: u8, chord: Chord, due: usize) -> bool {
        let Some(sec) = self.style.sections[self.cur].as_ref() else { return false };
        if self.parts & (1 << (dest.saturating_sub(8) & 7)) == 0 {
            return false;
        }
        let due = due.min(sec.events.len());
        let mut i = self.ev_idx.min(due);
        while i < due {
            let e = sec.events[i];
            // Group simultaneous note-ons on this source channel, as `emit_at_index` does.
            let mut keys = [0u8; 8];
            let mut n = 0;
            let mut j = i;
            while let Some(g) = sec.events.get(j) {
                match g.kind {
                    PKind::On { key, .. } if g.tick == e.tick && g.src == e.src && n < 8 => {
                        keys[n] = key;
                        n += 1;
                        j += 1;
                    }
                    _ => break,
                }
            }
            i = j.max(i + 1);
            if n == 0 {
                continue;
            }
            let Some(rule) = sec.rules[e.src as usize].as_ref().filter(|r| r.dest_ch == dest) else { continue };
            let Some(c) = effective_chord(Some(chord), rule).filter(|&c| plays(rule, c)) else { continue };
            let mut outs = [None; 8];
            transpose_group(&keys[..n], rule, c, &mut outs[..n]);
            if outs[..n].iter().flatten().any(|&o| self.master(dest, o) == pitch) {
                return true;
            }
        }
        false
    }

    /// A chord that lands just after the beat (within `LATE_CHORD_NS`) also brings in
    /// the parts the previous chord kept silent (no chord yet, Chord Cancel, or CASM
    /// chord-mute routing). Their notes from that window were skipped, so `revoice` has
    /// nothing to correct; start the ones the pattern still holds now.
    pub(super) fn catch_up(&mut self, prev: Option<Chord>, chord: Chord, now: u64, sink: &mut impl Sink) {
        let Some(sec) = self.style.sections[self.cur].as_ref() else { return };
        // Never reach back past where this section came in: those notes never played.
        let lo = (self.tick_at(now.saturating_sub(LATE_CHORD_NS)) - self.sec_start).max(self.entry);
        let end = self.ev_idx.min(sec.events.len());
        let mut i = end;
        while i > 0 && sec.events[i - 1].tick as f64 >= lo {
            i -= 1;
        }
        // (src, src key, dest, out, vel): room for 8 parts x 8 notes; beyond that the
        // rest are dropped rather than allocating.
        let mut buf = [(0u8, 0u8, 0u8, 0u8, 0u8); 64];
        let mut n_buf = 0;
        while i < end {
            let e = sec.events[i];
            // Group simultaneous note-ons on this source channel, as `emit_at_index` does.
            let mut keys = [0u8; 8];
            let mut vels = [0u8; 8];
            let mut n = 0;
            let mut j = i;
            while let Some(g) = sec.events[..end].get(j) {
                match g.kind {
                    PKind::On { key, vel } if g.tick == e.tick && g.src == e.src && n < 8 => {
                        keys[n] = key;
                        vels[n] = vel;
                        n += 1;
                        j += 1;
                    }
                    _ => break,
                }
            }
            i = j.max(i + 1);
            let Some(rule) = sec.rules[e.src as usize].as_ref() else { continue };
            let part_on = self.parts & (1 << (rule.dest_ch.saturating_sub(8) & 7)) != 0;
            let was = effective_chord(prev, rule).filter(|&c| plays(rule, c));
            let Some(now_chord) = effective_chord(Some(chord), rule).filter(|&c| plays(rule, c)) else { continue };
            // A part that was playing has its notes re-voiced by `revoice`, except guitar
            // strings the previous chord left out (muted by Stroke, or voiced nowhere):
            // those have no voice to re-pitch, so they come in here.
            let guitar = (0..n).any(|k| rule.zone_for(keys[k]).ntr == Ntr::Guitar);
            if n == 0 || !part_on || was.is_some() && !guitar {
                continue;
            }
            let mut outs = [None; 8];
            transpose_group(&keys[..n], rule, now_chord, &mut outs[..n]);
            if was.is_some() {
                let slot = self.cur as u8;
                for k in 0..n {
                    let voiced = rule.zone_for(keys[k]).ntr == Ntr::Guitar
                        && self.sounding.iter().any(|s| s.active && s.src == e.src && s.slot == slot && s.src_key == keys[k]);
                    if voiced || rule.zone_for(keys[k]).ntr != Ntr::Guitar {
                        outs[k] = None;
                    }
                }
            }
            // A note is started only if no more of it is lost than is still to come: not if
            // the section boundary, its own note-off or a new attack on its key ends it
            // sooner than it should have started ago.
            let missed = now.saturating_sub(self.ns_at(self.sec_start + e.tick as f64));
            let Some(due) = self.due_within(now, missed) else { continue };
            for k in 0..n {
                let released = sec.events[j..end]
                    .iter()
                    .any(|o| o.src == e.src && matches!(o.kind, PKind::Off { key } if key == keys[k]))
                    || self.ends_now(e.src, keys[k], due)
                    || outs[k].is_some_and(|o| self.struck_now(rule.dest_ch, self.master(rule.dest_ch, o), chord, due));
                if let (Some(out), false, true) = (outs[k], released, n_buf < buf.len()) {
                    buf[n_buf] = (e.src, keys[k], rule.dest_ch, out, vels[k]);
                    n_buf += 1;
                }
            }
        }
        let slot = self.cur as u8;
        for &(src, key, dest, out, vel) in &buf[..n_buf] {
            self.note_on(src, key, dest, out, vel, slot, now, sink);
        }
    }

    /// Re-pitch sounding notes after a chord change according to each part's retrigger rule.
    pub(super) fn revoice(&mut self, chord: Chord, now: u64, sink: &mut impl Sink) {
        // At a section boundary the all-off cuts every note now: nothing to re-pitch.
        let Some(due) = self.due_now(now) else { return };
        for dest in 8..16u8 {
            if follows_chords(dest) {
                self.revoice_part(dest, chord, now, due, sink);
            }
        }
    }

    /// Revoice the notes sounding on part `dest` (RM p.31, RTR). Pitch Shift bends a note
    /// with no new attack, but pitch bend is per channel, so every note on the part moves
    /// by the same bend: the one that serves the most of the part's continuing notes (a
    /// note that keeps its pitch counts for no change). A Pitch Shift note that needs a
    /// different shift is retriggered at its new pitch instead, and so is a held note the
    /// bend would detune. Two voices that land on one key sound once (a second note-on for
    /// a sounding key would be cut by the first note-off); the second is kept muted, so the
    /// next chord can part them again. A note whose pattern note-off is
    /// due now or within `EARLY_CHORD_NS` is never attacked again: where it would be
    /// retriggered it plays out as it is, or stops if the part's bend moves.
    pub(super) fn revoice_part(&mut self, dest: u8, chord: Chord, now: u64, due: usize, sink: &mut impl Sink) {
        let bend = self.rtr_bend[dest as usize & 15] as i16;
        // The part's sounding notes (indices into `sounding`) and what happens to each.
        let mut notes = [0u8; MAX_SOUNDING];
        let mut plan = [Revoice::Leave; MAX_SOUNDING];
        // Its pattern note-off is due now (`due_now`).
        let mut ends = [false; MAX_SOUNDING];
        let mut m = 0;
        for (i, s) in self.sounding.iter().enumerate() {
            if s.active && s.dest == dest {
                notes[m] = i as u8;
                plan[m] = Revoice::Hold;
                m += 1;
            }
        }
        let mut done = [false; MAX_SOUNDING];
        for a in 0..m {
            let s = self.sounding[notes[a] as usize];
            if done[a] || s.src >= 16 {
                continue;
            }
            let Some(sec) = self.style.sections[s.slot as usize].as_ref() else { continue };
            let Some(rule) = sec.rules[s.src as usize].as_ref() else { continue };
            // Group notes that started together on this channel so Root Fixed voicings move as a unit.
            let mut grp = [0usize; 8];
            let mut keys = [0u8; 8];
            let mut n = 0;
            for b in a..m {
                let o = self.sounding[notes[b] as usize];
                if !done[b] && o.src == s.src && o.slot == s.slot && o.started_ns == s.started_ns && n < 8 {
                    done[b] = true;
                    grp[n] = b;
                    keys[n] = o.src_key;
                    n += 1;
                }
            }
            let chord = effective_chord(Some(chord), rule).filter(|&c| plays(rule, c));
            let mut outs = [None; 8];
            if let Some(c) = chord {
                transpose_group(&keys[..n], rule, c, &mut outs[..n]);
            }
            // The player's chord landed just after these notes started: correct them outright.
            let late = now.saturating_sub(s.started_ns) < LATE_CHORD_NS;
            for k in 0..n {
                let b = grp[k];
                let o = self.sounding[notes[b] as usize];
                ends[b] = o.slot as usize == self.cur && self.ends_now(o.src, o.src_key, due);
                let Some(c) = chord else {
                    plan[b] = Revoice::Cut;
                    continue;
                };
                let cur = o.out as i16 + bend;
                let to = |t: Option<u8>| t.map(|t| self.master(dest, t));
                let zone = rule.zone_for(o.src_key);
                // A guitar noise key is not a pitch: no chord moves it, and it has no say
                // in the part's bend.
                if zone.ntr == Ntr::Guitar && o.src_key >= GUITAR_NOISE {
                    plan[b] = Revoice::Leave;
                    continue;
                }
                // Nearest note, up or down, with the pitch class of the new root (the slash
                // bass on a Bass On channel, as `theory::transpose` has it), in the same
                // octave or the next.
                let to_root = || {
                    let bass_on = zone.bass_on || zone.ntt == Ntt::Bass;
                    let pc = self.master(dest, if bass_on { c.bass.unwrap_or(c.root) } else { c.root }) as i16;
                    let mut d = (pc - cur).rem_euclid(12);
                    if d > 6 {
                        d -= 12;
                    }
                    (cur + d).clamp(0, 127) as u8
                };
                let target = match (late, zone.rtr) {
                    (true, _) => to(outs[k]).map(Revoice::Retrigger),
                    (false, Rtr::Stop) => None,
                    (false, Rtr::PitchShift) => to(outs[k]).map(Revoice::Shift),
                    (false, Rtr::PitchShiftToRoot) => Some(Revoice::Shift(to_root())),
                    (false, Rtr::Retrigger | Rtr::NoteGenerator) => to(outs[k]).map(Revoice::Retrigger),
                    (false, Rtr::RetriggerToRoot) => Some(Revoice::Retrigger(to_root())),
                };
                plan[b] = match target {
                    None => Revoice::Cut,
                    Some(Revoice::Shift(p) | Revoice::Retrigger(p)) if p as i16 == cur => Revoice::Hold,
                    Some(t) => t,
                };
            }
        }

        // The shift the bend makes: the one most continuing notes need, within what the
        // output range leaves over the pattern's own widest bend. (A note about to end has
        // no say, and a muted voice none beside its sounding twin wanting the same shift:
        // one key, one vote.)
        let room = self.style.shift_room[dest as usize & 15] as i16;
        let want = |a: usize, plan: &[Revoice]| match plan[a] {
            _ if ends[a] => None,
            Revoice::Hold => Some(0),
            Revoice::Shift(p) => Some(p as i16 - (self.sounding[notes[a] as usize].out as i16 + bend)),
            _ => None,
        };
        let shift_of = |a: usize, plan: &[Revoice]| {
            let d = want(a, plan)?;
            let s = self.sounding[notes[a] as usize];
            let twin = |b: usize| {
                let o = self.sounding[notes[b] as usize];
                !o.muted && o.out == s.out && want(b, plan) == Some(d)
            };
            (!s.muted || !(0..m).any(twin)).then_some(d)
        };
        let mut continuing = false;
        let mut best: Option<(usize, i16)> = None;
        for a in 0..m {
            let Some(d) = shift_of(a, &plan) else { continue };
            continuing = true;
            if (bend + d).abs() > room {
                continue;
            }
            let votes = (0..m).filter(|&b| shift_of(b, &plan) == Some(d)).count();
            // Ties: the smaller shift, then upwards.
            let better = best.is_none_or(|(v, bd)| (votes, -d.abs(), d) > (v, -bd.abs(), bd));
            if better {
                best = Some((votes, d));
            }
        }
        // With nothing sounding on through the change, the bend stays until the part falls
        // silent (see `note_on`), so release tails keep their pitch.
        let new_bend = bend + best.filter(|_| continuing).map_or(0, |b| b.1);

        // Note-offs first, so a retriggered note never lands on a key that is still held.
        for a in 0..m {
            let i = notes[a] as usize;
            let cur = self.sounding[i].out as i16 + bend;
            plan[a] = match plan[a] {
                Revoice::Hold if new_bend != bend => Revoice::Retrigger(cur.clamp(0, 127) as u8),
                Revoice::Shift(p) if p as i16 - cur != new_bend - bend => Revoice::Retrigger(p),
                p => p,
            };
            // A note about to end is not attacked again for a moment: it plays out as it is,
            // or, rather than bent out of tune for its last moment, it stops here (unless it
            // started just now and would last no time).
            if ends[a] && matches!(plan[a], Revoice::Retrigger(_)) {
                plan[a] = if new_bend != bend && self.sounding[i].attack_ns != now { Revoice::Cut } else { Revoice::Leave };
            }
        }
        // A sounding voice that stops or moves hands its key to a muted voice that stays on it.
        let moves = |p: Revoice| matches!(p, Revoice::Cut | Revoice::Retrigger(_));
        for a in 0..m {
            let s = self.sounding[notes[a] as usize];
            if s.muted || !moves(plan[a]) {
                continue;
            }
            let twin = (0..m).find(|&b| {
                let o = self.sounding[notes[b] as usize];
                o.muted && o.out == s.out && !moves(plan[b])
            });
            if let Some(b) = twin {
                self.sounding[notes[a] as usize].muted = true;
                self.sounding[notes[b] as usize].muted = false;
            }
        }
        for a in 0..m {
            if moves(plan[a]) {
                let s = &mut self.sounding[notes[a] as usize];
                s.active = false;
                if !s.muted {
                    sink.send(&[0x80 | s.dest, s.out, 0]);
                }
            }
        }
        self.set_rtr_bend(dest, new_bend as i8, sink);
        for a in 0..m {
            let Revoice::Retrigger(p) = plan[a] else { continue };
            if self.struck_now(dest, p, chord, due) {
                continue; // the pattern strikes that pitch right now: the note ends here
            }
            let i = notes[a] as usize;
            let out = shift_key(p, -(new_bend as i8));
            let on_key = |b: usize| {
                let o = self.sounding[notes[b] as usize];
                o.active && o.out == out
            };
            // The key already sounds on and on: this voice joins it, muted.
            let muted = (0..m).any(|b| on_key(b) && plan[b] != Revoice::Leave);
            if !muted {
                // A note about to end on that key makes way.
                for &n in &notes[..m] {
                    let o = &mut self.sounding[n as usize];
                    if o.active && o.out == out {
                        o.active = false;
                        if !o.muted {
                            sink.send(&[0x80 | o.dest, o.out, 0]);
                        }
                    }
                }
            }
            let s = &mut self.sounding[i];
            s.active = true;
            s.out = out;
            s.muted = muted;
            if muted {
                continue;
            }
            s.attack_ns = now;
            sink.send(&[0x90 | s.dest, out, s.vel]);
            #[cfg(test)]
            self.retriggered.push((now, dest, out));
        }
    }
}
