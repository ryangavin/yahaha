//! The mixer strips on the synth (the strip chain and sends 4-6): with every strip at its
//! defaults, the synth's output is bit-identical to before they were wired in; the chain
//! (compressor, insert 2) runs on a SoundFont stem, channel by channel; send 4 plays only
//! what a channel sends it.

use super::rack_tests::{tiny_font_with, CUTOFF, RELEASE};
use super::*;

/// A style's worth of parts on the tiny font through the whole `AudioCore` (sends 1-3 to
/// all three buses, a band scale, a part's own send, a style insert, a keyboard part and
/// a drum part), `buffers` buffers of 256 frames: every output sample's bits, hashed
/// (FNV-1a), and the output's energy.
fn style_render(buffers: usize) -> (u64, f64) {
    let font = tiny_font_with(&[(CUTOFF, 6000), (RELEASE, -2084)]);
    let rack = Box::new(Rack::new(&font, 48_000).unwrap());
    let (mut tx, rx) = RingBuffer::<Msg>::new(256);
    let ctl = Arc::new(SynthControl::new(0));
    // The variation block on, a band send scale, a Style part's own send and insert.
    ctl.fx.variation_return.store(90, Relaxed);
    ctl.fx.band_send[0].store(110, Relaxed);
    ctl.fx.part_send[2][1].store(80, Relaxed);
    ctl.fx.insert[3].store(yahaha_fx::fx::InsertKind::Distortion as u8, Relaxed);
    ctl.fx.insert_amount[3].store(90, Relaxed);
    let (mut core, _swap, _link) = AudioCore::new(Some(rack), vec![rx], Arc::new(Parts::new()), ctl, 48_000, 2);
    // Right 1 (ch 1) and the Style parts (ch 9-16): programs, sends and a chord.
    for (i, ch) in [0u8, 8, 9, 10, 11, 12, 13, 14, 15].into_iter().enumerate() {
        let key = 40 + 5 * i as u8;
        for m in [
            [0xC0 | ch, 16 + i as u8, 0],
            [0xB0 | ch, 91, 30 + 10 * i as u8],
            [0xB0 | ch, 93, 20 + 7 * i as u8],
            [0xB0 | ch, 94, 15 * i as u8],
            [0xB0 | ch, 10, 20 + 10 * i as u8],
            [0x90 | ch, key, 90],
        ] {
            tx.push(m).unwrap();
        }
    }
    let mut out = vec![0f32; 512];
    let (mut h, mut e) = (0xcbf2_9ce4_8422_2325u64, 0f64);
    for b in 0..buffers {
        if b == buffers / 2 {
            for ch in [0u8, 8, 9, 10, 11, 12, 13, 14, 15] {
                tx.push([0xB0 | ch, 123, 0]).unwrap();
            }
        }
        core.process(&mut out);
        for x in &out {
            e += (*x as f64).powi(2);
            for byte in x.to_bits().to_le_bytes() {
                h = (h ^ byte as u64).wrapping_mul(0x0100_0000_01b3);
            }
        }
    }
    (h, e)
}

/// Every strip at its defaults (no compressor, no insert 2, no send to 4-6): the output is
/// bit-identical to the synth's before the strips were wired in. The hash was taken on
/// that code (origin/fx/strip-dsp, 0e2deebb) on macOS/aarch64; libm may round differently
/// on other targets, so only there is it compared: only CI's macOS job covers the hash.
/// Elsewhere the test still renders and checks the style sounds, and prints that the hash
/// was not compared.
#[test]
fn default_strips_leave_the_output_bit_identical_hash_on_macos_aarch64_only() {
    let (h, e) = style_render(200);
    assert!(e > 1.0, "the style sounds: {e}");
    eprintln!("style render hash: {h:#018x}");
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    assert_eq!(h, 0x7e32_651d_731f_44e6, "bit-identical to before the strips");
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    println!("SKIPPED: golden hash not compared on this target (only macOS/aarch64 pins it); got {h:#018x}");
}

/// Buffers of 256 frames in a `note_render`, and the one the note is let go at.
const BUFFERS: usize = 160;
const HOLD: usize = 20;

/// One note (A2) on channel `ch`, program 17, through the whole `AudioCore` on the tiny font
/// (a soft tone, a 0.3 s release): held for `HOLD` buffers of 256 frames, then let go,
/// `BUFFERS` in all (0.85 s). Its sends 1-3 (CC91/93/94) are 0. `setup` sets the control
/// side before the first buffer. Every output sample, interleaved stereo.
fn note_render(ch: u8, setup: impl FnOnce(&SynthControl)) -> Vec<f32> {
    let font = tiny_font_with(&[(CUTOFF, 6000), (RELEASE, -2084)]);
    let rack = Box::new(Rack::new(&font, 48_000).unwrap());
    let (mut tx, rx) = RingBuffer::<Msg>::new(64);
    let ctl = Arc::new(SynthControl::new(0));
    setup(&ctl);
    let (mut core, _swap, _link) = AudioCore::new(Some(rack), vec![rx], Arc::new(Parts::new()), ctl, 48_000, 2);
    for m in [[0xC0 | ch, 16, 0], [0xB0 | ch, 91, 0], [0xB0 | ch, 93, 0], [0xB0 | ch, 94, 0], [0x90 | ch, 45, 100]] {
        tx.push(m).unwrap();
    }
    let mut out = vec![0f32; 512];
    let mut all = Vec::with_capacity(BUFFERS * out.len());
    for b in 0..BUFFERS {
        if b == HOLD {
            tx.push([0x80 | ch, 45, 0]).unwrap();
        }
        core.process(&mut out);
        all.extend_from_slice(&out);
    }
    all
}

fn energy(x: &[f32]) -> f64 {
    x.iter().map(|v| (*v as f64).powi(2)).sum()
}

/// How far `b` is from `a`: the energy of their difference over `a`'s.
fn change(a: &[f32], b: &[f32]) -> f64 {
    a.iter().zip(b).map(|(x, y)| ((x - y) as f64).powi(2)).sum::<f64>() / energy(a)
}

fn bits(x: &[f32]) -> Vec<u32> {
    x.iter().map(|v| v.to_bits()).collect()
}

/// The strip chain runs on a SoundFont part's stem: a heavy compressor on a Style part's
/// channel (the Bass, channel 11) changes what it plays, and leaves another channel's
/// output bit-identical; insert 2 (a distortion) on a keyboard part's channel changes its.
#[test]
fn the_strip_chain_runs_on_a_soundfont_stem() {
    use yahaha_fx::fx::master::CompPreset;
    use yahaha_fx::fx::{InsertSlot, InsertType, PartComp};
    let loud = PartComp::of(true, CompPreset::Loud);

    // The compressor on the Bass part (channel index 10).
    let off = note_render(10, |_| {});
    let on = note_render(10, |c| c.fx.strips.set_comp(10, &loud));
    assert!(energy(&off) > 1.0, "the part sounds: {}", energy(&off));
    let d = change(&off, &on);
    assert!(d > 0.01, "the compressor changed the part: {d}");

    // Another part (channel index 11) with the Bass part's compressor on: untouched.
    let other_off = note_render(11, |_| {});
    let other_on = note_render(11, |c| c.fx.strips.set_comp(10, &loud));
    assert!(energy(&other_off) > 1.0);
    assert!(bits(&other_off) == bits(&other_on), "another channel's compressor changed this one");

    // Insert 2 on Right 1 (channel index 0): a distortion.
    let dry = note_render(0, |_| {});
    let wet = note_render(0, |c| c.fx.strips.set_second(0, &InsertSlot::of(InsertType::Distortion)));
    assert!(energy(&dry) > 1.0);
    let d = change(&dry, &wet);
    assert!(d > 0.01, "insert 2 changed the part: {d}");
    // And on another channel only: this one is untouched.
    let elsewhere = note_render(0, |c| c.fx.strips.set_second(3, &InsertSlot::of(InsertType::Distortion)));
    assert!(bits(&dry) == bits(&elsewhere), "another channel's insert 2 changed this one");
}

/// Send 4 as a Hall reverb (the three buses' sends and returns at 0): a part whose send 4 is
/// 0 stops with its release; one whose send 4 is up rings on well after it.
#[test]
fn send_4_feeds_its_reverb_only_from_a_channel_that_sends_to_it() {
    use yahaha_fx::fx::{SendKind, SendSlot};
    let play = |level: u8| {
        note_render(0, |c| {
            for r in [&c.fx.reverb_return, &c.fx.chorus_return, &c.fx.variation_return] {
                r.store(0, Relaxed);
            }
            c.fx.sends[0].set(&SendSlot::of(SendKind::Hall));
            c.fx.strip_send[0][0].store(level, Relaxed);
        })
    };
    // The tail: from 0.5 s after the note-off (the release is 0.3 s) to the end, 0.75 s
    // after it.
    let tail = |x: &[f32]| energy(&x[(HOLD + 94) * 512..]);
    let dry = play(0);
    let wet = play(127);
    assert!(energy(&dry[..HOLD * 512]) > 1.0, "the note sounds");
    let (dry_tail, wet_tail) = (tail(&dry), tail(&wet));
    assert!(dry_tail < 1e-6, "no send 4: silent after the release ({dry_tail})");
    assert!(wet_tail > 1e-3 && wet_tail > 1000.0 * dry_tail, "send 4 up: a reverb tail ({wet_tail} vs {dry_tail})");
}
