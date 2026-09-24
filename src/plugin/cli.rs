//! `yahaha plugin-test`: load an Audio Unit instrument through [`PluginHost`], measure it, and
//! play a phrase through it in a [`super::PluginRack`] with the backing on the built-in
//! SoundFont synth in the same audio callback.
//!
//! - `--list`: the scan (cached; `--rescan` ignores the cache).
//! - `--bench`: offline, no audio device: load time, state round trip, render CPU per block
//!   at 64/128/256 frames, and preload-then-swap latency against a real-time paced render
//!   thread.
//! - default: play the phrase (the test channel through the rack, other channels on
//!   rustysynth). `--swap-to <name>` preloads a second instrument and swaps it in at bar 3.

use anyhow::{Context, Result, anyhow, bail};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rtrb::RingBuffer;
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};
use std::time::{Duration, Instant};

use super::{LoadConfig, LoadMode, LoadProgress, PluginHost, PluginInfo, PluginInstance, RackEvent, Swap, editor, rack};
use crate::synth::soft_clip;

const USAGE: &str = "usage: yahaha plugin-test [name | \"aumu subt manu\"] [--list] [--rescan] [--bench] [--swap-to name] [--oop] [--gui]
                           [--channel 1-16] [--sf2 file | --no-sf2] [--loops N] [--seconds N] [--audio-out N] [--timeout S]";

type Msg = [u8; 3];

/// The test phrase: (beat, message).
fn phrase(plugin_ch: u8, backing_ch: u8, fade: bool) -> Vec<(f64, Msg)> {
    let mut ev: Vec<(f64, Msg)> = Vec::new();
    let p = plugin_ch;
    let b = backing_ch;
    // Mixer: CC7 on each channel is the part's volume.
    ev.push((0.0, [0xB0 | p, 7, 100]));
    ev.push((0.0, [0xB0 | b, 7, 70]));
    ev.push((0.0, [0xC0 | b, 48, 0])); // Strings on the backing channel
    ev.push((0.0, [0xB0 | 9, 7, 80]));
    // Backing: C Am F G, one bar each, then C.
    let chords: [[u8; 3]; 5] = [[48, 52, 55], [45, 48, 52], [41, 45, 48], [43, 47, 50], [48, 52, 55]];
    for (bar, ch) in chords.iter().enumerate() {
        let t = bar as f64 * 4.0;
        let len = if bar == 4 { 4.0 } else { 3.9 };
        for &n in ch {
            ev.push((t, [0x90 | b, n, 70]));
            ev.push((t + len, [0x80 | b, n, 0]));
        }
        if bar < 4 {
            for beat in 0..4 {
                let tb = t + beat as f64;
                if beat % 2 == 0 {
                    ev.push((tb, [0x99, 36, 90]));
                    ev.push((tb + 0.1, [0x89, 36, 0]));
                }
                ev.push((tb, [0x99, 42, 60]));
                ev.push((tb + 0.1, [0x89, 42, 0]));
            }
        }
    }
    let melody: [(f64, u8, f64); 14] = [
        (0.0, 64, 1.0), (1.0, 67, 1.0), (2.0, 72, 1.5), (3.5, 71, 0.5),
        (4.0, 69, 1.0), (5.0, 72, 1.0), (6.0, 76, 2.0),
        (8.0, 77, 1.0), (9.0, 76, 1.0), (10.0, 72, 2.0),
        (12.0, 74, 1.0), (13.0, 71, 1.0), (14.0, 67, 2.0),
        (16.0, 72, 4.0),
    ];
    for (t, n, len) in melody {
        ev.push((t, [0x90 | p, n, 100]));
        ev.push((t + len * 0.95, [0x80 | p, n, 0]));
    }
    if fade {
        // The plugin part's fader down over the last bar: the rack applies it.
        for i in 0..=16 {
            ev.push((16.0 + i as f64 * 0.25, [0xB0 | p, 7, 100 - (i * 6) as u8]));
        }
    }
    ev.sort_by(|a, b| a.0.total_cmp(&b.0));
    ev
}

fn list(host: &PluginHost, rescan: bool) -> Result<()> {
    let t0 = Instant::now();
    let all = if rescan { host.rescan()? } else { host.scan()? };
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    let t1 = Instant::now();
    host.scan()?;
    let cached_ms = t1.elapsed().as_secs_f64() * 1000.0;
    println!(
        "{} instrument Audio Units (scan {ms:.1} ms, cached {cached_ms:.2} ms; cache {}):",
        all.len(),
        host.cache_path().map_or("off".into(), |p| p.display().to_string())
    );
    for p in &all {
        let mut tags = vec![match p.format {
            super::PluginFormat::Au2 => "AUv2",
            super::PluginFormat::Au3 => "AUv3",
        }];
        if p.requires_async {
            tags.push("async");
        }
        let last = match &p.last_load {
            Some(r) => match (&r.ms, &r.error) {
                (_, Some(e)) => format!("  last load FAILED: {e}"),
                (Some(ms), None) => format!("  last load {ms:.0} ms"),
                _ => String::new(),
            },
            None => String::new(),
        };
        println!("  {}  {}  {}  [{}]{last}", p.id, p.full_name(), p.version_string(), tags.join(", "));
    }
    Ok(())
}

struct Opts {
    name: Option<String>,
    swap_to: Option<String>,
    gui: bool,
    bench: bool,
    oop: bool,
    channel: u8,
    sf2: Option<PathBuf>,
    no_sf2: bool,
    loops: u32,
    seconds: Option<f64>,
    audio_out: Option<u8>,
    timeout: Duration,
}

pub fn run(args: &[String]) -> Result<()> {
    let mut o = Opts {
        name: None,
        swap_to: None,
        gui: false,
        bench: false,
        oop: false,
        channel: 1,
        sf2: None,
        no_sf2: false,
        loops: 1,
        seconds: None,
        audio_out: None,
        timeout: Duration::from_secs(20),
    };
    let mut do_list = false;
    let mut rescan = false;
    let mut i = 0;
    let num = |i: usize, what: &str| -> Result<f64> { args.get(i).and_then(|s| s.parse().ok()).ok_or_else(|| anyhow!("{what} takes a number")) };
    while i < args.len() {
        match args[i].as_str() {
            "--list" => do_list = true,
            "--rescan" => rescan = true,
            "--gui" => o.gui = true,
            "--bench" => o.bench = true,
            "--oop" => o.oop = true,
            "--no-sf2" => o.no_sf2 = true,
            "--sf2" => {
                i += 1;
                o.sf2 = args.get(i).map(PathBuf::from);
            }
            "--swap-to" => {
                i += 1;
                o.swap_to = Some(args.get(i).cloned().ok_or_else(|| anyhow!("--swap-to takes a plugin name"))?);
            }
            "--channel" => {
                i += 1;
                o.channel = Some(num(i, "--channel")? as u8).filter(|c| (1..=16).contains(c)).ok_or_else(|| anyhow!("--channel takes 1-16"))?;
            }
            "--loops" => {
                i += 1;
                o.loops = num(i, "--loops")? as u32;
            }
            "--seconds" => {
                i += 1;
                o.seconds = Some(num(i, "--seconds")?);
            }
            "--timeout" => {
                i += 1;
                o.timeout = Duration::from_secs_f64(num(i, "--timeout")?);
            }
            "--audio-out" => {
                i += 1;
                o.audio_out = Some(num(i, "--audio-out")? as u8).filter(|c| *c >= 1);
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(());
            }
            s if s.starts_with("--") => bail!("unknown option {s}\n{USAGE}"),
            s => o.name = Some(s.to_string()),
        }
        i += 1;
    }
    let host = PluginHost::with_default_cache();
    if do_list || rescan {
        return list(&host, rescan);
    }
    let query = o.name.clone().unwrap_or_else(|| "aumu dls  appl".into());
    let info = host.find(&query).context("try --list")?;
    let swap_info = o.swap_to.as_deref().map(|q| host.find(q)).transpose()?;
    if o.bench {
        return bench(&host, &info, swap_info.as_ref(), &o);
    }
    play(&host, &info, swap_info.as_ref(), o)
}

/// Load with progress lines, as the app's browser would show them.
fn load_verbose(host: &PluginHost, info: &PluginInfo, cfg: LoadConfig) -> Result<PluginInstance> {
    let mut h = host.load_async(&info.id, cfg)?;
    let mut last = LoadProgress::Queued;
    loop {
        if let Some(r) = h.take() {
            let inst = r?;
            let t = inst.load_times();
            println!(
                "load:    {} {} in {:.0} ms (instantiate {:.0} ms, initialize {:.0} ms{}){}",
                info.full_name(),
                info.version_string(),
                t.total().as_secs_f64() * 1000.0,
                t.instantiate.as_secs_f64() * 1000.0,
                t.initialize.as_secs_f64() * 1000.0,
                if t.restore.is_zero() { String::new() } else { format!(", restore {:.0} ms", t.restore.as_secs_f64() * 1000.0) },
                if inst.out_of_process() { ", out of process" } else { ", in process" },
            );
            return Ok(inst);
        }
        let p = h.progress();
        if p != last {
            println!("         {:>6.0} ms  {p:?}", h.elapsed().as_secs_f64() * 1000.0);
            last = p;
        }
        // Keep the main run loop turning while we wait: some plugins (Kontakt) finish their
        // set-up on the main queue and hang later calls if it never runs.
        editor::run_main_loop(Duration::from_millis(1));
    }
}

/// Run `f` on a helper thread while this (main) thread keeps its run loop turning, the way
/// the app's control thread and main thread divide the work.
fn off_main<R: Send>(f: impl FnOnce() -> R + Send) -> R {
    std::thread::scope(|s| {
        let h = s.spawn(f);
        while !h.is_finished() {
            editor::run_main_loop(Duration::from_millis(1));
        }
        h.join().expect("helper thread panicked")
    })
}

fn mode(o: &Opts) -> LoadMode {
    if o.oop { LoadMode::OutOfProcess } else { LoadMode::Auto }
}

fn chord_on(inst: &mut PluginInstance, ch: u8, on: bool) {
    for n in [48u8, 55, 60, 64, 67, 72] {
        let _ = inst.midi(if on { [0x90 | ch, n, 100] } else { [0x80 | ch, n, 0] }, 0);
    }
}

fn percentile(v: &mut [f64], p: f64) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    v[((v.len() - 1) as f64 * p).round() as usize]
}

/// Offline measurements: load, state, render CPU, preload-then-swap.
fn bench(host: &PluginHost, info: &PluginInfo, swap_info: Option<&PluginInfo>, o: &Opts) -> Result<()> {
    let rate = 48_000.0;
    let cfg = LoadConfig { sample_rate: rate, max_frames: 4096, mode: mode(o), timeout: o.timeout, ..Default::default() };
    let mut inst = load_verbose(host, info, cfg.clone())?;
    println!("latency: {:.2} ms reported by the plugin", inst.latency_seconds() * 1000.0);

    // State round trip, on a control thread as the Session will do it (Kontakt deadlocks
    // restoring state on the main thread: it waits for work it queues there).
    let (state, get_ms, set_ms, again) = off_main(|| -> Result<_> {
        let t0 = Instant::now();
        let state = inst.get_state()?;
        let get_ms = t0.elapsed().as_secs_f64() * 1000.0;
        let t1 = Instant::now();
        inst.set_state(&state)?;
        let set_ms = t1.elapsed().as_secs_f64() * 1000.0;
        let again = inst.get_state()?;
        Ok((state, get_ms, set_ms, again))
    })?;
    println!(
        "state:   {} bytes; get {get_ms:.1} ms, set {set_ms:.1} ms; round trip {}",
        state.len(),
        if again == state { "byte-identical".to_string() } else { format!("re-saved as {} bytes (not byte-identical)", again.len()) }
    );

    // Render CPU per block, a six-note chord held.
    let ch = o.channel - 1;
    println!("render:  six-note chord held, 2 s per block size (mean / p99 / max per block, share of the block)");
    for block in [64usize, 128, 256] {
        let (mut l, mut r) = (vec![0f32; block], vec![0f32; block]);
        chord_on(&mut inst, ch, true);
        // Warm up: the first blocks after a note-on page in samples.
        for _ in 0..20 {
            let _ = inst.render(&mut l, &mut r);
        }
        let n = (2.0 * rate / block as f64) as usize;
        let mut us = Vec::with_capacity(n);
        let mut peak = 0f32;
        for _ in 0..n {
            let t = Instant::now();
            inst.render(&mut l, &mut r).map_err(|e| anyhow!("{e}"))?;
            us.push(t.elapsed().as_secs_f64() * 1e6);
            peak = l.iter().chain(&r).fold(peak, |m, x| m.max(x.abs()));
        }
        chord_on(&mut inst, ch, false);
        for _ in 0..50 {
            let _ = inst.render(&mut l, &mut r);
        }
        let block_us = block as f64 * 1e6 / rate;
        let mean = us.iter().sum::<f64>() / us.len() as f64;
        let p99 = percentile(&mut us, 0.99);
        let max = us.last().copied().unwrap_or(0.0);
        println!(
            "         {block:>4} frames ({block_us:>5.0} us): {mean:>6.1} / {p99:>6.1} / {max:>6.1} us  = {:>4.1}% / {:>4.1}% / {:>5.1}%   peak {}",
            mean / block_us * 100.0,
            p99 / block_us * 100.0,
            max / block_us * 100.0,
            if peak > 0.0 { format!("{:.1} dBFS", 20.0 * peak.log10()) } else { "silent".into() }
        );
    }

    // Preload with state (what a Registration recall does ahead of time).
    let t2 = Instant::now();
    let other_info = swap_info.unwrap_or(info);
    let other_state = if swap_info.is_some() { None } else { Some(state.clone()) };
    let other = off_main(|| host.load(&other_info.id, LoadConfig { state: other_state, ..cfg.clone() }))?;
    println!("preload: second instance ({}) ready in {:.0} ms, off the audio thread", other_info.full_name(), t2.elapsed().as_secs_f64() * 1000.0);

    swap_bench(inst, other, ch, rate)
}

/// Swap two preloaded instances back and forth on a real-time paced render thread (64-frame
/// blocks at 48 kHz) and measure what the audio side sees.
fn swap_bench(a: PluginInstance, b: PluginInstance, ch: u8, rate: f64) -> Result<()> {
    const BLOCK: usize = 64;
    const SWAPS: usize = 40;
    let (mut rack, mut ctl) = rack(BLOCK, rate);
    let (mut notes_tx, mut notes_rx) = RingBuffer::<Msg>::new(64);
    let running = Arc::new(AtomicBool::new(true));
    // Render time of each block (us), and whether it swapped or crossfaded.
    let (mut times_tx, mut times_rx) = RingBuffer::<(f32, bool)>::new(1 << 16);
    let run2 = running.clone();
    let audio = std::thread::spawn(move || {
        let (mut l, mut r) = ([0f32; BLOCK], [0f32; BLOCK]);
        let period = Duration::from_secs_f64(BLOCK as f64 / rate);
        // The same thread policy as the real audio callback.
        let p = period.as_nanos() as u64;
        crate::rt::make_realtime(p, p / 2, p);
        let mut next = Instant::now();
        while run2.load(Relaxed) {
            let t = Instant::now();
            let fading_before = rack.is_fading(ch);
            let owned = rack.owns(ch);
            rack.begin_block();
            let swapped = fading_before || rack.is_fading(ch) || owned != rack.owns(ch);
            while let Ok(m) = notes_rx.pop() {
                rack.midi(m, 0);
            }
            l.fill(0.0);
            r.fill(0.0);
            rack.render_add(&mut l, &mut r);
            let _ = times_tx.push((t.elapsed().as_secs_f32() * 1e6, swapped));
            next += period;
            let now = Instant::now();
            if next > now {
                std::thread::sleep(next - now);
            } else {
                next = now;
            }
        }
        rack
    });

    let hold = |tx: &mut rtrb::Producer<Msg>, on: bool| {
        for n in [48u8, 55, 60, 64, 67] {
            let _ = tx.push(if on { [0x90 | ch, n, 100] } else { [0x80 | ch, n, 0] });
        }
    };
    let mut spare = Some(b);
    ctl.assign(ch, a, Swap::default()).map_err(|_| anyhow!("rack full"))?;
    std::thread::sleep(Duration::from_millis(100));
    let mut latencies = Vec::new();
    let mut assign_calls = Vec::new();
    for k in 0..SWAPS {
        hold(&mut notes_tx, true);
        // Land the swap at an arbitrary point in the block period.
        std::thread::sleep(Duration::from_micros(40_000 + (k as u64 * 331) % 1333));
        let inst = spare.take().ok_or_else(|| anyhow!("no spare instance"))?;
        let t = Instant::now();
        ctl.assign(ch, inst, Swap::default()).map_err(|_| anyhow!("rack full"))?;
        assign_calls.push(t.elapsed().as_secs_f64() * 1e6);
        // Wait for the swap and the retired instance.
        let deadline = Instant::now() + Duration::from_secs(1);
        let mut swapped = false;
        while Instant::now() < deadline && (spare.is_none() || !swapped) {
            for e in ctl.poll_events() {
                match e {
                    RackEvent::Swapped { latency, .. } => {
                        latencies.push(latency.as_secs_f64() * 1e6);
                        swapped = true;
                    }
                    RackEvent::Fault { error, .. } => bail!("fault during swap: {error}"),
                    _ => {}
                }
            }
            if spare.is_none() {
                spare = ctl.take_retired().into_iter().next();
            }
            std::thread::sleep(Duration::from_micros(200));
        }
        hold(&mut notes_tx, false);
        if spare.is_none() {
            bail!("the outgoing instance did not come back within 1 s");
        }
    }
    std::thread::sleep(Duration::from_millis(50));
    running.store(false, Relaxed);
    let rack = audio.join().map_err(|_| anyhow!("render thread panicked"))?;
    drop(rack);
    let (mut steady, mut swap) = (Vec::new(), Vec::new());
    while let Ok((us, swapped)) = times_rx.pop() {
        if swapped { swap.push(us as f64) } else { steady.push(us as f64) }
    }
    let summary = |v: &mut Vec<f64>| -> String {
        if v.is_empty() {
            return "none".into();
        }
        let mean = v.iter().sum::<f64>() / v.len() as f64;
        let p99 = percentile(v, 0.99);
        format!("{} blocks, mean {mean:.1} / p99 {p99:.1} / max {:.1} us", v.len(), v.last().copied().unwrap_or(0.0))
    };
    let mean_lat = latencies.iter().sum::<f64>() / latencies.len().max(1) as f64;
    let max_lat = latencies.iter().cloned().fold(0.0, f64::max);
    let assign_us = assign_calls.iter().sum::<f64>() / assign_calls.len().max(1) as f64;
    println!(
        "swap:    {SWAPS} preload-then-swap hand-overs at 64 frames / 48 kHz (block {:.0} us), chord held, 5 ms crossfade:",
        BLOCK as f64 * 1e6 / rate
    );
    println!("         assign() call {assign_us:.1} us; assign -> playing mean {:.2} ms, max {:.2} ms", mean_lat / 1000.0, max_lat / 1000.0);
    println!("         steady blocks: {}", summary(&mut steady));
    println!("         swap + crossfade blocks (both instances render): {}", summary(&mut swap));
    Ok(())
}

fn play(host: &PluginHost, info: &PluginInfo, swap_info: Option<&PluginInfo>, o: Opts) -> Result<()> {
    let mut sf2 = o.sf2.clone();
    if sf2.is_none() && !o.no_sf2 {
        sf2 = std::fs::read_dir("soundfonts").into_iter().flatten().flatten().map(|e| e.path())
            .find(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("sf2")));
    }
    let device = cpal::default_host().default_output_device().ok_or_else(|| anyhow!("no audio output device"))?;
    let device_name = device.description().map(|d| d.to_string()).unwrap_or_else(|_| "default output".into());
    let default = device.default_output_config()?;
    let sample_rate = default.sample_rate();
    // Same output choice as the built-in synth: every channel open, Model 16 on 11/12.
    let channels = device
        .supported_output_configs()
        .map(|it| {
            it.filter(|c| c.min_sample_rate() <= sample_rate && sample_rate <= c.max_sample_rate())
                .filter(|c| c.sample_format() == cpal::SampleFormat::F32)
                .map(|c| c.channels() as usize)
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0)
        .max(default.channels() as usize);
    if channels < 2 {
        bail!("{device_name} has fewer than two output channels");
    }
    let first = o.audio_out.map(|c| c - 1).unwrap_or(if device_name.contains("Model 16") { 10 } else { 0 });
    let lc = (first as usize).min(channels - 2);

    let cfg = LoadConfig { sample_rate: sample_rate as f64, max_frames: 4096, mode: mode(&o), timeout: o.timeout, ..Default::default() };
    let inst = load_verbose(host, info, cfg.clone())?;
    let stats = inst.stats();
    let editor_target = inst.editor_target();
    let swap_inst = match swap_info {
        Some(si) => Some(load_verbose(host, si, cfg.clone())?),
        None => None,
    };
    let swap_stats = swap_inst.as_ref().map(|i| i.stats());

    let mut synth = match &sf2 {
        Some(p) => {
            let mut f = std::fs::File::open(p).with_context(|| format!("opening {}", p.display()))?;
            let font = Arc::new(SoundFont::new(&mut f).map_err(|e| anyhow!("{e:?}"))?);
            let mut s = SynthesizerSettings::new(sample_rate as i32);
            s.maximum_polyphony = 64;
            Some(Synthesizer::new(&font, &s).map_err(|e| anyhow!("{e:?}"))?)
        }
        None => None,
    };

    let plugin_ch = o.channel - 1;
    let backing_ch = if plugin_ch == 1 { 2 } else { 1 };
    let (mut rack, mut ctl) = rack(4096, sample_rate as f64);
    ctl.assign(plugin_ch, inst, Swap::default()).map_err(|_| anyhow!("rack full"))?;
    let (mut tx, mut rx) = RingBuffer::<Msg>::new(1024);
    let peak = Arc::new(AtomicU32::new(0));
    let peak_rt = peak.clone();
    let (mut l, mut r, mut l2, mut r2) = (vec![0f32; 4096], vec![0f32; 4096], vec![0f32; 4096], vec![0f32; 4096]);
    let callback = move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
        rack.begin_block();
        while let Ok(m) = rx.pop() {
            if !rack.midi(m, 0)
                && let Some(s) = synth.as_mut()
            {
                s.process_midi_message((m[0] & 0x0F) as i32, (m[0] & 0xF0) as i32, m[1] as i32, m[2] as i32);
            }
        }
        for chunk in out.chunks_mut(channels * 4096) {
            let n = chunk.len() / channels;
            let (l, r) = (&mut l[..n], &mut r[..n]);
            l.fill(0.0);
            r.fill(0.0);
            rack.render_add(l, r);
            let p = l.iter().chain(r.iter()).fold(0f32, |m, x| m.max(x.abs()));
            if p > f32::from_bits(peak_rt.load(Relaxed)) {
                peak_rt.store(p.to_bits(), Relaxed);
            }
            if let Some(s) = synth.as_mut() {
                s.render(&mut l2[..n], &mut r2[..n]);
                for i in 0..n {
                    l[i] += l2[i];
                    r[i] += r2[i];
                }
            }
            for (i, frame) in chunk.chunks_mut(channels).enumerate() {
                frame.fill(0.0);
                frame[lc] = soft_clip(l[i]);
                frame[lc + 1] = soft_clip(r[i]);
            }
        }
    };
    let buffer = match default.buffer_size() {
        cpal::SupportedBufferSize::Range { min, max } if *min <= 64 && 64 <= *max => Some(64u32),
        cpal::SupportedBufferSize::Range { min, .. } if *min > 64 => Some(*min),
        _ => None,
    };
    let stream_cfg = cpal::StreamConfig {
        channels: channels as u16,
        sample_rate,
        buffer_size: buffer.map(cpal::BufferSize::Fixed).unwrap_or(cpal::BufferSize::Default),
    };
    let stream = device.build_output_stream(stream_cfg, callback, |e| eprintln!("audio error: {e}"), None)?;
    stream.play()?;

    println!("audio:   {device_name} out {}/{}, {sample_rate} Hz, buffer {}", lc + 1, lc + 2, buffer.map_or("default".into(), |b| format!("{b} frames ({:.1} ms)", b as f64 * 1000.0 / sample_rate as f64)));
    println!("routing: MIDI ch {} -> plugin rack (host-side CC7/CC11); ch {} strings + drums -> {}", o.channel, backing_ch + 1,
        sf2.as_ref().map_or("nothing (no SoundFont)".into(), |p| format!("rustysynth {}", p.display())));
    if let Some(si) = swap_info {
        println!("swap:    {} preloaded; swaps in at bar 3 of each pass", si.full_name());
    }

    let running = Arc::new(AtomicBool::new(true));
    let player = {
        let running = running.clone();
        let loops = if o.gui || o.seconds.is_some() { u32::MAX } else { o.loops.max(1) };
        let mut swap_inst = swap_inst;
        std::thread::spawn(move || {
            let beat = Duration::from_millis(600); // 100 bpm
            let mut swap_lat = Vec::new();
            for pass in 0..loops {
                if !running.load(Relaxed) {
                    break;
                }
                let start = Instant::now();
                let mut swapped_this_pass = false;
                for (t, m) in phrase(plugin_ch, backing_ch, pass + 1 == loops) {
                    let at = start + beat.mul_f64(t);
                    while Instant::now() < at && running.load(Relaxed) {
                        std::thread::sleep(at.saturating_duration_since(Instant::now()).min(Duration::from_millis(20)));
                        for e in ctl.poll_events() {
                            match e {
                                RackEvent::Swapped { latency, .. } => swap_lat.push(latency),
                                other => println!("rack:    {other:?}"),
                            }
                        }
                    }
                    if !running.load(Relaxed) {
                        break;
                    }
                    if t >= 8.0 && !swapped_this_pass {
                        swapped_this_pass = true;
                        // Hand the preloaded instance over; the outgoing one comes back and
                        // is the next pass's spare.
                        if let Some(next) = swap_inst.take() {
                            let _ = ctl.assign(plugin_ch, next, Swap::default());
                        }
                    }
                    let _ = tx.push(m);
                    if swap_inst.is_none() {
                        swap_inst = ctl.take_retired().into_iter().next();
                    }
                }
                std::thread::sleep(beat * 2);
                if swap_inst.is_none() {
                    swap_inst = ctl.take_retired().into_iter().next();
                }
            }
            for ch in 0..16u8 {
                let _ = tx.push([0xB0 | ch, 123, 0]);
            }
            std::thread::sleep(Duration::from_millis(800));
            running.store(false, Relaxed);
            for e in ctl.poll_events() {
                if let RackEvent::Swapped { latency, .. } = e {
                    swap_lat.push(latency);
                }
            }
            // The first "swap" is the initial assign.
            if swap_lat.len() > 1 {
                let ms: Vec<f64> = swap_lat[1..].iter().map(|d| d.as_secs_f64() * 1000.0).collect();
                println!("swap:    {} live swaps, assign -> playing mean {:.2} ms, max {:.2} ms",
                    ms.len(), ms.iter().sum::<f64>() / ms.len() as f64, ms.iter().cloned().fold(0.0, f64::max));
            }
            // Instances still in the rack are dropped with the stream.
        })
    };

    if let Some(secs) = o.seconds {
        let running = running.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs_f64(secs));
            running.store(false, Relaxed);
        });
    }
    if o.gui {
        let mtm = editor::main_thread()?;
        editor::prepare_app(mtm);
        match editor::open_editor(mtm, &editor_target) {
            Ok(ed) => {
                let (w, h) = ed.size();
                println!("gui:     {:?} view, {w:.0}x{h:.0}; close the window to stop", ed.kind());
                while running.load(Relaxed) && ed.is_open() {
                    editor::pump_events(mtm, Duration::from_millis(20));
                }
                editor::close_editor(ed);
            }
            Err(e) => eprintln!("gui: {e}"),
        }
        running.store(false, Relaxed);
    }
    let _ = player.join();
    drop(stream);
    let p = f32::from_bits(peak.load(Relaxed));
    println!("result:  plugin part peak {}", if p > 0.0 { format!("{:.1} dBFS", 20.0 * p.log10()) } else { "silent".into() });
    for (name, s) in std::iter::once((info.full_name(), stats)).chain(swap_info.map(|i| i.full_name()).zip(swap_stats)) {
        let s = s.snapshot(sample_rate as f64);
        println!(
            "cpu:     {name}: {} blocks, mean {:.1} us, max {:.1} us, {:.1}% of real time; overruns (>50% of a block) {}, deadline misses {}, errors {}",
            s.blocks, s.mean_us, s.max_us, s.cpu * 100.0, s.overruns, s.deadline_misses, s.errors
        );
    }
    Ok(())
}
