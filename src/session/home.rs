//! The Home screen's data (`HomeState`): the style's Main patterns, worked out once when
//! the style loads (`Patterns`), and the rest read from the state just built.

use super::Control;
use crate::api::{AppState, HomeFill, HomeLanes, HomeMain, HomeOts, HomeProgress, HomeSend, HomeSnapshot, HomeState};
use crate::engine::{slot_of, Prepared};
use crate::sff::SectionId;

/// GM drum keys for the preview lanes.
const KICKS: [u8; 2] = [35, 36];
const SNARES: [u8; 3] = [38, 39, 40];
const HATS: [u8; 3] = [42, 44, 46];
/// The style parts' channels (0-based): Rhythm 1 and 2, Bass.
const RHYTHM: [u8; 2] = [8, 9];
const BASS: u8 = 10;

/// Main A-D and their fills, as the Home screen previews them.
#[derive(Clone, Debug, Default)]
pub(super) struct Patterns {
    mains: Vec<HomeMain>,
}

impl Patterns {
    /// Work the previews out from the style (control side, at load).
    pub(super) fn of(prep: &Prepared, timesig: (u8, u8)) -> Patterns {
        let (num, den) = (timesig.0.max(1) as u32, timesig.1.max(1) as u32);
        let steps = (num * 16 / den).clamp(1, 64);
        let tpb = prep.tpb.max(1);
        let section = |id: SectionId| prep.sections.get(slot_of(id)).and_then(Option::as_ref);
        let bars = |len: u32| len.div_ceil(tpb).max(1);
        let mains = (0..4u8)
            .map(|m| {
                let main = section(SectionId::Main(m));
                let fill_id = SectionId::Fill(m);
                let fill = section(fill_id);
                let mut h = HomeMain {
                    name: SectionId::Main(m).name(),
                    present: main.is_some(),
                    steps_per_bar: steps as u8,
                    fill: HomeFill { name: fill_id.name(), present: fill.is_some(), bars: fill.map_or(0, |f| bars(f.len)), active: false },
                    ..HomeMain::default()
                };
                let Some(s) = main else { return h };
                h.bars = bars(s.len);
                let n = (h.bars * steps) as usize;
                h.density = vec![0; n];
                let mut lanes = [vec![0u8; steps as usize], vec![0u8; steps as usize], vec![0u8; steps as usize], vec![0u8; steps as usize]];
                for (tick, ch, key, vel) in s.notes() {
                    // The step the note starts on, rounded to the nearest sixteenth.
                    let step = ((tick as u64 * steps as u64 * 2 + tpb as u64) / (2 * tpb as u64)) as usize;
                    if let Some(d) = h.density.get_mut(step % n.max(1)) {
                        *d = d.saturating_add(1);
                    }
                    if step >= steps as usize {
                        continue;
                    }
                    let lane = if RHYTHM.contains(&ch) {
                        if KICKS.contains(&key) {
                            0
                        } else if SNARES.contains(&key) {
                            1
                        } else if HATS.contains(&key) {
                            2
                        } else {
                            continue;
                        }
                    } else if ch == BASS {
                        3
                    } else {
                        continue;
                    };
                    lanes[lane][step] = lanes[lane][step].max(vel);
                }
                let [kick, snare, hats, bass] = lanes;
                h.lanes = HomeLanes { kick, snare, hats, bass };
                h
            })
            .collect();
        Patterns { mains }
    }
}

impl Control {
    /// The Home screen's data, from the state `st` just built.
    pub(super) fn home_state(&self, st: &AppState) -> HomeState {
        let t = &st.transport;
        let mains = self
            .info
            .home
            .mains
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let mut m = m.clone();
                m.current = t.main as usize == i;
                m.fill.active = [&t.section, &t.queued].iter().any(|s| s.as_deref() == Some(m.fill.name.as_str()));
                m
            })
            .collect();
        let bars = t.section_bars;
        let fraction = match bars {
            Some(n) if t.running && n > 0 => {
                let beats = (n * t.beats_per_bar.max(1) as u32) as f64;
                (((t.bar.max(1) - 1) * t.beats_per_bar.max(1) as u32 + t.beat.max(1) - 1) as f64 / beats).clamp(0.0, 1.0)
            }
            _ => 0.0,
        };
        let r = &st.registration;
        let snapshot = r.selected.and_then(|i| {
            let b = r.buttons.get(i as usize).filter(|b| b.stored)?;
            let label = crate::registration::snapshot_label(i as usize);
            let name = if !b.name.is_empty() { b.name.clone() } else { b.style.clone().unwrap_or_else(|| format!("Snapshot {label}")) };
            Some(HomeSnapshot { index: i, label, name, bank: r.bank.name.clone() })
        });
        let o = &st.ots;
        let ots = o.applied.checked_sub(1).and_then(|i| o.settings.get(i as usize).map(|s| HomeOts { index: i, name: s.name.clone() }));
        let band_sends = st
            .effects
            .blocks
            .iter()
            .map(|b| HomeSend { block: b.block, name: b.name.clone(), effect_name: b.effect_name.clone(), level: b.band_send })
            .collect();
        HomeState {
            mains,
            progress: HomeProgress { running: t.running, bar: t.bar, beat: t.beat, bars, beats_per_bar: t.beats_per_bar, fraction },
            snapshot,
            ots,
            band_sends,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::api::{AppCmd, TransportCmd};
    use crate::session::tests::offline;

    #[test]
    fn home_shows_the_mains_their_fills_and_where_the_band_is() {
        let Some(s) = offline("SlowWalker.T552.sty") else { return };
        let st = s.state();
        let h = &st.home;
        assert_eq!(h.mains.len(), 4);
        for (i, m) in h.mains.iter().enumerate() {
            assert_eq!(m.name, format!("Main {}", ['A', 'B', 'C', 'D'][i]));
            assert!(m.present && m.bars > 0);
            assert_eq!(m.steps_per_bar, 16);
            assert_eq!(m.density.len(), (m.bars * 16) as usize);
            assert!(m.density.iter().any(|&d| d > 0), "{} plays notes", m.name);
            assert_eq!(m.lanes.kick.len(), 16);
            assert!(m.fill.present && m.fill.name == format!("Fill In {0}{0}", ['A', 'B', 'C', 'D'][i]));
        }
        assert!(h.mains.iter().any(|m| m.lanes.kick.iter().chain(&m.lanes.snare).chain(&m.lanes.hats).any(|&v| v > 0)), "the drums show: {:?}", h.mains[0].lanes);
        assert!(h.mains.iter().any(|m| m.lanes.bass.iter().any(|&v| v > 0)), "the bass shows");
        assert!(h.mains[st.transport.main as usize].current);
        assert_eq!(h.band_sends.len(), 3);
        assert_eq!(h.band_sends[0].level, st.effects.blocks[0].band_send);
        assert!(h.snapshot.is_none());
        assert!(!h.progress.running && h.progress.fraction == 0.0);
        s.send(AppCmd::Transport(TransportCmd::StartStop)).unwrap();
        s.advance(1_500_000_000);
        let p = s.state().home.progress.clone();
        assert!(p.running && p.bars.is_some() && p.fraction > 0.0 && p.fraction <= 1.0, "{p:?}");
    }
}
