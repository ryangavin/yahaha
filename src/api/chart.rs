//! The iReal Pro chart player: imported playlists, the chart playing, chart mode.

use serde::{Deserialize, Serialize};

/// Chart player commands (docs/app-api.md, "Chart player").
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ChartCmd {
    /// Import iReal Pro playlists from text: an `irealb://` / `irealbook://` link, or the
    /// contents of an exported `.html` playlist. Adds them to `chart.playlists`; with no
    /// song chosen yet, chooses the first one imported.
    ImportCharts { text: String },
    /// The same, reading a file (an exported `.html` playlist, or a text file of links).
    ImportChartFile { path: String },
    /// Choose song `song` of playlist `playlist` (0-based): the chart the band plays in
    /// chart mode. Suggests a style (`chart.suggestedStyle`) and loads it when
    /// `chart.autoStyle` is on; stopped, the tempo becomes the chart's (when it has one).
    SelectChart { playlist: usize, song: usize },
    /// The previous / next song of the playlist.
    StepChart { delta: i8 },
    /// Forget an imported playlist.
    RemoveChartPlaylist { playlist: usize },
    /// Chart mode: the band takes its chords and Mains from the chart while it plays.
    SetChartMode { on: bool },
    ToggleChartMode,
    /// Times through the form, 1-99.
    SetChartChoruses { choruses: u32 },
    /// Loop bars `[start, end)` of `chart.song.bars` (0-based, end exclusive) instead of
    /// ending; null: no loop.
    SetChartLoop { range: Option<[u32; 2]> },
    /// The Intro (0-2 = A-C) before the chart; null: none.
    SetChartIntro { index: Option<u8> },
    /// The Ending (0-2 = A-C) after the chart; null: the band stops after the last bar.
    SetChartEnding { index: Option<u8> },
    /// Load the style the chart suggests whenever a song is chosen.
    SetChartAutoStyle { on: bool },
}

/// The chart player (`state.chart`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartState {
    /// Chart mode on.
    pub on: bool,
    /// The imported playlists.
    pub playlists: Vec<ChartPlaylist>,
    /// The song chosen: [playlist, song].
    pub selected: Option<[usize; 2]>,
    /// The chart chosen, expanded into the bars the band plays.
    pub song: Option<ChartSong>,
    pub choruses: u32,
    pub intro: Option<u8>,
    pub ending: Option<u8>,
    /// The loop: bars `[start, end)` of `song.bars`.
    #[serde(rename = "loop")]
    pub loop_range: Option<[u32; 2]>,
    pub auto_style: bool,
    /// The library style the chart's style label suggests (`LibraryEntry::id`).
    pub suggested_style: Option<usize>,
    /// The bar of `song.bars` playing (0-based); null when stopped, in the Intro or Ending,
    /// or with chart mode off.
    pub bar: Option<u32>,
    /// The player's chord has taken over until the next bar line.
    pub overridden: bool,
}

impl Default for ChartState {
    fn default() -> ChartState {
        ChartState {
            on: false,
            playlists: Vec::new(),
            selected: None,
            song: None,
            choruses: 1,
            intro: Some(0),
            ending: Some(0),
            loop_range: None,
            auto_style: true,
            suggested_style: None,
            bar: None,
            overridden: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartPlaylist {
    pub name: String,
    pub songs: Vec<ChartSongInfo>,
}

/// A song as the browser lists it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartSongInfo {
    pub title: String,
    /// As iReal stores it, usually "Last First".
    pub composer: String,
    /// iReal's style label ("Medium Swing", "Bossa Nova").
    pub style: String,
    /// "C", "Eb", "A-" (minor).
    pub key: String,
    /// BPM; null when the chart doesn't say.
    pub tempo: Option<u16>,
}

/// The chart chosen.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartSong {
    #[serde(flatten)]
    pub info: ChartSongInfo,
    /// The form played `choruses` times through, bar by bar.
    pub bars: Vec<ChartBarState>,
    /// Runs of bars in one section (a section mark, or the top of a chorus, starts one).
    pub sections: Vec<ChartSection>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartBarState {
    /// "A", "B", "V" (verse), "i" (the chart's intro), or null before any mark.
    pub section: Option<String>,
    /// The section mark is on this bar.
    pub section_start: bool,
    /// The Main it plays (0-3).
    pub main: u8,
    /// Time signature, e.g. [4, 4].
    pub time: [u8; 2],
    /// 1-based.
    pub chorus: u32,
    /// Chords on beats; a bar with none holds the chord before.
    pub chords: Vec<ChartChordState>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartChordState {
    /// 0-based beat in the bar.
    pub beat: u8,
    /// e.g. "Dm7", "G7/B", "N.C."
    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartSection {
    /// The section letter, or "" before any mark.
    pub label: String,
    pub chorus: u32,
    /// First bar (index into `bars`) and how many.
    pub start: u32,
    pub bars: u32,
}
