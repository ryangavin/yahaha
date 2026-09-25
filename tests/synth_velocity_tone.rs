//! Velocity changes tone, not only level (#203, #127): the vendored rustysynth applies the
//! SF2 2.01 default modulator "note-on velocity to initial filter cutoff" (-2400 cents,
//! negative concave), so a soft hit is darker than a hard hit on the same note.
//!
//! A SoundFont's own velocity -> filter modulators count too: one that switches the default
//! off (amount 0, as Polyphone and FluidSynth write it) leaves the tone alone, and one of
//! its own adds to it.
//!
//! The SoundFont is built here in memory: one preset whose only sample is white noise, so
//! the filter's effect shows directly in the high-frequency share of the output. No
//! SoundFont file is needed.

use std::sync::Arc;
use yahaha_test_font::{noise_font, noise_font_with};

use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};

const RATE: i32 = 48_000;
/// Rendered length: 0.25 s of held note, then 0.25 s after the note-off.
const HELD: usize = 12_000;
const TAIL: usize = 12_000;

/// Render one note on a fresh synthesizer: the left channel, dry (no reverb or chorus).
fn hit(font: &Arc<SoundFont>, velocity: i32, velocity_to_filter: bool) -> Vec<f32> {
    let mut settings = SynthesizerSettings::new(RATE);
    settings.enable_reverb_and_chorus = false;
    settings.velocity_to_filter = velocity_to_filter;
    let mut s = Synthesizer::new(font, &settings).unwrap();
    let (mut l, mut r) = (vec![0f32; HELD], vec![0f32; HELD]);
    s.note_on(0, 60, velocity);
    s.render(&mut l, &mut r);
    let (mut l2, mut r2) = (vec![0f32; TAIL], vec![0f32; TAIL]);
    s.note_off(0, 60);
    s.render(&mut l2, &mut r2);
    l.extend_from_slice(&l2);
    l
}

fn energy(x: &[f32]) -> f64 {
    x.iter().map(|&v| (v as f64) * (v as f64)).sum()
}

/// The share of the signal's energy in its first difference: a level-free measure of how
/// bright it is (white noise gives about 2, a dull signal much less).
fn brightness(x: &[f32]) -> f64 {
    let diff: Vec<f32> = x.windows(2).map(|w| w[1] - w[0]).collect();
    energy(&diff) / energy(x)
}

/// The last sample above -80 dB of the signal's peak: where the note has died away.
fn last_audible(x: &[f32]) -> usize {
    let peak = x.iter().fold(0f32, |m, v| m.max(v.abs()));
    x.iter().rposition(|v| v.abs() > peak * 1e-4).unwrap_or(0)
}

#[test]
fn a_soft_hit_is_darker_than_a_hard_hit() {
    let font = noise_font();
    let hard = brightness(&hit(&font, 127, true)[..HELD]);
    let soft = brightness(&hit(&font, 32, true)[..HELD]);
    eprintln!("brightness: hard {hard:.4}, soft {soft:.4}");
    assert!(soft < hard * 0.85, "soft {soft:.4} should be well below hard {hard:.4}");
    // Softer again is darker again.
    let softer = brightness(&hit(&font, 12, true)[..HELD]);
    assert!(softer < soft, "v12 {softer:.4} should be darker than v32 {soft:.4}");
}

#[test]
fn without_the_modulator_velocity_only_changes_level() {
    let font = noise_font();
    let hard = brightness(&hit(&font, 127, false)[..HELD]);
    let soft = brightness(&hit(&font, 32, false)[..HELD]);
    assert!((soft / hard - 1.0).abs() < 0.01, "upstream: soft {soft:.4} vs hard {hard:.4}");
}

#[test]
fn a_full_velocity_hit_renders_exactly_as_before() {
    let font = noise_font();
    assert_eq!(hit(&font, 127, true), hit(&font, 127, false));
}

#[test]
fn a_soft_hit_keeps_its_length_and_most_of_its_level() {
    let font = noise_font();
    let (with, without) = (hit(&font, 32, true), hit(&font, 32, false));
    let (a, b) = (last_audible(&with), last_audible(&without));
    assert!(a.abs_diff(b) <= 64, "note length changed: {a} vs {b} samples");
    // White noise is the worst case (all of its energy reaches the top octave the filter
    // takes off); still well under 3 dB quieter.
    let db = 10.0 * (energy(&with) / energy(&without)).log10();
    eprintln!("v32 level change on white noise: {db:.2} dB");
    assert!(db > -3.0 && db <= 0.1, "level changed by {db:.2} dB");
}

/// A modulator record: source, destination (8 = initial filter cutoff), amount, amount
/// source, transform.
fn modulator(src: u16, dest: u16, amount: i16, amount_src: u16) -> [u8; 10] {
    let mut m = [0u8; 10];
    m[0..2].copy_from_slice(&src.to_le_bytes());
    m[2..4].copy_from_slice(&dest.to_le_bytes());
    m[4..6].copy_from_slice(&amount.to_le_bytes());
    m[6..8].copy_from_slice(&amount_src.to_le_bytes());
    m
}

#[test]
fn a_soundfont_can_switch_the_default_off() {
    // What 1,234 Arachno zones and 549 MuseScore_General zones carry: velocity (linear,
    // negative) -> cutoff, amount source a velocity switch, amount 0.
    let font = noise_font_with(&[modulator(0x0102, 8, 0, 0x0D02)], &[]);
    let hard = brightness(&hit(&font, 127, true)[..HELD]);
    let soft = brightness(&hit(&font, 32, true)[..HELD]);
    assert!((soft / hard - 1.0).abs() < 0.01, "switched off: soft {soft:.4} vs hard {hard:.4}");
}

#[test]
fn a_presets_own_modulator_adds_to_the_default() {
    // A preset-level velocity (linear, negative) -> cutoff of -3600 cents, as on many
    // MuseScore_General presets: much darker again than the default alone.
    let own = noise_font_with(&[], &[modulator(0x0102, 8, -3600, 0)]);
    let plain = noise_font();
    let with_own = brightness(&hit(&own, 32, true)[..HELD]);
    let default_only = brightness(&hit(&plain, 32, true)[..HELD]);
    assert!(with_own < default_only * 0.7, "own {with_own:.4} vs default {default_only:.4}");
    // Not at full velocity: linear negative is 0 there.
    assert_eq!(hit(&own, 127, true), hit(&plain, 127, true));
    // Other destinations and sources stay ignored, as upstream: velocity -> attenuation
    // (48) and key -> cutoff (source 3) change nothing.
    let foreign = noise_font_with(&[modulator(0x0502, 48, 800, 0), modulator(0x0003, 8, 2400, 0)], &[]);
    assert_eq!(hit(&foreign, 32, true), hit(&plain, 32, true));
}

/// A tiny SoundFont in memory: preset 0:0 plays one second of white noise, root key 60.
mod yahaha_test_font {
    use super::*;

    fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut v = id.to_vec();
        v.extend_from_slice(&(body.len() as u32).to_le_bytes());
        v.extend_from_slice(body);
        if body.len() % 2 == 1 {
            v.push(0);
        }
        v
    }

    fn list(kind: &[u8; 4], chunks: &[Vec<u8>]) -> Vec<u8> {
        let mut body = kind.to_vec();
        for c in chunks {
            body.extend_from_slice(c);
        }
        chunk(b"LIST", &body)
    }

    fn name(s: &str) -> Vec<u8> {
        let mut v = s.as_bytes().to_vec();
        v.resize(20, 0);
        v
    }

    fn u16s(xs: &[u16]) -> Vec<u8> {
        xs.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    pub fn noise_font() -> Arc<SoundFont> {
        noise_font_with(&[], &[])
    }

    /// With these modulators on its instrument zone and its preset zone.
    pub fn noise_font_with(imods: &[[u8; 10]], pmods: &[[u8; 10]]) -> Arc<SoundFont> {
        const LEN: u32 = 48_000;
        // A fixed LCG, so every run renders the same noise.
        let mut seed = 0x1234_5678u32;
        let mut smpl = Vec::new();
        for _ in 0..LEN {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let v = ((seed >> 16) as i16) / 4;
            smpl.extend_from_slice(&v.to_le_bytes());
        }
        // The spec asks for 46 zero samples after each sample.
        smpl.extend(std::iter::repeat_n(0u8, 92));

        let info = list(b"INFO", &[chunk(b"ifil", &u16s(&[2, 1])), chunk(b"isng", b"EMU8000\0"), chunk(b"INAM", b"test\0\0")]);
        let sdta = list(b"sdta", &[chunk(b"smpl", &smpl)]);

        let phdr = |n: &str, preset: u16, bag: u16| {
            let mut v = name(n);
            v.extend(u16s(&[preset, 0, bag]));
            v.extend([0u8; 12]);
            v
        };
        let mut phdrs = phdr("noise", 0, 0);
        phdrs.extend(phdr("EOP", 0, 1));
        let pbag = u16s(&[0, 0, 1, pmods.len() as u16]);
        let mut pmod: Vec<u8> = pmods.concat();
        pmod.extend([0u8; 10]);
        let pgen = u16s(&[41, 0, 0, 0]); // instrument 0, then the terminator
        let mut inst = name("noise");
        inst.extend(u16s(&[0]));
        inst.extend(name("EOI"));
        inst.extend(u16s(&[1]));
        let ibag = u16s(&[0, 0, 1, imods.len() as u16]);
        let mut imod: Vec<u8> = imods.concat();
        imod.extend([0u8; 10]);
        let igen = u16s(&[53, 0, 0, 0]); // sample 0, then the terminator
        let shdr_rec = |n: &str, start: u32, end: u32, rate: u32, pitch: u8| {
            let mut v = name(n);
            for x in [start, end, start, end, rate] {
                v.extend(x.to_le_bytes());
            }
            v.extend([pitch, 0]);
            v.extend(u16s(&[0, if rate == 0 { 0 } else { 1 }]));
            v
        };
        let mut shdr = shdr_rec("noise", 0, LEN, RATE as u32, 60);
        shdr.extend(shdr_rec("EOS", 0, 0, 0, 0));
        let pdta = list(
            b"pdta",
            &[
                chunk(b"phdr", &phdrs),
                chunk(b"pbag", &pbag),
                chunk(b"pmod", &pmod),
                chunk(b"pgen", &pgen),
                chunk(b"inst", &inst),
                chunk(b"ibag", &ibag),
                chunk(b"imod", &imod),
                chunk(b"igen", &igen),
                chunk(b"shdr", &shdr),
            ],
        );

        let mut body = b"sfbk".to_vec();
        body.extend(info);
        body.extend(sdta);
        body.extend(pdta);
        let riff = chunk(b"RIFF", &body);
        Arc::new(SoundFont::new(&mut &riff[..]).unwrap())
    }
}
