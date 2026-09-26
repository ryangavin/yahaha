//! #246: the built-in synth plays a part's voice settings (the GM2/XG sound controllers,
//! vibrato, portamento, mono) as a Genos part does. Rendered with a real SoundFont (the
//! smallest in the checkout's soundfonts/; skipped when there is none).

use super::*;

/// The smallest SoundFont in the checkout's soundfonts/ (None: skip).
fn font() -> Option<Arc<SoundFont>> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("soundfonts");
    let f = crate::library::sound_font_files(&dir).into_iter().map(|f| dir.join(f)).min_by_key(|p| p.metadata().map(|m| m.len()).unwrap_or(u64::MAX));
    let Some(f) = f else {
        eprintln!("no SoundFont; skipping");
        return None;
    };
    Some(Arc::new(SoundFont::new(&mut std::fs::File::open(f).unwrap()).unwrap()))
}

const RATE: i32 = 48_000;

/// A synthesizer on channel 0 with `program`, `setup` controllers sent (`[cc, value]`).
fn synth(font: &Arc<SoundFont>, program: i32, setup: &[[i32; 2]]) -> Synthesizer {
    let mut s = Synthesizer::new(font, &SynthesizerSettings::new(RATE)).unwrap();
    s.set_internal_effects(false);
    s.process_midi_message(0, 0xC0, program, 0);
    for &[cc, v] in setup {
        s.process_midi_message(0, 0xB0, cc, v);
    }
    s
}

/// `frames` frames of the mix (left + right).
fn render(s: &mut Synthesizer, frames: usize) -> Vec<f32> {
    let (mut l, mut r) = (vec![0f32; frames], vec![0f32; frames]);
    s.render(&mut l, &mut r);
    l.iter().zip(&r).map(|(a, b)| a + b).collect()
}

fn energy(x: &[f32]) -> f64 {
    x.iter().map(|v| (*v as f64).powi(2)).sum()
}

/// How much of the signal's energy is high frequency: the first difference's energy over
/// the signal's (a one-pole high-pass, about 2x per octave).
fn brightness(x: &[f32]) -> f64 {
    let hf: f64 = x.windows(2).map(|w| ((w[1] - w[0]) as f64).powi(2)).sum();
    hf / energy(x).max(1e-30)
}

/// Each `win`-frame window's RMS.
fn envelope(x: &[f32], win: usize) -> Vec<f64> {
    x.chunks(win).map(|c| (energy(c) / c.len() as f64).sqrt()).collect()
}

/// The first window whose RMS reaches `frac` of the loudest.
fn rise(env: &[f64], frac: f64) -> usize {
    let max = env.iter().cloned().fold(0.0, f64::max);
    env.iter().position(|&e| e >= frac * max).unwrap()
}

/// A note held for `hold` frames, then released for `tail` frames.
fn note(s: &mut Synthesizer, key: i32, hold: usize, tail: usize) -> (Vec<f32>, Vec<f32>) {
    s.note_on(0, key, 100);
    let a = render(s, hold);
    s.note_off(0, key);
    (a, render(s, tail))
}

/// Everything at 64 plays exactly as no sound controller at all.
#[test]
fn neutral_sound_controllers_change_nothing() {
    let Some(font) = font() else { return };
    let all: Vec<[i32; 2]> = (71..=78).map(|cc| [cc, 64]).collect();
    for program in [0, 48, 73] {
        let (a, at) = note(&mut synth(&font, program, &[]), 60, 24_000, 24_000);
        let (b, bt) = note(&mut synth(&font, program, &all), 60, 24_000, 24_000);
        assert!(energy(&a) > 1e-3, "program {program} sounds");
        assert!(a == b && at == bt, "program {program}: bit-identical");
    }
}

/// CC74: a lower cutoff is darker, a higher one (on a voice whose filter is closed some)
/// no darker; CC71 raises a resonant peak.
#[test]
fn cutoff_and_resonance_shape_new_notes() {
    let Some(font) = font() else { return };
    // A saw lead: bright enough to darken.
    let play = |setup: &[[i32; 2]]| note(&mut synth(&font, 81, setup), 60, 24_000, 0).0;
    let open = brightness(&play(&[]));
    let dark = brightness(&play(&[[74, 10]]));
    assert!(dark < open * 0.5, "CC74 10: brightness {dark} vs {open}");
    let bright = brightness(&play(&[[74, 110]]));
    assert!(bright >= open * 0.99, "CC74 110: brightness {bright} vs {open}");
    // A resonant peak at a low cutoff: more energy near it than without.
    let flat = energy(&play(&[[74, 30]]));
    let peak = energy(&play(&[[74, 30], [71, 127]]));
    assert!(peak > flat * 1.2, "CC71 127: energy {peak} vs {flat}");
    // A note with a cutoff of its own (a drum setup's, #239): the channel's scales it too.
    let own = |setup: &[[i32; 2]]| {
        let mut s = synth(&font, 81, setup);
        s.note_on_with(0, 60, 100, &rustysynth::NoteParams { cutoff: 0.25, ..rustysynth::NoteParams::NEUTRAL });
        brightness(&render(&mut s, 24_000))
    };
    let (note_only, both) = (own(&[]), own(&[[74, 40]]));
    assert!(note_only < open * 0.9 && both < note_only * 0.7, "note cutoff {note_only}, with CC74 40 {both}");
}

/// The filter moves on notes already sounding, as on the Genos, and glides there rather
/// than jumping.
#[test]
fn cutoff_moves_a_ringing_note_smoothly() {
    let Some(font) = font() else { return };
    let mut s = synth(&font, 81, &[]);
    s.note_on(0, 72, 100);
    let before = render(&mut s, 12_000);
    s.process_midi_message(0, 0xB0, 74, 10);
    let after = render(&mut s, 12_000);
    let mut still = synth(&font, 81, &[]);
    still.note_on(0, 72, 100);
    render(&mut still, 12_000);
    let open = render(&mut still, 12_000);
    assert!(brightness(&after[4800..]) < brightness(&open[4800..]) * 0.5, "the held note darkens");
    // No step at the change: the largest sample-to-sample jump around it stays within
    // what the note has anyway.
    let jump = |x: &[f32]| x.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0f32, f32::max);
    let edge = [&before[before.len() - 64..], &after[..256]].concat();
    assert!(jump(&edge) <= jump(&before[6000..]) * 1.5, "no click: {} vs {}", jump(&edge), jump(&before[6000..]));
}

/// CC73: a slower attack peaks later; CC75: a shorter decay dies away sooner while held;
/// CC72: a shorter (longer) release, a shorter (longer) tail.
#[test]
fn envelope_times_scale() {
    let Some(font) = font() else { return };
    // Slow strings: an attack of their own to slow down or speed up.
    let rise_at = |setup: &[[i32; 2]]| rise(&envelope(&note(&mut synth(&font, 49, setup), 60, 48_000, 0).0, 480), 0.5);
    let (own, slow, fast) = (rise_at(&[]), rise_at(&[[73, 127]]), rise_at(&[[73, 0]]));
    assert!(slow > own * 2 && fast * 2 < own, "attack: {fast} / {own} / {slow} windows");
    // An electric piano: its decay while held.
    let late = |setup: &[[i32; 2]]| energy(&note(&mut synth(&font, 4, setup), 60, 96_000, 0).0[48_000..]);
    let (own, short, long) = (late(&[]), late(&[[75, 0]]), late(&[[75, 127]]));
    assert!(short < own * 0.5 && long > own * 2.0, "decay: {short} / {own} / {long}");
    // Strings: the tail after the release.
    let tail = |setup: &[[i32; 2]]| energy(&note(&mut synth(&font, 48, setup), 60, 24_000, 48_000).1[2400..]);
    let (own, short, long) = (tail(&[]), tail(&[[72, 0]]), tail(&[[72, 127]]));
    assert!(short < own * 0.5 && long > own * 1.5, "release: {short} / {own} / {long}");
}

/// Each `win`-frame window's pitch, as zero crossings.
fn crossings(x: &[f32], win: usize) -> Vec<f64> {
    x.chunks(win).map(|c| c.windows(2).filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0)).count() as f64).collect()
}

/// How far the windows' pitch strays from their mean (relative standard deviation).
fn wobble(p: &[f64]) -> f64 {
    let m = p.iter().sum::<f64>() / p.len() as f64;
    (p.iter().map(|v| (v - m).powi(2)).sum::<f64>() / p.len() as f64).sqrt() / m
}

/// CC77 adds vibrato, CC78 holds it off for a while, CC76 slows or speeds it.
#[test]
fn vibrato_depth_delay_and_rate() {
    let Some(font) = font() else { return };
    // A steady tone (recorder) held for 2 s.
    let pitch = |setup: &[[i32; 2]]| crossings(&note(&mut synth(&font, 74, setup), 72, 96_000, 0).0[9600..], 960);
    let (own, deep) = (wobble(&pitch(&[])), wobble(&pitch(&[[77, 127]])));
    assert!(deep > own * 2.0 && deep > 0.004, "depth: wobble {deep} vs {own}");
    let delayed = pitch(&[[77, 127], [78, 127]]);
    // A 1.26 s delay: the first second as steady as without vibrato.
    let (early, later) = (wobble(&delayed[..40]), wobble(&delayed[60..]));
    assert!(early < deep * 0.5 && later > early * 2.0, "delay: {early} then {later}");
    // A flute's own vibrato rate, half as fast at -16 (an octave down), about 1.4x at +8.
    let rate = |setup: &[[i32; 2]]| vibrato_rate(&note(&mut synth(&font, 73, setup), 84, 96_000, 0).0[9600..]);
    let (base, slow, fast) = (rate(&[[77, 127]]), rate(&[[77, 127], [76, 48]]), rate(&[[77, 127], [76, 72]]));
    assert!(slow < base * 0.7 && fast > base * 1.2, "rate: {slow} / {base} / {fast} Hz");
}

/// The vibrato's rate (Hz): the strongest frequency (1-40 Hz) in the pitch, measured
/// period by period (upward zero crossings, interpolated) in 10 ms bins.
fn vibrato_rate(x: &[f32]) -> f64 {
    let mut ups = Vec::new();
    for (i, w) in x.windows(2).enumerate() {
        if w[0] < 0.0 && w[1] >= 0.0 {
            ups.push(i as f64 + (-w[0] / (w[1] - w[0])) as f64);
        }
    }
    let bin = RATE as f64 / 100.0;
    let bins = (x.len() as f64 / bin) as usize;
    let (mut sum, mut n) = (vec![0f64; bins], vec![0f64; bins]);
    for w in ups.windows(2) {
        let b = ((w[0] / bin) as usize).min(bins - 1);
        sum[b] += w[1] - w[0];
        n[b] += 1.0;
    }
    let p: Vec<f64> = sum.iter().zip(&n).filter(|(_, n)| **n > 0.0).map(|(s, n)| s / n).collect();
    let m = p.iter().sum::<f64>() / p.len() as f64;
    let mut best = (0.0, 0.0);
    let mut f = 1.0;
    while f <= 40.0 {
        let (mut re, mut im) = (0.0, 0.0);
        for (k, v) in p.iter().enumerate() {
            let a = 2.0 * std::f64::consts::PI * f * k as f64 / 100.0;
            re += (v - m) * a.cos();
            im += (v - m) * a.sin();
        }
        let mag = re * re + im * im;
        if mag > best.1 {
            best = (f, mag);
        }
        f += 0.25;
    }
    best.0
}

