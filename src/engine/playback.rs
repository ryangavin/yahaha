//! Pattern playback: `process`, the events, the notes and their pitch bend.

use super::*;

impl Engine {
    // ----- playback -----

    /// Emit everything due up to `now`.
    pub fn process(&mut self, now: u64, sink: &mut impl Sink) {
        self.on_wake(now, sink);
        if !self.running {
            return;
        }
        let mut target = self.tick_at(now) + 1e-6;
        loop {
            let Some(sec) = self.style.sections[self.cur].as_ref() else {
                self.stop(sink);
                return;
            };
            let (boundary, inclusive, swap) = self.boundary();
            // A bar or beat line before the next event and the boundary: its hooks first.
            let line = self.lines.next;
            if line <= target
                && line < boundary - 1e-6
                && sec.events.get(self.ev_idx).is_none_or(|e| line <= self.sec_start + e.tick as f64 + 1e-6)
            {
                self.beat_line(now, sink);
                continue;
            }
            if let Some(e) = sec.events.get(self.ev_idx) {
                let t = self.sec_start + e.tick as f64;
                let before = if inclusive { t <= boundary + 1e-6 } else { t < boundary - 1e-6 };
                if before && t <= target {
                    self.emit_at_index(now, sink);
                    continue;
                }
            }
            if boundary <= target {
                if swap {
                    self.swap_style(boundary, now, sink);
                    // The new style counts its own ticks.
                    target = self.tick_at(now) + 1e-6;
                } else {
                    self.transition(boundary, now, sink);
                }
                if !self.running {
                    return;
                }
                continue;
            }
            break;
        }
    }

    /// A pattern's controller, program change or pitch bend on `dest`; notes are not
    /// controls and send nothing here. A program change for the voice the part already
    /// has (a pattern restating its voice on its first beat) is not sent: a receiver would
    /// reload the voice for nothing.
    pub(super) fn emit_control(&mut self, dest: u8, kind: PKind, sink: &mut impl Sink) {
        match kind {
            PKind::Cc { cc: 7, val } => self.pattern_volume(dest, val, sink),
            PKind::Cc { cc: cc @ (6 | 98..=101), val } if follows_chords(dest) => self.pattern_rpn(dest, cc, val, sink),
            PKind::Cc { cc, val } => self.mirror.send(sink, &[0xB0 | dest, cc, val]),
            PKind::Pc { prog } => {
                let d = dest as usize & 15;
                if self.mirror.voice[d] != Some((self.mirror.cc[d][0], self.mirror.cc[d][32], prog)) {
                    self.mirror.send(sink, &[0xC0 | dest, prog]);
                    self.pattern_pc |= 1 << d;
                }
            }
            PKind::Bend { lo, hi } if follows_chords(dest) => {
                self.pat_bend[dest as usize] = (hi as u16) << 7 | lo as u16;
                self.send_bend(dest, sink);
            }
            PKind::Bend { lo, hi } => self.mirror.send(sink, &[0xE0 | dest, lo, hi]),
            PKind::On { .. } | PKind::Off { .. } => {}
        }
    }

    pub(super) fn chord_for(&self, rule: &ChannelRule) -> Option<Chord> {
        effective_chord(self.chord, rule)
    }

    pub(super) fn emit_at_index(&mut self, now: u64, sink: &mut impl Sink) {
        let sec = self.style.sections[self.cur].as_ref().unwrap();
        let e = sec.events[self.ev_idx];
        let Some(rule) = sec.rules[e.src as usize].as_ref() else {
            self.ev_idx += 1;
            return;
        };
        let dest = rule.dest_ch;
        match e.kind {
            PKind::On { .. } => {
                // Group simultaneous note-ons on this source channel for voice leading.
                let mut keys = [0u8; 8];
                let mut vels = [0u8; 8];
                let mut n = 0;
                let mut j = self.ev_idx;
                while let Some(g) = sec.events.get(j) {
                    if g.tick != e.tick || n == 8 {
                        break;
                    }
                    if g.src == e.src {
                        if let PKind::On { key, vel } = g.kind {
                            keys[n] = key;
                            vels[n] = vel;
                            n += 1;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                    j += 1;
                }
                self.ev_idx = j;
                let part_on = self.parts & (1 << (dest.saturating_sub(8) & 7)) != 0;
                let Some(chord) = self.chord_for(rule) else { return };
                if !part_on || !plays(rule, chord) {
                    return;
                }
                let mut outs = [None; 8];
                transpose_group(&keys[..n], rule, chord, &mut outs[..n]);
                let slot = self.cur as u8;
                for i in 0..n {
                    if let Some(out) = outs[i] {
                        self.note_on(e.src, keys[i], dest, out, vels[i], slot, now, sink);
                    }
                }
            }
            PKind::Off { key } => {
                self.ev_idx += 1;
                self.off_where(sink, |s| s.src == e.src && s.src_key == key);
            }
            PKind::Cc { .. } | PKind::Pc { .. } | PKind::Bend { .. } => {
                self.ev_idx += 1;
                self.emit_control(dest, e.kind, sink);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn note_on(&mut self, src: u8, src_key: u8, dest: u8, out: u8, vel: u8, slot: u8, now: u64, sink: &mut impl Sink) {
        if self.manual_bass && dest == BASS_CH {
            return;
        }
        let pitch = self.master(dest, out);
        // A part a pitch shift left bent is straightened once it has fallen silent.
        if self.rtr_bend[dest as usize & 15] != 0 && !self.sounding.iter().any(|s| s.active && s.dest == dest) {
            self.set_rtr_bend(dest, 0, sink);
        }
        let guitar = self.is_guitar(slot, src, src_key);
        // On a part still bent, the key goes out that much lower so it sounds at `pitch`.
        // A guitar noise key (MegaVoice) is not a pitch: moving it would pick another
        // noise, so it goes out as it is (and sounds with the part's bend).
        let out = if guitar && src_key >= GUITAR_NOISE { pitch } else { shift_key(pitch, -self.rtr_bend[dest as usize & 15]) };
        // Two voices landing on one key at the same instant (a chord that folds voices
        // together, such as 1+8, or a pattern note on the key a chord change just
        // retriggered) sound once: stealing would leave a zero-length note. The later
        // voice is kept muted beside the sounding one, and the key sounds until both
        // have ended. (Rhythm parts play as written: a doubled hit stays doubled.)
        // A guitar strum spreads its strings over a few ticks, so for a Guitar note the
        // "instant" is the strum: a string on a key another string struck within a 32nd
        // note joins it, muted, instead of cutting it short and striking the pitch again.
        // A later strum strikes it again, as a guitarist would.
        let strum = (self.ns_per_tick * self.style.ppq as f64 / 8.0) as u64;
        let same_instant = |s: &Sounding| {
            s.active && s.dest == dest && s.out == out && (s.attack_ns == now || guitar && now.saturating_sub(s.attack_ns) < strum)
        };
        let muted = follows_chords(dest) && self.sounding.iter().any(same_instant);
        if !muted {
            // Steal an identical sounding note on the same channel so offs stay balanced.
            self.off_where(sink, |s| s.dest == dest && s.out == out);
        }
        if let Some(free) = self.sounding.iter_mut().find(|s| !s.active) {
            *free = Sounding { active: true, src, src_key, dest, out, vel, slot, started_ns: now, attack_ns: now, muted };
            if !muted {
                sink.send(&[0x90 | dest, out, vel]);
            }
        }
    }

    /// Send a part's pitch bend: the pattern's own bend, rescaled from the style's bend
    /// range to the part's output range, plus the Retrigger Rule pitch shift.
    pub(super) fn send_bend(&mut self, ch: u8, sink: &mut impl Sink) {
        let c = ch as usize & 15;
        let (style, out) = (self.bend_range[c] as f32, self.out_range(ch) as f32);
        let centre = BEND_CENTRE as f32;
        let pat = (self.pat_bend[c] as f32 - centre) * style / out;
        let rtr = self.rtr_bend[c] as f32 * centre / out;
        let v = (centre + pat + rtr).round();
        // (A full bend up, centre + 8192, is sent as 16383, 0.15 cents short at most.)
        #[cfg(test)]
        if !(0.0..=16384.0).contains(&v) {
            self.bend_clamps.set(self.bend_clamps.get() + 1);
        }
        let v = v.clamp(0.0, 16383.0) as u16;
        self.mirror.send(sink, &[0xE0 | ch, (v & 0x7F) as u8, (v >> 7) as u8]);
    }

    /// The pitch bend range part `ch` has on the output.
    #[inline]
    pub(super) fn out_range(&self, ch: u8) -> u8 {
        out_bend_range(self.bend_range[ch as usize & 15], self.style.pat_bend_max[ch as usize & 15])
    }

    /// Bend every note on `ch` by `semis` (the Retrigger Rule pitch shift).
    pub(super) fn set_rtr_bend(&mut self, ch: u8, semis: i8, sink: &mut impl Sink) {
        let c = ch as usize & 15;
        if self.rtr_bend[c] != semis {
            self.rtr_bend[c] = semis;
            self.send_bend(ch, sink);
            sink.retune(ch, semis);
        }
    }

    /// (N)RPN messages from a pattern on a part that follows chords. A pitch bend range
    /// (RPN 0) becomes the style's range the part's bends are rescaled from; the part
    /// itself gets `out_range` of it.
    pub(super) fn pattern_rpn(&mut self, ch: u8, cc: u8, val: u8, sink: &mut impl Sink) {
        let c = ch as usize & 15;
        if cc == 6 && self.rpn[c] == 0 {
            self.bend_range[c] = val;
            let range = self.out_range(ch);
            self.mirror.send(sink, &[0xB0 | ch, 6, range]);
            self.send_bend(ch, sink);
            return;
        }
        self.rpn[c] = select_rpn(self.rpn[c], cc, val);
        self.mirror.send(sink, &[0xB0 | ch, cc, val]);
    }

    /// End the voices `f` picks. A muted voice ends silently; a sounding one hands its key
    /// to a muted voice that shares it and sounds on, if there is one.
    /// Does this source note follow the chord as a guitar string (NTR Guitar)?
    pub(super) fn is_guitar(&self, slot: u8, src: u8, src_key: u8) -> bool {
        let rule = self.style.sections.get(slot as usize).and_then(|s| s.as_ref()).and_then(|s| s.rules.get(src as usize));
        rule.and_then(|r| r.as_ref()).is_some_and(|r| r.zone_for(src_key).ntr == Ntr::Guitar)
    }

    /// Tests: pairs of sounding (not muted) guitar strings of one part on the same key.
    #[cfg(test)]
    pub fn guitar_unisons(&self) -> usize {
        let s = &self.sounding;
        let live = |i: usize| s[i].active && !s[i].muted && self.is_guitar(s[i].slot, s[i].src, s[i].src_key);
        (0..MAX_SOUNDING)
            .filter(|&i| live(i))
            .map(|i| (i + 1..MAX_SOUNDING).filter(|&j| live(j) && (s[j].dest, s[j].out) == (s[i].dest, s[i].out)).count())
            .sum()
    }

    /// Tests: the guitar strings of one part struck at or after `since`, muted twins
    /// included, as (source key, sounding pitch).
    #[cfg(test)]
    pub fn guitar_strings(&self, dest: u8, since: u64) -> Vec<(u8, u8)> {
        let bend = self.rtr_bend[dest as usize & 15];
        let mut v: Vec<_> = self
            .sounding
            .iter()
            .filter(|s| s.active && s.dest == dest && s.started_ns >= since && self.is_guitar(s.slot, s.src, s.src_key))
            .map(|s| (s.src_key, if s.src_key >= GUITAR_NOISE { s.out } else { shift_key(s.out, bend) }))
            .collect();
        v.sort();
        v
    }

    pub(super) fn off_where(&mut self, sink: &mut impl Sink, f: impl Fn(&Sounding) -> bool) {
        for i in 0..MAX_SOUNDING {
            let s = self.sounding[i];
            if !s.active || !f(&s) {
                continue;
            }
            self.sounding[i].active = false;
            if s.muted {
                continue;
            }
            let twin = |o: &Sounding| o.active && o.muted && o.dest == s.dest && o.out == s.out && !f(o);
            match self.sounding.iter_mut().find(|o| twin(o)) {
                Some(o) => o.muted = false,
                None => sink.send(&[0x80 | s.dest, s.out, 0]),
            }
        }
    }

    pub(super) fn all_off(&mut self, sink: &mut impl Sink) {
        self.notes_off(false, sink);
    }

    /// End every note. Patterns bend (bass slides etc.); leaving a section or style
    /// mid-bend would leave the part detuned, so bends are re-centred and wheels and pedal
    /// cleared on every part. At a section change (`changed_only`) only on the parts where
    /// they are not already there, so the new section's first notes are not queued behind
    /// 24 messages that change nothing.
    pub(super) fn notes_off(&mut self, changed_only: bool, sink: &mut impl Sink) {
        self.off_where(sink, |_| true);
        for ch in 8..16u8 {
            let c = ch as usize;
            if !changed_only || self.mirror.bend[c] != Some(BEND_CENTRE) {
                self.mirror.send(sink, &[0xE0 | ch, 0x00, 0x40]);
            }
            for cc in [1u8, 64] {
                if !changed_only || self.mirror.cc[c][cc as usize] != 0 {
                    self.mirror.send(sink, &[0xB0 | ch, cc, 0]);
                }
            }
        }
        self.pat_bend = [BEND_CENTRE; 16];
        for ch in 0..16u8 {
            if self.rtr_bend[ch as usize] != 0 {
                self.rtr_bend[ch as usize] = 0;
                sink.retune(ch, 0);
            }
        }
    }
}
