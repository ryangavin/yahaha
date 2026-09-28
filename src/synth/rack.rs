//! The built-in synth's SoundFont parts (#346 step 1): one `rustysynth::Synthesizer` per
//! part, all sharing the SoundFont (`Arc<SoundFont>`), with rustysynth's own reverb and
//! chorus off (`SynthesizerSettings::enable_reverb_and_chorus`).
//!
//! Each MIDI channel is a part and renders its own stereo stem. Everything that works on a
//! part's signal runs here, on its stem, in yahaha code: the Style parts' insertion effects
//! (#269), the meters (peak and RMS), the sends into the effect bus (#204) and the
//! performance view's per-part cost. Upstream rustysynth has no public voice count, so the
//! notes a part holds are counted here, from the messages it gets ([`Part::track`]).
//!
//! A lane (a part's synthesizer) is rendered while its part holds a note on it or its
//! output rings on; once it has been silent for `IDLE_FRAMES` with no note held, it is
//! skipped until a note-on (or a route to it) wakes it, so the parts not playing cost
//! nothing.
//!
//! A part can have more than one lane:
//! - one per SoundFont in the rack (the sound library, #103; routing.rs): notes start on
//!   the lane of the font the channel plays now; the others only ring out;
//! - on the main font, a few lanes for drum notes with sends of their own (an XG Drum Setup,
//!   #239): a note whose sends differ from the part's plays in a lane with those sends, so
//!   its stem can be sent at its own level. Such a lane is claimed while idle and rendered
//!   only while it sounds.
//!
//! Every lane of a part gets the part's controllers, pitch bend and note-offs; a note-on,
//! program change or bank select goes to the lanes of the font the channel plays now.
//!
//! All memory is allocated when the rack is built, off the audio thread; `render` never
//! allocates, locks or panics.

use super::*;
use crate::fx::BUSES;

/// Voices per part on the main SoundFont: as many as the band's and your playing's shared
/// synthesizers had each, so a part never has fewer than before.
const MAIN_POLYPHONY: usize = 128;
/// Voices per part on an extra SoundFont (the sound library, #103).
const EXTRA_POLYPHONY: usize = 64;
/// Lanes per part for drum notes with sends of their own, and their voices each.
const NOTE_LANES: usize = 4;
const NOTE_POLYPHONY: usize = 32;
/// A lane's sends when they are the part's own.
const PART_SENDS: [f32; BUSES] = [1.0; BUSES];

/// A lane with no note held is rendered until its output has been below `IDLE_LEVEL` for
/// this long (a tail ringing out), then skipped.
pub(super) const IDLE_FRAMES: u32 = 4800;
const IDLE_LEVEL: f32 = 1e-6;

/// One synthesizer of a part.
pub(super) struct Lane {
    synth: Synthesizer,
    /// The rack slot (SoundFont) it plays: 0 = the main font.
    slot: u8,
    /// Its notes' sends, as a factor on the part's (1: the part's own).
    sends: [f32; BUSES],
    /// Frames it has rendered below `IDLE_LEVEL` (a note-on starts it again).
    pub(super) quiet: u32,
}

/// A part (a MIDI channel): its lanes, and what the rack keeps of its state.
pub(super) struct Part {
    /// Slot k's lane at k (0 = the main font); then the main font's note lanes.
    pub(super) lanes: Vec<Lane>,
    /// CC7/39 and CC11/43, 14 bits each, as rustysynth keeps them (for an insert's level).
    volume: i32,
    expression: i32,
    /// Keys held down, and keys let go under the hold pedal (bit = key).
    held: u128,
    sustained: u128,
    pedal: bool,
}

impl Part {
    fn new(lanes: Vec<Lane>) -> Part {
        Part { lanes, volume: 100 << 7, expression: 127 << 7, held: 0, sustained: 0, pedal: false }
    }

    /// Follow a message to this part, as rustysynth's channel does: its volume and
    /// expression, and the keys it holds.
    #[inline]
    fn track(&mut self, st: i32, d1: i32, d2: i32) {
        let bit = 1u128 << (d1 & 127);
        let v = d2 & 127;
        match st {
            0x90 if d2 > 0 => {
                self.held |= bit;
                self.sustained &= !bit;
            }
            0x80 | 0x90 => {
                if self.held & bit != 0 {
                    self.held &= !bit;
                    if self.pedal {
                        self.sustained |= bit;
                    }
                }
            }
            0xB0 => match d1 {
                7 => self.volume = (self.volume & 0x7F) | (v << 7),
                39 => self.volume = (self.volume & !0x7F) | v,
                11 => self.expression = (self.expression & 0x7F) | (v << 7),
                43 => self.expression = (self.expression & !0x7F) | v,
                64 => {
                    self.pedal = d2 >= 64;
                    if !self.pedal {
                        self.sustained = 0;
                    }
                }
                // Reset All Controllers: expression and the pedal (rustysynth's; not volume).
                121 => {
                    self.expression = 127 << 7;
                    self.pedal = false;
                    self.sustained = 0;
                }
                // All Sound Off, All Notes Off, Mono / Poly (with All Notes Off).
                120 | 123 | 126 | 127 => {
                    self.held = 0;
                    self.sustained = 0;
                }
                _ => {}
            },
            _ => {}
        }
    }

    /// The notes it sounds: held, or kept by the hold pedal. (Upstream rustysynth has no
    /// public voice count; release tails are not counted.)
    #[inline]
    fn notes(&self) -> u32 {
        (self.held | self.sustained).count_ones()
    }

    /// Its gain in the mix, as rustysynth applies it: (volume x expression)², before the
    /// master volume.
    #[inline]
    fn level(&self) -> f32 {
        let ve = (self.volume as f32 / 16383.0) * (self.expression as f32 / 16383.0);
        ve * ve
    }
}

/// The built-in synth's SoundFont parts: one synthesizer per part (see the module docs).
pub struct Rack {
    pub(super) parts: Vec<Part>,
    /// SoundFonts in the rack: the main one and the extras (slot k = `lanes[k]`).
    pub(super) slots: usize,
    /// Font id (`patches::Route`) -> slot (0 = the main font), `NO_SLOT` if not here.
    pub(super) slot_of: [u8; crate::patches::route::MAX_FONTS],
    /// The slot each channel plays now.
    pub(super) ch_slot: [u8; 16],
    /// Channels routed to a library patch (bit = channel).
    pub(super) mapped: u16,
    /// Each channel's gains into the effect bus's send buses (#204), taken on its stem.
    send_gains: [[f32; BUSES]; 16],
    /// The master volume every lane renders at (it is in the stems).
    master: f32,
    /// A part's stem, and a second lane's block before it joins the stem.
    stem_l: Vec<f32>,
    stem_r: Vec<f32>,
    tmp_l: Vec<f32>,
    tmp_r: Vec<f32>,
    /// The performance view is on: `render` times each part and `report_profile` hands the
    /// costs to `perf::PERF`.
    profile: bool,
    /// Each channel's render time in the last `render` (profiling only).
    ch_ns: [u64; 16],
    /// Each channel's RMS in the last `render` (after a fade), for the meters: the audio
    /// thread folds it into `SynthControl::rms`.
    pub rms: [f32; 16],
}

/// The settings every lane is built with. `legacy`: rustysynth's own reverb and chorus on
/// (the sound before the effect bus, #204); otherwise off.
fn settings(sample_rate: i32, polyphony: usize, legacy: bool) -> SynthesizerSettings {
    let mut s = SynthesizerSettings::new(sample_rate);
    s.maximum_polyphony = polyphony;
    s.enable_reverb_and_chorus = legacy;
    s.velocity_to_filter = velocity_to_filter();
    s
}

impl Rack {
    /// A rack on `font`: its reverb and chorus are the effect bus's.
    pub fn new(font: &Arc<SoundFont>, sample_rate: i32) -> Result<Rack> {
        Rack::build(font, &[], sample_rate, false)
    }

    /// A rack on `font` whose synthesizers play the SoundFont's own reverb and chorus
    /// (`YAHAHA_FX=legacy`, for before/after listening; `FxControl::legacy` turns the bus
    /// off to go with it).
    pub fn new_legacy(font: &Arc<SoundFont>, sample_rate: i32) -> Result<Rack> {
        Rack::build(font, &[], sample_rate, true)
    }

    /// Every part's lanes on `main` and `extras` (slots 1..). Slow: off the audio thread.
    pub(super) fn build(main: &Arc<SoundFont>, extras: &[&Arc<SoundFont>], sample_rate: i32, legacy: bool) -> Result<Rack> {
        let lane = |font: &Arc<SoundFont>, slot: u8, polyphony: usize| -> Result<Lane> {
            let synth = Synthesizer::new(font, &settings(sample_rate, polyphony, legacy)).map_err(|e| anyhow!("{e:?}"))?;
            Ok(Lane { synth, slot, sends: PART_SENDS, quiet: IDLE_FRAMES })
        };
        let mut parts = Vec::with_capacity(16);
        for _ in 0..16 {
            let mut lanes = Vec::with_capacity(1 + extras.len() + NOTE_LANES);
            lanes.push(lane(main, 0, MAIN_POLYPHONY)?);
            for (i, f) in extras.iter().enumerate() {
                lanes.push(lane(f, i as u8 + 1, EXTRA_POLYPHONY)?);
            }
            for _ in 0..NOTE_LANES {
                lanes.push(lane(main, 0, NOTE_POLYPHONY)?);
            }
            parts.push(Part::new(lanes));
        }
        let mut r = Rack {
            parts,
            slots: 1 + extras.len(),
            slot_of: [routing::NO_SLOT; crate::patches::route::MAX_FONTS],
            ch_slot: [0; 16],
            mapped: 0,
            send_gains: [[0.0; BUSES]; 16],
            master: 0.5,
            stem_l: vec![0.0; 8192],
            stem_r: vec![0.0; 8192],
            tmp_l: vec![0.0; 8192],
            tmp_r: vec![0.0; 8192],
            profile: false,
            ch_ns: [0; 16],
            rms: [0.0; 16],
        };
        // Rhythm 1 (ch 9) is a drum part too: on the drum bank, on every font.
        for l in &mut r.parts[8].lanes {
            l.synth.process_midi_message(8, 0xB0, 0, 128);
        }
        r.set_master_volume(r.master);
        Ok(r)
    }

    /// Load a SoundFont into a new rack (slow: call it off the audio thread).
    pub fn load(sf2: &Path, sample_rate: u32) -> Result<Box<Rack>> {
        Ok(Box::new(Rack::new(&Rack::read(sf2)?, sample_rate as i32)?))
    }

    /// `load`, the SoundFont's own reverb and chorus on (`new_legacy`).
    pub fn load_legacy(sf2: &Path, sample_rate: u32) -> Result<Box<Rack>> {
        Ok(Box::new(Rack::new_legacy(&Rack::read(sf2)?, sample_rate as i32)?))
    }

    fn read(sf2: &Path) -> Result<Arc<SoundFont>> {
        let mut file = std::fs::File::open(sf2).with_context(|| format!("opening {}", sf2.display()))?;
        Ok(Arc::new(SoundFont::new(&mut file).map_err(|e| anyhow!("{e:?}"))?))
    }

    /// Measure for the performance view (`perf`), or stop.
    pub(super) fn set_timing(&mut self, on: bool) {
        self.profile = on;
    }

    /// The last render's per-channel costs and notes into `perf::PERF` (profiling only),
    /// leaving out the costs of the channels in `skip` (a plugin plays them).
    pub(super) fn report_profile(&mut self, skip: u16) {
        let perf = &crate::perf::PERF;
        let mut total = 0;
        for ch in 0..16 {
            let n = self.parts[ch].notes();
            total += n;
            perf.channel_notes[ch].store(n, Relaxed);
            if skip >> ch & 1 == 0 {
                perf.channel[ch].add(self.ch_ns[ch]);
            }
        }
        perf.voices.store(total, Relaxed);
        perf.voices_peak.fetch_max(total, Relaxed);
    }

    /// The notes sounding now, in every part (held or kept by the pedal; see `Part::notes`).
    pub fn voices(&self) -> usize {
        self.parts.iter().map(|p| p.notes() as usize).sum()
    }

    /// Each channel's gains into the effect bus's send buses (#204).
    pub(super) fn set_sends(&mut self, gains: &[[f32; BUSES]; 16]) {
        self.send_gains = *gains;
    }

    /// A channel's mono or poly mode (#246), on every lane (as a controller).
    pub(super) fn set_mono(&mut self, ch: u8, mono: bool) {
        for l in &mut self.parts[ch as usize & 15].lanes {
            l.synth.set_mono(ch as i32, mono);
        }
    }

    pub(super) fn set_master_volume(&mut self, v: f32) {
        self.master = v;
        for p in &mut self.parts {
            for l in &mut p.lanes {
                l.synth.set_master_volume(v);
            }
        }
    }

    /// A channel message to the lane(s) that take it (see the module docs).
    pub(super) fn process(&mut self, ch: i32, st: i32, d1: i32, d2: i32) {
        let c = ch as usize & 15;
        let slot = self.ch_slot[c];
        let part = &mut self.parts[c];
        part.track(st, d1, d2);
        match st {
            0x90 if d2 > 0 => {
                let l = &mut part.lanes[slot as usize];
                l.quiet = 0;
                l.synth.process_midi_message(ch, st, d1, d2);
            }
            0xC0 => part.slot_lanes(slot).for_each(|l| l.synth.process_midi_message(ch, st, d1, d2)),
            0xB0 if d1 == 0 || d1 == 32 => part.slot_lanes(slot).for_each(|l| l.synth.process_midi_message(ch, st, d1, d2)),
            _ => part.lanes.iter_mut().for_each(|l| l.synth.process_midi_message(ch, st, d1, d2)),
        }
    }

    /// A note-on on `ch` whose voices start with `note`'s own settings (a drum setup's,
    /// #239), on the font the channel plays. Sends of its own put it in a note lane.
    pub(super) fn note_on_with(&mut self, ch: u8, key: u8, velocity: u8, note: &rustysynth::NoteParams) {
        let c = ch as usize & 15;
        let slot = self.ch_slot[c];
        let slots = self.slots;
        let part = &mut self.parts[c];
        part.track(0x90, key as i32, velocity as i32);
        let i = if slot == 0 && note.sends != PART_SENDS { part.note_lane(slots, note.sends) } else { slot as usize };
        let l = &mut part.lanes[i];
        l.quiet = 0;
        l.synth.note_on_with(ch as i32, key as i32, velocity as i32, note);
    }

    /// Channel `ch`'s bank and program on the lanes of `slot` (a route, #103).
    pub(super) fn program_on(&mut self, ch: u8, slot: u8, bank: i32, program: i32) {
        let part = &mut self.parts[ch as usize & 15];
        for l in part.slot_lanes(slot) {
            l.synth.process_midi_message(ch as i32, 0xB0, 0, bank);
            l.synth.process_midi_message(ch as i32, 0xC0, program, 0);
        }
        // The slot's synthesizer renders again (until it has gone quiet once more).
        if let Some(l) = part.lanes.get_mut(slot as usize) {
            l.quiet = 0;
        }
    }

    /// Render `left.len()` frames of the mix into `left`/`right` and the effect bus's send
    /// buses into `sends` (all overwritten; bus b's left side at `2 * b * n`, its right at
    /// `(2 * b + 1) * n`), noting each channel's peak in `peaks`. `fade` ramps the whole
    /// from one gain to another over the buffer. `inserts`: the Style parts' insertion
    /// effects (#269), each on its part's stem before the meters, the sends and the mix.
    pub(super) fn render(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        sends: &mut [f32],
        peaks: &[AtomicU32; 16],
        fade: Option<(f32, f32)>,
        mut inserts: Option<&mut crate::fx::BandInserts>,
    ) {
        let n = left.len().min(self.tmp_l.len()).min(sends.len() / (2 * BUSES));
        let (left, right) = (&mut left[..n], &mut right[..n]);
        let sends = &mut sends[..2 * BUSES * n];
        left.fill(0.0);
        right.fill(0.0);
        sends.fill(0.0);
        let insert_mask = inserts.as_deref().map_or(0, |i| i.mask());
        let profile = self.profile;
        let mut clock = crate::perf::Lap::start(profile);
        // Time in the extra fonts' lanes, charged to their own stage.
        let (mut extra_ns, mut extra_total) = (0u64, 0u64);
        let mut ch_peak = [0f32; 16];
        let mut ch_sq = [0f32; 16];
        for ch in 0..16 {
            if ch == 4 {
                // The keyboard parts (ch 1-4) are your playing; the rest the band's.
                clock.lap_less(crate::perf::ST_KEYS, extra_ns);
                extra_total += extra_ns;
                extra_ns = 0;
            }
            let live = self.ch_slot[ch] as usize;
            let part = &mut self.parts[ch];
            let (sl, sr) = (&mut self.stem_l[..n], &mut self.stem_r[..n]);
            let (tl, tr) = (&mut self.tmp_l[..n], &mut self.tmp_r[..n]);
            let mut first = true;
            let mut ns = 0u64;
            // A part with no note sounding whose lanes have gone quiet renders nothing.
            let holds = part.notes() > 0;
            for (i, lane) in part.lanes.iter_mut().enumerate() {
                let is_live = i == live;
                if lane.quiet >= IDLE_FRAMES && !(is_live && holds) {
                    continue;
                }
                let t0 = if profile { crate::rt::host_now() } else { 0 };
                // The first lane renders straight into the stem; any other beside it.
                let (l, r): (&mut [f32], &mut [f32]) = if first { (&mut *sl, &mut *sr) } else { (&mut *tl, &mut *tr) };
                lane.synth.render(l, r);
                if profile {
                    let dt = crate::rt::host_to_ns(crate::rt::host_now().wrapping_sub(t0));
                    ns += dt;
                    if lane.slot != 0 {
                        extra_ns += dt;
                    }
                }
                let peak = l.iter().chain(r.iter()).fold(0f32, |m, x| m.max(x.abs()));
                lane.quiet = if peak >= IDLE_LEVEL { 0 } else { lane.quiet.saturating_add(n as u32) };
                // A note lane's own sends: the stem below sends it at the part's gains; add
                // the difference (before the insert, as the part's own voices were).
                if lane.sends != PART_SENDS && peak > 0.0 {
                    for (b, &g) in self.send_gains[ch].iter().enumerate() {
                        let k = g * (lane.sends[b] - 1.0);
                        if g <= 0.0 || k == 0.0 {
                            continue;
                        }
                        add_scaled(k, l, &mut sends[2 * b * n..(2 * b + 1) * n]);
                        add_scaled(k, r, &mut sends[(2 * b + 1) * n..(2 * b + 2) * n]);
                    }
                }
                if !first {
                    add_scaled(1.0, tl, sl);
                    add_scaled(1.0, tr, sr);
                }
                first = false;
            }
            self.ch_ns[ch] = ns;
            // The part's insert runs on its stem, silent too (the effect may still ring).
            if insert_mask >> ch & 1 == 1
                && let Some(ins) = inserts.as_deref_mut()
            {
                if first {
                    sl.fill(0.0);
                    sr.fill(0.0);
                }
                ins.process(ch, sl, sr, part.level() * self.master);
            } else if first {
                continue;
            }
            let (peak, sq) = sl.iter().chain(sr.iter()).fold((0f32, 0f32), |(p, q), x| (p.max(x.abs()), q + x * x));
            if peak == 0.0 {
                continue;
            }
            (ch_peak[ch], ch_sq[ch]) = (peak, sq);
            for (b, &g) in self.send_gains[ch].iter().enumerate() {
                if g > 0.0 {
                    add_scaled(g, sl, &mut sends[2 * b * n..(2 * b + 1) * n]);
                    add_scaled(g, sr, &mut sends[(2 * b + 1) * n..(2 * b + 2) * n]);
                }
            }
            add_scaled(1.0, sl, left);
            add_scaled(1.0, sr, right);
        }
        clock.lap_less(crate::perf::ST_BAND, extra_ns);
        if profile {
            crate::perf::PERF.stage[crate::perf::ST_EXTRA].add(extra_total + extra_ns);
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
        let frames = (2 * n).max(1) as f32;
        for ch in 0..16 {
            self.rms[ch] = (ch_sq[ch] / frames).sqrt() * most;
            let p = ch_peak[ch];
            if p > 0.0 {
                peaks[ch].fetch_max((p * most).to_bits(), Relaxed);
                if profile {
                    crate::perf::Perf::peak(&crate::perf::PERF.channel_peak[ch], p * most);
                }
            }
        }
    }

    /// `render`, the send buses left out (tests).
    #[cfg(test)]
    pub(super) fn render_dry(&mut self, left: &mut [f32], right: &mut [f32], peaks: &[AtomicU32; 16], fade: Option<(f32, f32)>) {
        let mut sends = vec![0f32; 2 * BUSES * left.len()];
        self.render(left, right, &mut sends, peaks, fade, None);
    }
}

impl Part {
    /// The lanes playing `slot`'s font.
    #[inline]
    fn slot_lanes(&mut self, slot: u8) -> impl Iterator<Item = &mut Lane> {
        self.lanes.iter_mut().filter(move |l| l.slot == slot)
    }

    /// The note lane for a note with `sends` of its own: the one with those sends, else an
    /// idle one (it takes them), else the one whose sends are nearest.
    fn note_lane(&mut self, slots: usize, sends: [f32; BUSES]) -> usize {
        let lanes = &mut self.lanes[slots..];
        let i = if let Some(i) = lanes.iter().position(|l| l.sends == sends) {
            i
        } else if let Some(i) = lanes.iter().position(|l| l.quiet >= IDLE_FRAMES) {
            lanes[i].sends = sends;
            i
        } else {
            let dist = |l: &Lane| l.sends.iter().zip(&sends).map(|(a, b)| (a - b).abs()).sum::<f32>();
            let mut best = 0;
            for (i, l) in lanes.iter().enumerate() {
                if dist(l) < dist(&lanes[best]) {
                    best = i;
                }
            }
            best
        };
        slots + i
    }
}

/// `dst += k * src`, over the shorter of the two.
#[inline]
fn add_scaled(k: f32, src: &[f32], dst: &mut [f32]) {
    for (d, s) in dst.iter_mut().zip(src) {
        *d += k * s;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustysynth::NoteParams;

    /// The tiny test SoundFont (built in code): a looped square wave on every program.
    fn rack() -> Rack {
        let font = Arc::new(SoundFont::new(&mut &crate::patches::sf2::tiny_gm_sound_font()[..]).unwrap());
        Rack::new(&font, 48_000).unwrap()
    }

    fn peaks() -> [AtomicU32; 16] {
        std::array::from_fn(|_| AtomicU32::new(0))
    }

    fn level(p: &[AtomicU32; 16], ch: usize) -> f32 {
        f32::from_bits(p[ch].swap(0, Relaxed))
    }

    /// Each part has a synthesizer of its own: a part playing more notes than it has voices
    /// takes none from another part's held note (on one shared synthesizer, the flood's
    /// new voices stole the held note, its oldest in the sustain stage).
    #[test]
    fn a_part_keeps_its_voices_when_another_floods_its_own() {
        let mut rack = rack();
        let p = peaks();
        let (mut l, mut r) = (vec![0f32; 4800], vec![0f32; 4800]);
        rack.process(10, 0x90, 60, 100);
        rack.render_dry(&mut l, &mut r, &p, None);
        level(&p, 10);
        rack.render_dry(&mut l, &mut r, &p, None);
        let held = level(&p, 10);
        assert!(held > 1e-3, "the held note sounds: {held}");
        for key in 0..128 {
            rack.process(11, 0x90, key, 100);
        }
        rack.render_dry(&mut l, &mut r, &p, None);
        assert!(level(&p, 11) > 1e-3, "the flood sounds");
        let after = level(&p, 10);
        assert!((after - held).abs() <= held * 0.02, "the held note is untouched: {after} vs {held}");
        assert_eq!(rack.voices(), 129, "every note counted");
    }

    /// The rack counts each part's notes (upstream rustysynth has no voice count): held,
    /// kept by the hold pedal, until let go or cut by a channel mode message. It keeps the
    /// part's volume and expression on rustysynth's curve, for an insert's level.
    #[test]
    fn a_part_counts_its_notes_and_keeps_its_level() {
        let mut p = Part::new(Vec::new());
        p.track(0x90, 60, 100);
        p.track(0x90, 64, 100);
        assert_eq!(p.notes(), 2);
        p.track(0x80, 60, 0);
        assert_eq!(p.notes(), 1);
        p.track(0xB0, 64, 127);
        p.track(0x90, 64, 0);
        assert_eq!(p.notes(), 1, "let go under the pedal: still sounding");
        p.track(0x90, 67, 90);
        assert_eq!(p.notes(), 2);
        p.track(0xB0, 64, 0);
        assert_eq!(p.notes(), 1, "the pedal up ends the kept note");
        p.track(0xB0, 123, 0);
        assert_eq!(p.notes(), 0, "All Notes Off");
        p.track(0x90, 40, 100);
        p.track(0xB0, 120, 0);
        assert_eq!(p.notes(), 0, "All Sound Off");

        let curve = |vol: f32, expr: f32| ((vol / 16383.0) * (expr / 16383.0)).powi(2);
        assert_eq!(p.level(), curve((100 << 7) as f32, (127 << 7) as f32), "GM defaults: CC7 100, CC11 127");
        p.track(0xB0, 7, 64);
        p.track(0xB0, 11, 32);
        p.track(0xB0, 43, 5);
        assert_eq!(p.level(), curve((64 << 7) as f32, ((32 << 7) | 5) as f32));
        p.track(0xB0, 121, 0);
        assert_eq!(p.level(), curve((64 << 7) as f32, (127 << 7) as f32), "Reset All Controllers: expression, not volume");
    }

    /// A drum note with sends of its own (#239) plays in a note lane with those sends: the
    /// same sends share a lane, other sends take an idle one, and with none idle the
    /// nearest. A note with the part's sends plays on the part's own synthesizer.
    #[test]
    fn notes_with_their_own_sends_play_in_a_lane_of_their_own() {
        let mut rack = rack();
        let with = |r: f32, c: f32| NoteParams { sends: [r, c, 1.0], ..NoteParams::NEUTRAL };
        let lane = |rack: &Rack, sends: [f32; BUSES]| rack.parts[9].lanes.iter().position(|l| l.sends == sends && l.quiet == 0);
        rack.note_on_with(9, 36, 100, &with(0.0, 1.0));
        assert_eq!(lane(&rack, [0.0, 1.0, 1.0]), Some(1), "the first note lane");
        rack.note_on_with(9, 38, 100, &with(0.0, 1.0));
        assert_eq!(rack.parts[9].lanes.iter().filter(|l| l.quiet == 0).count(), 1, "the same sends, the same lane");
        rack.note_on_with(9, 42, 100, &with(0.5, 1.0));
        rack.note_on_with(9, 44, 100, &with(0.5, 0.0));
        rack.note_on_with(9, 46, 100, &with(1.0, 0.25));
        assert_eq!(lane(&rack, [1.0, 0.25, 1.0]), Some(4), "each new sends in an idle lane");
        rack.note_on_with(9, 49, 100, &with(0.1, 1.0));
        assert_eq!(rack.parts[9].lanes[1].sends, [0.0, 1.0, 1.0], "none idle: the nearest lane, as it is");
        assert_eq!(rack.voices(), 6);
        rack.note_on_with(9, 51, 100, &NoteParams { gain: 0.5, ..NoteParams::NEUTRAL });
        assert_eq!(rack.parts[9].lanes[0].quiet, 0, "the part's own sends: its own synthesizer");
        // Every lane gets the part's note-offs.
        for key in [36, 38, 42, 44, 46, 49, 51] {
            rack.process(9, 0x80, key, 0);
        }
        assert_eq!(rack.voices(), 0);
    }
}
