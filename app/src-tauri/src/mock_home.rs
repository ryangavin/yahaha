//! The mock session's Home screen data (`HomeState`, as src/session/home.rs builds it): the
//! mock has no patterns, so each Main present gets a made-up 4-bar groove (kick on 1 and
//! 3, snare on 2 and 4, eighth hats, a bass figure); the rest comes from the state.

use yahaha::api::*;

const LETTERS: [char; 4] = ['A', 'B', 'C', 'D'];

fn lane(steps: &[usize], vel: u8) -> Vec<u8> {
    (0..16).map(|i| if steps.contains(&i) { vel } else { 0 }).collect()
}

pub fn home(st: &AppState) -> HomeState {
    let t = &st.transport;
    let has = |n: &str| st.style.sections.iter().any(|s| s == n);
    let mains = (0..4)
        .map(|i| {
            let name = format!("Main {}", LETTERS[i]);
            let fill = format!("Fill In {0}{0}", LETTERS[i]);
            let present = has(&name);
            let bars = if present { 4 } else { 0 };
            let lanes = if present {
                HomeLanes { kick: lane(&[0, 8], 110), snare: lane(&[4, 12], 100), hats: lane(&[0, 2, 4, 6, 8, 10, 12, 14], 70), bass: lane(&[0, 6, 8, 14], 96) }
            } else {
                HomeLanes::default()
            };
            let density = if present { (0..bars as usize * 16).map(|s| [3, 0, 1, 0, 2, 0, 2, 0][s % 8] + (i as u8 & 1)).collect() } else { vec![] };
            HomeMain {
                present,
                bars,
                steps_per_bar: 16,
                density,
                lanes,
                fill: HomeFill { present: has(&fill), bars: if has(&fill) { 1 } else { 0 }, active: [&t.section, &t.queued].iter().any(|s| s.as_deref() == Some(fill.as_str())), name: fill },
                current: t.main as usize == i,
                name,
            }
        })
        .collect();
    let fraction = match t.section_bars {
        Some(n) if t.running && n > 0 => {
            let b = t.beats_per_bar.max(1) as u32;
            (((t.bar.max(1) - 1) * b + t.beat.max(1) - 1) as f64 / (n * b) as f64).clamp(0.0, 1.0)
        }
        _ => 0.0,
    };
    let ots = st.ots.applied.checked_sub(1).and_then(|i| st.ots.settings.get(i as usize).map(|s| HomeOts { index: i, name: s.name.clone() }));
    HomeState {
        mains,
        progress: HomeProgress { running: t.running, bar: t.bar, beat: t.beat, bars: t.section_bars, beats_per_bar: t.beats_per_bar, fraction },
        ots,
        band_sends: st.effects.blocks.iter().map(|b| HomeSend { block: b.block, name: b.name.clone(), effect_name: b.effect_name.clone(), level: b.band_send }).collect(),
    }
}
