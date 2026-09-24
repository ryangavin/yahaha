//! `PluginRack`: the plugin parts, on the audio thread.
//!
//! The rack has one slot per MIDI channel, so it slots into the synth callback the same way
//! the SoundFont synth does: keyboard parts are channels 0-3 (`parts::CHANNEL`: Right 1 = 0,
//! Left = 1, Right 2 = 2, Right 3 = 3) and the Style parts 8-15. A slot with an instrument
//! *owns* its channel: [`PluginRack::midi`] takes that channel's messages and returns
//! `true`, and the caller sends everything else to rustysynth as before.
//!
//! Two halves, joined by lock-free `rtrb` rings:
//!
//! - [`RackControl`] (a control thread): [`RackControl::assign`] a preloaded
//!   [`PluginInstance`] to a channel, [`RackControl::clear`] it, and
//!   [`RackControl::poll`] the rack's events. Retired instances come back here to be
//!   dropped, so an Audio Unit is never disposed of on the audio thread.
//! - [`PluginRack`] (the audio callback): per block, [`PluginRack::begin_block`], then
//!   [`PluginRack::midi`] for each message, then [`PluginRack::render_add`], which mixes
//!   every slot into the caller's stereo buffers. None of these allocate or lock.
//!
//! **Mixer rule.** A part's level is its CC7 and CC11 on the GM curve, exactly as the
//! built-in synth answers them ([`PartGain`]): the rack keeps those two controllers (and
//! their LSBs) and applies the gain to the plugin's output; the plugin never sees them.
//! It tracks them (and the replayed controllers) on every channel, owned or not, so a plugin
//! assigned mid-song starts at the part's current level.
//! The master fader and soft clipper stay after the sum, in the caller. The part's pan
//! (CC10) is the rack's too: plugins disagree about CC10 as they do about CC7, so the
//! rack keeps it from the plugin and applies it as a balance on the plugin's stereo
//! output (centre = the plugin's own image, untouched).
//!
//! **Swaps.** An assign takes effect at the next block boundary. The outgoing instance gets
//! Sustain off + All Notes Off and keeps rendering while it fades out over `fade_frames`
//! (default 5 ms) as the new one fades in; then it is handed back to the control side. The
//! new instance first gets the part's current controllers (modulation, pan, sustain, pitch
//! bend and the rest the part has sent), so a swap mid-phrase picks up where the old voice
//! was. The part's CC7/CC11 gain belongs to the slot, not the instance, so it carries over.
//!
//! **Faults.** A render that fails (an OSStatus, or NaN/infinity) mutes the slot: it keeps
//! owning its channel (so the part goes quiet rather than jumping to the SoundFont), emits
//! [`RackEvent::Fault`] once, and renders nothing until the control side assigns or clears.
//! Renders slower than the instance's budget raise [`RackEvent::Overrun`] (at most one per
//! slot per second); the full timing is in each instance's [`super::PluginStats`].
//!
//! **Meters.** Each slot keeps its output peak (after the part's gain and pan, before the
//! caller's master), for the caller's per-channel meters: [`PluginRack::take_peak`].
//!
//! **Disposal.** Retired instances, and whatever a dropped rack still holds, are disposed
//! of on a thread of their own (`plugin-dispose`): an Audio Unit's dispose takes a
//! per-component lock that a slow load of the same plugin can hold, and neither the audio
//! thread nor the Session's control thread should wait on it.

use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::{Mutex, OnceLock, mpsc};
use std::time::{Duration, Instant};

use super::PartGain;
use super::instance::{PluginInstance, RenderError};

/// Slots: one per MIDI channel.
pub const SLOTS: usize = 16;

/// Default crossfade on a swap or clear: 5 ms at 48 kHz.
pub const DEFAULT_FADE_FRAMES: u32 = 240;

/// How a swap happens.
#[derive(Clone, Copy, Debug)]
pub struct Swap {
    /// Crossfade length. 0 = hand over at the block boundary (the old voice is cut).
    pub fade_frames: u32,
    /// A fixed level trim for this instance, linear (1.0 = none): the per-voice
    /// normalisation that makes a plugin sit near its SoundFont equivalent. A non-finite or
    /// negative trim is treated as 1.0.
    pub trim: f32,
}

impl Default for Swap {
    fn default() -> Self {
        Swap { fade_frames: DEFAULT_FADE_FRAMES, trim: 1.0 }
    }
}

enum RackCmd {
    Assign { channel: u8, inst: Box<PluginInstance>, swap: Swap, sent: Instant },
    Clear { channel: u8, fade_frames: u32 },
}

/// What the rack reports to the control side.
#[derive(Clone, Debug, PartialEq)]
pub enum RackEvent {
    /// An assigned instance started playing. `latency` runs from the `assign` call to the
    /// start of the block that swapped it in.
    Swapped { channel: u8, latency: Duration },
    /// A clear took effect; the channel goes back to the caller's synth.
    Cleared { channel: u8 },
    /// The slot's instance failed and the part is muted until reassigned or cleared.
    Fault { channel: u8, error: RenderError },
    /// A render took longer than its budget (see `PluginStats::budget_permille`).
    Overrun { channel: u8, render_us: f32, block_us: f32 },
}

/// The part's controllers as last sent, replayed into an incoming instance.
#[derive(Clone, Copy)]
struct Controllers {
    value: [u8; 128],
    seen: u128,
    bend: Option<[u8; 2]>,
    /// The parameter selects as last sent (CC101/100 RPN, CC99/98 NRPN; `NONE` unsent).
    select: [u8; 4],
    /// An NRPN was selected after the last RPN: data entry goes nowhere we track.
    nrpn: bool,
    /// Data entry (CC6, CC38) per RPN 0-2 (pitch bend range, fine and coarse tune), as
    /// the SoundFont side's `Shadow` keeps it: data entry reaches the RPN selected when it
    /// comes, so it is kept by RPN and replayed under its own select.
    rpn: [[u8; 2]; 3],
}

/// "Not sent" for a select or data byte (MIDI data is 0-127).
const NONE: u8 = 0xFF;

impl Controllers {
    const fn new() -> Self {
        Controllers { value: [0; 128], seen: 0, bend: None, select: [NONE; 4], nrpn: false, rpn: [[NONE; 2]; 3] }
    }

    /// Controllers that describe the part's playing state (not volume, which the rack owns,
    /// not bank select or the RPN/NRPN data sequence, not channel mode messages).
    fn tracked(cc: u8) -> bool {
        !matches!(cc, 0 | 32 | 6 | 38 | 7 | 39 | 10 | 42 | 11 | 43 | 96..=101 | 120..=127)
    }

    fn observe(&mut self, m: [u8; 3]) {
        match m[0] & 0xF0 {
            0xB0 if m[1] == 121 => {
                // Reset All Controllers: back to "nothing to replay" (plus the reset itself).
                self.seen = 0;
                self.bend = None;
            }
            0xB0 if Self::tracked(m[1]) => {
                self.value[m[1] as usize & 0x7F] = m[2];
                self.seen |= 1u128 << (m[1] & 0x7F);
            }
            0xB0 => match m[1] {
                101 => (self.select[0], self.nrpn) = (m[2], false),
                100 => (self.select[1], self.nrpn) = (m[2], false),
                99 => (self.select[2], self.nrpn) = (m[2], true),
                98 => (self.select[3], self.nrpn) = (m[2], true),
                6 | 38 => {
                    let (msb, lsb) = (self.select[0], self.select[1]);
                    if !self.nrpn && msb == 0 && lsb < 3 {
                        self.rpn[lsb as usize][(m[1] == 38) as usize] = m[2];
                    }
                }
                _ => {}
            },
            0xE0 => self.bend = Some([m[1], m[2]]),
            _ => {}
        }
    }

    fn replay(&self, ch: u8, inst: &mut PluginInstance) {
        let mut seen = self.seen;
        while seen != 0 {
            let cc = seen.trailing_zeros() as u8;
            seen &= seen - 1;
            let _ = inst.midi([0xB0 | ch, cc, self.value[cc as usize]], 0);
        }
        // RPN 0-2 under their own select, then the select the channel had (as
        // `synth::Shadow::replay` does for a new SoundFont).
        for (n, &[msb, lsb]) in self.rpn.iter().enumerate() {
            if msb == NONE && lsb == NONE {
                continue;
            }
            let _ = inst.midi([0xB0 | ch, 101, 0], 0);
            let _ = inst.midi([0xB0 | ch, 100, n as u8], 0);
            if msb != NONE {
                let _ = inst.midi([0xB0 | ch, 6, msb], 0);
            }
            if lsb != NONE {
                let _ = inst.midi([0xB0 | ch, 38, lsb], 0);
            }
        }
        // The RPN select as it was (a later CC100 alone then pairs with the right CC101),
        // then the NRPN select if that came last (data entry goes to it).
        let n = if self.nrpn { 4 } else { 2 };
        for (k, cc) in [101u8, 100, 99, 98].into_iter().enumerate().take(n) {
            if self.select[k] != NONE {
                let _ = inst.midi([0xB0 | ch, cc, self.select[k]], 0);
            }
        }
        if let Some([lsb, msb]) = self.bend {
            let _ = inst.midi([0xE0 | ch, lsb, msb], 0);
        }
    }
}

struct Slot {
    cur: Option<Box<PluginInstance>>,
    cur_trim: f32,
    /// Fading out (a swap or clear in progress).
    old: Option<Box<PluginInstance>>,
    old_trim: f32,
    fade_len: u32,
    fade_pos: u32,
    faulted: bool,
    gain: PartGain,
    /// The part's CC10 (64 = centre), applied as a balance, and the balance gains the last
    /// block ended on (ramped like the volume).
    pan: u8,
    bal: (f32, f32),
    /// Output peak since the last `take_peak`.
    peak: f32,
    ctl: Controllers,
    /// Sample count at the last overrun event, for rate limiting (`u64::MAX`: none yet).
    last_overrun: u64,
}

impl Slot {
    const fn new() -> Self {
        Slot {
            cur: None,
            cur_trim: 1.0,
            old: None,
            old_trim: 1.0,
            fade_len: 0,
            fade_pos: 0,
            faulted: false,
            gain: PartGain::new(),
            pan: 64,
            bal: (1.0, 1.0),
            peak: 0.0,
            ctl: Controllers::new(),
            last_overrun: u64::MAX,
        }
    }

    fn owns(&self) -> bool {
        self.cur.is_some() || self.faulted
    }

    /// Track a message's controllers, volume and pan. True when the host consumed it (a
    /// volume or pan message the plugin must not see).
    #[inline]
    fn track(&mut self, m: [u8; 3]) -> bool {
        self.ctl.observe(m);
        if m[0] & 0xF0 == 0xB0 && m[1] == 10 {
            self.pan = m[2] & 0x7F;
            return true;
        }
        // CC42 (pan LSB) is swallowed too; a balance has no use for 14 bits.
        if m[0] & 0xF0 == 0xB0 && m[1] == 42 {
            return true;
        }
        self.gain.take(m)
    }
}

/// The balance gains (left, right) for a CC10 value: centre (64) is (1, 1), hard left
/// (0) is (1, 0), hard right (127) is (0, 1), linear in between. A balance rather than a
/// pan law because a plugin's output is already stereo: at centre it is untouched.
#[inline]
pub fn balance(cc10: u8) -> (f32, f32) {
    let p = ((cc10.min(127) as f32 - 64.0) / 63.0).clamp(-1.0, 1.0);
    ((1.0 - p).min(1.0), (1.0 + p).min(1.0))
}

/// Dispose of an instance on the `plugin-dispose` thread (started on first use). Not
/// RT-safe (it may lock and allocate): for the control side and `Drop`.
pub fn dispose_later(inst: Box<PluginInstance>) {
    type Tx = Mutex<Option<mpsc::Sender<Box<PluginInstance>>>>;
    static TX: OnceLock<Tx> = OnceLock::new();
    let m = TX.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<Box<PluginInstance>>();
        let ok = std::thread::Builder::new().name("plugin-dispose".into()).spawn(move || {
            for inst in rx {
                drop(inst);
            }
        });
        Mutex::new(ok.ok().map(|_| tx))
    });
    let tx = m.lock().unwrap_or_else(|e| e.into_inner()).clone();
    match tx {
        Some(tx) => {
            if let Err(mpsc::SendError(inst)) = tx.send(inst) {
                drop(inst);
            }
        }
        None => drop(inst),
    }
}

/// The control-thread half. See the module docs.
pub struct RackControl {
    tx: Producer<RackCmd>,
    events: Consumer<RackEvent>,
    retired: Consumer<Box<PluginInstance>>,
    sample_rate: f64,
}

impl RackControl {
    /// The sample rate the rack renders at (instances must match it).
    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    /// Hand a loaded instance to `channel` (0-15). It starts at the next block boundary
    /// (or, if the channel is still crossfading from a previous swap, once that fade is
    /// done). Gives the instance back if the command ring is full (the audio thread is not
    /// running) or the instance runs at another sample rate than the rack (it would play
    /// at the wrong pitch and speed).
    pub fn assign(&mut self, channel: u8, inst: PluginInstance, mut swap: Swap) -> Result<(), Box<PluginInstance>> {
        if (inst.sample_rate() - self.sample_rate).abs() > 0.5 {
            return Err(Box::new(inst));
        }
        // The trim multiplies straight into the caller's mix: a NaN or infinity there would
        // bypass the render's non-finite check and silence everything.
        if !swap.trim.is_finite() || swap.trim < 0.0 {
            swap.trim = 1.0;
        }
        let cmd = RackCmd::Assign { channel: channel & 0x0F, inst: Box::new(inst), swap, sent: Instant::now() };
        self.tx.push(cmd).map_err(|rtrb::PushError::Full(c)| match c {
            RackCmd::Assign { inst, .. } => inst,
            RackCmd::Clear { .. } => unreachable!(),
        })
    }

    /// Fade `channel`'s instance out and give the channel back to the caller's synth.
    pub fn clear(&mut self, channel: u8, fade_frames: u32) -> bool {
        self.tx.push(RackCmd::Clear { channel: channel & 0x0F, fade_frames }).is_ok()
    }

    /// Drain the rack's events and dispose of retired instances (on the `plugin-dispose`
    /// thread, see the module docs). Call it regularly from the control loop.
    pub fn poll(&mut self) -> Vec<RackEvent> {
        while let Ok(inst) = self.retired.pop() {
            dispose_later(inst);
        }
        self.poll_events()
    }

    /// Drain the rack's events only, leaving retired instances for
    /// [`RackControl::take_retired`].
    pub fn poll_events(&mut self) -> Vec<RackEvent> {
        let mut out = Vec::new();
        while let Ok(e) = self.events.pop() {
            out.push(e);
        }
        out
    }

    /// Take retired instances instead of dropping them (to keep a warm pool).
    pub fn take_retired(&mut self) -> Vec<PluginInstance> {
        let mut out = Vec::new();
        while let Ok(inst) = self.retired.pop() {
            out.push(*inst);
        }
        out
    }
}

/// The audio-thread half. See the module docs.
pub struct PluginRack {
    slots: [Slot; SLOTS],
    rx: Consumer<RackCmd>,
    events: Producer<RackEvent>,
    retired: Producer<Box<PluginInstance>>,
    /// Retired instances the return ring had no room for; retried every block.
    backlog: [Option<Box<PluginInstance>>; 8],
    // Scratch: the incoming and outgoing instance's output for one slot.
    l: Box<[f32]>,
    r: Box<[f32]>,
    l2: Box<[f32]>,
    r2: Box<[f32]>,
    max_block: usize,
    sample_rate: f64,
    samples: u64,
    /// Events lost because the control side stopped polling.
    pub dropped_events: u64,
}

/// A rack for blocks of up to `max_block` frames at `sample_rate` (longer blocks are
/// rendered in pieces). All memory is allocated here, once.
pub fn rack(max_block: usize, sample_rate: f64) -> (PluginRack, RackControl) {
    let (tx, rx) = RingBuffer::new(64);
    let (etx, erx) = RingBuffer::new(256);
    let (rtx, rrx) = RingBuffer::new(32);
    let z = || vec![0f32; max_block].into_boxed_slice();
    let rack = PluginRack {
        slots: [const { Slot::new() }; SLOTS],
        rx,
        events: etx,
        retired: rtx,
        backlog: Default::default(),
        l: z(),
        r: z(),
        l2: z(),
        r2: z(),
        max_block,
        sample_rate,
        samples: 0,
        dropped_events: 0,
    };
    (rack, RackControl { tx, events: erx, retired: rrx, sample_rate })
}

impl PluginRack {
    /// Whether `channel` is played by a plugin (or muted by a faulted one).
    #[inline]
    pub fn owns(&self, channel: u8) -> bool {
        self.slots[(channel & 0x0F) as usize].owns()
    }

    /// The sample rate the rack renders at.
    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    /// Whether any slot has an instance (playing or fading out): when not, `render_add`
    /// would add nothing and the caller may skip it. RT-safe.
    #[inline]
    pub fn active(&self) -> bool {
        self.slots.iter().any(|s| s.cur.is_some() || s.old.is_some())
    }

    /// `channel`'s output peak (linear, both sides; after its gain and pan) since the last
    /// call, and start again from 0. RT-safe.
    #[inline]
    pub fn take_peak(&mut self, channel: u8) -> f32 {
        std::mem::take(&mut self.slots[(channel & 0x0F) as usize].peak)
    }

    /// Track a message on a channel the caller's synth plays (the rack keeps the part's
    /// level, pan and controllers for a plugin assigned later) without offering it to a
    /// plugin, even one that owns the channel. RT-safe.
    #[inline]
    pub fn track(&mut self, m: [u8; 3]) {
        if (0x80..0xF0).contains(&m[0]) {
            self.slots[(m[0] & 0x0F) as usize].track(m);
        }
    }

    /// Whether `channel` has an outgoing instance still fading out.
    #[inline]
    pub fn is_fading(&self, channel: u8) -> bool {
        self.slots[(channel & 0x0F) as usize].old.is_some()
    }

    fn event(&mut self, e: RackEvent) {
        if self.events.push(e).is_err() {
            self.dropped_events += 1;
        }
    }

    fn retire(&mut self, inst: Box<PluginInstance>) {
        let inst = match self.retired.push(inst) {
            Ok(()) => return,
            Err(rtrb::PushError::Full(i)) => i,
        };
        if let Some(slot) = self.backlog.iter_mut().find(|s| s.is_none()) {
            *slot = Some(inst);
        } else {
            // The control side has stopped polling for a long time. Never dispose of an
            // Audio Unit here: leak it instead (bounded by the number of swaps).
            std::mem::forget(inst);
        }
    }

    /// Start a block: apply pending assigns and clears. Call once per callback, before any
    /// [`PluginRack::midi`].
    pub fn begin_block(&mut self) {
        for i in 0..self.backlog.len() {
            if let Some(inst) = self.backlog[i].take()
                && let Err(rtrb::PushError::Full(inst)) = self.retired.push(inst)
            {
                self.backlog[i] = Some(inst);
            }
        }
        // A swap or clear for a channel still crossfading waits for the fade to end:
        // applying it now would cut the outgoing instance off mid-fade (a click).
        while let Ok(RackCmd::Assign { channel, .. } | RackCmd::Clear { channel, .. }) = self.rx.peek() {
            if self.slots[*channel as usize].old.is_some() {
                break;
            }
            let Ok(cmd) = self.rx.pop() else { break };
            match cmd {
                RackCmd::Assign { channel, mut inst, swap, sent } => {
                    let ch = channel as usize;
                    // Whatever was still fading goes straight back.
                    if let Some(prev) = self.slots[ch].old.take() {
                        self.retire(prev);
                    }
                    let slot = &mut self.slots[ch];
                    slot.ctl.replay(channel, &mut inst);
                    let outgoing = slot.cur.replace(inst);
                    let was_faulted = std::mem::replace(&mut slot.faulted, false);
                    slot.old_trim = slot.cur_trim;
                    slot.cur_trim = swap.trim;
                    let mut retire_now = None;
                    if let Some(mut old) = outgoing {
                        if swap.fade_frames == 0 || was_faulted {
                            retire_now = Some(old);
                        } else {
                            let _ = old.midi([0xB0 | channel, 64, 0], 0);
                            let _ = old.midi([0xB0 | channel, 123, 0], 0);
                            slot.old = Some(old);
                        }
                    }
                    slot.fade_len = if slot.old.is_some() { swap.fade_frames } else { 0 };
                    slot.fade_pos = 0;
                    if let Some(old) = retire_now {
                        self.retire(old);
                    }
                    self.event(RackEvent::Swapped { channel, latency: sent.elapsed() });
                }
                RackCmd::Clear { channel, fade_frames } => {
                    let ch = channel as usize;
                    if let Some(prev) = self.slots[ch].old.take() {
                        self.retire(prev);
                    }
                    let slot = &mut self.slots[ch];
                    let was_faulted = std::mem::replace(&mut slot.faulted, false);
                    let mut retire_now = None;
                    if let Some(mut old) = slot.cur.take() {
                        if fade_frames == 0 || was_faulted {
                            retire_now = Some(old);
                        } else {
                            let _ = old.midi([0xB0 | channel, 64, 0], 0);
                            let _ = old.midi([0xB0 | channel, 123, 0], 0);
                            slot.old = Some(old);
                            slot.old_trim = slot.cur_trim;
                            slot.fade_len = fade_frames;
                            slot.fade_pos = 0;
                        }
                    }
                    if let Some(old) = retire_now {
                        self.retire(old);
                    }
                    self.event(RackEvent::Cleared { channel });
                }
            }
        }
    }

    /// Offer one channel message. Returns `true` if a plugin slot took it (the caller must
    /// not also send it to its synth). `offset` is the frame within the coming block and
    /// must be below its length. RT-safe.
    #[inline]
    pub fn midi(&mut self, m: [u8; 3], offset: u32) -> bool {
        if m[0] < 0x80 || m[0] >= 0xF0 {
            return false;
        }
        let ch = m[0] & 0x0F;
        let slot = &mut self.slots[ch as usize];
        // Track the part's controllers, CC7/CC11 and CC10 on every channel, owned or not,
        // so a plugin assigned later starts at the level (and with the controllers) the
        // part already has, not at power-on defaults.
        let host = slot.track(m);
        if !slot.owns() {
            return false;
        }
        if host || slot.faulted {
            return true;
        }
        let offset = offset.min(self.max_block.saturating_sub(1) as u32);
        if let Some(inst) = slot.cur.as_mut()
            && let Err(e) = inst.midi(m, offset)
        {
            // A plugin whose process died fails here first.
            slot.faulted = true;
            self.event(RackEvent::Fault { channel: ch, error: e });
        }
        true
    }

    /// Render every slot and **add** it into `left` / `right` (the caller's mix, before its
    /// master gain and soft clipper). RT-safe.
    pub fn render_add(&mut self, left: &mut [f32], right: &mut [f32]) {
        let frames = left.len().min(right.len());
        let mut done = 0;
        while done < frames {
            let n = (frames - done).min(self.max_block);
            for ch in 0..SLOTS {
                self.render_slot(ch, &mut left[done..done + n], &mut right[done..done + n]);
            }
            done += n;
            self.samples += n as u64;
        }
    }

    fn render_slot(&mut self, ch: usize, out_l: &mut [f32], out_r: &mut [f32]) {
        let n = out_l.len();
        let rate = self.sample_rate;
        let samples = self.samples;
        let (l, r, l2, r2) = (&mut self.l[..n], &mut self.r[..n], &mut self.l2[..n], &mut self.r2[..n]);
        let slot = &mut self.slots[ch];
        if slot.cur.is_none() && slot.old.is_none() {
            return;
        }
        let mut fault = None;
        let mut overrun = None;

        // The incoming (current) instance.
        let cur_live = match slot.cur.as_mut() {
            Some(inst) if !slot.faulted => {
                let (res, over) = inst.render_timed(l, r);
                if over {
                    overrun = Some(inst.stats().last_ns.load(std::sync::atomic::Ordering::Relaxed));
                }
                match res {
                    Ok(()) => true,
                    Err(e) => {
                        slot.faulted = true;
                        fault = Some(e);
                        false
                    }
                }
            }
            _ => false,
        };
        // The outgoing one, while it fades.
        let old_live = match slot.old.as_mut() {
            Some(inst) => inst.render(l2, r2).is_ok(),
            None => false,
        };

        let fading = slot.old.is_some() && slot.fade_len > 0;
        let gain = slot.gain.ramp(n);
        let (bl0, br0) = slot.bal;
        let (bl1, br1) = balance(slot.pan);
        slot.bal = (bl1, br1);
        let (dl, dr) = ((bl1 - bl0) / n.max(1) as f32, (br1 - br0) / n.max(1) as f32);
        let mut peak = slot.peak;
        for i in 0..n {
            let g = gain.at(i);
            let (gl, gr) = (g * (bl0 + dl * (i + 1) as f32), g * (br0 + dr * (i + 1) as f32));
            let (fin, fout) = if fading {
                let p = ((slot.fade_pos as usize + i) as f32 / slot.fade_len as f32).min(1.0);
                (p, 1.0 - p)
            } else {
                (1.0, 0.0)
            };
            let mut sl = 0.0;
            let mut sr = 0.0;
            if cur_live {
                let k = fin * slot.cur_trim;
                sl += l[i] * k;
                sr += r[i] * k;
            }
            if old_live && fout > 0.0 {
                let k = fout * slot.old_trim;
                sl += l2[i] * k;
                sr += r2[i] * k;
            }
            let (yl, yr) = (sl * gl, sr * gr);
            peak = peak.max(yl.abs()).max(yr.abs());
            out_l[i] += yl;
            out_r[i] += yr;
        }
        slot.peak = peak;

        let mut retire = None;
        if slot.old.is_some() {
            slot.fade_pos = slot.fade_pos.saturating_add(n as u32);
            if slot.fade_pos >= slot.fade_len || !old_live {
                retire = slot.old.take();
                slot.fade_len = 0;
            }
        }
        if let Some(old) = retire {
            self.retire(old);
        }
        let channel = ch as u8;
        if let Some(error) = fault {
            self.event(RackEvent::Fault { channel, error });
        }
        let last = self.slots[ch].last_overrun;
        if let Some(ns) = overrun
            && (last == u64::MAX || samples.saturating_sub(last) >= rate as u64)
        {
            self.slots[ch].last_overrun = samples;
            let block_us = (n as f64 * 1e6 / rate) as f32;
            self.event(RackEvent::Overrun { channel, render_us: ns as f32 / 1000.0, block_us });
        }
    }
}

impl Drop for PluginRack {
    /// A rack dropped with instances in it (the audio stream closing) hands them to the
    /// `plugin-dispose` thread rather than disposing of them on whatever thread drops it.
    fn drop(&mut self) {
        for slot in &mut self.slots {
            for inst in [slot.cur.take(), slot.old.take()].into_iter().flatten() {
                dispose_later(inst);
            }
        }
        for inst in self.backlog.iter_mut().filter_map(Option::take) {
            dispose_later(inst);
        }
        // Assigns never applied.
        while let Ok(cmd) = self.rx.pop() {
            if let RackCmd::Assign { inst, .. } = cmd {
                dispose_later(inst);
            }
        }
    }
}
