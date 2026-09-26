//! A part's XG voice settings on the built-in synth (#246).
//!
//! An OTS or Registration recall (#248/#252) sets a keyboard part's mono/poly with XG Multi
//! Part SysEx (`F0 43 1n 4C 08 pp 05 vv F7`), and a style's SInt may set its parts' filter,
//! EG, vibrato and portamento the same way. The synth ring takes 3-byte messages only, so
//! [`encode`] turns them into what the synth plays:
//! - vibrato rate/depth/delay, filter cutoff/resonance, EG attack/decay/release
//!   (15H-1CH) and portamento switch/time (67H/68H): the controller that sets the same
//!   parameter (CC76-78, 74, 71, 73, 75, 72, 65, 5; Data List: the XG part parameter and the
//!   controller are one setting);
//! - mono/poly (05H): a *mono message* `[MONO, part, value]` (first byte below 80H, which no
//!   MIDI message starts with), which switches the part's channel to mono or poly without
//!   the All Notes Off that CC126/127 carry.
//!
//! The XG part is the channel of the same number (the XG default Receive Channel, as
//! `parts::Parts::send_tone` and the style setup address them).

use super::Msg;

/// The mono message's first byte (after the drum messages, 40H-62H).
pub const MONO: u8 = 0x70;

/// XG Multi Part parameter -> the controller that sets it.
fn controller(addr: u8) -> Option<u8> {
    Some(match addr {
        0x15 => 76,
        0x16 => 77,
        0x17 => 78,
        0x18 => 74,
        0x19 => 71,
        0x1A => 73,
        0x1B => 75,
        0x1C => 72,
        0x67 => 65,
        0x68 => 5,
        _ => return None,
    })
}

/// What the synth plays for `m`, if it is an XG Multi Part parameter the synth takes: the
/// controller that sets the same parameter, or a mono message.
#[inline]
pub fn encode(m: &[u8]) -> Option<Msg> {
    let (part, addr, v) = crate::sff::xg_part_param(m)?;
    match addr {
        0x05 => Some([MONO, part, v]),
        // Portamento switch: 00 off, 01 on.
        0x67 => Some([0xB0 | part, 65, if v > 0 { 127 } else { 0 }]),
        _ => controller(addr).map(|cc| [0xB0 | part, cc, v & 0x7F]),
    }
}

/// A mono message (`encode`): (channel, mono).
#[inline]
pub fn mono(m: &Msg) -> Option<(u8, bool)> {
    (m[0] == MONO).then_some((m[1] & 0x0F, m[2] == 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xg(part: u8, addr: u8, v: u8) -> Vec<u8> {
        vec![0xF0, 0x43, 0x10, 0x4C, 0x08, part, addr, v, 0xF7]
    }

    #[test]
    fn xg_part_voice_settings_become_what_the_synth_plays() {
        assert_eq!(encode(&xg(1, 0x05, 0)), Some([MONO, 1, 0]));
        assert_eq!(mono(&[MONO, 1, 0]), Some((1, true)));
        assert_eq!(mono(&[MONO, 3, 1]), Some((3, false)));
        assert_eq!(encode(&xg(0, 0x18, 30)), Some([0xB0, 74, 30]));
        assert_eq!(encode(&xg(11, 0x1C, 90)), Some([0xBB, 72, 90]));
        assert_eq!(encode(&xg(2, 0x67, 1)), Some([0xB2, 65, 127]));
        assert_eq!(encode(&xg(2, 0x68, 40)), Some([0xB2, 5, 40]));
        // Not a voice setting the synth takes: volume, part mode, a drum setup, a reset.
        for m in [xg(0, 0x0B, 100), xg(9, 0x07, 2), vec![0xF0, 0x43, 0x10, 0x4C, 0x30, 38, 0x02, 50, 0xF7], vec![0xF0, 0x7E, 0x7F, 0x09, 0x01, 0xF7]] {
            assert_eq!(encode(&m), None, "{m:02X?}");
        }
        assert_eq!(mono(&[0xB0, 74, 0]), None);
    }
}
