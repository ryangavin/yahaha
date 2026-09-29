//! The mixer strips on the synth (the strip chain and sends 4-6): with every strip at its
//! defaults, the synth's output is bit-identical to before they were wired in.

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
/// on other targets, so only there is it compared.
#[test]
fn default_strips_leave_the_output_bit_identical() {
    let (h, e) = style_render(200);
    assert!(e > 1.0, "the style sounds: {e}");
    eprintln!("style render hash: {h:#018x}");
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    assert_eq!(h, 0x7e32_651d_731f_44e6, "bit-identical to before the strips");
}
