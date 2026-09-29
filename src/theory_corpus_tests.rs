//! Tests of `theory` moved here from yahaha-core because they need a higher layer (the corpus loader in `library`).

#[cfg(feature = "slow-tests")]
use crate::style_types::{ChannelRule, Ntr, Ntt};
#[cfg(feature = "slow-tests")]
use crate::theory::*;
#[cfg(feature = "slow-tests")]
use crate::theory::{mask_of, rot, LOW_E};

#[cfg(feature = "slow-tests")]
fn every_chord(rule: &ChannelRule) -> impl Iterator<Item = Chord> {
    let plain = (0..TYPE_NAMES.len() as u8).flat_map(|ty| (0..12).map(move |root| Chord::new(root, ty)));
    let slash = (0..12).flat_map(|b| {
        [(0u8, 0u8), (9, 10), (7, 19), (2, 18), (0, M7B5), (5, FLAT5), (9, MM7B5)]
            .map(|(root, ty)| Chord { root, ty, bass: Some(b) })
    });
    plain.chain(slash).chain([Chord::new(rule.src_root, rule.src_type)])
}

/// Everything the engine does with a rule and a chord, over every key.
#[cfg(feature = "slow-tests")]
fn exercise(rule: &ChannelRule) {
    for c in every_chord(rule) {
        let p = plays(rule, c);
        if c.ty == CANCEL {
            assert_eq!(p, is_drum_part(rule.dest_ch));
        }
        // A display-only type is a valid chord: it plays exactly as its CASM type.
        let display_only = matches!(c.ty, M7B5 | FLAT5 | MM7B5);
        if display_only {
            assert_eq!(p, plays(rule, c.casm()), "{c:?}");
        }
        for k in 0..=127u8 {
            let n = transpose(k, rule, c);
            assert!(n.is_none_or(|n| n <= 127));
            if display_only {
                assert_eq!(n, transpose(k, rule, c.casm()), "{c:?} key {k}");
            }
        }
        for base in (0..=120u8).step_by(12) {
            let keys = [base, base + 4, base + 7];
            let mut out = [None; 3];
            transpose_group(&keys, rule, c, &mut out);
        }
    }
}

/// Every channel rule of every corpus style (plus the defaults used for channels without
/// CASM) transposes every key under every chord without panicking.
#[cfg(feature = "slow-tests")]
#[test]
fn corpus_every_rule_every_chord() {
    let files = crate::library::corpus_loaded();
    if files.is_empty() {
        eprintln!("no corpus; skipping");
        return;
    }
    // Most styles share their rules, so each distinct one is exercised once. The fields
    // neither `plays` nor `transpose` reads (source channel, name, flags) are left out of
    // the key.
    let key = |r: &ChannelRule| ChannelRule { src_ch: 0, name: String::new(), editable: false, autostart: false, sff2: false, ..r.clone() };
    let mut rules: std::collections::HashSet<ChannelRule> = (8..16).map(|ch| key(&ChannelRule::default_for(ch))).collect();
    let mut total = 8;
    for (f, s) in files {
        for r in s.casm.iter().flat_map(|seg| &seg.rules) {
            assert!((r.src_type as usize) < NUM_TYPES, "{}: src_type {}", f.display(), r.src_type);
            rules.insert(key(r));
            total += 1;
        }
    }
    eprintln!("{} styles, {total} channel rules, {} distinct", files.len(), rules.len());
    assert!(total > 8 && !rules.is_empty(), "no corpus channel rules to exercise");
    for r in &rules {
        exercise(r);
    }
}

#[cfg(feature = "slow-tests")]
fn every_target() -> impl Iterator<Item = Chord> {
    (0..12).flat_map(|root| (0..NUM_TYPES as u8).map(move |ty| Chord::new(root, ty)))
}

/// Every Guitar channel of every corpus style (all file types): noise keys come back
/// untouched, and under every chord the channel plays, every other note is a chord tone
/// (or the slash bass) on the neck, and Stroke alone leaves strings out. It also counts
/// how the corpus writes its Guitar parts, for the PR's evidence (counts only).
#[cfg(feature = "slow-tests")]
#[test]
fn corpus_guitar_parts_play_chord_tones() {
    let files = crate::library::corpus_loaded();
    let corpus = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
    let (mut notes, mut noise, mut records, mut styles) = (0u32, 0u32, 0u32, 0u32);
    // Per top folder of the corpus: strums of 3+ strings, and those written as stacked seconds.
    let mut strums: std::collections::BTreeMap<String, (u32, u32)> = Default::default();
    for (f, s) in files {
        let folder = f.strip_prefix(&corpus).ok().and_then(|p| p.components().next()).map(|c| c.as_os_str().to_string_lossy().to_string());
        let tally = strums.entry(folder.unwrap_or_default()).or_default();
        let mut any = false;
        for seg in &s.casm {
            for r in seg.rules.iter().filter(|r| r.zones.iter().any(|z| z.ntr == Ntr::Guitar)) {
                any = true;
                records += 1;
                let mut keys = [false; 128];
                for id in seg.sections.iter().filter_map(|n| crate::style_types::SectionId::parse(n)) {
                    let Some(sec) = s.sections.get(&id) else { continue };
                    let mut strum: Vec<(u32, u8)> = Vec::new();
                    for ev in &sec.events {
                        let crate::style_types::Ev::NoteOn { ch, key, vel } = ev.ev else { continue };
                        if ch != r.src_ch || vel == 0 || r.zone_for(key).ntr != Ntr::Guitar {
                            continue;
                        }
                        notes += 1;
                        noise += (key >= GUITAR_NOISE) as u32;
                        keys[key as usize] = true;
                        if key >= GUITAR_NOISE {
                            continue;
                        }
                        // A strum: notes at most 30 ticks apart. Count the ones of three
                        // or more strings written as two or more stacked seconds.
                        if strum.last().is_some_and(|&(t, _)| ev.tick - t > 30) {
                            tally.0 += (strum.len() >= 3) as u32;
                            tally.1 += (strum.len() >= 3 && stacked_seconds(&strum) >= 2) as u32;
                            strum.clear();
                        }
                        strum.push((ev.tick, key));
                    }
                    tally.0 += (strum.len() >= 3) as u32;
                    tally.1 += (strum.len() >= 3 && stacked_seconds(&strum) >= 2) as u32;
                }
                for c in every_target().filter(|&c| plays(r, c)) {
                    for k in (0..128u8).filter(|&k| keys[k as usize] && r.zone_for(k).ntr == Ntr::Guitar) {
                        let z = r.zone_for(k);
                        let out = transpose(k, r, c);
                        if k >= GUITAR_NOISE {
                            assert_eq!(out, Some(k), "{} noise key {k}", f.display());
                            continue;
                        }
                        let Some(n) = out else {
                            assert_eq!(z.ntt, Ntt::GuitarStroke, "{} {} key {k}", f.display(), c.name());
                            continue;
                        };
                        let tones = rot(mask_of(c.ty), c.root);
                        assert!(tones & 1 << (n % 12) != 0, "{} {} key {k}: {n}", f.display(), c.name());
                        assert!(n >= LOW_E.min(z.lo) && n <= z.hi.max(LOW_E), "{} {} key {k}: {n}", f.display(), c.name());
                    }
                }
            }
        }
        styles += any as u32;
    }
    eprintln!("{styles} styles, {records} Guitar rules, {notes} Guitar notes ({noise} noise keys)");
    for (folder, (all, stacked)) in strums.iter().filter(|(_, t)| t.0 > 0) {
        eprintln!("  {folder}: {stacked} of {all} strums of 3+ strings are stacked seconds");
    }
    assert!(files.is_empty() || notes > 0, "corpus has no Guitar parts");
}

/// Adjacent distinct keys of a strum at most two semitones apart.
#[cfg(feature = "slow-tests")]
fn stacked_seconds(strum: &[(u32, u8)]) -> usize {
    let mut k: Vec<u8> = strum.iter().map(|x| x.1).collect();
    k.sort_unstable();
    k.dedup();
    k.windows(2).filter(|w| w[1] - w[0] <= 2).count()
}
