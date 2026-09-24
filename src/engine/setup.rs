//! The style's channel setup (SInt): sent on load and start, re-applied (only what
//! differs) at section changes. Each section plays the setup as its own channel rules
//! route it (`Prepared::setup`, #64).

use super::*;

impl Engine {
    /// The style's channel setup, with the mixer's volume on each part in place of the
    /// style's own CC7 so a restart never undoes a fader the player moved.
    ///
    /// Playing, it is the setup as the section playing (`self.cur`) routes it. Stopped (a
    /// style loaded, a style change after an Ending, a resync after a preview), it is the
    /// setup as the Main the band would start on routes it: `self.cur` still names the
    /// section that played last (an Ending), whose routing is not the stopped style's.
    pub fn send_init(&mut self, sink: &mut impl Sink) {
        if !self.running {
            self.cur = self.home_slot();
        }
        self.bend_range = self.style.setup(self.cur).bend_range;
        self.rpn = [RPN_NULL; 16];
        self.reset_expression(sink);
        for i in 0..self.style.setup(self.cur).init.len() {
            let m = self.style.setup(self.cur).init.get(i);
            if m[0] == 0xF0 || m.len() > 3 {
                sink.send(m);
                continue;
            }
            let mut b = [0u8; 3];
            b[..m.len()].copy_from_slice(m);
            let m = &b[..m.len()];
            let ch = m[0] & 0x0F;
            if m[0] & 0xF0 == 0xE0 && m.len() == 3 && follows_chords(ch) {
                self.pat_bend[ch as usize] = (m[2] as u16) << 7 | m[1] as u16;
                self.send_bend(ch, sink);
            } else {
                self.mirror.send(sink, m);
            }
        }
        for p in 0..8u8 {
            let v = self.faded(self.mixer[p as usize]);
            self.mirror.send(sink, &[0xB0 | (8 + p), 7, v]);
        }
        self.pattern_pc = 0;
        self.sync_rpn();
    }

    /// Expression (CC11) back to full on every Style part where the engine left it lower
    /// (#122). Patterns move it (an Ending's fade-out, a swell, ghost notes), and most
    /// styles' setups never set it, so without this the last pattern's value stuck: after
    /// an Ending faded the Bass to 8, the next style (or the same one started again)
    /// played its Bass inaudibly. A setup that sets CC11 itself still has the last word
    /// (it goes out after this). Only where the mirror knows a lower value: a part never
    /// touched is already at the receiver's default, 127.
    fn reset_expression(&mut self, sink: &mut impl Sink) {
        for ch in 8..16u8 {
            let v = self.mirror.cc[ch as usize][11];
            if v != 127 && v != UNSENT {
                self.mirror.send(sink, &[0xB0 | ch, 11, 127]);
            }
        }
    }

    /// The section the band would start on (the Main in use), whose routing of the setup a
    /// stopped style has.
    pub(super) fn home_slot(&self) -> usize {
        self.style.resolve(4 + self.main as usize).unwrap_or(4)
    }

    /// The RPN each channel has selected, as far as a pattern's data entry is concerned.
    pub(super) fn sync_rpn(&mut self) {
        for ch in 0..16 {
            self.rpn[ch] = if self.mirror.nrpn_on & (1 << ch) != 0 { RPN_NULL } else { self.mirror.rpn[ch] };
        }
    }

    /// A section change puts the style's part setup (SInt) back, so a voice or controller
    /// a pattern changed does not carry into the next section. It sends only what differs
    /// from what the channel has (`Mirror`): a section change after sections that changed
    /// nothing sends nothing, as on a Genos, where changing section never reloads a voice
    /// or resets a part. Resending it all cost every boundary 150-200 messages (up to
    /// 700 bytes, with a program change per part and the drum setup SysEx) queued ahead of
    /// the new section's first notes.
    ///
    /// A program change goes out only for a voice that differs, and not on the channels in
    /// `own_voice`, whose new section sets its own voice by its entry point. The part's XG
    /// parameters follow a program change sent (it resets them on an XG receiver), and the
    /// drum setup SysEx goes again only after one (a program change resets it), or when a
    /// pattern's own program change reset them since (`pattern_pc`), even if that pattern
    /// has since gone back to the setup's voice and no program change is needed. The effect
    /// SysEx is never re-sent: no pattern changes it, and an XG receiver would cut the
    /// reverb and delay tails. The part levels (CC7) are the faders: like a pattern CC7,
    /// the SInt's moves only the faders the player has not moved.
    pub(super) fn reapply_init(&mut self, own_voice: u16, sink: &mut impl Sink) {
        self.restore_untouched_levels();
        self.bend_range = self.style.setup(self.cur).bend_range;
        let mut voice_sent = 0u16;
        // A pattern's program change reset these parts' XG parameters and drum setup on the
        // receiver, even if it went back to the setup's voice (no program change here). Not
        // on the parts whose new section sets its own voice: the setup's parameters belong
        // to the setup's voice. They are put back at the first section change that doesn't.
        let reset = self.pattern_pc & !own_voice;
        let kits = (0..16).filter(|&c| self.style.setup(self.cur).kit[c]).fold(0u16, |m, c| m | 1 << c);
        // The (N)RPN the setup selects on each channel as it goes, and the channels where it
        // selects one.
        let (mut rpn, mut nrpn, mut nrpn_on) = ([RPN_NULL; 16], [RPN_NULL; 16], 0u16);
        let mut selects = 0u16;
        for i in 0..self.style.setup(self.cur).init_resend {
            let m = self.style.setup(self.cur).init.get(i);
            if m[0] == 0xF0 {
                // XG Multi Part parameter (08 pp): after that part's program change only.
                let send = match *m {
                    [0xF0, 0x43, d, 0x4C, 0x08, part, ..] if d & 0xF0 == 0x10 => (voice_sent | reset) & (1 << (part & 15)) != 0,
                    _ if crate::sff::is_drum_setup(m) => voice_sent != 0 || reset & kits != 0,
                    _ => false,
                };
                if send {
                    sink.send(m);
                }
                continue;
            }
            if m.len() > 3 {
                continue;
            }
            let mut b = [0u8; 3];
            b[..m.len()].copy_from_slice(m);
            let m = &b[..m.len()];
            let ch = (m[0] & 0x0F) as usize;
            let bit = 1u16 << ch;
            match (m[0] & 0xF0, m.len()) {
                (0xB0, 3) => match m[1] {
                    101 | 100 => {
                        rpn[ch] = select_rpn(rpn[ch], m[1], m[2]);
                        nrpn_on &= !bit;
                        selects |= bit;
                    }
                    99 | 98 => {
                        nrpn[ch] = if m[1] == 99 { (nrpn[ch] & 0x7F) | (m[2] as u16) << 7 } else { (nrpn[ch] & !0x7F) | m[2] as u16 };
                        nrpn_on |= bit;
                        selects |= bit;
                    }
                    6 | 38 => {
                        let sel = if nrpn_on & bit != 0 {
                            (nrpn[ch] != RPN_NULL).then_some(NRPN_BIT | nrpn[ch])
                        } else {
                            (rpn[ch] != RPN_NULL).then_some(rpn[ch])
                        };
                        let Some(p) = sel else { continue };
                        let have = self.mirror.param(ch, p);
                        let same = have.is_some_and(|(msb, lsb)| if m[1] == 6 { msb == m[2] } else { lsb == m[2] });
                        if !same {
                            if self.mirror.selected(ch) != Some(p) {
                                self.mirror.select(sink, ch as u8, Some(p));
                            }
                            self.mirror.send(sink, m);
                        }
                    }
                    cc => {
                        if self.mirror.cc[ch][cc as usize] != m[2] {
                            self.mirror.send(sink, m);
                        }
                    }
                },
                (0xC0, _) => {
                    let want = (self.mirror.cc[ch][0], self.mirror.cc[ch][32], m[1]);
                    if own_voice & bit == 0 && self.mirror.voice[ch] != Some(want) {
                        self.mirror.send(sink, m);
                        voice_sent |= bit;
                    }
                }
                (0xE0, 3) => {
                    let v = (m[2] as u16) << 7 | m[1] as u16;
                    if follows_chords(ch as u8) {
                        if self.pat_bend[ch] != v {
                            self.pat_bend[ch] = v;
                            self.send_bend(ch as u8, sink);
                        }
                    } else if self.mirror.bend[ch] != Some(v) {
                        self.mirror.send(sink, m);
                    }
                }
                _ => {}
            }
        }
        // Leave each channel with the parameter the setup leaves selected (the null RPN).
        for ch in 0..16 {
            if selects & (1 << ch) == 0 {
                continue;
            }
            let want = if nrpn_on & (1 << ch) != 0 {
                (nrpn[ch] != RPN_NULL).then_some(NRPN_BIT | nrpn[ch])
            } else {
                (rpn[ch] != RPN_NULL).then_some(rpn[ch])
            };
            if self.mirror.selected(ch) != want {
                self.mirror.select(sink, ch as u8, want);
            }
        }
        self.pattern_pc &= own_voice;
        self.sync_rpn();
        for p in 0..8u8 {
            let v = self.faded(self.mixer[p as usize]);
            if self.user_set & (1 << p) == 0 && self.mirror.cc[8 + p as usize][7] != v {
                self.mirror.send(sink, &[0xB0 | (8 + p), 7, v]);
            }
        }
    }

    /// Something else played on these channels (a style preview): forget what was sent
    /// and send the style's setup again, all of it.
    pub fn resync(&mut self, sink: &mut impl Sink) {
        *self.mirror = Mirror::NEW;
        // The preview may have left any expression: taken as not full, so it goes back.
        for ch in 8..16 {
            self.mirror.cc[ch][11] = 0;
        }
        self.send_init(sink);
    }
}
