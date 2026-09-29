//! The shared effect bus (#204, `crate::fx`): the control side's part. Each block's type,
//! return level and band send (#236) (`FxCmd`, `AppState::effects`),
//! and the style tempo the Variation block's delay follows (from the engine's snapshot).
//! The audio thread reads them from `SynthControl::fx`, which `pump_fx` keeps up to date.

use super::Control;
use crate::api::{CmdError, EffectsState, FxBlock, FxCmd, FxType, InsertEffect, InsertState, MasterSettingsExt, StyleEffectState};
use crate::fx::xg::StyleFx;
use std::sync::atomic::Ordering::Relaxed;

/// Each block's type and return level (indexed by `FxBlock::index`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct FxSettings {
    pub(super) effect: [FxType; 3],
    pub(super) returns: [u8; 3],
    /// The band send scales (#236), 0-127 %.
    pub(super) band: [u8; 3],
    /// The Multi Pad send scales (#267), 0-127 %.
    pub(super) pad: [u8; 3],
    /// The effect parameters (#236, `crate::fx::Param::index`).
    pub(super) params: [u16; crate::fx::PARAMS],
    /// Each block follows the style's own type (#237).
    pub(super) follow: [bool; 3],
    /// The style's insertion effects play (#269).
    pub(super) inserts_on: bool,
    /// Each Style part's insert switched off by the player, and its amount set by the
    /// player (None: the style's); both until the next style (`clear_part_inserts`).
    pub(super) insert_off: [bool; 8],
    pub(super) insert_amount: [Option<u8>; 8],
    /// The rotary inserts fast.
    pub(super) rotary_fast: bool,
}

impl Default for FxSettings {
    /// The Genos defaults: Hall, Chorus, and here the dotted 1/8 delay; every return 0 dB.
    /// The band's reverb as the style wrote it, and no band chorus or delay (#236).
    fn default() -> FxSettings {
        FxSettings { effect: FxBlock::DEFAULT_TYPES, returns: [crate::fx::RETURN_UNITY; 3], band: crate::fx::BAND_SEND_DEFAULT, pad: crate::fx::PAD_SEND_DEFAULT, params: crate::fx::default_params(), follow: [true; 3], inserts_on: true, insert_off: [false; 8], insert_amount: [None; 8], rotary_fast: false }
    }
}

impl FxSettings {
    /// A new style: its inserts as it wrote them.
    pub(super) fn clear_part_inserts(&mut self) {
        self.insert_off = [false; 8];
        self.insert_amount = [None; 8];
    }

    /// The type's number within its block (`crate::fx::ReverbType` etc.).
    fn type_index(&self, b: FxBlock) -> u8 {
        b.type_index(self.effect[b.index()])
    }

    /// Block `b` takes type `t`, and its parameters that type's own values.
    fn set_type(&mut self, b: FxBlock, t: FxType) {
        self.effect[b.index()] = t;
        crate::fx::type_defaults(b.index(), b.type_index(t), &mut self.params);
    }

    /// The defaults, with the style's own effect types (#237).
    pub(super) fn for_style(style: &StyleFx) -> FxSettings {
        let mut s = FxSettings::default();
        s.apply_style(style, None);
        s
    }

    /// Each block that follows the style (or only `only`) takes the style's type and the
    /// parameters it sets; a block the style sets nothing near takes its default type.
    /// The type's own parameters come back either way, as a Genos style load does, and
    /// its return level is the style's, or 0 dB (64) where the style sets none (#269).
    pub(super) fn apply_style(&mut self, style: &StyleFx, only: Option<FxBlock>) {
        for b in FxBlock::ALL {
            if !self.follow[b.index()] || only.is_some_and(|o| o != b) {
                continue;
            }
            let choice = style.blocks[b.index()].as_ref();
            match choice.and_then(|c| c.kind.map(|k| (b.types()[k as usize], &c.params))) {
                Some((t, params)) => {
                    self.set_type(b, t);
                    for &(p, v) in params {
                        self.params[p.index()] = p.clamp(v);
                    }
                    // The style's return level (#269), where it sets one; else 0 dB, so
                    // the last style's return doesn't carry into this one.
                    self.returns[b.index()] = choice.and_then(|c| c.ret).map_or(crate::fx::RETURN_UNITY, |r| r.min(127));
                }
                None => {
                    self.set_type(b, FxBlock::DEFAULT_TYPES[b.index()]);
                    self.returns[b.index()] = crate::fx::RETURN_UNITY;
                }
            }
        }
    }
}

impl Control {
    pub(super) fn fx_cmd(&mut self, c: FxCmd) -> Result<(), CmdError> {
        // The Master Compressor and Master EQ (session/master_fx.rs).
        if let Some(r) = self.master_fx_cmd(&c) {
            return r;
        }
        match c {
            FxCmd::SetEffectType { block, effect } => {
                if !block.types().contains(&effect) {
                    return self.fail(format!("{} has no {} type", block.name(), effect.name()));
                }
                if self.fx.effect[block.index()] != effect {
                    self.fx.set_type(block, effect);
                }
                // The player's own choice: style changes leave it (#237).
                self.fx.follow[block.index()] = false;
            }
            FxCmd::SetFollowStyle { block, on } => {
                self.fx.follow[block.index()] = on;
                if on {
                    self.fx.apply_style(&self.info.effects, Some(block));
                }
            }
            FxCmd::SetInsertsOn { on } => self.fx.inserts_on = on,
            FxCmd::SetPartInsertOn { part, on } => {
                if part > 7 {
                    return self.fail(format!("no Style part {part}"));
                }
                self.fx.insert_off[part as usize] = !on;
            }
            FxCmd::SetPartInsertAmount { part, amount } => {
                if part > 7 || !self.info.inserts.iter().any(|i| i.channel == part + 8 && i.kind.is_some()) {
                    return self.fail(format!("Style part {part} has no insertion effect"));
                }
                self.fx.insert_amount[part as usize] = Some(amount.min(127));
            }
            FxCmd::SetRotaryFast { on } => self.fx.rotary_fast = on,
            FxCmd::ToggleRotaryFast => self.fx.rotary_fast = !self.fx.rotary_fast,
            // Taken above.
            FxCmd::SetMasterCompressorOn { .. }
            | FxCmd::SetMasterCompressorPreset { .. }
            | FxCmd::SetMasterCompressorParam { .. }
            | FxCmd::SetMasterEqOn { .. }
            | FxCmd::SetMasterEqPreset { .. }
            | FxCmd::SetMasterEqBand { .. } => {}
            FxCmd::SetEffectReturn { block, level } => self.fx.returns[block.index()] = level.min(127),
            FxCmd::SetBandSend { block, level } => self.fx.band[block.index()] = level.min(127),
            FxCmd::SetPadSend { block, level } => self.fx.pad[block.index()] = level.min(127),
            FxCmd::SetEffectParam { block, param, value } => {
                if param.spec().block != block.index() {
                    return self.fail(format!("{} has no {} parameter", block.name(), param.spec().name));
                }
                self.fx.params[param.index()] = param.clamp(value);
                // The player's own setting (the editor's or a knob's): the block no longer
                // follows the style (#237), so the next style change keeps it, as after a
                // type change.
                self.fx.follow[block.index()] = false;
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
        for (a, &v) in fx.pad_send.iter().zip(&s.pad) {
            a.store(v, Relaxed);
        }
        for (a, &v) in fx.params.iter().zip(&s.params) {
            a.store(v, Relaxed);
        }
        // The style's insertion effects (#269), on the Style parts they are on.
        let mut kinds = [(0u8, 64u8); 8];
        if s.inserts_on {
            for i in &self.info.inserts {
                if let (Some((k, a)), Some(p)) = (i.kind, (i.channel as usize).checked_sub(8).filter(|&p| p < 8)) {
                    if !s.insert_off[p] {
                        kinds[p] = (k as u8, s.insert_amount[p].unwrap_or(a));
                    }
                }
            }
        }
        for (p, (k, a)) in kinds.into_iter().enumerate() {
            fx.insert[p].store(k, Relaxed);
            fx.insert_amount[p].store(a, Relaxed);
        }
        fx.rotary_fast.store(s.rotary_fast, Relaxed);
        self.pump_master_fx();
        // The Style parts' own sends (#268): the engine owns them.
        for (a, own) in fx.part_send.iter().zip(&self.snap.style_send_own) {
            for (a, &v) in a.iter().zip(own) {
                a.store(v, Relaxed);
            }
        }
    }

    pub(super) fn effects_state(&self) -> EffectsState {
        let mut s = EffectsState::new(self.fx.effect, self.fx.returns, self.fx.band, self.fx.params);
        s.inserts_on = self.fx.inserts_on;
        s.rotary_fast = self.fx.rotary_fast;
        s.master = self.master.settings.state();
        s.inserts = self
            .info
            .inserts
            .iter()
            .map(|i| InsertState {
                part: i.channel - 8,
                part_name: crate::api::STYLE_PART_NAMES[(i.channel - 8) as usize & 7].to_string(),
                name: i.name.clone(),
                effect: i.kind.map(|k| InsertEffect::from(k.0)),
                on: !self.fx.insert_off[(i.channel - 8) as usize & 7],
                amount: self.fx.insert_amount[(i.channel - 8) as usize & 7].or(i.kind.map(|k| k.1)).unwrap_or(64),
            })
            .collect();
        for (b, st) in s.blocks.iter_mut().enumerate() {
            st.pad_send = self.fx.pad[b];
            st.follow_style = self.fx.follow[b];
            st.style_effect = self.info.effects.blocks[b].as_ref().map(|c| StyleEffectState {
                name: c.name.clone(),
                effect: c.kind.map(|k| FxBlock::ALL[b].types()[k as usize]),
            });
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use crate::api::TransportCmd;
    use crate::session::{Options, Session};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::Ordering::Relaxed;

    /// SlowWalker from the corpus, for the tests that check its own effects (Real Large
    /// Plate, its chorus and delay); None without the corpus, so they skip in CI.
    fn slow_walker() -> Option<PathBuf> {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            eprintln!("needs the corpus (SlowWalker's own effects); skipping");
            return None;
        }
        Some(p)
    }

    /// The delay follows the tempo: the style's own, then one the player sets.
    #[test]
    fn the_effect_bus_follows_the_style_tempo() {
        let p = crate::session::testing::style_path();
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
        let p = crate::session::testing::style_path();
        let s = Session::offline(Options { paths: vec![p], ..Options::default() }).unwrap();
        s.send(PartsCmd::SetPartSend { part: 0, send: PartSend::Reverb, value: 70 }).unwrap();
        s.take_output();
        let resent = |out: &[[u8; 3]]| out.contains(&[0xB0, 91, 70]) && out.contains(&[0xB1, 91, 0]) && out.contains(&[0xB2, 93, 0]);
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
    /// and refused when the type is another block's.
    #[test]
    fn effect_types_and_returns_reach_the_bus() {
        use crate::api::{FxBlock, FxCmd, FxType};
        let Some(p) = slow_walker() else { return };
        let s = Session::offline(Options { paths: vec![p], ..Options::default() }).unwrap();
        s.offline_audio(None, 48_000).unwrap();
        let blocks = |s: &Session| s.state().effects.blocks.iter().map(|b| (b.effect, b.return_level)).collect::<Vec<_>>();
        assert_eq!(blocks(&s), vec![(FxType::Plate, 64), (FxType::Chorus, 64), (FxType::DottedEighth, 64)], "the style's Real Large Plate (#237)");
        assert_eq!(s.state().effects.blocks[0].types.len(), 4);

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
    }

    /// #236: the band send scales, in the state and on the audio thread's atomics.
    #[test]
    fn band_sends_reach_the_bus() {
        use crate::api::{FxBlock, FxCmd};
        let p = crate::session::testing::style_path();
        let s = Session::offline(Options { paths: vec![p], ..Options::default() }).unwrap();
        s.offline_audio(None, 48_000).unwrap();
        let band = |s: &Session| s.state().effects.blocks.iter().map(|b| b.band_send).collect::<Vec<_>>();
        let atomics = |s: &Session| {
            let ctl = s.inner.lock();
            ctl.synth.as_ref().unwrap().control.fx.band_send.iter().map(|a| a.load(Relaxed)).collect::<Vec<_>>()
        };
        assert_eq!(band(&s), vec![100, 0, 0], "reverb as written, no band chorus or delay");
        assert_eq!(atomics(&s), vec![100, 0, 0]);
        s.send(FxCmd::SetBandSend { block: FxBlock::Variation, level: 60 }).unwrap();
        s.send(FxCmd::SetBandSend { block: FxBlock::Reverb, level: 200 }).unwrap();
        assert_eq!(band(&s), vec![127, 0, 60]);
        assert_eq!(atomics(&s), vec![127, 0, 60]);
    }

    /// #267: the Multi Pad send scales, in the state and on the audio thread's atomics.
    #[test]
    fn pad_sends_reach_the_bus() {
        use crate::api::{FxBlock, FxCmd};
        let p = crate::session::testing::style_path();
        let s = Session::offline(Options { paths: vec![p], ..Options::default() }).unwrap();
        s.offline_audio(None, 48_000).unwrap();
        let pad = |s: &Session| s.state().effects.blocks.iter().map(|b| b.pad_send).collect::<Vec<_>>();
        let atomics = |s: &Session| {
            let ctl = s.inner.lock();
            ctl.synth.as_ref().unwrap().control.fx.pad_send.iter().map(|a| a.load(Relaxed)).collect::<Vec<_>>()
        };
        assert_eq!(pad(&s), vec![100, 0, 0], "the pads' reverb as written, no chorus or delay");
        assert_eq!(atomics(&s), vec![100, 0, 0]);
        s.send(FxCmd::SetPadSend { block: FxBlock::Chorus, level: 80 }).unwrap();
        s.send(FxCmd::SetPadSend { block: FxBlock::Reverb, level: 200 }).unwrap();
        assert_eq!(pad(&s), vec![127, 80, 0]);
        assert_eq!(atomics(&s), vec![127, 80, 0]);
        assert_eq!(s.state().effects.blocks.iter().map(|b| b.band_send).collect::<Vec<_>>(), vec![100, 0, 0], "the band's are its own");
    }

    /// #236: the effect parameters: set in their range, refused on another block, back
    /// to the type's own values on a type change, and on the audio thread's atomics.
    #[test]
    fn effect_parameters_reach_the_bus() {
        use crate::api::{FxBlock, FxCmd, FxParam, FxType};
        let Some(p) = slow_walker() else { return };
        let s = Session::offline(Options { paths: vec![p], ..Options::default() }).unwrap();
        s.offline_audio(None, 48_000).unwrap();
        let reverb = |s: &Session| s.state().effects.blocks[0].params.iter().map(|p| (p.value, p.display.clone())).collect::<Vec<_>>();
        let atomic = |s: &Session, p: FxParam| s.inner.lock().synth.as_ref().unwrap().control.fx.params[p.index()].load(Relaxed);
        // The style's own reverb (#237): a plate.
        let hall = vec![(18, "1.8 s".to_string()), (1, "1 ms".to_string()), (90, "9.0 kHz".to_string())];
        assert_eq!(reverb(&s), hall);
        let st = s.state();
        let time = &st.effects.blocks[0].params[0];
        assert_eq!((time.param, time.min, time.max, time.default, time.name.as_str()), (FxParam::ReverbTime, 3, 100, 18, "Time"));

        s.send(FxCmd::SetEffectParam { block: FxBlock::Reverb, param: FxParam::ReverbTime, value: 500 }).unwrap();
        s.send(FxCmd::SetEffectParam { block: FxBlock::Reverb, param: FxParam::PreDelay, value: 120 }).unwrap();
        assert!(s.send(FxCmd::SetEffectParam { block: FxBlock::Chorus, param: FxParam::ReverbTime, value: 10 }).is_err(), "not the chorus's");
        assert_eq!(reverb(&s)[..2], [(100, "10.0 s".to_string()), (120, "120 ms".to_string())]);
        assert_eq!((atomic(&s, FxParam::ReverbTime), atomic(&s, FxParam::PreDelay)), (100, 120));

        // A type change: the Room's own values.
        s.send(FxCmd::SetEffectType { block: FxBlock::Reverb, effect: FxType::Room }).unwrap();
        assert_eq!(reverb(&s).iter().map(|p| p.0).collect::<Vec<_>>(), vec![9, 4, 60]);
        assert_eq!(s.state().effects.blocks[0].params[0].default, 9);
        // The same type again changes nothing.
        s.send(FxCmd::SetEffectParam { block: FxBlock::Reverb, param: FxParam::ReverbTone, value: 30 }).unwrap();
        s.send(FxCmd::SetEffectType { block: FxBlock::Reverb, effect: FxType::Room }).unwrap();
        assert_eq!(reverb(&s)[2].0, 30);
    }

    /// #236: the delay's parameters: a type sets the note value and the ping-pong switch,
    /// and they reach the audio thread.
    #[test]
    fn delay_parameters_follow_the_type_and_reach_the_bus() {
        use crate::api::{FxBlock, FxCmd, FxParam, FxType};
        let p = crate::session::testing::style_path();
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

    /// #236: the chorus's rate and depth start at the type's own and reach the bus; the
    /// FX knob page turns them.
    #[test]
    fn chorus_parameters_and_the_fx_knobs() {
        use crate::api::{FxBlock, FxCmd, FxParam, FxType, KnobsCmd};
        use crate::knobs::KnobPage;
        let Some(p) = slow_walker() else { return };
        let s = Session::offline(Options { paths: vec![p], ..Options::default() }).unwrap();
        s.offline_audio(None, 48_000).unwrap();
        let chorus = |s: &Session| s.state().effects.blocks[1].params.iter().map(|p| p.display.clone()).collect::<Vec<_>>();
        assert_eq!(chorus(&s), ["0.55 Hz", "2.2 ms"]);
        s.send(FxCmd::SetEffectType { block: FxBlock::Chorus, effect: FxType::Celeste }).unwrap();
        assert_eq!(chorus(&s), ["0.29 Hz", "0.9 ms"]);
        s.send(KnobsCmd::SetKnobPage { page: KnobPage::Chorus }).unwrap();
        s.send(KnobsCmd::TurnKnob { knob: 5, delta: 3 }).unwrap();
        s.send(KnobsCmd::SetKnobPage { page: KnobPage::Reverb }).unwrap();
        s.send(KnobsCmd::TurnKnob { knob: 4, delta: 6 }).unwrap();
        s.send(KnobsCmd::SetKnobPage { page: KnobPage::Delay }).unwrap();
        s.send(KnobsCmd::TurnKnob { knob: 5, delta: 10 }).unwrap();
        let st = s.state();
        assert_eq!(chorus(&s)[1], "1.2 ms");
        assert_eq!(st.effects.blocks[0].params[0].display, "2.4 s", "the reverb time (the style's plate, 1.8 s), 0.1 s a step");
        assert_eq!(st.effects.blocks[2].params[3].display, "58%", "the delay feedback, 2% a step");
        assert_eq!((st.knobs.page_name.as_str(), st.knobs.knobs[5].short.as_str(), st.knobs.knobs[5].value.as_str()), ("Delay", "DlyFdbk", "58%"));
        // A parameter knob pins its block to the player's own (#237), as the editor does.
        assert!(st.effects.blocks.iter().all(|b| !b.follow_style), "every block turned is Mine");
        let ctl = s.inner.lock();
        assert_eq!(ctl.synth.as_ref().unwrap().control.fx.params[FxParam::ChorusDepth.index()].load(Relaxed), 12);
    }

    /// #269: a style's insertion effects reach the audio thread on the parts they are on,
    /// show in the state, go off together, and another style brings its own.
    #[test]
    #[cfg(feature = "slow-tests")]
    fn the_styles_inserts_reach_the_bus() {
        use crate::api::{FxCmd, InsertEffect, LibraryCmd};
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
        let mut files: Vec<_> = std::fs::read_dir(dir.join("T5Style")).into_iter().flatten().flatten().map(|e| e.path()).collect();
        files.sort();
        let inserts = |p: &Path| {
            crate::sff::Style::load(p).map(|s| {
                let prep = crate::engine::Prepared::new(&s);
                crate::fx::xg::style_inserts(prep.setups[0].init.iter())
            })
        };
        // A style with an insert yahaha plays, and one with none.
        let with = files.iter().find(|p| inserts(p).is_ok_and(|i| i.iter().any(|i| i.kind.is_some())));
        let slow = files.iter().find(|p| inserts(p).is_ok_and(|i| i.is_empty()));
        let (Some(with), Some(slow)) = (with.cloned(), slow.cloned()) else {
            eprintln!("corpus missing; skipping");
            return;
        };
        let data = std::env::temp_dir().join(format!("yahaha-fx-inserts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&data);
        let s = Session::offline(Options { paths: vec![with.clone(), slow.clone()], data_dir: Some(data.clone()), ..Options::default() }).unwrap();
        s.offline_audio(None, 48_000).unwrap();
        let load = |s: &Session, p: &Path| {
            s.send(LibraryCmd::LoadStylePath { path: p.to_string_lossy().into() }).unwrap();
            s.advance(1_000_000_000);
        };
        let atomics = |s: &Session| {
            let ctl = s.inner.lock();
            ctl.synth.as_ref().unwrap().control.fx.insert.iter().map(|a| a.load(Relaxed)).collect::<Vec<_>>()
        };
        load(&s, &with);
        let st = s.state();
        assert!(st.effects.inserts_on);
        let played: Vec<_> = st.effects.inserts.iter().filter(|i| i.effect.is_some()).collect();
        assert!(!played.is_empty(), "{:?}", st.effects.inserts);
        let a = atomics(&s);
        for i in &st.effects.inserts {
            assert_eq!(a[i.part as usize] != 0, i.effect.is_some(), "{i:?} on the atomics {a:?}");
        }
        let first = played[0].clone();
        let kind = crate::fx::InsertKind::from_u8(a[first.part as usize]);
        assert_eq!(InsertEffect::from(kind), first.effect.unwrap());
        // One part off, then its amount, then the Leslie switch (software parity).
        let p = first.part as usize;
        s.send(FxCmd::SetPartInsertOn { part: first.part, on: false }).unwrap();
        assert_eq!(atomics(&s)[p], 0);
        assert!(!s.state().effects.inserts.iter().find(|i| i.part == first.part).unwrap().on);
        s.send(FxCmd::SetPartInsertOn { part: first.part, on: true }).unwrap();
        assert_eq!(atomics(&s), a);
        s.send(FxCmd::SetPartInsertAmount { part: first.part, amount: 120 }).unwrap();
        assert_eq!(s.inner.lock().synth.as_ref().unwrap().control.fx.insert_amount[p].load(Relaxed), 120);
        assert_eq!(s.state().effects.inserts.iter().find(|i| i.part == first.part).unwrap().amount, 120);
        let bare = (0..8u8).find(|q| !played.iter().any(|i| i.part == *q)).unwrap();
        assert!(s.send(FxCmd::SetPartInsertAmount { part: bare, amount: 10 }).is_err());
        s.send(FxCmd::SetRotaryFast { on: true }).unwrap();
        assert!(s.state().effects.rotary_fast);
        assert!(s.inner.lock().synth.as_ref().unwrap().control.fx.rotary_fast.load(Relaxed));
        s.send(FxCmd::SetRotaryFast { on: false }).unwrap();
        s.send(FxCmd::ToggleRotaryFast).unwrap();
        assert!(s.state().effects.rotary_fast, "toggled to fast");
        assert!(s.inner.lock().synth.as_ref().unwrap().control.fx.rotary_fast.load(Relaxed));
        s.send(FxCmd::ToggleRotaryFast).unwrap();
        assert!(!s.state().effects.rotary_fast, "and back to slow");
        // Off: every part dry.
        s.send(FxCmd::SetInsertsOn { on: false }).unwrap();
        assert!(atomics(&s).iter().all(|&k| k == 0));
        assert!(!s.state().effects.inserts_on);
        // On again.
        s.send(FxCmd::SetInsertsOn { on: true }).unwrap();
        assert!(s.state().effects.inserts_on);
        assert_eq!(atomics(&s), a);
        // A style with none: every part dry, and the player's amount gone with the style.
        load(&s, &slow);
        assert_eq!(s.inner.lock().fx.insert_amount, [None; 8]);
        assert!(atomics(&s).iter().all(|&k| k == 0));
        assert!(s.state().effects.inserts.iter().all(|i| i.effect.is_none()));
        let _ = std::fs::remove_dir_all(&data);
    }

    /// #269: a style's own reverb time, pre-delay and tone (its XG reverb parameters) come
    /// with its type; a block not following keeps the player's.
    #[test]
    fn the_styles_own_reverb_parameters() {
        use crate::api::{FxBlock, FxCmd, LibraryCmd};
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
        let (icy, cumbia) = (dir.join("SX900Style for Genos/IcyBallad.T559.prs"), dir.join("T5Style/Cumbia.T158.sst"));
        if !icy.exists() || !cumbia.exists() {
            eprintln!("corpus missing; skipping");
            return;
        }
        let s = Session::offline(Options { paths: vec![icy.clone(), cumbia.clone()], ..Options::default() }).unwrap();
        s.offline_audio(None, 48_000).unwrap();
        let load = |s: &Session, p: &Path| {
            s.send(LibraryCmd::LoadStylePath { path: p.to_string_lossy().into() }).unwrap();
            s.advance(1_000_000_000);
        };
        let reverb = |s: &Session| s.state().effects.blocks[0].params.iter().map(|p| p.display.clone()).collect::<Vec<_>>();
        let atomic = |s: &Session, p: crate::fx::Param| s.inner.lock().synth.as_ref().unwrap().control.fx.params[p.index()].load(Relaxed);
        // Real Large Hall +: time 27 (3.0 s), initial delay 59 (93 ms), high damp 54 (10 kHz).
        load(&s, &icy);
        assert_eq!(reverb(&s), ["3.0 s", "93 ms", "10.0 kHz"]);
        assert_eq!(atomic(&s, crate::fx::Param::PreDelay), 93);
        // Real Medium Hall: time 13 (1.6 s), initial delay 7 (11 ms); its own tone.
        load(&s, &cumbia);
        let st = s.state();
        assert_eq!(reverb(&s)[..2], ["1.6 s", "11 ms"]);
        assert_eq!(st.effects.blocks[0].params[2].value, st.effects.blocks[0].params[2].default);
        // A return the player set: a style that sets none puts a following block back at
        // 0 dB (64), so one style's return never carries into the next.
        s.send(FxCmd::SetEffectReturn { block: FxBlock::Chorus, level: 20 }).unwrap();
        s.send(FxCmd::SetEffectReturn { block: FxBlock::Reverb, level: 100 }).unwrap();
        load(&s, &icy);
        let rets = |s: &Session| s.state().effects.blocks.iter().map(|b| b.return_level).collect::<Vec<_>>();
        assert_eq!(rets(&s), vec![64, 64, 64]);
        // Not following: the player's reverb and its return stay.
        s.send(FxCmd::SetFollowStyle { block: FxBlock::Reverb, on: false }).unwrap();
        s.send(FxCmd::SetEffectReturn { block: FxBlock::Reverb, level: 100 }).unwrap();
        load(&s, &cumbia);
        assert_eq!(reverb(&s)[..2], ["3.0 s", "93 ms"], "IcyBallad's, pinned");
        assert_eq!(rets(&s)[0], 100);
    }

    /// #237: a style's own effect types. Loading it sets them (and the delay's time and
    /// feedback from its SysEx); another style sets its own; a type the player picks
    /// stays through style changes until the block follows the style again.
    #[test]
    #[cfg(feature = "slow-tests")]
    fn the_styles_own_effect_types() {
        use crate::api::{FxBlock, FxCmd, FxType, LibraryCmd};
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/T5Style");
        let (disco, dancehall) = (dir.join("DiscoTeens.T161.prs"), dir.join("Dancehall.T152.prs"));
        if !disco.exists() || !dancehall.exists() {
            eprintln!("corpus missing; skipping");
            return;
        }
        let s = Session::offline(Options { paths: vec![disco.clone(), dancehall.clone()], ..Options::default() }).unwrap();
        s.offline_audio(None, 48_000).unwrap();
        let types = |s: &Session| s.state().effects.blocks.iter().map(|b| b.effect).collect::<Vec<_>>();
        let load = |s: &Session, p: &Path| {
            s.send(LibraryCmd::LoadStylePath { path: p.to_string_lossy().into() }).unwrap();
            s.advance(1_000_000_000);
        };
        load(&s, &disco);
        let st = s.state();
        assert_eq!(types(&s), vec![FxType::Plate, FxType::Chorus, FxType::PingPong], "Real Large Plate, Chorus 3, Tempo Cross 2");
        let style = |b: usize| st.effects.blocks[b].style_effect.as_ref().map(|e| (e.name.clone(), e.effect));
        assert_eq!(style(0), Some(("Real Large Plate".into(), Some(FxType::Plate))));
        assert_eq!(style(2), Some(("Tempo Cross 2".into(), Some(FxType::PingPong))));
        let delay: Vec<_> = st.effects.blocks[2].params.iter().map(|p| p.display.clone()).collect();
        assert_eq!(delay, ["On", "1/16", "375 ms", "42%", "5.0 kHz", "On"], "the style's delay time and feedback");
        assert!(st.effects.blocks.iter().all(|b| b.follow_style));
        {
            let ctl = s.inner.lock();
            assert_eq!(ctl.synth.as_ref().unwrap().control.fx.reverb_type.load(Relaxed), crate::fx::ReverbType::Plate as u8);
        }

        // Another style, its own.
        load(&s, &dancehall);
        assert_eq!(types(&s), vec![FxType::Room, FxType::Flanger, FxType::PingPong], "Percussion Room, Flanger 3, Tempo Cross 1");

        // The player's reverb stays through a style change; the other blocks follow.
        s.send(FxCmd::SetEffectType { block: FxBlock::Reverb, effect: FxType::Hall }).unwrap();
        assert!(!s.state().effects.blocks[0].follow_style);
        load(&s, &disco);
        assert_eq!(types(&s), vec![FxType::Hall, FxType::Chorus, FxType::PingPong]);
        // Following the style again takes its type at once.
        s.send(FxCmd::SetFollowStyle { block: FxBlock::Reverb, on: true }).unwrap();
        assert_eq!(types(&s)[0], FxType::Plate);
    }

    /// Organ Rotary Slow/Fast (RM p.140) is an assignable switch: software (a button, a
    /// Toggle pedal) flips it, a Hold A pedal keeps it fast while down.
    #[test]
    fn rotary_fast_is_assignable() {
        use crate::api::ControllersCmd;
        use crate::controllers::Function;
        use crate::session::Port;
        let s = crate::session::testing::session();
        let fast = |s: &Session| s.state().effects.rotary_fast;
        s.send(ControllersCmd::TriggerFunction { function: Function::RotaryFast }).unwrap();
        assert!(fast(&s), "a press: fast");
        s.send(ControllersCmd::TriggerFunction { function: Function::RotaryFast }).unwrap();
        assert!(!fast(&s), "again: slow");
        let pedal = ControllersCmd::SetPedal { pedal: 1, cc: Some(66), function: Function::RotaryFast, control_type: Default::default(), reverse: false, range: Default::default() };
        s.send(pedal).unwrap();
        s.midi_in(Port::Keys, &[0xB0, 66, 127]);
        s.advance(1_000_000);
        assert!(fast(&s), "Hold A: fast while down");
        s.midi_in(Port::Keys, &[0xB0, 66, 0]);
        s.advance(1_000_000);
        assert!(!fast(&s), "and slow when let go");
    }
}
