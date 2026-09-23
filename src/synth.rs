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

pub const SLOTS: usize = 8;
/// Default voice slots (GM programs): Piano, E.Piano, Organ, Strings, Brass, Pad, Guitar, Lead.
pub const DEFAULT_SLOTS: [u8; SLOTS] = [0, 4, 16, 48, 61, 89, 27, 81];

/// Knobs the UI and input thread can turn without a ring: plain atomics.
///
/// Your playing has its own synthesizer instance: right hand (MIDI ch 1) on eight voice
/// slots (its channels 0-7, `active` = the layered set), left hand (MIDI ch 2) on the Left
/// voice (its channel 8). The band plays on a second instance, channels 9-16.
pub struct SynthControl {
    pub slots: [AtomicU8; SLOTS],
    /// Bitmask of layered slots.
    pub active: AtomicU8,
    /// Slot that 9/0 re-voice.
    pub focus: AtomicU8,
    /// Tapping a slot toggles it in/out of the layer instead of selecting it alone.
    pub layer_mode: AtomicBool,
    pub slots_changed: AtomicBool,
    pub master: AtomicU8,
    /// Left voice on: the left-hand (chord zone) notes sound on the Left voice.
    pub lh_sound: AtomicBool,
    pub left_program: AtomicU8,
    pub left_vol: AtomicU8,
    /// Octave shift + 2 (0..=4).
    pub left_oct: AtomicU8,
    pub slot_vol: [AtomicU8; SLOTS],
    /// Octave shift + 2 (0..=4) per slot.
    pub slot_oct: [AtomicU8; SLOTS],
    /// Main A-D recall One Touch Settings 1-4.
    pub ots_link: AtomicBool,
    /// Which OTS was applied last (0 = none, 1..=4).
    pub ots_applied: AtomicU8,
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

impl SynthControl {
    pub fn new(out_ch: u8) -> SynthControl {
        SynthControl {
            slots: DEFAULT_SLOTS.map(AtomicU8::new),
            active: AtomicU8::new(1),
            focus: AtomicU8::new(0),
            layer_mode: AtomicBool::new(false),
            slots_changed: AtomicBool::new(true),
            master: AtomicU8::new(100),
            lh_sound: AtomicBool::new(false),
            left_program: AtomicU8::new(48),
            left_vol: AtomicU8::new(100),
            left_oct: AtomicU8::new(2),
            slot_vol: [const { AtomicU8::new(100) }; SLOTS],
            slot_oct: [const { AtomicU8::new(2) }; SLOTS],
            ots_link: AtomicBool::new(false),
            ots_applied: AtomicU8::new(0),
            muted: AtomicBool::new(false),
            out_ch: AtomicU8::new(out_ch),
        }
    }

    /// A voice-slot button was pressed.
    pub fn press_slot(&self, i: u8) {
        let bit = 1u8 << (i & 7);
        if self.layer_mode.load(Relaxed) {
            let a = self.active.load(Relaxed) ^ bit;
            self.active.store(if a == 0 { bit } else { a }, Relaxed);
        } else {
            self.active.store(bit, Relaxed);
        }
        self.focus.store(i & 7, Relaxed);
    }

    /// Load a One Touch Setting: Right 1-3 into slots 1-3 (their on/off becomes the layer),
    /// Left into the Left voice.
    pub fn apply_ots(&self, ots: &crate::sff::Ots, number: u8) {
        let mut active = 0u8;
        for i in 0..3 {
            let part = &ots.parts[i];
            if let Some((msb, _, pc)) = part.voice {
                if msb < 126 {
                    self.slots[i].store(pc, Relaxed);
                }
            }
            self.slot_vol[i].store(part.volume, Relaxed);
            self.slot_oct[i].store((part.octave + 2) as u8, Relaxed);
            if part.on {
                active |= 1 << i;
            }
        }
        self.active.store(if active == 0 { 1 } else { active }, Relaxed);
        self.focus.store(0, Relaxed);
        let left = &ots.parts[3];
        if let Some((msb, _, pc)) = left.voice {
            if msb < 126 {
                self.left_program.store(pc, Relaxed);
            }
        }
        self.left_vol.store(left.volume, Relaxed);
        self.left_oct.store((left.octave + 2) as u8, Relaxed);
        self.lh_sound.store(left.on, Relaxed);
        self.ots_applied.store(number, Relaxed);
        self.slots_changed.store(true, Relaxed);
    }

    pub fn step_left_program(&self, delta: i32) {
        let p = (self.left_program.load(Relaxed) as i32 + delta).rem_euclid(128) as u8;
        self.left_program.store(p, Relaxed);
        self.slots_changed.store(true, Relaxed);
    }

    /// Re-voice the focused slot by `delta` programs.
    pub fn step_focus_program(&self, delta: i32) {
        let f = self.focus.load(Relaxed) as usize;
        let p = (self.slots[f].load(Relaxed) as i32 + delta).rem_euclid(128) as u8;
        self.slots[f].store(p, Relaxed);
        self.slots_changed.store(true, Relaxed);
    }
}

/// Per-note record of what a player note started (which slots, at which key after octave
/// shift), so note-offs reach the same voices even if the layer changed while held.
struct Player {
    rh: [u8; 128],
    rh_key: [[u8; SLOTS]; 128],
    lh: [u8; 128],
    master: u8,
}

impl Player {
    fn new() -> Player {
        Player { rh: [0; 128], rh_key: [[0; SLOTS]; 128], lh: [255; 128], master: 255 }
    }
}

const LEFT_CH: i32 = 8;

fn shifted(key: usize, oct: u8) -> i32 {
    (key as i32 + (oct as i32 - 2) * 12).clamp(0, 127)
}

/// Push slot/left programs, volumes to the player synth.
fn sync_player(player: &mut Synthesizer, ctl: &SynthControl) {
    for c in 0..SLOTS {
        player.process_midi_message(c as i32, 0xC0, ctl.slots[c].load(Relaxed) as i32, 0);
        player.process_midi_message(c as i32, 0xB0, 7, ctl.slot_vol[c].load(Relaxed) as i32);
    }
    player.process_midi_message(LEFT_CH, 0xC0, ctl.left_program.load(Relaxed) as i32, 0);
    player.process_midi_message(LEFT_CH, 0xB0, 7, ctl.left_vol.load(Relaxed) as i32);
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

fn apply(synth: &mut Synthesizer, player: &mut Synthesizer, m: &Msg, ctl: &SynthControl, bank: &mut [u8; 16], pl: &mut Player) {
    let ch = (m[0] & 0x0F) as i32;
    let st = (m[0] & 0xF0) as i32;
    let (k, v) = (m[1] as usize & 127, m[2] as i32);
    // Your playing goes to the player synth: right hand on the layered slots, left hand on
    // the Left voice.
    if ch <= 1 {
        let on = st == 0x90 && v > 0;
        let off = st == 0x80 || (st == 0x90 && v == 0);
        if ch == 0 {
            let active = ctl.active.load(Relaxed);
            if on {
                pl.rh[k] |= active;
                for c in 0..SLOTS {
                    if active & (1 << c) != 0 {
                        let key = shifted(k, ctl.slot_oct[c].load(Relaxed));
                        pl.rh_key[k][c] = key as u8;
                        player.note_on(c as i32, key, v);
                    }
                }
            } else if off {
                for c in 0..SLOTS {
                    if pl.rh[k] & (1 << c) != 0 {
                        player.note_off(c as i32, pl.rh_key[k][c] as i32);
                    }
                }
                pl.rh[k] = 0;
            } else {
                // Pedal, wheels, pressure: every slot, so releases always land.
                for c in 0..SLOTS as i32 {
                    player.process_midi_message(c, st, m[1] as i32, v);
                }
            }
        } else if on && ctl.lh_sound.load(Relaxed) {
            let key = shifted(k, ctl.left_oct.load(Relaxed));
            pl.lh[k] = key as u8;
            player.note_on(LEFT_CH, key, v);
        } else if off && pl.lh[k] != 255 {
            player.note_off(LEFT_CH, pl.lh[k] as i32);
            pl.lh[k] = 255;
        }
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
        _ => synth.process_midi_message(ch, st, m[1] as i32, v),
    }
}

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
    let mut player_synth = Synthesizer::new(&font, &settings).map_err(|e| anyhow!("{e:?}"))?;
    synth.set_master_volume(0.6);
    synth.process_midi_message(8, 0xB0, 0, 128);

    let control = Arc::new(SynthControl::new(first));
    let ctl = control.clone();
    let mut consumers = consumers;
    let mut bank = [0u8; 16];
    let mut player = Player::new();
    let mut left = vec![0f32; 8192];
    let mut right = vec![0f32; 8192];
    let mut left2 = vec![0f32; 8192];
    let mut right2 = vec![0f32; 8192];

    let callback = move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
        if ctl.slots_changed.swap(false, Relaxed) {
            sync_player(&mut player_synth, &ctl);
        }
        let master = ctl.master.load(Relaxed);
        if master != player.master {
            player.master = master;
            synth.set_master_volume(0.8 * master as f32 / 127.0);
            player_synth.set_master_volume(0.8 * master as f32 / 127.0);
        }
        for c in consumers.iter_mut() {
            while let Ok(m) = c.pop() {
                apply(&mut synth, &mut player_synth, &m, &ctl, &mut bank, &mut player);
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
        let ctl = SynthControl::new(0);
        let sr = 48_000;
        let mut rms_by_part = Vec::new();
        for part in 8..16u8 {
            let mut synth = Synthesizer::new(&font, &SynthesizerSettings::new(sr)).unwrap();
            let mut bank = [0u8; 16];
            let mut pl = Player::new();
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
                        apply(&mut synth, &mut player, &a, &ctl, &mut bank, &mut pl);
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
mod layer_tests {
    use super::*;

    #[test]
    fn slot_selection_and_layering() {
        let c = SynthControl::new(0);
        c.press_slot(3);
        assert_eq!(c.active.load(Relaxed), 0b1000);
        c.layer_mode.store(true, Relaxed);
        c.press_slot(0);
        assert_eq!(c.active.load(Relaxed), 0b1001);
        c.press_slot(3);
        assert_eq!(c.active.load(Relaxed), 0b0001);
        c.press_slot(0); // can't remove the last voice
        assert_eq!(c.active.load(Relaxed), 0b0001);
        c.layer_mode.store(false, Relaxed);
        c.press_slot(5);
        assert_eq!(c.active.load(Relaxed), 0b100000);
    }

    /// A note held while the layer changes must still be released on the voices it started on.
    #[test]
    fn note_off_follows_note_on_voices() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let sf2 = root.join("soundfonts/GeneralUser-GS.sf2");
        if !sf2.exists() {
            return;
        }
        let font = Arc::new(SoundFont::new(&mut std::fs::File::open(&sf2).unwrap()).unwrap());
        let mut synth = Synthesizer::new(&font, &SynthesizerSettings::new(48_000)).unwrap();
        let mut player = Synthesizer::new(&font, &SynthesizerSettings::new(48_000)).unwrap();
        let ctl = SynthControl::new(0);
        let mut bank = [0u8; 16];
        let mut pl = Player::new();
        ctl.layer_mode.store(true, Relaxed);
        ctl.press_slot(3); // piano + strings
        ctl.slot_oct[3].store(1, Relaxed); // strings an octave down
        apply(&mut synth, &mut player, &[0x90, 60, 100], &ctl, &mut bank, &mut pl);
        assert_eq!(pl.rh[60], 0b1001);
        assert_eq!(pl.rh_key[60][3], 48);
        ctl.layer_mode.store(false, Relaxed);
        ctl.press_slot(5); // layer changed while held
        ctl.slot_oct[3].store(2, Relaxed); // and the octave
        apply(&mut synth, &mut player, &[0x80, 60, 0], &ctl, &mut bank, &mut pl);
        assert_eq!(pl.rh[60], 0);
        // Left voice.
        ctl.lh_sound.store(true, Relaxed);
        apply(&mut synth, &mut player, &[0x91, 40, 90], &ctl, &mut bank, &mut pl);
        assert_eq!(pl.lh[40], 40);
        ctl.lh_sound.store(false, Relaxed); // turned off while held: still released
        apply(&mut synth, &mut player, &[0x81, 40, 0], &ctl, &mut bank, &mut pl);
        assert_eq!(pl.lh[40], 255);
    }

    #[test]
    fn ots_applies_to_slots_and_left() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            return;
        }
        let style = crate::sff::Style::load(&p).unwrap();
        let c = SynthControl::new(0);
        c.apply_ots(&style.ots[0], 1);
        assert_eq!(c.active.load(Relaxed), 0b011); // Right 1 + Right 2
        assert_eq!(c.slots[0].load(Relaxed), 80);
        assert_eq!(c.slots[1].load(Relaxed), 94);
        assert!(c.lh_sound.load(Relaxed));
        assert_eq!(c.left_program.load(Relaxed), 52);
        assert_eq!(c.left_vol.load(Relaxed), 40);
    }
}
