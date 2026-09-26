//! What the engine has sent on each channel, so a section change sends only what
//! differs.

use super::*;

/// A controller value `Mirror` has not seen sent.
pub(super) const UNSENT: u8 = 0xFF;
/// (N)RPN values `Mirror` remembers per channel.
pub(super) const MIRROR_PARAMS: usize = 8;
/// A free `Mirror::params` slot.
pub(super) const NO_PARAM: u16 = 0xFFFF;
/// `Mirror` parameter numbers: an NRPN has this bit set, an RPN has not.
pub(super) const NRPN_BIT: u16 = 0x4000;

/// What the engine has sent on each channel: controllers, voice, (N)RPN data entries and
/// pitch bend. A section change compares the style's channel setup with it and sends only
/// what differs, so a section change that changes nothing sends nothing: a receiver never
/// reloads a voice it already plays or has its drum setup rewritten (see
/// `Engine::reapply_init`).
pub(super) struct Mirror {
    pub(super) cc: [[u8; 128]; 16],
    /// (bank MSB, LSB, program) of the last program change, with the bank selects in
    /// effect when it was sent.
    pub(super) voice: [Option<(u8, u8, u8)>; 16],
    pub(super) rpn: [u16; 16],
    pub(super) nrpn: [u16; 16],
    /// Channels where an NRPN was selected last (else an RPN).
    pub(super) nrpn_on: u16,
    /// (parameter, data entry MSB, LSB) per channel; parameter as `selected` returns it.
    pub(super) params: [[(u16, u8, u8); MIRROR_PARAMS]; 16],
    pub(super) bend: [Option<u16>; 16],
    /// Channels in mono mode (#253): CC126/127, or the XG part's Mono/Poly (08 pp 05).
    pub(super) mono: u16,
}

impl Mirror {
    pub(super) const NEW: Mirror = Mirror {
        cc: [[UNSENT; 128]; 16],
        voice: [None; 16],
        rpn: [RPN_NULL; 16],
        nrpn: [RPN_NULL; 16],
        nrpn_on: 0,
        params: [[(NO_PARAM, UNSENT, UNSENT); MIRROR_PARAMS]; 16],
        bend: [None; 16],
        mono: 0,
    };

    /// The (N)RPN a data entry on `ch` sets, if any (the null RPN/NRPN sets nothing).
    pub(super) fn selected(&self, ch: usize) -> Option<u16> {
        if self.nrpn_on & (1 << ch) != 0 {
            (self.nrpn[ch] != RPN_NULL).then_some(NRPN_BIT | self.nrpn[ch])
        } else {
            (self.rpn[ch] != RPN_NULL).then_some(self.rpn[ch])
        }
    }

    /// The data entry sent for parameter `p` on `ch` as (MSB, LSB), if known.
    pub(super) fn param(&self, ch: usize, p: u16) -> Option<(u8, u8)> {
        self.params[ch].iter().find(|e| e.0 == p).map(|e| (e.1, e.2))
    }

    pub(super) fn set_param(&mut self, ch: usize, p: u16, msb: Option<u8>, lsb: Option<u8>) {
        let slots = &mut self.params[ch];
        let i = slots.iter().position(|e| e.0 == p).or_else(|| slots.iter().position(|e| e.0 == NO_PARAM));
        // Full: forget the oldest; an unknown value is simply sent again.
        let i = i.unwrap_or_else(|| {
            slots.rotate_left(1);
            slots[MIRROR_PARAMS - 1] = (NO_PARAM, UNSENT, UNSENT);
            MIRROR_PARAMS - 1
        });
        let e = &mut slots[i];
        if e.0 != p {
            *e = (p, UNSENT, UNSENT);
        }
        if let Some(v) = msb {
            e.1 = v;
        }
        if let Some(v) = lsb {
            e.2 = v;
        }
    }

    /// Note a message sent.
    pub(super) fn track(&mut self, m: &[u8]) {
        // An XG part's Mono/Poly (the XG part is the channel of its number).
        if let Some((part, 0x05, v)) = crate::sff::xg_part_param(m) {
            self.set_mono(part, v == 0);
            return;
        }
        if m.len() < 2 || m[0] >= 0xF0 {
            return;
        }
        let ch = (m[0] & 0x0F) as usize;
        match (m[0] & 0xF0, m.len()) {
            (0xB0, 3) => {
                let (cc, v) = (m[1] & 0x7F, m[2]);
                self.cc[ch][cc as usize] = v;
                let bit = 1 << ch;
                match cc {
                    101 | 100 => {
                        self.rpn[ch] = select_rpn(self.rpn[ch], cc, v);
                        self.nrpn_on &= !bit;
                    }
                    99 => {
                        self.nrpn[ch] = (self.nrpn[ch] & 0x7F) | (v as u16) << 7;
                        self.nrpn_on |= bit;
                    }
                    98 => {
                        self.nrpn[ch] = (self.nrpn[ch] & !0x7F) | v as u16;
                        self.nrpn_on |= bit;
                    }
                    6 | 38 => {
                        if let Some(p) = self.selected(ch) {
                            let (msb, lsb) = if cc == 6 { (Some(v), None) } else { (None, Some(v)) };
                            self.set_param(ch, p, msb, lsb);
                        }
                    }
                    126 => self.set_mono(ch as u8, true),
                    127 => self.set_mono(ch as u8, false),
                    _ => {}
                }
            }
            (0xC0, _) => self.voice[ch] = Some((self.cc[ch][0], self.cc[ch][32], m[1])),
            (0xE0, 3) => self.bend[ch] = Some((m[2] as u16) << 7 | m[1] as u16),
            _ => {}
        }
    }

    fn set_mono(&mut self, ch: u8, mono: bool) {
        let bit = 1 << (ch & 15);
        if mono {
            self.mono |= bit;
        } else {
            self.mono &= !bit;
        }
    }

    /// Send `m` and note it.
    #[inline]
    pub(super) fn send(&mut self, sink: &mut impl Sink, m: &[u8]) {
        self.track(m);
        sink.send(m);
    }

    /// Select parameter `p` (as `selected` returns it; `None` = the null RPN) on `ch`.
    pub(super) fn select(&mut self, sink: &mut impl Sink, ch: u8, p: Option<u16>) {
        let st = 0xB0 | ch;
        match p {
            Some(p) if p & NRPN_BIT != 0 => {
                let n = p & !NRPN_BIT;
                self.send(sink, &[st, 99, (n >> 7) as u8]);
                self.send(sink, &[st, 98, (n & 0x7F) as u8]);
            }
            p => {
                let r = p.unwrap_or(RPN_NULL);
                self.send(sink, &[st, 101, (r >> 7) as u8]);
                self.send(sink, &[st, 100, (r & 0x7F) as u8]);
            }
        }
    }
}
