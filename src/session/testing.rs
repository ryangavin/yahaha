//! Test sessions that need no corpus: a minimal style generated here (original notes, no
//! Yamaha data), written to a temp file, so session tests run in CI. Tests that need a
//! real style's content (its OTS, effects or particular parts) still use the git-ignored
//! `corpus/` and skip without it.

use super::{Options, Session};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Ticks per quarter note of the synthetic style.
const PPQ: u16 = 96;
/// One 4/4 bar.
const BAR: u32 = PPQ as u32 * 4;

/// The sections and their lengths in bars.
const SECTIONS: [(&str, u32); 11] = [
    ("Intro A", 1),
    ("Main A", 4),
    ("Main B", 4),
    ("Main C", 4),
    ("Main D", 4),
    ("Fill In AA", 1),
    ("Fill In BB", 1),
    ("Fill In CC", 1),
    ("Fill In DD", 1),
    ("Fill In BA", 1),
    ("Ending A", 1),
];

/// Each style part (channels 9-16): bank MSB, LSB, program and the notes it plays on beat 1.
const PARTS: [(u8, u8, u8, &[u8]); 8] = [
    (127, 0, 0, &[36]),         // Rhythm 1: kick
    (127, 0, 0, &[42]),         // Rhythm 2: closed hat
    (0, 0, 33, &[36]),          // Bass
    (0, 0, 0, &[60, 64, 67]),   // Chord 1
    (0, 0, 25, &[64, 67, 71]),  // Chord 2
    (0, 0, 48, &[60, 67]),      // Pad
    (0, 0, 61, &[72]),          // Phrase 1
    (0, 0, 73, &[79]),          // Phrase 2
];

fn vlq(mut v: u32, out: &mut Vec<u8>) {
    let mut bytes = vec![(v & 0x7F) as u8];
    v >>= 7;
    while v > 0 {
        bytes.push((v & 0x7F) as u8 | 0x80);
        v >>= 7;
    }
    out.extend(bytes.iter().rev());
}

fn chunk(id: &[u8], body: &[u8]) -> Vec<u8> {
    let mut v = id.to_vec();
    v.extend_from_slice(&(body.len() as u32).to_be_bytes());
    v.extend_from_slice(body);
    v
}

/// A minimal SFF2 style: a name, 120 BPM in 4/4, a voice per part, four-bar Mains and
/// one-bar Intro, Fills, Break and Ending with every part playing on beat 1 of each bar, and
/// four OTS. No CASM (the default rules apply).
pub(crate) fn style_bytes() -> Vec<u8> {
    // (absolute tick, event bytes), sorted (stably) by tick below.
    let mut evs: Vec<(u32, Vec<u8>)> = Vec::new();
    let text = |ty: u8, s: &str| {
        let mut e = vec![0xFF, ty];
        vlq(s.len() as u32, &mut e);
        e.extend_from_slice(s.as_bytes());
        e
    };
    evs.push((0, text(0x03, "Synthetic")));
    evs.push((0, vec![0xFF, 0x51, 3, 0x07, 0xA1, 0x20]));
    evs.push((0, vec![0xFF, 0x58, 4, 4, 2, 24, 8]));
    evs.push((0, text(0x06, "SFF2")));
    evs.push((0, text(0x06, "SInt")));
    for (i, &(msb, lsb, prog, _)) in PARTS.iter().enumerate() {
        let ch = 8 + i as u8;
        evs.push((0, vec![0xB0 | ch, 0, msb]));
        evs.push((0, vec![0xB0 | ch, 32, lsb]));
        evs.push((0, vec![0xC0 | ch, prog]));
        evs.push((0, vec![0xB0 | ch, 7, 100]));
    }
    // The sections start a bar after the SInt; every part plays on beat 1 of every bar.
    let mut start = BAR;
    for &(name, bars) in &SECTIONS {
        evs.push((start, text(0x06, name)));
        for bar in (0..bars).map(|b| start + b * BAR) {
            for (i, &(_, _, _, notes)) in PARTS.iter().enumerate() {
                let ch = 8 + i as u8;
                for &n in notes {
                    evs.push((bar, vec![0x90 | ch, n, 100]));
                    evs.push((bar + PPQ as u32, vec![0x80 | ch, n, 0]));
                }
            }
        }
        start += bars * BAR;
    }
    evs.push((start, vec![0xFF, 0x2F, 0]));
    evs.sort_by_key(|e| e.0);

    let mut trk = Vec::new();
    let mut now = 0;
    for (tick, e) in evs {
        vlq(tick - now, &mut trk);
        trk.extend(e);
        now = tick;
    }
    let mut out = chunk(b"MThd", &[0, 0, 0, 1, (PPQ >> 8) as u8, PPQ as u8]);
    out.extend(chunk(b"MTrk", &trk));
    out.extend(chunk(b"OTSc", &ots_bytes()));
    out
}

/// Four One Touch Settings: OTS n gives Right 1, Right 2, Right 3 and Left GM programs
/// 0, 48, 25 and 33 plus n; Right 1, Right 2 and Left on, Right 3 off.
fn ots_bytes() -> Vec<u8> {
    let mut out = Vec::new();
    for n in 0..4u8 {
        let mut trk = Vec::new();
        for (part, prog) in [0u8, 48, 25, 33].into_iter().enumerate() {
            let ch = part as u8;
            trk.extend_from_slice(&[0, 0xB0 | ch, 0, 0, 0, 0xB0 | ch, 32, 0, 0, 0xC0 | ch, prog + n]);
            let on = if part == 2 { 0x00 } else { 0x7F };
            trk.extend_from_slice(&[0, 0xF0, 9, 0x43, 0x73, 0x01, 0x50, 0x08, ch, 0x00, on, 0xF7]);
        }
        trk.extend_from_slice(&[0, 0xFF, 0x2F, 0]);
        out.extend(chunk(b"MTrk", &trk));
    }
    out
}

/// Write the synthetic style to `dir/styles/Synthetic.sty` (creating the folders) and return
/// its path.
pub(crate) fn write_style(dir: &Path) -> PathBuf {
    let p = dir.join("styles/Synthetic.sty");
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, style_bytes()).unwrap();
    p
}

/// The synthetic style, written once per test run to a shared temp folder.
pub(crate) fn style_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| write_style(&std::env::temp_dir().join(format!("yahaha-synthetic-{}", std::process::id()))))
        .clone()
}

/// Options for an offline session on the synthetic style.
pub(crate) fn options() -> Options {
    Options { paths: vec![style_path()], ..Options::default() }
}

/// An offline session on the synthetic style, with no data folder: for tests that need a
/// style loaded but not any real style's content.
pub(crate) fn session() -> Session {
    Session::offline(options()).unwrap()
}

/// A fresh data folder for test `test` (emptied first).
pub(crate) fn data_dir(test: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("yahaha-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

/// An offline session on the synthetic style with its data folder at `data`: for tests
/// that save or load records (racks, plugin presence).
pub(crate) fn session_in(data: &Path) -> Session {
    Session::offline(Options { paths: vec![write_style(data)], data_dir: Some(data.to_path_buf()), ..Options::default() })
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_synthetic_style_parses_with_every_section() {
        let s = crate::sff::parse(&style_bytes()).unwrap();
        assert_eq!(s.name, "Synthetic");
        assert_eq!(s.format, "SFF2");
        assert_eq!(s.sections.len(), SECTIONS.len());
        let main_a = s.sections.values().find(|x| x.id.name() == "Main A").unwrap();
        assert_eq!((main_a.len, main_a.events.len()), (4 * BAR, 4 * 2 * 13), "4 bars of 13 notes");
        assert_eq!((s.ppq, s.timesig), (PPQ, (4, 4)));
        assert!((s.bpm() - 120.0).abs() < 1e-6);
        assert_eq!(s.ots.len(), 4);
        let p = s.ots[1].parts;
        assert_eq!(p.map(|p| p.voice), [Some((0, 0, 1)), Some((0, 0, 49)), Some((0, 0, 26)), Some((0, 0, 34))]);
        assert_eq!(p.map(|p| p.on), [true, true, false, true]);
    }

    #[test]
    fn a_session_loads_it() {
        let s = session();
        let st = s.state();
        assert_eq!(st.style.name, "Synthetic");
    }
}
