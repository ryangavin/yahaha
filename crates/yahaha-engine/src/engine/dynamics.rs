//! Style Dynamics Control (Genos2 OM p.11, p.69; RM p.11, p.142, p.147), Touch, and Accent
//! (#180).
//!
//! - **Dynamics**: a level 0-127. The maximum, 127, plays the Style as written and is the
//!   default (the owner: Dynamics "should always be maxed out"); lower levels soften the
//!   band down to ×0.35. There is no boost above the Style as authored, so the maximum
//!   never pins velocities at 127. It scales the velocity of every Style note-on, on all
//!   eight parts. That changes the band's intensity
//!   (the synth's velocity layers and filters), not only its level. CC7 is never touched
//!   (the mixer rule). Style Setting > Dynamics Control (`control`) gates it: off, the Style
//!   plays as written whatever the level.
//! - **Touch**: OM p.69 says the Style's level follows your playing strength. With Touch
//!   on, each strike in the chord section sets the level from its velocity
//!   ([`touch_level`]). A strike at velocity 100 or more plays the Style as written.
//! - **Accent**: a stand-in for the PSR-SX "Unison & Accent" Accent, which needs accent
//!   data no style yahaha has carries. With it on, a strike at or above the threshold
//!   (a chord-section strike; with Source Both, a right-hand strike too) does one of:
//!   - **Hits** (the default Mode), running or stopped: a one-shot hit from the Style's
//!     drum kit on its drum channel (Rhythm 2), picked by the strike's velocity band
//!     ([`accent_hit`]): kick + closed hat, kick + snare, or from 120 kick + crash, each
//!     note's velocity scaled by the strike's. Each note retriggers (its note-off first),
//!     never stacks, and ends `HIT_LEN_NS` later.
//!   - **Fill** (while a Main plays): that Main's own Fill In from the next beat (as Fill
//!     Self). It is not a Main press, so OTS Link does not follow it. Nothing happens
//!     during an Intro, fill, break or Ending, or while a change is already queued. With
//!     the style stopped, Fill mode plays the hits (there is no fill to play).
//!
//! The strikes come from the input thread (`Cmd::Strike`, `Cmd::AccentStrike`), only while
//! Touch or Accent is on (`Shared::strikes`, `Shared::strikes_right`). Everything here is
//! `Copy` and allocation-free.

use super::*;

/// The Dynamics level that plays the Style as written: the maximum, and the default.
pub const DYNAMICS_NEUTRAL: u8 = 127;
/// The default Accent threshold (velocity).
pub const ACCENT_DEFAULT: u8 = 110;
/// The strike velocity from which an Accent hit is kick + crash.
pub const ACCENT_CRASH: u8 = 120;
/// How long an Accent hit's notes sound before their note-off.
const HIT_LEN_NS: u64 = 150_000_000;
/// The Style's drum channel (Rhythm 2, part bit 1).
const DRUM_CH: u8 = 9;
/// GM drum notes.
const KICK: u8 = 36;
const SNARE: u8 = 38;
const CLOSED_HAT: u8 = 42;
const CRASH: u8 = 49;
/// Every note an Accent hit can play (one pending note-off each).
const HIT_NOTES: [u8; 4] = [KICK, SNARE, CLOSED_HAT, CRASH];

/// What an Accent strike does while a Main plays.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AccentMode {
    /// A one-shot drum hit (also with the style stopped).
    #[default]
    Hits,
    /// The Main's own fill (Fill Self); hits while stopped.
    Fill,
}

/// Which strikes Accent hears.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AccentSource {
    /// The chord section only.
    #[default]
    Left,
    /// The chord section and the right hand.
    Both,
}

/// The drum notes (key, velocity) an Accent hit at strike velocity `vel` plays, with
/// threshold `min`: below the midpoint of `min`..`ACCENT_CRASH` kick + closed hat, above
/// it kick + snare, from `ACCENT_CRASH` kick + crash. Velocities scale with the strike.
pub fn accent_hit(vel: u8, min: u8) -> [(u8, u8); 2] {
    let scale = |base: u16| ((base * vel.min(127) as u16 / 127) as u8).max(1);
    let mid = min as u16 + (ACCENT_CRASH as u16).saturating_sub(min as u16) / 2;
    let other = if vel >= ACCENT_CRASH {
        (CRASH, scale(120))
    } else if vel as u16 >= mid {
        (SNARE, scale(120))
    } else {
        (CLOSED_HAT, scale(96))
    };
    [(KICK, scale(127)), other]
}

/// The Dynamics settings (the session keeps them; `Cmd::Dynamics` hands the engine the
/// whole set).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DynamicsSettings {
    /// Style Setting > Dynamics Control: the level may act on the Style.
    pub control: bool,
    /// The Dynamics level, 0-127 (127, the default: as written).
    pub level: u8,
    /// Touch: chord-section strikes set the level.
    pub touch: bool,
    /// Accent: a hard chord-section strike plays the Main's fill.
    pub accent: bool,
    /// The Accent threshold, a velocity 1-127.
    pub accent_min: u8,
    /// Accent Mode: hits (default) or the Main's fill while a Main plays.
    pub accent_mode: AccentMode,
    /// Accent Source: the chord section (default) or both hands.
    pub accent_source: AccentSource,
}

impl Default for DynamicsSettings {
    fn default() -> DynamicsSettings {
        DynamicsSettings { control: true, level: DYNAMICS_NEUTRAL, touch: false, accent: false, accent_min: ACCENT_DEFAULT, accent_mode: AccentMode::Hits, accent_source: AccentSource::Left }
    }
}

impl DynamicsSettings {
    /// The settings with every value in range.
    pub fn clamped(self) -> DynamicsSettings {
        DynamicsSettings { level: self.level.min(127), accent_min: self.accent_min.clamp(1, 127), ..self }
    }

    /// The input thread must send chord-section strikes.
    pub fn wants_strikes(&self) -> bool {
        self.touch || self.accent
    }

    /// The input thread must send right-hand strikes (Accent with Source Both).
    pub fn wants_right_strikes(&self) -> bool {
        self.accent && self.accent_source == AccentSource::Both
    }
}

/// The level a chord-section strike at velocity `vel` sets with Touch on: velocity × 1.27,
/// so a strike at 100 or harder plays the Style as written.
pub fn touch_level(vel: u8) -> u8 {
    (vel as u16 * 127 / 100).min(127) as u8
}

/// The velocity factor at Dynamics level `level`: ×0.35 at 0 up to ×1 (as written) at 127.
fn factor(level: u8) -> f32 {
    0.35 + 0.65 * level.min(127) as f32 / 127.0
}

/// Engine-side Dynamics state: the settings and the level in effect.
#[derive(Clone, Copy, Debug)]
pub(super) struct Dynamics {
    pub(super) settings: DynamicsSettings,
    /// The factor for the level in effect (1 while Dynamics Control is off).
    factor: f32,
    /// Accent hits: each note's pending note-off time (0 = none), by `HIT_NOTES` index.
    hit_off: [u64; HIT_NOTES.len()],
}

impl Default for Dynamics {
    fn default() -> Dynamics {
        Dynamics { settings: DynamicsSettings::default(), factor: 1.0, hit_off: [0; HIT_NOTES.len()] }
    }
}

impl Dynamics {
    fn update(&mut self) {
        self.factor = if self.settings.control { factor(self.settings.level) } else { 1.0 };
    }
}

impl Engine {
    /// New Dynamics settings (`Cmd::Dynamics`).
    pub fn set_dynamics(&mut self, s: DynamicsSettings) {
        let d = &mut self.features.dynamics;
        d.settings = s.clamped();
        d.update();
    }

    /// The Dynamics level from a Dynamics Control pedal (`Cmd::DynamicsLevel`). With
    /// Dynamics Control off it is kept but does nothing, as any level.
    pub fn set_dynamics_level(&mut self, level: u8) {
        let d = &mut self.features.dynamics;
        d.settings.level = level.min(127);
        d.update();
    }

    /// The Dynamics settings in effect; `level` is the level now (Touch moves it).
    pub fn dynamics(&self) -> DynamicsSettings {
        self.features.dynamics.settings
    }

    /// A Style note-on's velocity after Dynamics.
    #[inline]
    pub(super) fn dynamics_vel(&self, vel: u8) -> u8 {
        let f = self.features.dynamics.factor;
        if f == 1.0 {
            return vel;
        }
        (vel as f32 * f).round().clamp(1.0, 127.0) as u8
    }

    /// A key in the chord section went down with velocity `vel` (`Cmd::Strike`): Touch
    /// sets the level, and a strike at the Accent threshold or above accents.
    pub fn strike(&mut self, vel: u8, now: u64, sink: &mut impl Sink) {
        let s = self.features.dynamics.settings;
        if s.touch {
            self.features.dynamics.settings.level = touch_level(vel);
            self.features.dynamics.update();
        }
        self.accent(vel, now, sink);
    }

    /// A right-hand key went down with velocity `vel` (`Cmd::AccentStrike`): with Accent
    /// Source Both, it accents as a chord-section strike does (Touch does not hear it).
    pub fn accent_strike(&mut self, vel: u8, now: u64, sink: &mut impl Sink) {
        if self.features.dynamics.settings.accent_source == AccentSource::Both {
            self.accent(vel, now, sink);
        }
    }

    fn accent(&mut self, vel: u8, now: u64, sink: &mut impl Sink) {
        let s = self.features.dynamics.settings;
        if !s.accent || vel < s.accent_min {
            return;
        }
        if s.accent_mode == AccentMode::Fill && self.running {
            self.accent_fill(now);
        } else {
            self.accent_hit(vel, now, sink);
        }
    }

    /// Accent Hits: the drum notes for `vel` on the Style's drum channel, each retriggered
    /// (its note-off first) and ended `HIT_LEN_NS` later (`accent_due`). Nothing while
    /// Rhythm 2 is muted (or another part is soloed).
    fn accent_hit(&mut self, vel: u8, now: u64, sink: &mut impl Sink) {
        if self.audible() & (1 << (DRUM_CH - 8)) == 0 {
            return;
        }
        let min = self.features.dynamics.settings.accent_min;
        for (key, v) in accent_hit(vel, min) {
            let Some(i) = HIT_NOTES.iter().position(|&n| n == key) else { continue };
            sink.send(&[0x80 | DRUM_CH, key, 0]);
            sink.send(&[0x90 | DRUM_CH, key, v]);
            self.features.dynamics.hit_off[i] = now.saturating_add(HIT_LEN_NS).max(1);
        }
    }

    /// Accent hits' note-offs that are due by `now` (each wake, running or stopped).
    pub(super) fn accent_due(&mut self, now: u64, sink: &mut impl Sink) {
        for (i, t) in self.features.dynamics.hit_off.iter_mut().enumerate() {
            if *t != 0 && *t <= now {
                *t = 0;
                sink.send(&[0x80 | DRUM_CH, HIT_NOTES[i], 0]);
            }
        }
    }

    /// Accent: the Main playing plays its own fill from the next beat, unless a change is
    /// already queued. Not a Main press (OTS Link does not follow it).
    fn accent_fill(&mut self, now: u64) {
        if !self.running || self.queued.is_some() {
            return;
        }
        let SectionId::Main(m) = id_of(self.cur) else { return };
        if let Some(f) = self.style.resolve(8 + m as usize) {
            self.queue_fill(f, now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Rec(Vec<[u8; 3]>);
    impl Sink for Rec {
        fn send(&mut self, m: &[u8]) {
            if m.len() == 3 {
                self.0.push([m[0], m[1], m[2]]);
            }
        }
    }

    fn engine() -> Option<Engine> {
        let p = yahaha_sff::library::corpus_dir().join("MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return None;
        }
        Some(Engine::new(Box::new(Prepared::new(&Style::load(&p).unwrap()))))
    }

    /// The note-on velocities the band plays over the first two bars at the settings `s`.
    fn velocities(s: DynamicsSettings) -> Option<Vec<u8>> {
        let mut e = engine()?;
        e.set_dynamics(s);
        let mut rec = Rec(Vec::new());
        e.set_chord(yahaha_core::parse_chord("C").unwrap(), 0, &mut rec);
        let end = e.ns_at_bar(2);
        let mut now = 0;
        while now < end {
            e.process(now, &mut rec);
            now += 2_000_000;
        }
        Some(rec.0.iter().filter(|m| m[0] & 0xF0 == 0x90 && m[2] > 0).map(|m| m[2]).collect())
    }

    #[test]
    fn the_neutral_level_plays_the_style_as_written() {
        let Some(off) = velocities(DynamicsSettings { control: false, ..Default::default() }) else { return };
        assert!(!off.is_empty());
        assert_eq!(velocities(DynamicsSettings::default()).unwrap(), off);
        // Dynamics Control off: the level does nothing.
        assert_eq!(velocities(DynamicsSettings { control: false, level: 0, ..Default::default() }).unwrap(), off);
    }

    #[test]
    fn the_level_scales_every_style_note_on() {
        let Some(written) = velocities(DynamicsSettings::default()) else { return };
        let soft = velocities(DynamicsSettings { level: 0, ..Default::default() }).unwrap();
        let hard = velocities(DynamicsSettings { level: 127, ..Default::default() }).unwrap();
        let mid = velocities(DynamicsSettings { level: 64, ..Default::default() }).unwrap();
        assert_eq!((soft.len(), hard.len()), (written.len(), written.len()), "no note is left out");
        assert_eq!(hard, written, "the maximum plays the Style as written: no boost, no pinning at 127");
        for ((&w, &s), &m) in written.iter().zip(&soft).zip(&mid) {
            assert!(s >= 1 && s <= m && m <= w, "{w}: {s} / {m}");
            assert_eq!(s, ((w as f32 * 0.35).round() as u8).max(1));
        }
        assert_ne!(soft, written, "the level changed the velocities");
    }

    #[test]
    fn touch_sets_the_level_from_the_strike() {
        let Some(mut e) = engine() else { return };
        let mut rec = Rec(Vec::new());
        e.strike(20, 0, &mut rec);
        assert_eq!(e.dynamics().level, DYNAMICS_NEUTRAL, "Touch off: strikes change nothing");
        e.set_dynamics(DynamicsSettings { touch: true, ..Default::default() });
        e.strike(100, 0, &mut rec);
        assert_eq!(e.dynamics().level, 127);
        assert_eq!(e.dynamics_vel(90), 90, "a strike at 100 plays the Style as written");
        e.strike(20, 0, &mut rec);
        assert_eq!(e.dynamics().level, 25);
        assert!(e.dynamics_vel(100) < 50);
        e.strike(127, 0, &mut rec);
        assert_eq!(e.dynamics().level, 127);
        assert_eq!(e.dynamics_vel(100), 100, "never above the Style as written");
    }

    #[test]
    fn fill_mode_plays_the_mains_fill() {
        let Some(mut e) = engine() else { return };
        let mut rec = Rec(Vec::new());
        e.set_dynamics(DynamicsSettings { accent: true, accent_min: 110, accent_mode: AccentMode::Fill, ..Default::default() });
        e.strike(127, 0, &mut rec);
        assert_eq!(e.snapshot(0).queued, None, "stopped: nothing to fill");
        e.set_chord(yahaha_core::parse_chord("C").unwrap(), 0, &mut rec);
        let now = e.ns_at_bar(1) + e.ns_at_bar(1) / 3;
        e.process(now, &mut rec);
        let main = e.snapshot(now).main;
        let presses = e.snapshot(now).main_presses;
        e.strike(109, now, &mut rec);
        assert_eq!(e.snapshot(now).queued, None, "below the threshold");
        let before = rec.0.len();
        e.strike(110, now, &mut rec);
        assert_eq!(rec.0.len(), before, "Fill mode while a Main plays: no hits");
        let snap = e.snapshot(now);
        assert_eq!(snap.queued, Some(SectionId::Fill(main)), "the Main's own fill");
        assert_eq!(snap.main_presses, presses, "not a Main press: OTS Link does not follow");
        // The fill comes in at the next beat.
        let (at, _) = e.change_point(Change::Fill, now);
        assert!(e.queued.unwrap().at <= at + 1e-6);
        // A second strike while the fill is queued changes nothing.
        e.strike(127, now, &mut rec);
        assert_eq!(e.snapshot(now).queued, Some(SectionId::Fill(main)));
    }

    #[test]
    fn accent_off_or_outside_a_main_does_nothing() {
        let Some(mut e) = engine() else { return };
        let mut rec = Rec(Vec::new());
        e.set_chord(yahaha_core::parse_chord("C").unwrap(), 0, &mut rec);
        let now = e.ns_at_bar(1) / 3;
        e.process(now, &mut rec);
        e.strike(127, now, &mut rec);
        assert_eq!(e.snapshot(now).queued, None, "Accent off");
        e.set_dynamics(DynamicsSettings { accent: true, accent_mode: AccentMode::Fill, ..Default::default() });
        // A fill playing: no accent on top of it.
        e.button(Button::Main(e.snapshot(now).main), now, &mut rec);
        let bar = e.ns_at_bar(1);
        let mut t = now;
        while e.snapshot(t).cur.is_none_or(|c| !matches!(c, SectionId::Fill(_))) && t < 2 * bar {
            t += 2_000_000;
            e.process(t, &mut rec);
        }
        assert!(matches!(e.snapshot(t).cur, Some(SectionId::Fill(_))), "the fill plays");
        e.strike(127, t, &mut rec);
        assert_eq!(e.snapshot(t).queued, None, "no accent during a fill");
    }

    #[test]
    fn velocity_bands_pick_the_hit() {
        assert_eq!(accent_hit(110, 110), [(36, 110), (42, 83)], "medium hard: kick + closed hat");
        assert_eq!(accent_hit(114, 110)[1].0, 42);
        assert_eq!(accent_hit(115, 110), [(36, 115), (38, 108)], "hard: kick + snare");
        assert_eq!(accent_hit(119, 110)[1].0, 38);
        assert_eq!(accent_hit(120, 110), [(36, 120), (49, 113)], "very hard: kick + crash");
        assert_eq!(accent_hit(127, 110), [(36, 127), (49, 120)]);
        // A threshold at or above 120: every accent is a crash.
        assert_eq!(accent_hit(125, 125)[1].0, 49);
        // A low threshold: the bands spread.
        assert_eq!(accent_hit(80, 80)[1].0, 42);
        assert_eq!(accent_hit(100, 80)[1].0, 38);
        assert!(accent_hit(1, 1).iter().all(|&(_, v)| v >= 1));
    }

    /// Drum messages on channel 10.
    fn drums(rec: &Rec) -> Vec<[u8; 3]> {
        rec.0.iter().filter(|m| m[0] & 0x0F == 9 && m[0] & 0xE0 == 0x80).copied().collect()
    }

    #[test]
    fn stopped_style_hits_play_and_retrigger() {
        let Some(mut e) = engine() else { return };
        let mut rec = Rec(Vec::new());
        e.set_dynamics(DynamicsSettings { accent: true, accent_min: 110, ..Default::default() });
        e.strike(109, 0, &mut rec);
        assert!(drums(&rec).is_empty(), "below the threshold");
        e.strike(127, 0, &mut rec);
        assert_eq!(drums(&rec), [[0x89, 36, 0], [0x99, 36, 127], [0x89, 49, 0], [0x99, 49, 120]]);
        assert_eq!(e.snapshot(0).queued, None, "stopped: no fill");
        // Struck again before the note-off: retriggered, and only one note-off is due.
        rec.0.clear();
        e.strike(112, 1_000_000, &mut rec);
        assert_eq!(drums(&rec), [[0x89, 36, 0], [0x99, 36, 112], [0x89, 42, 0], [0x99, 42, 84]]);
        rec.0.clear();
        e.process(1_000_000 + HIT_LEN_NS, &mut rec);
        assert_eq!(drums(&rec), [[0x89, 36, 0], [0x89, 42, 0], [0x89, 49, 0]]);
        rec.0.clear();
        e.process(2 * HIT_LEN_NS, &mut rec);
        assert!(drums(&rec).is_empty(), "each note-off once");
        // Fill mode, stopped: still hits.
        e.set_dynamics(DynamicsSettings { accent_mode: AccentMode::Fill, ..e.dynamics() });
        e.strike(127, 0, &mut rec);
        assert_eq!(drums(&rec).len(), 4);
        // Rhythm 2 muted: nothing.
        e.set_style_parts(0xFF & !(1 << 1), &mut rec);
        rec.0.clear();
        e.strike(127, 0, &mut rec);
        assert!(drums(&rec).is_empty());
    }

    #[test]
    fn hits_play_while_the_style_plays_and_right_strikes_need_both() {
        let Some(mut e) = engine() else { return };
        let mut rec = Rec(Vec::new());
        e.set_dynamics(DynamicsSettings { accent: true, accent_min: 110, ..Default::default() });
        e.set_chord(yahaha_core::parse_chord("C").unwrap(), 0, &mut rec);
        let now = e.ns_at_bar(1) / 3;
        e.process(now, &mut rec);
        rec.0.clear();
        e.strike(127, now, &mut rec);
        assert!(drums(&rec).contains(&[0x99, 49, 120]), "hits while playing");
        assert_eq!(e.snapshot(now).queued, None, "Hits mode: no fill");
        rec.0.clear();
        e.accent_strike(127, now, &mut rec);
        assert!(drums(&rec).is_empty(), "Source Left: the right hand does not accent");
        e.set_dynamics(DynamicsSettings { accent_source: AccentSource::Both, ..e.dynamics() });
        e.accent_strike(127, now, &mut rec);
        assert!(drums(&rec).contains(&[0x99, 36, 127]), "Source Both");
    }

    #[test]
    fn a_pedal_sets_the_level() {
        let Some(mut e) = engine() else { return };
        e.set_dynamics_level(0);
        assert_eq!(e.dynamics().level, 0);
        assert!(e.dynamics_vel(100) < 40);
        e.set_dynamics_level(200);
        assert_eq!(e.dynamics().level, 127);
        e.set_dynamics(DynamicsSettings { control: false, ..e.dynamics() });
        e.set_dynamics_level(0);
        assert_eq!(e.dynamics_vel(100), 100, "Dynamics Control off: the pedal does nothing");
    }

    #[test]
    fn settings_clamp() {
        let s = DynamicsSettings { level: 200, accent_min: 0, ..Default::default() }.clamped();
        assert_eq!((s.level, s.accent_min), (127, 1));
        assert!(!DynamicsSettings::default().wants_strikes());
        assert!(DynamicsSettings { accent: true, ..Default::default() }.wants_strikes());
        assert!(!DynamicsSettings { accent: true, ..Default::default() }.wants_right_strikes());
        assert!(DynamicsSettings { accent: true, accent_source: AccentSource::Both, ..Default::default() }.wants_right_strikes());
    }
}
