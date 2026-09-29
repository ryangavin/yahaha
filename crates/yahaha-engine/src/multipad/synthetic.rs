//! Synthetic Multi Pad banks: small, original phrases written in the documented Tyros
//! layout (see `file`), for tests, the offline/dev session and `yahaha pad --demo`. No
//! Yamaha data: every note here is made up.

/// One pad of a synthetic bank.
pub struct Phrase {
    pub name: &'static str,
    /// GM program, and bank MSB (127 = a drum kit, as on the Genos).
    pub program: u8,
    pub bank_msb: u8,
    pub repeat: bool,
    pub chord_match: bool,
    /// (tick at 1920 ppq, key, velocity, length in ticks).
    pub notes: Vec<(u32, u8, u8, u32)>,
}

pub const PPQ: u16 = 1920;
const BEAT: u32 = PPQ as u32;
const BAR: u32 = 4 * BEAT;

/// The demo bank: 1 "Shaker Loop" (drums, Repeat, no Chord Match), 2 "Rise Arp" (a CM7
/// arpeggio, one shot, Chord Match), 3 "Bass Riff" (Repeat, Chord Match), 4 "Brass Hit"
/// (a CM7 stab, one shot, Chord Match). One 4/4 bar each at 120 BPM.
pub fn demo_phrases() -> Vec<Phrase> {
    let e = BEAT / 2;
    let mut shaker = Vec::new();
    for i in 0..8 {
        shaker.push((i * e, 42, if i % 2 == 0 { 100 } else { 70 }, e));
    }
    shaker.push((BEAT, 39, 110, e));
    shaker.push((3 * BEAT, 39, 110, e));
    let arp = [60, 64, 67, 71, 72, 71, 67, 64].iter().enumerate().map(|(i, &k)| (i as u32 * e, k, 96, e)).collect();
    let bass = vec![(0, 36, 110, e), (e, 36, 90, e), (BEAT, 43, 100, BEAT), (2 * BEAT, 45, 100, BEAT), (3 * BEAT, 43, 95, BEAT)];
    let brass = [60, 64, 67, 71].iter().map(|&k| (0, k, 112, BEAT + e)).chain([(2 * BEAT, 67, 90, e), (2 * BEAT, 72, 90, e)]).collect();
    vec![
        Phrase { name: "Shaker Loop", program: 0, bank_msb: 127, repeat: true, chord_match: false, notes: shaker },
        Phrase { name: "Rise Arp", program: 81, bank_msb: 0, repeat: false, chord_match: true, notes: arp },
        Phrase { name: "Bass Riff", program: 33, bank_msb: 0, repeat: true, chord_match: true, notes: bass },
        Phrase { name: "Brass Hit", program: 61, bank_msb: 0, repeat: false, chord_match: true, notes: brass },
    ]
}

/// A Tyros-layout bank file (type 1 SMF, 5 tracks, 1920 ppq, track 0 with the CM/RP/N
/// text events, pad n on track n and channel n) from up to four phrases. Every pad's pass
/// is padded to whole bars with a final controller, so repeats stay on the bar.
pub fn bank_bytes(name: &str, phrases: &[Phrase]) -> Vec<u8> {
    let flags = |f: &dyn Fn(&Phrase) -> bool| -> String {
        (0..4).map(|i| if phrases.get(i).is_some_and(f) { '1' } else { '0' }).collect()
    };
    let text = |s: &str| meta(0x01, s.as_bytes());
    let mut conductor = vec![
        (0, meta(0x03, name.as_bytes())),
        (0, meta(0x51, &[0x07, 0xA1, 0x20])), // 120 BPM
        (0, meta(0x58, &[4, 2, 24, 8])),
        (0, text(&format!("CM{}", flags(&|p| p.chord_match)))),
        (0, text(&format!("RP{}", flags(&|p| p.repeat)))),
    ];
    for (i, p) in phrases.iter().take(4).enumerate() {
        let mut t = format!("N{}{} {}", i + 1, p.name, i + 1);
        while t.len() < 52 {
            t.push(' ');
        }
        conductor.push((0, text(&t)));
    }
    let mut f = header(1, 5, PPQ);
    f.extend(track(&mut conductor));
    for i in 0..4 {
        let Some(p) = phrases.get(i) else {
            f.extend(track(&mut []));
            continue;
        };
        let ch = i as u8;
        let mut evs = vec![(0, vec![0xB0 | ch, 0, p.bank_msb]), (0, vec![0xB0 | ch, 32, 0]), (0, vec![0xC0 | ch, p.program])];
        let mut end = 0;
        for &(t, k, v, len) in &p.notes {
            evs.push((t, vec![0x90 | ch, k, v]));
            evs.push((t + len, vec![0x80 | ch, k, 0]));
            end = end.max(t + len);
        }
        let bars = end.div_ceil(BAR).max(1) * BAR;
        // Expression back to full on the bar line: a harmless event that marks the pass end.
        evs.push((bars, vec![0xB0 | ch, 11, 127]));
        f.extend(track(&mut evs));
    }
    f
}

/// The demo bank as a file.
pub fn demo_bank() -> Vec<u8> {
    bank_bytes("Synthetic Demo", &demo_phrases())
}

fn vlq(mut v: u32, out: &mut Vec<u8>) {
    let mut buf = [0u8; 5];
    let mut n = 0;
    loop {
        buf[n] = (v & 0x7F) as u8;
        n += 1;
        v >>= 7;
        if v == 0 {
            break;
        }
    }
    for i in (0..n).rev() {
        out.push(buf[i] | if i > 0 { 0x80 } else { 0 });
    }
}

/// An MTrk from (absolute tick, event bytes), sorted by tick (offs before ons at a tick).
/// Meta events are `[0xFF, ty, data...]` and get their length inserted.
fn track(events: &mut [(u32, Vec<u8>)]) -> Vec<u8> {
    events.sort_by_key(|(t, e)| (*t, e[0] & 0xF0 == 0x90));
    let mut body = Vec::new();
    let mut last = 0;
    for (t, ev) in events.iter() {
        vlq(t - last, &mut body);
        last = *t;
        if ev[0] == 0xFF {
            body.extend_from_slice(&ev[..2]);
            vlq(ev.len() as u32 - 2, &mut body);
            body.extend_from_slice(&ev[2..]);
        } else {
            body.extend_from_slice(ev);
        }
    }
    body.extend_from_slice(&[0, 0xFF, 0x2F, 0]);
    chunk(b"MTrk", &body)
}

fn chunk(id: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut v = id.to_vec();
    v.extend_from_slice(&(data.len() as u32).to_be_bytes());
    v.extend_from_slice(data);
    v
}

fn header(format: u16, ntracks: u16, ppq: u16) -> Vec<u8> {
    let mut h = Vec::new();
    h.extend_from_slice(&format.to_be_bytes());
    h.extend_from_slice(&ntracks.to_be_bytes());
    h.extend_from_slice(&ppq.to_be_bytes());
    chunk(b"MThd", &h)
}

fn meta(ty: u8, data: &[u8]) -> Vec<u8> {
    let mut v = vec![0xFF, ty];
    v.extend_from_slice(data);
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multipad::file::parse;

    #[test]
    fn the_demo_bank_parses_with_its_flags_names_and_one_bar_passes() {
        let b = parse(&demo_bank()).unwrap();
        assert_eq!(b.ppq, 1920);
        assert_eq!(b.name, "Synthetic Demo");
        let got: Vec<_> = b.pads.iter().map(|p| p.as_ref().map(|p| (p.name.clone(), p.repeat, p.chord_match, p.len))).collect();
        assert_eq!(
            got,
            [
                Some(("Shaker Loop".into(), Some(true), Some(false), 7680)),
                Some(("Rise Arp".into(), Some(false), Some(true), 7680)),
                Some(("Bass Riff".into(), Some(true), Some(true), 7680)),
                Some(("Brass Hit".into(), Some(false), Some(true), 7680)),
            ]
        );
    }

    #[test]
    fn fewer_phrases_leave_the_other_pads_empty() {
        let mut p = demo_phrases();
        p.truncate(2);
        let b = parse(&bank_bytes("Two", &p)).unwrap();
        assert!(b.pads[0].is_some() && b.pads[1].is_some());
        assert!(b.pads[2].is_none() && b.pads[3].is_none());
    }
}
