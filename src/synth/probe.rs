//! SCRATCH probes (not committed): loudness, CC11 carry-over and timing measurements.
use super::*;
use crate::engine::{Button, Prepared};
use crate::sff::Style;
use crate::sim::{run, run_observed, Step};
use crate::theory::Chord;

fn styles() -> Vec<std::path::PathBuf> {
    let filt: Vec<String> = std::env::var("PROBE_STYLES").unwrap_or_default().split(',').filter(|s| !s.is_empty()).map(|s| s.to_lowercase()).collect();
    crate::library::corpus_styles()
        .into_iter()
        .filter(|f| filt.is_empty() || filt.iter().any(|s| f.to_string_lossy().to_lowercase().contains(s)))
        .collect()
}

fn fonts() -> Vec<std::path::PathBuf> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("soundfonts");
    std::env::var("PROBE_FONTS")
        .unwrap_or_else(|_| "MuseScore_General(v0.1.3).sf2,Arachno_SoundFont_Version_1.0.sf2,yamaha tyros 4_just_t4_fixed.sf2".into())
        .split(',')
        .map(|f| root.join(f))
        .collect()
}

#[test]
#[ignore]
fn device_buffer() {
    let host = cpal::default_host();
    let d = host.default_output_device().unwrap();
    let c = d.default_output_config().unwrap();
    println!("device {:?} rate {:?} buffer {:?}", d.description().map(|x| x.to_string()), c.sample_rate(), c.buffer_size());
}

fn bar_ns(prep: &Prepared) -> u64 {
    (60e9 / prep.bpm * (prep.tpb as f64 / prep.ppq as f64)) as u64
}

/// Per part: solo RMS, CC7, CC11, mean velocity, the voice, and the residual (the level
/// the MIDI alone doesn't explain: the preset's own level).
#[test]
#[ignore]
fn loudness() {
    let fonts: Vec<(String, Arc<SoundFont>)> = fonts()
        .iter()
        .map(|p| (p.file_stem().unwrap().to_string_lossy().chars().take(12).collect(), Arc::new(SoundFont::new(&mut std::fs::File::open(p).unwrap()).unwrap())))
        .collect();
    for f in styles() {
        let style = Style::load(&f).unwrap();
        let prep = Box::new(Prepared::new(&style));
        let bar = bar_ns(&prep);
        let script: Vec<(u64, Step)> = (0..8).map(|i| (i * bar, Step::Chord(Chord::new([0, 9, 5, 7][i as usize % 4], 0)))).collect();
        let (_, rec) = run(prep, &script, 8 * bar);
        println!("== {}", f.file_name().unwrap().to_string_lossy());
        for part in 8..16u8 {
            let notes: Vec<u8> = rec.out.iter().filter(|(_, m)| m[0] == 0x90 | part && m[2] > 0).map(|(_, m)| m[2]).collect();
            if notes.is_empty() {
                continue;
            }
            // CC state at each note-on.
            let (mut cc7, mut cc11) = (100u8, 127u8);
            let (mut s7, mut s11, mut min11) = (0f64, 0f64, 127u8);
            let mut voice = (0u8, 0u8, 0u8);
            for (_, m) in &rec.out {
                if m[0] == 0xB0 | part {
                    match m[1] {
                        7 => cc7 = m[2],
                        11 => cc11 = m[2],
                        0 => voice.0 = m[2],
                        32 => voice.1 = m[2],
                        _ => {}
                    }
                } else if m[0] == 0xC0 | part {
                    voice.2 = m[1];
                } else if m[0] == 0x90 | part && m[2] > 0 {
                    s7 += cc7 as f64;
                    s11 += cc11 as f64;
                    min11 = min11.min(cc11);
                }
            }
            let n = notes.len() as f64;
            let (a7, a11) = (s7 / n, s11 / n);
            let vdb: f64 = notes.iter().map(|&v| 40.0 * (v as f64 / 127.0).log10()).sum::<f64>() / n;
            let midi_db = 40.0 * (a7 / 127.0).log10() + 40.0 * (a11 / 127.0).log10() + vdb;
            let mut line = format!(
                "  p{} v{:03}/{:03}/{:03} n{:4} cc7 {:5.1} cc11 {:5.1}(min {:3}) vel {:5.1}dB midi {:6.1}dB |",
                part - 7,
                voice.0,
                voice.1,
                voice.2,
                notes.len(),
                a7,
                a11,
                min11,
                vdb,
                midi_db
            );
            for (_, font) in &fonts {
                let mut synth = Synthesizer::new(font, &SynthesizerSettings::new(48000)).unwrap();
                let mut player = Synthesizer::new(font, &SynthesizerSettings::new(48000)).unwrap();
                synth.process_midi_message(8, 0xB0, 0, 128);
                let mut bank = [0u8; 16];
                let (mut l, mut r) = (vec![0f32; 64], vec![0f32; 64]);
                let (mut t, mut i, mut e, mut cnt) = (0u64, 0usize, 0f64, 0u64);
                let end = 8 * bar * 48000 / 1_000_000_000;
                while t < end {
                    let now = t * 1_000_000_000 / 48000;
                    while i < rec.out.len() && rec.out[i].0 <= now {
                        let m = &rec.out[i].1;
                        if m.len() <= 3 && (!matches!(m[0] & 0xF0, 0x80 | 0x90) || m[0] & 0xF == part) {
                            let mut a = [0u8; 3];
                            a[..m.len()].copy_from_slice(m);
                            apply(&mut synth, &mut player, &a, &mut bank);
                        }
                        i += 1;
                    }
                    synth.render(&mut l, &mut r);
                    for k in 0..64 {
                        e += (l[k] * l[k] + r[k] * r[k]) as f64;
                    }
                    cnt += 64;
                    t += 64;
                }
                let db = 10.0 * ((e / cnt as f64).max(1e-12)).log10();
                line.push_str(&format!(" {:6.1} (res {:6.1})", db, db - midi_db));
            }
            println!("{line}");
        }
    }
}

/// CC11 at each part's first note after each section change in a long script, against
/// the CC11 that section starts with when played on its own from a fresh start.
#[test]
#[ignore]
fn cc11_carry() {
    let buttons = [
        Button::Main(1),
        Button::Fill(0),
        Button::Main(2),
        Button::Main(3),
        Button::Fill(0),
        Button::Main(0),
        Button::Break,
        Button::Main(2),
        Button::Main(1),
        Button::Fill(0),
        Button::Main(3),
        Button::Main(0),
    ];
    let (mut bad_styles, mut total) = (0, 0);
    for f in styles() {
        let Ok(style) = Style::load(&f) else { continue };
        total += 1;
        let prep = Prepared::new(&style);
        let bar = bar_ns(&prep);
        // Fresh: each Main on its own, CC11 at the first note per part.
        let mut fresh = std::collections::HashMap::new();
        for m in 0..4u8 {
            let p = Box::new(Prepared::new(&style));
            let script = vec![(0, Step::Button(Button::Main(m))), (1, Step::Chord(Chord::new(0, 0)))];
            let mut cur = None;
            let (_, rec) = run_observed(p, &script, 2 * bar, |e, now| cur = cur.or(e.snapshot(now).cur));
            let Some(sec) = cur else { continue };
            let mut cc11 = [127u8; 16];
            let mut first = [None; 16];
            for (_, m) in &rec.out {
                let ch = (m[0] & 15) as usize;
                if m[0] & 0xF0 == 0xB0 && m[1] == 11 {
                    cc11[ch] = m[2];
                }
                if m[0] & 0xF0 == 0x90 && m.len() > 2 && m[2] > 0 && first[ch].is_none() {
                    first[ch] = Some(cc11[ch]);
                }
            }
            fresh.insert(format!("{sec:?}"), first);
        }
        // The long run.
        let mut script = vec![(0u64, Step::Chord(Chord::new(0, 0)))];
        for (i, b) in buttons.iter().enumerate() {
            script.push(((i as u64 + 1) * 2 * bar + bar / 2, Step::Button(*b)));
        }
        let end = (buttons.len() as u64 + 2) * 2 * bar;
        let mut secs: Vec<(u64, String)> = Vec::new();
        let (_, rec) = run_observed(Box::new(Prepared::new(&style)), &script, end, |e, now| {
            let s = format!("{:?}", e.snapshot(now).cur);
            if secs.last().is_none_or(|l| l.1 != s) {
                secs.push((now, s));
            }
        });
        if std::env::var("PROBE_DEBUG").is_ok() {
            println!("fresh {:?} secs {:?}", fresh.keys().collect::<Vec<_>>(), secs);
        }
        let mut cc11 = [127u8; 16];
        let mut idx = 0;
        let mut issues = Vec::new();
        for (k, (start, name)) in secs.iter().enumerate() {
            let stop = secs.get(k + 1).map(|s| s.0).unwrap_or(u64::MAX);
            let name = name.trim_start_matches("Some(").trim_end_matches(')').to_string();
            let mut first = [None; 16];
            while idx < rec.out.len() && rec.out[idx].0 < stop {
                let (t, m) = &rec.out[idx];
                let ch = (m[0] & 15) as usize;
                if m[0] & 0xF0 == 0xB0 && m[1] == 11 {
                    cc11[ch] = m[2];
                }
                if *t >= *start && m[0] & 0xF0 == 0x90 && m.len() > 2 && m[2] > 0 && first[ch].is_none() {
                    first[ch] = Some(cc11[ch]);
                }
                idx += 1;
            }
            if let Some(fr) = fresh.get(&name) {
                for ch in 8..16 {
                    if let (Some(a), Some(b)) = (first[ch], fr[ch])
                        && a != b
                    {
                        issues.push(format!("{name} p{} cc11 {a} (fresh {b})", ch - 7));
                    }
                }
            }
        }
        if !issues.is_empty() {
            bad_styles += 1;
            println!("{}: {}", f.file_name().unwrap().to_string_lossy(), issues.join("; "));
        }
    }
    println!("{bad_styles}/{total} styles start a Main with a different CC11 than a fresh start");
}

/// Note-on timing: engine output vs the source ticks, then what the audio drain does.
#[test]
#[ignore]
fn timing() {
    for f in styles() {
        let Ok(style) = Style::load(&f) else { continue };
        let prep = Box::new(Prepared::new(&style));
        let bar = bar_ns(&prep);
        let ns_per_tick = 60e9 / prep.bpm / prep.ppq as f64;
        let ppq = prep.ppq;
        let script = vec![(0, Step::Button(Button::Main(0))), (0, Step::Chord(Chord::new(0, 0)))];
        let mut t0 = None;
        let (_, rec) = run_observed(prep, &script, 4 * bar, |e, now| {
            if t0.is_none() && e.snapshot(now).running {
                t0 = Some(now);
            }
        });
        let t0 = t0.unwrap_or(0);
        // Position of each note-on within a beat, in ticks.
        let mut offgrid = 0;
        let mut errs = Vec::new();
        let mut n = 0;
        let mut pos_hist = std::collections::BTreeMap::<i64, usize>::new();
        for (t, m) in &rec.out {
            if m[0] & 0xF0 == 0x90 && m[2] > 0 && *t >= t0 {
                n += 1;
                let tick = (*t - t0) as f64 / ns_per_tick;
                // Engine error: distance to the nearest integer tick (source ticks are integers).
                errs.push((tick - tick.round()).abs() * ns_per_tick);
                let in16 = (tick.round() as i64) % (ppq as i64 / 4);
                if in16 != 0 {
                    offgrid += 1;
                }
                *pos_hist.entry((tick.round() as i64) % ppq as i64).or_default() += 1;
            }
        }
        let max = errs.iter().cloned().fold(0.0, f64::max);
        println!(
            "{:<32} ppq {ppq} bpm {:.0} notes {n} off-16th {offgrid} engine max err {:.3} us",
            f.file_name().unwrap().to_string_lossy(),
            60e9 / ns_per_tick / ppq as f64,
            max / 1e3
        );
        let top: Vec<String> = {
            let mut v: Vec<_> = pos_hist.iter().collect();
            v.sort_by(|a, b| b.1.cmp(a.1));
            v.iter().take(8).map(|(k, c)| format!("{k}:{c}")).collect()
        };
        println!("   in-beat tick positions (top): {}", top.join(" "));
        // Audio: drained at the next 64-frame boundary; rustysynth starts a voice at its next
        // 64-sample block.
        for frames in [64u64, 128, 256, 512] {
            let b = frames as f64 * 1e9 / 48000.0;
            let e: Vec<f64> = rec.out.iter().filter(|(_, m)| m[0] & 0xF0 == 0x90 && m[2] > 0).map(|(t, _)| ((*t as f64 / b).ceil() * b - *t as f64) / 1e6).collect();
            let mean = e.iter().sum::<f64>() / e.len() as f64;
            let sd = (e.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / e.len() as f64).sqrt();
            println!("   buffer {frames:3}: onset delay mean {mean:.2} ms sd {sd:.2} ms max {:.2} ms", e.iter().cloned().fold(0.0, f64::max));
        }
    }
}

/// Parts whose patterns take CC11 below 127 while the style's setup never sets CC11 on
/// them: whatever the patterns leave behind carries into the next style.
#[test]
#[ignore]
fn cc11_setup() {
    let (mut n, mut total) = (0, 0);
    for f in styles() {
        let Ok(style) = Style::load(&f) else { continue };
        total += 1;
        let prep = Prepared::new(&style);
        let bar = bar_ns(&prep);
        let mut set = 0u16;
        for m in prep.setup(0).init.iter() {
            if m.len() == 3 && m[0] & 0xF0 == 0xB0 && m[1] == 11 {
                set |= 1 << (m[0] & 15);
            }
        }
        let mut script = vec![(0u64, Step::Chord(Chord::new(0, 0)))];
        for (i, b) in [Button::Main(1), Button::Main(2), Button::Main(3), Button::Break, Button::Main(0)].iter().enumerate() {
            script.push(((i as u64 + 1) * 4 * bar + bar / 2, Step::Button(*b)));
        }
        let (_, rec) = run(Box::new(prep), &script, 24 * bar);
        let mut low = [127u8; 16];
        let mut last = [None; 16];
        for (_, m) in &rec.out {
            if m[0] & 0xF0 == 0xB0 && m[1] == 11 {
                let c = (m[0] & 15) as usize;
                low[c] = low[c].min(m[2]);
                last[c] = Some(m[2]);
            }
        }
        let bad: Vec<String> = (8..16).filter(|&c| set >> c & 1 == 0 && low[c] < 127).map(|c| format!("p{} low {} last {:?}", c - 7, low[c], last[c])).collect();
        if !bad.is_empty() {
            n += 1;
            println!("{}: setup cc11 {:016b} {}", f.file_name().unwrap().to_string_lossy(), set, bad.join(", "));
        }
    }
    println!("{n}/{total} styles leave CC11 below 127 on a part their setup does not reset");
}
