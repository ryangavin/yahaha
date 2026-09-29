//! The Master Compressor and Master EQ (`crate::fx::master`): the control side's part.
//!
//! They are a setup setting, as on the Genos, where they stay as set until changed (the
//! Genos keeps its own types in its User Effect memory): kept in
//! `<data>/master-effects.json`, never in a rack, and read at start. A missing or
//! unreadable file, or a missing field, reads as its default (both off), so a data folder
//! from before them loads as it always did.

use super::Control;
use crate::api::{CmdError, FxCmd, MasterSettings};
use std::path::{Path, PathBuf};

/// The settings' file in the data folder.
const FILE: &str = "master-effects.json";

/// The master chain's settings and where they are saved.
#[derive(Default)]
pub(super) struct MasterFile {
    /// None: not saved (sessions without a data folder).
    path: Option<PathBuf>,
    pub(super) settings: MasterSettings,
}

impl MasterFile {
    /// The settings saved in `data_dir` (both off when there are none).
    pub(super) fn load(data_dir: Option<&Path>) -> MasterFile {
        let Some(dir) = data_dir else { return MasterFile::default() };
        let path = dir.join(FILE);
        let settings = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str::<MasterSettings>(&t).ok()).unwrap_or_default();
        MasterFile { path: Some(path), settings }
    }

    fn save(&self) -> anyhow::Result<()> {
        let Some(path) = &self.path else { return Ok(()) };
        crate::data_files::write_atomic(path, &serde_json::to_string_pretty(&self.settings)?)
    }
}

impl Control {
    /// A Master Compressor or Master EQ command (None: `c` is another effects command).
    pub(super) fn master_fx_cmd(&mut self, c: &FxCmd) -> Option<Result<(), CmdError>> {
        let r = match self.master.settings.apply(c)? {
            Err(e) => self.fail(e),
            Ok(()) => {
                self.pump_master_fx();
                match self.master.save() {
                    Ok(()) => Ok(()),
                    Err(e) => self.fail(format!("saving the master effects: {e:#}")),
                }
            }
        };
        Some(r)
    }

    /// Keep the audio thread at the settings: the compressor's atomics, and the EQ's
    /// coefficients when they changed (`MasterControl::set_eq` compares).
    pub(super) fn pump_master_fx(&self) {
        let Some(synth) = self.synth.as_ref() else { return };
        let m = &synth.control.fx.master;
        m.set_compressor(&self.master.settings.compressor);
        m.set_eq(&self.master.settings.eq);
    }
}

#[cfg(test)]
mod tests {
    use crate::api::{CompParam, CompPreset, EqBand, EqPreset, FxCmd};
    use crate::session::{Options, Session};
    use std::sync::atomic::Ordering::Relaxed;

    fn data_dir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("yahaha-master-fx-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn session(data: &std::path::Path) -> Session {
        let p = crate::session::testing::style_path();
        let s = Session::offline(Options { paths: vec![p], data_dir: Some(data.to_path_buf()), ..Options::default() }).unwrap();
        s.offline_audio(None, 48_000).unwrap();
        s
    }

    /// The commands reach the state and the audio thread's settings, and are saved: a new
    /// session in the same data folder starts with them.
    #[test]
    fn master_effects_reach_the_bus_and_are_kept() {
        let data = data_dir("kept");
        let s = session(&data);
        let st = s.state().effects.master.clone();
        assert!(!st.compressor.on && !st.eq.on, "both off by default");
        assert_eq!((st.compressor.preset, st.eq.preset, st.eq.bands.len()), (CompPreset::Natural, EqPreset::Flat, 8));

        s.send(FxCmd::SetMasterCompressorOn { on: true }).unwrap();
        s.send(FxCmd::SetMasterCompressorPreset { preset: CompPreset::Punchy }).unwrap();
        s.send(FxCmd::SetMasterCompressorParam { param: CompParam::Output, value: 40 }).unwrap();
        s.send(FxCmd::SetMasterEqOn { on: true }).unwrap();
        s.send(FxCmd::SetMasterEqPreset { preset: EqPreset::Bright }).unwrap();
        s.send(FxCmd::SetMasterEqBand { band: 2, gain: -5, freq: 700, q: 14, shelf: true }).unwrap();
        assert!(s.send(FxCmd::SetMasterEqBand { band: 8, gain: 1, freq: 700, q: 14, shelf: false }).is_err(), "no band 8");

        let check = |s: &Session| {
            let m = s.state().effects.master.clone();
            assert!(m.compressor.on && m.compressor.edited);
            assert_eq!((m.compressor.preset, m.compressor.compression, m.compressor.output), (CompPreset::Punchy, 70, 12));
            assert!(m.eq.on && m.eq.edited);
            assert_eq!(m.eq.preset, EqPreset::Bright);
            assert_eq!(m.eq.bands[2], EqBand { gain: -5, freq: 700, q: 14, shelf: false }, "a middle band is a peak");
            let ctl = s.inner.lock();
            let fx = &ctl.synth.as_ref().unwrap().control.fx.master;
            assert!(fx.comp_on.load(Relaxed));
            assert_eq!((fx.compression.load(Relaxed), fx.output.load(Relaxed)), (70, 12));
            let (on, bands) = fx.eq();
            assert!(on);
            assert_eq!(bands[2].gain, -5);
            assert_eq!(bands[7].gain, 4, "Bright's top shelf");
        };
        check(&s);
        drop(s);
        check(&session(&data));

        // Off again: saved off.
        let s = session(&data);
        s.send(FxCmd::SetMasterCompressorOn { on: false }).unwrap();
        s.send(FxCmd::SetMasterEqOn { on: false }).unwrap();
        drop(s);
        let s = session(&data);
        let m = s.state().effects.master.clone();
        assert!(!m.compressor.on && !m.eq.on);
        assert!(!s.inner.lock().synth.as_ref().unwrap().control.fx.master.comp_on.load(Relaxed));
        let _ = std::fs::remove_dir_all(&data);
    }

    /// A data folder from before the master effects, or a damaged or partial file, starts
    /// with them off (and the file's own settings where it has them).
    #[test]
    fn old_or_partial_files_load() {
        let data = data_dir("old");
        assert!(!session(&data).state().effects.master.compressor.on, "no file");
        std::fs::write(data.join("master-effects.json"), "not json").unwrap();
        assert!(!session(&data).state().effects.master.eq.on, "unreadable");
        std::fs::write(data.join("master-effects.json"), r#"{"eq":{"on":true,"preset":"mellow"}}"#).unwrap();
        let m = session(&data).state().effects.master.clone();
        assert!(m.eq.on && !m.compressor.on);
        assert_eq!(m.eq.preset, EqPreset::Mellow);
        assert!(m.eq.edited, "the bands it lacks are flat, not Mellow's");
        let _ = std::fs::remove_dir_all(&data);
    }
}
