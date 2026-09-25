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

/// The receiver's voice per channel after `out` (bank select + program change).
fn voices_after(out: &[(u64, Vec<u8>)]) -> [Option<(u8, u8, u8)>; 16] {
    let mut bank = [(0u8, 0u8); 16];
    let mut voice = [None; 16];
    for (_, m) in out {
        let c = (m[0] & 0x0F) as usize;
        match m[0] & 0xF0 {
            0xB0 if m[1] == 0 => bank[c].0 = m[2],
            0xB0 if m[1] == 32 => bank[c].1 = m[2],
            0xC0 => voice[c] = Some((bank[c].0, bank[c].1, m[1])),
            _ => {}
        }
    }
    voice
}

/// A style loaded while stopped sends the setup as the Main the band would start on
/// routes it, not as the section that played last does (review of #101, B2: after an
/// Ending, a stopped load sent the Ending's routing). Two paths: a load after the Ending
/// ended (`Engine::load`, what `change_style` does stopped), and a style change queued in
/// the Ending's last bar, which takes over as the Ending ends (`swap_style`). The load's
/// output must be exactly a fresh engine's load (after the expression resets, #122), every
/// Style part's expression is back to full, and after the swap the receiver has Main A's
/// voices.
/// Each channel's expression (CC11) after `out`: the last value sent, else full.
fn expression<'a>(out: impl Iterator<Item = &'a Vec<u8>>) -> [u8; 16] {
    let mut expr = [127u8; 16];
    for m in out {
        if m.len() == 3 && m[0] & 0xF0 == 0xB0 && m[1] == 11 {
            expr[(m[0] & 0x0F) as usize] = m[2];
        }
    }
    expr
}

#[test]
fn corpus_stopped_load_after_an_ending_sends_the_mains_setup() {
    let files = tests::corpus();
    if files.is_empty() {
        eprintln!("no corpus; skipping");
        return;
    }
    let (mut endings, mut rerouting, mut fails) = (0, 0, Vec::new());
    for f in &files {
        let style = Style::load(f).unwrap();
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        let p = Prepared::new(&style);
        let bar = bar_ns(&p);
        let main_a = p.setup_of[slot_of(SectionId::Main(0))];
        let home = p.setups[main_a as usize].voices;
        // What a fresh stopped engine sends when the style loads.
        let fresh = {
            let mut e = Engine::new(Box::new(Prepared::new(&style)));
            let mut rec = Recorder::default();
            e.load(Box::new(Prepared::new(&style)), 0, &mut rec);
            rec.out.into_iter().map(|(_, m)| m).collect::<Vec<_>>()
        };
        let fresh_expr = expression(fresh.iter());
        for i in 0..4u8 {
            let slot = slot_of(SectionId::Ending(i));
            if p.sections[slot].is_none() {
                continue;
            }
            endings += 1;
            rerouting += (p.setup_of[slot] != main_a) as usize;
            let script = reach(slot, bar);
            let end = 40 * bar;
            // (1) The Ending plays out; then the style loads, stopped.
            let (mut e, mut rec) = run(Box::new(Prepared::new(&style)), &script, end);
            assert!(!e.is_running(), "{name} Ending {i}: still running after 40 bars");
            let from = rec.out.len();
            rec.now = end;
            e.load(Box::new(Prepared::new(&style)), end, &mut rec);
            let mut sent: Vec<_> = rec.out[from..].iter().map(|(_, m)| m.clone()).collect();
            // Past the expression the Ending left (#122): the Ending's fade-out moves CC11,
            // so the load first puts it back to full; the rest is a fresh load's.
            let at = sent.iter().zip(&fresh).take_while(|(a, b)| a == b).count();
            let resets = sent[at..].iter().take_while(|m| m.len() == 3 && m[0] & 0xF8 == 0xB8 && m[1] == 11 && m[2] == 127).count();
            sent.drain(at..at + resets);
            let expr = expression(rec.out.iter().map(|(_, m)| m));
            if let Some(c) = (8..16).find(|&c| expr[c] != fresh_expr[c]) {
                fails.push(format!("{name} Ending {i} ch{}: expression {} after the load, {} fresh", c + 1, expr[c], fresh_expr[c]));
            }
            if sent != fresh {
                let differ = sent.iter().zip(&fresh).filter(|(a, b)| a != b).count() + sent.len().abs_diff(fresh.len());
                fails.push(format!("{name} Ending {i}: the stopped load sent {differ} messages unlike a fresh load's"));
            }
            // (2) A style change queued while the Ending plays takes over as it ends.
            let mut e = Engine::new(Box::new(Prepared::new(&style)));
            let mut rec = Recorder::default();
            let (mut k, mut now, mut queued) = (0, 0u64, false);
            while now < end {
                let t = [script.get(k).map(|s| s.0), e.next_deadline(), Some(end)].into_iter().flatten().min().unwrap();
                now = now.max(t);
                rec.now = now;
                while let Some((ts, step)) = script.get(k) {
                    if *ts > now {
                        break;
                    }
                    match step {
                        Step::Chord(c) => e.set_chord(*c, now, &mut rec),
                        Step::Button(b) => e.button(*b, now, &mut rec),
                        _ => unreachable!(),
                    }
                    k += 1;
                }
                e.process(now, &mut rec);
                while e.take_retired().is_some() {}
                // Queued again at each bar line the Ending plays on through, so the last
                // one waits for the bar line where it ends.
                if e.snapshot(now).cur == Some(SectionId::Ending(i)) && !e.style_pending() {
                    e.change_style(Box::new(Prepared::new(&style)), now, &mut rec);
                    queued = true;
                }
                if queued && !e.is_running() {
                    break;
                }
            }
            assert!(queued && !e.is_running(), "{name} Ending {i}: the change never took over");
            let got = voices_after(&rec.out);
            for c in 8..16 {
                if let Some(w) = home[c]
                    && got[c] != Some(w)
                {
                    fails.push(format!("{name} Ending {i} ch{}: after the swap {:?}, Main A routes {w:?}", c + 1, got[c]));
                }
            }
        }
    }
    let swaps = fails.iter().filter(|f| f.contains("after the swap")).count();
    eprintln!("{endings} Endings, {rerouting} routed unlike Main A; {} load and {swaps} swap failures", fails.len() - swaps);
    for f in fails.iter().take(30) {
        eprintln!("{f}");
    }
    assert!(rerouting >= 4, "{rerouting}: the test should see Endings routed unlike Main A");
    assert!(fails.is_empty(), "{} failures", fails.len());
}

/// Part levels follow each section's routing too (review of #101, N1): at the first note
/// each part plays in a section, its CC7 is the level the section's own routing of the
/// setup gives it (a source channel rerouted to the part brings its level with its
/// voice), unless the section's pattern sets its own CC7 on the part.
#[test]
fn corpus_sections_play_the_levels_they_route() {
    let files = tests::corpus();
    if files.is_empty() {
        eprintln!("no corpus; skipping");
        return;
    }
    let (mut checked, mut rerouted, mut fails) = (0, 0, Vec::new());
    for f in &files {
        let style = Style::load(f).unwrap();
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        let p = Prepared::new(&style);
        if p.setups.len() < 2 {
            continue;
        }
        let bar = bar_ns(&p);
        for slot in (0..NUM_SLOTS).filter(|&s| p.sections[s].is_some() && p.setup_of[s] != 0) {
            let own_cc7 = p.sections[slot].as_ref().unwrap().own_levels();
            let want = p.setups[p.setup_of[slot] as usize].mix;
            let main_a = p.setups[0].mix;
            let script = reach(slot, bar);
            let (_, rec) = run(Box::new(Prepared::new(&style)), &script, 3 * bar);
            let span = section_span(&style, &script, slot, 3 * bar);
            let mut level = [None; 16];
            let mut seen = 0u16;
            for (t, m) in &rec.out {
                let c = (m[0] & 0x0F) as usize;
                match m[0] & 0xF0 {
                    0xB0 if m[1] == 7 => level[c] = Some(m[2]),
                    0x90 if m[2] > 0 && (8..16).contains(&c) && span.is_some_and(|(s, e)| *t >= s && *t < e) && seen & (1 << c) == 0 => {
                        seen |= 1 << c;
                        if own_cc7 & (1 << c) != 0 {
                            continue;
                        }
                        let w = want[c - 8];
                        checked += 1;
                        rerouted += (main_a[c - 8] != w) as usize;
                        if level[c] != Some(w) {
                            fails.push(format!("{name} {:?} ch{}: level {:?}, the section routes {w} (Main A {})", id_of(slot), c + 1, level[c], main_a[c - 8]));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    eprintln!("{checked} part entries checked in rerouting sections, {rerouted} with a level Main A does not route there");
    for f in fails.iter().take(30) {
        eprintln!("{f}");
    }
    assert!(rerouted >= 20, "{rerouted}: the test should see parts whose level the routing changes");
    assert!(fails.is_empty(), "{} failures", fails.len());
}
