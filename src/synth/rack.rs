//! The built-in synth's SoundFont parts (#346 step 1): one `rustysynth::Synthesizer` per
//! part, all sharing the SoundFont (`Arc<SoundFont>`), with rustysynth's own reverb and
//! chorus off (`SynthesizerSettings::enable_reverb_and_chorus`).
//!
//! Each MIDI channel is a part and renders its own stereo stem. Everything that works on a
//! part's signal runs here, on its stem, in yahaha code: the live tone filter (CC74/71,
//! #346 step 3; part_tone.rs), the channel-strip EQ (#247, `fx::part_eq`), the Style parts' insertion effects (#269), the meters (peak and RMS), the sends into the effect bus (#204) and the
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
//! - a drum setup's kit's lanes (kit.rs, #239): once the kit its setup needs is built, the
//!   part's drum notes play in them, the first with the part's sends, the others claimed
//!   by sends as the note lanes are. A part keeps two kits' lanes, so a kit that the setup
//!   no longer wants rings out while the next plays. A kit's lanes start from what the
//!   part's synthesizers have had of its controllers, bend and RPNs ([`Heard`]).
//!
//! Every lane of a part gets the part's controllers, pitch bend and note-offs; a note-on,
//! program change or bank select goes to the lanes of the font the channel plays now.
//! Mono mode and portamento are played on the way in (voicing.rs): a part's messages go to
//! all of its synthesizer's MIDI channels, and a gliding note plays on a channel of its own
//! whose pitch bend steps once a synthesizer block, so a lane with a glide renders a block
//! at a time. The envelope controllers (CC73/75/72, #346 step 4) are played on the way out
//! of the voicing (envelope.rs, `route`): a shaped note plays on a channel of its own
//! whose expression steps once a block, so its lane renders a block at a time too.
//!
//! All memory is allocated when the rack is built, off the audio thread; `render` never
//! allocates, locks or panics.

use super::envelope::{Env, PartEnvelope};
use super::kit::{Kit, KitKey, KitSource};
use super::part_tone::PartTone;
use super::voicing::{To, Voicing};
use super::*;
use crate::fx::BUSES;
use crate::fx::part_eq::{EqCoeffs, EqDsp};

/// Voices per part on the main SoundFont: as many as the band's and your playing's shared
/// synthesizers had each, so a part never has fewer than before.
const MAIN_POLYPHONY: usize = 128;
/// Voices per part on an extra SoundFont (the sound library, #103).
const EXTRA_POLYPHONY: usize = 64;
/// Lanes per part for drum notes with sends of their own, and their voices each.
const NOTE_LANES: usize = 4;
const NOTE_POLYPHONY: usize = 32;
/// A kit's lanes: one with the part's sends (with the main font's voices), then one for
/// each note lane.
pub(super) const KIT_LANES: usize = 1 + NOTE_LANES;
/// The kits a part keeps lanes for: the one playing, and the one before it ringing out.
const KITS: usize = 2;
/// A kit lane's `slot`: no font slot (a program change or bank select never reaches it).
const KIT_SLOT: u8 = 0xFE;
/// A lane's sends when they are the part's own.
pub(super) const PART_SENDS: [f32; BUSES] = [1.0; BUSES];

/// A lane with no note held is rendered until its output has been below `IDLE_LEVEL` for
/// this long (a tail ringing out), then skipped.
pub(super) const IDLE_FRAMES: u32 = 4800;
const IDLE_LEVEL: f32 = 1e-6;

/// One synthesizer of a part.
pub(super) struct Lane {
    synth: Synthesizer,
    /// The rack slot (SoundFont) it plays: 0 = the main font; `KIT_SLOT`: a kit.
    slot: u8,
    /// Its notes' sends, as a factor on the part's (1: the part's own).
    sends: [f32; BUSES],
    /// Frames it has rendered below `IDLE_LEVEL` (a note-on starts it again).
    pub(super) quiet: u32,
    /// The part's bank there (CC0; 128 and up: a drum kit), and its program.
    bank: i32,
    program: i32,
    /// Frames into the synthesizer's block (0: the next frame renders a new block).
    phase: usize,
    /// What it has had of the part's controllers.
    heard: Heard,
}

/// Not heard (`Heard::cc`).
const UNHEARD: u8 = 0xFF;

/// What a lane's synthesizer has had of its part's controllers, pitch bend and RPNs (the
/// part-wide messages, not a gliding or shaped note's own), so a kit's new lanes can start
/// from the same: rustysynth has no way to read a channel's state back.
#[derive(Clone, Copy)]
struct Heard {
    /// The last value of each controller (0-119, bank select aside; `UNHEARD`: none).
    cc: [u8; 120],
    /// RPN 0-2's data entry (MSB, LSB): bend range, fine and coarse tuning.
    rpn: [[u8; 2]; 3],
    /// An NRPN was selected after the last RPN.
    nrpn: bool,
    /// The pitch bend (LSB, MSB).
    bend: Option<[u8; 2]>,
}

impl Heard {
    const NONE: Heard = Heard { cc: [UNHEARD; 120], rpn: [[UNHEARD; 2]; 3], nrpn: false, bend: None };

    /// Follow a message the lane gets on all of its part's channels.
    #[inline]
    fn follow(&mut self, st: i32, d1: i32, d2: i32) {
        let v = (d2 & 127) as u8;
        match st {
            0xB0 => match d1 {
                0 | 32 => {}
                // Reset All Controllers, as rustysynth's channel takes it.
                121 => {
                    for cc in [1, 33, 11, 43, 64, 100, 101] {
                        self.cc[cc] = UNHEARD;
                    }
                    self.bend = None;
                }
                6 | 38 => {
                    self.cc[d1 as usize] = v;
                    let (msb, lsb) = (self.cc[101], self.cc[100]);
                    if !self.nrpn && msb == 0 && lsb < 3 {
                        self.rpn[lsb as usize][(d1 == 38) as usize] = v;
                    }
                }
                1..=119 => {
                    self.cc[d1 as usize] = v;
                    match d1 {
                        98 | 99 => self.nrpn = true,
                        100 | 101 => self.nrpn = false,
                        _ => {}
                    }
                }
                _ => {}
            },
            0xE0 => self.bend = Some([(d1 & 127) as u8, v]),
            _ => {}
        }
    }

    /// Bring `synth` to what was heard, on `chans`.
    fn replay(&self, synth: &mut Synthesizer, chans: u16) {
        let mut m = chans;
        while m != 0 {
            let c = m.trailing_zeros() as i32;
            m &= m - 1;
            let mut cc = |n: i32, v: u8| synth.process_midi_message(c, 0xB0, n, v as i32);
            for (n, &v) in self.cc.iter().enumerate().skip(1) {
                if v != UNHEARD && !matches!(n, 6 | 38 | 98..=101) {
                    cc(n as i32, v);
                }
            }
            for (n, d) in self.rpn.iter().enumerate() {
                if *d != [UNHEARD; 2] {
                    cc(101, 0);
                    cc(100, n as u8);
                    for (k, &v) in d.iter().enumerate() {
                        if v != UNHEARD {
                            cc(if k == 0 { 6 } else { 38 }, v);
                        }
                    }
                }
            }
            let select = if self.nrpn { [99, 98] } else { [101, 100] };
            for n in select {
                if self.cc[n] != UNHEARD {
                    cc(n as i32, self.cc[n]);
                }
            }
            if let Some([lo, hi]) = self.bend {
                synth.process_midi_message(c, 0xE0, lo as i32, hi as i32);
            }
        }
    }
}

/// A part's drum-setup kits (kit.rs): what each of its two kit lane sets holds, which one
/// its setup plays now, and what that was worked out from.
pub(super) struct PartKits {
    /// Each kit lane set's kit (None: no lanes there yet).
    keys: [Option<KitKey>; KITS],
    /// The set played last (the other is replaced next).
    newest: usize,
    /// The set the part's setup plays now (None: no kit; its notes play on its font).
    pub(super) live: Option<usize>,
    /// The drum setups' changes, the slot, bank and program `live` was worked out for.
    pub(super) seen: Option<(u32, u8, i32, i32)>,
}

/// A part (a MIDI channel): its lanes, and what the rack keeps of its state.
pub(super) struct Part {
    /// Slot k's lane at k (0 = the main font); then the main font's note lanes; then its
    /// kits' lanes, `KIT_LANES` each, once they have come (`Rack::install_kit`).
    pub(super) lanes: Vec<Lane>,
    /// Its mono mode and portamento, and the channels its notes play on.
    voicing: Voicing,
    /// CC7/39 and CC11/43, 14 bits each, as rustysynth keeps them (for an insert's level).
    volume: i32,
    expression: i32,
    /// Keys held down, and keys let go under the hold pedal (bit = key).
    held: u128,
    sustained: u128,
    pedal: bool,
    /// Its live tone controls (#346 step 3): the stem filter (CC74/71) and the mod wheel
    /// with CC77/78 (part_tone.rs).
    tone: PartTone,
    /// Its envelope controllers (CC73/75/72, #346 step 4) and the notes they shape
    /// (envelope.rs).
    env: PartEnvelope,
    /// Its drum setup's kits (kit.rs, #239).
    pub(super) kits: PartKits,
    /// Its channel-strip EQ (#247, `fx::part_eq`), after the tone filter.
    eq: EqDsp,
}

impl Part {
    fn new(lanes: Vec<Lane>, voicing: Voicing, sample_rate: f32, block: usize) -> Part {
        Part {
            lanes,
            voicing,
            volume: 100 << 7,
            expression: 127 << 7,
            held: 0,
            sustained: 0,
            pedal: false,
            tone: PartTone::new(sample_rate),
            env: PartEnvelope::new(sample_rate, block),
            kits: PartKits { keys: [None, None], newest: 0, live: None, seen: None },
            eq: EqDsp::new(),
        }
    }

    /// The mod wheel to every lane, on every channel the part's notes play on (a glide's
    /// too), if the tone's has changed.
    #[inline]
    fn send_modulation(&mut self) {
        if let Some(v) = self.tone.modulation() {
            send(&mut self.lanes, 0, To::All, self.voicing.chans(), 0xB0, 1, v);
        }
    }

    /// Whether it plays a melodic voice (not a drum kit) on `slot`'s font.
    #[inline]
    fn melodic(&self, slot: u8) -> bool {
        self.voicing.home() != 9 && self.lanes.get(slot as usize).is_some_and(|l| l.bank < 128)
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
    /// The synthesizers' block, in frames.
    block: usize,
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
    /// Each slot's SoundFont as kits are derived from it (None: its preset data wasn't
    /// kept; its parts play no kits), and what the lanes are built for.
    sources: Vec<Option<Arc<KitSource>>>,
    sample_rate: i32,
    legacy: bool,
}

/// The settings every lane is built with. `legacy`: rustysynth's own reverb and chorus on
/// (the sound before the effect bus, #204); otherwise off.
fn settings(sample_rate: i32, polyphony: usize, legacy: bool) -> SynthesizerSettings {
    let mut s = SynthesizerSettings::new(sample_rate);
    s.maximum_polyphony = polyphony;
    s.enable_reverb_and_chorus = legacy;
    s
}

/// A lane on `font`, for `slot`.
fn new_lane(font: &Arc<SoundFont>, slot: u8, polyphony: usize, sample_rate: i32, legacy: bool) -> Result<Lane> {
    let synth = Synthesizer::new(font, &settings(sample_rate, polyphony, legacy)).map_err(|e| anyhow!("{e:?}"))?;
    Ok(Lane { synth, slot, sends: PART_SENDS, quiet: IDLE_FRAMES, bank: 0, program: 0, phase: 0, heard: Heard::NONE })
}

/// A kit's lanes on its SoundFont (kit.rs). Slow: off the audio thread.
pub(super) fn kit_lanes(font: &Arc<SoundFont>, sample_rate: i32, legacy: bool) -> Result<Vec<Lane>> {
    (0..KIT_LANES).map(|i| new_lane(font, KIT_SLOT, if i == 0 { MAIN_POLYPHONY } else { NOTE_POLYPHONY }, sample_rate, legacy)).collect()
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
        let lane = |font: &Arc<SoundFont>, slot: u8, polyphony: usize| new_lane(font, slot, polyphony, sample_rate, legacy);
        let block = settings(sample_rate, MAIN_POLYPHONY, legacy).block_size;
        let mut parts = Vec::with_capacity(16);
        for ch in 0..16u8 {
            // Room for the kits' lanes too: they are moved in on the audio thread.
            let mut lanes = Vec::with_capacity(1 + extras.len() + NOTE_LANES + KITS * KIT_LANES);
            lanes.push(lane(main, 0, MAIN_POLYPHONY)?);
            for (i, f) in extras.iter().enumerate() {
                lanes.push(lane(f, i as u8 + 1, EXTRA_POLYPHONY)?);
            }
            for _ in 0..NOTE_LANES {
                lanes.push(lane(main, 0, NOTE_POLYPHONY)?);
            }
            parts.push(Part::new(lanes, Voicing::new(ch, block, sample_rate), sample_rate as f32, block));
        }
        let mut r = Rack {
            parts,
            slots: 1 + extras.len(),
            slot_of: [routing::NO_SLOT; crate::patches::route::MAX_FONTS],
            ch_slot: [0; 16],
            mapped: 0,
            send_gains: [[0.0; BUSES]; 16],
            block,
            master: 0.5,
            stem_l: vec![0.0; 8192],
            stem_r: vec![0.0; 8192],
            tmp_l: vec![0.0; 8192],
            tmp_r: vec![0.0; 8192],
            profile: false,
            ch_ns: [0; 16],
            rms: [0.0; 16],
            sources: std::iter::once(main).chain(extras.iter().copied()).map(KitSource::of).collect(),
            sample_rate,
            legacy,
        };
        // Rhythm 1 (ch 9) is a drum part too: on the drum bank, on every font.
        let p = &mut r.parts[8];
        send(&mut p.lanes, 0, To::All, p.voicing.chans(), 0xB0, 0, 128);
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

    /// The SoundFont at `sf2`, velocity -> filter cutoff baked in (font.rs).
    fn read(sf2: &Path) -> Result<Arc<SoundFont>> {
        super::font::open(sf2)
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

    /// A channel's mono or poly mode (#246), without an All Notes Off (voicing.rs).
    pub(super) fn set_mono(&mut self, ch: u8, mono: bool) {
        let slot = self.ch_slot[ch as usize & 15];
        let part = &mut self.parts[ch as usize & 15];
        let melodic = part.melodic(slot);
        let (lanes, env) = (&mut part.lanes, &mut part.env);
        part.voicing.set_mono(mono, melodic, &mut |to, chans, st, d1, d2| route(env, lanes, slot, to, chans, st, d1, d2));
    }

    /// A channel's EQ coefficients (#247), computed off the audio thread.
    pub(super) fn set_eq(&mut self, ch: u8, c: &EqCoeffs) {
        self.parts[ch as usize & 15].eq.set(c);
    }

    pub(super) fn set_master_volume(&mut self, v: f32) {
        self.master = v;
        for p in &mut self.parts {
            for l in &mut p.lanes {
                l.synth.set_master_volume(v);
            }
        }
    }

    /// A channel message to the lane(s) that take it (see the module docs), through the
    /// part's mono mode and portamento (voicing.rs).
    pub(super) fn process(&mut self, ch: i32, st: i32, d1: i32, d2: i32) {
        let c = ch as usize & 15;
        let slot = self.ch_slot[c];
        let part = &mut self.parts[c];
        part.track(st, d1, d2);
        // The part's tone and envelope first (part_tone.rs, envelope.rs): their own
        // controllers go no further.
        let own = part.tone.follow(st, d1, d2) || part.env.follow(st, d1, d2);
        // Reset All Controllers zeroes the synthesizers' mod wheel: the tone's goes after.
        let reset = st == 0xB0 && d1 == 121;
        if !reset {
            part.send_modulation();
        }
        if !own {
            let melodic = part.melodic(slot);
            part.voicing.set_spread(part.env.spread());
            let (lanes, env) = (&mut part.lanes, &mut part.env);
            part.voicing.process(st, d1, d2, melodic, slot, &mut |to, chans, st, d1, d2| route(env, lanes, slot, to, chans, st, d1, d2));
        }
        if reset {
            part.send_modulation();
        }
    }

    /// A drum note-on on `ch` (a part playing a drum setup, #239) with `sends` of its own
    /// (as a factor on the part's): in the lanes of the part's kit `kit` if its setup has
    /// one (`PartKits::live`), else in a note lane with those sends (on the main font), else
    /// as any note. It goes through the part's voicing and envelope as any note does.
    pub(super) fn drum_note(&mut self, ch: u8, key: u8, velocity: u8, sends: [f32; BUSES], kit: Option<usize>) {
        let c = ch as usize & 15;
        let slot = self.ch_slot[c] as usize;
        let base = self.slots + NOTE_LANES;
        let part = &mut self.parts[c];
        let lane = match kit {
            Some(k) if part.lanes.len() >= base + (k + 1) * KIT_LANES => part.kit_lane(base + k * KIT_LANES, sends),
            _ if slot == 0 && sends != PART_SENDS => part.note_lane(self.slots, sends),
            _ => slot,
        };
        self.note_on_in(c, key as i32, velocity as i32, lane);
    }

    /// A note-on on part `c`, played in lane `lane` rather than the live font's: through
    /// the part's tone, voicing and envelope as `process` plays a note-on.
    fn note_on_in(&mut self, c: usize, key: i32, velocity: i32, lane: usize) {
        let slot = self.ch_slot[c];
        let part = &mut self.parts[c];
        part.track(0x90, key, velocity);
        part.tone.follow(0x90, key, velocity);
        part.env.follow(0x90, key, velocity);
        part.send_modulation();
        let melodic = part.melodic(slot);
        part.voicing.set_spread(part.env.spread());
        let (lanes, env) = (&mut part.lanes, &mut part.env);
        let live = lane as u8;
        part.voicing.process(0x90, key, velocity, melodic, live, &mut |to, chans, st, d1, d2| route(env, lanes, live, to, chans, st, d1, d2));
    }

    /// The font slot the part on `ch` plays, and its bank and program there as rustysynth
    /// finds its preset (MIDI channel 10 adds 128 to the bank).
    pub(super) fn voice_of(&self, ch: u8) -> (u8, i32, i32) {
        let c = ch as usize & 15;
        let slot = self.ch_slot[c];
        let part = &self.parts[c];
        let l = &part.lanes[slot as usize];
        (slot, if part.voicing.home() == 9 { l.bank + 128 } else { l.bank }, l.program)
    }

    /// The bank (CC0) the part on `ch` has on the main font.
    pub(super) fn main_bank(&self, ch: u8) -> i32 {
        self.parts[ch as usize & 15].lanes[0].bank
    }

    /// Slot `slot`'s font as kits are derived from it, if they can be.
    pub(super) fn kit_source(&self, slot: u8) -> Option<&Arc<KitSource>> {
        self.sources.get(slot as usize)?.as_ref()
    }

    /// The key a kit for this rack's parts has: `want` with the lanes' sample rate and
    /// effects.
    pub(super) fn kit_key(&self, want: &mut KitKey, source: &KitSource, preset: usize) {
        want.font = source.id();
        want.preset = preset as u32;
        want.sample_rate = self.sample_rate;
        want.legacy = self.legacy;
    }

    /// Part `ch`'s kits.
    pub(super) fn kits(&mut self, ch: u8) -> &mut PartKits {
        &mut self.parts[ch as usize & 15].kits
    }

    /// Whether any part holds the kit `key` is.
    pub(super) fn holds_kit(&self, key: &KitKey) -> bool {
        self.parts.iter().any(|p| p.kits.keys.iter().flatten().any(|k| k == key))
    }

    /// Every part looks for its kit again (a kit has come).
    pub(super) fn forget_kits_seen(&mut self) {
        for p in &mut self.parts {
            p.kits.seen = None;
        }
    }

    /// Play part `ch`'s kit `want` if one of its kit lane sets holds it; which set.
    pub(super) fn play_kit(&mut self, ch: u8, want: &KitKey) -> Option<usize> {
        let k = &mut self.parts[ch as usize & 15].kits;
        let i = k.keys.iter().position(|x| x.as_ref() == Some(want))?;
        k.newest = i;
        k.live = Some(i);
        Some(i)
    }

    /// Put `kit`'s lanes in part `ch`'s kit lane set played least lately and play it; the
    /// lanes they replace (a kit before the last, cut if it still rings) come back in the
    /// kit's box, to be freed off the audio thread. Its lanes start from what the part's
    /// synthesizers have had of its controllers. No allocation: the part has room for them.
    pub(super) fn install_kit(&mut self, ch: u8, mut kit: Box<Kit>) -> Box<Kit> {
        let base = self.slots + NOTE_LANES;
        let master = self.master;
        let part = &mut self.parts[ch as usize & 15];
        let chans = part.voicing.chans();
        let heard = part.lanes[0].heard;
        for l in &mut kit.lanes {
            heard.replay(&mut l.synth, chans);
            l.heard = heard;
            l.synth.set_master_volume(master);
        }
        let i = match part.kits.keys.iter().position(|k| k.is_none()) {
            Some(i) => i,
            None => 1 - part.kits.newest,
        };
        let at = base + i * KIT_LANES;
        if part.lanes.len() == at && kit.lanes.len() == KIT_LANES {
            part.lanes.extend(kit.lanes.drain(..));
        } else {
            for (j, l) in kit.lanes.iter_mut().enumerate().take(KIT_LANES) {
                if let Some(old) = part.lanes.get_mut(at + j) {
                    std::mem::swap(old, l);
                }
            }
        }
        part.kits.keys[i] = Some(kit.key.clone());
        part.kits.newest = i;
        part.kits.live = Some(i);
        kit
    }

    /// Channel `ch`'s bank and program on the lanes of `slot` (a route, #103).
    pub(super) fn program_on(&mut self, ch: u8, slot: u8, bank: i32, program: i32) {
        let part = &mut self.parts[ch as usize & 15];
        let chans = part.voicing.chans();
        send(&mut part.lanes, slot, To::Slot, chans, 0xB0, 0, bank);
        send(&mut part.lanes, slot, To::Slot, chans, 0xC0, program, 0);
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
        let block = self.block;
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
            // The mod wheel as the tone has it now (a vibrato delay fading in).
            part.send_modulation();
            // The envelope shaping's clock, for how long released notes may still sound.
            part.env.tick(n);
            for (i, lane) in part.lanes.iter_mut().enumerate() {
                let is_live = i == live;
                // A shaped note's lane renders until its shaping is done (envelope.rs).
                if lane.quiet >= IDLE_FRAMES && !(is_live && holds) && !part.env.on_lane(i) {
                    continue;
                }
                let t0 = if profile { crate::rt::host_now() } else { 0 };
                // The first lane renders straight into the stem; any other beside it.
                let (l, r): (&mut [f32], &mut [f32]) = if first { (&mut *sl, &mut *sr) } else { (&mut *tl, &mut *tr) };
                render_lane(lane, i, &mut part.voicing, &mut part.env, block, l, r);
                if profile {
                    let dt = crate::rt::host_to_ns(crate::rt::host_now().wrapping_sub(t0));
                    ns += dt;
                    if lane.slot != 0 && lane.slot != KIT_SLOT {
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
            // The part's filter (CC74/71, part_tone.rs) on its stem, before its insert,
            // meters and sends, as the voices' own filter was.
            part.tone.advance(n);
            if first {
                part.tone.filter.clear();
            } else if part.tone.filter.active() {
                let t0 = if profile { crate::rt::host_now() } else { 0 };
                part.tone.filter.process(sl, sr);
                if profile {
                    ns += crate::rt::host_to_ns(crate::rt::host_now().wrapping_sub(t0));
                }
            }
            // The part's channel-strip EQ (#247) after it, still before the insert, meters
            // and sends. Flat, it is not run at all.
            if first {
                part.eq.clear();
            } else if part.eq.active() {
                part.eq.process(sl, sr);
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
    /// The note lane for a note with `sends` of its own: the one with those sends, else an
    /// idle one (it takes them), else the one whose sends are nearest.
    fn note_lane(&mut self, slots: usize, sends: [f32; BUSES]) -> usize {
        slots + claim(&mut self.lanes[slots..slots + NOTE_LANES], sends)
    }

    /// The lane of the kit whose lanes start at `at` for a note with `sends`: its first
    /// with the part's own, else one of the others as `note_lane` finds one.
    fn kit_lane(&mut self, at: usize, sends: [f32; BUSES]) -> usize {
        if sends == PART_SENDS { at } else { at + 1 + claim(&mut self.lanes[at + 1..at + KIT_LANES], sends) }
    }
}

/// Of `lanes`, the one for a note with `sends`: the one with those sends, else an idle one
/// (it takes them), else the one whose sends are nearest.
fn claim(lanes: &mut [Lane], sends: [f32; BUSES]) -> usize {
    if let Some(i) = lanes.iter().position(|l| l.sends == sends) {
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
    }
}

/// A message to the lanes of a part that `to` names (`slot`: the font the part plays), on
/// each of the synthesizer channels in `chans`.
fn send(lanes: &mut [Lane], slot: u8, to: To, chans: u16, st: i32, d1: i32, d2: i32) {
    // A message to all of the part's channels (MIDI channel 10's part has only that one):
    // the part's own, not a gliding or shaped note's.
    let part_wide = chans == 1 << 9 || chans.count_ones() > 1;
    for (i, l) in lanes.iter_mut().enumerate() {
        let take = match to {
            To::Live => i == slot as usize,
            To::Slot => l.slot == slot,
            To::All => true,
            To::Lane(k) => i == k as usize,
            To::AllBut(k) => i != k as usize,
        };
        if !take {
            continue;
        }
        if to == To::Live {
            l.quiet = 0;
        }
        if st == 0xB0 && d1 == 0 {
            l.bank = d2;
        }
        if st == 0xC0 {
            l.program = d1;
        }
        if part_wide {
            l.heard.follow(st, d1, d2);
        }
        let mut m = chans;
        while m != 0 {
            let c = m.trailing_zeros() as i32;
            m &= m - 1;
            l.synth.process_midi_message(c, st, d1, d2);
        }
    }
}

/// A message from a part's voicing on to its lanes (`send`), through the part's envelope
/// shaping (envelope.rs): a note-on looks up the SoundFont's envelope for a shaped note,
/// and a shaped note's note-off, the player's expression, the hold pedal and the channel
/// mode messages are played there.
#[allow(clippy::too_many_arguments)]
fn route(env: &mut PartEnvelope, lanes: &mut [Lane], slot: u8, to: To, chans: u16, st: i32, d1: i32, d2: i32) {
    match st {
        0x90 if d2 & 127 > 0 => {
            let c = chans.trailing_zeros() as u8 & 15;
            // Every note is counted (how long it may sound), and shaped where it can be.
            // As rustysynth finds the preset: MIDI channel 10 adds 128 to the bank.
            let own = lanes.get(slot as usize).and_then(|l| {
                let bank = if c == 9 { l.bank + 128 } else { l.bank };
                Env::of(l.synth.get_sound_font(), bank, l.program, d1 & 127, d2 & 127)
            });
            env.note_on(chans, slot, d1 as u8, own, &mut |to, m, st, a, b| send(lanes, slot, to, m, st, a, b));
            send(lanes, slot, to, chans, st, d1, d2);
        }
        0x80 | 0x90 => {
            let rest = env.note_off(chans, d1, &mut |to, m, st, a, b| send(lanes, slot, to, m, st, a, b));
            if rest != 0 {
                send(lanes, slot, to, rest, st, d1, d2);
            }
        }
        0xB0 if d1 == 11 || d1 == 43 => env.expression(chans, d1, d2, &mut |to, m, st, a, b| send(lanes, slot, to, m, st, a, b)),
        0xB0 if matches!(d1, 64 | 120 | 121 | 123) => {
            send(lanes, slot, to, chans, st, d1, d2);
            let emit = &mut |to, m, st, a, b| send(lanes, slot, to, m, st, a, b);
            match d1 {
                64 => env.pedal(d2 >= 64, emit),
                120 => env.sound_off(chans, emit),
                121 => env.reset(emit),
                _ => env.notes_off(chans),
            }
        }
        _ => send(lanes, slot, to, chans, st, d1, d2),
    }
}

/// Render lane `i` of a part into `l`/`r`: at once, or with a glide or a shaped note in it
/// a block at a time, each glide and shaped note stepping before the block (voicing.rs,
/// envelope.rs).
#[inline]
fn render_lane(lane: &mut Lane, i: usize, voicing: &mut Voicing, env: &mut PartEnvelope, block: usize, l: &mut [f32], r: &mut [f32]) {
    let n = l.len();
    if !voicing.glides_in(i) && !env.on_lane(i) {
        lane.synth.render(l, r);
        lane.phase = (lane.phase + n) % block;
        return;
    }
    let mut k = 0;
    while k < n {
        if lane.phase == 0 {
            let synth = &mut lane.synth;
            voicing.block(i, &mut |c, st, d1, d2| synth.process_midi_message(c, st, d1, d2));
            env.block(i, &mut |c, st, d1, d2| synth.process_midi_message(c, st, d1, d2));
        }
        let m = (block - lane.phase).min(n - k);
        lane.synth.render(&mut l[k..k + m], &mut r[k..k + m]);
        lane.phase = (lane.phase + m) % block;
        k += m;
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

    /// The tiny test SoundFont (built in code): a looped square wave on every program.
    fn rack() -> Rack {
        let font = Arc::new(crate::synth::font::read(&mut std::io::Cursor::new(crate::patches::sf2::tiny_gm_sound_font())).unwrap());
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
        let mut p = Part::new(Vec::new(), Voicing::new(0, 64, 48_000), 48_000.0, 64);
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

    /// #346 step 3: CC74 swept down on a held note darkens the part's stem (the rack's
    /// filter, not rustysynth's), gliding rather than stepping; back at 64 the stem is
    /// exactly what it would have been with no sweep at all.
    #[test]
    fn a_cutoff_sweep_on_a_held_note_darkens_the_stem() {
        let p = peaks();
        let block = |r: &mut Rack| {
            let (mut l, mut rr) = (vec![0f32; 480], vec![0f32; 480]);
            r.render_dry(&mut l, &mut rr, &p, None);
            l
        };
        let bright = |x: &[f32]| {
            let hf: f32 = x.windows(2).map(|w| (w[1] - w[0]).powi(2)).sum();
            hf / x.iter().map(|v| v * v).sum::<f32>().max(1e-30)
        };
        let jump = |x: &[f32]| x.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0f32, f32::max);
        let (mut swept, mut still) = (rack(), rack());
        let (mut a, mut b) = (Vec::new(), Vec::new());
        for r in [&mut swept, &mut still] {
            r.process(3, 0x90, 60, 100);
        }
        for i in 0..40 {
            if i < 16 {
                swept.process(3, 0xB0, 74, 64 - 4 * i);
            }
            a.extend(block(&mut swept));
            b.extend(block(&mut still));
        }
        let (dark, open) = (bright(&a[a.len() - 4800..]), bright(&b[b.len() - 4800..]));
        assert!(dark < open * 0.3, "the held note darkens: {dark} vs {open}");
        // The part's synthesizer never got CC74: what it renders is untouched.
        let raw = |r: &mut Rack| {
            let (mut l, mut rr) = (vec![0f32; 480], vec![0f32; 480]);
            r.parts[3].lanes[0].synth.render(&mut l, &mut rr);
            l
        };
        assert_eq!(raw(&mut swept), raw(&mut still), "rustysynth plays no CC74");
        // The square wave's own edges are its largest steps; the sweep adds none.
        assert!(jump(&a) <= jump(&b) * 1.01, "no click: {} vs {}", jump(&a), jump(&b));
        swept.process(3, 0xB0, 74, 64);
        for _ in 0..20 {
            block(&mut swept);
            block(&mut still);
        }
        assert_eq!(block(&mut swept), block(&mut still), "at 64 again: bit-identical");
        assert_eq!(swept.voices(), 1);
    }

    /// CC74 127 on a held note brightens the stem (the high shelf), without a click.
    #[test]
    fn cutoff_127_on_a_held_note_brightens_the_stem() {
        let p = peaks();
        let block = |r: &mut Rack| {
            let (mut l, mut rr) = (vec![0f32; 480], vec![0f32; 480]);
            r.render_dry(&mut l, &mut rr, &p, None);
            l
        };
        let bright = |x: &[f32]| {
            let hf: f32 = x.windows(2).map(|w| (w[1] - w[0]).powi(2)).sum();
            hf / x.iter().map(|v| v * v).sum::<f32>().max(1e-30)
        };
        let jump = |x: &[f32]| x.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0f32, f32::max);
        let (mut up, mut still) = (rack(), rack());
        let (mut a, mut b) = (Vec::new(), Vec::new());
        for r in [&mut up, &mut still] {
            r.process(3, 0x90, 72, 100);
        }
        for i in 0..40 {
            if i == 10 {
                up.process(3, 0xB0, 74, 127);
            }
            a.extend(block(&mut up));
            b.extend(block(&mut still));
        }
        let (lit, open) = (bright(&a[a.len() - 4800..]), bright(&b[b.len() - 4800..]));
        assert!(lit > open * 1.3, "the held note brightens: {lit} vs {open}");
        let edge = &a[4800 - 64..4800 + 480];
        assert!(jump(edge) <= jump(&b[..4800]) * 2.0, "no click: {} vs {}", jump(edge), jump(&b[..4800]));
    }

    /// #247: a flat channel-strip EQ leaves the part's stem bit-identical; a treble cut
    /// darkens it, on that part only.
    #[test]
    fn the_part_eq_plays_on_the_stem_and_flat_is_bit_identical() {
        use crate::fx::part_eq::PartEq;
        let run = |eq: Option<PartEq>| {
            let mut r = rack();
            if let Some(eq) = eq {
                r.set_eq(0, &EqCoeffs::new(eq, 48_000.0));
            }
            r.process(0, 0x90, 72, 100);
            r.process(1, 0x90, 60, 100);
            play(&mut r, 9600, 480)
        };
        let plain = run(None);
        assert_eq!(run(Some(PartEq::FLAT)), plain, "flat");
        assert_eq!(run(Some(PartEq { low_freq: 1_000, high_freq: 2_000, ..PartEq::FLAT })), plain, "flat at other frequencies");
        let bright = |x: &[f32]| x.windows(2).map(|w| (w[1] - w[0]).powi(2)).sum::<f32>() / x.iter().map(|v| v * v).sum::<f32>().max(1e-30);
        let cut = run(Some(PartEq { high_gain: -12, high_freq: 1_000, ..PartEq::FLAT }));
        assert!(bright(&cut) < bright(&plain) * 0.7, "darker: {} vs {}", bright(&cut), bright(&plain));
        // Only on its part: channel 1 alone is as it was.
        let solo = |eq: bool| {
            let mut r = rack();
            if eq {
                r.set_eq(0, &EqCoeffs::new(PartEq { high_gain: -12, ..PartEq::FLAT }, 48_000.0));
            }
            r.process(1, 0x90, 60, 100);
            play(&mut r, 4800, 480)
        };
        assert_eq!(solo(true), solo(false), "another part's EQ");
    }

    /// With portamento on, CC77's vibrato (folded into CC1 by the tone) reaches a gliding
    /// note on a channel of its own, not only the part's home channel.
    #[test]
    fn vibrato_depth_reaches_a_gliding_note() {
        let run = |depth: Option<i32>| {
            let mut r = rack();
            r.process(2, 0xB0, 65, 127);
            r.process(2, 0xB0, 5, 60);
            r.process(2, 0x90, 48, 100);
            play(&mut r, 4800, 480);
            r.process(2, 0x80, 48, 0);
            play(&mut r, 96_000, 480);
            assert!(play(&mut r, 4800, 480).iter().all(|&v| v == 0.0), "the home channel has gone quiet");
            if let Some(d) = depth {
                r.process(2, 0xB0, 77, d);
            }
            r.process(2, 0x90, 72, 100);
            assert!(r.parts[2].voicing.glides_in(0), "the new note glides, on a channel of its own");
            play(&mut r, 48_000, 480)
        };
        assert_ne!(run(Some(127)), run(None), "CC77 changes the gliding note");
    }

    /// The rack's mix (left + right) over `frames`, rendered in buffers of `buf`.
    fn play(rack: &mut Rack, frames: usize, buf: usize) -> Vec<f32> {
        let p = peaks();
        let (mut l, mut r) = (vec![0f32; buf], vec![0f32; buf]);
        let mut mix = Vec::new();
        while mix.len() < frames {
            let n = buf.min(frames - mix.len());
            rack.render_dry(&mut l[..n], &mut r[..n], &p, None);
            mix.extend(l[..n].iter().zip(&r[..n]).map(|(a, b)| a + b));
        }
        mix
    }

    /// Zero crossings in each `win`-frame window (the tiny font's square wave: its pitch).
    fn crossings(x: &[f32], win: usize) -> Vec<usize> {
        x.chunks(win).map(|c| c.windows(2).filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0)).count()).collect()
    }

    /// Mono with portamento, in the rack (on upstream rustysynth's note-on, note-off, RPN
    /// and pitch bend): the second key glides up from the first and lands on its own pitch;
    /// letting go of it glides back down to the key still held. Buffers of any size (here
    /// not a whole number of blocks) give the same sound.
    #[test]
    fn mono_portamento_glides_up_and_back() {
        // Each 50 ms window's crossings over 0.5 s.
        let reference = |key: i32| {
            let mut r = rack();
            r.process(2, 0x90, key, 100);
            crossings(&play(&mut r, 24_000, 480)[..24_000], 2400)
        };
        let (c3, c4) = (reference(48), reference(60));
        let tail = |x: &[usize]| x[7..].iter().sum::<usize>();
        assert!(tail(&c4) > tail(&c3) * 3 / 2, "{c3:?} {c4:?}");
        let run = |buf: usize| {
            let mut r = rack();
            // Time 64: an octave in 0.32 s.
            for cc in [[126, 0], [65, 127], [5, 64]] {
                r.process(2, 0xB0, cc[0], cc[1]);
            }
            r.process(2, 0x90, 48, 100);
            play(&mut r, 4800, buf);
            r.process(2, 0x90, 60, 100);
            let up = play(&mut r, 24_000, buf);
            assert_eq!(r.voices(), 2, "two keys held");
            r.process(2, 0x80, 60, 0);
            let down = play(&mut r, 24_000, buf);
            (crossings(&up[..24_000], 2400), crossings(&down[..24_000], 2400))
        };
        let (up, down) = run(480);
        assert!(up[0] < c4[0] * 3 / 4 && up[0] < up[2] && up[2] < up[4], "starts low and climbs: {up:?} vs C4 {c4:?}");
        assert!(tail(&up).abs_diff(tail(&c4)) <= 2, "lands on C4: {up:?} vs {c4:?}");
        assert!(down[0] > c3[0] * 5 / 4, "glides back down: {down:?} vs C3 {c3:?}");
        assert!(tail(&down).abs_diff(tail(&c3)) <= 2, "lands on C3: {down:?} vs {c3:?}");
        assert_eq!(run(333), (up, down), "the same in any buffer size");
    }

    /// Portamento off (its time set) and poly mode play exactly as neither was sent.
    #[test]
    fn portamento_off_and_poly_change_nothing() {
        let run = |setup: &[[i32; 2]]| {
            let mut r = rack();
            for &[cc, v] in setup {
                r.process(2, 0xB0, cc, v);
            }
            for k in [48, 55, 60] {
                r.process(2, 0x90, k, 100);
            }
            let mut out = play(&mut r, 4800, 480);
            r.process(2, 0x80, 55, 0);
            r.process(2, 0x90, 64, 100);
            out.extend(play(&mut r, 4800, 480));
            out
        };
        assert_eq!(run(&[]), run(&[[5, 40], [65, 0], [127, 0]]));
    }

    /// Poly portamento: a new note glides on a channel of its own; a note held keeps its
    /// pitch. CC126/127 and CC5/65 never reach the synthesizer (upstream has none).
    #[test]
    fn poly_portamento_leaves_the_held_note_alone() {
        let held_alone = {
            let mut r = rack();
            r.process(2, 0x90, 48, 100);
            play(&mut r, 9600, 480)
        };
        let mut r = rack();
        r.process(2, 0xB0, 65, 127);
        r.process(2, 0xB0, 5, 60);
        r.process(2, 0x90, 48, 100);
        let first = play(&mut r, 4800, 480);
        assert_eq!(first[..], held_alone[..4800], "the first note: no glide, as without portamento");
        r.process(2, 0x90, 72, 100);
        let both = play(&mut r, 4800, 480);
        // Take the held note away (the same note alone, in the same place): what is left
        // is the gliding one, which starts well below C5.
        let rest: Vec<f32> = both.iter().zip(&held_alone[4800..]).map(|(a, b)| a - b).collect();
        let c5 = {
            let mut q = rack();
            q.process(2, 0x90, 72, 100);
            crossings(&play(&mut q, 960, 480), 960)[0]
        };
        let early = crossings(&rest[..960], 960)[0];
        assert!(early * 2 < c5, "the new note glides up: {early} crossings in its first 20 ms vs C5's {c5}");
    }

    /// A rack on the tiny font with these instrument generators.
    fn font_rack(gens: &[(u16, i16)]) -> Rack {
        let font = Arc::new(SoundFont::new(&mut &crate::patches::sf2::tiny_sound_font_with(&[(0, 0, "Tone")], gens)[..]).unwrap());
        Rack::new(&font, 48_000).unwrap()
    }

    /// A pad: 0.1 s attack, 0.5 s decay to -20 dB, 0.3 s release.
    fn pad_rack() -> Rack {
        font_rack(&[(34, -3986), (36, -1200), (37, 200), (38, -2084)])
    }

    /// Strings: 0.1 s attack, held at full, 0.3 s release.
    fn strings_rack() -> Rack {
        font_rack(&[(34, -3986), (38, -2084)])
    }

    /// Each `win`-frame window's RMS.
    fn rms(x: &[f32], win: usize) -> Vec<f32> {
        x.chunks(win).map(|c| (c.iter().map(|v| v * v).sum::<f32>() / c.len() as f32).sqrt()).collect()
    }

    /// A note on part 3 of `r` after `setup`, held `hold` frames, then let go for `tail`.
    fn pad_note(r: &mut Rack, setup: &[[i32; 2]], hold: usize, tail: usize) -> Vec<f32> {
        for &[cc, v] in setup {
            r.process(3, 0xB0, cc, v);
        }
        r.process(3, 0x90, 60, 100);
        let mut out = play(r, hold, 480);
        r.process(3, 0x80, 60, 0);
        out.extend(play(r, tail, 480));
        out
    }

    /// #346 step 4: CC73/75/72 never reach rustysynth: a note played straight on the
    /// part's synthesizer after them sounds as on a rack that never had them.
    #[test]
    fn envelope_controllers_never_reach_rustysynth() {
        let raw = |setup: &[[i32; 2]]| {
            let mut r = pad_rack();
            for &[cc, v] in setup {
                r.process(3, 0xB0, cc, v);
            }
            let synth = &mut r.parts[3].lanes[0].synth;
            synth.note_on(3, 60, 100);
            let (mut l, mut rr) = (vec![0f32; 24_000], vec![0f32; 24_000]);
            synth.render(&mut l, &mut rr);
            synth.note_off(3, 60);
            let mut tail = (vec![0f32; 24_000], vec![0f32; 24_000]);
            synth.render(&mut tail.0, &mut tail.1);
            l.extend(tail.0);
            l
        };
        assert_eq!(raw(&[[73, 127], [75, 0], [72, 0]]), raw(&[]));
        assert_eq!(raw(&[[72, 127]]), raw(&[]));
    }

    /// At 64, and in the directions that can't be played (a faster attack, a longer
    /// decay), the envelope controllers change nothing: bit-identical.
    #[test]
    fn neutral_envelope_controllers_change_nothing() {
        let plain = pad_note(&mut pad_rack(), &[], 48_000, 48_000);
        assert!(plain.iter().any(|v| v.abs() > 1e-3));
        for setup in [&[[73, 64], [75, 64], [72, 64]][..], &[[73, 0], [75, 127]]] {
            assert_eq!(pad_note(&mut pad_rack(), setup, 48_000, 48_000), plain, "{setup:?}");
        }
    }

    /// CC73 127: the note fades in over about 15x its attack (1.5 s), never louder than
    /// without it, then plays as without it.
    #[test]
    fn a_slower_attack_fades_the_note_in() {
        let own = rms(&pad_note(&mut strings_rack(), &[], 144_000, 0), 4800);
        let slow = rms(&pad_note(&mut strings_rack(), &[[73, 127]], 144_000, 0), 4800);
        assert!(slow[1] < own[1] * 0.2 && slow[5] < own[5] * 0.6, "fades in: {:?} vs {:?}", &slow[..6], &own[..6]);
        assert!(slow[5] > slow[1] && slow[10] > slow[5], "climbing: {slow:?}");
        assert!(slow.iter().zip(&own).all(|(s, o)| *s <= o * 1.001), "never louder: {slow:?} vs {own:?}");
        assert!((slow[25] / own[25] - 1.0).abs() < 0.01, "then as without it: {} vs {}", slow[25], own[25]);
    }

    /// CC75 0: the note falls to its sustain level sooner while held.
    #[test]
    fn a_shorter_decay_falls_sooner() {
        let own = rms(&pad_note(&mut pad_rack(), &[], 48_000, 0), 2400);
        let short = rms(&pad_note(&mut pad_rack(), &[[75, 0]], 48_000, 0), 2400);
        // 0.15-0.2 s: the own decay is half way down, the short one at its sustain.
        assert!(short[3] < own[3] * 0.6, "{} vs {}", short[3], own[3]);
        assert!((short[19] / own[19] - 1.0).abs() < 0.01, "both at the sustain level");
    }

    /// CC72: a shorter release dies away sooner, a longer one later (the note-off held
    /// back while the note fades); neither is louder than the note was.
    #[test]
    fn the_release_is_shorter_or_longer() {
        let tail = |setup: &[[i32; 2]]| {
            let x = pad_note(&mut pad_rack(), setup, 48_000, 96_000);
            (rms(&x[45_600..48_000], 2400)[0], rms(&x[48_000..], 2400))
        };
        let ((held, own), (_, short), (_, long)) = (tail(&[]), tail(&[[72, 0]]), tail(&[[72, 127]]));
        let sum = |x: &[f32]| x.iter().map(|v| v * v).sum::<f32>();
        assert!(sum(&short) < sum(&own) * 0.3, "shorter: {short:?} vs {own:?}");
        assert!(sum(&long) > sum(&own) * 3.0, "longer: {long:?} vs {own:?}");
        assert!(long[0] <= held * 1.001 && long[4] > own[4] * 2.0, "{long:?} (held {held})");
        // The long tail ends: its channel is cut once inaudible, and the lane goes idle.
        let mut r = pad_rack();
        pad_note(&mut r, &[[72, 127]], 48_000, 5 * 48_000);
        assert!(!r.parts[3].env.on_lane(0), "done shaping");
        assert!(play(&mut r, 4800, 480).iter().all(|&v| v == 0.0), "silent");
    }

    /// The player's CC11 still works with the shaping: a shaped note's level follows it as
    /// an unshaped note's does.
    #[test]
    fn the_players_expression_combines_with_the_shaping() {
        // A longer release shapes nothing while the note is held: CC11 plays as without.
        let held = |setup: &[[i32; 2]]| {
            let mut r = pad_rack();
            for &[cc, v] in setup {
                r.process(3, 0xB0, cc, v);
            }
            r.process(3, 0x90, 60, 100);
            let mut out = play(&mut r, 9600, 480);
            r.process(3, 0xB0, 11, 64);
            out.extend(play(&mut r, 9600, 480));
            out
        };
        let (plain, shaped) = (held(&[]), held(&[[72, 127]]));
        assert!(plain.iter().zip(&shaped).all(|(a, b)| (a - b).abs() < 1e-6), "CC11 plays as without the shaping");
        // During a slow attack: CC11 64 takes the level down by (64 / 127)², as it would.
        let attack = |cc11: bool| {
            let mut r = strings_rack();
            r.process(3, 0xB0, 73, 127);
            if cc11 {
                r.process(3, 0xB0, 11, 64);
            }
            r.process(3, 0x90, 60, 100);
            rms(&play(&mut r, 48_000, 480), 4800)
        };
        let (full, less) = (attack(false), attack(true));
        let want = (64.0f32 / 127.0).powi(2);
        assert!((less[6] / full[6] / want - 1.0).abs() < 0.02, "{} vs {}", less[6] / full[6], want);
    }

    /// Each note is shaped on its own: a new note's slow attack leaves a note already at
    /// full untouched (they play on channels of their own).
    #[test]
    fn a_new_notes_attack_leaves_the_others_alone() {
        let run = |second: bool| {
            let mut r = strings_rack();
            r.process(3, 0xB0, 73, 127);
            r.process(3, 0x90, 60, 100);
            play(&mut r, 96_000, 480);
            if second {
                r.process(3, 0x90, 67, 100);
            }
            play(&mut r, 2400, 480)
        };
        let (alone, both) = (run(false), run(true));
        let diff: Vec<f32> = both.iter().zip(&alone).map(|(a, b)| a - b).collect();
        let (d, a) = (rms(&diff, 2400)[0], rms(&alone, 2400)[0]);
        assert!(a > 1e-3 && d < a * 0.05, "the held note keeps its level: the new one adds {d} to {a}");
    }

    /// A drum note with sends of its own (#239) plays in a note lane with those sends: the
    /// same sends share a lane, other sends take an idle one, and with none idle the
    /// nearest. A note with the part's sends plays on the part's own synthesizer.
    #[test]
    fn notes_with_their_own_sends_play_in_a_lane_of_their_own() {
        let mut rack = rack();
        let lane = |rack: &Rack, sends: [f32; BUSES]| rack.parts[9].lanes.iter().position(|l| l.sends == sends && l.quiet == 0);
        rack.drum_note(9, 36, 100, [0.0, 1.0, 1.0], None);
        assert_eq!(lane(&rack, [0.0, 1.0, 1.0]), Some(1), "the first note lane");
        rack.drum_note(9, 38, 100, [0.0, 1.0, 1.0], None);
        assert_eq!(rack.parts[9].lanes.iter().filter(|l| l.quiet == 0).count(), 1, "the same sends, the same lane");
        rack.drum_note(9, 42, 100, [0.5, 1.0, 1.0], None);
        rack.drum_note(9, 44, 100, [0.5, 0.0, 1.0], None);
        rack.drum_note(9, 46, 100, [1.0, 0.25, 1.0], None);
        assert_eq!(lane(&rack, [1.0, 0.25, 1.0]), Some(4), "each new sends in an idle lane");
        rack.drum_note(9, 49, 100, [0.1, 1.0, 1.0], None);
        assert_eq!(rack.parts[9].lanes[1].sends, [0.0, 1.0, 1.0], "none idle: the nearest lane, as it is");
        assert_eq!(rack.voices(), 6);
        rack.drum_note(9, 51, 100, PART_SENDS, None);
        assert_eq!(rack.parts[9].lanes[0].quiet, 0, "the part's own sends: its own synthesizer");
        // Every lane gets the part's note-offs.
        for key in [36, 38, 42, 44, 46, 49, 51] {
            rack.process(9, 0x80, key, 0);
        }
        assert_eq!(rack.voices(), 0);
    }

    /// The tiny font, its preset data kept, so kits can be derived from it.
    fn kit_rack() -> Rack {
        let font = crate::synth::font::read_arc(&mut std::io::Cursor::new(crate::patches::sf2::tiny_gm_sound_font())).unwrap();
        Rack::new(&font, 48_000).unwrap()
    }

    /// A kit (#239): once in, the part's drum notes play in its lanes, which start from
    /// the controllers the part had (a kit of nothing set plays exactly as the part's own
    /// synthesizer). A second kit goes beside it; a third replaces the one played least
    /// lately, whose lanes come back to be freed. Every kit lane gets the part's note-offs.
    #[test]
    fn a_kit_plays_the_parts_drum_notes_from_the_parts_controllers() {
        let none = [[crate::synth::drum_setup::NONE; crate::synth::drum_setup::PARAMS]; 128];
        let setup = |r: &mut Rack| {
            for cc in [[7, 50], [10, 20], [11, 90], [101, 0], [100, 1], [6, 70], [101, 127]] {
                r.process(9, 0xB0, cc[0], cc[1]);
            }
            r.process(9, 0xE0, 0, 80);
        };
        let (mut own, mut kitted) = (kit_rack(), kit_rack());
        setup(&mut own);
        setup(&mut kitted);
        let first = crate::synth::kit::for_part(&kitted, 9, &none);
        let shell = kitted.install_kit(9, first);
        assert!(shell.lanes.is_empty(), "moved in, nothing replaced");
        assert_eq!(kitted.parts[9].kits.live, Some(0));
        own.drum_note(9, 38, 100, PART_SENDS, None);
        kitted.drum_note(9, 38, 100, PART_SENDS, Some(0));
        let base = kitted.slots + NOTE_LANES;
        assert_eq!(kitted.parts[9].lanes[base].quiet, 0, "in the kit's first lane");
        assert!(kitted.parts[9].lanes[..base].iter().all(|l| l.quiet > 0), "not on the font's");
        let (a, b) = (play(&mut own, 9600, 480), play(&mut kitted, 9600, 480));
        assert!(a.iter().any(|&x| x != 0.0));
        assert_eq!(a, b, "the kit plays as the part's own synthesizer, from its controllers");
        // Its own sends: the kit's other lanes.
        kitted.drum_note(9, 40, 100, [0.0, 1.0, 1.0], Some(0));
        assert_eq!(kitted.parts[9].lanes[base + 1].quiet, 0);

        let mut level = none;
        level[38][2] = 50;
        let second = crate::synth::kit::for_part(&kitted, 9, &level);
        assert!(kitted.install_kit(9, second).lanes.is_empty());
        assert_eq!((kitted.parts[9].kits.live, kitted.parts[9].lanes.len()), (Some(1), base + 2 * KIT_LANES));
        assert!(kitted.play_kit(9, &crate::synth::kit::for_part(&kitted, 9, &none).key) == Some(0), "the first is still there");
        level[38][2] = 60;
        let third = crate::synth::kit::for_part(&kitted, 9, &level);
        let back = kitted.install_kit(9, third);
        assert_eq!((back.lanes.len(), kitted.parts[9].kits.live), (KIT_LANES, Some(1)), "the second's set, played least lately");
        for key in [38, 40] {
            kitted.process(9, 0x80, key, 0);
        }
        assert_eq!(kitted.voices(), 0);
    }
}
