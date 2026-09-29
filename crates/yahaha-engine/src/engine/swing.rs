//! Live Swing (parity matrix feel/groove; docs/research/genos-parity/notes/style-assembly.md):
//! a tick remap of the Style's events as they play, like the Genos Style Creator's Groove
//! Swing but live, and never written into the Style.
//!
//! Within each swing period (a beat for the 8th grid, half a beat for the 16th grid) the
//! position `p` (0..1) maps piecewise-linearly through the knots 0 → 0, ½ → `m`, ⅔ → ⅔,
//! 1 → 1, where `m` goes from ½ (swing 0: as written) to ⅔ (swing 100: the triplet
//! position). A straight off-beat moves toward the triplet; a part already swung (an
//! off-beat at ⅔) stays where it is, and positions between scale in proportion, so a
//! swung part is not swung again. The map is monotonic, so the events keep their order,
//! and it never moves the period's downbeat. Drums and every accompaniment part follow
//! it; the player's own keys are live and never pass through it. Plain maths only: no
//! allocation.

use super::*;

/// Where a period position `p` (0..1) plays at swing `m` (the off-beat's new place, ½..⅔).
#[inline]
fn remap(p: f64, m: f64) -> f64 {
    const T: f64 = 2.0 / 3.0;
    if p <= 0.5 {
        p * 2.0 * m
    } else if p <= T {
        m + (p - 0.5) * (T - m) / (T - 0.5)
    } else {
        p
    }
}

/// The swung section-relative tick of a Style event at `tick`, with `ppq` ticks a beat.
#[inline]
pub(super) fn swung_tick(tick: u32, ppq: u32, swing: u8, grid: u8) -> f64 {
    if swing == 0 || ppq == 0 {
        return tick as f64;
    }
    let period = if grid >= 16 { ppq as f64 / 2.0 } else { ppq as f64 };
    let t = tick as f64;
    let base = (t / period).floor() * period;
    let m = 0.5 + (swing.min(100) as f64 / 100.0) * (2.0 / 3.0 - 0.5);
    base + remap((t - base) / period, m) * period
}

impl Engine {
    /// Where a Style event at section tick `tick` plays, with the swing in effect.
    #[inline]
    pub(super) fn ev_tick(&self, tick: u32) -> f64 {
        let s = &self.features.settings;
        swung_tick(tick, self.style.ppq, s.swing, s.swing_grid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_swing_plays_as_written() {
        for t in [0, 1, 240, 479, 480, 481, 959, 960, 12345] {
            assert_eq!(swung_tick(t, 480, 0, 8), t as f64);
        }
    }

    #[test]
    fn a_straight_off_beat_moves_toward_the_triplet() {
        // 8th grid at 480 ppq: the off-beat 8th at 240 goes to 320 (the triplet) at 100 %.
        assert!((swung_tick(240, 480, 100, 8) - 320.0).abs() < 1e-9);
        assert!((swung_tick(240, 480, 50, 8) - 280.0).abs() < 1e-9);
        // Downbeats stay; the next beat's off-beat moves the same way.
        assert_eq!(swung_tick(480, 480, 100, 8), 480.0);
        assert!((swung_tick(720, 480, 100, 8) - 800.0).abs() < 1e-9);
        // 16th grid: the off-beat 16th at 120 goes to 160.
        assert!((swung_tick(120, 480, 100, 16) - 160.0).abs() < 1e-9);
        assert_eq!(swung_tick(240, 480, 100, 16), 240.0);
    }

    #[test]
    fn an_already_swung_part_is_not_swung_again() {
        // A triplet off-beat (at 2/3 of the beat) and anything after it stay put.
        for s in [0, 30, 100] {
            assert!((swung_tick(320, 480, s, 8) - 320.0).abs() < 1e-9);
            assert!((swung_tick(400, 480, s, 8) - 400.0).abs() < 1e-9);
        }
    }

    struct Rec(u64, Vec<(u64, [u8; 3])>);
    impl Sink for Rec {
        fn send(&mut self, m: &[u8]) {
            if m.len() == 3 && m[0] & 0xF0 == 0x90 && m[2] > 0 {
                self.1.push((self.0, [m[0], m[1], m[2]]));
            }
        }
    }

    /// The band's note-ons over two bars (time, message) at swing `swing`.
    fn play(swing: u8, grid: u8) -> Option<Vec<(u64, [u8; 3])>> {
        let p = yahaha_sff::library::corpus_dir().join("MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return None;
        }
        let mut e = Engine::new(Box::new(Prepared::new(&Style::load(&p).unwrap())));
        e.set_style_settings(StyleSettings { swing, swing_grid: grid, ..StyleSettings::default() });
        let mut rec = Rec(0, Vec::new());
        e.set_chord(yahaha_core::parse_chord("C").unwrap(), 0, &mut rec);
        let end = e.ns_at_bar(2);
        while rec.0 < end {
            e.process(rec.0, &mut rec);
            rec.0 += 1_000_000;
        }
        Some(rec.1)
    }

    /// The whole band follows the swing: the same notes in the same order, none earlier,
    /// some later; swing 0 is the Style as written.
    #[test]
    fn the_band_plays_swung() {
        let Some(straight) = play(0, 8) else { return };
        assert_eq!(play(0, 16).unwrap(), straight, "swing 0 plays as written whatever the grid");
        for grid in [8, 16] {
            let swung = play(100, grid).unwrap();
            assert_eq!(swung.len(), straight.len());
            let mut later = 0;
            for (s, w) in straight.iter().zip(&swung) {
                assert_eq!(s.1, w.1, "the same notes in the same order");
                assert!(w.0 >= s.0, "no note plays earlier");
                later += (w.0 > s.0 + 2_000_000) as usize;
            }
            assert!(later > 0, "grid {grid}: some off-beats moved");
        }
    }

    #[test]
    fn the_remap_keeps_event_order() {
        for s in [0, 1, 50, 99, 100] {
            for g in [8, 16] {
                let mut last = -1.0;
                for t in 0..2000 {
                    let x = swung_tick(t, 480, s, g);
                    assert!(x >= last, "{s} {g} {t}");
                    last = x;
                }
            }
        }
    }
}
