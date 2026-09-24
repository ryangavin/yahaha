//! The iReal Pro chart player, control side: imported playlists, the chart chosen, its
//! plan for the engine (engine/chart.rs), the settings, the style it suggests.

use super::Control;
use crate::api::{
    ChartBarState, ChartChordState, ChartCmd, ChartPlaylist, ChartSection, ChartSong, ChartSongInfo, ChartState, CmdError, LibraryCmd,
};
use crate::engine::{ChartPlan, ChartSettings};
use crate::ireal::{self, Bar, Candidate, Playlist, Song};
use crate::library::Info;
use crate::live::Cmd;
use rtrb::{Consumer, Producer};

/// Most times through the form.
pub const MAX_CHORUSES: u32 = 99;

/// The chart player's control-side state (a field of `Control`).
pub(super) struct Charts {
    playlists: Vec<Playlist>,
    selected: Option<(usize, usize)>,
    /// The chosen song, expanded `choruses` times, and its state for clients.
    bars: Vec<Bar>,
    song: Option<ChartSong>,
    choruses: u32,
    settings: ChartSettings,
    auto_style: bool,
    suggested: Option<usize>,
    /// The last plan's tag; the engine's position counts only for it.
    tag: u64,
    tx: Producer<Box<ChartPlan>>,
    old_rx: Consumer<Box<ChartPlan>>,
}

impl Charts {
    pub(super) fn new(tx: Producer<Box<ChartPlan>>, old_rx: Consumer<Box<ChartPlan>>) -> Charts {
        let d = ChartState::default();
        Charts {
            playlists: Vec::new(),
            selected: None,
            bars: Vec::new(),
            song: None,
            choruses: d.choruses,
            settings: ChartSettings { on: d.on, intro: d.intro, ending: d.ending, loop_range: None },
            auto_style: d.auto_style,
            suggested: None,
            tag: 0,
            tx,
            old_rx,
        }
    }
}

fn song_info(s: &Song) -> ChartSongInfo {
    ChartSongInfo {
        title: s.title.clone(),
        composer: s.composer.clone(),
        style: s.style.clone(),
        key: s.key.clone(),
        tempo: (s.tempo > 0).then_some(s.tempo),
    }
}

/// The chart as clients see it: its bars and its sections.
fn song_state(s: &Song, bars: &[Bar], plan: &ChartPlan) -> ChartSong {
    let out: Vec<ChartBarState> = bars
        .iter()
        .zip(&plan.bars)
        .map(|(b, p)| ChartBarState {
            section: b.section.map(|c| c.to_string()),
            section_start: b.section_start,
            main: p.main,
            time: [b.time.0, b.time.1],
            chorus: b.chorus,
            chords: b.chords.iter().map(|c| ChartChordState { beat: c.beat, name: c.chord.name() }).collect(),
        })
        .collect();
    let mut sections: Vec<ChartSection> = Vec::new();
    for (i, (b, p)) in bars.iter().zip(&plan.bars).enumerate() {
        match sections.last_mut() {
            Some(last) if !p.section_start && !(b.section_start && i > 0) => last.bars += 1,
            _ => sections.push(ChartSection {
                label: b.section.map(|c| c.to_string()).unwrap_or_default(),
                chorus: b.chorus,
                start: i as u32,
                bars: 1,
            }),
        }
    }
    ChartSong { info: song_info(s), bars: out, sections }
}

impl Control {
    pub(super) fn chart_cmd(&mut self, c: ChartCmd) -> Result<(), CmdError> {
        match c {
            ChartCmd::ImportCharts { text } => self.import_charts(&text),
            ChartCmd::ImportChartFile { path } => match std::fs::read_to_string(&path) {
                Ok(text) => self.import_charts(&text),
                Err(e) => self.fail(format!("{path}: {e}")),
            },
            ChartCmd::SelectChart { playlist, song } => self.select_chart(playlist, song),
            ChartCmd::StepChart { delta } => {
                let Some((p, s)) = self.charts.selected else { return self.fail("No chart chosen") };
                let n = self.charts.playlists[p].songs.len() as i64;
                let to = (s as i64 + delta as i64).clamp(0, n - 1) as usize;
                if to == s {
                    return Ok(());
                }
                self.select_chart(p, to)
            }
            ChartCmd::RemoveChartPlaylist { playlist } => {
                if playlist >= self.charts.playlists.len() {
                    return self.fail(format!("no playlist {playlist}"));
                }
                self.charts.playlists.remove(playlist);
                match self.charts.selected {
                    Some((p, _)) if p == playlist => self.clear_chart()?,
                    Some((p, s)) if p > playlist => self.charts.selected = Some((p - 1, s)),
                    _ => {}
                }
                Ok(())
            }
            ChartCmd::SetChartMode { on } => self.chart_settings(ChartSettings { on, ..self.charts.settings }),
            ChartCmd::ToggleChartMode => {
                let on = !self.charts.settings.on;
                if on && self.charts.song.is_none() {
                    return self.fail("Import an iReal Pro chart first");
                }
                self.chart_settings(ChartSettings { on, ..self.charts.settings })
            }
            ChartCmd::SetChartChoruses { choruses } => {
                self.charts.choruses = choruses.clamp(1, MAX_CHORUSES);
                match self.charts.selected {
                    Some((p, s)) => self.load_chart(p, s, false),
                    None => Ok(()),
                }
            }
            ChartCmd::SetChartLoop { range } => {
                let n = self.charts.bars.len() as u32;
                let range = match range {
                    Some([a, b]) if a < b && b <= n => Some((a, b)),
                    Some(r) => return self.fail(format!("no bars {}-{} in the chart", r[0] + 1, r[1])),
                    None => None,
                };
                self.chart_settings(ChartSettings { loop_range: range, ..self.charts.settings })
            }
            ChartCmd::SetChartIntro { index } => self.chart_settings(ChartSettings { intro: index.map(|i| i.min(2)), ..self.charts.settings }),
            ChartCmd::SetChartEnding { index } => {
                self.chart_settings(ChartSettings { ending: index.map(|i| i.min(2)), ..self.charts.settings })
            }
            ChartCmd::SetChartAutoStyle { on } => {
                self.charts.auto_style = on;
                Ok(())
            }
        }
    }

    fn chart_settings(&mut self, s: ChartSettings) -> Result<(), CmdError> {
        self.engine_cmd(Cmd::Chart(s))?;
        self.charts.settings = s;
        Ok(())
    }

    fn import_charts(&mut self, text: &str) -> Result<(), CmdError> {
        let lists = match ireal::parse(text) {
            Ok(l) => l,
            Err(e) => return self.fail(format!("iReal import: {e:#}")),
        };
        let first = self.charts.playlists.len();
        let songs: usize = lists.iter().map(|l| l.songs.len()).sum();
        let n = lists.len();
        for (i, mut l) in lists.into_iter().enumerate() {
            if l.name.as_deref().is_none_or(|s| s.trim().is_empty()) {
                l.name = Some(match l.songs.as_slice() {
                    [one] => one.title.clone(),
                    _ => format!("Playlist {}", first + i + 1),
                });
            }
            self.charts.playlists.push(l);
        }
        let mut r = Ok(());
        if self.charts.selected.is_none() && self.charts.playlists.get(first).is_some_and(|l| !l.songs.is_empty()) {
            r = self.select_chart(first, 0);
        }
        if r.is_ok() {
            self.say(format!("Imported {songs} song{} ({n} playlist{})", if songs == 1 { "" } else { "s" }, if n == 1 { "" } else { "s" }), false);
        }
        r
    }

    fn select_chart(&mut self, playlist: usize, song: usize) -> Result<(), CmdError> {
        if self.charts.playlists.get(playlist).and_then(|l| l.songs.get(song)).is_none() {
            return self.fail(format!("no song {song} in playlist {playlist}"));
        }
        self.load_chart(playlist, song, true)
    }

    /// Expand song `song` of `playlist` and hand its plan to the engine. A new song
    /// (`fresh`) drops the loop, suggests a style (and loads it with Auto Style on) and
    /// brings the chart's tempo; a new chorus count keeps them.
    fn load_chart(&mut self, playlist: usize, song: usize, fresh: bool) -> Result<(), CmdError> {
        let s = self.charts.playlists[playlist].songs[song].clone();
        let bars = s.bars(self.charts.choruses);
        let tag = self.charts.tag + 1;
        let bpm = (fresh && s.tempo > 0).then_some(s.tempo as f64);
        let plan = ChartPlan::from_bars(&bars, tag, bpm);
        let state = song_state(&s, &bars, &plan);
        if fresh {
            self.charts.suggested = self.suggest_style(&s, &bars);
            if let (true, Some(id)) = (self.charts.auto_style, self.charts.suggested) {
                // The style goes in first, so a stopped band takes the chart's tempo after
                // the style's own.
                let _ = self.library_cmd(LibraryCmd::LoadStyle { id });
            }
        }
        self.charts.tx.push(Box::new(plan)).map_err(|_| CmdError::Busy)?;
        self.wake_engine();
        self.charts.tag = tag;
        self.charts.selected = Some((playlist, song));
        self.charts.bars = bars;
        self.charts.song = Some(state);
        if fresh && self.charts.settings.loop_range.is_some() {
            self.chart_settings(ChartSettings { loop_range: None, ..self.charts.settings })?;
        }
        Ok(())
    }

    /// No chart: an empty plan, chart mode off.
    fn clear_chart(&mut self) -> Result<(), CmdError> {
        let tag = self.charts.tag + 1;
        self.charts.tx.push(Box::new(ChartPlan { tag, ..ChartPlan::default() })).map_err(|_| CmdError::Busy)?;
        self.charts.tag = tag;
        self.charts.selected = None;
        self.charts.bars.clear();
        self.charts.song = None;
        self.charts.suggested = None;
        self.chart_settings(ChartSettings { on: false, loop_range: None, ..self.charts.settings })
    }

    /// The library style the song's style label suggests.
    fn suggest_style(&self, s: &Song, bars: &[Bar]) -> Option<usize> {
        let beats = bars.first().map_or(4, |b| b.time.0);
        let lib = &self.lib;
        let cands = (0..lib.len()).filter_map(|id| {
            let e = lib.entry(id);
            let (beats, bpm) = match &e.info {
                Info::Ok(sm) => (Some(sm.timesig.0), Some(sm.bpm)),
                Info::Err(_) => return None,
                Info::Pending => (None, None),
            };
            Some(Candidate { id, name: e.name(), folder: &e.folder, beats, bpm })
        });
        ireal::suggest_style(&s.style, &s.groove, beats, (s.tempo > 0).then_some(s.tempo), cands)
    }

    /// Replaced plans come back from the engine to be freed here.
    pub(super) fn pump_chart(&mut self) {
        while self.charts.old_rx.pop().is_ok() {}
    }

    pub(super) fn chart_state(&self) -> ChartState {
        let c = &self.charts;
        let s = &self.snap;
        let current = s.chart_tag == c.tag && c.song.is_some();
        ChartState {
            on: c.settings.on,
            playlists: c
                .playlists
                .iter()
                .map(|l| ChartPlaylist { name: l.name.clone().unwrap_or_default(), songs: l.songs.iter().map(song_info).collect() })
                .collect(),
            selected: c.selected.map(|(p, s)| [p, s]),
            song: c.song.clone(),
            choruses: c.choruses,
            intro: c.settings.intro,
            ending: c.settings.ending,
            loop_range: c.settings.loop_range.map(|(a, b)| [a, b]),
            auto_style: c.auto_style,
            suggested_style: c.suggested,
            bar: s.chart_bar.filter(|_| current),
            overridden: s.chart_override && current,
        }
    }
}
