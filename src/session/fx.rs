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
}
