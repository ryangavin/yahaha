//! Chord settling (engine/settle.rs): a chord and a command in one engine wake (#47), and
//! rolled chords under the chord-settle window (#65).

use super::*;
use crate::theory::{is_drum_part, CANCEL};

fn bar_ns(p: &Prepared) -> u64 {
    (60e9 / p.bpm * (p.tpb as f64 / p.ppq as f64)) as u64
}

fn step_name(s: &Step) -> String {
    match s {
        Step::Chord(c) => format!("chord {}/{}", c.root, c.ty),
        Step::Release => "release".into(),
        Step::Button(b) => format!("{b:?}"),
        Step::ManualBass(b) => format!("mb {b}"),
        Step::Transpose(t) => format!("tr {}", t.keyboard),
    }
}

/// `Recorder::pitch_ons` without guitar noise keys (MegaVoice): they are not pitches, and
/// sound with whatever bend the part has.
fn pitched_ons(rec: &Recorder) -> Vec<(u64, u8, u8)> {
    let sent = rec.out.iter().filter(|(_, m)| m[0] & 0xF0 == 0x90 && m[2] > 0).map(|(_, m)| m[1]);
    rec.pitch_ons().into_iter().zip(sent).filter(|&(_, key)| key < crate::theory::GUITAR_NOISE).map(|(n, _)| n).collect()
}

fn follows(ch: u8) -> bool {
    (8..16).contains(&ch) && !is_drum_part(ch)
}

/// Run `script` against a fresh engine with a chord-settle window of `settle` ns. Steps at
/// one instant run in script order, then `process`: one engine wake, as `EngineLoop::step`.
fn drive(style: &Style, settle: u64, script: &[(u64, Step)], end: u64) -> (Engine, Recorder) {
    let mut e = Engine::new(Box::new(Prepared::new(style)));
    e.set_chord_settle(settle);
    let mut rec = Recorder::default();
    let mut i = 0;
    let mut now = 0u64;
    loop {
        let t = [script.get(i).map(|s| s.0), e.next_deadline(), Some(end)].into_iter().flatten().min().unwrap();
        now = now.max(t);
        rec.now = now;
        while let Some((ts, step)) = script.get(i) {
            if *ts > now {
                break;
            }
            match step {
                Step::Chord(c) => e.set_chord(*c, now, &mut rec),
                Step::Release => e.chord_released(now, &mut rec),
                Step::Button(b) => e.button(*b, now, &mut rec),
                Step::ManualBass(on) => e.set_manual_bass(*on, &mut rec),
                Step::Transpose(t) => e.set_transpose(*t, now, &mut rec),
            }
            i += 1;
        }
        e.process(now, &mut rec);
        if now >= end {
            break;
        }
    }
    (e, rec)
}

/// The notes on the parts that follow chords, as (channel, key, start, end), and the
/// problems: a key started twice, a note-off with no note, a note still sounding at the end.
fn notes(rec: &Recorder) -> (Vec<(u8, u8, u64, u64)>, Vec<String>) {
    let mut on = std::collections::HashMap::<(u8, u8), u64>::new();
    let mut v = Vec::new();
    let mut bad = Vec::new();
    for (t, m) in &rec.out {
        let ch = m[0] & 0x0F;
        if !follows(ch) || m[0] & 0xE0 != 0x80 {
            continue;
        }
        if m[0] & 0xF0 == 0x90 && m[2] > 0 {
            if on.insert((ch, m[1]), *t).is_some() {
                bad.push(format!("ch{} key {} started twice at {t}", ch + 1, m[1]));
            }
        } else {
            match on.remove(&(ch, m[1])) {
                Some(s) => v.push((ch, m[1], s, *t)),
                None => bad.push(format!("ch{} key {} off with no note at {t}", ch + 1, m[1])),
            }
        }
    }
    if !on.is_empty() {
        bad.push(format!("stuck {on:?}"));
    }
    (v, bad)
}

/// A chord and a command handled in one engine wake (the chord word is read first, then
/// the commands: `EngineLoop::step`), at uneven moments around the beat, through every
/// corpus style: no note on a chord part lasts zero time, no key starts twice, nothing is
/// stuck. The #63 review's probe (chord + Keyboard transpose at one instant) found
/// thousands of zero-length notes before chords settled in `process`. With the window at 0
/// (the engine's default), so it is the wake that coalesces them, not the window.
#[test]
fn corpus_chord_and_command_in_one_wake_leave_no_blips() {
    let files = tests::corpus();
    if files.is_empty() {
        eprintln!("no corpus; skipping");
        return;
    }
    let mut changes = 0;
    let mut fails = Vec::new();
    for (fi, f) in files.iter().enumerate() {
        let style = Style::load(f).unwrap();
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        let bar = bar_ns(&Prepared::new(&style));
        let mut x = fi as u64 * 7919 + 1;
        let mut rand = |n: u64| {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (x >> 33) % n
        };
        let mut s = vec![(0, Step::Chord(Chord::new(0, 0)))];
        let mut kbd = 0i8;
        for i in 1..24u64 {
            let base = i * bar / 2;
            let t = match i % 4 {
                0 => base,
                1 => base - 1_000_003,
                2 => base + 3_000_017,
                _ => base + rand(bar / 2),
            };
            let chord = Step::Chord(Chord { root: rand(12) as u8, ty: rand(34) as u8, bass: (rand(4) == 0).then(|| rand(12) as u8) });
            let cmd = match rand(9) {
                0 | 1 | 2 => {
                    kbd = (kbd + 1 + rand(5) as i8).rem_euclid(7) - 3;
                    Step::Transpose(Transpose::new(kbd, 0))
                }
                3 => Step::Button(Button::TogglePart(2 + rand(6) as u8)),
                4 => Step::Button(Button::Break),
                5 => Step::Button(Button::Main(rand(4) as u8)),
                6 => Step::Button(Button::StopAcmp),
                7 => Step::Chord(Chord::new(0, CANCEL)),
                _ => Step::Button(Button::StartStop),
            };
            // The live order is chord first; a transpose may also come first (the input
            // ring before the chord of the next wake is not the same wake, but a script
            // may do it).
            if matches!(cmd, Step::Transpose(_)) && rand(2) == 0 {
                s.push((t, cmd));
                s.push((t, chord));
            } else {
                s.push((t, chord));
                s.push((t, cmd));
            }
            // A band stopped by StartStop starts again on the next chord.
            if matches!(cmd, Step::Button(Button::StartStop)) {
                s.push((t + bar / 8, Step::Button(Button::SyncStart)));
            }
            changes += 1;
        }
        s.push((13 * bar, Step::Button(Button::Stop)));
        s.sort_by_key(|x| x.0);
        let (e, rec) = drive(&style, 0, &s, 13 * bar + 1);
        if e.snapshot(13 * bar + 1).stop_acmp {
            continue; // Stop Accompaniment holds its chord while stopped, by design
        }
        let (v, bad) = notes(&rec);
        fails.extend(bad.into_iter().map(|b| format!("{name}: {b}")));
        for (ch, key, st, t) in v {
            if t <= st {
                let near: Vec<String> = s
                    .iter()
                    .filter(|x| x.0 + 50_000_000 >= st && x.0 <= st + 50_000_000)
                    .map(|x| format!("{:+}ms {}", (x.0 as i64 - st as i64) / 1_000_000, step_name(&x.1)))
                    .collect();
                let bar_pos = st as f64 / bar as f64;
                if std::env::var("SETTLE_DEBUG").is_ok_and(|v| name.starts_with(&v)) {
                    for (t, m) in rec.out.iter().filter(|(t, _)| *t + 2_000_000 >= st && *t <= st + 2_000_000) {
                        eprintln!("  {t} {m:02X?}");
                    }
                }
                fails.push(format!("{name}: ch{} key {key} zero-length at {st} (bar {bar_pos:.3}) near {near:?}", ch + 1));
            }
        }
    }
    for f in fails.iter().take(30) {
        eprintln!("{f}");
    }
    assert!(changes > 1000, "{changes} changes");
    assert!(fails.is_empty(), "{} failures", fails.len());
}

/// A rolled chord: F, then F7 3 ms later, every half bar at uneven moments, through every
/// corpus style. With no window, notes attacked on the passing F are cut and struck again
/// 3 ms later (#65: the review counted 188 in 114 styles). With the default window, no
/// note on a chord part starts in the roll and ends by the time the chord settled, and
/// none waits longer than the window allows.
#[test]
fn corpus_rolled_chords_settle_once() {
    let files = tests::corpus();
    if files.is_empty() {
        eprintln!("no corpus; skipping");
        return;
    }
    let window = crate::engine::CHORD_SETTLE_DEFAULT_MS as u64 * 1_000_000;
    let roll = 3_000_000;
    let mut blips = [0usize; 2];
    let mut styles = [0usize; 2];
    let mut fails = Vec::new();
    let mut held = 0;
    for f in &files {
        let style = Style::load(f).unwrap();
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        let bar = bar_ns(&Prepared::new(&style));
        let mut s = vec![(0, Step::Chord(Chord::new(0, 0)))];
        let mut rolls = Vec::new();
        let pairs = [(5, 0, 5, 10), (7, 0, 7, 10), (9, 8, 9, 13), (2, 8, 2, 13), (0, 0, 0, 1)];
        for i in 1..20u64 {
            let t = i * bar / 2 + [0, bar / 7, bar / 3, 2 * bar / 5][i as usize % 4] - [0, 1_000_003][i as usize % 2];
            let (r1, t1, r2, t2) = pairs[i as usize % pairs.len()];
            s.push((t, Step::Chord(Chord::new(r1, t1))));
            s.push((t + roll, Step::Chord(Chord::new(r2, t2))));
            rolls.push(t);
        }
        s.push((11 * bar, Step::Button(Button::Stop)));
        for (k, settle) in [0, window].into_iter().enumerate() {
            let (_, rec) = drive(&style, settle, &s, 11 * bar + 1);
            let (v, bad) = notes(&rec);
            fails.extend(bad.into_iter().map(|b| format!("{name}/{settle}: {b}")));
            let settled = roll + settle;
            let mut n = 0;
            for &(ch, key, start, end) in &v {
                let Some(&t) = rolls.iter().find(|&&t| start >= t && start <= t + settled) else { continue };
                if end < t + settled + 1_000_000 {
                    n += 1;
                    if settle > 0 {
                        fails.push(format!("{name}: ch{} key {key} {} ms into a roll lasts {} ms", ch + 1, (start - t) / 1_000_000, (end - start) / 1_000_000));
                    }
                }
                // Held back no longer than the window after the roll's last change.
                if settle > 0 && start > t && start != t + settled && start - t < settled {
                    fails.push(format!("{name}: ch{} key {key} started {} ns into the roll", ch + 1, start - t));
                }
                held += (settle > 0 && start == t + settled) as usize;
            }
            blips[k] += n;
            styles[k] += (n > 0) as usize;
        }
    }
    eprintln!("roll blips: window 0: {} in {} styles; window {} ms: {} in {} styles; {held} notes started at the settle",
              blips[0], styles[0], window / 1_000_000, blips[1], styles[1]);
    for f in fails.iter().take(30) {
        eprintln!("{f}");
    }
    assert!(blips[0] > 100, "the rolls should blip without a window: {}", blips[0]);
    assert!(held > 1000, "{held} notes held back");
    assert!(fails.is_empty(), "{} failures", fails.len());
}

/// A chord struck a window or more ahead of the beat costs nothing: the notes the pattern
/// starts on the beat sound the same pitches, at the same time, with the window as without.
#[test]
fn corpus_chord_ahead_of_the_beat_is_not_delayed() {
    let files = tests::corpus();
    if files.is_empty() {
        eprintln!("no corpus; skipping");
        return;
    }
    let window = crate::engine::CHORD_SETTLE_DEFAULT_MS as u64 * 1_000_000;
    let mut compared = 0;
    for f in &files {
        let style = Style::load(f).unwrap();
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        let bar = bar_ns(&Prepared::new(&style));
        let chords = [Chord::new(5, 0), Chord::new(7, 10), Chord::new(9, 8), Chord::new(2, 13), Chord::new(0, 1)];
        let mut s = vec![(0, Step::Chord(Chord::new(0, 0)))];
        let beats: Vec<u64> = (1..16u64).map(|i| i * bar / 2).collect();
        for (i, &b) in beats.iter().enumerate() {
            s.push((b - window - 5_000_000, Step::Chord(chords[i % chords.len()])));
        }
        s.push((8 * bar + bar / 3, Step::Button(Button::Stop)));
        let on_beats = |rec: &Recorder| {
            let mut v: Vec<(u64, u8, u8)> = pitched_ons(rec).into_iter().filter(|n| follows(n.1) && beats.binary_search(&n.0).is_ok()).collect();
            v.sort();
            v
        };
        let a = on_beats(&drive(&style, 0, &s, 9 * bar).1);
        let b = on_beats(&drive(&style, window, &s, 9 * bar).1);
        assert_eq!(a, b, "{name}");
        compared += a.len();
    }
    assert!(compared > 1000, "{compared} notes compared");
}

/// A Sync Start chord under the window, through every corpus style: the rhythm parts play
/// exactly as with no window; the chord parts' notes due before the settle start at the
/// settle, in the chord, with the pitches they have with no window; later ones are
/// untouched.
#[test]
fn corpus_sync_start_under_the_window() {
    let files = tests::corpus();
    if files.is_empty() {
        eprintln!("no corpus; skipping");
        return;
    }
    let window = crate::engine::CHORD_SETTLE_DEFAULT_MS as u64 * 1_000_000;
    let mut held = 0;
    for f in &files {
        let style = Style::load(f).unwrap();
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        let bar = bar_ns(&Prepared::new(&style));
        let s = [(0, Step::Chord(Chord::new(5, 0)))];
        let a = pitched_ons(&drive(&style, 0, &s, bar).1);
        let b = pitched_ons(&drive(&style, window, &s, bar).1);
        let part = |v: &[(u64, u8, u8)], chord: bool| {
            let mut v: Vec<_> = v.iter().copied().filter(|n| follows(n.1) == chord && (8..16).contains(&n.1)).collect();
            v.sort();
            v
        };
        assert_eq!(part(&a, false), part(&b, false), "{name}: the rhythm parts");
        // A chord-part note due before the settle moves to it (unless it has ended by then,
        // or its key is struck again first: once is enough); the later ones are untouched.
        let (a, b) = (part(&a, true), part(&b, true));
        let split = |v: &[(u64, u8, u8)]| {
            let mut early: Vec<_> = v.iter().filter(|n| n.0 <= window).map(|n| (n.1, n.2)).collect();
            early.dedup();
            (early, v.iter().copied().filter(|n| n.0 > window).collect::<Vec<_>>())
        };
        let ((a_early, a_late), (b_early, b_late)) = (split(&a), split(&b));
        assert_eq!(a_late, b_late, "{name}: the chord parts after the settle");
        assert!(b.iter().all(|n| n.0 >= window), "{name}: a chord part before the settle");
        assert!(b_early.iter().all(|n| a_early.contains(n)), "{name}: {b_early:?} at the settle, {a_early:?} due");
        held += b_early.len();
    }
    assert!(held > 100, "{held} notes started at the settle");
}

/// With the window on, a chord just after the beat still takes the downbeat (the "late
/// chord" test counts from when it arrived, not from the settle).
#[test]
fn late_chord_under_the_window() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/FunkyFinger.S930.STY");
    if !p.exists() {
        return;
    }
    let style = Style::load(&p).unwrap();
    let bar = bar_ns(&Prepared::new(&style));
    let window = crate::engine::CHORD_SETTLE_DEFAULT_MS as u64 * 1_000_000;
    // What sounds just after a chord 20 ms late matches the on-beat chord, window or not.
    let late = 20_000_000;
    let at = |t: u64, settle: u64| {
        let s = [(0, Step::Chord(Chord::new(0, 0))), (t, Step::Chord(Chord::new(5, 0)))];
        let (_, rec) = drive(&style, settle, &s, 4 * bar + late + window + 1);
        rec.sounding_at(4 * bar + late + window).into_iter().filter(|n| follows(n.0)).collect::<Vec<_>>()
    };
    assert_eq!(at(4 * bar + late, window), at(4 * bar, 0), "a late chord keeps the downbeat");
}
