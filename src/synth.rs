//! Built-in SoundFont player: rustysynth rendering inside CoreAudio's IO callback (via cpal).
//!
//! MIDI reaches the audio thread through SPSC rings (one per producer thread), drained at
//! the start of each buffer. With a 64-frame buffer at 48 kHz, an event waits at most
//! 1.3 ms before it is rendered.

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rtrb::{Consumer, Producer, RingBuffer};
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering::{Acquire, Relaxed}};
use std::sync::Arc;

use crate::parts::{self, Parts};

pub type Msg = [u8; 3];

pub struct SynthInfo {
    pub name: String,
    pub sample_rate: u32,
    pub buffer: Option<u32>,
    pub device: String,
    /// Number of output channels on the device.
    pub channels: usize,
}

/// Knobs the UI and input thread can turn without a ring: plain atomics.
///
/// Your playing has its own synthesizer instance: the keyboard parts on their channels
/// (`parts::CHANNEL`: Right 1 = 0, Left = 1, Right 2 = 2, Right 3 = 3), exactly as the
/// input thread sends them to the port. The band plays on a second instance, channels 9-16.
pub struct SynthControl {
    pub master: AtomicU8,
    pub muted: AtomicBool,
    /// First (left) output channel of the stereo pair, 0-based.
    pub out_ch: AtomicU8,
    /// The Launchkey master fader is waiting to pick up `master` (soft takeover).
    pub master_waiting: AtomicBool,
}

pub struct Synth {
    _stream: cpal::Stream,
    pub info: SynthInfo,
    pub control: Arc<SynthControl>,
}

/// Rings feeding the synth: hand the producers to the threads that generate MIDI.
pub struct Feeds {
    pub engine: Option<Producer<Msg>>,
    pub input: Option<Producer<Msg>>,
    pub consumers: Vec<Consumer<Msg>>,
}

impl SynthControl {
    pub fn new(out_ch: u8) -> SynthControl {
        SynthControl {
            master: AtomicU8::new(MASTER_UNITY),
            muted: AtomicBool::new(false),
            out_ch: AtomicU8::new(out_ch),
            master_waiting: AtomicBool::new(false),
        }
    }
}

/// Push the keyboard parts' programs to the player synth. Their volumes arrive as CC7 on
/// their channels, like any other message.
fn sync_player(player: &mut Synthesizer, parts: &Parts) {
    for p in 0..parts::COUNT {
        player.process_midi_message(parts::CHANNEL[p] as i32, 0xC0, parts.channel_program(p) as i32, 0);
    }
}

pub fn feeds() -> Feeds {
    let (engine, c1) = RingBuffer::new(4096);
    let (input, c2) = RingBuffer::new(1024);
    Feeds { engine: Some(engine), input: Some(input), consumers: vec![c1, c2] }
}

/// GM program to use for a Yamaha voice. Yamaha's GM/XG banks (MSB 0) follow GM
/// numbering; Genos-only banks don't, so the part's role decides when the number would
/// land in the wrong instrument family.
pub fn gm_fallback(dest: u8, msb: u8, prog: u8) -> u8 {
    if msb == 0 {
        return prog;
    }
    match dest {
        10 if !(32..=39).contains(&prog) => 33, // Bass part -> Finger Bass
        _ => prog,
    }
}

/// GM program for the Style's Bass part voice, which Manual Bass moves onto the Left part.
/// TODO: this is the voice from the Style's init setup only; a Bass program change inside a
/// section is not followed yet (see docs/backlog.md, #5 follow-ups).
pub fn style_bass_program(voice: Option<(u8, u8, u8)>) -> u8 {
    match voice {
        Some((msb, _, pc)) if msb < 126 => gm_fallback(10, msb, pc),
        _ => 33,
    }
}

/// Master fader value at which the synth's output is at unity gain (the SoundFont's own
/// level). The master fader is the only gain here that is not a MIDI message: part levels
/// come from each channel's CC7 (the mixer faders), CC11 and velocity alone, on the
/// standard GM curves (rustysynth: gain = (vel/127)² · ((CC7/127)·(CC11/127))²).
pub const MASTER_UNITY: u8 = 100;

/// Output gain for a master fader value: linear, 1.0 at `MASTER_UNITY`.
#[inline]
pub fn master_gain(master: u8) -> f32 {
    master.min(127) as f32 / MASTER_UNITY as f32
}

/// Where the output safety clipper starts: -1 dBFS. Below it the output is untouched.
pub const CLIP_KNEE: f32 = 0.891_250_9;

/// Safety soft clipper on the final output only (not a level control: nothing below
/// -1 dBFS changes). Above the knee the sample bends smoothly (slope 1 at the knee, tanh
/// shape) towards full scale, which it never exceeds, so a hot mix at master 127 plus
/// your playing saturates gently instead of wrapping into digital clipping.
#[inline]
pub fn soft_clip(x: f32) -> f32 {
    let a = x.abs();
    if a <= CLIP_KNEE {
        return x;
    }
    let room = 1.0 - CLIP_KNEE;
    (CLIP_KNEE + room * ((a - CLIP_KNEE) / room).tanh()).copysign(x)
}

fn apply(synth: &mut Synthesizer, player: &mut Synthesizer, m: &Msg, bank: &mut [u8; 16]) {
    let ch = (m[0] & 0x0F) as i32;
    let st = (m[0] & 0xF0) as i32;
    let v = m[2] as i32;
    // Your playing goes to the player synth, each keyboard part on its own channel. The
    // input thread already decided which parts sound and where (on/off, octave).
    if parts::part_of_channel(ch as u8).is_some() {
        player.process_midi_message(ch, st, m[1] as i32, v);
        return;
    }
    match st {
        // Style bank selects are Yamaha banks; the SoundFont gets GM banks instead.
        0xB0 if m[1] == 0 => bank[ch as usize] = m[2],
        0xB0 if m[1] == 32 => {}
        // Rhythm 1 (ch 9) is a drum part too: keep it on the drum bank.
        0xC0 if ch == 8 => {
            synth.process_midi_message(8, 0xB0, 0, 128);
            synth.process_midi_message(8, 0xC0, m[1] as i32, 0);
        }
        0xC0 => {
            let p = gm_fallback(ch as u8, bank[ch as usize], m[1]);
            synth.process_midi_message(ch, 0xC0, p as i32, 0);
        }
        // Everything else as sent: CC7 (the mixer fader), CC11 and velocity reach the voice
        // unchanged, so the SoundFont answers them exactly as an external GM instrument would.
        _ => synth.process_midi_message(ch, st, m[1] as i32, v),
    }
}

pub fn start(sf2: &Path, consumers: Vec<Consumer<Msg>>, out_pair: Option<u8>, parts: Arc<Parts>) -> Result<Synth> {
    let mut file = std::fs::File::open(sf2).with_context(|| format!("opening {}", sf2.display()))?;
    let font = Arc::new(SoundFont::new(&mut file).map_err(|e| anyhow!("{e:?}"))?);

    let host = cpal::default_host();
    let device = host.default_output_device().ok_or_else(|| anyhow!("no audio output device"))?;
    let device_name = device.description().map(|d| d.to_string()).unwrap_or_else(|_| "default output".into());
    let default = device.default_output_config()?;
    let sample_rate = default.sample_rate();
    // Open every output channel the device has so any stereo pair can be used.
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
    let first = out_pair.map(|c| c.saturating_sub(1)).unwrap_or(if device_name.contains("Model 16") { 10 } else { 0 });
    let first = (first as usize).min(channels.saturating_sub(2)) as u8;

    let mut settings = SynthesizerSettings::new(sample_rate as i32);
    settings.maximum_polyphony = 128;
    let mut synth = Synthesizer::new(&font, &settings).map_err(|e| anyhow!("{e:?}"))?;
    let mut player_synth = Synthesizer::new(&font, &settings).map_err(|e| anyhow!("{e:?}"))?;
    synth.process_midi_message(8, 0xB0, 0, 128);

    let control = Arc::new(SynthControl::new(first));
    let ctl = control.clone();
    let mut consumers = consumers;
    let mut bank = [0u8; 16];
    let mut last_master = 255u8;
    let mut left = vec![0f32; 8192];
    let mut right = vec![0f32; 8192];
    let mut left2 = vec![0f32; 8192];
    let mut right2 = vec![0f32; 8192];

    let callback = move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
        // Acquire pairs with the Release stores in `Parts`: the new programs are visible.
        if parts.changed.swap(false, Acquire) {
            sync_player(&mut player_synth, &parts);
        }
        let master = ctl.master.load(Relaxed);
        if master != last_master {
            last_master = master;
            synth.set_master_volume(master_gain(master));
            player_synth.set_master_volume(master_gain(master));
        }
        for c in consumers.iter_mut() {
            while let Ok(m) = c.pop() {
                apply(&mut synth, &mut player_synth, &m, &mut bank);
            }
        }
        let frames = (out.len() / channels).min(left.len());
        synth.render(&mut left[..frames], &mut right[..frames]);
        player_synth.render(&mut left2[..frames], &mut right2[..frames]);
        for i in 0..frames {
            left[i] += left2[i];
            right[i] += right2[i];
        }
        let mute = ctl.muted.load(Relaxed);
        let lc = (ctl.out_ch.load(Relaxed) as usize).min(channels.saturating_sub(1));
        let rc = (lc + 1).min(channels - 1);
        for (i, frame) in out.chunks_mut(channels).take(frames).enumerate() {
            frame.fill(0.0);
            if !mute {
                frame[lc] += soft_clip(left[i]);
                frame[rc] += soft_clip(right[i]);
            }
        }
    };

    // Ask for a 64-frame buffer when the device allows it.
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
    let name = sf2.file_stem().unwrap_or_default().to_string_lossy().to_string();
    Ok(Synth { _stream: stream, info: SynthInfo { name, sample_rate, buffer, device: device_name, channels }, control })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Prepared;
    use crate::sim::{run, Step};
    use crate::sff::Style;
    use crate::theory::Chord;

    /// Render a style's engine output offline through the SoundFont and measure each part.
    #[test]
    fn soundfont_renders_every_part() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let sf2 = root.join("soundfonts/GeneralUser-GS.sf2");
        let style_path = root.join("corpus/MOX_v2/SlowWalker.T552.sty");
        if !sf2.exists() || !style_path.exists() {
            eprintln!("soundfont or corpus missing; skipping");
            return;
        }
        let font = Arc::new(SoundFont::new(&mut std::fs::File::open(&sf2).unwrap()).unwrap());
        let style = Style::load(&style_path).unwrap();
        let prep = Box::new(Prepared::new(&style));
        let bar = (60e9 / prep.bpm * (prep.tpb as f64 / prep.ppq as f64)) as u64;
        let mut script: Vec<(u64, Step)> = [Chord::new(9, 10), Chord::new(2, 10), Chord::new(7, 19), Chord::new(0, 2)]
            .iter()
            .enumerate()
            .map(|(i, c)| (i as u64 * bar, Step::Chord(*c)))
            .collect();
        // Fill In AA (uses Rhythm 1 on ch 9) during bar 2.
        script.insert(2, (bar + bar / 8, Step::Button(crate::engine::Button::Main(0))));
        let (_, rec) = run(prep, &script, 4 * bar);
        let sr = 48_000;
        let mut rms_by_part = Vec::new();
        for part in 8..16u8 {
            let mut synth = Synthesizer::new(&font, &SynthesizerSettings::new(sr)).unwrap();
            let mut bank = [0u8; 16];
            let mut player = Synthesizer::new(&font, &SynthesizerSettings::new(sr)).unwrap();
            synth.process_midi_message(8, 0xB0, 0, 128);
            let (mut l, mut r) = (vec![0f32; 64], vec![0f32; 64]);
            let mut t_samples = 0u64;
            let mut energy = 0f64;
            let mut n = 0u64;
            let mut i = 0;
            let end = (4 * bar) * sr as u64 / 1_000_000_000;
            while t_samples < end {
                let now_ns = t_samples * 1_000_000_000 / sr as u64;
                while i < rec.out.len() && rec.out[i].0 <= now_ns {
                    let m = &rec.out[i].1;
                    // Setup messages go to everyone; notes only for the part under test.
                    let is_note = matches!(m[0] & 0xF0, 0x80 | 0x90);
                    if !is_note || m[0] & 0x0F == part {
                        let mut a = [0u8; 3];
                        a[..m.len().min(3)].copy_from_slice(&m[..m.len().min(3)]);
                        apply(&mut synth, &mut player, &a, &mut bank);
                    }
                    i += 1;
                }
                synth.render(&mut l, &mut r);
                for k in 0..64 {
                    energy += (l[k] * l[k] + r[k] * r[k]) as f64;
                }
                n += 64;
                t_samples += 64;
            }
            let notes = rec.out.iter().filter(|(_, m)| m[0] == 0x90 | part).count();
            rms_by_part.push((part + 1, notes, (energy / n as f64).sqrt()));
        }
        for (ch, notes, rms) in &rms_by_part {
            eprintln!("ch {ch:>2}: {notes:>4} notes  rms {rms:.4}");
            if *notes > 0 {
                assert!(*rms > 0.001, "ch {ch} played {notes} notes but rendered silence");
            }
        }
    }
}

#[cfg(test)]
mod parts_tests {
    use super::*;

    fn energy(s: &mut Synthesizer) -> f64 {
        let (mut l, mut r) = (vec![0f32; 4800], vec![0f32; 4800]);
        s.render(&mut l, &mut r);
        l.iter().zip(&r).map(|(a, b)| (a * a + b * b) as f64).sum()
    }

    /// Each keyboard part sounds on its own channel of the player synth, as sent; the band
    /// synth never hears it, and a part's CC7 reaches that part alone.
    #[test]
    fn keyboard_parts_play_on_their_own_channels() {
        assert_eq!(style_bass_program(Some((0, 0, 35))), 35);
        assert_eq!(style_bass_program(Some((8, 0, 4))), 33); // Genos-only bank, not a bass number
        assert_eq!(style_bass_program(None), 33);
        let sf2 = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("soundfonts/GeneralUser-GS.sf2");
        if !sf2.exists() {
            eprintln!("soundfont missing; skipping");
            return;
        }
        let font = Arc::new(SoundFont::new(&mut std::fs::File::open(&sf2).unwrap()).unwrap());
        let parts = Parts::new();
        for p in 0..parts::COUNT {
            let mut synth = Synthesizer::new(&font, &SynthesizerSettings::new(48_000)).unwrap();
            let mut player = Synthesizer::new(&font, &SynthesizerSettings::new(48_000)).unwrap();
            sync_player(&mut player, &parts);
            let mut bank = [0u8; 16];
            let ch = parts::CHANNEL[p];
            // Every other part's channel muted by its CC7: only this part can sound.
            for q in (0..parts::COUNT).filter(|&q| q != p) {
                apply(&mut synth, &mut player, &[0xB0 | parts::CHANNEL[q], 7, 0], &mut bank);
                apply(&mut synth, &mut player, &[0x90 | parts::CHANNEL[q], 64, 100], &mut bank);
            }
            assert!(energy(&mut player) < 1e-9, "{}: muted parts are silent", parts::NAMES[p]);
            apply(&mut synth, &mut player, &[0x90 | ch, 60, 100], &mut bank);
            assert!(energy(&mut player) > 1e-3, "{} sounds on ch {}", parts::NAMES[p], ch + 1);
            assert_eq!(energy(&mut synth), 0.0, "the band synth never plays your parts");
        }
    }
}

/// The synth answers velocity, CC7 and CC11 on the standard GM curves (each 40·log10(v/127)
/// dB), with nothing in between, and the master fader is unity at its default.
#[cfg(test)]
mod curve_tests {
    use super::*;

    #[test]
    fn master_is_unity_at_default() {
        assert_eq!(SynthControl::new(0).master.load(Relaxed), MASTER_UNITY);
        assert_eq!(master_gain(MASTER_UNITY), 1.0);
        assert_eq!(master_gain(0), 0.0);
        assert_eq!(master_gain(50), 0.5);
    }

    #[test]
    fn soft_clip_is_transparent_below_minus_1_dbfs() {
        for x in [0.0f32, 0.1, -0.5, 0.7, -0.89, CLIP_KNEE, -CLIP_KNEE] {
            assert_eq!(soft_clip(x), x);
        }
        // Above the knee: continuous, monotonic, never past full scale, odd.
        let mut prev = CLIP_KNEE;
        for i in 1..=400 {
            let x = CLIP_KNEE + i as f32 * 0.01;
            let y = soft_clip(x);
            assert!(y >= prev && y < 1.0 + 1e-6, "{x} -> {y}");
            assert_eq!(soft_clip(-x), -y);
            prev = y;
        }
        assert!((soft_clip(CLIP_KNEE + 1e-4) - (CLIP_KNEE + 1e-4)).abs() < 1e-5, "slope 1 at the knee");
        assert!(soft_clip(1.0) < 1.0 && soft_clip(1.0) > 0.97);
    }

    /// Level (dB) of a sustained organ note on band channel 11 after `setup`, through `apply`.
    fn level(font: &Arc<SoundFont>, setup: &[Msg], vel: u8) -> f64 {
        let mut synth = Synthesizer::new(font, &SynthesizerSettings::new(48_000)).unwrap();
        let mut player = Synthesizer::new(font, &SynthesizerSettings::new(48_000)).unwrap();
        let mut bank = [0u8; 16];
        for m in [[0xCA, 16, 0]].iter().chain(setup).chain(&[[0x9A, 60, vel]]) {
            apply(&mut synth, &mut player, m, &mut bank);
        }
        let (mut l, mut r) = (vec![0f32; 4800], vec![0f32; 4800]);
        synth.render(&mut l, &mut r); // attack
        synth.render(&mut l, &mut r);
        let e: f64 = l.iter().zip(&r).map(|(a, b)| (a * a + b * b) as f64).sum();
        10.0 * (e / l.len() as f64).log10()
    }

    #[test]
    fn velocity_cc7_cc11_follow_gm_curves() {
        let sf2 = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("soundfonts/GeneralUser-GS.sf2");
        if !sf2.exists() {
            eprintln!("soundfont missing; skipping");
            return;
        }
        let font = Arc::new(SoundFont::new(&mut std::fs::File::open(&sf2).unwrap()).unwrap());
        let gm = |v: f64| 40.0 * (v / 127.0).log10();
        let full = level(&font, &[[0xBA, 7, 127], [0xBA, 11, 127]], 127);
        let cases: [(&[Msg], u8, f64); 6] = [
            (&[[0xBA, 7, 64], [0xBA, 11, 127]], 127, gm(64.0)),
            (&[[0xBA, 7, 100], [0xBA, 11, 127]], 127, gm(100.0)),
            (&[[0xBA, 7, 32], [0xBA, 11, 127]], 127, gm(32.0)),
            (&[[0xBA, 7, 127], [0xBA, 11, 64]], 127, gm(64.0)),
            (&[[0xBA, 7, 127], [0xBA, 11, 127]], 64, gm(64.0)),
            (&[[0xBA, 7, 100], [0xBA, 11, 90]], 80, gm(100.0) + gm(90.0) + gm(80.0)),
        ];
        for (setup, vel, want) in cases {
            let got = level(&font, setup, vel) - full;
            assert!((got - want).abs() < 0.2, "{setup:?} vel {vel}: {got:.2} dB, GM says {want:.2} dB");
        }
    }
}

#[cfg(test)]
mod loudness_probe {
    use super::*;
    use crate::engine::Prepared;
    use crate::sim::{run, Step};
    use crate::sff::Style;
    use crate::theory::Chord;

    #[test]
    #[ignore]
    fn part_loudness() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let font = Arc::new(SoundFont::new(&mut std::fs::File::open(root.join("soundfonts/GeneralUser-GS.sf2")).unwrap()).unwrap());
        let mut files: Vec<_> = std::fs::read_dir(root.join("corpus/MOX_v2")).unwrap().flatten().map(|e| e.path())
            .filter(|p| p.extension().map_or(false, |x| x.eq_ignore_ascii_case("sty"))).collect();
        files.sort();
        for f in &files {
            let style = Style::load(f).unwrap();
            let prep = Box::new(Prepared::new(&style));
            let bar = (60e9 / prep.bpm * (prep.tpb as f64 / prep.ppq as f64)) as u64;
            let script: Vec<(u64, Step)> = (0..4).map(|i| (i * bar, Step::Chord(Chord::new([0, 9, 5, 7][i as usize], 0)))).collect();
            let (_, rec) = run(prep, &script, 4 * bar);
            let mut line = format!("{:<28}", f.file_name().unwrap().to_string_lossy().chars().take(27).collect::<String>());
            for part in 8..16u8 {
                let notes = rec.out.iter().filter(|(_, m)| m[0] == 0x90 | part).count();
                if notes == 0 { line.push_str("        -        "); continue; }
                let cc = |n: u8| rec.out.iter().rev().find(|(_, m)| m[0] == 0xB0 | part && m[1] == n).map(|(_, m)| m[2] as i32).unwrap_or(-1);
                let vel: f64 = rec.out.iter().filter(|(_, m)| m[0] == 0x90 | part).map(|(_, m)| m[2] as f64).sum::<f64>() / notes as f64;
                let mut synth = Synthesizer::new(&font, &SynthesizerSettings::new(48000)).unwrap();
                let mut player = Synthesizer::new(&font, &SynthesizerSettings::new(48000)).unwrap();
                synth.process_midi_message(8, 0xB0, 0, 128);
                let mut bank = [0u8; 16];
                let (mut l, mut r) = (vec![0f32; 64], vec![0f32; 64]);
                let (mut t, mut i, mut e, mut n) = (0u64, 0usize, 0f64, 0u64);
                while t < 4 * bar * 48000 / 1_000_000_000 {
                    let now = t * 1_000_000_000 / 48000;
                    while i < rec.out.len() && rec.out[i].0 <= now {
                        let m = &rec.out[i].1;
                        if !matches!(m[0] & 0xF0, 0x80 | 0x90) || m[0] & 0xF == part {
                            let mut a = [0u8; 3]; a[..m.len().min(3)].copy_from_slice(&m[..m.len().min(3)]);
                            apply(&mut synth, &mut player, &a, &mut bank);
                        }
                        i += 1;
                    }
                    synth.render(&mut l, &mut r);
                    for k in 0..64 { e += (l[k] * l[k] + r[k] * r[k]) as f64; }
                    n += 64; t += 64;
                }
                let db = 10.0 * ((e / n as f64).max(1e-12)).log10();
                line.push_str(&format!(" {:>5.1}dB v{:>3}e{:>3}vl{:>3.0}", db, cc(7), cc(11), vel));
            }
            // The whole band as the audio callback renders it: master at unity, reverb and
            // chorus on (rustysynth's default), peak before the safety clipper.
            let mut synth = Synthesizer::new(&font, &SynthesizerSettings::new(48000)).unwrap();
            let mut player = Synthesizer::new(&font, &SynthesizerSettings::new(48000)).unwrap();
            synth.set_master_volume(master_gain(MASTER_UNITY));
            synth.process_midi_message(8, 0xB0, 0, 128);
            let mut bank = [0u8; 16];
            let (mut l, mut r) = (vec![0f32; 64], vec![0f32; 64]);
            let (mut t, mut i, mut peak) = (0u64, 0usize, 0f32);
            while t < 4 * bar * 48000 / 1_000_000_000 {
                let now = t * 1_000_000_000 / 48000;
                while i < rec.out.len() && rec.out[i].0 <= now {
                    let m = &rec.out[i].1;
                    let mut a = [0u8; 3];
                    a[..m.len().min(3)].copy_from_slice(&m[..m.len().min(3)]);
                    apply(&mut synth, &mut player, &a, &mut bank);
                    i += 1;
                }
                synth.render(&mut l, &mut r);
                peak = l.iter().chain(&r).fold(peak, |p, x| p.max(x.abs()));
                t += 64;
            }
            line.push_str(&format!("  mix peak {:>5.1} dBFS", 20.0 * peak.max(1e-9).log10()));
            println!("{line}");
        }
    }
}
