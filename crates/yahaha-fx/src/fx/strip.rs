//! The channel strip's chain on the SoundFont side (the mixer rework): every MIDI
//! channel's compressor, insert 1 and insert 2, run in that order on the part's stem by the
//! synth's rack, after the part's EQ and before its meters, sends and the mix.
//!
//! [`StripControl`] (in `FxControl::strips`) carries each channel's compressor and insert
//! slots from the control side: atomics only. [`ChannelInserts`] takes them once per
//! buffer (`set`, then `set_strips`) and runs the chain (`process`). Insert 1's kind, on
//! and amount come from the style's XG insert or the keyboard part's own slot
//! ([`super::PartInsert`], through `set`); its settings 2-4 and the whole of insert 2 come
//! from [`StripControl`].

use super::insert::{Insert, InsertKind, InsertSettings, KIND_DEFAULT};
use super::kinds::{INSERT_VALUES, InsertSlot, InsertType};
use super::part_comp::{PartComp, PartCompCell, PartCompDsp};
use std::sync::atomic::{AtomicU16, AtomicU64, Ordering::Relaxed};

/// MIDI channels.
const CHANNELS: usize = 16;

/// One insert slot's settings from the control side to the audio thread: one atomic
/// packing its kind (`InsertKind as u8`), on and its four values (10 bits each; the values
/// are at most 1000).
pub struct InsertCell(AtomicU64);

impl Default for InsertCell {
    fn default() -> InsertCell {
        InsertCell::new()
    }
}

impl InsertCell {
    /// Empty and off.
    pub const fn new() -> InsertCell {
        InsertCell(AtomicU64::new(0))
    }

    /// Its settings (an unknown kind is kept as an empty one: it plays dry).
    pub fn set(&self, slot: &InsertSlot) {
        let mut v = slot.kind.kind() as u64 | (slot.on as u64) << 8;
        for (i, x) in slot.values.iter().enumerate() {
            v |= ((*x).min(1023) as u64) << (16 + 10 * i);
        }
        self.0.store(v, Relaxed);
    }

    fn bits(&self) -> (InsertKind, bool, [u16; INSERT_VALUES]) {
        let v = self.0.load(Relaxed);
        let values = std::array::from_fn(|i| ((v >> (16 + 10 * i)) & 1023) as u16);
        (InsertKind::from_u8(v as u8), (v >> 8) & 1 == 1, values)
    }

    /// Its settings as set (control side).
    pub fn get(&self) -> InsertSlot {
        let (kind, on, values) = self.bits();
        InsertSlot { kind: InsertType::from(kind), on, values }
    }

    /// What the audio thread plays for it at `bpm`, fast or slow (off: nothing). Never
    /// allocates.
    pub fn settings(&self, bpm: f32, fast: bool) -> InsertSettings {
        let (kind, on, v) = self.bits();
        InsertSettings { kind: if on { kind } else { InsertKind::None }, amount: v[0].min(127) as u8, rest: [v[1], v[2], v[3]], bpm, fast }
    }
}

/// Each channel's strip settings from the control side, read once per buffer by the audio
/// thread: atomics only. Channels are MIDI channels 0-15; an out-of-range channel is
/// ignored (a setter does nothing, a reader gives the default).
pub struct StripControl {
    /// The compressor.
    comp: [PartCompCell; CHANNELS],
    /// Insert 1's settings 2-4, [`KIND_DEFAULT`] until set (its kind, on and amount come
    /// from the style's insert or the keyboard part's `PartInsert`).
    first_rest: [[AtomicU16; 3]; CHANNELS],
    /// Insert 2, whole.
    second: [InsertCell; CHANNELS],
}

impl StripControl {
    /// Every compressor off, insert 1's settings 2-4 at the kind's defaults, insert 2
    /// empty.
    pub fn new() -> StripControl {
        StripControl {
            comp: std::array::from_fn(|_| PartCompCell::new()),
            first_rest: std::array::from_fn(|_| std::array::from_fn(|_| AtomicU16::new(KIND_DEFAULT))),
            second: std::array::from_fn(|_| InsertCell::new()),
        }
    }

    /// Channel `ch`'s compressor.
    pub fn set_comp(&self, ch: usize, c: &PartComp) {
        if let Some(cell) = self.comp.get(ch) {
            cell.set(c);
        }
    }

    /// Insert 1's settings 2-4 on channel `ch` ([`KIND_DEFAULT`]: the kind's default).
    pub fn set_first_rest(&self, ch: usize, rest: [u16; 3]) {
        if let Some(cells) = self.first_rest.get(ch) {
            for (cell, v) in cells.iter().zip(rest) {
                cell.store(v, Relaxed);
            }
        }
    }

    /// Insert 1's settings 2-4 on channel `ch` from its slot (its kind, on and first value
    /// come through `PartInsert`).
    pub fn set_first(&self, ch: usize, slot: &InsertSlot) {
        self.set_first_rest(ch, [slot.values[1], slot.values[2], slot.values[3]]);
    }

    /// Insert 2 on channel `ch`.
    pub fn set_second(&self, ch: usize, slot: &InsertSlot) {
        if let Some(cell) = self.second.get(ch) {
            cell.set(slot);
        }
    }

    /// Channel `ch`'s compressor (its type reads as Natural).
    pub fn comp(&self, ch: usize) -> PartComp {
        self.comp.get(ch).map_or_else(PartComp::default, PartCompCell::get)
    }

    /// Insert 1's settings 2-4 on channel `ch`.
    pub fn first_rest(&self, ch: usize) -> [u16; 3] {
        self.first_rest.get(ch).map_or([KIND_DEFAULT; 3], |c| [c[0].load(Relaxed), c[1].load(Relaxed), c[2].load(Relaxed)])
    }

    /// Insert 2 on channel `ch`, as set (control side).
    pub fn second(&self, ch: usize) -> InsertSlot {
        self.second.get(ch).map_or_else(InsertSlot::default, InsertCell::get)
    }

    /// What insert 2 on channel `ch` plays at `bpm`, fast or slow (the audio thread, the
    /// plugin rack). Never allocates.
    pub fn second_settings(&self, ch: usize, bpm: f32, fast: bool) -> InsertSettings {
        self.second.get(ch).map_or(InsertSettings { bpm, fast, ..InsertSettings::NONE }, |c| c.settings(bpm, fast))
    }
}

impl Default for StripControl {
    fn default() -> StripControl {
        StripControl::new()
    }
}

/// Every MIDI channel's strip chain on the SoundFont side, run on its part's stem by the
/// synth's rack (audio thread; everything allocated in `new`): the compressor, insert 1
/// (the Style parts' XG insert on channels 9-16, the keyboard parts' own slot on channels
/// 1-4) and insert 2, in that order (`super::INSERT_SLOTS`). A stage that is off and
/// settled isn't run, so it leaves the stem bit-identical.
///
/// A channel another rack plays (a plugin, whose own strip runs there) is skipped
/// ([`ChannelInserts::set_skip`]): every stage of its chain here is off, whatever the
/// strip says, so once its glide or fade out is done it does no work at all.
pub struct ChannelInserts {
    comp: [PartCompDsp; CHANNELS],
    comps: [PartComp; CHANNELS],
    slots: [Insert; CHANNELS],
    settings: [InsertSettings; CHANNELS],
    second: [Insert; CHANNELS],
    second_settings: [InsertSettings; CHANNELS],
    /// The skipped channels (bit = channel).
    skip: u16,
}

impl ChannelInserts {
    pub fn new(rate: f32) -> ChannelInserts {
        ChannelInserts {
            comp: [PartCompDsp::new(rate); CHANNELS],
            comps: [PartComp::default(); CHANNELS],
            slots: std::array::from_fn(|_| Insert::new(rate)),
            settings: [InsertSettings::NONE; CHANNELS],
            second: std::array::from_fn(|_| Insert::new(rate)),
            second_settings: [InsertSettings::NONE; CHANNELS],
            skip: 0,
        }
    }

    /// The channels (bit = channel) whose chain here is off, whatever their strip says:
    /// the ones a plugin plays, its strip in the plugin rack. Set it each buffer before
    /// `set`, `set_strips` and `set_second`, which honour it. A skipped channel's
    /// compressor glides back to unity as one turned off does and then stops; its inserts
    /// fade out and stop. Once it is no longer skipped, its compressor starts from rest,
    /// as one turned on does. Never allocates.
    pub fn set_skip(&mut self, mask: u16) {
        self.skip = mask;
    }

    fn skipped(&self, ch: usize) -> bool {
        self.skip >> ch & 1 == 1
    }

    /// Take insert 1's settings for this buffer (`InsertSettings::channels`).
    pub fn set(&mut self, settings: &[InsertSettings; 16]) {
        self.settings = *settings;
        for ch in 0..CHANNELS {
            if self.skipped(ch) {
                self.settings[ch].kind = InsertKind::None;
            }
        }
    }

    /// Take the rest of each channel's strip for this buffer, after `set`: its compressor,
    /// insert 1's settings 2-4 (each one not at [`KIND_DEFAULT`] overrides what `set`
    /// took) and insert 2 (at insert 1's tempo and rotary speed). Never allocates.
    pub fn set_strips(&mut self, strips: &StripControl) {
        for ch in 0..CHANNELS {
            let skip = self.skipped(ch);
            self.comps[ch] = strips.comp(ch);
            self.comps[ch].on &= !skip;
            let s = &mut self.settings[ch];
            for (r, v) in s.rest.iter_mut().zip(strips.first_rest(ch)) {
                if v != KIND_DEFAULT {
                    *r = v;
                }
            }
            self.second_settings[ch] = strips.second_settings(ch, s.bpm, s.fast);
            if skip {
                self.second_settings[ch].kind = InsertKind::None;
            }
        }
    }

    /// Take insert 2's settings for this buffer (overriding what `set_strips` took).
    pub fn set_second(&mut self, settings: &[InsertSettings; 16]) {
        self.second_settings = *settings;
        for ch in 0..CHANNELS {
            if self.skipped(ch) {
                self.second_settings[ch].kind = InsertKind::None;
            }
        }
    }

    /// Insert 2's settings as last taken.
    pub fn second(&self) -> &[InsertSettings; 16] {
        &self.second_settings
    }

    /// The channels (bit = channel) whose stem runs through `process` before the mix: the
    /// compressor on or gliding off, or either insert playing or fading out.
    pub fn mask(&self) -> u16 {
        let mut m = 0;
        for ch in 0..CHANNELS {
            if self.comp[ch].active(&self.comps[ch]) || self.slots[ch].active(self.settings[ch].kind) || self.second[ch].active(self.second_settings[ch].kind) {
                m |= 1 << ch;
            }
        }
        m
    }

    /// Run channel `channel`'s stem (`left`/`right`) through its chain in place. `level`
    /// is the part's gain in the mix (volume x expression, squared, x the master volume):
    /// every stage sees the part as if at full volume.
    pub fn process(&mut self, channel: usize, left: &mut [f32], right: &mut [f32], level: f32) {
        if channel >= CHANNELS {
            return;
        }
        // The top view (#296) shows the Style parts' inserts: the whole chain.
        let band = channel.checked_sub(super::BAND_CHANNELS.start).filter(|&p| p < 8);
        let perf = &yahaha_core::perf::PERF;
        let t0 = band.is_some_and(|_| perf.on()).then(yahaha_core::rt::host_now);
        self.comp[channel].process(left, right, level, &self.comps[channel]);
        let (slot, s) = (&mut self.slots[channel], &self.settings[channel]);
        if slot.active(s.kind) {
            slot.process(left, right, level, s);
        }
        let (slot, s) = (&mut self.second[channel], &self.second_settings[channel]);
        if slot.active(s.kind) {
            slot.process(left, right, level, s);
        }
        // Its time and output peak, atomics only.
        if let (Some(t0), Some(p)) = (t0, band) {
            perf.insert[p].add(yahaha_core::rt::host_to_ns(yahaha_core::rt::host_now().wrapping_sub(t0)));
            let peak = left.iter().chain(right.iter()).fold(0f32, |m, x| m.max(x.abs()));
            yahaha_core::perf::Perf::peak(&perf.insert_peak[p], peak);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fx::master::CompPreset;
    use std::f32::consts::TAU;

    const RATE: f32 = 48_000.0;
    const BUF: usize = 256;

    fn sine(hz: f32, amp: f32, n: usize) -> Vec<f32> {
        (0..n).map(|i| (TAU * hz * i as f32 / RATE).sin() * amp).collect()
    }

    /// Run `x` through channel `ch` buffer by buffer, the same on both sides.
    fn run(ci: &mut ChannelInserts, ch: usize, x: &[f32], level: f32) -> Vec<f32> {
        let (mut l, mut r) = (x.to_vec(), x.to_vec());
        for (lb, rb) in l.chunks_mut(BUF).zip(r.chunks_mut(BUF)) {
            ci.process(ch, lb, rb, level);
        }
        assert_eq!(l, r);
        l
    }

    fn by_hand(x: &[f32], level: f32, c: &PartComp, s1: &InsertSettings, s2: &InsertSettings, first_then_second: bool) -> Vec<f32> {
        let (mut comp, mut a, mut b) = (PartCompDsp::new(RATE), Insert::new(RATE), Insert::new(RATE));
        let (mut l, mut r) = (x.to_vec(), x.to_vec());
        for (lb, rb) in l.chunks_mut(BUF).zip(r.chunks_mut(BUF)) {
            comp.process(lb, rb, level, c);
            if first_then_second {
                a.process(lb, rb, level, s1);
                b.process(lb, rb, level, s2);
            } else {
                b.process(lb, rb, level, s2);
                a.process(lb, rb, level, s1);
            }
        }
        l
    }

    fn slot(kind: InsertType, values: [u16; 4]) -> InsertSlot {
        InsertSlot { kind, on: true, values }
    }

    #[test]
    fn a_compressor_off_leaves_the_stem_bit_identical_even_after_it_was_on() {
        let x = sine(220.0, 0.8, 4 * BUF);
        let strips = StripControl::new();
        let mut ci = ChannelInserts::new(RATE);
        let mut tremolo = [InsertSettings::NONE; 16];
        tremolo[3].kind = InsertKind::Tremolo;
        // Never on: nothing runs, and with an insert the output is the insert's alone.
        ci.set(&[InsertSettings::NONE; 16]);
        ci.set_strips(&strips);
        assert_eq!(ci.mask(), 0);
        assert_eq!(run(&mut ci, 2, &x, 0.5), x);
        ci.set(&tremolo);
        ci.set_strips(&strips);
        let mut alone = Insert::new(RATE);
        let (mut l, mut r) = (x.clone(), x.clone());
        for (lb, rb) in l.chunks_mut(BUF).zip(r.chunks_mut(BUF)) {
            alone.process(lb, rb, 0.5, &tremolo[3]);
        }
        assert_eq!(run(&mut ci, 3, &x, 0.5), l);
        // On, then off: once settled, the stem is its own again.
        let mut ci = ChannelInserts::new(RATE);
        ci.set(&[InsertSettings::NONE; 16]);
        strips.set_comp(2, &PartComp::of(true, CompPreset::Loud));
        ci.set_strips(&strips);
        assert_eq!(ci.mask(), 1 << 2);
        assert_ne!(run(&mut ci, 2, &x, 0.5), x);
        strips.set_comp(2, &PartComp::of(false, CompPreset::Loud));
        ci.set_strips(&strips);
        let mut buffers = 0;
        while ci.mask() != 0 {
            run(&mut ci, 2, &x[..BUF], 0.5);
            buffers += 1;
            assert!(buffers < 200, "never settled");
        }
        assert_eq!(run(&mut ci, 2, &x, 0.5), x);
    }

    /// A skipped channel (a plugin plays it) does no work here, whatever its strip says: its
    /// compressor and inserts go off and settle, it drops out of `mask`, and its stem passes
    /// bit-identical. Other channels run on. Unskipped, its compressor starts from rest,
    /// exactly as a fresh one does.
    #[test]
    fn a_skipped_channel_runs_nothing_and_starts_clean_when_unskipped() {
        let x = sine(220.0, 0.8, 4 * BUF);
        let strips = StripControl::new();
        let loud = PartComp::of(true, CompPreset::Loud);
        strips.set_comp(4, &loud);
        strips.set_comp(6, &loud);
        strips.set_second(4, &slot(InsertType::Distortion, [120, 40, 60, 80]));
        let mut first = [InsertSettings::NONE; 16];
        first[4].kind = InsertKind::Tremolo;
        let take = |ci: &mut ChannelInserts, skip: u16| {
            ci.set_skip(skip);
            ci.set(&first);
            ci.set_strips(&strips);
            let second = *ci.second();
            ci.set_second(&second);
        };
        let mut ci = ChannelInserts::new(RATE);
        take(&mut ci, 0);
        assert_eq!(ci.mask(), 1 << 4 | 1 << 6);
        assert_ne!(run(&mut ci, 4, &x, 0.5), x);
        // Skipped (a plugin took the channel): it glides and fades out, then stops.
        take(&mut ci, 1 << 4);
        let mut buffers = 0;
        while ci.mask() != 1 << 6 {
            run(&mut ci, 4, &x[..BUF], 0.5);
            buffers += 1;
            assert!(buffers < 200, "never settled");
        }
        assert_eq!(ci.mask() & 1 << 4, 0);
        assert_eq!(run(&mut ci, 4, &x, 0.5), x, "a skipped channel's stem is untouched");
        // Skipped from the start: never in the mask at all.
        let mut fresh = ChannelInserts::new(RATE);
        take(&mut fresh, 1 << 4);
        assert_eq!(fresh.mask(), 1 << 6);
        // Unskipped (the plugin let go): the whole chain runs again.
        take(&mut ci, 0);
        assert_eq!(ci.mask(), 1 << 4 | 1 << 6);
        assert_ne!(run(&mut ci, 4, &x, 0.5), x);
        // A compressor alone (channel 6), skipped mid-squash and then unskipped: the same
        // samples as one that was never on, turned on now.
        let x = sine(110.0, 0.9, 8 * BUF);
        run(&mut ci, 6, &x, 0.5);
        assert!(ci.comp[6].gain_reduction_db() < -1.0, "squashing");
        take(&mut ci, 1 << 6);
        let mut buffers = 0;
        while ci.mask() & 1 << 6 != 0 {
            run(&mut ci, 6, &x[..BUF], 0.5);
            buffers += 1;
            assert!(buffers < 200, "never settled");
        }
        take(&mut ci, 0);
        let mut never = ChannelInserts::new(RATE);
        take(&mut never, 0);
        assert_eq!(run(&mut ci, 6, &x, 0.5), run(&mut never, 6, &x, 0.5), "the compressor starts from rest");
    }

    #[test]
    fn the_chain_runs_compressor_then_insert_1_then_insert_2() {
        let x = sine(330.0, 0.9, 8 * BUF);
        let level = 0.6;
        let comp = PartComp::of(true, CompPreset::Punchy);
        let strips = StripControl::new();
        strips.set_comp(5, &comp);
        strips.set_first_rest(5, [3, KIND_DEFAULT, 90]);
        strips.set_second(5, &slot(InsertType::Distortion, [120, 40, 60, 80]));
        let mut first = [InsertSettings { bpm: 132.0, fast: true, ..InsertSettings::NONE }; 16];
        first[5] = InsertSettings { kind: InsertKind::Tremolo, amount: 100, ..first[5] };
        let mut ci = ChannelInserts::new(RATE);
        ci.set(&first);
        ci.set_strips(&strips);
        assert_eq!(ci.mask(), 1 << 5);
        let s1 = InsertSettings { rest: [3, KIND_DEFAULT, 90], ..first[5] };
        let s2 = slot(InsertType::Distortion, [120, 40, 60, 80]).settings(132.0, true);
        let got = run(&mut ci, 5, &x, level);
        assert_eq!(got, by_hand(&x, level, &strips.comp(5), &s1, &s2, true));
        // Insert 2's distortion before insert 1's tremolo sounds different.
        assert_ne!(got, by_hand(&x, level, &strips.comp(5), &s1, &s2, false));
    }

    #[test]
    fn an_empty_insert_2_costs_nothing() {
        let x = sine(440.0, 0.7, 4 * BUF);
        let mut first = [InsertSettings::NONE; 16];
        first[9].kind = InsertKind::Distortion;
        let strips = StripControl::new();
        strips.set_second(1, &slot(InsertType::None, [0; 4]));
        let mut ci = ChannelInserts::new(RATE);
        ci.set(&first);
        ci.set_strips(&strips);
        assert_eq!(ci.mask(), 1 << 9);
        let mut alone = Insert::new(RATE);
        let (mut l, mut r) = (x.clone(), x.clone());
        for (lb, rb) in l.chunks_mut(BUF).zip(r.chunks_mut(BUF)) {
            alone.process(lb, rb, 0.8, &first[9]);
        }
        assert_eq!(run(&mut ci, 9, &x, 0.8), l);
        assert_eq!(run(&mut ci, 1, &x, 0.8), x);
    }

    #[test]
    fn set_strips_carries_insert_1s_settings_and_insert_2() {
        let strips = StripControl::new();
        let second = slot(InsertType::Phaser, [127, 500, 90, 0]);
        strips.set_second(7, &second);
        strips.set_first(7, &slot(InsertType::AutoWah, [10, 20, KIND_DEFAULT, 30]));
        let comp = PartComp { threshold: -40, ratio: 200, attack: 1, release: 1000, makeup: 24, ..PartComp::of(true, CompPreset::Loud) };
        strips.set_comp(7, &comp);
        // The cells round-trip.
        assert_eq!(strips.second(7), second);
        assert_eq!(strips.first_rest(7), [20, KIND_DEFAULT, 30]);
        assert_eq!(strips.comp(7), PartComp { preset: CompPreset::Natural, ..comp });
        // Out of range: ignored, read as the defaults.
        strips.set_second(16, &second);
        strips.set_first_rest(99, [1, 2, 3]);
        strips.set_comp(16, &comp);
        assert_eq!(strips.second(16), InsertSlot::default());
        assert_eq!(strips.first_rest(16), [KIND_DEFAULT; 3]);
        assert_eq!(strips.comp(16), PartComp::default());
        // Through `set_strips`: insert 1's rest overridden where set, insert 2 at insert
        // 1's tempo and speed.
        let mut first = [InsertSettings { bpm: 90.0, fast: true, ..InsertSettings::NONE }; 16];
        first[7] = InsertSettings { kind: InsertKind::AutoWah, amount: 10, rest: [1, 2, 3], ..first[7] };
        let mut ci = ChannelInserts::new(RATE);
        ci.set(&first);
        ci.set_strips(&strips);
        assert_eq!(ci.settings[7].rest, [20, 2, 30]);
        assert_eq!(ci.second()[7], second.settings(90.0, true));
        assert_eq!(ci.comps[7], strips.comp(7));
        // An insert 2 turned off plays nothing.
        strips.set_second(7, &InsertSlot { on: false, ..second });
        ci.set_strips(&strips);
        assert_eq!(ci.second()[7].kind, InsertKind::None);
    }
}
