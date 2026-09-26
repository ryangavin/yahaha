//! The shared effect bus (#204, `crate::fx`): the control side's part. The Variation
//! block's delay follows the style tempo, which the engine's snapshot carries.

use super::Control;

impl Control {
    /// Keep the effect bus at the style's tempo (a store per pump; the audio thread reads
    /// it once per buffer).
    pub(super) fn pump_fx(&mut self) {
        if let Some(synth) = self.synth.as_ref() {
            synth.control.fx.set_tempo(self.snap.bpm);
        }
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
}
