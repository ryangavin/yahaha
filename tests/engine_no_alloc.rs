//! The engine thread must not allocate or free: a style preview (its whole run, and its
//! end) and a style change at the next bar line run there. A counting global allocator (in
//! this test binary only) checks `EngineLoop::step` through both.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use yahaha::engine::{Button, DynamicsSettings, Engine, PadCmd, Prepared, StyleControls, StyleSettings, Transpose, PAD_PPQ};
use yahaha::live::{self, Audition, Cmd, EngineLoop, FxConfig, FxKey, FxMode, Out, PadBank, Shared};
use yahaha::multipad::{file::parse, synthetic, MultiPadPlayer};
use yahaha::rt::{PacketSink, Target};
use yahaha::sff::Style;

struct Counting;

thread_local! {
    /// Count on this thread only: the test's own. The counts are per thread too, so the
    /// harness's own work on another test's thread (freeing that test's captured output
    /// after it ends) never lands in this test's window (#188).
    static COUNT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static ALLOCS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static FREES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn bump(n: &'static std::thread::LocalKey<std::cell::Cell<usize>>) {
    if COUNT.try_with(|c| c.get()).unwrap_or(false) {
        let _ = n.try_with(|n| n.set(n.get() + 1));
    }
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        bump(&ALLOCS);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        bump(&FREES);
        unsafe { System.dealloc(p, l) }
    }
}

#[global_allocator]
static A: Counting = Counting;

/// This thread's allocations and frees so far.
fn counts() -> (usize, usize) {
    (ALLOCS.with(|n| n.get()), FREES.with(|n| n.get()))
}

/// Counting on, for this thread, until the guard drops.
struct Counted;

impl Drop for Counted {
    fn drop(&mut self) {
        COUNT.with(|c| c.set(false));
    }
}

fn count_here() -> Counted {
    COUNT.with(|c| c.set(true));
    Counted
}

fn prep(name: &str) -> Option<Box<Prepared>> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2").join(name);
    p.exists().then(|| Box::new(Prepared::new(&Style::load(&p).unwrap())))
}

/// An Ending with written tempo changes (#243) plays them on the engine thread, and the
/// tempo comes back at the stop, without allocating or freeing.
#[test]
fn written_section_tempo_does_not_allocate() {
    use yahaha::sff::{SectionId, Timing, TimingChange};
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
    if !p.exists() {
        eprintln!("corpus missing; skipping");
        return;
    }
    let mut s = Style::load(&p).unwrap();
    let bar = s.ticks_per_bar();
    let name = SectionId::Ending(0).name();
    s.timing_changes = (0..8).map(|i| TimingChange { section: name.clone(), tick: bar + i * bar / 8, change: Timing::Tempo(800_000 + i * 50_000) }).collect();
    let a = Box::new(Prepared::new(&s));
    let bar_ns = (60e9 / a.bpm * (a.tpb as f64 / a.ppq as f64)) as u64;
    let _one = count_here();
    let shared = Arc::new(Shared::new(54));
    let mut ch = live::channels(Out::new(PacketSink::new(Target::Null), None));
    let mut l = EngineLoop::new(Engine::new(a), ch.io, shared.clone());
    l.step(1);
    let (allocs, frees) = counts();
    let mut now = 1_000;
    shared.chord.store(yahaha::parse_chord("C").unwrap().pack(1), Ordering::Release);
    l.step(now);
    ch.ui_tx.push(Cmd::Button(Button::Ending(0))).ok().unwrap();
    now += 1;
    l.step(now);
    run(&mut l, &mut now, 3 * bar_ns);
    ch.ui_tx.push(Cmd::Button(Button::TempoUp)).ok().unwrap();
    run(&mut l, &mut now, 8 * bar_ns);
    assert_eq!(counts().0 - allocs, 0, "allocations on the engine thread");
    assert_eq!(counts().1 - frees, 0, "frees on the engine thread");
    let snaps: Vec<_> = std::iter::from_fn(|| ch.snap_rx.pop().ok()).collect();
    assert!(snaps.iter().any(|s| s.bpm < 70.0), "the written ritardando played");
}

#[test]
fn preview_and_next_bar_style_change_do_not_allocate() {
    let (Some(a), Some(b), Some(c), Some(d)) = (prep("SlowWalker.T552.sty"), prep("TickingAway.T162.sty"), prep("CoolRevibed.T552.sty"), prep("SlowWalker.T552.sty")) else {
        eprintln!("corpus missing; skipping");
        return;
    };
    let _one = count_here();
    let bar = (60e9 / a.bpm * (a.tpb as f64 / a.ppq as f64)) as u64;
    let preview_bar = (60e9 / c.bpm * (c.tpb as f64 / c.ppq as f64)) as u64;
    // Taps in a bar of the style playing at the end (`d`, SlowWalker again).
    let beats = d.tpb / d.ppq;
    let shared = Arc::new(Shared::new(54));
    let mut ch = live::channels(Out::new(PacketSink::new(Target::Null), None));
    let mut l = EngineLoop::new(Engine::new(a), ch.io, shared.clone());
    let chords = ["C", "Am", "F", "G7"].map(|s| yahaha::parse_chord(s).unwrap());
    let preview = Box::new(Audition { engine: Engine::new(c), id: 7, chords });
    let chord = yahaha::parse_chord("F").unwrap();
    // Warm up: the first step sizes nothing lazily later on.
    l.step(1);

    let (allocs, frees) = counts();
    let mut now = 1_000;
    ch.audition_tx.push(preview).ok().unwrap();
    let end = now + 4 * preview_bar + 50_000_000;
    while now < end {
        l.step(now);
        now = l.next_deadline().unwrap_or(now + 5_000_000).max(now + 1);
    }
    // The band: a Sync Start chord, then a style change mid-bar, played past the bar line.
    shared.chord.store(chord.pack(1), Ordering::Release);
    l.step(now);
    let t0 = now;
    while now < t0 + bar / 2 {
        now = l.next_deadline().unwrap_or(now + 5_000_000).max(now + 1);
        l.step(now);
    }
    ch.style_tx.push(b).ok().unwrap();
    // What a Registration recall sends the engine: tempo, Style part levels and mutes.
    ch.ui_tx.push(Cmd::Button(Button::SetTempo(96))).ok().unwrap();
    ch.ui_tx.push(Cmd::StyleVolume(3, 64)).ok().unwrap();
    ch.ui_tx.push(Cmd::Button(Button::TogglePart(5))).ok().unwrap();
    // The Style parts' own sends (#268): set, and handed back to the style.
    ch.ui_tx.push(Cmd::StyleSend(2, 1, 90)).ok().unwrap();
    ch.ui_tx.push(Cmd::StyleSend(6, 2, 50)).ok().unwrap();
    ch.ui_tx.push(Cmd::ResetStyleSends(1 << 6)).ok().unwrap();
    let controls = StyleControls { acmp: None, main: Some(1), intro: None, sync_start: None, sync_stop: Some(true), stop_acmp: Some(true), stop_acmp_mode: None, parts: Some(0b1011_1111), volumes: Some([90, 80, 100, 64, 100, 100, 100, 70]), player_set: Some(0b1000_0001), retrigger: Some(true), sends: Some([[255, 60, 255], [255; 3], [255; 3], [100, 255, 40], [255; 3], [255; 3], [255; 3], [0, 0, 0]]) };
    // Part 4 (moved above) goes back to the style: the player_set mask leaves it out.
    ch.ui_tx.push(Cmd::StyleControls(controls)).ok().unwrap();
    while now < t0 + 2 * bar {
        now = l.next_deadline().unwrap_or(now + 5_000_000).max(now + 1);
        l.step(now);
    }
    // TAP TEMPO while the band plays: a Section Reset by default (the Genos's); with the
    // setting off it sets the tempo. Section Reset is also its own button.
    ch.ui_tx.push(Cmd::Button(Button::TapTempo)).ok().unwrap();
    l.step(now + 1);
    ch.ui_tx.push(Cmd::StyleSettings(StyleSettings { section_reset: false, ..StyleSettings::default() })).ok().unwrap();
    ch.ui_tx.push(Cmd::Button(Button::TapTempo)).ok().unwrap();
    l.step(now + 2);
    now += 400_000_000;
    l.step(now);
    ch.ui_tx.push(Cmd::Button(Button::TapTempo)).ok().unwrap();
    l.step(now + 1);
    ch.ui_tx.push(Cmd::Button(Button::SectionReset)).ok().unwrap();
    l.step(now + 2);
    // An Ending queued with a style change waiting for it, then a Section Reset (#174).
    ch.ui_tx.push(Cmd::Button(Button::Ending(0))).ok().unwrap();
    ch.style_tx.push(d).ok().unwrap();
    l.step(now + 3);
    ch.ui_tx.push(Cmd::Button(Button::SectionReset)).ok().unwrap();
    l.step(now + 4);
    now += 4;
    // Controllers: a bend range, a part switched on under a held pedal, Fill Up, KeysOff
    // and Panic (the pedal reset).
    shared.controllers.set_bend_range(0, 9);
    shared.controllers.toggle_switch(yahaha::controllers::SUSTAIN);
    shared.parts.toggle(1);
    l.step(now + 1);
    // An OTS recall's voice settings (#238), then a voice change putting them back.
    let mut ots = yahaha::sff::Ots::default();
    ots.parts[0].tone = [Some(80); yahaha::parts::TONE];
    ots.parts[0].xg.set(0x08, 0x05, 0);
    shared.parts.apply_ots(&ots, 1);
    l.step(now + 1);
    shared.parts.set_program(0, 3);
    l.step(now + 1);
    ch.ui_tx.push(Cmd::Button(Button::FillUp)).ok().unwrap();
    ch.ui_tx.push(Cmd::KeysOff).ok().unwrap();
    l.step(now + 1);
    ch.ui_tx.push(Cmd::Button(Button::StartStop)).ok().unwrap();
    ch.ui_tx.push(Cmd::Panic).ok().unwrap();
    l.step(now + 1);
    // Stopped: a bar of taps at 120 BPM counts the band in, and it starts a beat after
    // the last tap (#195). The earlier taps are long forgotten.
    now += 20_000_000_000;
    for i in 0..beats {
        if i > 0 {
            let next = now + 500_000_000;
            while now < next {
                now = l.next_deadline().unwrap_or(next).clamp(now + 1, next);
                l.step(now);
            }
        }
        ch.ui_tx.push(Cmd::Button(Button::TapTempo)).ok().unwrap();
        l.step(now);
    }
    let end = now + 1_000_000_000;
    while now < end {
        now = l.next_deadline().unwrap_or(now + 5_000_000).max(now + 1);
        l.step(now);
    }
    assert_eq!(counts().0 - allocs, 0, "allocations on the engine thread");
    assert_eq!(counts().1 - frees, 0, "frees on the engine thread");
    let snaps: Vec<_> = std::iter::from_fn(|| ch.snap_rx.pop().ok()).collect();
    assert!(snaps.iter().any(|s| s.running && (s.bpm - 120.0).abs() < 1e-6), "the taps started the band");
    assert!(snaps.iter().any(|s| s.audition.is_some_and(|a| a.bar == 4)), "the preview played its 4 bars");
    assert!(snaps.iter().any(|s| s.running && s.style_pending), "the style change waited for the bar line");
    assert!(snaps.iter().any(|s| s.running && (s.bpm - 96.0).abs() < 1e-9), "the recalled tempo took");
    assert!(snaps.iter().any(|s| s.running && (s.bpm - 150.0).abs() < 1e-6), "the tapped tempo took");
    // The preview and the old style came back to be freed off it.
    assert!(ch.old_audition_rx.pop().is_ok());
    assert!(ch.old_rx.pop().is_ok());
}

/// Keyboard Harmony's Echo category, the arpeggio and Strum run on the engine thread
/// (`live::KbdFx`): keys, type switches, the band starting, a style change to another
/// resolution and PANIC, all without allocating.
#[test]
fn harmony_and_arpeggio_do_not_allocate() {
    let (Some(a), Some(b)) = (prep("SlowWalker.T552.sty"), prep("TickingAway.T162.sty")) else {
        eprintln!("corpus missing; skipping");
        return;
    };
    let _one = count_here();
    let shared = Arc::new(Shared::new(54));
    for p in 0..3 {
        shared.parts.on[p].store(true, Ordering::Relaxed);
    }
    let (synth, mut heard) = rtrb::RingBuffer::new(1 << 16);
    let mut ch = live::channels(Out::new(PacketSink::new(Target::Null), Some(synth)));
    let mut l = EngineLoop::new(Engine::new(a), ch.io, shared.clone());
    let arp = FxConfig { on: true, mode: FxMode::Arpeggio, hold: true, pattern: 3, ..FxConfig::default() }.pack();
    let mut trill = FxConfig { on: true, ..FxConfig::default() };
    trill.harmony.ty = yahaha::harmony::HarmonyType::Trill;
    let trill = trill.pack();
    let chord = yahaha::parse_chord("C").unwrap();
    l.step(1);

    let (allocs, frees) = counts();
    let mut now = 1_000;
    let mut ons = [0usize; 4];
    let mut play = |l: &mut EngineLoop, now: &mut u64, ns: u64| {
        let end = *now + ns;
        while *now < end {
            l.step(*now);
            *now = l.next_deadline().unwrap_or(*now + 5_000_000).clamp(*now + 1, end);
            while let Ok(m) = heard.pop() {
                if m[0] & 0xF0 == 0x90 && m[2] > 0 && m[0] & 0x0F < 4 {
                    ons[(m[0] & 3) as usize] += 1;
                }
            }
        }
    };
    let press = |tx: &mut rtrb::Producer<FxKey>, k: u8, on: bool| {
        let (w, b) = ((k / 64) as usize, 1u64 << (k % 64));
        if on {
            shared.fx_held[w].fetch_or(b, Ordering::Release);
            tx.push(FxKey::On { key: k, vel: 100 }).ok().unwrap();
        } else {
            shared.fx_held[w].fetch_and(!b, Ordering::Release);
            tx.push(FxKey::Off { key: k }).ok().unwrap();
        }
    };
    // The arpeggio with the band stopped, then started, then a style with another PPQ.
    shared.kbd_fx.store(arp, Ordering::Release);
    for k in [60, 64, 67] {
        press(&mut ch.fx_tx, k, true);
    }
    play(&mut l, &mut now, 1_000_000_000);
    shared.chord.store(chord.pack(1), Ordering::Release);
    play(&mut l, &mut now, 1_000_000_000);
    ch.style_tx.push(b).ok().unwrap();
    play(&mut l, &mut now, 4_000_000_000);
    for k in [60, 64, 67] {
        press(&mut ch.fx_tx, k, false);
    }
    play(&mut l, &mut now, 500_000_000);
    // Trill, then Strum notes, then PANIC.
    shared.kbd_fx.store(trill, Ordering::Release);
    press(&mut ch.fx_tx, 72, true);
    press(&mut ch.fx_tx, 76, true);
    play(&mut l, &mut now, 1_000_000_000);
    press(&mut ch.fx_tx, 72, false);
    press(&mut ch.fx_tx, 76, false);
    shared.fx_held[1].fetch_or(1 << (72 - 64), Ordering::Release);
    ch.fx_tx.push(FxKey::Strum { melody: 72, ch: 0, note: 67, vel: 90, delay_ms: 15 }).ok().unwrap();
    play(&mut l, &mut now, 100_000_000);
    ch.fx_tx.push(FxKey::StrumOff { melody: 72 }).ok().unwrap();
    ch.ui_tx.push(Cmd::Panic).ok().unwrap();
    play(&mut l, &mut now, 100_000_000);
    assert_eq!(counts().0 - allocs, 0, "allocations on the engine thread");
    assert_eq!(counts().1 - frees, 0, "frees on the engine thread");
    assert!(ch.old_rx.pop().is_ok(), "the style change happened");
    assert!(ons.iter().sum::<usize>() > 50, "the arpeggio and the trill played: {ons:?}");
}

/// Chord settling (engine/settle.rs) under the chord-settle window: a Sync Start chord,
/// rolled chords, a chord and a transpose in one wake, Stop Accompaniment chords while
/// stopped. Notes held back and started at the settle, all on the engine thread, with
/// Chord Match pads playing (their notes wait for the settle too).
#[test]
fn chord_settling_does_not_allocate() {
    let Some(a) = prep("SlowWalker.T552.sty") else {
        eprintln!("corpus missing; skipping");
        return;
    };
    let _one = count_here();
    let bar = (60e9 / a.bpm * (a.tpb as f64 / a.ppq as f64)) as u64;
    let shared = Arc::new(Shared::new(54));
    let mut ch = live::channels(Out::new(PacketSink::new(Target::Null), None));
    let mut l = EngineLoop::new(Engine::new(a), ch.io, shared.clone());
    let pads = Box::new(MultiPadPlayer::new(&parse(&synthetic::demo_bank()).unwrap(), PAD_PPQ));
    l.step(1);

    let (allocs, frees) = counts();
    let mut now = 1_000;
    ch.ui_tx.push(Cmd::ChordSettle(10)).ok().unwrap();
    ch.pad_tx.push(PadBank { player: Some(pads), tag: 1 }).ok().unwrap();
    // Bass Riff (Repeat, Chord Match) and the Shaker Loop play throughout.
    ch.ui_tx.push(Cmd::MultiPad(PadCmd::Trigger(2))).ok().unwrap();
    ch.ui_tx.push(Cmd::MultiPad(PadCmd::Trigger(0))).ok().unwrap();
    l.step(now);
    let chord = |s: &str| yahaha::parse_chord(s).unwrap();
    let mut generation = 1;
    let mut play = |c: &str, at: u64, l: &mut EngineLoop| {
        shared.chord.store(chord(c).pack(generation), Ordering::Release);
        generation += 1;
        l.step(at);
    };
    // Sync Start, then a rolled chord every half bar, 3 ms apart, some with a transpose.
    play("C", now, &mut l);
    let run = |l: &mut EngineLoop, now: &mut u64, to: u64| {
        while *now < to {
            *now = l.next_deadline().unwrap_or(*now + 5_000_000).max(*now + 1).min(to);
            l.step(*now);
        }
    };
    for (i, (c1, c2)) in [("F", "F7"), ("G", "G7"), ("Am", "Am7"), ("D", "Dm")].into_iter().enumerate() {
        run(&mut l, &mut now, (i as u64 + 1) * bar / 2 - 2_000_000);
        play(c1, now, &mut l);
        if i % 2 == 0 {
            ch.ui_tx.push(Cmd::Transpose(Transpose::new(i as i8 - 1, 0))).ok().unwrap();
        }
        let to = now + 3_000_000;
        run(&mut l, &mut now, to);
        play(c2, now, &mut l);
    }
    run(&mut l, &mut now, 3 * bar);
    // Stopped, with Stop Accompaniment: a rolled chord settles too.
    ch.ui_tx.push(Cmd::Button(Button::StartStop)).ok().unwrap();
    ch.ui_tx.push(Cmd::Button(Button::SyncStart)).ok().unwrap();
    ch.ui_tx.push(Cmd::Button(Button::StopAcmp)).ok().unwrap();
    l.step(now + 1);
    now += 1;
    play("E", now, &mut l);
    let to = now + 3_000_000;
        run(&mut l, &mut now, to);
    play("E7", now, &mut l);
    let to = now + 50_000_000;
        run(&mut l, &mut now, to);
    assert_eq!(counts().0 - allocs, 0, "allocations on the engine thread");
    assert_eq!(counts().1 - frees, 0, "frees on the engine thread");
    let snaps: Vec<_> = std::iter::from_fn(|| ch.snap_rx.pop().ok()).collect();
    assert!(snaps.iter().any(|s| s.running && s.played.is_some_and(|c| c.name() == "Dm")), "the band followed the rolls");
}

/// Run the engine loop as its thread does, waking at each deadline, until `until`.
fn run(l: &mut EngineLoop, now: &mut u64, until: u64) {
    while *now < until {
        *now = l.next_deadline().unwrap_or(*now + 5_000_000).max(*now + 1);
        l.step(*now);
    }
}

/// The Chord Looper (record, loop, a memory at the bar line), the metronome's clicks,
/// solos, Style Track Mute and Style Dynamics (Touch, an Accent fill) run on the engine
/// thread too.
#[test]
fn looper_metronome_and_solo_do_not_allocate() {
    let _one = count_here();
    use yahaha::engine::LoopState;
    use yahaha::looper::{ChordSeq, LoopEvent};
    let Some(a) = prep("SlowWalker.T552.sty") else {
        eprintln!("corpus missing; skipping");
        return;
    };
    let bar = (60e9 / a.bpm * (a.tpb as f64 / a.ppq as f64)) as u64;
    let shared = Arc::new(Shared::new(54));
    let (synth_tx, mut synth_rx) = rtrb::RingBuffer::<[u8; 3]>::new(1 << 16);
    let mut ch = live::channels(Out::new(PacketSink::new(Target::Null), Some(synth_tx)));
    let mut l = EngineLoop::new(Engine::new(a), ch.io, shared.clone());
    let chord = |s: &str| yahaha::parse_chord(s).unwrap();
    let memory = ChordSeq::from_events(
        2,
        &[LoopEvent { bar: 0, at: 0, chord: chord("E") }, LoopEvent { bar: 1, at: 960, chord: chord("A") }],
    );
    l.step(1);

    let (allocs, frees) = counts();
    let mut now = 1_000;
    ch.ui_tx.push(Cmd::Metronome { on: true, bell: true }).ok().unwrap();
    ch.ui_tx.push(Cmd::Dynamics(DynamicsSettings { touch: true, accent: true, ..DynamicsSettings::default() })).ok().unwrap();
    ch.ui_tx.push(Cmd::Looper(true)).ok().unwrap();
    l.step(now);
    // Stopped, REC arms Sync Start: this chord starts the band and the recording.
    shared.chord.store(chord("C").pack(1), Ordering::Release);
    l.step(now);
    let t0 = now;
    run(&mut l, &mut now, t0 + bar + bar / 2);
    shared.chord.store(chord("F").pack(2), Ordering::Release);
    run(&mut l, &mut now, t0 + 2 * bar - bar / 4);
    ch.input_tx.push(Cmd::Strike(40)).ok().unwrap();
    ch.input_tx.push(Cmd::DynamicsLevel(90)).ok().unwrap();
    ch.input_tx.push(Cmd::Strike(40)).ok().unwrap();
    run(&mut l, &mut now, t0 + 2 * bar - bar / 8);
    ch.ui_tx.push(Cmd::Looper(false)).ok().unwrap();
    ch.ui_tx.push(Cmd::StyleSolo(Some(2))).ok().unwrap();
    run(&mut l, &mut now, t0 + 4 * bar + bar / 2);
    ch.looper_tx.push(memory).ok().unwrap();
    ch.ui_tx.push(Cmd::StyleSolo(None)).ok().unwrap();
    ch.ui_tx.push(Cmd::StyleParts(0b0000_1010)).ok().unwrap();
    run(&mut l, &mut now, t0 + 5 * bar + bar / 3);
    ch.input_tx.push(Cmd::Strike(127)).ok().unwrap();
    run(&mut l, &mut now, t0 + 7 * bar);
    ch.ui_tx.push(Cmd::Button(Button::StartStop)).ok().unwrap();
    l.step(now + 1);
    run(&mut l, &mut now, t0 + 9 * bar);
    assert_eq!(counts().0 - allocs, 0, "allocations on the engine thread");
    assert_eq!(counts().1 - frees, 0, "frees on the engine thread");

    let snaps: Vec<_> = std::iter::from_fn(|| ch.snap_rx.pop().ok()).collect();
    assert!(snaps.iter().any(|s| s.looper.state == LoopState::Recording));
    assert!(snaps.iter().any(|s| s.dynamics == 50), "Touch set the level");
    assert!(snaps.iter().any(|s| matches!(s.cur, Some(yahaha::sff::SectionId::Fill(_)))), "the Accent fill played");
    assert!(snaps.iter().any(|s| s.looper.state == LoopState::Looping && s.style_solo == Some(2)));
    assert!(snaps.iter().any(|s| s.looper.state == LoopState::Looping && s.played == Some(chord("A"))), "the memory took over");
    let rec = ch.recorded_rx.pop().expect("the recording came back");
    assert_eq!(rec.bars(), 2);
    let clicks = std::iter::from_fn(|| synth_rx.pop().ok()).filter(|m| m[0] == yahaha::click::CLICK).count();
    assert!(clicks >= 9 * 4 - 2, "{clicks} clicks");
}

/// Fill Up / Down / Self, Half Bar Fill In, the Stop Accompaniment modes (Fixed voices and
/// back), Change Behavior rules, an OTS Sync Start and a Reset-tempo style change at the bar
/// line all run on the engine thread without allocating or freeing.
#[test]
fn fills_stop_acmp_and_change_rules_do_not_allocate() {
    use yahaha::engine::{ChangeRule, ChangeRules, StopAcmp};
    let _one = count_here();
    let (Some(a), Some(b)) = (prep("SlowWalker.T552.sty"), prep("TickingAway.T162.sty")) else {
        eprintln!("corpus missing; skipping");
        return;
    };
    let bar = (60e9 / a.bpm * (a.tpb as f64 / a.ppq as f64)) as u64;
    let shared = Arc::new(Shared::new(54));
    let mut ch = live::channels(Out::new(PacketSink::new(Target::Null), None));
    let mut l = EngineLoop::new(Engine::new(a), ch.io, shared.clone());
    l.step(1);
    let (allocs, frees) = counts();
    let mut now = 1_000;
    let mut cmd = |c: Cmd, now: &mut u64, l: &mut EngineLoop| {
        ch.ui_tx.push(c).ok().unwrap();
        *now += 1;
        l.step(*now);
    };
    let run_to = |to: u64, now: &mut u64, l: &mut EngineLoop| {
        while *now < to {
            *now = l.next_deadline().unwrap_or(*now + 5_000_000).max(*now + 1);
            l.step(*now);
        }
    };
    let chord = |s: &str, g: u16| yahaha::parse_chord(s).unwrap().pack(g);
    // Stopped, Sync Start off: Stop Accompaniment in Fixed, then Style, then Off.
    cmd(Cmd::Button(Button::SyncStart), &mut now, &mut l);
    cmd(Cmd::Button(Button::SetStopAcmp(StopAcmp::Fixed)), &mut now, &mut l);
    for (i, c) in ["C", "F", "G7"].iter().enumerate() {
        shared.chord.store(chord(c, 1 + i as u16), Ordering::Release);
        now += 1_000_000;
        l.step(now);
    }
    cmd(Cmd::Button(Button::SetStopAcmp(StopAcmp::Style)), &mut now, &mut l);
    cmd(Cmd::Button(Button::StopAcmp), &mut now, &mut l);
    cmd(Cmd::ChangeRules(ChangeRules { tempo: ChangeRule::Reset, parts: ChangeRule::Reset, section: Some(1) }), &mut now, &mut l);
    cmd(Cmd::Button(Button::SetHalfBarFill(true)), &mut now, &mut l);
    // An OTS recall arms Sync Start; the next chord starts the band.
    cmd(Cmd::SyncStartOn, &mut now, &mut l);
    shared.chord.store(chord("C", 9), Ordering::Release);
    now += 1;
    l.step(now);
    let t0 = now;
    // Fill Up on beat 1 (a half-bar fill), Fill Down mid-bar, Fill Self, then a style change.
    run_to(t0 + bar + 10_000_000, &mut now, &mut l);
    cmd(Cmd::Button(Button::FillUp), &mut now, &mut l);
    run_to(t0 + 2 * bar + bar / 3, &mut now, &mut l);
    cmd(Cmd::Button(Button::FillDown), &mut now, &mut l);
    run_to(t0 + 3 * bar + bar / 3, &mut now, &mut l);
    // An Ending queued with a style change waiting for it, then Fill Self in the Ending's
    // place: the style no longer waits for the Ending (#175).
    cmd(Cmd::Button(Button::Ending(0)), &mut now, &mut l);
    ch.style_tx.push(b).ok().unwrap();
    now += 1;
    l.step(now);
    cmd(Cmd::Button(Button::FillSelf), &mut now, &mut l);
    run_to(t0 + 5 * bar, &mut now, &mut l);
    // A TEMPO button held (repeating on the engine's own deadlines), let go, then − and +
    // together (#263).
    // [ACMP] off (the chord parts stop), a key, and on again (#266).
    cmd(Cmd::Button(Button::SetAcmp(false)), &mut now, &mut l);
    cmd(Cmd::AnyKey, &mut now, &mut l);
    run_to(now + 500_000_000, &mut now, &mut l);
    cmd(Cmd::Button(Button::Acmp), &mut now, &mut l);
    cmd(Cmd::TempoHold(1), &mut now, &mut l);
    run_to(now + 1_500_000_000, &mut now, &mut l);
    cmd(Cmd::TempoHold(0), &mut now, &mut l);
    cmd(Cmd::Button(Button::TempoReset), &mut now, &mut l);
    cmd(Cmd::Button(Button::StartStop), &mut now, &mut l);
    assert_eq!(counts().0 - allocs, 0, "allocations on the engine thread");
    assert_eq!(counts().1 - frees, 0, "frees on the engine thread");
    let snaps: Vec<_> = std::iter::from_fn(|| ch.snap_rx.pop().ok()).collect();
    assert!(snaps.iter().any(|s| s.half_bar_fill), "Half Bar Fill on");
    assert!(snaps.iter().any(|s| s.cur == Some(yahaha::sff::SectionId::Fill(1))), "Fill Up played B's fill");
    assert!(ch.old_rx.pop().is_ok(), "the old style came back");
}

/// Fills back to back (#229): Main A tapped during its fill, bar after bar (Fill Self too),
/// and a style chosen during a fill that takes over as the next fill starts, all without
/// allocating or freeing on the engine thread.
#[test]
fn back_to_back_fills_do_not_allocate() {
    use yahaha::sff::SectionId::{Fill, Main};
    let _one = count_here();
    let (Some(a), Some(b)) = (prep("SlowWalker.T552.sty"), prep("TickingAway.T162.sty")) else {
        eprintln!("corpus missing; skipping");
        return;
    };
    let bar = (60e9 / a.bpm * (a.tpb as f64 / a.ppq as f64)) as u64;
    let shared = Arc::new(Shared::new(54));
    let mut ch = live::channels(Out::new(PacketSink::new(Target::Null), None));
    let mut l = EngineLoop::new(Engine::new(a), ch.io, shared.clone());
    l.step(1);
    let (allocs, frees) = counts();
    let mut now = 1_000;
    // Sync Start is armed: the chord starts the band on Main A.
    shared.chord.store(yahaha::parse_chord("C").unwrap().pack(1), Ordering::Release);
    l.step(now);
    let t0 = now;
    // Bar 2, beat 2: Main A's fill from the middle of the bar; then a tap late in each fill.
    let taps = [(13, Button::Main(0)), (18, Button::Main(0)), (28, Button::FillSelf), (38, Button::Main(0))];
    for (tenths, b) in taps {
        run(&mut l, &mut now, t0 + tenths * bar / 10);
        ch.ui_tx.push(Cmd::Button(b)).ok().unwrap();
        now += 1;
        l.step(now);
    }
    // A style chosen in the last fill, and one more tap: the next fill plays in it.
    run(&mut l, &mut now, t0 + 45 * bar / 10);
    ch.style_tx.push(b).ok().unwrap();
    ch.ui_tx.push(Cmd::Button(Button::Main(0))).ok().unwrap();
    now += 1;
    l.step(now);
    run(&mut l, &mut now, t0 + 8 * bar);
    ch.ui_tx.push(Cmd::Button(Button::StartStop)).ok().unwrap();
    now += 1;
    l.step(now);
    assert_eq!(counts().0 - allocs, 0, "allocations on the engine thread");
    assert_eq!(counts().1 - frees, 0, "frees on the engine thread");
    let snaps: Vec<_> = std::iter::from_fn(|| ch.snap_rx.pop().ok()).collect();
    let first = snaps.iter().position(|s| s.cur == Some(Fill(0))).expect("the fill played");
    let last = snaps.iter().rposition(|s| s.cur == Some(Fill(0))).unwrap();
    assert!(snaps[first..=last].iter().all(|s| s.cur == Some(Fill(0))), "no Main between the fills");
    assert!(snaps[last..].iter().any(|s| s.cur == Some(Main(0))), "the Main came back");
    assert!(ch.old_rx.pop().is_ok(), "the new style took over");
}

/// Fills pressed just after a beat line (#265): inside the grace window they start at once
/// from that point of their pattern; one tapped just after a fill ended too. No allocating
/// or freeing on the engine thread.
#[test]
fn late_fill_presses_do_not_allocate() {
    use yahaha::sff::SectionId::Fill;
    let _one = count_here();
    let Some(a) = prep("SlowWalker.T552.sty") else {
        eprintln!("corpus missing; skipping");
        return;
    };
    let bar = (60e9 / a.bpm * (a.tpb as f64 / a.ppq as f64)) as u64;
    let shared = Arc::new(Shared::new(54));
    let mut ch = live::channels(Out::new(PacketSink::new(Target::Null), None));
    let mut l = EngineLoop::new(Engine::new(a), ch.io, shared.clone());
    l.step(1);
    let (allocs, frees) = counts();
    let mut now = 1_000;
    shared.chord.store(yahaha::parse_chord("C").unwrap().pack(1), Ordering::Release);
    l.step(now);
    let t0 = now;
    // 20 ms after beat 2 of bar 2, then 20 ms after the fill ended (bar 3's line).
    for at in [t0 + bar + bar / 4 + 20_000_000, t0 + 2 * bar + 20_000_000] {
        // Up to the press exactly (`run` may step past it).
        while let Some(d) = l.next_deadline().filter(|&d| d < at) {
            now = d.max(now + 1);
            l.step(now);
        }
        now = at;
        ch.ui_tx.push(Cmd::Button(Button::Main(0))).ok().unwrap();
        l.step(now);
    }
    run(&mut l, &mut now, t0 + 4 * bar);
    ch.ui_tx.push(Cmd::Button(Button::StartStop)).ok().unwrap();
    now += 1;
    l.step(now);
    assert_eq!(counts().0 - allocs, 0, "allocations on the engine thread");
    assert_eq!(counts().1 - frees, 0, "frees on the engine thread");
    let snaps: Vec<_> = std::iter::from_fn(|| ch.snap_rx.pop().ok()).collect();
    assert!(snaps.iter().filter(|s| s.cur == Some(Fill(0))).count() >= 2, "the fills played");
}

/// The built-in synth's drum setup (#239): a style's XG Drum Setup SysEx turned into drum
/// messages for the synth, and its drum notes, on the engine thread.
#[test]
fn drum_setup_on_the_way_to_the_synth_does_not_allocate() {
    // Set up under the lock, as the other tests do: a test that just finished may still be
    // dropping its engine on a counting thread.
    let _one = count_here();
    let Some(p) = prep("AustinCityBlues.S930.STY") else {
        eprintln!("corpus missing; skipping");
        return;
    };
    let (synth, mut heard) = rtrb::RingBuffer::new(1 << 16);
    let mut out = Out::new(PacketSink::new(Target::Null), Some(synth));
    let init = &p.setups[0].init;
    let mut pass = |out: &mut Out| {
        for i in 0..init.len() {
            out.push(init.get(i));
        }
        for ch in [8u8, 9] {
            for note in 0..128u8 {
                out.push(&[0x90 | ch, note, 100]);
                out.push(&[0x80 | ch, note, 0]);
            }
            out.push(&[0xC0 | ch, 0]);
        }
        out.flush();
        while heard.pop().is_ok() {}
    };
    // Warm up (the port side's first packets), then count a second pass.
    pass(&mut out);
    let (allocs, frees) = counts();
    pass(&mut out);
    assert_eq!(counts().0 - allocs, 0, "allocations on the engine thread");
    assert_eq!(counts().1 - frees, 0, "frees on the engine thread");
}
