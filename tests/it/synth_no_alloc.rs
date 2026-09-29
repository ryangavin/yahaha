//! The audio callback (`synth::AudioCore::process`) must not allocate or free: SoundFont
//! notes and controllers, a style's XG drum setup (#239), a part's sound controllers, portamento and mono (#246), the keyboard parts' channel-strip EQ and a plugin part's mono and velocity curve (#247), a keyboard part's insert slot (SoundFont and plugin), the effect bus (sends, band send scales, types, parameters, returns, legacy effects), the
//! mixer strips (compressors, insert 2, sends 4-6 and a send's kind changing, on SoundFont
//! and plugin parts), the
//! master fader, a SoundFont swap, and (feature `plugins`) a
//! keyboard part going over to an Audio Unit instrument (Apple's DLSMusicDevice), playing
//! it, crossfading to a second instance, and back to the SoundFont. SoundFont swaps while
//! the control side is not taking old racks back must not free one either. The crate's
//! counting allocator (`alloc_count`) checks every `process` call, on the calling thread
//! only and only inside `process`: plugin load threads and the dispose thread allocate and
//! free on their own time. What the plugin does inside its own render is its own business
//! and does not go through Rust's allocator.

use crate::alloc_count::counted;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use yahaha::fx::master::CompPreset;
use yahaha::fx::part_eq::PartEq;
use yahaha::fx::{InsertEffect, InsertSlot, InsertType, PartComp, PartInsert, SendKind, SendSlot};
use yahaha::parts::Parts;
use yahaha::synth::{self, AudioCore, Rack, SynthControl};

/// The smallest SoundFont in the checkout's soundfonts/, if any.
fn sound_font() -> Option<std::path::PathBuf> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("soundfonts");
    yahaha::library::sound_font_files(&dir).into_iter().map(|f| dir.join(f)).min_by_key(|p| p.metadata().map(|m| m.len()).unwrap_or(u64::MAX))
}

#[test]
fn the_audio_callback_does_not_allocate() {
    let font = sound_font();
    let rack = font.as_ref().map(|f| Rack::load(f, 48_000).unwrap());
    if rack.is_none() {
        eprintln!("no SoundFont: checking the callback without one");
    }
    let (mut feed, rx) = rtrb::RingBuffer::<synth::Msg>::new(256);
    let ctl = Arc::new(SynthControl::new(0));
    let parts = Arc::new(Parts::new());
    let (mut core, mut swap, _link) = AudioCore::new(rack, vec![rx], parts.clone(), ctl.clone(), 48_000, 2);
    let mut out = vec![0f32; 128];
    let mut run = |core: &mut AudioCore, feed: &mut rtrb::Producer<synth::Msg>, msgs: &[[u8; 3]]| -> (usize, usize) {
        for m in msgs {
            feed.push(*m).unwrap();
        }
        counted(|| core.process(&mut out))
    };
    let none = (0, 0);
    assert_eq!(run(&mut core, &mut feed, &[[0xC0, 0, 0], [0xB0, 7, 100], [0x90, 60, 100], [0x9A, 40, 100]]), none, "notes");
    for _ in 0..10 {
        assert_eq!(run(&mut core, &mut feed, &[]), none, "steady");
    }
    // The meters (peak and RMS per channel and the master) are measured in the callback,
    // into atomics: with a SoundFont, the notes on channels 1 and 11 show.
    if font.is_some() {
        let (peaks, master, _) = synth::take_meters(&ctl);
        let (rms, master_rms) = synth::take_rms(&ctl);
        assert!(peaks[0] > 0.0 && rms[0] > 0.0 && rms[0] <= peaks[0], "Right 1: peak {} rms {}", peaks[0], rms[0]);
        assert!(rms[10] > 0.0, "the Bass part's RMS");
        assert!(master[0] > 0.0 && master_rms[0] > 0.0 && master_rms[0] <= master[0]);
    }
    // The effect bus (#204): sends on, every reverb, chorus and delay type, tempo changes, the returns, the
    // SoundFont's own effects instead (legacy) and back, and tails ringing out.
    assert_eq!(run(&mut core, &mut feed, &[[0xB0, 91, 100], [0xB0, 93, 80], [0xBA, 91, 127], [0xBA, 94, 60], [0x90, 64, 100]]), none, "sends");
    for t in 0..4u8 {
        ctl.fx.reverb_type.store(t, Ordering::Relaxed);
        ctl.fx.chorus_type.store(t % 3, Ordering::Relaxed);
        ctl.fx.reverb_return.store(40 + t * 20, Ordering::Relaxed);
        ctl.fx.variation_type.store(t, Ordering::Relaxed);
        ctl.fx.set_tempo(90.0 + t as f64 * 20.0);
        assert_eq!(run(&mut core, &mut feed, &[]), none, "effect types and returns");
    }
    // The effect parameters (#236): the reverb's time, pre-delay and tone, gliding.
    for (time, pre, tone) in [(80u16, 150u16, 20u16), (5, 0, 180), (24, 22, 45)] {
        ctl.fx.params[yahaha::fx::Param::ReverbTime.index()].store(time, Ordering::Relaxed);
        ctl.fx.params[yahaha::fx::Param::PreDelay.index()].store(pre, Ordering::Relaxed);
        ctl.fx.params[yahaha::fx::Param::ReverbTone.index()].store(tone, Ordering::Relaxed);
        for _ in 0..3 {
            assert_eq!(run(&mut core, &mut feed, &[]), none, "effect parameters");
        }
    }
    // The delay's parameters (#236): note values, free time, feedback, tone, ping-pong.
    use yahaha::fx::Param as P;
    for (sync, note, ms, fb, tone, pp) in [(1u16, 7u16, 375u16, 90u16, 20u16, 1u16), (0, 0, 60, 0, 200, 0), (1, 4, 375, 38, 50, 0)] {
        for (p, v) in [(P::DelaySync, sync), (P::DelayNote, note), (P::DelayTime, ms), (P::DelayFeedback, fb), (P::DelayTone, tone), (P::PingPong, pp)] {
            ctl.fx.params[p.index()].store(v, Ordering::Relaxed);
        }
        for _ in 0..3 {
            assert_eq!(run(&mut core, &mut feed, &[]), none, "delay parameters");
        }
    }
    // The chorus's rate and depth (#236), and every chorus type with them.
    for (t, rate, depth) in [(0u8, 400u16, 50u16), (2, 5, 0), (1, 29, 9), (0, 55, 22)] {
        ctl.fx.chorus_type.store(t, Ordering::Relaxed);
        ctl.fx.params[yahaha::fx::Param::ChorusRate.index()].store(rate, Ordering::Relaxed);
        ctl.fx.params[yahaha::fx::Param::ChorusDepth.index()].store(depth, Ordering::Relaxed);
        assert_eq!(run(&mut core, &mut feed, &[[0xB0, 93, 127]]), none, "chorus parameters");
    }
    // The band send scales (#236) gliding up and back.
    for (b, level) in [(1, 100u8), (2, 127), (0, 50), (1, 0), (2, 0), (0, 100)] {
        ctl.fx.band_send[b].store(level, Ordering::Relaxed);
        for _ in 0..3 {
            assert_eq!(run(&mut core, &mut feed, &[]), none, "band send scales");
        }
    }
    // The Style parts' own sends (#268) set, changed and handed back to the style.
    for (p, b, v) in [(2usize, 0usize, 100u8), (2, 2, 127), (7, 1, 0), (2, 0, 255), (2, 2, 255), (7, 1, 255)] {
        ctl.fx.part_send[p][b].store(v, Ordering::Relaxed);
        assert_eq!(run(&mut core, &mut feed, &[[0xBA, 91, 60], [0x9A, 50, 90]]), none, "own sends");
    }
    assert_eq!(run(&mut core, &mut feed, &[[0x8A, 50, 0]]), none, "own sends, note off");
    // The Multi Pad send scales (#267) gliding up and back, a pad sending to every block.
    assert_eq!(run(&mut core, &mut feed, &[[0xB5, 91, 100], [0xB5, 93, 100], [0xB5, 94, 100], [0x95, 64, 100]]), none, "a pad's sends");
    for (b, level) in [(1, 100u8), (2, 127), (0, 50), (1, 0), (2, 0), (0, 100)] {
        ctl.fx.pad_send[b].store(level, Ordering::Relaxed);
        for _ in 0..3 {
            assert_eq!(run(&mut core, &mut feed, &[]), none, "pad send scales");
        }
    }
    assert_eq!(run(&mut core, &mut feed, &[[0x85, 64, 0]]), none, "a pad's note off");
    // The Style parts' insertion effects (#269): every kind on two parts playing, a change
    // (it fades), and off again.
    assert_eq!(run(&mut core, &mut feed, &[[0x9B, 60, 100], [0x9C, 64, 100]]), none, "notes for the inserts");
    for kind in [1u8, 2, 3, 4, 5, 0] {
        ctl.fx.insert[3].store(kind, Ordering::Relaxed);
        ctl.fx.insert[4].store((kind + 2) % 6, Ordering::Relaxed);
        ctl.fx.insert_amount[3].store(kind * 25, Ordering::Relaxed);
        for _ in 0..3 {
            assert_eq!(run(&mut core, &mut feed, &[]), none, "insertion effects");
        }
    }
    ctl.fx.insert[4].store(0, Ordering::Relaxed);
    // The Master Compressor and Master EQ: on, every type, then off (the compressor
    // gliding back to unity).
    {
        use yahaha::fx::master::{EqPreset, MasterComp, MasterEq};
        for (c, e) in CompPreset::ALL.into_iter().zip(EqPreset::ALL) {
            ctl.fx.master.set_compressor(&MasterComp::of(true, c));
            ctl.fx.master.set_eq(&MasterEq { on: true, preset: e, bands: e.bands() });
            for _ in 0..3 {
                assert_eq!(run(&mut core, &mut feed, &[]), none, "master compressor and EQ");
            }
        }
        ctl.fx.master.set_compressor(&MasterComp::default());
        ctl.fx.master.set_eq(&MasterEq::default());
        for _ in 0..3 {
            assert_eq!(run(&mut core, &mut feed, &[]), none, "master effects off");
        }
    }
    assert_eq!(run(&mut core, &mut feed, &[[0x8B, 60, 0], [0x8C, 64, 0]]), none, "inserts off");
    // The mixer strips: a compressor on a keyboard part and three Style parts, insert 2 on
    // two of them (one with its style insert as insert 1), sends 4-6 (a Hall and a chorus,
    // then a delay) fed from four channels, the sends' kinds changing while they sound
    // (each fades out and the new kind in), then all of it off again, the tails ringing
    // out. The control side's setters run between buffers, as the app's would.
    assert_eq!(run(&mut core, &mut feed, &[[0x90, 62, 100], [0x9A, 45, 100], [0x9B, 60, 100], [0x9C, 64, 100]]), none, "notes for the strips");
    ctl.fx.insert[3].store(yahaha::fx::InsertKind::Rotary as u8, Ordering::Relaxed);
    for (i, ch) in [0usize, 10, 11, 12].into_iter().enumerate() {
        ctl.fx.strips.set_comp(ch, &PartComp::of(true, CompPreset::ALL[(i + 2) % CompPreset::ALL.len()]));
        ctl.fx.strip_send[ch][0].store(60 + 20 * i as u8, Ordering::Relaxed);
        ctl.fx.strip_send[ch][1].store(127 - 30 * i as u8, Ordering::Relaxed);
        ctl.fx.strip_send[ch][2].store(40, Ordering::Relaxed);
    }
    ctl.fx.strips.set_second(11, &InsertSlot::of(InsertType::Distortion));
    ctl.fx.strips.set_second(12, &InsertSlot::of(InsertType::Phaser));
    ctl.fx.sends[0].set(&SendSlot::of(SendKind::Hall));
    ctl.fx.sends[1].set(&SendSlot::of(SendKind::Chorus));
    for _ in 0..6 {
        assert_eq!(run(&mut core, &mut feed, &[]), none, "strips and sends 4-5");
    }
    for (a, b) in [(SendKind::Plate, SendKind::PingPong), (SendKind::Room, SendKind::Phaser), (SendKind::Hall, SendKind::Flanger)] {
        ctl.fx.sends[0].set(&SendSlot::of(a));
        ctl.fx.sends[1].set(&SendSlot::of(b));
        ctl.fx.sends[2].set(&SendSlot::of(SendKind::Quarter));
        for _ in 0..3 {
            assert_eq!(run(&mut core, &mut feed, &[]), none, "a send's kind changing");
        }
    }
    ctl.fx.strips.set_comp(10, &PartComp::default());
    ctl.fx.strips.set_second(12, &InsertSlot::default());
    ctl.fx.sends[2].clear();
    assert_eq!(run(&mut core, &mut feed, &[[0x80, 62, 0], [0x8A, 45, 0], [0x8B, 60, 0], [0x8C, 64, 0]]), none, "strips: notes off");
    for _ in 0..6 {
        assert_eq!(run(&mut core, &mut feed, &[]), none, "strips: tails");
    }
    for ch in [0usize, 10, 11, 12] {
        ctl.fx.strips.set_comp(ch, &PartComp::default());
        ctl.fx.strips.set_second(ch, &InsertSlot::default());
        for s in &ctl.fx.strip_send[ch] {
            s.store(0, Ordering::Relaxed);
        }
    }
    ctl.fx.insert[3].store(0, Ordering::Relaxed);
    for _ in 0..3 {
        assert_eq!(run(&mut core, &mut feed, &[]), none, "strips off, sends 4-6 ringing out");
    }
    ctl.fx.legacy.store(true, Ordering::Relaxed);
    assert_eq!(run(&mut core, &mut feed, &[[0x90, 67, 100]]), none, "the SoundFont's own effects");
    ctl.fx.legacy.store(false, Ordering::Relaxed);
    assert_eq!(run(&mut core, &mut feed, &[[0x80, 60, 0], [0x80, 64, 0], [0x80, 67, 0], [0x8A, 40, 0]]), none, "the bus again");
    for _ in 0..20 {
        assert_eq!(run(&mut core, &mut feed, &[]), none, "tails");
    }
    // The performance view (`perf`, `--top`) collecting: per-stage, per-channel and
    // per-effect timing, voices and levels, the rings' depths. The view's own reads (and
    // their resets) run on its thread, outside `process`.
    yahaha::perf::enable();
    assert_eq!(run(&mut core, &mut feed, &[[0xB0, 91, 100], [0x90, 60, 100], [0x9A, 40, 100], [0x95, 64, 100]]), none, "profiling, notes");
    for _ in 0..10 {
        assert_eq!(run(&mut core, &mut feed, &[]), none, "profiling");
    }
    let snap = yahaha::perf::take(0.01);
    if font.is_some() {
        assert!(snap.voices > 0 && snap.stages[yahaha::perf::ST_BAND].0 > 0, "it measured: {snap:?}");
        assert!(snap.channels[0].sum_ns > 0 && snap.channels[0].voices > 0, "Right 1's voices: {:?}", snap.channels[0]);
    }
    assert_eq!(run(&mut core, &mut feed, &[[0x80, 60, 0], [0x8A, 40, 0], [0x85, 64, 0]]), none, "profiling, note offs");
    // The style's XG Drum Setup (#239): drum messages, drum notes starting with their own
    // level, pitch, pan (random too), sends, filter and envelope, a program change resetting
    // the setup, and a system reset. The setup's kits (#346 step 5): asked for, built off
    // the callback (`serve`, here between buffers), taken in and played, and a later setup's
    // kit replacing them, the old ones going back to be freed.
    let ds = |key: u8, p: u8, v: u8| synth::drum_setup::encode(&[0xF0, 0x43, 0x10, 0x4C, 0x30, key, p, v, 0xF7]).unwrap();
    let mut drum: Vec<[u8; 3]> = (0..16u8).map(|p| ds(38, p, if p == 4 { 0 } else { 0x50 })).collect();
    drum.extend([ds(36, 0x05, 0), ds(42, 0x02, 80), synth::drum_setup::encode(&[0xF0, 0x43, 0x10, 0x4C, 0x08, 0x08, 0x07, 0x03, 0xF7]).unwrap()]);
    assert_eq!(run(&mut core, &mut feed, &drum), none, "drum setup");
    assert_eq!(run(&mut core, &mut feed, &[[0xB9, 91, 100], [0x99, 38, 100], [0x99, 36, 110], [0x99, 42, 90], [0x98, 38, 90]]), none, "drum notes with their setup");
    let built = swap.kits.serve();
    for _ in 0..10 {
        assert_eq!(run(&mut core, &mut feed, &[[0x99, 38, 60], [0x99, 36, 60]]), none, "drum notes in their kit, ringing");
    }
    for level in [60u8, 70, 80, 90, 100, 110] {
        assert_eq!(run(&mut core, &mut feed, &[ds(42, 0x02, level), [0x99, 42, 100]]), none, "a setup change");
        swap.kits.serve();
        assert_eq!(run(&mut core, &mut feed, &[[0x99, 42, 100], [0x99, 38, 100]]), none, "its kit in, the one before it back");
    }
    // Style kits built as it loads.
    let plan = synth::drum_setup::prebuilds([&[0xC9u8, 1][..], &[0xF0, 0x43, 0x10, 0x4C, 0x30, 38, 0x02, 40, 0xF7]]);
    swap.kits.prebuild(plan);
    assert_eq!(run(&mut core, &mut feed, &[]), none, "a style's kits asked for");
    swap.kits.serve();
    let reset = synth::drum_setup::encode(&[0xF0, 0x43, 0x10, 0x4C, 0x00, 0x00, 0x7E, 0x00, 0xF7]).unwrap();
    assert_eq!(run(&mut core, &mut feed, &[[0xC9, 0, 0], [0x99, 38, 100], reset]), none, "drum setup resets");
    if font.is_some() {
        assert!(built > 0, "the setup's kits were built");
    }
    // A part's voice settings (#246): the sound controllers on new notes, the filter moving
    // on notes already sounding, vibrato.
    let tone: Vec<[u8; 3]> = (71..=78u8).map(|cc| [0xB1, cc, 20 + cc]).chain([[0x91, 60, 100], [0x91, 64, 90], [0xB9, 74, 30], [0x99, 38, 100]]).collect();
    assert_eq!(run(&mut core, &mut feed, &tone), none, "sound controllers");
    for v in [0u8, 127, 64, 10] {
        assert_eq!(run(&mut core, &mut feed, &[[0xB1, 74, v], [0xB1, 71, 127 - v]]), none, "the filter moving");
        assert_eq!(run(&mut core, &mut feed, &[]), none, "the filter gliding");
    }
    assert_eq!(run(&mut core, &mut feed, &[[0x81, 60, 0], [0x81, 64, 0], [0xB1, 121, 0]]), none, "sound controllers: notes off");
    // Portamento and mono (#246): gliding notes, CC126/127, the XG part's Mono/Poly as a
    // mono message, a mono note let go going back to the key still held (pedal down too).
    let xg_mono = |v: u8| synth::sysex_msg(&[0xF0, 0x43, 0x10, 0x4C, 0x08, 0x02, 0x05, v, 0xF7]).unwrap();
    assert_eq!(run(&mut core, &mut feed, &[[0xB2, 65, 127], [0xB2, 5, 40], [0x92, 60, 100], [0x92, 72, 100], xg_mono(0)]), none, "portamento");
    for _ in 0..4 {
        assert_eq!(run(&mut core, &mut feed, &[]), none, "gliding");
    }
    assert_eq!(run(&mut core, &mut feed, &[[0xB2, 64, 127], [0x92, 62, 90], [0x92, 65, 90], [0x92, 67, 90]]), none, "mono notes");
    assert_eq!(run(&mut core, &mut feed, &[[0x82, 67, 0], [0x82, 60, 0], [0x82, 65, 0], [0xB2, 64, 0]]), none, "mono: back to the keys held");
    assert_eq!(run(&mut core, &mut feed, &[[0xB3, 126, 1], [0x93, 60, 90], [0x93, 64, 90], [0xB3, 127, 0], xg_mono(1), [0x82, 62, 0]]), none, "mono/poly");
    // The keyboard parts' channel-strip EQ (#247): set on the control side (coefficients
    // computed there), taken in by the callback, played on notes sounding, changed, flat.
    assert_eq!(run(&mut core, &mut feed, &[[0x90, 60, 100], [0x91, 48, 100]]), none, "notes for the EQ");
    for (low, high) in [(6i8, -4i8), (-12, 12), (3, 0), (0, 0)] {
        parts.set_eq(0, PartEq { low_gain: low, low_freq: 200, high_gain: high, high_freq: 4_000 });
        parts.set_eq(1, PartEq { low_gain: high, ..PartEq::FLAT });
        for _ in 0..3 {
            assert_eq!(run(&mut core, &mut feed, &[]), none, "part EQ");
        }
    }
    parts.set_eq(0, PartEq { low_gain: 5, ..PartEq::FLAT });
    assert_eq!(run(&mut core, &mut feed, &[[0x80, 60, 0], [0x81, 48, 0]]), none, "part EQ, notes off");
    // The Style parts' EQ (channels 9-16, `FxControl::style_eq`): the same, from the fx
    // control.
    assert_eq!(run(&mut core, &mut feed, &[[0x98, 60, 100], [0x9B, 48, 100]]), none, "notes for the Style EQ");
    for (low, high) in [(6i8, -4i8), (-12, 12), (0, 0)] {
        ctl.fx.set_style_eq(0, PartEq { low_gain: low, low_freq: 200, high_gain: high, high_freq: 4_000 });
        ctl.fx.set_style_eq(3, PartEq { high_gain: low, ..PartEq::FLAT });
        for _ in 0..3 {
            assert_eq!(run(&mut core, &mut feed, &[]), none, "Style part EQ");
        }
    }
    ctl.fx.set_style_eq(3, PartEq::FLAT);
    assert_eq!(run(&mut core, &mut feed, &[[0x88, 60, 0], [0x8B, 48, 0]]), none, "Style part EQ, notes off");
    // A keyboard part's insert slot: on, its effect changed (a fade), off.
    assert_eq!(run(&mut core, &mut feed, &[[0x90, 60, 100]]), none, "a note for the insert");
    for (effect, on) in [(InsertEffect::Distortion, true), (InsertEffect::Rotary, true), (InsertEffect::Compressor, false)] {
        parts.set_insert(0, PartInsert { effect, on, amount: 100 });
        for _ in 0..3 {
            assert_eq!(run(&mut core, &mut feed, &[]), none, "insert slot");
        }
    }
    assert_eq!(run(&mut core, &mut feed, &[[0x80, 60, 0]]), none, "insert slot, note off");
    ctl.master.store(90, Ordering::Relaxed);
    parts.set_program(0, 5);
    assert_eq!(run(&mut core, &mut feed, &[[0xB0, 1, 30], [0xE0, 0, 80]]), none, "master, program, controllers");
    // A new SoundFont swapped in: replayed, the old one fades and goes back.
    if let Some(f) = &font {
        swap.tx.push(Rack::load(f, 48_000).unwrap()).ok().unwrap();
        for _ in 0..3 {
            assert_eq!(run(&mut core, &mut feed, &[]), none, "SoundFont swap");
        }
        while swap.old.pop().is_ok() {}

        // The control side stops taking old racks back (busy, or not pumping): the return
        // ring fills. The callback holds further swaps until there is room again, rather
        // than freeing a rack itself.
        let sf = yahaha::synth::font::open(f).unwrap();
        let cap = swap.old.buffer().capacity();
        let before = ctl.swaps.load(Ordering::Relaxed);
        let mut sent = 0;
        for _ in 0..(cap + 3) * 2 {
            if swap.tx.push(Box::new(Rack::new(&sf, 48_000).unwrap())).is_ok() {
                sent += 1;
            }
            assert_eq!(run(&mut core, &mut feed, &[[0x90, 62, 90]]), none, "SoundFont swaps with the return ring full");
        }
        let taken = (ctl.swaps.load(Ordering::Relaxed) - before) as usize;
        assert_eq!(taken, cap, "swaps wait once the return ring is full");
        assert_eq!(swap.old.slots(), taken, "every replaced rack came back");
        // The control side drains the ring: the waiting racks go in.
        while swap.old.pop().is_ok() {}
        for _ in 0..8 {
            assert_eq!(run(&mut core, &mut feed, &[]), none, "swaps resume");
            while swap.old.pop().is_ok() {}
        }
        assert_eq!((ctl.swaps.load(Ordering::Relaxed) - before) as usize, sent, "every rack sent was taken");
    }

    #[cfg(feature = "plugins")]
    {
        use yahaha::plugin::{LoadConfig, PluginHost, PluginId, Swap};
        use yahaha::route::Source;
        let mut link = _link.unwrap();
        let dls = || PluginHost::new(None).load(&PluginId::DLS, LoadConfig { max_frames: synth::PLUGIN_MAX_BLOCK as u32, ..Default::default() }).unwrap();
        let (a, b) = (dls(), dls());
        // Right 1 (channel 0) over to the plugin.
        link.assign(0, a, Swap::default()).ok().unwrap();
        ctl.routes.set(0, Source::Plugin);
        assert_eq!(run(&mut core, &mut feed, &[[0x90, 64, 100], [0xB0, 7, 110], [0xB0, 10, 30], [0xB0, 64, 127]]), none, "the part goes over to its plugin");
        assert_eq!(core.plugin_channels(), 1);
        for _ in 0..10 {
            assert_eq!(run(&mut core, &mut feed, &[[0x90, 67, 90], [0x80, 64, 0]]), none, "plugin playing");
        }
        // The part's XG settings in front of the plugin (#247): mono, a velocity curve, and
        // its EQ on the plugin's output.
        let xg = |nn: u8, v: u8| synth::sysex_msg(&[0xF0, 0x43, 0x10, 0x4C, 0x08, 0x00, nn, v, 0xF7]).unwrap();
        parts.set_eq(0, PartEq { low_gain: -6, high_gain: 8, ..PartEq::FLAT });
        assert_eq!(run(&mut core, &mut feed, &[xg(0x05, 0), xg(0x0C, 90), xg(0x0D, 50), [0x90, 60, 100], [0x90, 64, 100]]), none, "plugin: mono, velocity, EQ");
        for _ in 0..4 {
            assert_eq!(run(&mut core, &mut feed, &[[0x80, 64, 0], [0xB0, 64, 0], [0x90, 67, 80], [0xB0, 64, 127]]), none, "plugin: mono notes");
        }
        assert_eq!(run(&mut core, &mut feed, &[xg(0x05, 1), [0x80, 60, 0], [0x80, 67, 0], [0xB0, 64, 0]]), none, "plugin: poly again");
        parts.set_eq(0, PartEq::FLAT);
        // Its insert slot on the plugin's output.
        parts.set_insert(0, PartInsert { effect: InsertEffect::Tremolo, on: true, amount: 90 });
        for _ in 0..3 {
            assert_eq!(run(&mut core, &mut feed, &[[0x90, 62, 90], [0x80, 62, 0]]), none, "plugin: insert");
        }
        parts.set_insert(0, PartInsert::OFF);
        // Its mixer strip on the plugin's output: a compressor, insert 2, sends 4 and 5 (a
        // reverb, then its kind changing while it sounds).
        ctl.fx.strips.set_comp(0, &PartComp::of(true, CompPreset::Loud));
        ctl.fx.strips.set_second(0, &InsertSlot::of(InsertType::Tremolo));
        ctl.fx.strip_send[0][0].store(110, Ordering::Relaxed);
        ctl.fx.strip_send[0][1].store(70, Ordering::Relaxed);
        ctl.fx.sends[0].set(&SendSlot::of(SendKind::Stage));
        ctl.fx.sends[1].set(&SendSlot::of(SendKind::Eighth));
        for kind in [SendKind::Stage, SendKind::Room, SendKind::Phaser] {
            ctl.fx.sends[0].set(&SendSlot::of(kind));
            for _ in 0..3 {
                assert_eq!(run(&mut core, &mut feed, &[[0x90, 62, 90], [0x80, 62, 0]]), none, "plugin: strip and sends 4-5");
            }
        }
        ctl.fx.strips.set_comp(0, &PartComp::default());
        ctl.fx.strips.set_second(0, &InsertSlot::default());
        ctl.fx.strip_send[0][0].store(0, Ordering::Relaxed);
        ctl.fx.strip_send[0][1].store(0, Ordering::Relaxed);
        assert_eq!(run(&mut core, &mut feed, &[]), none, "plugin: strip off");
        link.assign(0, b, Swap::default()).ok().unwrap();
        for _ in 0..6 {
            assert_eq!(run(&mut core, &mut feed, &[[0xB0, 7, 90]]), none, "crossfade to a second instance");
        }
        link.clear(0, 240);
        ctl.routes.set(0, Source::SoundFont(0));
        for _ in 0..6 {
            assert_eq!(run(&mut core, &mut feed, &[[0x90, 60, 100]]), none, "back to the SoundFont");
        }
        assert_eq!(core.plugin_channels(), 0);
        let _ = link.take_retired();
    }
}

/// The meters on their own: every channel's peak and RMS, the Multi Pads' included, and
/// the master's are measured in the callback without allocating, reading after reading
/// (the app shell takes them at 30 Hz). Without a SoundFont the path still runs (silent).
#[test]
fn the_meters_do_not_allocate() {
    let font = sound_font();
    let rack = font.as_ref().map(|f| Rack::load(f, 48_000).unwrap());
    let (mut feed, rx) = rtrb::RingBuffer::<synth::Msg>::new(256);
    let ctl = Arc::new(SynthControl::new(0));
    let (mut core, _swap, _link) = AudioCore::new(rack, vec![rx], Arc::new(Parts::new()), ctl.clone(), 48_000, 2);
    let mut out = vec![0f32; 128];
    // Right 1, a Multi Pad (ch 5) and the Bass part.
    for m in [[0x90u8, 60, 100], [0x94, 64, 100], [0x9A, 40, 100]] {
        feed.push(m).unwrap();
    }
    let mut got = (0, 0);
    let mut seen = [0f32; 16];
    let mut seen_rms = [0f32; 16];
    for _ in 0..30 {
        let (a, f) = counted(|| core.process(&mut out));
        got = (got.0 + a, got.1 + f);
        let (peaks, _, _) = synth::take_meters(&ctl);
        let (rms, _) = synth::take_rms(&ctl);
        for ch in 0..16 {
            seen[ch] = seen[ch].max(peaks[ch]);
            seen_rms[ch] = seen_rms[ch].max(rms[ch]);
        }
    }
    assert_eq!(got, (0, 0), "metering allocated");
    if font.is_some() {
        for ch in [0, 4, 10] {
            assert!(seen[ch] > 0.0 && seen_rms[ch] > 0.0 && seen_rms[ch] <= seen[ch], "ch {}: peak {} rms {}", ch + 1, seen[ch], seen_rms[ch]);
        }
        assert_eq!(seen_rms[1], 0.0, "a silent channel reads 0");
    }
}

/// Each track's CPU (#340) is timed in the callback, per channel, into atomics, without
/// allocating: the channels playing take time, a silent one none, and every buffer counts
/// its length (64 frames at 48 kHz). Off the callback, a reading over a second of buffers
/// turns them into shares.
#[test]
fn track_cpu_is_measured_per_track_without_allocating() {
    const BUFFERS: u64 = 760;
    let font = sound_font();
    let rack = font.as_ref().map(|f| Rack::load(f, 48_000).unwrap());
    let (mut feed, rx) = rtrb::RingBuffer::<synth::Msg>::new(256);
    let ctl = Arc::new(SynthControl::new(0));
    let (mut core, _swap, _link) = AudioCore::new(rack, vec![rx], Arc::new(Parts::new()), ctl.clone(), 48_000, 2);
    let mut out = vec![0f32; 128];
    // Right 1 and the Bass part.
    for m in [[0x90u8, 60, 100], [0x9A, 40, 100]] {
        feed.push(m).unwrap();
    }
    let mut window = synth::CpuWindow::default();
    window.read(&ctl.cpu);
    let got = counted(|| {
        for _ in 0..BUFFERS {
            core.process(&mut out);
        }
    });
    assert_eq!(got, (0, 0), "timing the tracks allocated");
    assert_eq!(ctl.cpu.buffers.load(Ordering::Relaxed), BUFFERS);
    assert_eq!(ctl.cpu.budget_ns.load(Ordering::Relaxed), BUFFERS * 1_333_333, "64 frames at 48 kHz, each");
    let r = window.read(&ctl.cpu);
    assert!((r.buffer_us - 1333.333).abs() < 0.01, "{r:?}");
    if font.is_some() {
        let ns: Vec<u64> = ctl.cpu.track_ns.iter().map(|a| a.load(Ordering::Relaxed)).collect();
        assert!(ns[0] > 0 && ns[10] > 0, "Right 1 and the Bass part took time: {ns:?}");
        assert_eq!(ns[1], 0, "Left played nothing");
        assert!(r.track[0] > 0.0 && r.track[10] > 0.0 && r.track[1] == 0.0, "{r:?}");
        assert!(r.track_peak[0] >= r.track[0] && r.total >= r.track[0] + r.track[10], "{r:?}");
    }
}

/// A track a plugin plays (#340): its time is the plugin's render, on its own channel only.
/// No SoundFont here, so nothing else takes any time.
#[cfg(feature = "plugins")]
#[test]
fn a_plugin_tracks_cpu_is_its_render() {
    use yahaha::plugin::{LoadConfig, PluginHost, PluginId, Swap};
    use yahaha::route::Source;
    let (mut feed, rx) = rtrb::RingBuffer::<synth::Msg>::new(256);
    let ctl = Arc::new(SynthControl::new(0));
    let (mut core, _swap, link) = AudioCore::new(None, vec![rx], Arc::new(Parts::new()), ctl.clone(), 48_000, 2);
    let mut link = link.unwrap();
    let dls = PluginHost::new(None).load(&PluginId::DLS, LoadConfig { max_frames: synth::PLUGIN_MAX_BLOCK as u32, ..Default::default() }).unwrap();
    // Right 2 (channel 2) plays it.
    link.assign(2, dls, Swap::default()).ok().unwrap();
    ctl.routes.set(2, Source::Plugin);
    feed.push([0x92, 64, 100]).unwrap();
    let mut out = vec![0f32; 128];
    let got = counted(|| {
        for _ in 0..20 {
            core.process(&mut out);
        }
    });
    assert_eq!(got, (0, 0), "timing the plugin's track allocated");
    let ns: Vec<u64> = ctl.cpu.track_ns.iter().map(|a| a.load(Ordering::Relaxed)).collect();
    assert!(ns[2] > 0, "the plugin's track took time: {ns:?}");
    assert!(ns.iter().enumerate().all(|(ch, &t)| ch == 2 || t == 0), "only its own: {ns:?}");
    link.clear(2, 0);
    ctl.routes.set(2, Source::SoundFont(0));
    core.process(&mut out);
    let _ = link.take_retired();
}
