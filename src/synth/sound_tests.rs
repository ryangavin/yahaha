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
    let mut settings = SynthesizerSettings::new(RATE);
    settings.enable_reverb_and_chorus = false;
    let mut s = Synthesizer::new(font, &settings).unwrap();
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

/// Everything at 64 plays exactly as no sound controller at all, and so do portamento off
/// and poly mode.
#[test]
fn neutral_sound_controllers_change_nothing() {
    let Some(font) = font() else { return };
    // The sound controllers at 64. (Portamento off and poly mode are the rack's:
    // rack.rs, `portamento_off_and_poly_change_nothing`.)
    let all: Vec<[i32; 2]> = (71..=78).map(|cc| [cc, 64]).collect();
    for program in [0, 48, 73] {
        let (a, at) = note(&mut synth(&font, program, &[]), 60, 24_000, 24_000);
        let (b, bt) = note(&mut synth(&font, program, &all), 60, 24_000, 24_000);
        assert!(energy(&a) > 1e-3, "program {program} sounds");
        assert!(a == b && at == bt, "program {program}: bit-identical");
    }
}

/// A rack (the tone controls as yahaha plays them, #346 step 3) with `program` on channel
/// 0 and `setup` controllers sent (`[cc, value]`).
fn rack(font: &Arc<SoundFont>, program: i32, setup: &[[i32; 2]]) -> Rack {
    let mut r = Rack::new(font, RATE).unwrap();
    r.process(0, 0xC0, program, 0);
    for &[cc, v] in setup {
        r.process(0, 0xB0, cc, v);
    }
    r
}

/// `frames` frames of a rack's mix (left + right), in 10 ms buffers as the audio thread
/// renders them.
fn rack_render(r: &mut Rack, frames: usize) -> Vec<f32> {
    let peaks: [AtomicU32; 16] = std::array::from_fn(|_| AtomicU32::new(0));
    let (mut l, mut rr) = (vec![0f32; 480], vec![0f32; 480]);
    let mut out = Vec::with_capacity(frames);
    while out.len() < frames {
        let n = (frames - out.len()).min(480);
        r.render_dry(&mut l[..n], &mut rr[..n], &peaks, None);
        out.extend(l[..n].iter().zip(&rr[..n]).map(|(a, b)| a + b));
    }
    out
}

/// A note held on a rack for `hold` frames.
fn rack_note(r: &mut Rack, key: i32, hold: usize) -> Vec<f32> {
    r.process(0, 0x90, key, 100);
    rack_render(r, hold)
}

/// The tone controllers at 64 and the mod wheel at 0 play exactly as none at all, through
/// the rack (its stem filter out of the signal).
#[test]
fn neutral_tone_controls_change_nothing_on_the_stem() {
    let Some(font) = font() else { return };
    let all: Vec<[i32; 2]> = [1, 71, 74, 76, 77, 78].iter().map(|&cc| [cc, if cc == 1 { 0 } else { 64 }]).collect();
    for program in [0, 48, 81] {
        let a = rack_note(&mut rack(&font, program, &[]), 60, 24_000);
        let b = rack_note(&mut rack(&font, program, &all), 60, 24_000);
        assert!(energy(&a) > 1e-3, "program {program} sounds");
        assert!(a == b, "program {program}: bit-identical");
    }
}

/// CC74: a lower cutoff is darker, a higher one no darker; CC71 raises a resonant peak.
/// (#346 step 3: the part's stem filter plays them, from 20 kHz down.)
#[test]
fn cutoff_and_resonance_shape_new_notes() {
    let Some(font) = font() else { return };
    // A saw lead: bright enough to darken.
    let play = |setup: &[[i32; 2]]| rack_note(&mut rack(&font, 81, setup), 60, 24_000);
    let open = brightness(&play(&[]));
    let dark = brightness(&play(&[[74, 10]]));
    assert!(dark < open * 0.5, "CC74 10: brightness {dark} vs {open}");
    let bright = brightness(&play(&[[74, 110]]));
    assert!(bright >= open * 0.99, "CC74 110: brightness {bright} vs {open}");
    // A resonant peak at a low cutoff: more energy near it than without.
    let flat = energy(&play(&[[74, 20]]));
    let peak = energy(&play(&[[74, 20], [71, 127]]));
    assert!(peak > flat * 1.2, "CC71 127: energy {peak} vs {flat}");
    // A note with a cutoff of its own (a drum setup's, #239): the part's filter darkens it
    // further.
    let own = |setup: &[[i32; 2]]| {
        let mut r = rack(&font, 81, setup);
        r.note_on_with(0, 60, 100, &rustysynth::NoteParams { cutoff: 0.25, ..rustysynth::NoteParams::NEUTRAL });
        brightness(&rack_render(&mut r, 24_000))
    };
    let (note_only, both) = (own(&[]), own(&[[74, 20]]));
    assert!(note_only < open * 0.9 && both < note_only * 0.7, "note cutoff {note_only}, with CC74 20 {both}");
}

/// The filter moves on notes already sounding, as on the Genos, and glides there rather
/// than jumping.
#[test]
fn cutoff_moves_a_ringing_note_smoothly() {
    let Some(font) = font() else { return };
    let mut s = rack(&font, 81, &[]);
    s.process(0, 0x90, 72, 100);
    let before = rack_render(&mut s, 12_000);
    s.process(0, 0xB0, 74, 10);
    let after = rack_render(&mut s, 12_000);
    let mut still = rack(&font, 81, &[]);
    still.process(0, 0x90, 72, 100);
    rack_render(&mut still, 12_000);
    let open = rack_render(&mut still, 12_000);
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

/// CC77 adds vibrato, CC78 holds it off for a while (#346 step 3: through the mod wheel,
/// on the rack). CC76 isn't played (upstream rustysynth's vibrato keeps the voice's rate).
#[test]
fn vibrato_depth_and_delay() {
    let Some(font) = font() else { return };
    // A steady tone (recorder) held for 2 s.
    let pitch = |setup: &[[i32; 2]]| crossings(&rack_note(&mut rack(&font, 74, setup), 72, 96_000)[9600..], 960);
    let (own, deep) = (wobble(&pitch(&[])), wobble(&pitch(&[[77, 127]])));
    assert!(deep > own * 2.0 && deep > 0.004, "depth: wobble {deep} vs {own}");
    // On top of the player's mod wheel.
    let (wheel, both) = (wobble(&pitch(&[[1, 64]])), wobble(&pitch(&[[1, 64], [77, 127]])));
    assert!(both > wheel * 1.2, "CC77 on the mod wheel: wobble {both} vs {wheel}");
    let delayed = pitch(&[[77, 127], [78, 127]]);
    // A 1.26 s delay: the first second has only the voice's own vibrato (upstream can't
    // delay that), then CC77's comes in.
    let (early, later) = (wobble(&delayed[..40]), wobble(&delayed[60..]));
    let mid = (own + deep) / 2.0;
    assert!(early < mid && later > mid, "delay: {early} then {later} (own {own}, deep {deep})");
}


/// Each `win`-frame window's zero crossings, from `from`.
fn pitch_track(x: &[f32], from: usize, win: usize) -> Vec<f64> {
    crossings(&x[from..], win)
}

/// `frames` frames of a rack's mix (left + right).
fn render_rack(r: &mut Rack, frames: usize) -> Vec<f32> {
    let peaks: [AtomicU32; 16] = std::array::from_fn(|_| AtomicU32::new(0));
    let (mut l, mut rr) = (vec![0f32; 4800], vec![0f32; 4800]);
    let mut mix = Vec::with_capacity(frames);
    while mix.len() < frames {
        let n = (frames - mix.len()).min(4800);
        r.render_dry(&mut l[..n], &mut rr[..n], &peaks, None);
        mix.extend(l[..n].iter().zip(&rr[..n]).map(|(a, b)| a + b));
    }
    mix
}

/// CC65 on with a CC5 time: a note glides from the key played before at a fixed rate; with
/// time 0 (or CC65 off) it starts on its own pitch.
#[test]
fn portamento_glides_the_pitch() {
    let Some(font) = font() else { return };
    // Flute C4, then C5: each 20 ms window's crossings over the first 0.6 s of C5.
    let glide = |setup: &[[i32; 2]]| {
        let mut s = rack(&font, 73, setup);
        s.process(0, 0x90, 60, 100);
        render_rack(&mut s, 14_400);
        s.process(0, 0x80, 60, 0);
        s.process(0, 0x90, 72, 100);
        pitch_track(&render_rack(&mut s, 28_800), 0, 960)
    };
    let mean = |p: &[f64]| p.iter().sum::<f64>() / p.len() as f64;
    let plain = glide(&[[65, 127], [5, 0]]);
    assert!(mean(&plain[..3]) > mean(&plain[20..]) * 0.85, "time 0: no glide {plain:?}");
    assert_eq!(plain, glide(&[[65, 0], [5, 64]]), "CC65 off: no glide");
    // Time 64: about 0.3 s an octave, so the first windows sound well below C5 and the
    // pitch climbs.
    let slow = glide(&[[65, 127], [5, 64]]);
    let (start, mid, end) = (mean(&slow[..2]), mean(&slow[6..9]), mean(&slow[20..]));
    assert!(start < end * 0.75 && start < mid && mid < end, "time 64: {start} -> {mid} -> {end}");
    assert!((end / mean(&plain[20..]) - 1.0).abs() < 0.05, "it lands on C5: {end}");
}

/// Mono mode (CC126): a new note ends the one sounding (the hold pedal too), and letting
/// go of it goes back to the key still held. CC127: poly again.
#[test]
fn mono_plays_one_note_at_a_time() {
    let Some(font) = font() else { return };
    // Flute C4 and G4 held together, then G4 let go: (energy of the last 0.5 s of both
    // held, the last 0.5 s after G4 is up, and the crossings then).
    let play = |setup: &[[i32; 2]]| {
        let mut s = rack(&font, 73, setup);
        s.process(0, 0x90, 60, 100);
        render_rack(&mut s, 4800);
        s.process(0, 0x90, 67, 100);
        let both = render_rack(&mut s, 48_000);
        s.process(0, 0x80, 67, 0);
        let after = render_rack(&mut s, 48_000);
        (energy(&both[24_000..]), energy(&after[24_000..]), crossings(&after[24_000..], 24_000)[0])
    };
    let one = {
        let mut s = rack(&font, 73, &[]);
        s.process(0, 0x90, 67, 100);
        energy(&render_rack(&mut s, 52_800)[28_800..])
    };
    let c4 = {
        let mut s = rack(&font, 73, &[]);
        s.process(0, 0x90, 60, 100);
        crossings(&render_rack(&mut s, 48_000)[24_000..], 24_000)[0]
    };
    let (poly, _, _) = play(&[]);
    assert!(poly > one * 1.5, "poly: two notes {poly} vs one {one}");
    for setup in [&[[126, 1]][..], &[[64, 127], [126, 1]]] {
        let (mono, back, pitch) = play(setup);
        assert!((mono / one - 1.0).abs() < 0.25, "{setup:?}: one note {mono} vs {one}");
        assert!(back > one * 0.3 && (pitch / c4 - 1.0).abs() < 0.03, "{setup:?}: back to C4 ({pitch} vs {c4} crossings)");
    }
    let (poly_again, _, _) = play(&[[126, 1], [127, 0]]);
    assert!(poly_again > one * 1.5, "CC127: poly again");
}

/// Through the audio thread: a part's XG Mono/Poly SysEx (as an OTS recall sends it) makes
/// the channel mono without cutting the notes sounding, and its XG filter cutoff darkens it
/// as CC74 does.
#[test]
fn xg_part_mono_and_filter_reach_the_synth() {
    let Some(font) = font() else { return };
    let xg = |addr: u8, v: u8| super::sysex_msg(&[0xF0, 0x43, 0x10, 0x4C, 0x08, 0x01, addr, v, 0xF7]).unwrap();
    let play = |setup: &[Msg]| -> (f64, f64, Vec<f32>) {
        let rack = Box::new(Rack::with_fonts(&[(0, font.clone())], RATE).unwrap());
        let (mut tx, rx) = RingBuffer::<Msg>::new(64);
        let (mut core, _swap, _link) = AudioCore::new(Some(rack), vec![rx], Arc::new(Parts::new()), Arc::new(SynthControl::new(0)), RATE as u32, 2);
        let mut out = vec![0f32; 2 * 480];
        let mut run = |core: &mut AudioCore, n: usize| {
            let mut mix = Vec::new();
            for _ in 0..n {
                core.process(&mut out);
                mix.extend(out.chunks(2).map(|f| f[0] + f[1]));
            }
            mix
        };
        for m in [[0xC1, 73, 0], [0xB1, 91, 0], [0xB1, 93, 0], [0x91, 60, 100]] {
            tx.push(m).unwrap();
        }
        run(&mut core, 10);
        for m in setup {
            tx.push(*m).unwrap();
        }
        let before = run(&mut core, 1);
        tx.push([0x91, 67, 100]).unwrap();
        let held = run(&mut core, 100);
        (energy(&before), energy(&held[50 * 480..]), held)
    };
    let (_, poly, open) = play(&[]);
    let (sounding, mono, _) = play(&[xg(0x05, 0)]);
    let (_, poly_again, _) = play(&[xg(0x05, 0), xg(0x05, 1)]);
    assert!(sounding > 0.0, "the Mono SysEx doesn't cut the note sounding");
    assert!(mono < poly * 0.7 && poly_again > poly * 0.95, "mono {mono}, poly {poly}, poly again {poly_again}");
    let (_, _, dark) = play(&[xg(0x18, 10)]);
    let (dark, open) = (brightness(&dark[24_000..]), brightness(&open[24_000..]));
    assert!(dark < open * 0.7, "XG cutoff darkens: {dark} vs {open}");
}
