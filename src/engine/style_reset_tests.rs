//! A style change puts the Style parts' voice settings back (#253): the sound controllers
//! (CC71-78), portamento and mono/poly a style left on its parts don't carry into the next
//! style, as a Genos style's own system reset sees to. The keyboard parts keep theirs.

use super::*;

const MS: u64 = 1_000_000;

#[derive(Default)]
struct Rec(Vec<Vec<u8>>);

impl Sink for Rec {
    fn send(&mut self, m: &[u8]) {
        self.0.push(m.to_vec());
    }
}

fn prepared(rel: &str) -> Option<Box<Prepared>> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus").join(rel);
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
fn after_sweeps(next: &str) -> Option<(Engine, Rec)> {
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
    let left: Vec<usize> = (8..16).filter(|&c| !matches!(e.mirror.cc[c][74], 64 | UNSENT)).collect();
    assert!(!left.is_empty(), "the sweeping style leaves CC74 off 64 somewhere");
    let mut load = Rec::default();
    e.change_style(prepared(next)?, now, &mut load);
    Some((e, load))
}

#[test]
fn a_new_style_puts_the_style_parts_voice_settings_back() {
    let Some((e, load)) = after_sweeps(PLAIN) else { return };
    let sent = ccs(&load);
    for ch in 8..16u8 {
        // Whatever the old style left, the new one (which sets none) starts at the defaults.
        assert!(matches!(e.mirror.cc[ch as usize][74], 64 | UNSENT), "ch {}: CC74 {}", ch + 1, e.mirror.cc[ch as usize][74]);
        for cc in (71..=78).chain([65, 5]) {
            let v = e.mirror.cc[ch as usize][cc];
            assert!(v == UNSENT || v == if cc >= 71 { 64 } else { 0 }, "ch {}: CC{cc} {v}", ch + 1);
        }
    }
    assert!(sent.iter().any(|&(ch, cc, v)| ch >= 8 && cc == 74 && v == 64), "CC74 64 went out: {sent:?}");
    // Only the Style parts: nothing on the keyboard parts or the Multi Pads, and no note
    // cut (no All Notes Off / Sound Off / Mono / Poly controller).
    assert!(sent.iter().all(|&(ch, cc, _)| ch >= 8 || !(71..=78).contains(&cc) && cc != 65 && cc != 5), "keyboard parts untouched: {sent:?}");
    assert!(sent.iter().all(|&(_, cc, _)| !matches!(cc, 120 | 123 | 126 | 127)), "no note cut: {sent:?}");
}

/// A load after a style that set nothing sends no reset at all.
#[test]
fn nothing_to_put_back_sends_nothing() {
    let Some(first) = prepared(PLAIN) else { return };
    let mut e = Engine::new(first);
    let mut r = Rec::default();
    e.set_chord(Chord::new(0, 0), 0, &mut r);
    let mut now = 0;
    while now <= e.ns_at_bar(1) {
        e.process(now, &mut r);
        now += 5 * MS;
    }
    e.stop(&mut r);
    let mut load = Rec::default();
    e.change_style(prepared(PLAIN).unwrap(), now, &mut load);
    let resets: Vec<_> = ccs(&load).into_iter().filter(|&(_, cc, _)| (71..=78).contains(&cc) || cc == 65 || cc == 5).collect();
    assert_eq!(resets, vec![], "no voice setting to put back");
    assert!(load.0.iter().all(|m| !(m.len() == 9 && m[4] == 0x08 && m[6] == 0x05)), "no Poly");
}

/// A Style part left in mono (CC126, or the XG part parameter) goes back to poly with the
/// XG part parameter, which cuts no note; portamento goes off.
#[test]
fn mono_and_portamento_go_back() {
    let Some(first) = prepared(PLAIN) else { return };
    let mut e = Engine::new(first);
    let mut r = Rec::default();
    e.send_init(&mut r);
    e.mirror.send(&mut r, &[0xBA, 126, 1]);
    e.mirror.send(&mut r, &[0xBB, 65, 127]);
    e.mirror.send(&mut r, &[0xBB, 5, 40]);
    e.mirror.track(&[0xF0, 0x43, 0x10, 0x4C, 0x08, 0x0C, 0x05, 0x00, 0xF7]);
    assert_eq!(e.mirror.mono, 1 << 10 | 1 << 12);
    let mut load = Rec::default();
    e.change_style(prepared(PLAIN).unwrap(), 0, &mut load);
    let poly = |ch: u8| vec![0xF0, 0x43, 0x10, 0x4C, 0x08, ch, 0x05, 0x01, 0xF7];
    assert!(load.0.contains(&poly(10)) && load.0.contains(&poly(12)), "poly again: {:?}", load.0);
    assert_eq!(e.mirror.mono, 0);
    let sent = ccs(&load);
    assert!(sent.contains(&(11, 65, 0)) && sent.contains(&(11, 5, 0)), "portamento off: {sent:?}");
}

/// After a style preview (anything may have been sent on the Style channels) the resync
/// puts the voice settings back too.
#[test]
fn a_resync_puts_them_back() {
    let Some(first) = prepared(PLAIN) else { return };
    let mut e = Engine::new(first);
    let mut r = Rec::default();
    e.resync(&mut r);
    let sent = ccs(&r);
    for ch in 8..16u8 {
        assert!(sent.contains(&(ch, 74, 64)) && sent.contains(&(ch, 65, 0)), "ch {}", ch + 1);
        assert!(r.0.contains(&vec![0xF0, 0x43, 0x10, 0x4C, 0x08, ch, 0x05, 0x01, 0xF7]), "ch {} poly", ch + 1);
    }
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
    let Some((_, load)) = after_sweeps(PLAIN) else { return };
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
        crate::synth::render_offline(&sf2, &msgs, 2000 * MS, 48_000, 120.0).unwrap()
    };
    let with = render(&tone, true);
    assert!(with.0.iter().any(|v| v.abs() > 1e-3), "the part sounds");
    assert!(with == render(&tone, false), "the style change leaves the keyboard part as it was");
    assert!(with != render(&tone[..1], true), "(its own settings do change how it sounds)");
}
