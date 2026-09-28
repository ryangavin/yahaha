//! Loading a SoundFont for the built-in synth, with velocity -> filter cutoff baked in
//! (#203; #346 step 5, on upstream rustysynth's `SoundFont::new`).
//!
//! A soft note is darker than a hard one: the SF2 2.01 default modulator "note-on velocity
//! to initial filter cutoff". Upstream rustysynth reads no modulators, so the SoundFont's
//! preset data (`pdta`, a few hundred KB) is rewritten as it loads, off the audio thread:
//! each preset zone whose velocity -> cutoff isn't flat is split into velocity layers, and
//! each layer's initial filter cutoff (a preset generator, which adds to the instrument's)
//! is moved by what the modulators give at its velocities. The samples stream through
//! untouched, so the only cost is the extra zones (measured in the tests below). Playing
//! costs nothing more: a note-on finds its layer as it finds any zone.
//!
//! The modulators baked are those the vendored rustysynth played:
//! - the default: velocity, unipolar negative concave, -2400 cents: 0 at velocity 127,
//!   about -300 at 64, -600 at 32;
//! - a zone's own velocity -> cutoff modulators. One the same as a modulator already there
//!   replaces it (SF2 2.01 section 9.5): a local zone's replaces the global zone's, and an
//!   instrument zone's the default. A negative linear velocity source with no amount source
//!   or a velocity switch counts as the same as the default: FluidSynth's and Polyphone's
//!   form of it, which SoundFonts use to switch it off (amount 0). Any other adds, and a
//!   preset's add to its instrument's.
//!
//! Other modulators stay ignored, as upstream.
//!
//! **Precision.** A layer takes the curve's value at its top velocity (so a zone's top
//! velocity, 127 for most, plays exactly as the SoundFont has it), or the middle of its
//! values below that; every velocity is within the tolerance of the curve, plus half a cent
//! of rounding. SF2 indexes zones and generators with 16 bits, so a font holds at most
//! 65,535 of each: the finest tolerance in [`TOLERANCES`] that fits is used.
//!
//! An instrument whose zones differ in their own velocity -> cutoff modulators is split
//! into one instrument per set of them, so that a preset zone's layers can differ per set.
//!
//! Every instrument zone written carries the default modulator switched off (amount 0): the
//! SF2 way to say the default is replaced, as the cutoff is baked in already. A player that
//! reads modulators adds nothing again (upstream rustysynth reads none).
//!
//! The preset data a font was loaded with is kept beside it ([`preset_data`]): an XG Drum
//! Setup's kit is derived from it (kit.rs, #239).

use anyhow::{anyhow, bail, Context, Result};
use rustysynth::SoundFont;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::{Arc, Mutex, Weak};

/// Generator numbers (SF2 2.01 section 8.1.2).
pub(super) const INITIAL_FILTER_FC: u16 = 8;
pub(super) const INSTRUMENT: u16 = 41;
pub(super) const KEY_RANGE: u16 = 43;
pub(super) const VEL_RANGE: u16 = 44;
pub(super) const SAMPLE_ID: u16 = 53;
/// The generators rustysynth knows; it ignores the rest.
pub(super) const GENERATORS: u16 = 61;
/// A zone's key or velocity range when it sets none: 0-127.
const FULL_RANGE: u16 = 0x7F00;
/// The most zones or generators a font can index (16 bits).
pub(super) const MAX_RECORDS: usize = 65_535;
/// The cutoff tolerances tried, in cents, finest first. 0.5 is exact to the cent.
pub const TOLERANCES: [f32; 10] = [0.5, 2.0, 5.0, 10.0, 15.0, 25.0, 50.0, 100.0, 200.0, 2400.0];

/// The preset data each font [`open`] or [`read_arc`] loaded was built from, for as long
/// as the font lives (kit.rs derives drum kits from it). Only the control side and the
/// loaders touch it, never the audio thread.
static PRESET_DATA: Mutex<Vec<(Weak<SoundFont>, Arc<Vec<u8>>)>> = Mutex::new(Vec::new());

/// The preset data `font` was loaded with (as rustysynth read it: velocity -> cutoff
/// baked), if [`open`] or [`read_arc`] loaded it.
pub fn preset_data(font: &Arc<SoundFont>) -> Option<Arc<Vec<u8>>> {
    let list = PRESET_DATA.lock().unwrap_or_else(|e| e.into_inner());
    list.iter().find(|(f, _)| std::ptr::eq(f.as_ptr(), Arc::as_ptr(font))).map(|(_, p)| p.clone())
}

/// `font`, its preset data kept for [`preset_data`]; fonts gone since are forgotten.
fn keep(font: SoundFont, pdta: Vec<u8>) -> Arc<SoundFont> {
    let font = Arc::new(font);
    let mut list = PRESET_DATA.lock().unwrap_or_else(|e| e.into_inner());
    list.retain(|(f, _)| f.strong_count() > 0);
    list.push((Arc::downgrade(&font), Arc::new(pdta)));
    font
}

/// The SoundFont at `path`, velocity -> cutoff baked in unless `YAHAHA_VEL_FILTER=off`
/// ([`super::velocity_to_filter`]). Slow: off the audio thread.
pub fn open(path: &Path) -> Result<Arc<SoundFont>> {
    let file = std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    read_arc(&mut BufReader::new(file)).with_context(|| format!("loading {}", path.display()))
}

/// [`read`], shared and with its preset data kept, as [`open`] loads a file.
pub fn read_arc<R: Read + Seek>(r: &mut R) -> Result<Arc<SoundFont>> {
    let (font, pdta) = load(r, super::velocity_to_filter())?;
    Ok(keep(font, pdta))
}

/// The SoundFont `r` holds (from its current position), as [`open`] loads it.
pub fn read<R: Read + Seek>(r: &mut R) -> Result<SoundFont> {
    read_as(r, super::velocity_to_filter())
}

/// [`read`], velocity -> cutoff baked in (`velocity_tone`) or not.
pub fn read_as<R: Read + Seek>(r: &mut R, velocity_tone: bool) -> Result<SoundFont> {
    Ok(load(r, velocity_tone)?.0)
}

/// The SoundFont `r` holds, and the preset data it was built from.
fn load<R: Read + Seek>(r: &mut R, velocity_tone: bool) -> Result<(SoundFont, Vec<u8>)> {
    let start = r.stream_position()?;
    let (pdta_at, pdta) = find_pdta(r)?;
    let (out, _) = bake(&pdta, velocity_tone)?;
    // Everything before the preset data (the RIFF header, INFO and the samples) streams
    // through as it is; the RIFF size follows the new preset data.
    r.seek(SeekFrom::Start(start + 12))?;
    let riff_size = u32::try_from(pdta_at - 8 + 8 + out.len() as u64).context("SoundFont too big")?;
    let mut head = b"RIFF".to_vec();
    head.extend(riff_size.to_le_bytes());
    head.extend(b"sfbk");
    let mut list = b"LIST".to_vec();
    list.extend((out.len() as u32).to_le_bytes());
    let mut stream = (&head[..]).chain(r.take(pdta_at - 12)).chain(&list[..]).chain(&out[..]);
    let font = SoundFont::new(&mut stream).map_err(|e| anyhow!("{e:?}"))?;
    Ok((font, out))
}

/// The offset of the `pdta` LIST from the start of the SoundFont, and its body (from its
/// "pdta" type on). Leaves `r` anywhere.
fn find_pdta<R: Read + Seek>(r: &mut R) -> Result<(u64, Vec<u8>)> {
    let start = r.stream_position()?;
    let mut h = [0u8; 12];
    r.read_exact(&mut h).context("reading the RIFF header")?;
    if &h[..4] != b"RIFF" || &h[8..] != b"sfbk" {
        bail!("not a SoundFont");
    }
    let mut at = 12u64;
    loop {
        let mut c = [0u8; 12];
        r.read_exact(&mut c[..8]).context("no preset data")?;
        let size = u32::from_le_bytes([c[4], c[5], c[6], c[7]]) as u64;
        if &c[..4] == b"LIST" && size >= 4 {
            r.read_exact(&mut c[8..])?;
            if &c[8..] == b"pdta" {
                if size > 64 << 20 {
                    bail!("preset data too big");
                }
                let mut body = vec![0u8; size as usize];
                body[..4].copy_from_slice(b"pdta");
                r.read_exact(&mut body[4..]).context("truncated preset data")?;
                return Ok((at, body));
            }
        }
        at += 8 + size + (size & 1);
        r.seek(SeekFrom::Start(start + at))?;
    }
}

/// What baking did to a font's preset data (for the tests' measurements).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Baked {
    /// The tolerance used, in cents (0: nothing to bake).
    pub tolerance: f32,
    /// Preset zones before and after, and the generators after.
    pub zones_in: usize,
    pub zones_out: usize,
    pub generators_out: usize,
    /// Preset zones split into velocity layers.
    pub layered: usize,
    /// Instruments split by their zones' velocity -> cutoff modulators.
    pub split: usize,
}

/// The preset data `pdta` (its body, "pdta" first) with velocity -> cutoff baked in (or,
/// with `velocity_tone` off, only the default switched off), and what was done.
pub fn bake(pdta: &[u8], velocity_tone: bool) -> Result<(Vec<u8>, Baked)> {
    bake_within(pdta, velocity_tone, MAX_RECORDS)
}

/// [`bake`] with at most `limit` zones and generators on each side (the format's
/// [`MAX_RECORDS`]; lower in tests). If the instruments can't fit, the preset data as it was.
fn bake_within(pdta: &[u8], velocity_tone: bool, limit: usize) -> Result<(Vec<u8>, Baked)> {
    let p = Pdta::parse(pdta)?;
    let insts = p.instruments()?;
    let inst_zones: usize = insts.out.iter().map(|(_, zs)| zs.len()).sum();
    let inst_gens: usize = insts.out.iter().flat_map(|(_, zs)| zs).map(|&z| (p.ibag[z + 1][0] as usize).saturating_sub(p.ibag[z][0] as usize)).sum();
    if inst_zones > limit || inst_gens > limit {
        // The split instruments don't fit in 16 bits: the font as it was, a flat tone.
        let zones = p.pbag.len().saturating_sub(1);
        let baked = Baked { zones_in: zones, zones_out: zones, generators_out: p.pgen.len().saturating_sub(1), ..Baked::default() };
        return Ok((pdta.to_vec(), baked));
    }
    let mut last = None;
    for &tol in &TOLERANCES {
        let presets = p.presets(&insts, velocity_tone.then_some(tol))?;
        let fits = presets.generators.len() <= limit && presets.bags.len() <= limit;
        let baked = Baked {
            tolerance: if presets.layered > 0 { tol } else { 0.0 },
            zones_in: p.pbag.len().saturating_sub(1),
            zones_out: presets.bags.len(),
            generators_out: presets.generators.len(),
            layered: presets.layered,
            split: insts.split,
        };
        if fits || !velocity_tone {
            return Ok((p.write(&presets, &insts), baked));
        }
        last = Some(baked);
    }
    // Not even the coarsest layers fit: the tone as upstream plays it.
    let presets = p.presets(&insts, None)?;
    let baked = Baked { tolerance: 0.0, zones_out: presets.bags.len(), generators_out: presets.generators.len(), layered: 0, ..last.unwrap_or_default() };
    Ok((p.write(&presets, &insts), baked))
}

/// A modulator record (SF2 2.01 sections 7.4 and 7.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Modulator {
    src: u16,
    dest: u16,
    amount: i16,
    amount_src: u16,
    transform: u16,
}

impl Modulator {
    /// The SF2 2.01 default "note-on velocity to initial filter cutoff" (section 8.4.2):
    /// velocity, unipolar negative concave (0x0502), -2400 cents.
    const DEFAULT: Modulator = Modulator { src: 0x0502, dest: INITIAL_FILTER_FC, amount: -2400, amount_src: 0, transform: 0 };
    /// The default switched off (amount 0), written on every instrument zone.
    pub(super) const DEFAULT_OFF: Modulator = Modulator { amount: 0, ..Modulator::DEFAULT };

    fn from(r: &[u8]) -> Modulator {
        let u = |i: usize| u16::from_le_bytes([r[i], r[i + 1]]);
        Modulator { src: u(0), dest: u(2), amount: u(4) as i16, amount_src: u(6), transform: u(8) }
    }

    pub(super) fn bytes(&self) -> [u8; 10] {
        let mut b = [0u8; 10];
        for (i, v) in [self.src, self.dest, self.amount as u16, self.amount_src, self.transform].into_iter().enumerate() {
            b[2 * i..2 * i + 2].copy_from_slice(&v.to_le_bytes());
        }
        b
    }

    fn is_velocity(src: u16) -> bool {
        src & 0x80 == 0 && src & 0x7F == 2 && (src >> 10) <= 3
    }

    /// A velocity -> initial filter cutoff modulator: its source note-on velocity, its
    /// amount source none or velocity, its transform linear or absolute value.
    fn is_velocity_to_filter(&self) -> bool {
        self.dest == INITIAL_FILTER_FC
            && Modulator::is_velocity(self.src)
            && (self.amount_src == 0 || Modulator::is_velocity(self.amount_src))
            && (self.transform == 0 || self.transform == 2)
    }

    /// Whether `other` stands for the same modulator, so it replaces this one (see the
    /// module docs).
    fn same_as(&self, other: &Modulator) -> bool {
        let default_like = |m: &Modulator| {
            m.dest == INITIAL_FILTER_FC && (m.src == 0x0502 || m.src == 0x0102) && (m.amount_src == 0 || m.amount_src == 0x0D02 || m.amount_src == 0x0C02) && m.transform == 0
        };
        (default_like(self) && default_like(other))
            || (self.src == other.src && self.dest == other.dest && self.amount_src == other.amount_src && self.transform == other.transform)
    }

    /// A zone's velocity -> cutoff modulators into `list`: each replaces the same one
    /// there, or is added.
    fn merge(list: &mut Vec<Modulator>, zone: &[Modulator]) {
        for m in zone.iter().filter(|m| m.is_velocity_to_filter()) {
            match list.iter_mut().find(|l| l.same_as(m)) {
                Some(l) => *l = *m,
                None => list.push(*m),
            }
        }
    }

    /// A source's value for a note-on velocity (SF2 section 8.2): 1 for no source.
    fn source(src: u16, velocity: i32) -> f32 {
        if src == 0 {
            return 1_f32;
        }
        let mut x = velocity.clamp(0, 127) as f32 / 127_f32;
        if src & 0x0100 != 0 {
            x = 1_f32 - x;
        }
        let kind = src >> 10;
        let curve = |x: f32| match kind {
            1 => Modulator::concave(x),
            2 => 1_f32 - Modulator::concave(1_f32 - x),
            3 => {
                if x >= 0.5_f32 {
                    1_f32
                } else {
                    0_f32
                }
            }
            _ => x,
        };
        if src & 0x0200 == 0 {
            curve(x)
        } else if kind == 3 {
            if x >= 0.5_f32 { 1_f32 } else { -1_f32 }
        } else {
            // Bipolar: the curve on each half, mirrored about the middle.
            let s = 2_f32 * x - 1_f32;
            if s >= 0_f32 { curve(s) } else { -curve(-s) }
        }
    }

    fn concave(x: f32) -> f32 {
        if x >= 1_f32 {
            return 1_f32;
        }
        (-(40_f32 / 96_f32) * (1_f32 - x).log10()).clamp(0_f32, 1_f32)
    }

    /// Its output in cents for a note-on velocity.
    fn cents(&self, velocity: i32) -> f32 {
        let v = self.amount as f32 * Modulator::source(self.src, velocity) * Modulator::source(self.amount_src, velocity);
        if self.transform == 2 { v.abs() } else { v }
    }
}

/// A font's preset data as records (terminal records included).
pub(super) struct Pdta {
    pub(super) phdr: Vec<[u8; 38]>,
    /// (generator index, modulator index) per zone.
    pub(super) pbag: Vec<[u16; 2]>,
    pub(super) pmod: Vec<Modulator>,
    /// (generator, amount) records.
    pub(super) pgen: Vec<[u16; 2]>,
    pub(super) inst: Vec<[u8; 22]>,
    pub(super) ibag: Vec<[u16; 2]>,
    pub(super) imod: Vec<Modulator>,
    pub(super) igen: Vec<[u16; 2]>,
    pub(super) shdr: Vec<u8>,
}

/// The instruments as written: the font's, in order, then those split off them.
struct Instruments {
    /// Each output instrument's name (from `inst`) and zones (`ibag` indices).
    out: Vec<([u8; 20], Vec<usize>)>,
    /// For each of the font's instruments: the output instruments it plays as, each with
    /// its zones' velocity -> cutoff modulators.
    groups: Vec<Vec<(u16, Vec<Modulator>)>>,
    split: usize,
}

/// The preset zones as written.
struct Presets {
    /// Each preset's first zone (`bags` index).
    starts: Vec<usize>,
    /// Each zone's first generator (`generators` index).
    bags: Vec<usize>,
    generators: Vec<[u16; 2]>,
    layered: usize,
}

fn records<const N: usize>(body: &[u8]) -> Vec<[u8; N]> {
    body.chunks_exact(N).map(|c| c.try_into().expect("N bytes")).collect()
}

fn pairs(body: &[u8]) -> Vec<[u16; 2]> {
    body.chunks_exact(4).map(|c| [u16::from_le_bytes([c[0], c[1]]), u16::from_le_bytes([c[2], c[3]])]).collect()
}

impl Pdta {
    pub(super) fn parse(data: &[u8]) -> Result<Pdta> {
        if data.get(..4) != Some(b"pdta") {
            bail!("not preset data");
        }
        let mut p = Pdta { phdr: Vec::new(), pbag: Vec::new(), pmod: Vec::new(), pgen: Vec::new(), inst: Vec::new(), ibag: Vec::new(), imod: Vec::new(), igen: Vec::new(), shdr: Vec::new() };
        let mut i = 4;
        while i + 8 <= data.len() {
            let id = &data[i..i + 4];
            let size = u32::from_le_bytes([data[i + 4], data[i + 5], data[i + 6], data[i + 7]]) as usize;
            let body = data.get(i + 8..i + 8 + size).context("truncated preset data")?;
            // A modulator list of a size no records fill reads as none (as the vendored
            // rustysynth read it).
            let mods = |b: &[u8]| if b.len() % 10 == 0 { b.chunks_exact(10).map(Modulator::from).collect() } else { Vec::new() };
            match id {
                b"phdr" => p.phdr = records(body),
                b"pbag" => p.pbag = pairs(body),
                b"pmod" => p.pmod = mods(body),
                b"pgen" => p.pgen = pairs(body),
                b"inst" => p.inst = records(body),
                b"ibag" => p.ibag = pairs(body),
                b"imod" => p.imod = mods(body),
                b"igen" => p.igen = pairs(body),
                b"shdr" => p.shdr = body.to_vec(),
                _ => bail!("unknown preset data chunk {:?}", String::from_utf8_lossy(id)),
            }
            i += 8 + size + (size & 1);
        }
        if p.phdr.len() < 2 || p.pbag.len() < 2 || p.pgen.is_empty() || p.inst.len() < 2 || p.ibag.len() < 2 || p.igen.is_empty() || p.shdr.is_empty() {
            bail!("incomplete preset data");
        }
        Ok(p)
    }

    /// Zone `z`'s generators (`gens` without its terminal record) and modulators, as
    /// rustysynth reads them.
    pub(super) fn zone<'a>(bags: &[[u16; 2]], gens: &'a [[u16; 2]], mods: &'a [Modulator], z: usize) -> Result<(&'a [[u16; 2]], &'a [Modulator])> {
        let (g0, g1) = (bags[z][0] as usize, bags[z + 1][0] as usize);
        let g = gens.get(g0..g1).filter(|_| g1 < gens.len()).context("zone generators out of range")?;
        let (m0, m1) = (bags[z][1] as usize, bags[z + 1][1] as usize);
        Ok((g, mods.get(m0..m1.max(m0)).unwrap_or(&[])))
    }

    /// Record `i`'s first zone (the `u16` at byte `at` of a header).
    pub(super) fn first_zone<const N: usize>(headers: &[[u8; N]], i: usize, at: usize) -> usize {
        u16::from_le_bytes([headers[i][at], headers[i][at + 1]]) as usize
    }

    /// The instruments, each split by its zones' velocity -> cutoff modulators.
    fn instruments(&self) -> Result<Instruments> {
        let n = self.inst.len() - 1;
        let mut out: Vec<([u8; 20], Vec<usize>)> = Vec::with_capacity(n);
        let mut extra = Vec::new();
        let mut groups = Vec::with_capacity(n);
        let mut split = 0;
        for i in 0..n {
            let (z0, z1) = (Pdta::first_zone(&self.inst, i, 20), Pdta::first_zone(&self.inst, i + 1, 20));
            if z0 > z1 || z1 >= self.ibag.len() {
                bail!("instrument {i}: zones out of range");
            }
            let name: [u8; 20] = self.inst[i][..20].try_into().expect("20 bytes");
            let zone = |z: usize| Pdta::zone(&self.ibag, &self.igen, &self.imod, z);
            // rustysynth: the first zone is the global one unless it ends with a sample.
            let global = if z0 < z1 { zone(z0)?.0.last().is_none_or(|g| g[0] != SAMPLE_ID) } else { false };
            let mut base = vec![Modulator::DEFAULT];
            if global {
                Modulator::merge(&mut base, zone(z0)?.1);
            }
            let mut sets: Vec<(Vec<Modulator>, Vec<usize>)> = Vec::new();
            for z in z0 + global as usize..z1 {
                let mut list = base.clone();
                Modulator::merge(&mut list, zone(z)?.1);
                match sets.iter_mut().find(|s| s.0 == list) {
                    Some(s) => s.1.push(z),
                    None => sets.push((list, vec![z])),
                }
            }
            let head: Vec<usize> = if global { vec![z0] } else { Vec::new() };
            if sets.len() <= 1 {
                out.push((name, (z0..z1).collect()));
                groups.push(vec![(i as u16, sets.pop().map_or(base, |s| s.0))]);
                continue;
            }
            split += 1;
            let mut g = Vec::with_capacity(sets.len());
            for (k, (list, zones)) in sets.into_iter().enumerate() {
                let all: Vec<usize> = head.iter().copied().chain(zones).collect();
                if k == 0 {
                    out.push((name, all));
                    g.push((i as u16, list));
                } else {
                    g.push(((n + extra.len()) as u16, list));
                    extra.push((name, all));
                }
            }
            groups.push(g);
        }
        if n + extra.len() > MAX_RECORDS {
            bail!("too many instruments");
        }
        out.extend(extra);
        Ok(Instruments { out, groups, split })
    }

    /// The preset zones, each layered by velocity within `tolerance` cents (None: none
    /// layered).
    fn presets(&self, insts: &Instruments, tolerance: Option<f32>) -> Result<Presets> {
        let n = self.phdr.len() - 1;
        let mut p = Presets { starts: Vec::with_capacity(n), bags: Vec::with_capacity(self.pbag.len()), generators: Vec::with_capacity(self.pgen.len()), layered: 0 };
        let mut curve = [0f32; 128];
        let mut layers = Vec::with_capacity(128);
        for i in 0..n {
            p.starts.push(p.bags.len());
            let (z0, z1) = (Pdta::first_zone(&self.phdr, i, 24), Pdta::first_zone(&self.phdr, i + 1, 24));
            if z0 > z1 || z1 >= self.pbag.len() {
                bail!("preset {i}: zones out of range");
            }
            let zone = |z: usize| Pdta::zone(&self.pbag, &self.pgen, &self.pmod, z);
            // rustysynth: the first zone is the global one unless it ends with an instrument.
            let global = if z0 < z1 { zone(z0)?.0.last().is_none_or(|g| g[0] != INSTRUMENT) } else { false };
            let (global_gens, global_mods) = if global { zone(z0)? } else { (&[][..], &[][..]) };
            if global {
                p.bags.push(p.generators.len());
                p.generators.extend_from_slice(global_gens);
            }
            for z in z0 + global as usize..z1 {
                let (gens, mods) = zone(z)?;
                // The zone's own generators as rustysynth sets them (the last of a kind
                // wins; unknown kinds are ignored), and those it takes from the global zone.
                let mut own: Vec<[u16; 2]> = Vec::with_capacity(gens.len() + 2);
                for g in gens.iter().filter(|g| g[0] < GENERATORS) {
                    match own.iter_mut().find(|o| o[0] == g[0]) {
                        Some(o) => o[1] = g[1],
                        None => own.push(*g),
                    }
                }
                let value = |kind: u16, default: u16| {
                    own.iter().chain(global_gens.iter().rev().filter(|g| g[0] < GENERATORS)).find(|g| g[0] == kind).map_or(default, |g| g[1])
                };
                let instrument = value(INSTRUMENT, 0) as usize;
                let vel = value(VEL_RANGE, FULL_RANGE);
                let fc = value(INITIAL_FILTER_FC, 0) as i16 as i32;
                let group = insts.groups.get(instrument).with_context(|| format!("preset {i}: no instrument {instrument}"))?;
                let mut preset_mods = Vec::new();
                Modulator::merge(&mut preset_mods, global_mods);
                Modulator::merge(&mut preset_mods, mods);
                let (lo, hi) = ((vel & 0xFF) as u8, (vel >> 8) as u8);
                let rest = own.iter().filter(|g| ![KEY_RANGE, VEL_RANGE, INITIAL_FILTER_FC, INSTRUMENT].contains(&g[0]));
                let key = own.iter().find(|g| g[0] == KEY_RANGE);
                for (to, inst_mods) in group {
                    layers.clear();
                    if let Some(tol) = tolerance
                        && lo <= hi
                        && hi >= 1
                    {
                        for (v, c) in curve.iter_mut().enumerate().skip(1) {
                            *c = inst_mods.iter().chain(&preset_mods).map(|m| m.cents(v as i32)).sum();
                        }
                        velocity_layers(&curve, lo.max(1), hi.min(127), tol, &mut layers);
                    }
                    if layers.len() <= 1 && layers.first().is_none_or(|l| l.2 == 0) {
                        // Flat: the zone as it is, on its instrument (or the part split off).
                        p.bags.push(p.generators.len());
                        p.generators.extend(own.iter().filter(|g| g[0] != INSTRUMENT));
                        p.generators.push([INSTRUMENT, *to]);
                        continue;
                    }
                    p.layered += 1;
                    let bottom = layers.last().map_or(lo, |l| l.0);
                    for &(a, b, cents) in &layers {
                        let a = if a == bottom { lo } else { a };
                        p.bags.push(p.generators.len());
                        p.generators.extend(key);
                        p.generators.push([VEL_RANGE, a as u16 | (b as u16) << 8]);
                        p.generators.extend(rest.clone());
                        p.generators.push([INITIAL_FILTER_FC, (fc + cents).clamp(i16::MIN as i32, i16::MAX as i32) as i16 as u16]);
                        p.generators.push([INSTRUMENT, *to]);
                    }
                }
            }
        }
        Ok(p)
    }

    /// The preset data with `presets` and `insts`: the samples' headers as they were, every
    /// instrument zone with the default modulator switched off, no preset modulators.
    fn write(&self, presets: &Presets, insts: &Instruments) -> Vec<u8> {
        fn chunk(out: &mut Vec<u8>, id: &[u8; 4], body: &[u8]) {
            out.extend(id);
            out.extend((body.len() as u32).to_le_bytes());
            out.extend(body);
        }
        let u16s = |v: &mut Vec<u8>, xs: &[u16]| xs.iter().for_each(|x| v.extend(x.to_le_bytes()));
        let mut out = b"pdta".to_vec();

        let mut phdr = Vec::with_capacity(38 * self.phdr.len());
        for (i, h) in self.phdr.iter().enumerate() {
            let start = presets.starts.get(i).copied().unwrap_or(presets.bags.len());
            phdr.extend(&h[..24]);
            phdr.extend((start as u16).to_le_bytes());
            phdr.extend(&h[26..]);
        }
        chunk(&mut out, b"phdr", &phdr);
        let mut pbag = Vec::with_capacity(4 * presets.bags.len() + 4);
        for &g in presets.bags.iter().chain(std::iter::once(&presets.generators.len())) {
            u16s(&mut pbag, &[g as u16, 0]);
        }
        chunk(&mut out, b"pbag", &pbag);
        chunk(&mut out, b"pmod", &[0u8; 10]);
        let mut pgen = Vec::with_capacity(4 * presets.generators.len() + 4);
        for g in presets.generators.iter().chain(std::iter::once(&[0, 0])) {
            u16s(&mut pgen, g);
        }
        chunk(&mut out, b"pgen", &pgen);

        let (mut inst, mut ibag, mut imod, mut igen) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let (mut zones, mut gens) = (0u16, 0usize);
        for (name, zs) in &insts.out {
            inst.extend(name);
            inst.extend(zones.to_le_bytes());
            for &z in zs {
                u16s(&mut ibag, &[gens as u16, zones]);
                imod.extend(Modulator::DEFAULT_OFF.bytes());
                let (g0, g1) = (self.ibag[z][0] as usize, self.ibag[z + 1][0] as usize);
                for g in &self.igen[g0..g1] {
                    u16s(&mut igen, g);
                }
                gens += g1 - g0;
                zones += 1;
            }
        }
        inst.extend(&self.inst[self.inst.len() - 1][..20]);
        inst.extend(zones.to_le_bytes());
        u16s(&mut ibag, &[gens as u16, zones]);
        imod.extend([0u8; 10]);
        igen.extend([0u8; 4]);
        chunk(&mut out, b"inst", &inst);
        chunk(&mut out, b"ibag", &ibag);
        chunk(&mut out, b"imod", &imod);
        chunk(&mut out, b"igen", &igen);
        chunk(&mut out, b"shdr", &self.shdr);
        out
    }
}

/// `curve`'s velocity layers over `lo..=hi`, top down, as (lowest, highest velocity,
/// cents). The top layer takes the curve's value at `hi`; each other the middle of its
/// values; every velocity is within `tol` cents of its layer's value (before rounding).
fn velocity_layers(curve: &[f32; 128], lo: u8, hi: u8, tol: f32, out: &mut Vec<(u8, u8, i32)>) {
    let (lo, mut v) = (lo as usize, hi as usize);
    while v >= lo {
        let top = v;
        let (mut min, mut max) = (curve[top], curve[top]);
        while v > lo {
            let c = curve[v - 1];
            let (a, b) = (min.min(c), max.max(c));
            let fits = if out.is_empty() { (c - curve[top]).abs() <= tol } else { b - a <= 2_f32 * tol };
            if !fits {
                break;
            }
            (min, max) = (a, b);
            v -= 1;
        }
        let cents = if out.is_empty() { curve[top] } else { (min + max) / 2_f32 };
        out.push((v as u8, top as u8, cents.round() as i32));
        if v == lo {
            break;
        }
        v -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The default curve over 1..=127.
    fn default_curve() -> [f32; 128] {
        let mut c = [0f32; 128];
        for (v, x) in c.iter_mut().enumerate().skip(1) {
            *x = Modulator::DEFAULT.cents(v as i32);
        }
        c
    }

    /// The layers cover the range once, top down; each velocity is within the tolerance;
    /// the top velocity is exact.
    #[test]
    fn layers_cover_the_range_within_the_tolerance() {
        let c = default_curve();
        assert_eq!(c[127], 0.0);
        assert!((c[64] + 298.0).abs() < 1.0 && (c[32] + 598.0).abs() < 1.0, "{} {}", c[64], c[32]);
        let mut out = Vec::new();
        for (lo, hi, tol) in [(1, 127, 0.5), (1, 127, 10.0), (1, 127, 100.0), (20, 90, 25.0), (64, 64, 5.0)] {
            velocity_layers(&c, lo, hi, tol, &mut out);
            assert_eq!(out[0].1, hi);
            assert_eq!(out.last().unwrap().0, lo);
            assert_eq!(out[0].2, c[hi as usize].round() as i32, "the top velocity exact");
            for w in out.windows(2) {
                assert_eq!(w[1].1 + 1, w[0].0, "contiguous: {out:?}");
            }
            for &(a, b, cents) in &out {
                for v in a..=b {
                    assert!((c[v as usize] - cents as f32).abs() <= tol + 0.5, "v{v}: {} vs {cents} (tol {tol})", c[v as usize]);
                }
            }
            out.clear();
        }
        velocity_layers(&c, 1, 127, 0.5, &mut out);
        assert_eq!(out.len(), 127, "every velocity its own at half a cent");
    }

    /// A zone's own modulator replaces the default (the switch-off form), a preset's adds.
    #[test]
    fn modulators_replace_and_add_as_the_vendored_rustysynth_played_them() {
        let off = Modulator { src: 0x0102, dest: 8, amount: 0, amount_src: 0x0D02, transform: 0 };
        let mut list = vec![Modulator::DEFAULT];
        Modulator::merge(&mut list, &[off]);
        assert_eq!(list, [off]);
        let own = Modulator { src: 0x0102, dest: 8, amount: -3600, amount_src: 0, transform: 0 };
        let mut list = vec![Modulator::DEFAULT];
        Modulator::merge(&mut list, &[Modulator { src: 0x0502, dest: 48, amount: 800, amount_src: 0, transform: 0 }]);
        assert_eq!(list, [Modulator::DEFAULT], "other destinations ignored");
        let mut preset = Vec::new();
        Modulator::merge(&mut preset, &[own]);
        assert_eq!(preset, [own]);
        assert_eq!(own.cents(127), 0.0);
        assert!((own.cents(0) + 3600.0).abs() < 1e-3);
    }

    /// Preset data with one preset on one instrument, whose global zone sets a cutoff and
    /// whose two zones (keys 0-63 and 64-127) differ: the first switches the default off.
    fn split_pdta() -> Vec<u8> {
        let mut out = b"pdta".to_vec();
        let mut chunk = |id: &[u8; 4], body: Vec<u8>| {
            out.extend(id);
            out.extend((body.len() as u32).to_le_bytes());
            out.extend(body);
        };
        let u16s = |xs: &[u16]| xs.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
        let header = |name: &str, tail: &[u16], pad: usize| {
            let mut v = name.as_bytes().to_vec();
            v.resize(20, 0);
            v.extend(u16s(tail));
            v.extend(vec![0u8; pad]);
            v
        };
        chunk(b"phdr", [header("P", &[0, 0, 0], 12), header("EOP", &[0, 0, 1], 12)].concat());
        chunk(b"pbag", u16s(&[0, 0, 1, 0]));
        chunk(b"pmod", vec![0; 10]);
        chunk(b"pgen", u16s(&[INSTRUMENT, 0, 0, 0]));
        chunk(b"inst", [header("I", &[0], 0), header("EOI", &[3], 0)].concat());
        chunk(b"ibag", u16s(&[0, 0, 1, 0, 3, 1, 5, 1]));
        let off = Modulator { src: 0x0102, dest: 8, amount: 0, amount_src: 0x0D02, transform: 0 };
        chunk(b"imod", [off.bytes(), [0; 10]].concat());
        chunk(b"igen", u16s(&[INITIAL_FILTER_FC, 9000, KEY_RANGE, 0x3F00, SAMPLE_ID, 0, KEY_RANGE, 0x7F40, SAMPLE_ID, 0, 0, 0]));
        chunk(b"shdr", vec![0; 92]);
        out
    }

    /// An instrument whose zones differ is split: the part that switches the default off
    /// keeps the instrument and its preset zone as it was; the other part becomes a new
    /// instrument (with the global zone again) that a layer per velocity plays, the top
    /// one at the cutoff the zone had. Every instrument zone switches the default off.
    #[test]
    fn an_instrument_whose_zones_differ_is_split() {
        let (out, baked) = bake(&split_pdta(), true).unwrap();
        assert_eq!((baked.split, baked.layered, baked.tolerance), (1, 1, 0.5));
        let p = Pdta::parse(&out).unwrap();
        assert_eq!(p.inst.len(), 3, "the instrument, the part split off, the terminal");
        let zone_gens = |z: usize| p.igen[p.ibag[z][0] as usize..p.ibag[z + 1][0] as usize].to_vec();
        assert_eq!((Pdta::first_zone(&p.inst, 1, 20), Pdta::first_zone(&p.inst, 2, 20)), (2, 4));
        assert_eq!(zone_gens(0), [[INITIAL_FILTER_FC, 9000]]);
        assert_eq!(zone_gens(1), [[KEY_RANGE, 0x3F00], [SAMPLE_ID, 0]]);
        assert_eq!(zone_gens(2), [[INITIAL_FILTER_FC, 9000]], "the global zone again");
        assert_eq!(zone_gens(3), [[KEY_RANGE, 0x7F40], [SAMPLE_ID, 0]]);
        assert_eq!(p.imod, [Modulator::DEFAULT_OFF; 4].into_iter().chain([Modulator::from(&[0; 10])]).collect::<Vec<_>>());
        assert!(p.ibag.iter().enumerate().all(|(i, b)| b[1] as usize == i));
        // The preset: its zone on the switched-off part, then 127 layers on the other.
        let zones: Vec<Vec<[u16; 2]>> = (0..p.pbag.len() - 1).map(|z| p.pgen[p.pbag[z][0] as usize..p.pbag[z + 1][0] as usize].to_vec()).collect();
        assert_eq!(zones.len(), 1 + 127);
        assert_eq!(zones[0], [[INSTRUMENT, 0]]);
        assert_eq!(zones[1], [[VEL_RANGE, 127 | 127 << 8], [INITIAL_FILTER_FC, 0], [INSTRUMENT, 1]], "velocity 127 as it was");
        let v64 = &zones[1 + 127 - 64];
        assert_eq!(v64[0], [VEL_RANGE, 64 | 64 << 8]);
        assert_eq!(v64[1], [INITIAL_FILTER_FC, (Modulator::DEFAULT.cents(64).round() as i16) as u16]);
        assert_eq!(zones[127][0], [VEL_RANGE, 1 << 8], "the lowest layer from velocity 0");
        // Without the tone: nothing layered.
        let (out, baked) = bake(&split_pdta(), false).unwrap();
        assert_eq!((baked.layered, baked.tolerance), (0, 0.0));
        let p = Pdta::parse(&out).unwrap();
        assert_eq!(p.pgen[..p.pgen.len() - 1], [[INSTRUMENT, 0], [INSTRUMENT, 1]]);
    }

    /// Instrument zones or generators over the limit: the preset data as it was (no
    /// wrapped indices), nothing layered or split, and no panic.
    #[test]
    fn instruments_over_the_limit_fall_back_to_the_font_as_it_was() {
        let pdta = split_pdta();
        // The split writes 4 instrument zones and 6 generators; the presets fit in 128.
        let (out, baked) = bake_within(&pdta, true, 5).unwrap();
        assert_eq!(out, pdta);
        assert_eq!((baked.layered, baked.split, baked.tolerance), (0, 0, 0.0));
        assert!(Pdta::parse(&out).is_ok());
        let (_, baked) = bake_within(&pdta, true, 128).unwrap();
        assert_eq!(baked.split, 1, "within the limit it still splits");
    }

    /// The SoundFonts in `soundfonts/` (git-ignored; skipped without them): each bakes
    /// within the format's limits and loads with the same presets. Prints the tolerance
    /// used, the zones and the load time with and without baking (the PR's measurements).
    #[test]
    fn real_fonts_bake_within_the_limits() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("soundfonts");
        let Ok(entries) = std::fs::read_dir(&dir) else {
            eprintln!("no soundfonts; skipping");
            return;
        };
        let mut fonts: Vec<_> = entries.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "sf2")).collect();
        fonts.sort();
        for path in fonts {
            let mut f = BufReader::new(std::fs::File::open(&path).unwrap());
            let (_, pdta) = find_pdta(&mut f).unwrap();
            let t = std::time::Instant::now();
            let (out, baked) = bake(&pdta, true).unwrap();
            let bake_ms = t.elapsed().as_secs_f64() * 1e3;
            assert!(baked.generators_out <= MAX_RECORDS && baked.zones_out <= MAX_RECORDS, "{baked:?}");
            f.seek(SeekFrom::Start(0)).unwrap();
            let t = std::time::Instant::now();
            let plain = SoundFont::new(&mut f).unwrap();
            let plain_ms = t.elapsed().as_secs_f64() * 1e3;
            f.seek(SeekFrom::Start(0)).unwrap();
            let t = std::time::Instant::now();
            let font = read_as(&mut f, true).unwrap();
            let load_ms = t.elapsed().as_secs_f64() * 1e3;
            let regions = |f: &SoundFont| f.get_presets().iter().map(|p| p.get_regions().len()).sum::<usize>();
            let names = |f: &SoundFont| f.get_presets().iter().map(|p| (p.get_bank_number(), p.get_patch_number(), p.get_name().to_string())).collect::<Vec<_>>();
            assert_eq!(names(&font), names(&plain), "{}", path.display());
            eprintln!(
                "{}: tolerance {} cents, {} of {} preset zones layered, {} -> {} preset regions ({} generators), {} instruments split, pdta {} -> {} KB; bake {bake_ms:.1} ms, load {load_ms:.0} ms vs {plain_ms:.0} ms plain",
                path.file_name().unwrap().to_string_lossy(),
                baked.tolerance,
                baked.layered,
                baked.zones_in,
                regions(&plain),
                regions(&font),
                baked.generators_out,
                baked.split,
                pdta.len() / 1024,
                out.len() / 1024,
            );
        }
    }
}
