//! `yahaha plugin-test`: play a short phrase through an Audio Unit instrument so you can
//! hear it, with the backing on the built-in SoundFont synth in the same audio callback.
//!
//! Routing, as the real integration would do it per part (docs/plugin-hosting.md): the
//! test channel (default MIDI channel 1 = Right 1) goes to the plugin, every other channel
//! to rustysynth. The plugin's CC7/CC11 are applied by the host ([`super::PartGain`]).

use anyhow::{anyhow, bail, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rtrb::RingBuffer;
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::{au, gui, Instrument, PartGain};
use crate::synth::soft_clip;

const USAGE: &str = "usage: yahaha plugin-test [name | \"aumu subt manu\"] [--list] [--gui] [--channel 1-16] [--sf2 file | --no-sf2] [--loops N] [--seconds N] [--audio-out N]";

type Msg = [u8; 3];

/// The test phrase: (beat, message). Channel 0 is replaced by the plugin channel.
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
        // Kick on 1 and 3, closed hat on every beat.
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
    // Melody on the plugin channel.
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
        // The plugin part's fader down over the last bar: the host applies it.
        for i in 0..=16 {
            ev.push((16.0 + i as f64 * 0.25, [0xB0 | p, 7, 100 - (i * 6) as u8]));
        }
    }
    ev.sort_by(|a, b| a.0.total_cmp(&b.0));
    ev
}

fn list() {
    let all = au::scan();
    println!("{} instrument Audio Units:", all.len());
    for c in &all {
        let mut tags = Vec::new();
        if c.v3 {
            tags.push("AUv3");
        }
        if c.async_only {
            tags.push("async-only: not loadable here");
        }
        println!("  {}  {}  v{:#x}{}", c.code(), c.name, c.version, if tags.is_empty() { String::new() } else { format!("  [{}]", tags.join(", ")) });
    }
}

pub fn run(args: &[String]) -> Result<()> {
    let mut name: Option<String> = None;
    let mut show_gui = false;
    let mut channel = 1u8;
    let mut sf2: Option<PathBuf> = None;
    let mut no_sf2 = false;
    let mut loops = 1u32;
    let mut seconds: Option<f64> = None;
    let mut audio_out: Option<u8> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--list" => {
                list();
                return Ok(());
            }
            "--gui" => show_gui = true,
            "--no-sf2" => no_sf2 = true,
            "--sf2" => {
                i += 1;
                sf2 = args.get(i).map(PathBuf::from);
            }
            "--channel" => {
                i += 1;
                channel = args.get(i).and_then(|s| s.parse().ok()).filter(|c| (1..=16).contains(c)).ok_or_else(|| anyhow!("--channel takes 1-16"))?;
            }
            "--loops" => {
                i += 1;
                loops = args.get(i).and_then(|s| s.parse().ok()).ok_or_else(|| anyhow!("--loops takes a number"))?;
            }
            "--seconds" => {
                i += 1;
                seconds = Some(args.get(i).and_then(|s| s.parse().ok()).ok_or_else(|| anyhow!("--seconds takes a number"))?);
            }
            "--audio-out" => {
                i += 1;
                audio_out = Some(args.get(i).and_then(|s| s.parse().ok()).filter(|c| *c >= 1).ok_or_else(|| anyhow!("--audio-out takes the first channel of a pair, 1-based"))?);
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(());
            }
            s if s.starts_with("--") => bail!("unknown option {s}\n{USAGE}"),
            s => name = Some(s.to_string()),
        }
        i += 1;
    }
    let query = name.unwrap_or_else(|| "aumu dls  appl".into());
    let info = au::find(&query).ok_or_else(|| anyhow!("no instrument Audio Unit matches {query:?} (try --list)"))?;
    if sf2.is_none() && !no_sf2 {
        sf2 = std::fs::read_dir("soundfonts").into_iter().flatten().flatten().map(|e| e.path())
            .find(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("sf2")));
    }

    let host = cpal::default_host();
    let device = host.default_output_device().ok_or_else(|| anyhow!("no audio output device"))?;
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
    let first = audio_out.map(|c| c - 1).unwrap_or(if device_name.contains("Model 16") { 10 } else { 0 });
    let lc = (first as usize).min(channels - 2);

    let t0 = Instant::now();
    let mut inst = Instrument::load(&info, sample_rate as f64, 4096)?;
    let load_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let latency = inst.latency_seconds();
    match inst.save_state() {
        Ok(state) => {
            inst.restore_state(&state).context("restoring the state just saved")?;
            println!("state: {} bytes (binary plist), save + restore OK", state.len());
        }
        Err(e) => println!("state: {e}"),
    }
    let unit = inst.raw();

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

    let plugin_ch = channel - 1;
    let backing_ch = if plugin_ch == 1 { 2 } else { 1 };
    let (mut tx, mut rx) = RingBuffer::<Msg>::new(1024);
    let mut gain = PartGain::default();
    // Peak of the plugin part after its gain, as f32 bits: proof it really sounded.
    let peak = Arc::new(AtomicU32::new(0));
    let peak_rt = peak.clone();
    let (mut l, mut r, mut l2, mut r2) = (vec![0f32; 4096], vec![0f32; 4096], vec![0f32; 4096], vec![0f32; 4096]);
    let callback = move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
        while let Ok(m) = rx.pop() {
            if m[0] & 0x0F == plugin_ch {
                if !gain.take(m) {
                    inst.midi(m, 0);
                }
            } else if let Some(s) = synth.as_mut() {
                s.process_midi_message((m[0] & 0x0F) as i32, (m[0] & 0xF0) as i32, m[1] as i32, m[2] as i32);
            }
        }
        for chunk in out.chunks_mut(channels * 4096) {
            let n = chunk.len() / channels;
            let (l, r) = (&mut l[..n], &mut r[..n]);
            // On a render error the buffers come back silent; nothing to do on this thread.
            let _ = inst.render(l, r);
            gain.apply(l, r);
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
    let cfg = cpal::StreamConfig {
        channels: channels as u16,
        sample_rate,
        buffer_size: buffer.map(cpal::BufferSize::Fixed).unwrap_or(cpal::BufferSize::Default),
    };
    let stream = device.build_output_stream(cfg, callback, |e| eprintln!("audio error: {e}"), None)?;
    stream.play()?;

    println!("plugin:  {} ({}){}, loaded in {load_ms:.0} ms, reported latency {:.1} ms", info.name, info.code(), if info.v3 { " AUv3" } else { "" }, latency * 1000.0);
    println!("audio:   {device_name} out {}/{}, {sample_rate} Hz, buffer {}", lc + 1, lc + 2, buffer.map_or("default".into(), |b| format!("{b} frames ({:.1} ms)", b as f64 * 1000.0 / sample_rate as f64)));
    println!("routing: MIDI ch {channel} -> plugin (host-side CC7/CC11); ch {} strings + drums -> {}", backing_ch + 1,
        sf2.as_ref().map_or("nothing (no SoundFont)".into(), |p| format!("rustysynth {}", p.display())));

    let running = Arc::new(AtomicBool::new(true));
    let player = {
        let running = running.clone();
        let loops = if show_gui || seconds.is_some() { u32::MAX } else { loops.max(1) };
        std::thread::spawn(move || {
            let beat = Duration::from_millis(600); // 100 bpm
            for pass in 0..loops {
                if !running.load(Relaxed) {
                    break;
                }
                let start = Instant::now();
                for (t, m) in phrase(plugin_ch, backing_ch, pass + 1 == loops) {
                    let at = start + beat.mul_f64(t);
                    while Instant::now() < at {
                        if !running.load(Relaxed) {
                            break;
                        }
                        std::thread::sleep(at.saturating_duration_since(Instant::now()).min(Duration::from_millis(20)));
                    }
                    if !running.load(Relaxed) {
                        break;
                    }
                    let _ = tx.push(m);
                }
                std::thread::sleep(beat * 2);
            }
            // All notes off everywhere, then let the tails ring.
            for ch in 0..16u8 {
                let _ = tx.push([0xB0 | ch, 123, 0]);
            }
            std::thread::sleep(Duration::from_millis(800));
            running.store(false, Relaxed);
        })
    };

    if let Some(secs) = seconds {
        let running = running.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs_f64(secs));
            running.store(false, Relaxed);
        });
    }
    if show_gui {
        println!("gui:     close the plugin window to stop");
        // SAFETY: this is the main thread, and `inst` (owned by the stream's callback) lives
        // until `stream` is dropped below.
        let r = unsafe { gui::show(unit, &info.name, &running) };
        running.store(false, Relaxed);
        if let Err(e) = r {
            eprintln!("gui: {e}");
        }
    }
    let _ = player.join();
    drop(stream);
    let p = f32::from_bits(peak.load(Relaxed));
    println!("result:  plugin part peak {}", if p > 0.0 { format!("{:.1} dBFS", 20.0 * p.log10()) } else { "silent".into() });
    Ok(())
}
