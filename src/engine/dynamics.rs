//! Style Dynamics Control (Genos2 OM p.11, p.69; RM p.11, p.142, p.147), Touch, and Accent
//! (#180).
//!
//! - **Dynamics**: a level 0-127, where 64 plays the Style as written. It scales the
//!   velocity of every Style note-on, on all eight parts. That changes the band's intensity
//!   (the synth's velocity layers and filters), not only its level. CC7 is never touched
//!   (the mixer rule). Style Setting > Dynamics Control (`control`) gates it: off, the Style
//!   plays as written whatever the level.
//! - **Touch**: OM p.69 says the Style's level follows your playing strength. With Touch
//!   on, each strike in the chord section sets the level from its velocity
//!   ([`touch_level`]). A strike at velocity 100 plays the Style as written.
//! - **Accent**: a stand-in for the PSR-SX "Unison & Accent" Accent, which needs accent
//!   data no style yahaha has carries. With it on, a chord-section strike at or above the
//!   threshold, while a Main plays, starts that Main's own Fill In from the next beat
//!   (as Fill Self). It is not a Main press, so OTS Link does not follow it. Nothing
//!   happens during an Intro, fill, break or Ending, or while a change is already queued.
//!
//! The chord-section strikes come from the input thread (`Cmd::Strike`), only while Touch
//! or Accent is on (`Shared::strikes`). Everything here is `Copy` and allocation-free.

use super::*;

/// The Dynamics level that plays the Style as written.
pub const DYNAMICS_NEUTRAL: u8 = 64;
/// The default Accent threshold (velocity).
pub const ACCENT_DEFAULT: u8 = 110;

/// The Dynamics settings (the session keeps them; `Cmd::Dynamics` hands the engine the
/// whole set).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DynamicsSettings {
    /// Style Setting > Dynamics Control: the level may act on the Style.
    pub control: bool,
    /// The Dynamics level, 0-127 (64: as written).
    pub level: u8,
    /// Touch: chord-section strikes set the level.
    pub touch: bool,
    /// Accent: a hard chord-section strike plays the Main's fill.
    pub accent: bool,
    /// The Accent threshold, a velocity 1-127.
    pub accent_min: u8,
}

impl Default for DynamicsSettings {
    fn default() -> DynamicsSettings {
        DynamicsSettings { control: true, level: DYNAMICS_NEUTRAL, touch: false, accent: false, accent_min: ACCENT_DEFAULT }
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
}

/// The level a chord-section strike at velocity `vel` sets with Touch on: velocity − 36,
/// so a strike at 100 plays the Style as written.
pub fn touch_level(vel: u8) -> u8 {
    vel.saturating_sub(36).min(127)
}

/// The velocity factor at Dynamics level `level`: ×0.35 at 0, ×1 at 64, ×1.6 at 127.
fn factor(level: u8) -> f32 {
    let l = level.min(127) as f32;
    let n = DYNAMICS_NEUTRAL as f32;
    if l <= n { 0.35 + 0.65 * l / n } else { 1.0 + 0.6 * (l - n) / (127.0 - n) }
}

/// Engine-side Dynamics state: the settings and the level in effect.
#[derive(Clone, Copy, Debug)]
pub(super) struct Dynamics {
    pub(super) settings: DynamicsSettings,
    /// The factor for the level in effect (1 while Dynamics Control is off).
    factor: f32,
}

impl Default for Dynamics {
    fn default() -> Dynamics {
        Dynamics { settings: DynamicsSettings::default(), factor: 1.0 }
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
    /// sets the level, and a strike at the Accent threshold or above plays the Main's fill.
    pub fn strike(&mut self, vel: u8, now: u64) {
        let s = self.features.dynamics.settings;
        if s.touch {
            self.features.dynamics.settings.level = touch_level(vel);
            self.features.dynamics.update();
        }
        if s.accent && vel >= s.accent_min {
            self.accent_fill(now);
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
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
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
        e.set_chord(crate::parse_chord("C").unwrap(), 0, &mut rec);
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
        assert_eq!((soft.len(), hard.len()), (written.len(), written.len()), "no note is left out");
        for ((&w, &s), &h) in written.iter().zip(&soft).zip(&hard) {
            assert!(s >= 1 && s <= w && h >= w, "{w}: {s} / {h}");
            assert_eq!(s, ((w as f32 * 0.35).round() as u8).max(1));
            assert_eq!(h, ((w as f32 * 1.6).round() as u8).min(127));
        }
        assert_ne!(soft, written, "the level changed the velocities");
    }

    #[test]
    fn touch_sets_the_level_from_the_strike() {
        let Some(mut e) = engine() else { return };
        e.strike(20, 0);
        assert_eq!(e.dynamics().level, DYNAMICS_NEUTRAL, "Touch off: strikes change nothing");
        e.set_dynamics(DynamicsSettings { touch: true, ..Default::default() });
        e.strike(100, 0);
        assert_eq!(e.dynamics().level, 64);
        assert_eq!(e.dynamics_vel(90), 90, "a strike at 100 plays the Style as written");
        e.strike(20, 0);
        assert_eq!(e.dynamics().level, 0);
        assert!(e.dynamics_vel(100) < 40);
        e.strike(127, 0);
        assert_eq!(e.dynamics().level, 91);
        assert!(e.dynamics_vel(100) > 100);
    }

    #[test]
    fn a_hard_strike_plays_the_mains_fill() {
        let Some(mut e) = engine() else { return };
        let mut rec = Rec(Vec::new());
        e.set_dynamics(DynamicsSettings { accent: true, accent_min: 110, ..Default::default() });
        e.strike(127, 0);
        assert_eq!(e.snapshot(0).queued, None, "stopped: nothing to fill");
        e.set_chord(crate::parse_chord("C").unwrap(), 0, &mut rec);
        let now = e.ns_at_bar(1) + e.ns_at_bar(1) / 3;
        e.process(now, &mut rec);
        let main = e.snapshot(now).main;
        let presses = e.snapshot(now).main_presses;
        e.strike(109, now);
        assert_eq!(e.snapshot(now).queued, None, "below the threshold");
        e.strike(110, now);
        let snap = e.snapshot(now);
        assert_eq!(snap.queued, Some(SectionId::Fill(main)), "the Main's own fill");
        assert_eq!(snap.main_presses, presses, "not a Main press: OTS Link does not follow");
        // The fill comes in at the next beat.
        let (at, _) = e.change_point(Change::Fill, now);
        assert!(e.queued.unwrap().at <= at + 1e-6);
        // A second strike while the fill is queued changes nothing.
        e.strike(127, now);
        assert_eq!(e.snapshot(now).queued, Some(SectionId::Fill(main)));
    }

    #[test]
    fn accent_off_or_outside_a_main_does_nothing() {
        let Some(mut e) = engine() else { return };
        let mut rec = Rec(Vec::new());
        e.set_chord(crate::parse_chord("C").unwrap(), 0, &mut rec);
        let now = e.ns_at_bar(1) / 3;
        e.process(now, &mut rec);
        e.strike(127, now);
        assert_eq!(e.snapshot(now).queued, None, "Accent off");
        e.set_dynamics(DynamicsSettings { accent: true, ..Default::default() });
        // A fill playing: no accent on top of it.
        e.button(Button::Main(e.snapshot(now).main), now, &mut rec);
        let bar = e.ns_at_bar(1);
        let mut t = now;
        while e.snapshot(t).cur.is_none_or(|c| !matches!(c, SectionId::Fill(_))) && t < 2 * bar {
            t += 2_000_000;
            e.process(t, &mut rec);
        }
        assert!(matches!(e.snapshot(t).cur, Some(SectionId::Fill(_))), "the fill plays");
        e.strike(127, t);
        assert_eq!(e.snapshot(t).queued, None, "no accent during a fill");
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
    }
}
