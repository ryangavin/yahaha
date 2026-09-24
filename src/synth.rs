//! Built-in SoundFont player: rustysynth rendering inside CoreAudio's IO callback (via cpal).
//!
//! MIDI reaches the audio thread through SPSC rings (one per producer thread), drained at
//! the start of each buffer. With a 64-frame buffer at 48 kHz, an event waits at most
//! 1.3 ms before it is rendered.
//!
//! The band and your playing each have a synthesizer (a [`Rack`]), with the same
//! SoundFont. They measure each channel's level as they mix it, for the meters. A new
//! SoundFont is loaded into a new rack off the audio thread and swapped in between two
//! buffers (`SetSoundFont`).

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rtrb::{Consumer, Producer, RingBuffer};
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering::{Acquire, Relaxed}};
use std::sync::Arc;

use crate::parts::{self, Parts};

pub type Msg = [u8; 3];

#[derive(Clone, Debug)]
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
    /// Peak level per MIDI channel since the last `take_meters` (f32 bits; a positive
    /// f32's bits order as the value does, so `fetch_max` works on them).
    pub peaks: [AtomicU32; 16],
    /// Left and right peaks after the soft clipper.
    pub master_peaks: [AtomicU32; 2],
    /// Buffers in which the soft clipper worked.
    pub clips: AtomicU64,
    /// Racks swapped in (`SetSoundFont`).
    pub swaps: AtomicU64,
}

pub struct Synth {
    _stream: cpal::Stream,
    pub info: SynthInfo,
    pub control: Arc<SynthControl>,
    /// The ends of the rings that swap racks: new ones to the audio thread, old ones back
    /// to be freed off it. Taken by whoever runs `SetSoundFont`.
    pub swap: Option<RackSwap>,
}

/// Swapping SoundFonts: `tx` hands a new rack to the audio thread, `old` brings back the
/// one it replaced (drop it there, not on the audio thread).
pub struct RackSwap {
    pub tx: Producer<Box<Rack>>,
    pub old: Consumer<Box<Rack>>,
}

/// The channels the meters report: the keyboard parts and the Style parts.
pub const RACK_CHANNELS: [u8; 12] = [0, 1, 2, 3, 8, 9, 10, 11, 12, 13, 14, 15];

/// The built-in synth's synthesizers, one SoundFont: the band's (ch 9-16) and your
/// playing's (the keyboard parts, ch 1-4), 128 voices each. Each measures its channels'
/// levels as it mixes them (`Synthesizer::channel_peaks`, a patch in vendor/rustysynth),
/// so the meters cost no extra synthesizers. (A synthesizer per part would run each
/// part's reverb and chorus again and give every part 128 voices of its own: up to 12 x
/// 128 voices on the audio thread, about 1 ms of a 64-frame buffer's 1.33 ms.)
pub struct Rack {
    band: Synthesizer,
    player: Synthesizer,
    tmp_l: Vec<f32>,
    tmp_r: Vec<f32>,
}

impl Rack {
    pub fn new(font: &Arc<SoundFont>, sample_rate: i32) -> Result<Rack> {
        let mut settings = SynthesizerSettings::new(sample_rate);
        settings.maximum_polyphony = 128;
        let band = Synthesizer::new(font, &settings).map_err(|e| anyhow!("{e:?}"))?;
        let player = Synthesizer::new(font, &settings).map_err(|e| anyhow!("{e:?}"))?;
        let mut r = Rack { band, player, tmp_l: vec![0.0; 8192], tmp_r: vec![0.0; 8192] };
        // Rhythm 1 (ch 9) is a drum part too: on the drum bank.
        r.process(8, 0xB0, 0, 128);
        Ok(r)
    }

    /// Load a SoundFont into a new rack (slow: call it off the audio thread).
    pub fn load(sf2: &Path, sample_rate: u32) -> Result<Box<Rack>> {
        let mut file = std::fs::File::open(sf2).with_context(|| format!("opening {}", sf2.display()))?;
        let font = Arc::new(SoundFont::new(&mut file).map_err(|e| anyhow!("{e:?}"))?);
        Ok(Box::new(Rack::new(&font, sample_rate as i32)?))
    }

    /// A channel message to the synthesizer that plays the channel: the keyboard parts'
    /// to the player's, everything else to the band's.
    #[inline]
    fn process(&mut self, ch: i32, st: i32, d1: i32, d2: i32) {
        if parts::part_of_channel(ch as u8).is_some() {
            self.player.process_midi_message(ch, st, d1, d2);
        } else {
            self.band.process_midi_message(ch, st, d1, d2);
        }
    }

    fn set_master_volume(&mut self, v: f32) {
        self.band.set_master_volume(v);
        self.player.set_master_volume(v);
    }

    /// Render `left.len()` frames of the mix into `left`/`right` (overwritten), noting each
    /// channel's peak in `peaks`. `fade` ramps the whole from one gain to another over the
    /// buffer.
    fn render(&mut self, left: &mut [f32], right: &mut [f32], peaks: &[AtomicU32; 16], fade: Option<(f32, f32)>) {
        let n = left.len().min(self.tmp_l.len());
        let (left, right) = (&mut left[..n], &mut right[..n]);
        self.band.render(left, right);
        let (l, r) = (&mut self.tmp_l[..n], &mut self.tmp_r[..n]);
        self.player.render(l, r);
        for k in 0..n {
            left[k] += l[k];
            right[k] += r[k];
        }
        let mut most = 1f32;
        if let Some((a, b)) = fade {
            most = a.max(b);
            for k in 0..n {
                let g = a + (b - a) * k as f32 / n as f32;
                left[k] *= g;
                right[k] *= g;
            }
        }
        for s in [&mut self.band, &mut self.player] {
            for (ch, &p) in s.channel_peaks().iter().enumerate() {
                if p > 0.0 {
                    peaks[ch].fetch_max((p * most).to_bits(), Relaxed);
                }
            }
            s.reset_channel_peaks();
        }
    }
}

/// What the audio thread last sent each channel, so a new rack (`SetSoundFont`) takes over
/// with the same voices and controllers.
struct Shadow {
    cc: [[u8; 128]; 16],
    program: [Option<u8>; 16],
    bend: [Option<(u8, u8)>; 16],
    /// Data entry (CC6, CC38) per channel for RPN 0-2 (bend range, fine and coarse tune).
    /// Data entry reaches the RPN selected when it comes, so it is kept by RPN.
    rpn: [[(u8, u8); 3]; 16],
    /// An NRPN (CC98/99) was selected after the last RPN: data entry goes nowhere.
    nrpn: [bool; 16],
}

const NO_CC: u8 = 0xFF;

impl Shadow {
    fn new() -> Shadow {
        Shadow { cc: [[NO_CC; 128]; 16], program: [None; 16], bend: [None; 16], rpn: [[(NO_CC, NO_CC); 3]; 16], nrpn: [false; 16] }
    }

    fn note(&mut self, m: &Msg) {
        let ch = (m[0] & 0x0F) as usize;
        match m[0] & 0xF0 {
            // Channel mode messages (120-127) are actions, not settings.
            0xB0 if m[1] < 120 => {
                let cc = m[1] as usize & 127;
                self.cc[ch][cc] = m[2];
                match cc {
                    98 | 99 => self.nrpn[ch] = true,
                    100 | 101 => self.nrpn[ch] = false,
                    6 | 38 => {
                        let (msb, lsb) = (self.cc[ch][101], self.cc[ch][100]);
                        if !self.nrpn[ch] && msb == 0 && lsb < 3 {
                            let e = &mut self.rpn[ch][lsb as usize];
                            if cc == 6 { e.0 = m[2] } else { e.1 = m[2] }
                        }
                    }
                    _ => {}
                }
            }
            0xC0 => self.program[ch] = Some(m[1]),
            0xE0 => self.bend[ch] = Some((m[1], m[2])),
            _ => {}
        }
    }

    /// Bring `rack` to what the channels have: bank and voice, controllers, pitch bend.
    fn replay(&self, rack: &mut Rack, bank: &mut [u8; 16], parts: &Parts) {
        for ch in RACK_CHANNELS {
            let c = ch as usize;
            if self.cc[c][0] != NO_CC {
                apply_rack(rack, &[0xB0 | ch, 0, self.cc[c][0]], bank);
            }
            if let Some(p) = self.program[c] {
                apply_rack(rack, &[0xC0 | ch, p, 0], bank);
            }
            // Data entry and parameter selects are not settings of their own: the RPNs'
            // values go out under their own select, then the select the channel had.
            for (n, &(msb, lsb)) in self.rpn[c].iter().enumerate() {
                if msb == NO_CC && lsb == NO_CC {
                    continue;
                }
                apply_rack(rack, &[0xB0 | ch, 101, 0], bank);
                apply_rack(rack, &[0xB0 | ch, 100, n as u8], bank);
                if msb != NO_CC {
                    apply_rack(rack, &[0xB0 | ch, 6, msb], bank);
                }
                if lsb != NO_CC {
                    apply_rack(rack, &[0xB0 | ch, 38, lsb], bank);
                }
            }
            for cc in 1..120u8 {
                let v = self.cc[c][cc as usize];
                if v != NO_CC && !matches!(cc, 6 | 32 | 38 | 98..=101) {
                    apply_rack(rack, &[0xB0 | ch, cc, v], bank);
                }
            }
            let select = if self.nrpn[c] { [99, 98] } else { [101, 100] };
            for cc in select {
                if self.cc[c][cc as usize] != NO_CC {
                    apply_rack(rack, &[0xB0 | ch, cc, self.cc[c][cc as usize]], bank);
                }
            }
            if let Some((lo, hi)) = self.bend[c] {
                apply_rack(rack, &[0xE0 | ch, lo, hi], bank);
            }
        }
        sync_player_rack(rack, parts);
    }
}

/// Take the meters: each channel's and the master's peak since the last take, and the
/// clip count. For one reader.
pub fn take_meters(c: &SynthControl) -> ([f32; 16], [f32; 2], u64) {
    let peaks = std::array::from_fn(|i| f32::from_bits(c.peaks[i].swap(0, Relaxed)));
    let master = std::array::from_fn(|i| f32::from_bits(c.master_peaks[i].swap(0, Relaxed)));
    (peaks, master, c.clips.load(Relaxed))
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
            peaks: std::array::from_fn(|_| AtomicU32::new(0)),
            master_peaks: std::array::from_fn(|_| AtomicU32::new(0)),
            clips: AtomicU64::new(0),
            swaps: AtomicU64::new(0),
        }
    }
}

/// Push the keyboard parts' programs to the player synth. Their volumes arrive as CC7 on
/// their channels, like any other message.
#[cfg(test)]
fn sync_player(player: &mut Synthesizer, parts: &Parts) {
    for p in 0..parts::COUNT {
        player.process_midi_message(parts::CHANNEL[p] as i32, 0xC0, parts.channel_program(p) as i32, 0);
    }
}

fn sync_player_rack(rack: &mut Rack, parts: &Parts) {
    for p in 0..parts::COUNT {
        rack.process(parts::CHANNEL[p] as i32, 0xC0, parts.channel_program(p) as i32, 0);
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

/// A message as the SoundFont gets it: `out(channel, status, data1, data2)` for each
/// channel message to process (none, one, or two).
#[inline]
fn translate(m: &Msg, bank: &mut [u8; 16], mut out: impl FnMut(i32, i32, i32, i32)) {
    let ch = (m[0] & 0x0F) as i32;
    let st = (m[0] & 0xF0) as i32;
    let v = m[2] as i32;
    // Your playing: each keyboard part on its own channel, as sent. The input thread
    // already decided which parts sound and where (on/off, octave).
    if parts::part_of_channel(ch as u8).is_some() {
        out(ch, st, m[1] as i32, v);
        return;
    }
    match st {
        // Style bank selects are Yamaha banks; the SoundFont gets GM banks instead.
        0xB0 if m[1] == 0 => bank[ch as usize] = m[2],
        0xB0 if m[1] == 32 => {}
        // Rhythm 1 (ch 9) is a drum part too: keep it on the drum bank.
        0xC0 if ch == 8 => {
            out(8, 0xB0, 0, 128);
            out(8, 0xC0, m[1] as i32, 0);
        }
        0xC0 => {
            let p = gm_fallback(ch as u8, bank[ch as usize], m[1]);
            out(ch, 0xC0, p as i32, 0);
        }
        // Everything else as sent: CC7 (the mixer fader), CC11 and velocity reach the voice
        // unchanged, so the SoundFont answers them exactly as an external GM instrument would.
        _ => out(ch, st, m[1] as i32, v),
    }
}

/// A message to a band synth and a player synth (the keyboard parts' channels).
#[cfg(test)]
fn apply(synth: &mut Synthesizer, player: &mut Synthesizer, m: &Msg, bank: &mut [u8; 16]) {
    translate(m, bank, |ch, st, a, b| {
        if parts::part_of_channel(ch as u8).is_some() {
            player.process_midi_message(ch, st, a, b)
        } else {
            synth.process_midi_message(ch, st, a, b)
        }
    });
}

/// A message to a rack: each channel to its own synthesizer.
#[inline]
fn apply_rack(rack: &mut Rack, m: &Msg, bank: &mut [u8; 16]) {
    translate(m, bank, |ch, st, a, b| rack.process(ch, st, a, b));
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

    let mut rack = Box::new(Rack::new(&font, sample_rate as i32)?);
    drop(font);
    let (swap_tx, mut swap_rx) = RingBuffer::<Box<Rack>>::new(2);
    let (mut old_tx, old_rx) = RingBuffer::<Box<Rack>>::new(4);

    let control = Arc::new(SynthControl::new(first));
    let ctl = control.clone();
    let mut consumers = consumers;
    let mut bank = [0u8; 16];
    let mut shadow = Box::new(Shadow::new());
    let mut last_master = 255u8;
    let mut left = vec![0f32; 8192];
    let mut right = vec![0f32; 8192];
    let mut left2 = vec![0f32; 8192];
    let mut right2 = vec![0f32; 8192];
    // A rack just replaced: it plays out one buffer, fading, then goes back to be freed.
    let mut fading: Option<Box<Rack>> = None;
    let unmetered: [AtomicU32; 16] = std::array::from_fn(|_| AtomicU32::new(0));

    let callback = move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
        // A new SoundFont: the new rack takes over with the channels' voices and controllers.
        if let Ok(mut new) = swap_rx.pop() {
            shadow.replay(&mut new, &mut bank, &parts);
            let old = std::mem::replace(&mut rack, new);
            if let Some(f) = fading.replace(old) {
                let _ = old_tx.push(f);
            }
            last_master = 255;
            ctl.swaps.fetch_add(1, Relaxed);
        }
        // Acquire pairs with the Release stores in `Parts`: the new programs are visible.
        if parts.changed.swap(false, Acquire) {
            sync_player_rack(&mut rack, &parts);
        }
        let master = ctl.master.load(Relaxed);
        if master != last_master {
            last_master = master;
            rack.set_master_volume(master_gain(master));
            if let Some(f) = fading.as_mut() {
                f.set_master_volume(master_gain(master));
            }
        }
        for c in consumers.iter_mut() {
            while let Ok(m) = c.pop() {
                shadow.note(&m);
                apply_rack(&mut rack, &m, &mut bank);
            }
        }
        let frames = (out.len() / channels).min(left.len());
        rack.render(&mut left[..frames], &mut right[..frames], &ctl.peaks, None);
        if let Some(mut f) = fading.take() {
            f.render(&mut left2[..frames], &mut right2[..frames], &unmetered, Some((1.0, 0.0)));
            for i in 0..frames {
                left[i] += left2[i];
                right[i] += right2[i];
            }
            let _ = old_tx.push(f);
        }
        let mute = ctl.muted.load(Relaxed);
        let lc = (ctl.out_ch.load(Relaxed) as usize).min(channels.saturating_sub(1));
        let rc = (lc + 1).min(channels - 1);
        let (mut pl, mut pr, mut clipped) = (0f32, 0f32, false);
        for (i, frame) in out.chunks_mut(channels).take(frames).enumerate() {
            frame.fill(0.0);
            clipped |= left[i].abs() > CLIP_KNEE || right[i].abs() > CLIP_KNEE;
            let (l, r) = (soft_clip(left[i]), soft_clip(right[i]));
            pl = pl.max(l.abs());
            pr = pr.max(r.abs());
            if !mute {
                frame[lc] += l;
                frame[rc] += r;
            }
        }
        ctl.master_peaks[0].fetch_max(pl.to_bits(), Relaxed);
        ctl.master_peaks[1].fetch_max(pr.to_bits(), Relaxed);
        if clipped {
            ctl.clips.fetch_add(1, Relaxed);
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
    Ok(Synth {
        _stream: stream,
        info: SynthInfo { name, sample_rate, buffer, device: device_name, channels },
        control,
        swap: Some(RackSwap { tx: swap_tx, old: old_rx }),
    })
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
        let files = crate::library::corpus_styles();
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

/// The rack: one synthesizer per part, metered per channel, and a new rack (another
/// SoundFont) takes over with the voices and controllers the channels had.
#[cfg(test)]
mod rack_tests {
    use super::*;

    /// The smallest SoundFont in the checkout's soundfonts/ (None: skip).
    fn font() -> Option<Arc<SoundFont>> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("soundfonts");
        let f = crate::library::sound_font_files(&dir).into_iter().map(|f| dir.join(f)).min_by_key(|p| p.metadata().map(|m| m.len()).unwrap_or(u64::MAX));
        let Some(f) = f else {
            eprintln!("no SoundFont; skipping");
            return None;
        };
        Some(Arc::new(SoundFont::new(&mut std::fs::File::open(f).unwrap()).unwrap()))
    }

    fn peaks() -> [AtomicU32; 16] {
        std::array::from_fn(|_| AtomicU32::new(0))
    }

    fn level(p: &[AtomicU32; 16], ch: usize) -> f32 {
        f32::from_bits(p[ch].swap(0, Relaxed))
    }

    #[test]
    fn each_part_is_metered_on_its_own_channel() {
        let Some(font) = font() else { return };
        let mut rack = Rack::new(&font, 48_000).unwrap();
        let mut bank = [0u8; 16];
        let p = peaks();
        let (mut l, mut r) = (vec![0f32; 512], vec![0f32; 512]);
        for m in [[0xCA, 32, 0], [0xBA, 7, 100], [0x9A, 40, 110], [0x90, 60, 100]] {
            apply_rack(&mut rack, &m, &mut bank);
        }
        for _ in 0..8 {
            rack.render(&mut l, &mut r, &p, None);
        }
        let (bass, right1) = (level(&p, 10), level(&p, 0));
        assert!(bass > 1e-3 && right1 > 1e-3, "bass {bass}, right 1 {right1}");
        for ch in [1, 2, 3, 8, 9, 11, 12, 13, 14, 15] {
            assert_eq!(level(&p, ch), 0.0, "ch {} is silent", ch + 1);
        }
        // CC7 is the part's level: the meter follows it.
        apply_rack(&mut rack, &[0xBA, 7, 30], &mut bank);
        for _ in 0..8 {
            rack.render(&mut l, &mut r, &p, None);
        }
        assert!(level(&p, 10) < bass * 0.5);
    }

    #[test]
    fn a_new_rack_takes_over_the_channels_voices_and_levels() {
        let Some(font) = font() else { return };
        let parts = Parts::new();
        let p = peaks();
        let (mut l, mut r) = (vec![0f32; 4800], vec![0f32; 4800]);
        // What the band sent: a voice and a level on ch 11.
        let setup = [[0xBA, 0, 0], [0xCA, 32, 0], [0xBA, 7, 60], [0xBA, 11, 90]];
        let (mut bank, mut shadow) = ([0u8; 16], Shadow::new());
        let mut old = Rack::new(&font, 48_000).unwrap();
        for m in &setup {
            shadow.note(m);
            apply_rack(&mut old, m, &mut bank);
        }
        // A fresh rack with the same setup sent directly, and one that took over by replay.
        let mut sent = Rack::new(&font, 48_000).unwrap();
        let mut b2 = [0u8; 16];
        for m in &setup {
            apply_rack(&mut sent, m, &mut b2);
        }
        sync_player_rack(&mut sent, &parts);
        let mut replayed = Rack::new(&font, 48_000).unwrap();
        shadow.replay(&mut replayed, &mut bank, &parts);
        let mut energy = |rack: &mut Rack| {
            apply_rack(rack, &[0x9A, 45, 100], &mut [0u8; 16]);
            rack.render(&mut l, &mut r, &p, None);
            l.iter().zip(&r).map(|(a, b)| (a * a + b * b) as f64).sum::<f64>()
        };
        let (a, b) = (energy(&mut sent), energy(&mut replayed));
        assert!(a > 1e-6 && (a - b).abs() < a * 1e-6, "{a} vs {b}");
    }

    /// The RPNs a channel was set to (pitch bend range, tuning) carry over too: a bent
    /// bass slide sounds the same after a SoundFont change. Data entry only reaches the
    /// RPN selected when it is sent, so a replay in controller order (CC6 before CC100/101)
    /// would lose it.
    #[test]
    fn a_new_rack_keeps_the_bend_range_and_tuning() {
        let Some(font) = font() else { return };
        let parts = Parts::new();
        let p = peaks();
        // As the engine sends a style's setup: bend range 12, fine tune, coarse tune -2,
        // then RPN null; the part bent all the way up.
        let setup = [
            [0xCA, 38, 0],
            [0xBA, 101, 0], [0xBA, 100, 0], [0xBA, 6, 12], [0xBA, 38, 0],
            [0xBA, 101, 0], [0xBA, 100, 1], [0xBA, 6, 80], [0xBA, 38, 0],
            [0xBA, 101, 0], [0xBA, 100, 2], [0xBA, 6, 62],
            [0xBA, 101, 127], [0xBA, 100, 127],
            [0xEA, 127, 127],
        ];
        let (mut bank, mut shadow) = ([0u8; 16], Shadow::new());
        let mut sent = Rack::new(&font, 48_000).unwrap();
        for m in &setup {
            shadow.note(m);
            apply_rack(&mut sent, m, &mut bank);
        }
        sync_player_rack(&mut sent, &parts);
        let mut replayed = Rack::new(&font, 48_000).unwrap();
        shadow.replay(&mut replayed, &mut [0u8; 16], &parts);
        let render = |rack: &mut Rack| {
            let (mut l, mut r) = (vec![0f32; 4800], vec![0f32; 4800]);
            apply_rack(rack, &[0x9A, 40, 100], &mut [0u8; 16]);
            rack.render(&mut l, &mut r, &p, None);
            l
        };
        let (a, b) = (render(&mut sent), render(&mut replayed));
        let diff = a.iter().zip(&b).map(|(x, y)| (x - y).abs()).fold(0f32, f32::max);
        assert!(a.iter().any(|x| x.abs() > 1e-3) && diff < 1e-6, "the replayed rack plays another pitch (max diff {diff})");
        // A data entry after that (RPN null selected) changes nothing, as on the channel.
        let mut s2 = Shadow::new();
        for m in setup.iter().chain(&[[0xBA, 6, 2]]) {
            s2.note(m);
        }
        assert_eq!(s2.rpn[10], shadow.rpn[10]);
    }
}
