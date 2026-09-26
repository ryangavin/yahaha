//! The shared effect bus (#204, `crate::fx`): the control side's part. Each block's type,
//! return level and band send (#236) (`FxCmd`, `AppState::effects`, the Registration's `effects` section),
//! and the style tempo the Variation block's delay follows (from the engine's snapshot).
//! The audio thread reads them from `SynthControl::fx`, which `pump_fx` keeps up to date.

use super::Control;
use crate::api::{CmdError, EffectsState, FxBlock, FxCmd, FxParam, FxType};
use crate::registration::{Group, Groups};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering::Relaxed;

/// Each block's type and return level (indexed by `FxBlock::index`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct FxSettings {
    pub(super) effect: [FxType; 3],
    pub(super) returns: [u8; 3],
    /// The band send scales (#236), 0-127 %.
    pub(super) band: [u8; 3],
    /// The effect parameters (#236, `crate::fx::Param::index`).
    pub(super) params: [u16; crate::fx::PARAMS],
}

impl Default for FxSettings {
    /// The Genos defaults: Hall, Chorus, and here the dotted 1/8 delay; every return 0 dB.
    /// The band's reverb as the style wrote it, and no band chorus or delay (#236).
    fn default() -> FxSettings {
        FxSettings { effect: FxBlock::DEFAULT_TYPES, returns: [crate::fx::RETURN_UNITY; 3], band: crate::fx::BAND_SEND_DEFAULT, params: crate::fx::default_params() }
    }
}

impl FxSettings {
    /// The type's number within its block (`crate::fx::ReverbType` etc.).
    fn type_index(&self, b: FxBlock) -> u8 {
        b.type_index(self.effect[b.index()])
    }

    /// Block `b` takes type `t`, and its parameters that type's own values.
    fn set_type(&mut self, b: FxBlock, t: FxType) {
        self.effect[b.index()] = t;
        crate::fx::type_defaults(b.index(), b.type_index(t), &mut self.params);
    }
}

/// A block in a registration.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EffectReg {
    effect: FxType,
    return_level: u8,
    /// Absent in a registration from before #236: the block's default.
    #[serde(default)]
    band_send: Option<u8>,
    /// The block's parameters (#236); one absent is the type's own value.
    #[serde(default)]
    params: std::collections::BTreeMap<FxParam, u16>,
}

#[derive(Serialize, Deserialize)]
struct EffectsReg {
    reverb: EffectReg,
    chorus: EffectReg,
    variation: EffectReg,
}

pub(super) fn effects_capture(c: &Control, g: Groups) -> Option<serde_json::Value> {
    c.effects_capture(g)
}

pub(super) fn effects_recall(c: &mut Control, v: &serde_json::Value, g: Groups) -> Result<(), String> {
    c.effects_recall(v, g)
}

impl Control {
    pub(super) fn fx_cmd(&mut self, c: FxCmd) -> Result<(), CmdError> {
        match c {
            FxCmd::SetEffectType { block, effect } => {
                if !block.types().contains(&effect) {
                    return self.fail(format!("{} has no {} type", block.name(), effect.name()));
                }
                if self.fx.effect[block.index()] != effect {
                    self.fx.set_type(block, effect);
                }
            }
            FxCmd::SetEffectReturn { block, level } => self.fx.returns[block.index()] = level.min(127),
            FxCmd::SetBandSend { block, level } => self.fx.band[block.index()] = level.min(127),
            FxCmd::SetEffectParam { block, param, value } => {
                if param.spec().block != block.index() {
                    return self.fail(format!("{} has no {} parameter", block.name(), param.spec().name));
                }
                self.fx.params[param.index()] = param.clamp(value);
            }
        }
        self.pump_fx();
        Ok(())
    }

    /// Keep the effect bus at the blocks' settings and the style's tempo (stores per
    /// pump; the audio thread reads them once per buffer).
    pub(super) fn pump_fx(&mut self) {
        let Some(synth) = self.synth.as_ref() else { return };
        let fx = &synth.control.fx;
        fx.set_tempo(self.snap.bpm);
        let s = &self.fx;
        fx.reverb_type.store(s.type_index(FxBlock::Reverb), Relaxed);
        fx.chorus_type.store(s.type_index(FxBlock::Chorus), Relaxed);
        fx.variation_type.store(s.type_index(FxBlock::Variation), Relaxed);
        fx.reverb_return.store(s.returns[0], Relaxed);
        fx.chorus_return.store(s.returns[1], Relaxed);
        fx.variation_return.store(s.returns[2], Relaxed);
        for (a, &v) in fx.band_send.iter().zip(&s.band) {
            a.store(v, Relaxed);
        }
        for (a, &v) in fx.params.iter().zip(&s.params) {
            a.store(v, Relaxed);
        }
    }

    /// The Registration's `effects` section (group Style, as the Genos Data List files
    /// the Reverb/Chorus/Variation type and return level).
    pub(super) fn effects_capture(&self, g: Groups) -> Option<serde_json::Value> {
        if !g.has(Group::Style) {
            return None;
        }
        let blocks = FxBlock::ALL.map(|b| EffectReg {
            effect: self.fx.effect[b.index()],
            return_level: self.fx.returns[b.index()],
            band_send: Some(self.fx.band[b.index()]),
            params: FxParam::of_block(b.index()).map(|p| (p, self.fx.params[p.index()])).collect(),
        });
        let [reverb, chorus, variation] = blocks;
        serde_json::to_value(EffectsReg { reverb, chorus, variation }).ok()
    }

    pub(super) fn effects_recall(&mut self, v: &serde_json::Value, g: Groups) -> Result<(), String> {
        if !g.has(Group::Style) {
            return Ok(());
        }
        let r: EffectsReg = serde_json::from_value(v.clone()).map_err(|e| format!("registration effects: {e}"))?;
        for (b, reg) in FxBlock::ALL.into_iter().zip([r.reverb, r.chorus, r.variation]) {
            if b.types().contains(&reg.effect) {
                self.fx.set_type(b, reg.effect);
            }
            for (p, v) in reg.params {
                if p.spec().block == b.index() {
                    self.fx.params[p.index()] = p.clamp(v);
                }
            }
            self.fx.returns[b.index()] = reg.return_level.min(127);
            self.fx.band[b.index()] = reg.band_send.unwrap_or(crate::fx::BAND_SEND_DEFAULT[b.index()]).min(127);
        }
        self.pump_fx();
        Ok(())
    }

    pub(super) fn effects_state(&self) -> EffectsState {
        EffectsState::new(self.fx.effect, self.fx.returns, self.fx.band, self.fx.params)
    }
}

#[cfg(test)]
mod tests {
    use crate::api::TransportCmd;
    use crate::session::{Options, Session};
    use std::path::Path;
    use std::sync::atomic::Ordering::Relaxed;

    /// The delay follows the tempo: the style's own, then one the player sets.
    #[test]
    fn the_effect_bus_follows_the_style_tempo() {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return;
        }
        let s = Session::offline(Options { paths: vec![p], ..Options::default() }).unwrap();
        s.offline_audio(None, 48_000).unwrap();
        let tempo = |s: &Session| {
            s.advance(1_000_000);
            let ctl = s.inner.lock();
            (ctl.synth.as_ref().unwrap().control.fx.tempo.load(Relaxed), (ctl.snap.bpm * 100.0).round() as u32)
        };
        let (bus, style) = tempo(&s);
        assert!(style > 0 && bus == style, "the style's tempo: {bus} vs {style}");
        s.send(TransportCmd::SetTempo { bpm: 93 }).unwrap();
        assert_eq!(tempo(&s).0, 9300);
    }

    /// The keyboard parts' sends go out again after anything that may reset a receiver to
    /// its power-on sends: a Panic, a Reset All Controllers from the keyboard (and a new
    /// offline synth, `offline_audio`). A send set since boot goes out as it is now.
    #[test]
    fn the_parts_sends_go_out_again_after_a_reset() {
        use crate::api::{PartSend, PartsCmd, SystemCmd};
        use crate::session::Port;
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return;
        }
        let s = Session::offline(Options { paths: vec![p], ..Options::default() }).unwrap();
        s.send(PartsCmd::SetPartSend { part: 0, send: PartSend::Reverb, value: 70 }).unwrap();
        s.take_output();
        let resent = |out: &[[u8; 3]]| out.contains(&[0xB0, 91, 70]) && out.contains(&[0xB1, 91, 40]) && out.contains(&[0xB2, 93, 10]);
        s.send(SystemCmd::Panic).unwrap();
        let out = s.take_output();
        assert!(resent(&out), "Panic: {out:?}");
        s.midi_in(Port::Keys, &[0xB0, 121, 0]);
        // The input thread sends the reset at once; the engine thread the sends at its wake
        // just after.
        let out = s.take_output();
        assert!(out.contains(&[0xB0, 121, 0]) && resent(&out), "Reset All Controllers, then the sends: {out:?}");
        assert!(!resent(&s.take_output()), "once");
    }

    /// Each block's type and return level: in the state, on the audio thread's atomics,
    /// refused when the type is another block's, and stored in a Registration Memory.
    #[test]
    fn effect_types_and_returns_reach_the_bus_and_the_registration() {
        use crate::api::{FxBlock, FxCmd, FxType, RegistrationCmd};
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return;
        }
        let dir = std::env::temp_dir().join(format!("yahaha-fx-regist-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let s = Session::offline(Options { paths: vec![p], data_dir: Some(dir.clone()), ..Options::default() }).unwrap();
        s.offline_audio(None, 48_000).unwrap();
        let blocks = |s: &Session| s.state().effects.blocks.iter().map(|b| (b.effect, b.return_level)).collect::<Vec<_>>();
        assert_eq!(blocks(&s), vec![(FxType::Hall, 64), (FxType::Chorus, 64), (FxType::DottedEighth, 64)]);
        assert_eq!(s.state().effects.blocks[0].types.len(), 4);
        s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();

        s.send(FxCmd::SetEffectType { block: FxBlock::Reverb, effect: FxType::Plate }).unwrap();
        s.send(FxCmd::SetEffectType { block: FxBlock::Variation, effect: FxType::PingPong }).unwrap();
        s.send(FxCmd::SetEffectReturn { block: FxBlock::Variation, level: 200 }).unwrap();
        assert!(s.send(FxCmd::SetEffectType { block: FxBlock::Reverb, effect: FxType::Flanger }).is_err(), "not a reverb");
        assert_eq!(blocks(&s), vec![(FxType::Plate, 64), (FxType::Chorus, 64), (FxType::PingPong, 127)]);
        {
            let ctl = s.inner.lock();
            let fx = &ctl.synth.as_ref().unwrap().control.fx;
            let got = [fx.reverb_type.load(Relaxed), fx.variation_type.load(Relaxed), fx.variation_return.load(Relaxed)];
            assert_eq!(got, [crate::fx::ReverbType::Plate as u8, crate::fx::DelayType::PingPong as u8, 127]);
        }
        // Registration: the bank keeps what was memorized.
        s.send(RegistrationCmd::MemorizeRegist { index: 1 }).unwrap();
        s.send(RegistrationCmd::RecallRegist { index: 0 }).unwrap();
        s.advance(1_000_000_000);
        assert_eq!(blocks(&s), vec![(FxType::Hall, 64), (FxType::Chorus, 64), (FxType::DottedEighth, 64)]);
        s.send(RegistrationCmd::RecallRegist { index: 1 }).unwrap();
        s.advance(1_000_000_000);
        assert_eq!(blocks(&s), vec![(FxType::Plate, 64), (FxType::Chorus, 64), (FxType::PingPong, 127)]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// #236: the band send scales, in the state, on the audio thread's atomics and in a
    /// Registration Memory; a bank from before them recalls the defaults.
    #[test]
    fn band_sends_reach_the_bus_and_the_registration() {
        use crate::api::{FxBlock, FxCmd, RegistrationCmd};
        use crate::registration::Groups;
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return;
        }
        let dir = std::env::temp_dir().join(format!("yahaha-fx-band-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let s = Session::offline(Options { paths: vec![p], data_dir: Some(dir.clone()), ..Options::default() }).unwrap();
        s.offline_audio(None, 48_000).unwrap();
        let band = |s: &Session| s.state().effects.blocks.iter().map(|b| b.band_send).collect::<Vec<_>>();
        let atomics = |s: &Session| {
            let ctl = s.inner.lock();
            ctl.synth.as_ref().unwrap().control.fx.band_send.iter().map(|a| a.load(Relaxed)).collect::<Vec<_>>()
        };
        assert_eq!(band(&s), vec![100, 0, 0], "reverb as written, no band chorus or delay");
        assert_eq!(atomics(&s), vec![100, 0, 0]);
        s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();
        s.send(FxCmd::SetBandSend { block: FxBlock::Variation, level: 60 }).unwrap();
        s.send(FxCmd::SetBandSend { block: FxBlock::Reverb, level: 200 }).unwrap();
        assert_eq!(band(&s), vec![127, 0, 60]);
        assert_eq!(atomics(&s), vec![127, 0, 60]);
        s.send(RegistrationCmd::MemorizeRegist { index: 1 }).unwrap();
        s.send(RegistrationCmd::RecallRegist { index: 0 }).unwrap();
        s.advance(1_000_000_000);
        assert_eq!(band(&s), vec![100, 0, 0]);
        s.send(RegistrationCmd::RecallRegist { index: 1 }).unwrap();
        s.advance(1_000_000_000);
        assert_eq!(band(&s), vec![127, 0, 60]);
        // A bank memorized before #236 has no band sends: the defaults.
        let old = serde_json::json!({
            "reverb": { "effect": "hall", "returnLevel": 64 },
            "chorus": { "effect": "chorus", "returnLevel": 64 },
            "variation": { "effect": "dottedEighth", "returnLevel": 64 },
        });
        s.inner.lock().effects_recall(&old, Groups::all()).unwrap();
        s.advance(1_000_000_000);
        assert_eq!(band(&s), vec![100, 0, 0]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// #236: the effect parameters: set in their range, refused on another block, back
    /// to the type's own values on a type change, on the audio thread's atomics, and in a
    /// Registration Memory.
    #[test]
    fn effect_parameters_reach_the_bus_and_the_registration() {
        use crate::api::{FxBlock, FxCmd, FxParam, FxType, RegistrationCmd};
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return;
        }
        let dir = std::env::temp_dir().join(format!("yahaha-fx-params-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let s = Session::offline(Options { paths: vec![p], data_dir: Some(dir.clone()), ..Options::default() }).unwrap();
        s.offline_audio(None, 48_000).unwrap();
        let reverb = |s: &Session| s.state().effects.blocks[0].params.iter().map(|p| (p.value, p.display.clone())).collect::<Vec<_>>();
        let atomic = |s: &Session, p: FxParam| s.inner.lock().synth.as_ref().unwrap().control.fx.params[p.index()].load(Relaxed);
        let hall = vec![(24, "2.4 s".to_string()), (22, "22 ms".to_string()), (45, "4.5 kHz".to_string())];
        assert_eq!(reverb(&s), hall);
        let st = s.state();
        let time = &st.effects.blocks[0].params[0];
        assert_eq!((time.param, time.min, time.max, time.default, time.name.as_str()), (FxParam::ReverbTime, 3, 100, 24, "Time"));
        s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();

        s.send(FxCmd::SetEffectParam { block: FxBlock::Reverb, param: FxParam::ReverbTime, value: 500 }).unwrap();
        s.send(FxCmd::SetEffectParam { block: FxBlock::Reverb, param: FxParam::PreDelay, value: 120 }).unwrap();
        assert!(s.send(FxCmd::SetEffectParam { block: FxBlock::Chorus, param: FxParam::ReverbTime, value: 10 }).is_err(), "not the chorus's");
        assert_eq!(reverb(&s)[..2], [(100, "10.0 s".to_string()), (120, "120 ms".to_string())]);
        assert_eq!((atomic(&s, FxParam::ReverbTime), atomic(&s, FxParam::PreDelay)), (100, 120));
        s.send(RegistrationCmd::MemorizeRegist { index: 1 }).unwrap();

        // A type change: the Room's own values.
        s.send(FxCmd::SetEffectType { block: FxBlock::Reverb, effect: FxType::Room }).unwrap();
        assert_eq!(reverb(&s).iter().map(|p| p.0).collect::<Vec<_>>(), vec![9, 4, 60]);
        assert_eq!(s.state().effects.blocks[0].params[0].default, 9);
        // The same type again changes nothing.
        s.send(FxCmd::SetEffectParam { block: FxBlock::Reverb, param: FxParam::ReverbTone, value: 30 }).unwrap();
        s.send(FxCmd::SetEffectType { block: FxBlock::Reverb, effect: FxType::Room }).unwrap();
        assert_eq!(reverb(&s)[2].0, 30);

        s.send(RegistrationCmd::RecallRegist { index: 1 }).unwrap();
        s.advance(1_000_000_000);
        assert_eq!(reverb(&s).iter().map(|p| p.0).collect::<Vec<_>>(), vec![100, 120, 45]);
        s.send(RegistrationCmd::RecallRegist { index: 0 }).unwrap();
        s.advance(1_000_000_000);
        assert_eq!(reverb(&s), hall);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// #236: the delay's parameters: a type sets the note value and the ping-pong switch,
    /// and they reach the audio thread.
    #[test]
    fn delay_parameters_follow_the_type_and_reach_the_bus() {
        use crate::api::{FxBlock, FxCmd, FxParam, FxType};
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return;
        }
        let s = Session::offline(Options { paths: vec![p], ..Options::default() }).unwrap();
        s.offline_audio(None, 48_000).unwrap();
        let delay = |s: &Session| s.state().effects.blocks[2].params.iter().map(|p| p.display.clone()).collect::<Vec<_>>();
        assert_eq!(delay(&s), ["On", "1/8.", "375 ms", "38%", "5.0 kHz", "Off"]);
        s.send(FxCmd::SetEffectType { block: FxBlock::Variation, effect: FxType::PingPong }).unwrap();
        assert_eq!(delay(&s), ["On", "1/8", "375 ms", "38%", "5.0 kHz", "On"]);
        s.send(FxCmd::SetEffectParam { block: FxBlock::Variation, param: FxParam::DelayNote, value: 1 }).unwrap();
        s.send(FxCmd::SetEffectParam { block: FxBlock::Variation, param: FxParam::DelayFeedback, value: 95 }).unwrap();
        s.send(FxCmd::SetEffectParam { block: FxBlock::Variation, param: FxParam::DelaySync, value: 0 }).unwrap();
        s.send(FxCmd::SetEffectParam { block: FxBlock::Variation, param: FxParam::DelayTime, value: 5 }).unwrap();
        assert!(s.send(FxCmd::SetEffectParam { block: FxBlock::Reverb, param: FxParam::PingPong, value: 1 }).is_err());
        assert_eq!(delay(&s), ["Off", "1/8T", "10 ms", "90%", "5.0 kHz", "On"]);
        let ctl = s.inner.lock();
        let fx = &ctl.synth.as_ref().unwrap().control.fx;
        let got = [FxParam::DelaySync, FxParam::DelayNote, FxParam::DelayTime, FxParam::DelayFeedback, FxParam::PingPong].map(|p| fx.params[p.index()].load(Relaxed));
        assert_eq!(got, [0, 1, 10, 90, 1]);
    }
}
