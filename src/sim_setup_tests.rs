//! The style's channel setup (SInt) routed by each section's own channel rules (#64).

use super::*;
use crate::engine::{id_of, slot_of, NUM_SLOTS};

fn bar_ns(p: &Prepared) -> u64 {
    (60e9 / p.bpm * (p.tpb as f64 / p.ppq as f64)) as u64
}

/// A script that plays section slot `slot`: started into (an Intro, a Main), or reached
/// from Main A (an Ending, a Fill, the Break) half a bar in.
fn reach(slot: usize, bar: u64) -> Vec<(u64, Step)> {
    let c = Step::Chord(Chord::new(0, 0));
    match id_of(slot) {
        SectionId::Intro(i) => vec![(0, Step::Button(Button::Intro(i))), (0, c)],
        SectionId::Main(i) => vec![(0, Step::Button(Button::Main(i))), (0, c)],
        SectionId::Ending(i) => vec![(0, c), (bar / 2, Step::Button(Button::Ending(i)))],
        SectionId::Fill(i) => vec![(0, Step::Button(Button::Main(i))), (0, c), (bar / 2, Step::Button(Button::Main(i)))],
        SectionId::Break => vec![(0, c), (bar / 2, Step::Button(Button::Break))],
    }
}

/// Every section of every corpus style: at the first note each part plays in the
/// section, the receiver has the voice the section's own routing of the setup gives the
/// part (unless the section's pattern sets its own voice). Where a section routes another
/// source channel to a part than Main A does, that is the other source's voice, not Main
/// A's (the review of #59 counted 48 such cases in 5 styles).
#[test]
fn corpus_sections_play_the_voices_they_route() {
    let files = tests::corpus();
    if files.is_empty() {
        eprintln!("no corpus; skipping");
        return;
    }
    let (mut checked, mut rerouted, mut styles) = (0, 0, 0);
    let mut fails = Vec::new();
    for f in &files {
        let style = Style::load(f).unwrap();
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        let p = Prepared::new(&style);
        let bar = bar_ns(&p);
        styles += (p.setups.len() > 1) as usize;
        for slot in (0..NUM_SLOTS).filter(|&s| p.sections[s].is_some()) {
            let own = p.sections[slot].as_ref().unwrap().own_programs();
            let want = p.setups[p.setup_of[slot] as usize].voices;
            let main_a = p.setups[0].voices;
            let script = reach(slot, bar);
            let (_, rec) = run(Box::new(Prepared::new(&style)), &script, 3 * bar);
            // Receiver model: bank and program per channel.
            let mut bank = [(0u8, 0u8); 16];
            let mut voice: [Option<(u8, u8, u8)>; 16] = [None; 16];
            let mut seen = 0u16;
            let span = section_span(&style, &script, slot, 3 * bar);
            for (t, m) in &rec.out {
                let c = (m[0] & 0x0F) as usize;
                match m[0] & 0xF0 {
                    0xB0 if m[1] == 0 => bank[c].0 = m[2],
                    0xB0 if m[1] == 32 => bank[c].1 = m[2],
                    0xC0 => voice[c] = Some((bank[c].0, bank[c].1, m[1])),
                    0x90 if m[2] > 0 && (8..16).contains(&c) && span.is_some_and(|(s, e)| *t >= s && *t < e) && seen & (1 << c) == 0 => {
                        seen |= 1 << c;
                        let Some(w) = want[c] else { continue };
                        if own & (1 << c) != 0 {
                            continue;
                        }
                        checked += 1;
                        rerouted += (main_a[c] != Some(w)) as usize;
                        if voice[c] != Some(w) {
                            fails.push(format!("{name} {:?} ch{}: voice {:?}, the section routes {w:?} (Main A {:?})", id_of(slot), c + 1, voice[c], main_a[c]));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    eprintln!("{checked} part entries checked, {rerouted} with a voice Main A does not route there, in {styles} styles with more than one routing");
    for f in fails.iter().take(30) {
        eprintln!("{f}");
    }
    assert!(checked > 1000, "{checked} checked");
    assert!(rerouted >= 20, "{rerouted} rerouted parts: the test should see the #64 cases");
    assert!(fails.is_empty(), "{} failures", fails.len());
}

/// When section slot `slot` plays under `script` (the first time): from, to.
fn section_span(style: &Style, script: &[(u64, Step)], slot: usize, end: u64) -> Option<(u64, u64)> {
    let (mut from, mut to) = (None, end);
    run_observed(Box::new(Prepared::new(style)), script, end, |e, now| {
        let here = e.snapshot(now).cur == Some(id_of(slot));
        match from {
            None if here => from = Some(now),
            Some(_) if !here && to == end => to = now,
            _ => {}
        }
    });
    Some((from?, to))
}

/// Main A's routing is the first setup, and every section has one.
#[test]
fn one_setup_per_routing() {
    let files = tests::corpus();
    for f in files.iter().take(40) {
        let p = Prepared::new(&Style::load(f).unwrap());
        assert!(!p.setups.is_empty());
        assert_eq!(p.setup_of[slot_of(SectionId::Main(0))], 0, "Main A's routing is the first");
        for (slot, sec) in p.sections.iter().enumerate() {
            if sec.is_none() {
                continue;
            }
            assert!((p.setup_of[slot] as usize) < p.setups.len());
        }
    }
}
