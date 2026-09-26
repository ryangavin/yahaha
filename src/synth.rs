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
//!
//! Each MIDI channel renders from the SoundFont or, with the `plugins` feature, from an
//! Audio Unit instrument in the plugin rack: the per-channel route table
//! ([`crate::route`], `SynthControl::routes`) says which. [`AudioCore`] is the whole
//! callback as a plain struct, so offline renders (tests, `Session::render`) run exactly
//! the code the audio device does.

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait};
use rtrb::{Consumer, Producer, RingBuffer};
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering::{Acquire, Relaxed}};
use std::sync::Arc;

use crate::click::{Click, CLICK};
use crate::parts::{self, Parts};
use crate::route::ChannelRoutes;
#[cfg(feature = "plugins")]
use crate::plugin::{PluginRack, RackControl};

/// The control side's handle on the audio thread's plugin rack (`plugins` feature; `()`
/// without it, so the types around it need no `cfg`).
#[cfg(feature = "plugins")]
pub type PluginLink = RackControl;
#[cfg(not(feature = "plugins"))]
pub type PluginLink = ();

/// The plugin rack's largest render slice, and the block size plugins are loaded for.
pub const PLUGIN_MAX_BLOCK: usize = 1024;

pub mod drum_setup;
mod routing;
pub mod xg_part;
#[cfg(test)]
mod sound_tests;
mod stream;
pub use routing::Router;
pub use stream::{BUFFER_CHOICES, DEFAULT_BUFFER};
use routing::{apply_routed, NO_SLOT};

pub type Msg = [u8; 3];

/// What the built-in synth's ring carries for SysEx `m`, if anything: a drum setup's drum
/// message (#239), or a part's XG voice setting (#246) as its controller or a mono message.
#[inline]
pub fn sysex_msg(m: &[u8]) -> Option<Msg> {
    drum_setup::encode(m).or_else(|| xg_part::encode(m))
}

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
    /// The metronome's click volume (0-127), read when a click starts.
    pub click_volume: AtomicU8,
    /// Which engine renders each MIDI channel: the SoundFont or the plugin rack
    /// (`crate::route`). The control side writes it; the audio thread reads it once per
    /// buffer.
    pub routes: ChannelRoutes,
    /// The shared effect bus's settings (#204): types, return levels.
    pub fx: crate::fx::FxControl,
}

pub struct Synth {
    output: stream::Output,
    pub info: SynthInfo,
    pub control: Arc<SynthControl>,
    /// The ends of the rings that swap racks: new ones to the audio thread, old ones back
    /// to be freed off it. Taken by whoever runs `SetSoundFont`.
    pub swap: Option<RackSwap>,
    /// The plugin rack's control half (`plugins` feature). Taken by the Session.
    pub plugins: Option<PluginLink>,
}

impl Synth {
    /// Reopen the output with `frames` per buffer (#104); every voice, plugin and queued
    /// message carries over. Returns the buffer size now in use (None: the device default).
    pub fn set_buffer(&mut self, frames: u32) -> Result<Option<u32>> {
        let r = self.output.set_buffer(frames);
        self.info.buffer = self.output.buffer;
        r
    }
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
///
/// With the sound library (#103) a rack can hold more SoundFonts, one synthesizer each,
/// and route each channel to one of them (routing.rs).
pub struct Rack {
    band: Synthesizer,
    player: Synthesizer,
    tmp_l: Vec<f32>,
    tmp_r: Vec<f32>,
    /// The extra SoundFonts' synthesizers (slots 1..).
    extra: Vec<Synthesizer>,
    /// Font id (`patches::Route`) -> slot (0 = the main font), `NO_SLOT` if not here.
    slot_of: [u8; crate::patches::route::MAX_FONTS],
    /// The slot each channel plays now.
    ch_slot: [u8; 16],
    /// Channels routed to a library patch (bit = channel).
    mapped: u16,
    /// Per extra synthesizer: how many frames it has rendered silence with no channel on
    /// it. Past `IDLE_FRAMES` it is not rendered until a channel routes to it again.
    quiet: Vec<u32>,
}

/// An extra synthesizer no channel plays is rendered until its output (reverb and chorus
/// tails included) has been below `IDLE_LEVEL` for this long, then skipped.
const IDLE_FRAMES: u32 = 4800;
const IDLE_LEVEL: f32 = 1e-6;

impl Rack {
    pub fn new(font: &Arc<SoundFont>, sample_rate: i32) -> Result<Rack> {
        let mut settings = SynthesizerSettings::new(sample_rate);
        settings.maximum_polyphony = 128;
        settings.velocity_to_filter = velocity_to_filter();
        let mut band = Synthesizer::new(font, &settings).map_err(|e| anyhow!("{e:?}"))?;
        let mut player = Synthesizer::new(font, &settings).map_err(|e| anyhow!("{e:?}"))?;
        // The shared effect bus (src/fx.rs) plays the reverb and chorus (#204).
        band.set_internal_effects(false);
        player.set_internal_effects(false);
        let mut r = Rack {
            band,
            player,
            tmp_l: vec![0.0; 8192],
            tmp_r: vec![0.0; 8192],
            extra: Vec::new(),
            slot_of: [NO_SLOT; crate::patches::route::MAX_FONTS],
            ch_slot: [0; 16],
            mapped: 0,
            quiet: Vec::new(),
        };
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

    fn synths(&mut self) -> impl Iterator<Item = &mut Synthesizer> {
        [&mut self.band, &mut self.player].into_iter().chain(self.extra.iter_mut())
    }

    /// Each channel's gains into the effect bus's send buses (#204).
    fn set_sends(&mut self, gains: &[[f32; crate::fx::BUSES]; 16]) {
        for s in self.synths() {
            for (ch, g) in gains.iter().enumerate() {
                s.set_channel_sends(ch, *g);
            }
        }
    }

    /// The synthesizers' own reverb and chorus on (the sound before #204) or off (the
    /// effect bus plays them).
    fn set_internal_effects(&mut self, on: bool) {
        for s in self.synths() {
            s.set_internal_effects(on);
        }
    }

    /// A channel's mono or poly mode (#246), on every synthesizer (as a controller).
    fn set_mono(&mut self, ch: u8, mono: bool) {
        for s in self.synths() {
            s.set_mono(ch as i32, mono);
        }
    }

    fn set_master_volume(&mut self, v: f32) {
        self.band.set_master_volume(v);
        self.player.set_master_volume(v);
        for s in &mut self.extra {
            s.set_master_volume(v);
        }
    }

    /// `render`, the send buses left out (tests).
    #[cfg(test)]
    fn render_dry(&mut self, left: &mut [f32], right: &mut [f32], peaks: &[AtomicU32; 16], fade: Option<(f32, f32)>) {
        let mut sends = vec![0f32; 2 * crate::fx::BUSES * left.len()];
        self.render(left, right, &mut sends, peaks, fade);
    }

    /// Render `left.len()` frames of the mix into `left`/`right` and the effect bus's send
    /// buses into `sends` (all overwritten; bus b's left side at `2 * b * n`, its right at
    /// `(2 * b + 1) * n`), noting each channel's peak in `peaks`. `fade` ramps the whole
    /// from one gain to another over the buffer.
    fn render(&mut self, left: &mut [f32], right: &mut [f32], sends: &mut [f32], peaks: &[AtomicU32; 16], fade: Option<(f32, f32)>) {
        let n = left.len().min(self.tmp_l.len()).min(sends.len() / (2 * crate::fx::BUSES));
        let (left, right) = (&mut left[..n], &mut right[..n]);
        let sends = &mut sends[..2 * crate::fx::BUSES * n];
        sends.fill(0.0);
        self.band.render_with_sends(left, right, sends);
        let (l, r) = (&mut self.tmp_l[..n], &mut self.tmp_r[..n]);
        // The extra synthesizers some channel plays (slot k+1 = extra[k]).
        let mut used = 0u64;
        for &s in &self.ch_slot {
            if s != 0 && s != NO_SLOT {
                used |= 1 << ((s - 1) & 63);
            }
        }
        for (i, s) in std::iter::once(&mut self.player).chain(self.extra.iter_mut()).enumerate() {
            let quiet = if i == 0 { None } else { self.quiet.get_mut(i - 1) };
            let played = i == 0 || (used >> ((i - 1) & 63)) & 1 == 1;
            if let Some(q) = quiet.as_deref()
                && !played
                && *q >= IDLE_FRAMES
            {
                continue;
            }
            s.render_with_sends(l, r, sends);
            let mut peak = 0f32;
            for k in 0..n {
                left[k] += l[k];
                right[k] += r[k];
                peak = peak.max(l[k].abs()).max(r[k].abs());
            }
            if let Some(q) = quiet {
                *q = if played || peak >= IDLE_LEVEL { 0 } else { q.saturating_add(n as u32) };
            }
        }
        let mut most = 1f32;
        if let Some((a, b)) = fade {
            most = a.max(b);
            for k in 0..n {
                let g = a + (b - a) * k as f32 / n as f32;
                left[k] *= g;
                right[k] *= g;
            }
            for (i, x) in sends.iter_mut().enumerate() {
                let k = i % n;
                *x *= a + (b - a) * k as f32 / n as f32;
            }
        }
        for s in [&mut self.band, &mut self.player].into_iter().chain(self.extra.iter_mut()) {
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
    /// Channels in mono mode (#246; bit = channel): CC126/127 or the XG part's Mono/Poly.
    mono: u16,
}

const NO_CC: u8 = 0xFF;

impl Shadow {
    fn new() -> Shadow {
        Shadow { cc: [[NO_CC; 128]; 16], program: [None; 16], bend: [None; 16], rpn: [[(NO_CC, NO_CC); 3]; 16], nrpn: [false; 16], mono: 0 }
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
            0xB0 if m[1] == 126 => self.set_mono(ch as u8, true),
            0xB0 if m[1] == 127 => self.set_mono(ch as u8, false),
            0xC0 => self.program[ch] = Some(m[1]),
            0xE0 => self.bend[ch] = Some((m[1], m[2])),
            _ => {}
        }
    }

    fn set_mono(&mut self, ch: u8, mono: bool) {
        let bit = 1 << (ch & 15);
        if mono { self.mono |= bit } else { self.mono &= !bit }
    }

    /// Bring `rack` to what the channels have: bank and voice, controllers, pitch bend.
    fn replay(&self, rack: &mut Rack, bank: &mut [u8; 16], parts: &Parts, router: Option<&Router>) {
        // Every channel: the metered ones and the Multi Pads' (5-8).
        for ch in 0..16u8 {
            self.replay_channel(rack, bank, ch, router);
            rack.set_mono(ch, self.mono >> ch & 1 == 1);
        }
        routing::sync_parts(rack, parts, router);
    }

    /// Bring channel `ch` of `rack` to what it has.
    fn replay_channel(&self, rack: &mut Rack, bank: &mut [u8; 16], ch: u8, router: Option<&Router>) {
        let apply_rack = |rack: &mut Rack, m: &Msg, bank: &mut [u8; 16]| apply_routed(rack, m, bank, router);
        {
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
    /// The control side's: sound library auditions (#103).
    pub control: Option<Producer<Msg>>,
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
            click_volume: AtomicU8::new(crate::click::DEFAULT_VOLUME),
            routes: ChannelRoutes::new(),
            fx: crate::fx::FxControl::new(),
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

#[cfg(test)]
fn sync_player_rack(rack: &mut Rack, parts: &Parts) {
    routing::sync_parts(rack, parts, None);
}

/// `Feeds::consumers`: the control side's ring (auditions) is the third.
const CONTROL_RING: usize = 2;

pub fn feeds() -> Feeds {
    let (engine, c1) = RingBuffer::new(4096);
    let (input, c2) = RingBuffer::new(1024);
    let (control, c3) = RingBuffer::new(256);
    Feeds { engine: Some(engine), input: Some(input), control: Some(control), consumers: vec![c1, c2, c3] }
}

/// GM program to use for a Yamaha voice. Yamaha's GM/XG banks (MSB 0) follow GM
/// numbering, and so does the Genos's bank 104. Banks 8 (MegaVoice, S.Art!) and 9 (the
/// Ensemble parts' S.Art! voices) number their voices by instrument, which the Data List's
/// tables map to GM (`voice_gm`, #228, #270). For the other Genos-only banks, the part's
/// role decides when the number would land in the wrong instrument family.
pub fn gm_fallback(dest: u8, msb: u8, prog: u8) -> u8 {
    if msb == 0 {
        return prog;
    }
    let prog = crate::voice_gm::gm_program(msb, prog).unwrap_or(prog);
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

/// Whether a note's velocity lowers its filter cutoff, the SF2 default modulator the
/// vendored rustysynth applies (#203): on, unless the environment has
/// `YAHAHA_VEL_FILTER=off` (or `0`), which renders the old flat tone for A/B listening.
/// Read when a rack is built, never on the audio thread.
pub fn velocity_to_filter() -> bool {
    std::env::var("YAHAHA_VEL_FILTER").map_or(true, |v| v != "off" && v != "0")
}

/// Whether renders use the SoundFont's own reverb and chorus instead of the effect bus
/// (#204): `YAHAHA_FX=legacy` (or `off`), for before/after listening. Read by
/// [`render_offline`] only.
pub fn legacy_fx() -> bool {
    std::env::var("YAHAHA_FX").is_ok_and(|v| v == "legacy" || v == "off")
}

/// Render what the band sent (`(time ns, message)`, as `sim::record` gives it) through
/// the audio callback ([`AudioCore`]: a rack on `sf2`, the effect bus, the master at unity
/// and the safety clipper; the delay at `bpm`) into stereo at `sample_rate`, for `end_ns` plus three seconds
/// of tails. Messages take effect at the start of the 64-frame block they fall in, as
/// live. For listening tests (`yahaha render`); not the audio thread.
pub fn render_offline(sf2: &Path, msgs: &[(u64, Vec<u8>)], end_ns: u64, sample_rate: u32, bpm: f64) -> Result<(Vec<f32>, Vec<f32>)> {
    const BLOCK: usize = 64;
    let rack = Rack::load(sf2, sample_rate)?;
    let (mut tx, rx) = RingBuffer::<Msg>::new(4096);
    let ctl = Arc::new(SynthControl::new(0));
    ctl.fx.legacy.store(legacy_fx(), Relaxed);
    ctl.fx.set_tempo(bpm);
    let (mut core, _swap, _plugins) = AudioCore::new(Some(rack), vec![rx], Arc::new(Parts::new()), ctl, sample_rate, 2);
    let frames = ((end_ns as f64 / 1e9 + 3.0) * sample_rate as f64) as usize;
    let (mut left, mut right) = (Vec::with_capacity(frames), Vec::with_capacity(frames));
    let mut out = [0f32; 2 * BLOCK];
    let mut next = 0;
    for start in (0..frames).step_by(BLOCK) {
        let t = (start as f64 * 1e9 / sample_rate as f64) as u64;
        while let Some((at, m)) = msgs.get(next)
            && *at <= t
        {
            next += 1;
            // Channel messages only (SysEx and the like don't reach the SoundFont live), and
            // the drum setup and the parts' XG voice settings as synth messages, as live (`live::Out`).
            let msg: Msg = if let Some(d) = m.first().filter(|&&b| b == 0xF0).and_then(|_| sysex_msg(m)) {
                d
            } else if m.is_empty() || m[0] < 0x80 || m[0] >= 0xF0 {
                continue;
            } else {
                [m[0], m.get(1).copied().unwrap_or(0), m.get(2).copied().unwrap_or(0)]
            };
            if tx.push(msg).is_err() {
                // A burst larger than the ring: take it in without rendering.
                core.process(&mut out[..0]);
                let _ = tx.push(msg);
            }
        }
        let n = (frames - start).min(BLOCK);
        core.process(&mut out[..2 * n]);
        for f in out[..2 * n].chunks(2) {
            left.push(f[0]);
            right.push(f[1]);
        }
    }
    Ok((left, right))
}

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
        // Multi Pads (ch 5-8): a Yamaha drum kit bank (MSB 126/127) is the SoundFont's drum
        // bank; any other voice is on its GM bank.
        0xC0 if (4..8).contains(&ch) => {
            let drums = bank[ch as usize] >= 126;
            out(ch, 0xB0, 0, if drums { 128 } else { 0 });
            let p = if drums { m[1] } else { gm_fallback(ch as u8, bank[ch as usize], m[1]) };
            out(ch, 0xC0, p as i32, 0);
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

/// A message to a rack: each channel to its own synthesizer (no program map).
#[cfg(test)]
#[inline]
fn apply_rack(rack: &mut Rack, m: &Msg, bank: &mut [u8; 16]) {
    apply_routed(rack, m, bank, None);
}

/// The sound library's side of the synth (#103): the program map's table and the main
/// SoundFont's font id.
pub struct Routing {
    pub routes: Arc<crate::patches::Routes>,
    pub font_id: u8,
}

/// How long a band send scale takes to glide to a new value (time constant, s).
const BAND_GLIDE_S: f32 = 0.03;

/// The audio callback: the SoundFont rack, the plugin rack (feature `plugins`), the click,
/// the master fader and the safety clipper, fed by the MIDI rings. [`AudioCore::process`]
/// renders one buffer. All memory is allocated in [`AudioCore::new`]; `process` never
/// allocates, locks or blocks (`tests/synth_no_alloc.rs`).
pub struct AudioCore {
    /// The SoundFont synthesizers (None: no SoundFont, e.g. an offline plugin test).
    rack: Option<Box<Rack>>,
    swap_rx: Consumer<Box<Rack>>,
    old_tx: Producer<Box<Rack>>,
    ctl: Arc<SynthControl>,
    parts: Arc<Parts>,
    consumers: Vec<Consumer<Msg>>,
    channels: usize,
    bank: [u8; 16],
    shadow: Box<Shadow>,
    /// The style's XG Drum Setup (#239): each drum note starts with its own settings.
    drums: Box<drum_setup::DrumSetups>,
    last_master: u8,
    left: Vec<f32>,
    right: Vec<f32>,
    left2: Vec<f32>,
    right2: Vec<f32>,
    /// A rack just replaced: it plays out one buffer, fading, then goes back to be freed.
    fading: Option<Box<Rack>>,
    /// A played-out rack the return ring had no room for: it waits here and goes back on a
    /// later buffer, so it is never freed on the audio thread. No new rack is taken while
    /// it waits.
    parked: Option<Box<Rack>>,
    unmetered: [AtomicU32; 16],
    click: Click,
    #[cfg(feature = "plugins")]
    plugins: PluginRack,
    /// Channels that played a plugin in the last buffer (bit per channel).
    plugin_on: u16,
    /// The sound library's program map (#103; None: every channel plays its GM voice on
    /// the main SoundFont, as before).
    router: Option<Router>,
    /// The shared effect bus (#204) and its send buses (see `Rack::render`): the playing
    /// rack's, then the fading one's.
    fx: crate::fx::FxBus,
    sends: Vec<f32>,
    sends2: Vec<f32>,
    /// Each channel's send controllers (CC91/93/94) as last sent, and their gains.
    send_cc: [[u8; crate::fx::BUSES]; 16],
    send_gains: [[f32; crate::fx::BUSES]; 16],
    /// The racks' send gains need setting again (a send changed, a rack came in).
    sends_dirty: bool,
    /// `FxControl::legacy` as last applied.
    legacy: bool,
    /// The band send scales (#236) as they glide towards `FxControl::band_send`.
    band_scale: [f32; crate::fx::BUSES],
    sample_rate: f32,
}

impl AudioCore {
    /// A callback for `channels` interleaved output channels at `sample_rate`, playing
    /// `rack` (None: no SoundFont) and the messages from `consumers`. Returns it with the
    /// SoundFont swap rings and the plugin rack's control half.
    pub fn new(
        rack: Option<Box<Rack>>,
        consumers: Vec<Consumer<Msg>>,
        parts: Arc<Parts>,
        control: Arc<SynthControl>,
        sample_rate: u32,
        channels: usize,
    ) -> (AudioCore, RackSwap, Option<PluginLink>) {
        let (swap_tx, swap_rx) = RingBuffer::<Box<Rack>>::new(2);
        let (old_tx, old_rx) = RingBuffer::<Box<Rack>>::new(4);
        #[cfg(feature = "plugins")]
        let (plugins, link) = crate::plugin::rack(PLUGIN_MAX_BLOCK, sample_rate as f64);
        #[cfg(not(feature = "plugins"))]
        let link = ();
        let core = AudioCore {
            rack,
            swap_rx,
            old_tx,
            ctl: control,
            parts,
            consumers,
            channels: channels.max(1),
            bank: [0u8; 16],
            shadow: Box::new(Shadow::new()),
            drums: Box::new(drum_setup::DrumSetups::new()),
            last_master: 255,
            left: vec![0f32; 8192],
            right: vec![0f32; 8192],
            left2: vec![0f32; 8192],
            right2: vec![0f32; 8192],
            fading: None,
            parked: None,
            unmetered: std::array::from_fn(|_| AtomicU32::new(0)),
            click: Click::new(sample_rate),
            #[cfg(feature = "plugins")]
            plugins,
            plugin_on: 0,
            router: None,
            fx: crate::fx::FxBus::new(sample_rate),
            sends: vec![0f32; 2 * crate::fx::BUSES * 8192],
            sends2: vec![0f32; 2 * crate::fx::BUSES * 8192],
            send_cc: [crate::fx::DEFAULT_SENDS; 16],
            send_gains: [[0f32; crate::fx::BUSES]; 16],
            sends_dirty: true,
            legacy: false,
            band_scale: crate::fx::BAND_SEND_DEFAULT.map(crate::fx::band_scale),
            sample_rate: sample_rate.max(1) as f32,
        };
        (core, RackSwap { tx: swap_tx, old: old_rx }, Some(link))
    }

    /// Play the sound library's program map (#103): band program changes and the
    /// keyboard parts' voices go through `routes`.
    pub fn set_routes(&mut self, routes: Arc<crate::patches::Routes>) {
        self.router = Some(Router::new(routes));
    }

    /// The channels playing a plugin as of the last buffer (bit per MIDI channel).
    pub fn plugin_channels(&self) -> u16 {
        self.plugin_on
    }

    /// Render one buffer of interleaved output (`out.len() / channels` frames).
    pub fn process(&mut self, out: &mut [f32]) {
        let channels = self.channels;
        let ctl = &*self.ctl;
        // A rack still waiting to go back: try again.
        if let Some(p) = self.parked.take() {
            retire(&mut self.old_tx, &mut self.parked, p);
        }
        // A new SoundFont: the new rack takes over with the channels' voices and controllers.
        // Only when the one it replaces has a place to go back to: a free slot in the return
        // ring (the audio thread is its only producer, so the slot is still free when the
        // fade ends below). Otherwise the new rack waits in its ring for a later buffer.
        if self.parked.is_none()
            && self.fading.is_none()
            && self.old_tx.slots() > 0
            && let Ok(mut new) = self.swap_rx.pop()
        {
            self.shadow.replay(&mut new, &mut self.bank, &self.parts, self.router.as_ref());
            new.set_internal_effects(self.legacy);
            self.sends_dirty = true;
            self.fading = self.rack.replace(new);
            self.last_master = 255;
            ctl.swaps.fetch_add(1, Relaxed);
        }
        // Acquire pairs with the Release stores in `Parts`: the new programs are visible.
        if self.parts.changed.swap(false, Acquire)
            && let Some(rack) = self.rack.as_mut()
        {
            routing::sync_parts(rack, &self.parts, self.router.as_ref());
        }
        // The program map changed under the channels (#103): route them again.
        if let (Some(rack), Some(router)) = (self.rack.as_mut(), self.router.as_mut()) {
            routing::follow_table(rack, &self.shadow, &mut self.bank, &self.parts, router);
        }
        let master = ctl.master.load(Relaxed);
        if master != self.last_master {
            self.last_master = master;
            if let Some(rack) = self.rack.as_mut() {
                rack.set_master_volume(master_gain(master));
            }
            if let Some(f) = self.fading.as_mut() {
                f.set_master_volume(master_gain(master));
            }
        }

        // Which channels play a plugin this buffer: routed to one, and its slot has it.
        #[cfg(feature = "plugins")]
        let active = {
            self.plugins.begin_block();
            let routed = ctl.routes.table().plugin_mask();
            let mut a = 0u16;
            for ch in 0..16u8 {
                if routed >> ch & 1 == 1 && self.plugins.owns(ch) {
                    a |= 1 << ch;
                }
            }
            a
        };
        #[cfg(not(feature = "plugins"))]
        let active = {
            let _ = ctl.routes.table();
            0u16
        };
        // A channel going over to its plugin: the SoundFont's notes there release (their
        // own envelopes, no click). Channel mode messages are actions, not shadowed.
        let started = active & !self.plugin_on;
        if started != 0
            && let Some(rack) = self.rack.as_mut()
        {
            for ch in 0..16u8 {
                if started >> ch & 1 == 1 {
                    apply_routed(rack, &[0xB0 | ch, 64, 0], &mut self.bank, None);
                    apply_routed(rack, &[0xB0 | ch, 123, 0], &mut self.bank, None);
                }
            }
        }
        self.plugin_on = active;

        for (i, c) in self.consumers.iter_mut().enumerate() {
            while let Ok(m) = c.pop() {
                // The metronome's click voice: not a MIDI part.
                if m[0] == CLICK {
                    self.click.trigger(m[1] != 0, ctl.click_volume.load(Relaxed));
                    continue;
                }
                // The style's drum setup (#239): not a MIDI message.
                if drum_setup::is_drum_msg(&m) {
                    self.drums.observe(&m);
                    continue;
                }
                // A part's Mono/Poly (#246): not a MIDI message either.
                if let Some((ch, mono)) = xg_part::mono(&m) {
                    if i != CONTROL_RING {
                        self.shadow.set_mono(ch, mono);
                    }
                    if let Some(rack) = self.rack.as_mut() {
                        rack.set_mono(ch, mono);
                    }
                    continue;
                }
                // The program map's own messages (a table bank switch, an audition; #103).
                if let (Some(rack), Some(router)) = (self.rack.as_mut(), self.router.as_mut())
                    && routing::control_msg(&m, rack, &self.shadow, &mut self.bank, &self.parts, router)
                {
                    continue;
                }
                #[cfg(feature = "plugins")]
                if (0x80..0xF0).contains(&m[0]) {
                    if active >> (m[0] & 0x0F) & 1 == 1 {
                        self.plugins.midi(m, 0);
                        // The plugin plays the notes; the SoundFont side keeps everything
                        // else (controllers, program, bend), so it takes the channel back
                        // in step.
                        if m[0] & 0xF0 == 0x90 && m[2] > 0 {
                            continue;
                        }
                    } else {
                        self.plugins.track(m);
                    }
                }
                // The control side's ring carries auditions, not the band: the shadow keeps
                // the band's setup of the channel for when the audition ends.
                if i != CONTROL_RING {
                    self.shadow.note(&m);
                    // A send to the effect bus (#204).
                    if m[0] & 0xF0 == 0xB0
                        && let Some(b) = crate::fx::SEND_CC.iter().position(|&c| c == m[1])
                    {
                        self.send_cc[(m[0] & 0x0F) as usize][b] = m[2];
                        self.sends_dirty = true;
                    }
                }
                // A program change initializes its part's drum setup; a drum note starts with
                // its setup's settings.
                self.drums.observe(&m);
                if let Some(rack) = self.rack.as_mut() {
                    match self.drums.note(&m) {
                        Some(n) => rack.note_on_with(m[0] & 0x0F, m[1], m[2], &n),
                        None => apply_routed(rack, &m, &mut self.bank, self.router.as_ref()),
                    }
                }
            }
        }
        // The effect bus (#204): the SoundFont's own reverb and chorus instead, for a
        // before/after comparison; each channel's send gains.
        let legacy = ctl.fx.legacy.load(Relaxed);
        if legacy != self.legacy {
            self.legacy = legacy;
            self.sends_dirty = true;
            for r in [self.rack.as_mut(), self.fading.as_mut()].into_iter().flatten() {
                r.set_internal_effects(legacy);
            }
        }
        let frames = (out.len() / channels).min(self.left.len());
        // The band send scales glide to where the control side set them (about 30 ms),
        // one step a buffer, so a change never clicks.
        let k = 1.0 - (-(frames as f32) / (BAND_GLIDE_S * self.sample_rate)).exp();
        for (b, scale) in self.band_scale.iter_mut().enumerate() {
            let target = crate::fx::band_scale(ctl.fx.band_send[b].load(Relaxed));
            if *scale != target {
                let d = target - *scale;
                *scale = if d.abs() < 1e-3 { target } else { *scale + d * k };
                self.sends_dirty = true;
            }
        }
        if self.sends_dirty {
            self.sends_dirty = false;
            for (ch, (g, cc)) in self.send_gains.iter_mut().zip(&self.send_cc).enumerate() {
                *g = if legacy {
                    [0.0; crate::fx::BUSES]
                } else if crate::fx::BAND_CHANNELS.contains(&ch) {
                    std::array::from_fn(|b| crate::fx::band_send_gain(cc[b], self.band_scale[b]))
                } else {
                    cc.map(crate::fx::send_gain)
                };
            }
            for r in [self.rack.as_mut(), self.fading.as_mut()].into_iter().flatten() {
                r.set_sends(&self.send_gains);
            }
        }
        let (left, right) = (&mut self.left[..frames], &mut self.right[..frames]);
        let (left2, right2) = (&mut self.left2[..frames], &mut self.right2[..frames]);
        let sends = &mut self.sends[..2 * crate::fx::BUSES * frames];
        match self.rack.as_mut() {
            Some(rack) => rack.render(left, right, sends, &ctl.peaks, None),
            None => {
                left.fill(0.0);
                right.fill(0.0);
                sends.fill(0.0);
            }
        }
        if let Some(mut f) = self.fading.take() {
            let sends2 = &mut self.sends2[..2 * crate::fx::BUSES * frames];
            f.render(left2, right2, sends2, &self.unmetered, Some((1.0, 0.0)));
            for i in 0..frames {
                left[i] += left2[i];
                right[i] += right2[i];
            }
            for (a, b) in sends.iter_mut().zip(sends2.iter()) {
                *a += *b;
            }
            retire(&mut self.old_tx, &mut self.parked, f);
        }
        // The plugin parts: their own CC7/CC11/CC10 applied in the rack, then the master
        // fader (rustysynth applies it inside its render; the rack does not).
        #[cfg(feature = "plugins")]
        if self.plugins.active() {
            left2.fill(0.0);
            right2.fill(0.0);
            // Their sends too, at the master gain the SoundFont's carry (its channels'
            // mix includes it).
            let g = master_gain(master);
            let gains: [[f32; crate::fx::BUSES]; 16] = std::array::from_fn(|ch| self.send_gains[ch].map(|x| x * g));
            self.plugins.render_add_sends(left2, right2, Some((&mut *sends, &gains)));
            for i in 0..frames {
                left[i] += left2[i] * g;
                right[i] += right2[i] * g;
            }
            for ch in 0..16u8 {
                let p = self.plugins.take_peak(ch) * g;
                if p > 0.0 {
                    ctl.peaks[ch as usize].fetch_max(p.to_bits(), Relaxed);
                }
            }
        }
        if !self.legacy {
            self.fx.process_add(sends, frames, left, right, &ctl.fx);
        }
        self.click.render_add(left, right, master_gain(master));
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
    }
}

/// Send a played-out rack back to the control side to be freed there. The ring is never
/// full here (a swap waits for a free slot), but if it were, the rack is parked for a later
/// buffer rather than dropped on the audio thread.
fn retire(old_tx: &mut Producer<Box<Rack>>, parked: &mut Option<Box<Rack>>, rack: Box<Rack>) {
    if let Err(rtrb::PushError::Full(rack)) = old_tx.push(rack) {
        // `parked` is empty whenever a rack is retired (it is retried first, and a swap
        // waits for it), so this never drops one; if it somehow held one, leaking it beats
        // freeing it here.
        if let Some(stray) = parked.replace(rack) {
            std::mem::forget(stray);
        }
    }
}

/// Start the synth on the default output device. `buffer`: frames per buffer to ask for
/// (None: [`DEFAULT_BUFFER`]), within what the device allows.
pub fn start(sf2: &Path, consumers: Vec<Consumer<Msg>>, out_pair: Option<u8>, parts: Arc<Parts>, routing: Routing, buffer: Option<u32>) -> Result<Synth> {
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

    let rack = Box::new(Rack::with_fonts(&[(routing.font_id, font.clone())], sample_rate as i32)?);
    drop(font);
    let control = Arc::new(SynthControl::new(first));
    let (mut core, swap, plugins) = AudioCore::new(Some(rack), consumers, parts, control.clone(), sample_rate, channels);
    core.set_routes(routing.routes);
    let range = match default.buffer_size() {
        cpal::SupportedBufferSize::Range { min, max } => Some((*min, *max)),
        _ => None,
    };
    let output = stream::Output::open(device, channels as u16, sample_rate, range, core, buffer.unwrap_or(DEFAULT_BUFFER))?;
    let buffer = output.buffer;
    let name = sf2.file_stem().unwrap_or_default().to_string_lossy().to_string();
    Ok(Synth {
        output,
        info: SynthInfo { name, sample_rate, buffer, device: device_name, channels },
        control,
        swap: Some(swap),
        plugins,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Prepared;
    use crate::sim::{run, Step};
    use crate::sff::Style;
    use crate::theory::Chord;

    /// Multi Pad channels (5-8): a Yamaha drum kit bank goes to the SoundFont's drum bank,
    /// any other voice to its GM bank.
    #[test]
    fn multi_pad_channels_take_drum_kits_and_gm_voices() {
        let mut bank = [0u8; 16];
        let mut got = Vec::new();
        for m in [[0xB4, 0, 127], [0xB4, 32, 0], [0xC4, 0, 0], [0xB5, 0, 0], [0xC5, 33, 0], [0xB4, 0, 0], [0xC4, 61, 0]] {
            translate(&m, &mut bank, |ch, st, a, b| got.push((ch, st, a, b)));
        }
        assert_eq!(
            got,
            [(4, 0xB0, 0, 128), (4, 0xC0, 0, 0), (5, 0xB0, 0, 0), (5, 0xC0, 33, 0), (4, 0xB0, 0, 0), (4, 0xC0, 61, 0)]
        );
    }

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

    /// #204: a part's sends feed the shared effect bus. A note sent to the reverb rings
    /// on after its release; with no send it stops. The SoundFont's own reverb and chorus
    /// are off (fully sent at return 0, nothing is added), and `legacy` brings them back
    /// in place of the bus.
    #[test]
    fn sends_feed_the_effect_bus_and_the_soundfonts_own_effects_are_off() {
        let Some(font) = font() else { return };
        let play = |setup: &[Msg], ret: u8, legacy: bool| -> (f64, f64) {
            let rack = Box::new(Rack::new(&font, 48_000).unwrap());
            let (mut tx, rx) = RingBuffer::<Msg>::new(64);
            let ctl = Arc::new(SynthControl::new(0));
            ctl.fx.reverb_return.store(ret, Relaxed);
            ctl.fx.chorus_return.store(ret, Relaxed);
            ctl.fx.legacy.store(legacy, Relaxed);
            let (mut core, _swap, _link) = AudioCore::new(Some(rack), vec![rx], Arc::new(Parts::new()), ctl, 48_000, 2);
            let mut out = vec![0f32; 256];
            let mut energy = |core: &mut AudioCore, buffers: usize| {
                let mut e = 0f64;
                for _ in 0..buffers {
                    core.process(&mut out);
                    e += out.iter().map(|x| (*x as f64).powi(2)).sum::<f64>();
                }
                e
            };
            for m in setup.iter().chain(&[[0xCA, 32, 0], [0x9A, 45, 110]]) {
                tx.push(*m).unwrap();
            }
            let note = energy(&mut core, 150);
            tx.push([0x8A, 45, 0]).unwrap();
            energy(&mut core, 150);
            (note, energy(&mut core, 300))
        };
        let (dry_note, dry) = play(&[[0xBA, 91, 0], [0xBA, 93, 0]], 64, false);
        let (_, wet) = play(&[[0xBA, 91, 127], [0xBA, 93, 0]], 64, false);
        let (note, muted) = play(&[[0xBA, 91, 127], [0xBA, 93, 127]], 0, false);
        let (_, legacy) = play(&[[0xBA, 91, 127], [0xBA, 93, 0]], 64, true);
        assert!(dry_note > 1e-3, "the note sounds");
        assert!(wet > dry * 100.0 + 1e-6, "the reverb rings on: {wet} vs {dry}");
        assert!((note - dry_note).abs() <= dry_note * 1e-9 && (muted - dry).abs() <= 1e-12, "no reverb inside the SoundFont");
        assert!(legacy > dry * 100.0 + 1e-6, "legacy: the SoundFont's own reverb");
    }

    /// #239: a drum note starts with its drum setup's own level, pan, pitch, send and
    /// decay; a note already sounding keeps what it started with.
    #[test]
    fn a_drum_note_starts_with_its_drum_setup() {
        use drum_setup::encode;
        let Some(font) = font() else { return };
        // Drum Setup 1 (part 10, ch index 9): note `key`, parameter `p` = `v`.
        let ds = |key: u8, p: u8, v: u8| encode(&[0xF0, 0x43, 0x10, 0x4C, 0x30, key, p, v, 0xF7]).unwrap();
        // (left energy, right energy, zero crossings) while the note sounds, and the energy
        // of the tail after it, with `setup` before the note and `during` after it starts.
        let play = |key: u8, setup: &[Msg], during: &[Msg]| -> (f64, f64, usize, f64) {
            let rack = Box::new(Rack::new(&font, 48_000).unwrap());
            let (mut tx, rx) = RingBuffer::<Msg>::new(64);
            let ctl = Arc::new(SynthControl::new(0));
            ctl.fx.reverb_return.store(64, Relaxed);
            let (mut core, _swap, _link) = AudioCore::new(Some(rack), vec![rx], Arc::new(Parts::new()), ctl, 48_000, 2);
            let mut out = vec![0f32; 256];
            let mut run = |core: &mut AudioCore, buffers: usize| {
                let (mut l, mut r, mut z, mut prev) = (0f64, 0f64, 0usize, 0f32);
                for _ in 0..buffers {
                    core.process(&mut out);
                    for f in out.chunks(2) {
                        l += (f[0] as f64).powi(2);
                        r += (f[1] as f64).powi(2);
                        if (f[0] + f[1] >= 0.0) != (prev >= 0.0) {
                            z += 1;
                        }
                        prev = f[0] + f[1];
                    }
                }
                (l, r, z)
            };
            for m in setup.iter().chain(&[[0xB9, 91, 127], [0xB9, 93, 0], [0x99, key, 100]]) {
                tx.push(*m).unwrap();
            }
            run(&mut core, 1);
            for m in during {
                tx.push(*m).unwrap();
            }
            let (l, r, z) = run(&mut core, 40);
            tx.push([0x89, key, 0]).unwrap();
            run(&mut core, 150);
            let (tl, tr, _) = run(&mut core, 300);
            (l, r, z, tl + tr)
        };
        let snare = 38;
        let (l, r, _, wet) = play(snare, &[], &[]);
        assert!(l + r > 1e-3, "the snare sounds");
        // Level 50 of 100: a quarter of the amplitude.
        let (ql, qr, _, _) = play(snare, &[ds(snare, 0x02, 50)], &[]);
        let ratio = (ql + qr) / (l + r);
        assert!((ratio - 1.0 / 16.0).abs() < 0.01, "level 50: energy x{ratio}");
        // Pan hard left.
        let (pl, pr, _, _) = play(snare, &[ds(snare, 0x04, 1)], &[]);
        assert!(pr < pl * 0.01, "panned left: {pl} / {pr}");
        // No reverb send for this note, though the part sends fully.
        let (_, _, _, dry) = play(snare, &[ds(snare, 0x05, 0)], &[]);
        assert!(dry < wet * 0.1, "reverb send 0: tail {dry} vs {wet}");
        // A faster decay: a shorter tail.
        let (_, _, _, short) = play(snare, &[ds(snare, 0x0E, 0x7F), ds(snare, 0x05, 0)], &[]);
        assert!(short <= dry, "decay 1 faster: tail {short} vs {dry}");
        // Pitch: the note sounds otherwise (the tuning itself: `note_tune_shifts_the_pitch`).
        let (tl, tr, _, _) = play(snare, &[ds(snare, 0x00, 0x40 + 7), ds(snare, 0x01, 0x40 + 30)], &[]);
        assert!((tl, tr) != (l, r), "coarse +7, fine +30 cents");
        // A setup change while the note sounds leaves it as it started.
        let (cl, cr, _, _) = play(snare, &[], &[ds(snare, 0x02, 10), ds(snare, 0x04, 1)]);
        assert_eq!((cl, cr), (l, r), "the sounding note is untouched");
    }

    /// #239: `NoteParams::tune` moves a voice's pitch by semitones: a piano C4 tuned an
    /// octave up plays its sample twice as fast.
    #[test]
    fn note_tune_shifts_the_pitch() {
        let Some(font) = font() else { return };
        let crossings = |key: i32, tune: f32| {
            let mut s = Synthesizer::new(&font, &SynthesizerSettings::new(48_000)).unwrap();
            s.set_internal_effects(false);
            s.process_midi_message(0, 0xC0, 0, 0);
            let note = rustysynth::NoteParams { tune, ..rustysynth::NoteParams::NEUTRAL };
            s.note_on_with(0, key, 100, &note);
            let (mut l, mut r) = (vec![0f32; 9600], vec![0f32; 9600]);
            s.render(&mut l, &mut r);
            l.windows(2).filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0)).count() as f64
        };
        // The same sample played twice (half) as fast: roughly twice (half) the zero
        // crossings; the attack's noise keeps the count from being exact.
        let (tuned, c4, down) = (crossings(60, 12.0), crossings(60, 0.0), crossings(60, -12.0));
        assert!((1.6..2.6).contains(&(tuned / c4)), "C4 +12 {tuned} vs C4 {c4} crossings");
        assert!((1.6..2.6).contains(&(c4 / down)), "C4 -12 {down} vs C4 {c4} crossings");
    }

    /// #236: the band send scales. A Style part's (channel 11) delay send reaches the
    /// delay only as far as the Delay band send lets it: off by default, as written at
    /// 100%. A keyboard part's (channel 1) send is never scaled. A change glides in over
    /// a few buffers rather than jumping.
    #[test]
    fn the_band_send_scales_only_the_style_parts() {
        let Some(font) = font() else { return };
        let play = |ch: u8, band: Option<u8>, send: u8| -> (f64, f64) {
            let rack = Box::new(Rack::new(&font, 48_000).unwrap());
            let (mut tx, rx) = RingBuffer::<Msg>::new(64);
            let ctl = Arc::new(SynthControl::new(0));
            ctl.fx.reverb_return.store(0, Relaxed);
            ctl.fx.chorus_return.store(0, Relaxed);
            if let Some(b) = band {
                ctl.fx.band_send[crate::fx::VARIATION].store(b, Relaxed);
            }
            let (mut core, _swap, _link) = AudioCore::new(Some(rack), vec![rx], Arc::new(Parts::new()), ctl, 48_000, 2);
            let mut out = vec![0f32; 256];
            let mut energy = |core: &mut AudioCore, buffers: usize| {
                let mut e = 0f64;
                for _ in 0..buffers {
                    core.process(&mut out);
                    e += out.iter().map(|x| (*x as f64).powi(2)).sum::<f64>();
                }
                e
            };
            for m in [[0xC0 | ch, 0, 0], [0xB0 | ch, 94, send], [0x90 | ch, 60, 110]] {
                tx.push(m).unwrap();
            }
            energy(&mut core, 40);
            tx.push([0x80 | ch, 60, 0]).unwrap();
            // Past the note's release, into the echoes (a dotted 1/8 at 120 is 375 ms).
            energy(&mut core, 60);
            (energy(&mut core, 150), energy(&mut core, 1))
        };
        let (dry, _) = play(10, None, 0);
        let (style_default, _) = play(10, None, 127);
        let (style_full, _) = play(10, Some(100), 127);
        let (keys_default, _) = play(0, None, 127);
        let (keys_zero, _) = play(0, Some(0), 127);
        assert!(dry > 0.0 && (style_default - dry).abs() <= dry * 1e-6, "by default no delay on the band: {style_default} vs {dry}");
        assert!(style_full > dry * 2.0, "at 100% the style's delay send plays: {style_full} vs {dry}");
        assert!(keys_default > dry * 2.0 && (keys_zero - keys_default).abs() <= keys_default * 1e-6, "a keyboard part is never scaled");

        // The glide: a scale moves over a few buffers, not at once.
        let (_tx, rx) = RingBuffer::<Msg>::new(4);
        let ctl = Arc::new(SynthControl::new(0));
        let (mut core, _swap, _link) = AudioCore::new(None, vec![rx], Arc::new(Parts::new()), ctl.clone(), 48_000, 2);
        let mut out = vec![0f32; 256];
        core.process(&mut out);
        assert_eq!(core.band_scale, [1.0, 0.0, 0.0], "the defaults: reverb as written, no chorus, no delay");
        ctl.fx.band_send[crate::fx::CHORUS].store(100, Relaxed);
        core.process(&mut out);
        let first = core.band_scale[crate::fx::CHORUS];
        assert!(first > 0.0 && first < 0.2, "the first buffer moves a little: {first}");
        // 128 frames a buffer: 0.4 s.
        for _ in 0..150 {
            core.process(&mut out);
        }
        assert_eq!(core.band_scale[crate::fx::CHORUS], 1.0, "and arrives");
        assert_eq!(core.send_gains[10][crate::fx::CHORUS], 0.0, "no send, no gain");
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
            rack.render_dry(&mut l, &mut r, &p, None);
        }
        let (bass, right1) = (level(&p, 10), level(&p, 0));
        assert!(bass > 1e-3 && right1 > 1e-3, "bass {bass}, right 1 {right1}");
        for ch in [1, 2, 3, 8, 9, 11, 12, 13, 14, 15] {
            assert_eq!(level(&p, ch), 0.0, "ch {} is silent", ch + 1);
        }
        // CC7 is the part's level: the meter follows it.
        apply_rack(&mut rack, &[0xBA, 7, 30], &mut bank);
        for _ in 0..8 {
            rack.render_dry(&mut l, &mut r, &p, None);
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
        shadow.replay(&mut replayed, &mut bank, &parts, None);
        let mut energy = |rack: &mut Rack| {
            apply_rack(rack, &[0x9A, 45, 100], &mut [0u8; 16]);
            rack.render_dry(&mut l, &mut r, &p, None);
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
        shadow.replay(&mut replayed, &mut [0u8; 16], &parts, None);
        let render = |rack: &mut Rack| {
            let (mut l, mut r) = (vec![0f32; 4800], vec![0f32; 4800]);
            apply_rack(rack, &[0x9A, 40, 100], &mut [0u8; 16]);
            rack.render_dry(&mut l, &mut r, &p, None);
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
