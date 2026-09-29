//! XG Drum Setup kits (#239) on upstream rustysynth (#346 step 5).
//!
//! rustysynth starts a note only as its SoundFont's generators say, so a drum part whose
//! setup changes how its notes sound plays a **kit**: a small SoundFont derived from the
//! part's drum kit ([`derive`]), each note the setup sets baked into its generators, and
//! the part's synthesizers on it (its *kit lanes*, rack.rs). The kit holds the whole kit
//! preset, notes set or not, so its notes choke each other (exclusive classes) and share
//! voices as the kit's own do; only the samples it plays are copied.
//!
//! What a note's settings ([`NoteSettings::steps`]) become:
//! - **Level**: initial attenuation, so that the note's gain is (level / 100)² as before
//!   (rustysynth plays 0.4 of the attenuation: 1000 x log10(100 / level) centibels).
//! - **Coarse / fine tune**: coarse and fine tune, added.
//! - **Pan**: the pan, in place of the kit's (every layer of the note, a stereo pair too).
//! - **Cutoff**: initial filter cutoff moved by the setup's cents, then held within
//!   19.45 Hz - 20 kHz as the vendored rustysynth held it.
//! - **Resonance**: initial filter Q moved by the setup's centibels (held within 0-96 dB),
//!   and attenuation moved back by what rustysynth takes off for the Q (half of it), so
//!   only the filter changes, as before.
//! - **EG attack, decay 1 and decay 2**: the volume envelope's attack, decay and release
//!   timecents.
//!
//! Every amount is exact but the attenuation's, which is rounded to a centibel
//! (±0.02 dB). Every instrument zone carries the default velocity -> cutoff modulator
//! switched off, as font.rs writes the fonts it loads.
//!
//! Kits are built off the audio thread: the audio thread asks for one through a ring
//! ([`KitCore`]) and the control side builds it ([`KitLink::pump`], on a thread of its own,
//! or [`KitLink::serve`] at once) and hands it back through another. The style's kits are
//! asked for as it loads ([`KitLink::prebuild`]), so its first downbeat has them. A kit
//! the audio thread no longer holds goes back to the control side to be freed.

use super::drum_setup::{KitParams, NoteSettings, Prebuild, NONE};
use super::font::{Modulator, Pdta, GENERATORS, INITIAL_FILTER_FC, INSTRUMENT, KEY_RANGE, MAX_RECORDS, SAMPLE_ID, VEL_RANGE};
use super::rack::Lane;
use anyhow::{anyhow, bail, Context, Result};
use rtrb::{Consumer, Producer, RingBuffer};
use rustysynth::SoundFont;
use std::collections::{HashMap, VecDeque};
use std::sync::{mpsc, Arc};

/// Generator numbers (SF2 2.01 section 8.1.2).
const START_OFFSET: u16 = 0;
const END_OFFSET: u16 = 1;
const START_LOOP_OFFSET: u16 = 2;
const END_LOOP_OFFSET: u16 = 3;
const START_COARSE_OFFSET: u16 = 4;
const INITIAL_FILTER_Q: u16 = 9;
const END_COARSE_OFFSET: u16 = 12;
const PAN: u16 = 17;
const ATTACK_VOLUME: u16 = 34;
const DECAY_VOLUME: u16 = 36;
const RELEASE_VOLUME: u16 = 38;
const START_LOOP_COARSE_OFFSET: u16 = 45;
const INITIAL_ATTENUATION: u16 = 48;
const END_LOOP_COARSE_OFFSET: u16 = 50;
const COARSE_TUNE: u16 = 51;
const FINE_TUNE: u16 = 52;
/// rustysynth's instrument defaults for the generators a kit moves.
const DEFAULT_CUTOFF: i32 = 13500;
const DEFAULT_TIME: i32 = -12000;
/// The cutoff range the vendored rustysynth held a note's own cutoff to, in cents:
/// 19.45 Hz and 20 kHz (8.176 Hz x 2^(cents / 1200)).
const MIN_CUTOFF: i32 = 1500;
const MAX_CUTOFF: i32 = 13508;
/// The most filter Q, in centibels (96 dB).
const MAX_Q: i32 = 960;
/// Attenuation for Level 0 (centibels): rustysynth plays 0.4 of it, far below hearing.
const SILENT: f64 = 14_400.0;
/// Bytes of a sample header record.
const SHDR: usize = 46;
/// Zero samples after each sample, as SF2 asks.
const PAD: usize = 46;

/// A zone's generators, merged as rustysynth reads them (a local zone's over its global
/// zone's; unknown kinds left out).
type Gens = [Option<i16>; GENERATORS as usize];

fn merged(global: &[[u16; 2]], local: &[[u16; 2]]) -> Gens {
    let mut g = [None; GENERATORS as usize];
    for r in global.iter().chain(local).filter(|r| r[0] < GENERATORS) {
        g[r[0] as usize] = Some(r[1] as i16);
    }
    g
}

#[inline]
fn get(g: &Gens, kind: u16, default: i32) -> i32 {
    g[kind as usize].map_or(default, |v| v as i32)
}

/// A key or velocity range: (lowest, highest).
fn range(g: &Gens, kind: u16) -> (u8, u8) {
    let r = g[kind as usize].map_or(0x7F00, |v| v as u16);
    ((r & 0xFF) as u8, (r >> 8) as u8)
}

fn to_i16(v: i32) -> i16 {
    v.clamp(i16::MIN as i32, i16::MAX as i32) as i16
}

/// Move instrument zone `ig` (under preset zone `pg`) by a note's settings `p`
/// (`NoteSettings::steps`; see the module docs).
fn bake(ig: &mut Gens, pg: &Gens, p: &[u8; super::drum_setup::PARAMS]) {
    let s = NoteSettings::steps(p);
    let add = |ig: &mut Gens, kind: u16, by: i32, default: i32| {
        if by != 0 {
            ig[kind as usize] = Some(to_i16(get(ig, kind, default) + by));
        }
    };
    add(ig, COARSE_TUNE, s.coarse, 0);
    add(ig, FINE_TUNE, s.fine, 0);
    if let Some(pan) = s.pan {
        ig[PAN as usize] = Some(to_i16(pan - get(pg, PAN, 0)));
    }
    if s.cutoff_cents != 0 {
        let own = get(pg, INITIAL_FILTER_FC, 0) + get(ig, INITIAL_FILTER_FC, DEFAULT_CUTOFF);
        let fc = (own + s.cutoff_cents).clamp(MIN_CUTOFF, MAX_CUTOFF);
        ig[INITIAL_FILTER_FC as usize] = Some(to_i16(fc - get(pg, INITIAL_FILTER_FC, 0)));
    }
    let mut attenuation = 0f64;
    if s.resonance_cb != 0 {
        let own = get(pg, INITIAL_FILTER_Q, 0) + get(ig, INITIAL_FILTER_Q, 0);
        let q = (own + s.resonance_cb).clamp(0, MAX_Q);
        ig[INITIAL_FILTER_Q as usize] = Some(to_i16(q - get(pg, INITIAL_FILTER_Q, 0)));
        // rustysynth takes 0.5 x Q off the gain and plays 0.4 x the attenuation.
        attenuation -= 1.25 * (q - own) as f64;
    }
    if let Some(level) = s.level {
        attenuation += if level == 0 { SILENT } else { -1000.0 * (level as f64 / super::drum_setup::DEFAULT_LEVEL as f64).log10() };
    }
    add(ig, INITIAL_ATTENUATION, attenuation.round() as i32, 0);
    add(ig, ATTACK_VOLUME, s.attack_tc, DEFAULT_TIME);
    add(ig, DECAY_VOLUME, s.decay_tc, DEFAULT_TIME);
    add(ig, RELEASE_VOLUME, s.release_tc, DEFAULT_TIME);
}

/// A zone's records: key and velocity range first and `last` (the instrument or the
/// sample) last, as SF2 orders them.
fn records(g: &Gens, last: u16, out: &mut Vec<[u16; 2]>) {
    let middle = (0..GENERATORS).filter(|&k| k != KEY_RANGE && k != VEL_RANGE && k != last);
    for kind in [KEY_RANGE, VEL_RANGE].into_iter().chain(middle).chain([last]) {
        if let Some(v) = g[kind as usize] {
            out.push([kind, v as u16]);
        }
    }
}

/// A preset's (or an instrument's) zones as rustysynth reads them: the global zone's
/// generators, and each local zone's. `at`: where a header holds its first zone; `last`:
/// the generator a local zone ends with.
#[allow(clippy::type_complexity)]
fn zones<'a, const N: usize>(headers: &[[u8; N]], at: usize, bags: &[[u16; 2]], gens: &'a [[u16; 2]], i: usize, last: u16) -> Result<(&'a [[u16; 2]], Vec<&'a [[u16; 2]]>)> {
    if i + 1 >= headers.len() {
        bail!("no record {i}");
    }
    let (z0, z1) = (Pdta::first_zone(headers, i, at), Pdta::first_zone(headers, i + 1, at));
    if z0 > z1 || z1 >= bags.len() {
        bail!("record {i}: zones out of range");
    }
    let zone = |z: usize| Pdta::zone(bags, gens, &[], z).map(|x| x.0);
    let global = z0 < z1 && zone(z0)?.last().is_none_or(|g| g[0] != last);
    let g = if global { zone(z0)? } else { &[][..] };
    let locals = (z0 + global as usize..z1).map(zone).collect::<Result<Vec<_>>>()?;
    Ok((g, locals))
}

/// The kit being written.
#[derive(Default)]
struct Writer {
    /// Its preset's zones.
    preset_zones: Vec<Gens>,
    /// Its instruments: the source's instrument name, and zones.
    insts: Vec<([u8; 20], Vec<Gens>)>,
    /// Instruments written: a source instrument as it is, or baked for a note (and the
    /// preset zone's cutoff, Q and pan it was baked under).
    plain: HashMap<usize, Option<u16>>,
    baked: HashMap<(usize, u8, [i32; 3]), Option<u16>>,
    /// The source's samples it plays, in the order written.
    samples: Vec<u16>,
    sample_of: HashMap<u16, u16>,
}

impl Writer {
    /// Source instrument `i`, as it is or with note `key`'s settings (under preset zone
    /// `pg`): its index in the kit, or None if it plays nothing there.
    fn instrument(&mut self, p: &Pdta, i: usize, bake_for: Option<(u8, &Gens, &[u8; super::drum_setup::PARAMS])>) -> Result<Option<u16>> {
        let memo = bake_for.map(|(k, pg, _)| (i, k, [INITIAL_FILTER_FC, INITIAL_FILTER_Q, PAN].map(|g| get(pg, g, 0))));
        let known = match memo {
            Some(m) => self.baked.get(&m),
            None => self.plain.get(&i),
        };
        if let Some(&x) = known {
            return Ok(x);
        }
        let (global, locals) = zones(&p.inst, 20, &p.ibag, &p.igen, i, SAMPLE_ID)?;
        let mut out = Vec::with_capacity(locals.len());
        for local in locals {
            let mut ig = merged(global, local);
            if let Some((key, pg, params)) = bake_for {
                let (lo, hi) = range(&ig, KEY_RANGE);
                if !(lo..=hi).contains(&key) {
                    continue;
                }
                bake(&mut ig, pg, params);
            }
            let sample = get(&ig, SAMPLE_ID, 0) as u16;
            let n = self.samples.len() as u16;
            let at = *self.sample_of.entry(sample).or_insert(n);
            if at == n {
                self.samples.push(sample);
            }
            ig[SAMPLE_ID as usize] = Some(at as i16);
            out.push(ig);
        }
        let index = if out.is_empty() {
            None
        } else {
            let name: [u8; 20] = p.inst[i][..20].try_into().expect("20 bytes");
            self.insts.push((name, out));
            Some((self.insts.len() - 1) as u16)
        };
        match memo {
            Some(m) => self.baked.insert(m, index),
            None => self.plain.insert(i, index),
        };
        Ok(index)
    }

    /// A preset zone `pg` over `keys`, on instrument `inst` (None: it plays nothing; left
    /// out).
    fn preset_zone(&mut self, pg: &Gens, keys: (u8, u8), inst: Option<u16>) {
        if let Some(inst) = inst {
            let mut g = *pg;
            g[KEY_RANGE as usize] = Some((keys.0 as u16 | (keys.1 as u16) << 8) as i16);
            g[INSTRUMENT as usize] = Some(inst as i16);
            self.preset_zones.push(g);
        }
    }
}

/// The kit for preset `preset` (rustysynth's index) of the font whose preset data is
/// `pdta` and samples `wave`, as SoundFont bytes: that preset alone, each note `params`
/// sets playing with its settings baked in (see the module docs), and only the samples
/// it plays. Slow: off the audio thread.
pub(super) fn derive(pdta: &[u8], wave: &[i16], preset: usize, params: &KitParams) -> Result<Vec<u8>> {
    let p = Pdta::parse(pdta)?;
    let (global, locals) = zones(&p.phdr, 24, &p.pbag, &p.pgen, preset, INSTRUMENT).context("the kit's preset")?;
    let mut w = Writer::default();
    for local in locals {
        let pg = merged(global, local);
        let inst = get(&pg, INSTRUMENT, 0) as u16 as usize;
        let (lo, hi) = range(&pg, KEY_RANGE);
        let set = |k: u8| params[k as usize & 127].iter().any(|&v| v != NONE);
        if lo > hi || !(lo..=hi).any(set) {
            let i = w.instrument(&p, inst, None)?;
            w.preset_zone(&pg, (lo, hi), i);
            continue;
        }
        // Each note the setup sets gets a zone of its own; the notes between keep the
        // zone as it was.
        let mut k = lo;
        loop {
            if set(k) {
                let i = w.instrument(&p, inst, Some((k, &pg, &params[k as usize])))?;
                w.preset_zone(&pg, (k, k), i);
            } else {
                let mut end = k;
                while end < hi && !set(end + 1) {
                    end += 1;
                }
                let i = w.instrument(&p, inst, None)?;
                w.preset_zone(&pg, (k, end), i);
                k = end;
            }
            if k == hi {
                break;
            }
            k += 1;
        }
    }
    if w.preset_zones.is_empty() {
        bail!("the kit's preset plays nothing");
    }
    w.write(&p, wave, preset)
}

impl Writer {
    /// The kit as SoundFont bytes.
    fn write(&self, p: &Pdta, wave: &[i16], preset: usize) -> Result<Vec<u8>> {
        fn chunk(out: &mut Vec<u8>, id: &[u8; 4], body: &[u8]) {
            out.extend(id);
            out.extend((body.len() as u32).to_le_bytes());
            out.extend(body);
            if body.len() % 2 == 1 {
                out.push(0);
            }
        }
        fn list(out: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
            let mut b = kind.to_vec();
            b.extend(body);
            chunk(out, b"LIST", &b);
        }
        let u16s = |v: &mut Vec<u8>, xs: &[u16]| xs.iter().for_each(|x| v.extend(x.to_le_bytes()));
        let u32at = |h: &[u8], at: usize| u32::from_le_bytes([h[at], h[at + 1], h[at + 2], h[at + 3]]) as i64;

        // The samples it plays, each with the stretch of the source's data its zones may
        // reach (their address offsets) and SF2's pad after it.
        let mut smpl: Vec<u8> = Vec::new();
        let mut shdr: Vec<u8> = Vec::with_capacity(SHDR * (self.samples.len() + 1));
        let headers = p.shdr.len() / SHDR;
        for (new, &old) in self.samples.iter().enumerate() {
            if old as usize + 1 >= headers {
                bail!("no sample {old}");
            }
            let h = &p.shdr[old as usize * SHDR..(old as usize + 1) * SHDR];
            let (start, end, start_loop, end_loop) = (u32at(h, 20), u32at(h, 24), u32at(h, 28), u32at(h, 32));
            let (mut lo, mut hi) = (start.min(start_loop), end.max(end_loop));
            for g in self.insts.iter().flat_map(|(_, zs)| zs).filter(|g| get(g, SAMPLE_ID, 0) == new as i32) {
                let off = |fine: u16, coarse: u16| (get(g, fine, 0) + 32768 * get(g, coarse, 0)) as i64;
                lo = lo.min(start + off(START_OFFSET, START_COARSE_OFFSET)).min(start_loop + off(START_LOOP_OFFSET, START_LOOP_COARSE_OFFSET));
                hi = hi.max(end + off(END_OFFSET, END_COARSE_OFFSET)).max(end_loop + off(END_LOOP_OFFSET, END_LOOP_COARSE_OFFSET));
            }
            let lo = lo.clamp(0, wave.len() as i64);
            // One more than the zones reach, for the interpolation, where the source has it.
            let hi = (hi + 2).clamp(lo, wave.len() as i64);
            let base = (smpl.len() / 2) as i64;
            for s in &wave[lo as usize..hi as usize] {
                smpl.extend(s.to_le_bytes());
            }
            smpl.extend([0u8; 2 * PAD]);
            let at = |x: i64| u32::try_from(x - lo + base).map_err(|_| anyhow!("sample {old} out of range"));
            shdr.extend(&h[..20]);
            for x in [start, end, start_loop, end_loop] {
                shdr.extend(at(x)?.to_le_bytes());
            }
            // Rate, original pitch, pitch correction; no link (rustysynth plays none).
            shdr.extend(&h[36..42]);
            shdr.extend(0u16.to_le_bytes());
            shdr.extend(&h[44..46]);
        }
        let mut eos = [0u8; SHDR];
        eos[..3].copy_from_slice(b"EOS");
        shdr.extend(eos);
        if smpl.len() > u32::MAX as usize / 2 {
            bail!("the kit's samples are too big");
        }

        // The preset.
        let (mut pbag, mut pgen) = (Vec::new(), Vec::new());
        let mut gens = Vec::new();
        for g in &self.preset_zones {
            u16s(&mut pbag, &[gens.len() as u16, 0]);
            records(g, INSTRUMENT, &mut gens);
        }
        u16s(&mut pbag, &[gens.len() as u16, 0]);
        let preset_gens = gens.len();
        for g in gens.iter().chain(&[[0, 0]]) {
            u16s(&mut pgen, g);
        }
        let h = &p.phdr[preset];
        let mut phdr = h[..24].to_vec();
        phdr.extend(0u16.to_le_bytes());
        phdr.extend(&h[26..]);
        let mut eop = [0u8; 38];
        eop[..3].copy_from_slice(b"EOP");
        eop[24..26].copy_from_slice(&(self.preset_zones.len() as u16).to_le_bytes());
        phdr.extend(eop);

        // The instruments.
        let (mut inst, mut ibag, mut imod, mut igen) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let (mut zones, mut gens) = (0usize, Vec::new());
        for (name, zs) in &self.insts {
            inst.extend(name);
            inst.extend((zones as u16).to_le_bytes());
            for g in zs {
                u16s(&mut ibag, &[gens.len() as u16, zones as u16]);
                imod.extend(Modulator::DEFAULT_OFF.bytes());
                records(g, SAMPLE_ID, &mut gens);
                zones += 1;
            }
        }
        let mut eoi = [0u8; 22];
        eoi[..3].copy_from_slice(b"EOI");
        eoi[20..].copy_from_slice(&(zones as u16).to_le_bytes());
        inst.extend(eoi);
        u16s(&mut ibag, &[gens.len() as u16, zones as u16]);
        imod.extend([0u8; 10]);
        for g in gens.iter().chain(&[[0, 0]]) {
            u16s(&mut igen, g);
        }
        if [self.preset_zones.len(), preset_gens, zones, gens.len(), self.insts.len(), self.samples.len()].iter().any(|&n| n >= MAX_RECORDS) {
            bail!("the kit is too big for SF2");
        }

        let mut info = Vec::new();
        chunk(&mut info, b"ifil", &[2, 0, 1, 0]);
        chunk(&mut info, b"isng", b"EMU8000\0");
        chunk(&mut info, b"INAM", b"yahaha drum kit\0");
        let mut sdta = Vec::new();
        chunk(&mut sdta, b"smpl", &smpl);
        let mut pdta = Vec::new();
        for (id, body) in [(b"phdr", &phdr), (b"pbag", &pbag), (b"pmod", &vec![0u8; 10]), (b"pgen", &pgen), (b"inst", &inst), (b"ibag", &ibag), (b"imod", &imod), (b"igen", &igen), (b"shdr", &shdr)] {
            chunk(&mut pdta, id, body);
        }
        let mut body = b"sfbk".to_vec();
        list(&mut body, b"INFO", &info);
        list(&mut body, b"sdta", &sdta);
        list(&mut body, b"pdta", &pdta);
        let mut out = Vec::with_capacity(body.len() + 8);
        chunk(&mut out, b"RIFF", &body);
        Ok(out)
    }
}

/// The preset rustysynth plays for `bank` and `program` on `font` (its index): the preset
/// itself, else bank 0's `program` (a melodic bank) or the standard kit 128:0 (a drum
/// bank), else the preset with the lowest bank and program.
pub(super) fn preset_index(font: &SoundFont, bank: i32, program: i32) -> Option<usize> {
    let presets = font.get_presets();
    // rustysynth keeps the last of the same bank and program.
    let find = |b: i32, p: i32| presets.iter().rposition(|x| x.get_bank_number() == b && x.get_patch_number() == p);
    find(bank, program).or_else(|| if bank < 128 { find(0, program) } else { find(128, 0) }).or_else(|| {
        let mut best: Option<(i32, usize)> = None;
        for (i, x) in presets.iter().enumerate() {
            let id = (x.get_bank_number() << 16) | x.get_patch_number();
            if best.is_none_or(|(b, _)| id < b) {
                best = Some((id, i));
            }
        }
        best.map(|b| b.1)
    })
}

/// A SoundFont a kit can be derived from: the font and the preset data it was loaded with
/// (font.rs).
pub struct KitSource {
    font: Arc<SoundFont>,
    pdta: Arc<Vec<u8>>,
}

impl KitSource {
    /// `font`'s, if its preset data was kept (`font::open`, `font::read_arc`).
    pub fn of(font: &Arc<SoundFont>) -> Option<Arc<KitSource>> {
        Some(Arc::new(KitSource { font: font.clone(), pdta: super::font::preset_data(font)? }))
    }

    #[inline]
    pub(super) fn font(&self) -> &SoundFont {
        &self.font
    }

    /// Which font it is (for a kit's key).
    #[inline]
    pub(super) fn id(&self) -> usize {
        Arc::as_ptr(&self.font) as usize
    }
}

/// What a kit is: the font and preset it comes from, what its lanes are built for, and
/// the setup baked in.
#[derive(Clone, PartialEq)]
pub struct KitKey {
    pub(super) font: usize,
    pub(super) preset: u32,
    pub(super) sample_rate: i32,
    pub(super) legacy: bool,
    pub(super) params: KitParams,
}

impl KitKey {
    pub(super) const EMPTY: KitKey = KitKey { font: 0, preset: 0, sample_rate: 0, legacy: false, params: [[NONE; super::drum_setup::PARAMS]; 128] };
}

/// A kit: its key and its lanes (once installed, the lanes it replaced, or none).
pub struct Kit {
    pub(super) key: KitKey,
    pub(super) lanes: Vec<Lane>,
}

/// A kit the audio thread asks for.
pub struct KitRequest {
    source: Arc<KitSource>,
    key: KitKey,
}

/// Build the kit `r` asks for (slow: off the audio thread).
pub fn build(r: &KitRequest) -> Result<Box<Kit>> {
    let bytes = derive(&r.source.pdta, r.source.font.get_wave_data(), r.key.preset as usize, &r.key.params)?;
    let font = Arc::new(SoundFont::new(&mut &bytes[..]).map_err(|e| anyhow!("the kit: {e:?}"))?);
    let lanes = super::rack::kit_lanes(&font, r.key.sample_rate, r.key.legacy)?;
    Ok(Box::new(Kit { key: r.key.clone(), lanes }))
}

/// The kit `rack`'s part on `ch` would ask for with `params` (tests: built at once).
#[cfg(test)]
pub(super) fn for_part(rack: &super::Rack, ch: u8, params: &KitParams) -> Box<Kit> {
    let (slot, bank, program) = rack.voice_of(ch);
    let source = rack.kit_source(slot).expect("a font with its preset data kept").clone();
    let preset = preset_index(source.font(), bank, program).expect("a preset");
    let mut key = KitKey::EMPTY;
    key.params = *params;
    rack.kit_key(&mut key, &source, preset);
    build(&KitRequest { source, key }).unwrap()
}

/// Ring sizes: requests in flight, kits on their way to the audio thread, kits on their
/// way back, prebuilds.
const REQUESTS: usize = 4;
const ARRIVALS: usize = 4;
const RETIRED: usize = 16;
const PREBUILDS: usize = 8;
/// Kits the audio thread keeps built and not playing, and requests it remembers.
const CACHE: usize = 4;
const ASKED: usize = 4;

/// The two ends of the kits: the control side's and the audio thread's.
pub fn link() -> (KitLink, KitCore) {
    let (req_tx, req_rx) = RingBuffer::new(REQUESTS);
    let (kit_tx, kit_rx) = RingBuffer::new(ARRIVALS);
    let (old_tx, old_rx) = RingBuffer::new(RETIRED);
    let (pre_tx, pre_rx) = RingBuffer::new(PREBUILDS);
    (
        KitLink { requests: req_rx, kits: kit_tx, retired: old_rx, prebuild: pre_tx, waiting: VecDeque::new(), ready: VecDeque::new(), worker: None },
        KitCore {
            requests: req_tx,
            arrived: kit_rx,
            retired: old_tx,
            prebuilds: pre_rx,
            cache: [None, None, None, None],
            next: 0,
            asked: [None, None, None, None],
            next_ask: 0,
            parked: None,
            want: KitKey::EMPTY,
        },
    )
}

/// The builder thread: requests in, kits (or why not) out.
struct Worker {
    tx: mpsc::Sender<KitRequest>,
    rx: mpsc::Receiver<Result<Box<Kit>, String>>,
}

impl Worker {
    fn spawn() -> Option<Worker> {
        let (tx, jobs) = mpsc::channel::<KitRequest>();
        let (done, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("yahaha-kits".into())
            .spawn(move || {
                while let Ok(r) = jobs.recv() {
                    if done.send(build(&r).map_err(|e| format!("{e:#}"))).is_err() {
                        break;
                    }
                }
            })
            .ok()?;
        Some(Worker { tx, rx })
    }
}

/// The control side's end of the kits: it builds what the audio thread asks for, hands
/// the kits over and frees those that come back.
pub struct KitLink {
    requests: Consumer<KitRequest>,
    kits: Producer<Box<Kit>>,
    retired: Consumer<Box<Kit>>,
    prebuild: Producer<Prebuild>,
    /// Prebuilds and kits waiting for room in their ring.
    waiting: VecDeque<Prebuild>,
    ready: VecDeque<Box<Kit>>,
    worker: Option<Worker>,
}

impl KitLink {
    /// Ask the audio thread to build the kits a style's setup plays ([`super::drum_setup::prebuilds`])
    /// as it loads, before any of its notes. It finds each part's voice as its program
    /// change will leave it, so the kits are ready when the setup comes.
    pub fn prebuild(&mut self, plan: Vec<Prebuild>) {
        self.waiting.extend(plan);
        self.hand_over();
    }

    fn hand_over(&mut self) {
        while self.retired.pop().is_ok() {}
        while let Some(p) = self.waiting.pop_front() {
            if let Err(rtrb::PushError::Full(p)) = self.prebuild.push(p) {
                self.waiting.push_front(p);
                break;
            }
        }
        while let Some(k) = self.ready.pop_front() {
            if let Err(rtrb::PushError::Full(k)) = self.kits.push(k) {
                self.ready.push_front(k);
                break;
            }
        }
    }

    /// For the control side's loop: new requests go to the builder thread, built kits to
    /// the audio thread, and kits back from it are freed. Never blocks. Returns why a kit
    /// could not be built, if one failed.
    pub fn pump(&mut self) -> Option<String> {
        self.hand_over();
        let mut failed = None;
        if self.requests.slots() > 0 && self.worker.is_none() {
            self.worker = Worker::spawn();
        }
        if let Some(w) = &self.worker {
            while let Ok(r) = self.requests.pop() {
                let _ = w.tx.send(r);
            }
            while let Ok(k) = w.rx.try_recv() {
                match k {
                    Ok(k) => self.ready.push_back(k),
                    Err(e) => failed = Some(e),
                }
            }
        }
        self.hand_over();
        failed
    }

    /// Build every kit asked for now, on this thread, and hand them over (offline renders
    /// and tests, which have no control loop). Returns how many were built.
    pub fn serve(&mut self) -> usize {
        self.hand_over();
        let mut n = 0;
        while let Ok(r) = self.requests.pop() {
            if let Ok(k) = build(&r) {
                self.ready.push_back(k);
                n += 1;
            }
        }
        self.hand_over();
        n
    }
}

/// The audio thread's end of the kits: what it asks for, the kits built and not playing,
/// and the ring back. Nothing here allocates, locks or frees.
pub struct KitCore {
    requests: Producer<KitRequest>,
    arrived: Consumer<Box<Kit>>,
    retired: Producer<Box<Kit>>,
    pub(super) prebuilds: Consumer<Prebuild>,
    cache: [Option<Box<Kit>>; CACHE],
    next: usize,
    asked: [Option<KitKey>; ASKED],
    next_ask: usize,
    /// A kit the ring back had no room for: it goes on a later buffer.
    parked: Option<Box<Kit>>,
    /// The key a part wants, worked out in place.
    pub(super) want: KitKey,
}

impl KitCore {
    /// Send `kit` back to be freed off the audio thread.
    pub(super) fn retire(&mut self, kit: Box<Kit>) {
        if let Some(p) = self.parked.take()
            && let Err(rtrb::PushError::Full(p)) = self.retired.push(p)
        {
            self.parked = Some(p);
        }
        if let Err(rtrb::PushError::Full(kit)) = self.retired.push(kit)
            && let Some(stray) = self.parked.replace(kit)
        {
            // Both the ring and the slot full: never free on the audio thread.
            std::mem::forget(stray);
        }
    }

    /// Ask for the kit `want` is, from `source`, unless it has been asked for already.
    /// False if the ring had no room (ask again later).
    pub(super) fn ask(&mut self, source: &Arc<KitSource>, want: &KitKey) -> bool {
        if self.asked.iter().flatten().any(|k| k == want) || self.cache.iter().flatten().any(|k| k.key == *want) {
            return true;
        }
        if self.requests.push(KitRequest { source: source.clone(), key: want.clone() }).is_err() {
            return false;
        }
        self.asked[self.next_ask] = Some(want.clone());
        self.next_ask = (self.next_ask + 1) % ASKED;
        true
    }

    /// The kit built for `want`, taken out of the cache.
    pub(super) fn take(&mut self, want: &KitKey) -> Option<Box<Kit>> {
        let i = self.cache.iter().position(|k| k.as_ref().is_some_and(|k| k.key == *want))?;
        self.cache[i].take()
    }

    /// Take in the kits built since the last call; true if any came.
    pub(super) fn receive(&mut self) -> bool {
        let mut any = false;
        while let Ok(kit) = self.arrived.pop() {
            any = true;
            for a in &mut self.asked {
                if a.as_ref() == Some(&kit.key) {
                    *a = None;
                }
            }
            let slot = match self.cache.iter().position(|k| k.is_none()) {
                Some(i) => i,
                None => {
                    let i = self.next;
                    self.next = (self.next + 1) % CACHE;
                    i
                }
            };
            if let Some(old) = self.cache[slot].replace(kit) {
                self.retire(old);
            }
        }
        any
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::drum_setup::PARAMS;

    /// The smallest SoundFont in the checkout's soundfonts/ (None: skip), loaded once.
    fn font() -> Option<Arc<SoundFont>> {
        crate::synth::rack_tests::font()
    }

    /// Setup parameters: `(key, parameter, value)`.
    fn params(set: &[(u8, usize, u8)]) -> Box<KitParams> {
        let mut p = Box::new([[NONE; PARAMS]; 128]);
        for &(k, a, v) in set {
            p[k as usize][a] = v;
        }
        p
    }

    /// The zones rustysynth plays for `key` on `font`'s preset `i` (velocity 100), as
    /// (preset region, instrument region) pairs' sample start and generators it reads.
    fn plays(font: &SoundFont, i: usize, key: i32) -> Vec<(i32, i32, f32, f32, f32, f32, f32, f32)> {
        let insts = font.get_instruments();
        let mut out = Vec::new();
        for p in font.get_presets()[i].get_regions().iter().filter(|r| r.contains(key, 100)) {
            for r in insts[p.get_instrument_id()].get_regions().iter().filter(|r| r.contains(key, 100)) {
                let wave = font.get_wave_data();
                let first = wave[r.get_sample_start() as usize] as i32;
                out.push((
                    first,
                    r.get_coarse_tune() + p.get_coarse_tune(),
                    r.get_pan() + p.get_pan(),
                    r.get_initial_attenuation() + p.get_initial_attenuation(),
                    r.get_initial_filter_cutoff_frequency() * p.get_initial_filter_cutoff_frequency() / 8.176,
                    r.get_initial_filter_q() + p.get_initial_filter_q(),
                    r.get_decay_volume_envelope() * p.get_decay_volume_envelope(),
                    r.get_release_volume_envelope() * p.get_release_volume_envelope(),
                ));
            }
        }
        out
    }

    /// A kit plays every note of its preset as the font does, but the notes its setup
    /// sets, which play with their settings in their generators; it holds that preset
    /// alone, and only the samples it plays. Prints the build's cost for each font (the
    /// PR's measurements).
    #[cfg(feature = "slow-tests")]
    #[test]
    fn a_kit_plays_its_preset_with_the_setup_baked_in() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("soundfonts");
        let files = crate::library::sound_font_files(&dir);
        if files.is_empty() {
            eprintln!("no SoundFont; skipping");
            return;
        }
        for f in files {
            let font = crate::synth::font::open(&dir.join(&f)).unwrap();
            let source = KitSource::of(&font).unwrap();
            let kit = preset_index(&font, 128, 0).unwrap();
            // Level 50, a fifth up, hard left, a darker cutoff with more resonance, a
            // shorter decay and a longer release, on the snare; the kick left alone.
            let set = params(&[(38, 2, 50), (38, 0, 0x47), (38, 4, 1), (38, 0x0B, 0x30), (38, 0x0C, 0x50), (38, 0x0E, 0x50), (38, 0x0F, 0x30)]);
            let t = std::time::Instant::now();
            let bytes = derive(&source.pdta, font.get_wave_data(), kit, &set).unwrap();
            let derive_ms = t.elapsed().as_secs_f64() * 1e3;
            let t = std::time::Instant::now();
            let built = build(&KitRequest { source: source.clone(), key: KitKey { font: source.id(), preset: kit as u32, sample_rate: 48_000, legacy: false, params: *set } }).unwrap();
            let build_ms = t.elapsed().as_secs_f64() * 1e3;
            let k = SoundFont::new(&mut &bytes[..]).unwrap();
            assert_eq!(k.get_presets().len(), 1);
            assert_eq!(k.get_presets()[0].get_name(), font.get_presets()[kit].get_name());
            for key in (27..88).filter(|&k| k != 38) {
                assert_eq!(plays(&k, 0, key), plays(&font, kit, key), "{f}: key {key} as the font plays it");
            }
            let (own, baked) = (plays(&font, kit, 38), plays(&k, 0, 38));
            assert_eq!(own.len(), baked.len(), "{f}: the snare's layers");
            for (o, b) in own.iter().zip(&baked) {
                assert_eq!(b.0, o.0, "the same sample");
                assert_eq!(b.1, o.1 + 7, "a fifth up");
                // Pan 1: (1 - 64) x 50 / 64 = -49.22, in place of the kit's own.
                assert!((b.2 + 49.21875).abs() < 0.05, "hard left: {}", b.2);
                // Level 50: 40 log10(0.5) dB. rustysynth takes 0.4 x the attenuation and
                // 0.5 x the Q off the gain; the kit gives the Q's share back.
                let dq = b.5 - o.5;
                let db = -(0.4 * (b.3 - o.3) + 0.5 * dq);
                assert!((db - 40.0 * 0.5f32.log10()).abs() < 0.03, "level 50: {db} dB");
                let cut = (o.4 * 0.5f32).clamp(19.45, 20_000.0);
                assert!((b.4 / cut - 1.0).abs() < 1e-3, "cutoff an octave down: {} vs {}", b.4, o.4);
                assert!((dq - (o.5 + 3.2).clamp(0.0, 96.0) + o.5).abs() < 1e-3, "resonance +3.2 dB");
                assert!((b.6 / o.6 - 0.5).abs() < 1e-3 && (b.7 / o.7 - 2.0).abs() < 1e-3, "decay halved, release doubled");
            }
            let samples: std::collections::BTreeSet<i32> = (0..128).flat_map(|key| plays(&font, kit, key)).map(|p| p.0).collect();
            eprintln!(
                "{f}: kit {:?} ({} zones), {} KB of samples of {} KB; derive {derive_ms:.1} ms, build (with its lanes) {build_ms:.1} ms",
                font.get_presets()[kit].get_name(),
                k.get_presets()[0].get_regions().len(),
                k.get_wave_data().len() * 2 / 1024,
                font.get_wave_data().len() * 2 / 1024,
            );
            assert!(k.get_wave_data().len() < font.get_wave_data().len(), "only the kit's samples");
            assert!(!samples.is_empty() && built.lanes.len() == super::super::rack::KIT_LANES);
        }
    }

    /// With nothing set, a kit is its preset as the font plays it; an unknown preset or a
    /// font without its preset data gives no kit.
    #[test]
    fn a_kit_of_nothing_is_the_preset_itself() {
        let Some(font) = font() else { return };
        let source = KitSource::of(&font).unwrap();
        let none = params(&[]);
        for (i, p) in font.get_presets().iter().enumerate().step_by(7) {
            let k = SoundFont::new(&mut &derive(&source.pdta, font.get_wave_data(), i, &none).unwrap()[..]).unwrap();
            for key in [36, 38, 60, 72] {
                assert_eq!(plays(&k, 0, key), plays(&font, i, key), "{} key {key}", p.get_name());
            }
        }
        assert!(derive(&source.pdta, font.get_wave_data(), font.get_presets().len(), &none).is_err());
        let plain = Arc::new(crate::synth::font::read(&mut std::io::Cursor::new(crate::patches::sf2::tiny_gm_sound_font())).unwrap());
        assert!(KitSource::of(&plain).is_none(), "no preset data kept");
    }

    /// rustysynth's preset lookup: the preset, else bank 0's program or kit 128:0, else
    /// the lowest.
    #[test]
    fn presets_are_found_as_rustysynth_finds_them() {
        let Some(font) = font() else { return };
        let p = font.get_presets();
        let at = |i: Option<usize>| i.map(|i| (p[i].get_bank_number(), p[i].get_patch_number()));
        assert_eq!(at(preset_index(&font, 0, 0)), Some((0, 0)));
        assert_eq!(at(preset_index(&font, 128, 0)), Some((128, 0)));
        assert_eq!(at(preset_index(&font, 120, 5)), Some((0, 5)), "a missing melodic bank: bank 0");
        assert_eq!(at(preset_index(&font, 128, 126)).map(|b| b.0), Some(128), "a missing kit: a kit");
    }
}
