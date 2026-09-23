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
    /// Manual Bass in effect: the left hand sounds, on the Style's Bass voice.
    pub manual_bass: AtomicBool,
    /// GM program for the current Style's Bass part (see `style_bass_program`).
    pub bass_program: AtomicU8,
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
    /// The Launchkey master fader is waiting to pick up `master` (soft takeover).
    pub master_waiting: AtomicBool,
    /// The Style's Bass part fader (its CC7): the Left part's volume under Manual Bass.
    pub bass_vol: AtomicU8,
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
            master: AtomicU8::new(MASTER_UNITY),
            lh_sound: AtomicBool::new(false),
            left_program: AtomicU8::new(48),
            left_vol: AtomicU8::new(100),
            left_oct: AtomicU8::new(2),
            manual_bass: AtomicBool::new(false),
            bass_program: AtomicU8::new(33),
            slot_vol: [const { AtomicU8::new(100) }; SLOTS],
            slot_oct: [const { AtomicU8::new(2) }; SLOTS],
            ots_link: AtomicBool::new(false),
            ots_applied: AtomicU8::new(0),
            muted: AtomicBool::new(false),
            out_ch: AtomicU8::new(out_ch),
            master_waiting: AtomicBool::new(false),
            bass_vol: AtomicU8::new(100),
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

    /// The CC7 of your two parts, as sent on their channels (right hand ch 1, left hand
    /// ch 2): the right hand's is the lowest layered slot's volume (Right 1 when it is on),
    /// the left hand's the Left volume, or the Style's Bass fader under Manual Bass.
    pub fn port_volumes(&self) -> (u8, u8) {
        let active = self.active.load(Relaxed);
        let slot = if active == 0 { 0 } else { active.trailing_zeros() as usize & (SLOTS - 1) };
        let rh = self.slot_vol[slot].load(Relaxed);
        let lh = if self.manual_bass.load(Relaxed) { self.bass_vol.load(Relaxed) } else { self.left_vol.load(Relaxed) };
        (rh, lh)
    }

    /// The Style's Bass fader moved (engine thread): under Manual Bass the Left part follows.
    pub fn set_bass_vol(&self, v: u8) {
        if self.bass_vol.swap(v, Relaxed) != v && self.manual_bass.load(Relaxed) {
            self.slots_changed.store(true, Relaxed);
        }
    }

    /// Manual Bass on/off: the Left part switches between its own voice and level and the Style's Bass voice and fader.
    pub fn set_manual_bass(&self, on: bool) {
        self.manual_bass.store(on, Relaxed);
        self.slots_changed.store(true, Relaxed);
    }

    /// A new Style is loaded: its Bass voice is what Manual Bass plays.
    pub fn set_bass_program(&self, prog: u8) {
        self.bass_program.store(prog, Relaxed);
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
    let left = if ctl.manual_bass.load(Relaxed) { &ctl.bass_program } else { &ctl.left_program };
    player.process_midi_message(LEFT_CH, 0xC0, left.load(Relaxed) as i32, 0);
    player.process_midi_message(LEFT_CH, 0xB0, 7, ctl.port_volumes().1 as i32);
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
        } else if on && (ctl.lh_sound.load(Relaxed) || ctl.manual_bass.load(Relaxed)) {
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
        // Everything else as sent: CC7 (the mixer fader), CC11 and velocity reach the voice
        // unchanged, so the SoundFont answers them exactly as an external GM instrument would.
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
            synth.set_master_volume(master_gain(master));
            player_synth.set_master_volume(master_gain(master));
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

    /// Manual Bass: the left hand sounds even with the Left voice off.
    #[test]
    fn manual_bass_sounds_the_left_hand() {
        assert_eq!(style_bass_program(Some((0, 0, 35))), 35);
        assert_eq!(style_bass_program(Some((8, 0, 4))), 33); // Genos-only bank, not a bass number
        assert_eq!(style_bass_program(None), 33);
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let sf2 = root.join("soundfonts/GeneralUser-GS.sf2");
        if !sf2.exists() {
            eprintln!("soundfont missing; skipping");
            return;
        }
        let font = Arc::new(SoundFont::new(&mut std::fs::File::open(&sf2).unwrap()).unwrap());
        let mut synth = Synthesizer::new(&font, &SynthesizerSettings::new(48_000)).unwrap();
        let mut player = Synthesizer::new(&font, &SynthesizerSettings::new(48_000)).unwrap();
        let ctl = SynthControl::new(0);
        let (mut bank, mut pl) = ([0u8; 16], Player::new());
        apply(&mut synth, &mut player, &[0x91, 40, 90], &ctl, &mut bank, &mut pl);
        assert_eq!(pl.lh[40], 255, "Left voice off: silent");
        ctl.set_manual_bass(true);
        assert!(ctl.slots_changed.load(Relaxed));
        apply(&mut synth, &mut player, &[0x91, 40, 90], &ctl, &mut bank, &mut pl);
        assert_eq!(pl.lh[40], 40);
        ctl.set_manual_bass(false); // turned off while held: still released
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

    /// Your parts' CC7 for the port: the lowest layered slot, the Left volume, and under
    /// Manual Bass the Style's Bass fader.
    #[test]
    fn port_volumes_follow_slots_left_and_manual_bass() {
        let c = SynthControl::new(0);
        assert_eq!(c.port_volumes(), (100, 100));
        c.slot_vol[0].store(80, Relaxed);
        c.slot_vol[3].store(60, Relaxed);
        c.left_vol.store(40, Relaxed);
        assert_eq!(c.port_volumes(), (80, 40));
        c.press_slot(3);
        assert_eq!(c.port_volumes(), (60, 40));
        c.slots_changed.store(false, Relaxed);
        c.set_bass_vol(90); // Manual Bass off: the Left part keeps its own level
        assert!(!c.slots_changed.load(Relaxed));
        c.set_manual_bass(true);
        assert_eq!(c.port_volumes(), (60, 90));
        c.slots_changed.store(false, Relaxed);
        c.set_bass_vol(70);
        assert!(c.slots_changed.load(Relaxed), "the player synth's Left channel follows the Bass fader");
        assert_eq!(c.port_volumes().1, 70);
    }

    /// Level (dB) of a sustained organ note on band channel 11 after `setup`, through `apply`.
    fn level(font: &Arc<SoundFont>, setup: &[Msg], vel: u8) -> f64 {
        let mut synth = Synthesizer::new(font, &SynthesizerSettings::new(48_000)).unwrap();
        let mut player = Synthesizer::new(font, &SynthesizerSettings::new(48_000)).unwrap();
        let ctl = SynthControl::new(0);
        let (mut bank, mut pl) = ([0u8; 16], Player::new());
        for m in [[0xCA, 16, 0]].iter().chain(setup).chain(&[[0x9A, 60, vel]]) {
            apply(&mut synth, &mut player, m, &ctl, &mut bank, &mut pl);
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
                let ctl = SynthControl::new(0);
                let (mut bank, mut pl) = ([0u8; 16], Player::new());
                let (mut l, mut r) = (vec![0f32; 64], vec![0f32; 64]);
                let (mut t, mut i, mut e, mut n) = (0u64, 0usize, 0f64, 0u64);
                while t < 4 * bar * 48000 / 1_000_000_000 {
                    let now = t * 1_000_000_000 / 48000;
                    while i < rec.out.len() && rec.out[i].0 <= now {
                        let m = &rec.out[i].1;
                        if !matches!(m[0] & 0xF0, 0x80 | 0x90) || m[0] & 0xF == part {
                            let mut a = [0u8; 3]; a[..m.len().min(3)].copy_from_slice(&m[..m.len().min(3)]);
                            apply(&mut synth, &mut player, &a, &ctl, &mut bank, &mut pl);
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
            let ctl = SynthControl::new(0);
            let (mut bank, mut pl) = ([0u8; 16], Player::new());
            let (mut l, mut r) = (vec![0f32; 64], vec![0f32; 64]);
            let (mut t, mut i, mut peak) = (0u64, 0usize, 0f32);
            while t < 4 * bar * 48000 / 1_000_000_000 {
                let now = t * 1_000_000_000 / 48000;
                while i < rec.out.len() && rec.out[i].0 <= now {
                    let m = &rec.out[i].1;
                    let mut a = [0u8; 3];
                    a[..m.len().min(3)].copy_from_slice(&m[..m.len().min(3)]);
                    apply(&mut synth, &mut player, &a, &ctl, &mut bank, &mut pl);
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
