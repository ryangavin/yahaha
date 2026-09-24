//! Style changes under stress (#122): "sometimes after changing styles, the accompaniment
//! just stops working". Many style changes across corpus styles, at every timing (stopped,
//! playing on the first beat and mid-bar, mid-fill, during an Ending, an Ending only
//! queued, rapid double changes, a change as the last one takes over), with a chord held.
//! After each change: the band runs when it should, the accompaniment channels (9-16) play
//! note-ons, and through the built-in synth (`offline_audio`) they sound.
//!
//! Every failing case is collected and reported together.

use yahaha::api::*;
use yahaha::session::Port;
use yahaha::{Options, Session};
use std::cell::RefCell;
use std::path::{Path, PathBuf};

const MS: u64 = 1_000_000;
/// One offline step: 10 ms (480 frames at 48 kHz).
const STEP_MS: u64 = 10;
const RATE: u32 = 48_000;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Corpus styles from every folder: every `stride`-th file (sorted), for variety.
fn corpus_styles(stride: usize) -> Vec<PathBuf> {
    let mut v = Vec::new();
    for dir in ["corpus/MOX_v2", "corpus/SX900Style for Genos", "corpus/T5Style"] {
        let Ok(rd) = std::fs::read_dir(root().join(dir)) else { continue };
        let mut files: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| matches!(x.to_ascii_lowercase().to_str(), Some("sty" | "prs" | "sst" | "bcs"))))
            .collect();
        files.sort();
        v.extend(files.into_iter().step_by(stride));
    }
    v
}

/// A tiny xorshift, so every run makes the same choices.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// How the band is heard: MIDI note-ons (the band's output), or the synth's meters.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Ear {
    Midi,
    Audio,
}

/// What the accompaniment did over a stretch of time.
#[derive(Default, Debug)]
struct Heard {
    /// MIDI: note-ons heard (at a level and expression that sound), and all note-ons.
    ons: [u32; 16],
    sent: [u32; 16],
    peak: [f32; 16],
}

impl Heard {
    /// MIDI: the channels that play notes no one hears (level or expression left near 0).
    fn inaudible(&self) -> Vec<usize> {
        (8..16).filter(|&c| self.sent[c] >= 4 && self.ons[c] == 0).map(|c| c + 1).collect()
    }

    fn rhythm(&self, ear: Ear) -> bool {
        match ear {
            Ear::Midi => self.ons[8] + self.ons[9] > 0,
            Ear::Audio => self.peak[8].max(self.peak[9]) > 1e-4,
        }
    }
    /// The chord-following parts (Bass, Chord 1-2, Pad, Phrase 1-2: channels 11-16).
    fn chords(&self, ear: Ear) -> bool {
        match ear {
            Ear::Midi => self.ons[10..16].iter().any(|&n| n > 0),
            Ear::Audio => self.peak[10..16].iter().any(|&p| p > 1e-4),
        }
    }
}

struct Rig {
    s: Session,
    ids: Vec<usize>,
    ear: Ear,
    rng: Rng,
    /// The chord held now (left hand).
    chord: [u8; 3],
    failures: Vec<String>,
    log: Vec<String>,
    /// MIDI: each channel's (CC7, CC11) as sent so far, and the heard note-ons since
    /// `forget`.
    levels: RefCell<[(u8, u8); 16]>,
    ons: RefCell<[u32; 16]>,
    sent: RefCell<[u32; 16]>,
}

const CHORDS: [[u8; 3]; 4] = [[36, 40, 43], [41, 45, 48], [43, 47, 50], [45, 48, 52]];

impl Rig {
    fn new(ear: Ear, stride: usize, seed: u64, data_dir: Option<PathBuf>, sf2: Option<PathBuf>) -> Option<Rig> {
        let paths = corpus_styles(stride);
        if paths.len() < 4 {
            eprintln!("corpus missing; skipping");
            return None;
        }
        let opts = Options { paths, data_dir, sf2: sf2.clone(), ..Options::default() };
        let s = Session::offline(opts).unwrap();
        s.finish_indexing();
        let ids: Vec<usize> = s.library_list().entries.iter().filter(|e| e.status == "ok").map(|e| e.id).collect();
        if ear == Ear::Audio {
            let font = sf2.unwrap_or_else(|| tiny_font_file("main"));
            s.offline_audio(Some(&font), RATE).unwrap();
        }
        let mut r = Rig { s, ids, ear, rng: Rng(seed), chord: CHORDS[0], failures: Vec::new(), log: Vec::new(), levels: RefCell::new([(100, 127); 16]), ons: RefCell::new([0; 16]), sent: RefCell::new([0; 16]) };
        r.hold(CHORDS[0]);
        Some(r)
    }

    /// Release the chord held and hold `c`.
    fn hold(&mut self, c: [u8; 3]) {
        for n in self.chord {
            self.s.midi_in(Port::Keys, &[0x80, n, 0]);
        }
        for n in c {
            self.s.midi_in(Port::Keys, &[0x90, n, 100]);
        }
        self.chord = c;
    }

    fn st(&self) -> std::sync::Arc<AppState> {
        self.s.state()
    }

    /// One 10 ms step: the clock, and the synth's buffers for it (or, MIDI, what the band
    /// sent, heard).
    fn step(&self) {
        self.s.advance(STEP_MS * MS);
        match self.ear {
            Ear::Audio => {
                let _ = self.s.render((RATE as u64 * STEP_MS / 1000) as usize);
            }
            Ear::Midi => self.hear_midi(),
        }
    }

    /// One 1 ms step, heard as `step`.
    fn step_1ms(&self) {
        self.s.advance(MS);
        match self.ear {
            Ear::Audio => {
                let _ = self.s.render((RATE / 1000) as usize);
            }
            Ear::Midi => self.hear_midi(),
        }
    }

    /// The band's MIDI since the last call: each channel's level (CC7) and expression
    /// (CC11) followed, and the note-ons counted that would be heard (a GM receiver's
    /// gain, both squared, above -26 dB). A note-on at expression 8 is not "playing".
    fn hear_midi(&self) {
        let mut lv = self.levels.borrow_mut();
        let mut ons = self.ons.borrow_mut();
        let mut sent = self.sent.borrow_mut();
        for m in self.s.take_output() {
            let ch = (m[0] & 15) as usize;
            match m[0] & 0xF0 {
                0xB0 if m[1] == 7 => lv[ch].0 = m[2],
                0xB0 if m[1] == 11 => lv[ch].1 = m[2],
                // Reset All Controllers: expression back to full.
                0xB0 if m[1] == 121 => lv[ch].1 = 127,
                0x90 if m[2] > 0 => {
                    sent[ch] += 1;
                    let g = |v: u8| (v as f32 / 127.0).powi(2);
                    if g(lv[ch].0) * g(lv[ch].1) > 0.0025 {
                        ons[ch] += 1;
                    }
                }
                _ => {}
            }
        }
    }

    /// Step until `f` holds (at most `max_ms`).
    fn until(&self, max_ms: u64, f: impl Fn(&AppState) -> bool) -> bool {
        for _ in 0..max_ms / STEP_MS {
            if f(&self.st()) {
                return true;
            }
            self.step();
        }
        f(&self.st())
    }

    /// Throw away what was played so far (the MIDI, the meters).
    fn forget(&self) {
        if self.ear == Ear::Midi {
            self.hear_midi();
        }
        *self.ons.borrow_mut() = [0; 16];
        *self.sent.borrow_mut() = [0; 16];
        let _ = self.s.meters();
    }

    /// Step for `ms`, listening to the accompaniment.
    fn listen(&self, ms: u64) -> Heard {
        let mut h = Heard::default();
        for _ in 0..ms / STEP_MS {
            self.step();
            match self.ear {
                Ear::Midi => {}
                Ear::Audio => {
                    for c in self.s.meters().channels {
                        let i = c.channel as usize - 1;
                        h.peak[i] = h.peak[i].max(c.peak);
                    }
                }
            }
        }
        h.ons = std::mem::take(&mut *self.ons.borrow_mut());
        h.sent = std::mem::take(&mut *self.sent.borrow_mut());
        h
    }

    /// Two bars at the current tempo (and time signature), in ms.
    fn two_bars_ms(&self) -> u64 {
        let st = self.st();
        let beats = st.transport.beats_per_bar.max(1) as f64 * 2.0;
        (beats * 60_000.0 / st.transport.tempo.max(20.0)) as u64 + 50
    }

    fn fail(&mut self, what: String) {
        let st = self.st();
        let msg = format!(
            "{what} | style {:?} section {:?} running {} sync_start {} stop_acmp {} queued {:?} bar {} beat {} | after: {}",
            st.style.name,
            st.transport.section,
            st.transport.running,
            st.transport.sync_start,
            st.transport.stop_acmp,
            st.preview.queued,
            st.transport.bar,
            st.transport.beat,
            self.log.iter().rev().take(4).rev().cloned().collect::<Vec<_>>().join(" ; ")
        );
        eprintln!("FAIL: {msg}");
        self.failures.push(msg);
    }

    /// Another style than the one chosen.
    fn other_style(&mut self) -> usize {
        let cur = self.st().preview.queued.unwrap_or(self.st().style.id);
        loop {
            let id = self.ids[self.rng.below(self.ids.len())];
            if id != cur {
                return id;
            }
        }
    }

    fn change(&mut self, id: usize) -> bool {
        let name = self.s.library_list().entries.iter().find(|e| e.id == id).map(|e| e.name.clone()).unwrap_or_default();
        self.log.push(format!("change to {name}"));
        match self.s.send(LibraryCmd::LoadStyle { id }) {
            Ok(()) => true,
            Err(e) => {
                self.log.push(format!("(refused: {e:?})"));
                false
            }
        }
    }

    /// The band is running on style `id` (it took over within `max_ms`).
    fn wait_for(&mut self, id: usize, max_ms: u64, what: &str) -> bool {
        if !self.until(max_ms, |st| st.style.id == id) {
            self.fail(format!("{what}: the new style never took over (still {}, queued {:?})", self.st().style.id, self.st().preview.queued));
            return false;
        }
        true
    }

    /// After a change: the band runs (it should), and the accompaniment plays and sounds.
    fn check_plays(&mut self, what: &str) {
        self.forget();
        let ms = self.two_bars_ms() * 2;
        let h = self.listen(ms);
        let st = self.st();
        if !st.transport.running {
            self.fail(format!("{what}: the band is not running"));
            return;
        }
        let ear = self.ear;
        if !h.rhythm(ear) && !h.chords(ear) {
            self.fail(format!("{what}: the accompaniment is silent ({h:?})"));
        } else if !h.chords(ear) {
            self.fail(format!("{what}: the chord parts are silent with a chord held ({h:?})"));
        } else if !h.inaudible().is_empty() {
            self.fail(format!("{what}: channels {:?} play notes at a level or expression no one hears ((CC7, CC11) {:?}; {h:?})", h.inaudible(), h.inaudible().iter().map(|&c| self.levels.borrow()[c - 1]).collect::<Vec<_>>()));
        }
    }

    /// The band playing (started by StartStop if it's stopped).
    fn ensure_running(&mut self) -> bool {
        if !self.st().transport.running {
            self.log.push("start".into());
            self.s.send(TransportCmd::StartStop).unwrap();
            self.step();
        }
        if !self.until(500, |st| st.transport.running) {
            self.fail("the band would not start".into());
            return false;
        }
        true
    }

    /// The band stopped.
    fn ensure_stopped(&mut self) {
        if self.st().transport.running {
            self.log.push("stop".into());
            self.s.send(TransportCmd::StartStop).unwrap();
            self.step();
        }
    }

    /// A pending change settled either way (the new style took over or the band stopped),
    /// so the next case starts clean.
    fn settle(&mut self) {
        let _ = self.until(30_000, |st| st.preview.queued.is_none());
    }

    // ----- the timings -----

    fn stopped(&mut self) {
        self.ensure_stopped();
        let id = self.other_style();
        if !self.change(id) {
            return;
        }
        self.step();
        if !self.wait_for(id, 100, "stopped") {
            return;
        }
        if self.st().transport.running {
            self.fail("stopped: the change started the band".into());
        }
        self.ensure_running();
        self.check_plays("stopped, then Start");
    }

    /// Stopped, change, then a new chord starts the band (Sync Start).
    fn stopped_sync_start(&mut self) {
        self.ensure_stopped();
        if !self.st().transport.sync_start {
            self.s.send(TransportCmd::ToggleSyncStart).unwrap();
        }
        let id = self.other_style();
        if !self.change(id) {
            return;
        }
        self.step();
        if !self.wait_for(id, 100, "stopped (Sync Start)") {
            return;
        }
        if !self.st().transport.sync_start {
            self.fail("stopped: the change turned Sync Start off".into());
            self.s.send(TransportCmd::ToggleSyncStart).unwrap();
        }
        let next = CHORDS[(CHORDS.iter().position(|c| *c == self.chord).unwrap_or(0) + 1) % CHORDS.len()];
        self.log.push("new chord (Sync Start)".into());
        self.hold(next);
        if !self.until(200, |st| st.transport.running) {
            self.fail("stopped, Sync Start: a new chord did not start the band".into());
            self.ensure_running();
        }
        self.check_plays("stopped, then Sync Start");
    }

    /// Playing: change at beat `beat` of a bar (1 = right after the bar line).
    fn playing_at_beat(&mut self, beat: u32) {
        if !self.ensure_running() {
            return;
        }
        // The Main playing (not an Intro, a Fill or an Ending), at `beat`.
        let _ = self.until(20_000, |st| st.transport.section.as_deref().is_some_and(|s| s.starts_with("Main")));
        let bar = self.st().transport.bar;
        let ok = self.until(8_000, |st| st.transport.beat == beat && (beat != 1 || st.transport.bar != bar));
        if !ok {
            self.log.push(format!("(beat {beat} not reached)"));
        }
        let id = self.other_style();
        if !self.change(id) {
            return;
        }
        let what = format!("playing, beat {beat}");
        if !self.wait_for(id, self.two_bars_ms() * 2, &what) {
            return;
        }
        self.check_plays(&what);
    }

    fn mid_fill(&mut self) {
        if !self.ensure_running() {
            return;
        }
        let _ = self.until(20_000, |st| st.transport.section.as_deref().is_some_and(|s| s.starts_with("Main")));
        self.log.push("fill".into());
        self.s.send(TransportCmd::FillSelf).unwrap();
        if !self.until(8_000, |st| st.transport.section.as_deref().is_some_and(|s| s.starts_with("Fill") || s.starts_with("Break"))) {
            self.log.push("(no fill)".into());
        }
        self.step();
        self.step();
        let id = self.other_style();
        if !self.change(id) {
            return;
        }
        if !self.wait_for(id, self.two_bars_ms() * 2, "mid-fill") {
            return;
        }
        self.check_plays("mid-fill");
    }

    /// During an Ending: the change waits for it; the band stops with the new style.
    fn during_ending(&mut self, queued_only: bool) {
        if !self.ensure_running() {
            return;
        }
        let _ = self.until(20_000, |st| st.transport.section.as_deref().is_some_and(|s| s.starts_with("Main")));
        let _ = self.until(8_000, |st| st.transport.beat == 2);
        self.log.push("ending".into());
        self.s.send(TransportCmd::Ending { index: 0 }).unwrap();
        self.step();
        if !queued_only {
            let _ = self.until(8_000, |st| st.transport.section.as_deref().is_some_and(|s| s.starts_with("Ending")));
            self.step();
        }
        let id = self.other_style();
        if !self.change(id) {
            return;
        }
        let what = if queued_only { "an Ending queued" } else { "during an Ending" };
        if !self.wait_for(id, 40_000, what) {
            return;
        }
        // Stopped by the Ending (or, queued only, possibly still playing it out).
        let _ = self.until(40_000, |st| !st.transport.running);
        self.ensure_running();
        self.check_plays(&format!("{what}, then Start"));
    }

    /// Two changes a few ms apart, the second before the first takes over.
    fn rapid_double(&mut self, playing: bool) {
        if playing {
            if !self.ensure_running() {
                return;
            }
            let _ = self.until(8_000, |st| st.transport.beat == 2);
        } else {
            self.ensure_stopped();
        }
        let a = self.other_style();
        if !self.change(a) {
            return;
        }
        self.step();
        let b = self.other_style();
        if !self.change(b) {
            return;
        }
        let what = if playing { "rapid double (playing)" } else { "rapid double (stopped)" };
        if !self.wait_for(b, self.two_bars_ms() * 2, what) {
            return;
        }
        self.ensure_running();
        self.check_plays(what);
    }

    /// A change as the last one takes over (the step right after the bar line).
    fn change_at_takeover(&mut self) {
        if !self.ensure_running() {
            return;
        }
        let _ = self.until(8_000, |st| st.transport.beat == 2);
        let a = self.other_style();
        if !self.change(a) {
            return;
        }
        if !self.wait_for(a, self.two_bars_ms() * 2, "at takeover (first)") {
            return;
        }
        let b = self.other_style();
        if !self.change(b) {
            return;
        }
        if !self.wait_for(b, self.two_bars_ms() * 2, "at takeover") {
            return;
        }
        self.check_plays("a change as the last one takes over");
    }

    /// Playing, a change and a new chord in the same moment.
    fn change_with_new_chord(&mut self) {
        if !self.ensure_running() {
            return;
        }
        let _ = self.until(8_000, |st| st.transport.beat == 3);
        let id = self.other_style();
        if !self.change(id) {
            return;
        }
        let next = CHORDS[(CHORDS.iter().position(|c| *c == self.chord).unwrap_or(0) + 1) % CHORDS.len()];
        self.log.push("new chord".into());
        self.hold(next);
        if !self.wait_for(id, self.two_bars_ms() * 2, "with a new chord") {
            return;
        }
        self.check_plays("a change with a new chord");
    }

    /// Playing: a new chord a few ms before (or at) the bar line where the new style takes
    /// over, inside the chord-settle window.
    fn chord_at_takeover(&mut self) {
        if !self.ensure_running() {
            return;
        }
        let _ = self.until(20_000, |st| st.transport.section.as_deref().is_some_and(|s| s.starts_with("Main")));
        let _ = self.until(8_000, |st| st.transport.beat == 2);
        let id = self.other_style();
        if !self.change(id) {
            return;
        }
        // The last beat of the bar begins: the bar line is one beat later.
        let last = self.st().transport.beats_per_bar.max(1) as u32;
        let _ = self.until(8_000, |st| st.transport.beat == last);
        let mut n = 0u64;
        while self.st().transport.beat == last && n < 5_000 {
            self.step_1ms();
            n += 1;
        }
        let beat_ms = n.max(1);
        // Now just past the bar line; the next one is a bar away. A style change queued at
        // the first beat waits for it, so aim at the next bar line if it has not taken over.
        if self.st().style.id != id {
            let _ = self.until(8_000, |st| st.transport.beat == last || st.style.id == id);
            let lead = self.rng.below(15) as u64;
            let skip = beat_ms.saturating_sub(lead);
            for _ in 0..skip {
                self.step_1ms();
            }
        }
        let next = CHORDS[(CHORDS.iter().position(|c| *c == self.chord).unwrap_or(0) + 1) % CHORDS.len()];
        self.log.push("new chord at the bar line".into());
        self.hold(next);
        if !self.wait_for(id, self.two_bars_ms() * 2, "chord at the takeover") {
            return;
        }
        self.check_plays("a new chord at the takeover");
    }

    /// Stop Accompaniment on, playing: change.
    fn with_stop_acmp(&mut self) {
        if !self.st().transport.stop_acmp {
            self.log.push("stop acmp on".into());
            self.s.send(TransportCmd::ToggleStopAcmp).unwrap();
        }
        self.playing_at_beat(3);
        if self.st().transport.stop_acmp {
            self.s.send(TransportCmd::ToggleStopAcmp).unwrap();
        }
    }

    fn run(&mut self, cases: usize) {
        for i in 0..cases {
            match self.rng.below(13) {
                0 => self.stopped(),
                1 => self.stopped_sync_start(),
                2 => self.playing_at_beat(1),
                3 => self.playing_at_beat(2),
                4 => self.playing_at_beat(3),
                5 => self.mid_fill(),
                6 => self.during_ending(false),
                7 => self.during_ending(true),
                8 => self.rapid_double(self.rng.0 & 1 == 0),
                9 => self.change_at_takeover(),
                10 => self.change_with_new_chord(),
                11 => self.chord_at_takeover(),
                _ => self.with_stop_acmp(),
            }
            self.settle();
            if i % 8 == 7 {
                let next = CHORDS[self.rng.below(CHORDS.len())];
                self.hold(next);
            }
        }
    }

    fn report(self, what: &str) {
        if !self.failures.is_empty() {
            panic!("{what}: {} failing style changes:\n{}", self.failures.len(), self.failures.join("\n"));
        }
    }
}

/// The tiny test SoundFont (`patches::sf2::tiny_gm_sound_font`) in a file of its own.
fn tiny_font_file(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("yahaha-stress-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("Test.sf2");
    std::fs::write(&p, yahaha::patches::sf2::tiny_gm_sound_font()).unwrap();
    p
}

fn cases(default: usize) -> usize {
    std::env::var("YAHAHA_STRESS_CASES").ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn seed() -> u64 {
    std::env::var("YAHAHA_STRESS_SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(0x5eed_1234_abcd)
}

#[test]
fn style_changes_keep_the_accompaniment_playing() {
    let Some(mut r) = Rig::new(Ear::Midi, 4, seed(), None, None) else { return };
    r.run(cases(160));
    r.report("MIDI");
}

#[test]
fn style_changes_keep_the_accompaniment_sounding() {
    // `YAHAHA_STRESS_SF2=<file.sf2>`: through a real SoundFont (else the tiny test one).
    let sf2 = std::env::var_os("YAHAHA_STRESS_SF2").map(PathBuf::from);
    let Some(mut r) = Rig::new(Ear::Audio, 7, seed() ^ 0x77, None, sf2) else { return };
    r.run(cases(60));
    r.report("audio");
}

/// The sound library in play (#103): every GM family and the drums on patches of a second
/// SoundFont, one family on a patch whose SoundFont is missing, and per-style rules on some
/// styles, so every style change flips the route table's bank and moves channels between
/// the rack's synthesizers.
#[test]
fn style_changes_through_the_sound_library_keep_sounding() {
    let data = std::env::temp_dir().join(format!("yahaha-stress-lib-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data);
    std::fs::create_dir_all(data.join("sf")).unwrap();
    for f in ["Test.sf2", "Other.sf2"] {
        std::fs::write(data.join("sf").join(f), yahaha::patches::sf2::tiny_gm_sound_font()).unwrap();
    }
    let main = data.join("sf").join("Test.sf2");
    let Some(mut r) = Rig::new(Ear::Audio, 9, seed() ^ 0x1b, Some(data.clone()), Some(main)) else { return };
    let patch = |r: &Rig, file: &str, bank: u16, program: u8| {
        let f = PatchFields {
            name: format!("{file} {bank}:{program}"),
            category: yahaha::patches::Category::guess(bank, program),
            tags: vec![],
            favourite: false,
            source: PatchSource::SoundFont { file: file.into(), bank, program },
            defaults: PatchDefaults::default(),
        };
        r.s.send(SoundLibraryCmd::CreatePatch { patch: f }).unwrap();
        r.st().sound_library.last_added.clone().unwrap()
    };
    for family in 0..16u8 {
        let file = if family == 5 { "Missing.sf2" } else { "Other.sf2" };
        let id = patch(&r, file, 0, family * 8);
        r.s.send(SoundLibraryCmd::SetFamilyRule { family, patch: Some(id), style: false }).unwrap();
    }
    let kit = patch(&r, "Other.sf2", 128, 0);
    r.s.send(SoundLibraryCmd::SetDrumRule { patch: Some(kit), style: false }).unwrap();
    // A few styles with rules of their own (back on the main SoundFont).
    for &id in r.ids.clone().iter().step_by(3) {
        r.s.send(LibraryCmd::LoadStyle { id }).unwrap();
        r.step();
        let p = patch(&r, "Test.sf2", 0, 33);
        r.s.send(SoundLibraryCmd::SetFamilyRule { family: 4, patch: Some(p), style: true }).unwrap();
    }
    // The second SoundFont joins the rack (loaded on a thread).
    for _ in 0..500 {
        if r.st().sound_library.extra_sound_fonts.iter().any(|f| f == "Other.sf2") {
            break;
        }
        r.step();
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert!(r.st().sound_library.extra_sound_fonts.iter().any(|f| f == "Other.sf2"), "the library's SoundFont loads");
    r.run(cases(40));
    let _ = std::fs::remove_dir_all(&data);
    r.report("sound library");
}

#[test]
#[ignore]
fn debug_street() {
    let target = std::env::var("DBG_STYLE").unwrap_or("StreetGenos".into());
    for ear in [Ear::Midi, Ear::Audio] {
        for via_ending in [false, true] {
            let paths = vec![root().join("corpus/MOX_v2/SlowWalker.T552.sty"), root().join(format!("corpus/MOX_v2/{target}.T552.sty"))];
            let s = Session::offline(Options { paths, ..Options::default() }).unwrap();
            s.finish_indexing();
            let lib = s.library_list();
            let id = |n: &str| lib.entries.iter().find(|e| e.path.contains(n)).unwrap().id;
            s.send(LibraryCmd::LoadStyle { id: id("SlowWalker") }).unwrap();
            s.advance(10 * MS);
            if ear == Ear::Audio {
                s.offline_audio(Some(&tiny_font_file("dbg")), RATE).unwrap();
            }
            let mut r = Rig { s, ids: vec![], ear, rng: Rng(1), chord: CHORDS[0], failures: vec![], log: vec![], levels: RefCell::new([(100, 127); 16]), ons: RefCell::new([0; 16]), sent: RefCell::new([0; 16]) };
            r.hold(CHORDS[0]);
            r.ensure_running();
            let _ = r.listen(2000);
            if via_ending {
                r.s.send(TransportCmd::Ending { index: 0 }).unwrap();
                r.until(8000, |st| st.transport.section.as_deref().is_some_and(|s| s.starts_with("Ending")));
                r.step();
            }
            let sid = id(&target);
            r.change(sid);
            r.until(40000, |st| st.style.id == sid);
            let _ = r.until(40000, |st| !st.transport.running || !via_ending);
            r.ensure_running();
            r.forget();
            let h = r.listen(8000);
            eprintln!("{ear:?} via_ending={via_ending}: {:?} {:?}", h, r.st().transport.section);
        }
    }
}

#[test]
#[ignore]
fn debug_baseline() {
    for p in corpus_styles(4) {
        let s = Session::offline(Options { paths: vec![p.clone()], ..Options::default() }).unwrap();
        let mut r = Rig { s, ids: vec![], ear: Ear::Midi, rng: Rng(1), chord: CHORDS[0], failures: vec![], log: vec![], levels: RefCell::new([(100, 127); 16]), ons: RefCell::new([0; 16]), sent: RefCell::new([0; 16]) };
        r.hold(CHORDS[0]);
        r.ensure_running();
        r.forget();
        let ms = r.two_bars_ms() * 2;
        let h = r.listen(ms);
        if !h.inaudible().is_empty() {
            eprintln!("BASELINE {}: {:?} levels {:?}", p.file_name().unwrap().to_string_lossy(), h.inaudible(), r.levels.borrow());
        }
    }
}
