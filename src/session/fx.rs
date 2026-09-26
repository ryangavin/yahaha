//! The shared effect bus (#204, `crate::fx`): the control side's part. Each block's type
//! and return level (`FxCmd`, `AppState::effects`, the Registration's `effects` section),
//! and the style tempo the Variation block's delay follows (from the engine's snapshot).
//! The audio thread reads them from `SynthControl::fx`, which `pump_fx` keeps up to date.

use super::Control;
use crate::api::{CmdError, EffectBlockState, EffectsState, FxBlock, FxCmd, FxOption, FxType};
use crate::registration::{Group, Groups};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering::Relaxed;

/// Each block's type and return level (indexed by `FxBlock::index`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct FxSettings {
    pub(super) effect: [FxType; 3],
    pub(super) returns: [u8; 3],
}

impl Default for FxSettings {
    /// The Genos defaults: Hall, Chorus, and here the dotted 1/8 delay; every return 0 dB.
    fn default() -> FxSettings {
        FxSettings { effect: [FxType::Hall, FxType::Chorus, FxType::DottedEighth], returns: [crate::fx::RETURN_UNITY; 3] }
    }
}

impl FxSettings {
    /// The type's number within its block (`crate::fx::ReverbType` etc.).
    fn type_index(&self, b: FxBlock) -> u8 {
        b.types().iter().position(|t| *t == self.effect[b.index()]).unwrap_or(0) as u8
    }
}

/// A block in a registration.
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EffectReg {
    effect: FxType,
    return_level: u8,
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
                self.fx.effect[block.index()] = effect;
            }
            FxCmd::SetEffectReturn { block, level } => self.fx.returns[block.index()] = level.min(127),
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
    }

    /// The Registration's `effects` section (group Style, as the Genos Data List files
    /// the Reverb/Chorus/Variation type and return level).
    pub(super) fn effects_capture(&self, g: Groups) -> Option<serde_json::Value> {
        if !g.has(Group::Style) {
            return None;
        }
        let blocks = FxBlock::ALL.map(|b| EffectReg { effect: self.fx.effect[b.index()], return_level: self.fx.returns[b.index()] });
        serde_json::to_value(EffectsReg { reverb: blocks[0], chorus: blocks[1], variation: blocks[2] }).ok()
    }

    pub(super) fn effects_recall(&mut self, v: &serde_json::Value, g: Groups) -> Result<(), String> {
        if !g.has(Group::Style) {
            return Ok(());
        }
        let r: EffectsReg = serde_json::from_value(v.clone()).map_err(|e| format!("registration effects: {e}"))?;
        for (b, reg) in FxBlock::ALL.into_iter().zip([r.reverb, r.chorus, r.variation]) {
            if b.types().contains(&reg.effect) {
                self.fx.effect[b.index()] = reg.effect;
            }
            self.fx.returns[b.index()] = reg.return_level.min(127);
        }
        self.pump_fx();
        Ok(())
    }

    pub(super) fn effects_state(&self) -> EffectsState {
        let blocks = FxBlock::ALL
            .iter()
            .map(|&b| {
                let effect = self.fx.effect[b.index()];
                EffectBlockState {
                    block: b,
                    name: b.name().into(),
                    effect,
                    effect_name: effect.name().into(),
                    types: b.types().iter().map(|&t| FxOption { effect: t, name: t.name().into() }).collect(),
                    return_level: self.fx.returns[b.index()],
                }
            })
            .collect();
        EffectsState { blocks }
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
}
