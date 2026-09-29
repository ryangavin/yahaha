//! Engine tests that need the synth: they render the engine's output, so they can't live in
//! `yahaha-engine`, which sits below `synth`. They reach the engine through its public API.

use crate::engine::{Engine, Prepared, Sink};
use crate::sff::Style;
use crate::theory::Chord;

const MS: u64 = 1_000_000;

#[derive(Default)]
struct Rec(Vec<Vec<u8>>);

impl Sink for Rec {
    fn send(&mut self, m: &[u8]) {
        self.0.push(m.to_vec());
    }
}

fn prepared(rel: &str) -> Option<Box<Prepared>> {
    let p = crate::library::corpus_dir().join(rel);
    if !p.exists() {
        eprintln!("corpus missing; skipping");
        return None;
    }
    Some(Box::new(Prepared::new(&Style::load(&p).unwrap())))
}

/// 90sDancePop's Main A moves CC74 on its parts; ClubJazz1 sets no voice settings.
const SWEEPS: &str = "SX900Style for Genos/90sDancePop.T547.prs";
const PLAIN: &str = "MOX_v2/ClubJazz1.S930.STY";

/// Every controller (channel, cc, value) `r` sent, in order.
fn ccs(r: &Rec) -> Vec<(u8, u8, u8)> {
    r.0.iter().filter(|m| m.len() == 3 && m[0] & 0xF0 == 0xB0).map(|m| (m[0] & 0x0F, m[1], m[2])).collect()
}

/// The sweeping style played for two bars, stopped, then `next` loaded: what the load sent.
fn after_sweeps(next: &str) -> Option<Rec> {
    let mut e = Engine::new(prepared(SWEEPS)?);
    let mut r = Rec::default();
    e.set_chord(Chord::new(0, 0), 0, &mut r);
    let two_bars = e.ns_at_bar(2);
    let mut now = 0;
    while now <= two_bars {
        e.process(now, &mut r);
        now += 5 * MS;
    }
    e.stop(&mut r);
    // The last CC74 each Style channel was sent (the engine's own mirror isn't public).
    let mut last = [None; 16];
    for (ch, cc, v) in ccs(&r) {
        if cc == 74 {
            last[ch as usize] = Some(v);
        }
    }
    assert!(last[8..16].iter().any(|v| matches!(v, Some(v) if *v != 64)), "the sweeping style leaves CC74 off 64 somewhere");
    let mut load = Rec::default();
    e.change_style(prepared(next)?, now, &mut load);
    Some(load)
}

/// Rendered: a keyboard part with voice settings of its own (an OTS's filter and release,
/// Mono) sounds exactly the same after a style change that puts the Style parts back.
#[test]
fn the_keyboard_parts_sound_the_same_after_a_style_change() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("soundfonts");
    let Some(sf2) = crate::library::sound_font_files(&dir).into_iter().map(|f| dir.join(f)).min_by_key(|p| p.metadata().map(|m| m.len()).unwrap_or(u64::MAX)) else {
        eprintln!("no SoundFont; skipping");
        return;
    };
    let Some(load) = after_sweeps(PLAIN) else { return };
    assert!(ccs(&load).iter().any(|&(ch, cc, v)| ch >= 8 && cc == 74 && v == 64), "the load resets something");
    // Right 1 (channel 1): a flute with an OTS's cutoff, release and Mono, two notes.
    let tone: Vec<Vec<u8>> = vec![vec![0xC0, 73], vec![0xB0, 74, 30], vec![0xB0, 72, 90], vec![0xF0, 0x43, 0x10, 0x4C, 0x08, 0x00, 0x05, 0x00, 0xF7]];
    let notes = [(100 * MS, vec![0x90, 60, 100]), (400 * MS, vec![0x90, 67, 100]), (900 * MS, vec![0x80, 67, 0]), (1200 * MS, vec![0x80, 60, 0])];
    let render = |tone: &[Vec<u8>], style_change: bool| {
        let mut msgs: Vec<(u64, Vec<u8>)> = tone.iter().map(|m| (0, m.clone())).collect();
        if style_change {
            msgs.extend(load.0.iter().map(|m| (50 * MS, m.clone())));
        }
        msgs.extend(notes.iter().cloned());
        crate::synth::render_offline(&sf2, &msgs, 2000 * MS, 48_000, 120.0, &[]).unwrap()
    };
    let with = render(&tone, true);
    assert!(with.0.iter().any(|v| v.abs() > 1e-3), "the part sounds");
    assert!(with == render(&tone, false), "the style change leaves the keyboard part as it was");
    assert!(with != render(&tone[..1], true), "(its own settings do change how it sounds)");
}
