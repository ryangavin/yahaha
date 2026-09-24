//! Tests with Apple's DLSMusicDevice, which every Mac has.

use super::*;
use std::time::Duration;

const RATE: f64 = 48_000.0;

fn host() -> PluginHost {
    PluginHost::new(None)
}

fn dls(max_frames: u32) -> PluginInstance {
    host()
        .load(&PluginId::DLS, LoadConfig { sample_rate: RATE, max_frames, ..Default::default() })
        .expect("Apple's DLSMusicDevice is always installed")
}

fn energy(l: &[f32], r: &[f32]) -> f64 {
    l.iter().chain(r).map(|x| (*x as f64).powi(2)).sum()
}

fn render(inst: &mut PluginInstance, frames: usize) -> f64 {
    let (mut l, mut r) = (vec![0f32; frames], vec![0f32; frames]);
    inst.render(&mut l, &mut r).unwrap();
    energy(&l, &r)
}

#[test]
fn part_gain_follows_the_synths_curve_and_swallows_volume() {
    let mut g = PartGain::new();
    assert!((g.target() - ((12800.0f32 / 16383.0) * (16256.0 / 16383.0)).powi(2)).abs() < 1e-6, "power-on CC7 100, CC11 127");
    assert!(g.take([0xB0, 7, 127]));
    assert!(g.take([0xB0, 39, 127]));
    assert!(g.take([0xB0, 11, 127]));
    assert!(g.take([0xB0, 43, 127]));
    assert_eq!(g.target(), 1.0);
    assert!(g.take([0xB3, 7, 64]));
    assert!(g.take([0xB3, 39, 0]));
    assert!((g.target() - PartGain::curve(64, 127) * (16383.0f32 / (127 << 7) as f32).powi(2)).abs() < 1e-3);
    assert!(!g.take([0xB0, 1, 20]), "mod wheel goes to the plugin");
    assert!(!g.take([0x90, 7, 100]), "a note-on is not CC7");
    g.take([0xB0, 11, 0]);
    assert!(!g.take([0xB0, 121, 0]), "Reset All Controllers is forwarded");
    assert!(g.target() > 0.0, "and restores expression, as rustysynth does");
    let (mut l, mut r) = (vec![1.0f32; 64], vec![1.0f32; 64]);
    g.apply(&mut l, &mut r);
    assert!((l[0] - g.target()).abs() < 1e-6, "the first block starts at the target, no fade-in");
    l.fill(1.0);
    r.fill(1.0);
    g.take([0xB0, 7, 0]);
    g.apply(&mut l, &mut r);
    assert!(l[0] > 0.0 && l[63].abs() < 1e-6, "ramps down within the block");
}

/// The rack must scale a plugin part exactly as rustysynth scales a SoundFont part: the
/// same 40·log10 dB per controller (checked here against the formula `synth.rs`'s own
/// `velocity_cc7_cc11_follow_gm_curves` test asserts for the SoundFont).
#[test]
fn part_gain_matches_the_gm_curve_in_db() {
    let db = |cc7: u8, cc11: u8| 20.0 * (PartGain::curve(cc7, cc11) as f64 / PartGain::curve(127, 127) as f64).log10();
    let gm = |v: f64| 40.0 * (v / 127.0).log10();
    for (cc7, cc11, want) in [(64, 127, gm(64.0)), (100, 127, gm(100.0)), (127, 64, gm(64.0)), (100, 90, gm(100.0) + gm(90.0))] {
        assert!((db(cc7, cc11) - want).abs() < 0.01, "CC7 {cc7} CC11 {cc11}");
    }
}

#[test]
fn scan_finds_dls_and_caches_it() {
    let dir = std::env::temp_dir().join(format!("yahaha-plugin-scan-{}", std::process::id()));
    let path = dir.join("plugins.json");
    let _ = std::fs::remove_dir_all(&dir);
    let h = PluginHost::new(Some(path.clone()));
    let all = h.scan().unwrap();
    let d = all.iter().find(|p| p.id == PluginId::DLS).expect("DLSMusicDevice in the scan");
    assert_eq!(d.manufacturer, "Apple");
    assert_eq!(d.name, "DLSMusicDevice");
    assert_eq!(d.format, PluginFormat::Au2);
    assert!(d.version > 0);
    assert!(path.exists(), "the scan was cached");
    // A second host (a new app launch) is served from the file.
    let again = PluginHost::new(Some(path.clone())).scan().unwrap();
    assert_eq!(again, all);
    // A load is recorded in the cache.
    let _inst = h.load(&PluginId::DLS, LoadConfig::default()).unwrap();
    let cached = PluginHost::new(Some(path.clone())).scan().unwrap();
    let rec = cached.iter().find(|p| p.id == PluginId::DLS).unwrap().last_load.clone().expect("load recorded");
    assert!(rec.ms.is_some() && rec.error.is_none());
    assert!(h.find("dlsmusic").is_ok());
    assert!(h.find("aumu dls  appl").is_ok());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn dls_renders_a_note_offline() {
    let mut inst = dls(512);
    assert!(!inst.out_of_process());
    assert!(render(&mut inst, 4800) < 1e-9, "silent before any note");
    inst.midi([0x90, 60, 110], 0).unwrap();
    // 3000 frames is not a multiple of max_frames: exercises the slicing.
    assert!(render(&mut inst, 3000) > 1e-3, "note-on makes sound");
    let s = inst.stats().snapshot(RATE);
    assert!(s.blocks >= 2 && s.errors == 0 && s.mean_us > 0.0);
}

#[test]
fn midi_offsets_are_sample_accurate() {
    let mut inst = dls(512);
    inst.midi([0x90, 72, 127], 300).unwrap();
    let (mut l, mut r) = (vec![0f32; 512], vec![0f32; 512]);
    inst.render(&mut l, &mut r).unwrap();
    let first = l.iter().zip(&r).position(|(a, b)| a.abs() + b.abs() > 1e-6).expect("the note sounds");
    assert!((300..310).contains(&first), "note starts at frame {first}, asked for 300");
}

#[test]
fn state_round_trips_and_preloads() {
    let mut inst = dls(512);
    let state = inst.get_state().unwrap();
    assert!(state.starts_with(b"bplist00"));
    inst.set_state(&state).unwrap();
    assert!(inst.set_state(b"not a plist").is_err());
    // A preload with the state restores it before the hand-over.
    let pre = host().load(&PluginId::DLS, LoadConfig { state: Some(state.clone()), ..Default::default() }).unwrap();
    assert!(pre.load_times().total() > Duration::ZERO);
    assert_eq!(pre.get_state().unwrap().len(), state.len());
}

#[test]
fn load_async_reports_progress_and_finishes() {
    let mut h = host().load_async(&PluginId::DLS, LoadConfig::default()).unwrap();
    let t0 = std::time::Instant::now();
    let inst = loop {
        if let Some(r) = h.take() {
            break r.unwrap();
        }
        assert!(!matches!(h.progress(), LoadProgress::Failed(_) | LoadProgress::TimedOut(_)));
        assert!(t0.elapsed() < Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(1));
    };
    assert_eq!(inst.info().id, PluginId::DLS);
}

#[test]
fn a_load_past_its_deadline_times_out_and_is_abandoned() {
    // No plugin hangs on demand; a zero deadline stands in for one that never returns.
    let h = host().load_async(&PluginId::DLS, LoadConfig { timeout: Duration::ZERO, ..Default::default() }).unwrap();
    assert!(matches!(h.progress(), LoadProgress::TimedOut(_)));
    let err = h.wait().err().expect("timed out");
    assert!(format!("{err}").contains("did not load"), "{err}");
    // The abandoned load thread disposes of the instance when the plugin finishes; give it
    // the time and make sure nothing blows up.
    std::thread::sleep(Duration::from_millis(200));
}

#[test]
fn unknown_plugins_fail_cleanly() {
    let bogus = PluginId::parse("aumu zzzz zzzz").unwrap();
    assert!(host().load_async(&bogus, LoadConfig::default()).is_err());
}

#[test]
fn out_of_process_loading_works_for_v2_units() {
    // macOS 11+ can host any AU in the AUHostingService; this is the AUv3 path too.
    let r = host().load(&PluginId::DLS, LoadConfig { mode: LoadMode::OutOfProcess, max_frames: 512, timeout: Duration::from_secs(20), ..Default::default() });
    let mut inst = match r {
        Ok(i) => i,
        Err(e) => {
            eprintln!("out-of-process hosting unavailable here: {e:#}; skipping");
            return;
        }
    };
    assert!(inst.out_of_process());
    inst.midi([0x90, 60, 110], 0).unwrap();
    let e = render(&mut inst, 512) + render(&mut inst, 2048);
    assert!(e > 1e-3, "sounds out of process");
    assert!(inst.get_state().unwrap().starts_with(b"bplist00"));
}

/// Run one rack block: begin, MIDI, render into zeroed buffers.
fn block(rack: &mut PluginRack, msgs: &[[u8; 3]], frames: usize) -> (Vec<f32>, Vec<f32>) {
    rack.begin_block();
    for m in msgs {
        rack.midi(*m, 0);
    }
    let (mut l, mut r) = (vec![0f32; frames], vec![0f32; frames]);
    rack.render_add(&mut l, &mut r);
    (l, r)
}

#[test]
fn rack_owns_assigned_channels_and_applies_cc7() {
    let (mut rack, mut ctl) = rack(256, RATE);
    assert!(!rack.midi([0x90, 60, 100], 0), "nothing assigned: the caller's synth plays it");
    ctl.assign(0, dls(256), Swap { fade_frames: 0, trim: 1.0 }).ok().unwrap();
    let (l, r) = block(&mut rack, &[[0xB0, 7, 127], [0x90, 60, 110]], 256);
    assert!(rack.owns(0) && !rack.owns(1));
    assert!(!rack.midi([0x91, 60, 100], 0), "other channels are not the rack's");
    assert!(matches!(ctl.poll().as_slice(), [RackEvent::Swapped { channel: 0, .. }]));
    let mut loud = energy(&l, &r);
    for _ in 0..8 {
        let (l, r) = block(&mut rack, &[], 256);
        loud += energy(&l, &r);
    }
    assert!(loud > 1e-3);
    // Fader to 64: the same note, -11.7 dB (40·log10(64/127)) on the rack's gain, whatever
    // the plugin does with CC7 (it never sees it).
    let (_, _) = block(&mut rack, &[[0xB0, 7, 64]], 256); // the ramp block
    let mut a = 0.0;
    for _ in 0..4 {
        let (l, r) = block(&mut rack, &[], 256);
        a += energy(&l, &r);
    }
    let _ = block(&mut rack, &[[0xB0, 7, 127]], 256);
    let mut b = 0.0;
    for _ in 0..4 {
        let (l, r) = block(&mut rack, &[], 256);
        b += energy(&l, &r);
    }
    let db = 10.0 * (a / b).log10();
    // The note decays between the two windows, so allow a little.
    assert!((db - 40.0 * (64.0f64 / 127.0).log10()).abs() < 1.5, "{db:.2} dB");
}

#[test]
fn rack_swaps_at_the_block_boundary_with_a_crossfade() {
    let (mut rack, mut ctl) = rack(128, RATE);
    ctl.assign(0, dls(128), Swap::default()).ok().unwrap();
    let _ = block(&mut rack, &[[0xB0, 1, 90], [0xB0, 7, 127], [0x90, 64, 120]], 128);
    for _ in 0..10 {
        block(&mut rack, &[], 128);
    }
    // Hand over to a second, preloaded instance: continuous level at the boundary (no gap
    // to silence), the old one comes back to the control side after the fade.
    let (prev_l, _) = block(&mut rack, &[], 128);
    ctl.assign(0, dls(128), Swap::default()).ok().unwrap();
    let (l, _) = block(&mut rack, &[], 128);
    let edge = (l[0] - prev_l[127]).abs();
    let typical = prev_l.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0f32, f32::max);
    assert!(edge <= typical * 4.0 + 1e-4, "no click at the swap: edge {edge}, typical step {typical}");
    let ev = ctl.poll_events();
    assert!(ev.iter().filter(|e| matches!(e, RackEvent::Swapped { .. })).count() == 2, "{ev:?}");
    assert!(ctl.take_retired().is_empty(), "still fading out");
    for _ in 0..3 {
        block(&mut rack, &[], 128);
    }
    assert_eq!(ctl.take_retired().len(), 1, "the outgoing instance is back after the 240-frame fade");
    // Clearing gives the channel back.
    ctl.clear(0, 0);
    block(&mut rack, &[], 128);
    assert!(!rack.owns(0));
    assert!(ctl.poll().contains(&RackEvent::Cleared { channel: 0 }));
}

#[test]
fn a_failing_plugin_is_muted_and_reported() {
    let (mut rack, mut ctl) = rack(512, RATE);
    // A unit configured for 64-frame slices, told it may render 512: AudioUnitRender fails
    // with TooManyFramesToProcess, as a misbehaving plugin would.
    let mut inst = dls(64);
    inst.force_max_frames(512);
    ctl.assign(3, inst, Swap::default()).ok().unwrap();
    let (l, r) = block(&mut rack, &[[0x93, 60, 100]], 512);
    assert_eq!(energy(&l, &r), 0.0);
    let ev = ctl.poll();
    assert!(ev.iter().any(|e| matches!(e, RackEvent::Fault { channel: 3, error: RenderError::Status(-10874) })), "{ev:?}");
    assert!(rack.owns(3), "the part stays muted rather than falling back to the SoundFont");
    assert!(rack.midi([0x93, 62, 100], 0), "and swallows its MIDI");
    // Later blocks stay silent and quiet (no repeated fault events).
    block(&mut rack, &[], 512);
    assert!(ctl.poll().is_empty());
    // A fresh instance brings the part back.
    ctl.assign(3, dls(512), Swap::default()).ok().unwrap();
    let _ = block(&mut rack, &[[0x93, 60, 100]], 512);
    let (l, r) = block(&mut rack, &[], 512);
    assert!(energy(&l, &r) > 0.0);
}

#[test]
fn slow_renders_are_counted_as_overruns() {
    let mut inst = dls(512);
    let stats = inst.stats();
    // A zero budget makes every render an overrun.
    stats.budget_permille.store(0, std::sync::atomic::Ordering::Relaxed);
    let (mut rack, mut ctl) = rack(512, RATE);
    inst.midi([0x90, 60, 100], 0).unwrap();
    ctl.assign(0, inst, Swap::default()).ok().unwrap();
    for _ in 0..4 {
        block(&mut rack, &[], 512);
    }
    let s = stats.snapshot(RATE);
    assert_eq!(s.overruns, 4);
    let overruns = ctl.poll().into_iter().filter(|e| matches!(e, RackEvent::Overrun { channel: 0, .. })).count();
    assert_eq!(overruns, 1, "events are rate-limited to one per second per slot");
}

/// A part's CC7 sent while the SoundFont still played it (before any plugin was assigned)
/// sets the plugin's level too: the rack tracks it on unowned channels, while still leaving
/// the message to the caller's synth.
#[test]
fn a_plugin_assigned_later_starts_at_the_parts_level() {
    let run = |cc7_before_assign: bool| {
        let (mut rack, mut ctl) = rack(256, RATE);
        if cc7_before_assign {
            assert!(!rack.midi([0xB0, 7, 20], 0), "unowned: still the caller's synth's message");
        }
        ctl.assign(0, dls(256), Swap { fade_frames: 0, trim: 1.0 }).ok().unwrap();
        let first: &[[u8; 3]] = if cc7_before_assign { &[[0x90, 60, 110]] } else { &[[0xB0, 7, 20], [0x90, 60, 110]] };
        let (l, r) = block(&mut rack, first, 256);
        let mut e = energy(&l, &r);
        for _ in 0..4 {
            let (l, r) = block(&mut rack, &[], 256);
            e += energy(&l, &r);
        }
        e
    };
    let db = 10.0 * (run(true) / run(false)).log10();
    assert!(db.abs() < 0.5, "CC7 sent before the assign is honoured: {db:.2} dB off");
}

#[test]
fn a_non_finite_trim_cannot_poison_the_mix() {
    let (mut rack, mut ctl) = rack(256, RATE);
    ctl.assign(0, dls(256), Swap { fade_frames: 0, trim: f32::NAN }).ok().unwrap();
    let (l, r) = block(&mut rack, &[[0x90, 60, 110]], 256);
    assert!(l.iter().chain(&r).all(|x| x.is_finite()));
    assert!(energy(&l, &r) > 0.0, "treated as no trim");
}

/// CC10 is the host's, as CC7 is: a balance on the plugin's output, never sent to it.
#[test]
fn pan_is_a_host_side_balance() {
    assert_eq!(balance(64), (1.0, 1.0));
    assert_eq!(balance(0), (1.0, 0.0));
    assert_eq!(balance(127), (0.0, 1.0));
    let side = |pan: u8| {
        let (mut rack, mut ctl) = rack(256, RATE);
        ctl.assign(0, dls(256), Swap { fade_frames: 0, trim: 1.0 }).ok().unwrap();
        let (mut el, mut er) = (0.0, 0.0);
        let _ = block(&mut rack, &[[0xB0, 10, pan], [0x90, 60, 110]], 256);
        for _ in 0..8 {
            let (l, r) = block(&mut rack, &[], 256);
            el += energy(&l, &[]);
            er += energy(&[], &r);
        }
        (el, er, rack.take_peak(0))
    };
    let (cl, cr, peak) = side(64);
    assert!(cl > 0.0 && cr > 0.0 && peak > 0.0, "centre: both sides, metered");
    let (ll, lr, _) = side(0);
    assert!(ll > 0.0 && lr == 0.0, "hard left: left only ({ll} / {lr})");
    assert!((ll - cl).abs() < cl * 0.01, "the left side at hard left is the centre's left: a balance, not a boost");
}

/// A second swap sent while the first is still crossfading waits for that fade to end,
/// instead of cutting the outgoing instance off mid-fade.
#[test]
fn a_swap_during_a_crossfade_waits_for_it() {
    let (mut rack, mut ctl) = rack(64, RATE);
    ctl.assign(0, dls(64), Swap::default()).ok().unwrap();
    let _ = block(&mut rack, &[[0x90, 60, 110]], 64);
    ctl.assign(0, dls(64), Swap::default()).ok().unwrap();
    block(&mut rack, &[], 64);
    assert!(rack.is_fading(0));
    ctl.assign(0, dls(64), Swap::default()).ok().unwrap();
    let swapped = |ctl: &mut RackControl| ctl.poll_events().iter().filter(|e| matches!(e, RackEvent::Swapped { .. })).count();
    assert_eq!(swapped(&mut ctl), 2);
    block(&mut rack, &[], 64);
    assert_eq!(swapped(&mut ctl), 0, "the third assign waits: 240-frame fade, 128 frames in");
    for _ in 0..4 {
        block(&mut rack, &[], 64);
    }
    assert_eq!(swapped(&mut ctl), 1, "then it lands");
}

/// A command waiting for one channel's crossfade does not hold up another channel's
/// (#104 review item 2): Right 2's assign, sent after Right 1's, lands at once.
#[test]
fn a_waiting_swap_does_not_hold_up_other_channels() {
    let (mut rack, mut ctl) = rack(64, RATE);
    ctl.assign(0, dls(64), Swap::default()).ok().unwrap();
    let _ = block(&mut rack, &[[0x90, 60, 110]], 64);
    ctl.assign(0, dls(64), Swap::default()).ok().unwrap();
    block(&mut rack, &[], 64);
    assert!(rack.is_fading(0));
    let _ = ctl.poll_events();
    ctl.assign(0, dls(64), Swap::default()).ok().unwrap();
    ctl.assign(2, dls(64), Swap::default()).ok().unwrap();
    block(&mut rack, &[], 64);
    let ev = ctl.poll_events();
    assert_eq!(ev.iter().filter(|e| matches!(e, RackEvent::Swapped { channel: 2, .. })).count(), 1, "Right 2 lands now: {ev:?}");
    assert!(!ev.iter().any(|e| matches!(e, RackEvent::Swapped { channel: 0, .. })), "Right 1 still waits: {ev:?}");
    assert!(rack.owns(2));
    for _ in 0..4 {
        block(&mut rack, &[], 64);
    }
    let ev = ctl.poll_events();
    assert_eq!(ev.iter().filter(|e| matches!(e, RackEvent::Swapped { channel: 0, .. })).count(), 1, "then Right 1's: {ev:?}");
}

/// Two commands for a crossfading channel: the later one counts. A clear after an assign
/// that was still waiting clears the channel, and the waiting instance goes back to the
/// control side without playing.
#[test]
fn the_latest_waiting_command_for_a_channel_wins() {
    let (mut rack, mut ctl) = rack(64, RATE);
    ctl.assign(0, dls(64), Swap::default()).ok().unwrap();
    block(&mut rack, &[], 64);
    ctl.assign(0, dls(64), Swap::default()).ok().unwrap();
    block(&mut rack, &[], 64);
    assert!(rack.is_fading(0));
    let _ = ctl.poll_events();
    ctl.assign(0, dls(64), Swap::default()).ok().unwrap();
    assert!(ctl.clear(0, 0));
    block(&mut rack, &[], 64);
    assert_eq!(ctl.take_retired().len(), 1, "the replaced assign's instance came back unplayed");
    for _ in 0..4 {
        block(&mut rack, &[], 64);
    }
    let ev = ctl.poll_events();
    assert!(!ev.iter().any(|e| matches!(e, RackEvent::Swapped { .. })), "{ev:?}");
    assert!(ev.contains(&RackEvent::Cleared { channel: 0 }), "{ev:?}");
    assert!(!rack.owns(0) && !rack.active());
}

/// The strongest autocorrelation lag (the pitch period, in samples) between `lo` and `hi`.
fn period(x: &[f32], lo: usize, hi: usize) -> usize {
    (lo..hi)
        .max_by(|&a, &b| {
            let c = |lag: usize| x.iter().zip(&x[lag..]).map(|(p, q)| (p * q) as f64).sum::<f64>();
            c(a).partial_cmp(&c(b)).unwrap()
        })
        .unwrap()
}

/// A plugin assigned after the part's Pitch Bend Range (RPN 0) was set plays with that
/// range: the rack replays RPN 0-2 into the incoming instance, as the SoundFont side's
/// `Shadow` does for a new SoundFont (#105 review B1).
#[test]
fn a_plugin_assigned_later_gets_the_parts_bend_range() {
    let run = |range_before_assign: bool| {
        let (mut rack, mut ctl) = rack(512, RATE);
        let range: [[u8; 3]; 5] = [[0xB0, 101, 0], [0xB0, 100, 0], [0xB0, 6, 12], [0xB0, 38, 0], [0xB0, 101, 127]];
        if range_before_assign {
            for m in range {
                assert!(!rack.midi(m, 0));
            }
        }
        ctl.assign(0, dls(512), Swap { fade_frames: 0, trim: 1.0 }).ok().unwrap();
        let mut first: Vec<[u8; 3]> = if range_before_assign { vec![] } else { range.to_vec() };
        first.extend([[0xE0, 127, 127], [0x90, 57, 110]]);
        let _ = block(&mut rack, &first, 512);
        let mut l = Vec::new();
        for _ in 0..8 {
            l.extend(block(&mut rack, &[], 512).0);
        }
        period(&l[2048..], 60, 400)
    };
    let (sent_after, replayed) = (run(false), run(true));
    // A3 (220 Hz) bent up 12 semitones: 440 Hz, a 109-sample period at 48 kHz (+2 would be 196).
    assert!((100..120).contains(&sent_after), "the range sent to the plugin itself: {sent_after}");
    assert!(replayed.abs_diff(sent_after) <= 2, "replayed on assign: {replayed} vs {sent_after}");
}

/// Only a typed "the system won't host this out of process" status allows an in-process
/// retry: never a timeout, a crash of the hosting process, a later stage, or an error that
/// merely mentions the code (#105 review B3).
#[test]
fn only_a_refusal_to_host_out_of_process_retries_in_process() {
    let st = |status, what| anyhow::Error::new(StatusError { status, what });
    assert!(may_retry_in_process(&st(-66748, "AudioComponentInstantiate")));
    assert!(may_retry_in_process(&st(-66751, "AudioComponentInstantiate").context("loading")));
    assert!(!may_retry_in_process(&st(-66749, "AudioComponentInstantiate")), "the hosting process died");
    assert!(!may_retry_in_process(&st(-66748, "AudioUnitInitialize")), "a later stage");
    assert!(!may_retry_in_process(&st(-10875, "AudioUnitInitialize")));
    assert!(!may_retry_in_process(&anyhow::Error::new(LoadTimedOut(Duration::from_secs(20)))));
    assert!(!may_retry_in_process(&anyhow::anyhow!("AudioComponentInstantiate failed: OSStatus -66748")), "text is not a status");
}

#[test]
fn an_instance_at_another_sample_rate_is_refused() {
    let (_rack, mut ctl) = rack(256, 44_100.0);
    assert!(ctl.assign(0, dls(256), Swap::default()).is_err(), "a 48 kHz instance in a 44.1 kHz rack");
}
