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
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering::Relaxed};
use std::sync::Arc;

pub type Msg = [u8; 3];

pub struct SynthInfo {
    pub name: String,
    pub sample_rate: u32,
    pub buffer: Option<u32>,
    pub device: String,
    /// Number of output channels on the device.
    pub channels: usize,
}

/// Knobs the UI can turn without talking to the audio thread through a ring.
pub struct SynthControl {
    /// Program for the right-hand part (channel 1).
    pub rh_program: AtomicU8,
    pub rh_changed: AtomicBool,
    /// Whether the left-hand (chord zone) notes sound.
    pub lh_sound: AtomicBool,
    pub muted: AtomicBool,
    /// First (left) output channel of the stereo pair, 0-based.
    pub out_ch: AtomicU8,
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

fn apply(synth: &mut Synthesizer, m: &Msg, control: &SynthControl, bank: &mut [u8; 16]) {
    let ch = (m[0] & 0x0F) as i32;
    let st = (m[0] & 0xF0) as i32;
    if ch == 1 && (st == 0x90 || st == 0x80) && !control.lh_sound.load(Relaxed) {
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
        _ => synth.process_midi_message(ch, st, m[1] as i32, m[2] as i32),
    }
}

/// `out_pair`: 1-based left output channel (e.g. 11 for outputs 11/12); None = auto
/// (11/12 on a TASCAM Model 16, else 1/2).
pub fn start(sf2: &Path, consumers: Vec<Consumer<Msg>>, out_pair: Option<u8>) -> Result<Synth> {
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
    synth.set_master_volume(0.6);
    synth.process_midi_message(8, 0xB0, 0, 128);

    let control = Arc::new(SynthControl {
        rh_program: AtomicU8::new(0),
        rh_changed: AtomicBool::new(true),
        lh_sound: AtomicBool::new(false),
        muted: AtomicBool::new(false),
        out_ch: AtomicU8::new(first),
    });
    let ctl = control.clone();
    let mut consumers = consumers;
    let mut bank = [0u8; 16];
    let mut left = vec![0f32; 8192];
    let mut right = vec![0f32; 8192];

    let callback = move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
        if ctl.rh_changed.swap(false, Relaxed) {
            synth.process_midi_message(0, 0xC0, ctl.rh_program.load(Relaxed) as i32, 0);
        }
        for c in consumers.iter_mut() {
            while let Ok(m) = c.pop() {
                apply(&mut synth, &m, &ctl, &mut bank);
            }
        }
        let frames = (out.len() / channels).min(left.len());
        synth.render(&mut left[..frames], &mut right[..frames]);
        let mute = ctl.muted.load(Relaxed);
        let lc = (ctl.out_ch.load(Relaxed) as usize).min(channels.saturating_sub(1));
        let rc = (lc + 1).min(channels - 1);
        for (i, frame) in out.chunks_mut(channels).take(frames).enumerate() {
            frame.fill(0.0);
            if !mute {
                frame[lc] += left[i];
                frame[rc] += right[i];
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
        let ctl = SynthControl {
            rh_program: AtomicU8::new(0),
            rh_changed: AtomicBool::new(false),
            lh_sound: AtomicBool::new(false),
            muted: AtomicBool::new(false),
            out_ch: AtomicU8::new(0),
        };
        let sr = 48_000;
        let mut rms_by_part = Vec::new();
        for part in 8..16u8 {
            let mut synth = Synthesizer::new(&font, &SynthesizerSettings::new(sr)).unwrap();
            let mut bank = [0u8; 16];
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
                        apply(&mut synth, &a, &ctl, &mut bank);
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
