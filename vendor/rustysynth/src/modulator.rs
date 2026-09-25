// yahaha: this file is not upstream rustysynth's. See the Cargo.toml note in the yahaha repo.
//
// SF2 modulators, read from the pmod and imod chunks, for one destination only: the
// initial filter cutoff driven by note-on velocity (velocity -> tone). Upstream read no
// modulators. Everything else a SoundFont's modulators do is still ignored, as upstream.

use std::io::Read;

use crate::binary_reader::BinaryReader;
use crate::error::SoundFontError;

/// Generator number of initialFilterFc.
const DEST_INITIAL_FILTER_FC: u16 = 8;
/// Source index of note-on velocity (general controller, not a MIDI CC).
const SRC_VELOCITY: u16 = 2;

/// One modulator record (SF2 2.01 section 7.4 / 7.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) struct Modulator {
    pub(crate) src: u16,
    pub(crate) dest: u16,
    pub(crate) amount: i16,
    pub(crate) amount_src: u16,
    pub(crate) transform: u16,
}

impl Modulator {
    /// The SF2 2.01 default modulator "MIDI note-on velocity to initial filter cutoff"
    /// (section 8.4.2): note-on velocity, unipolar, negative, concave (0x0502), -2400 cents.
    /// Its curve of velocity v is -(40/96)*log10(v/127), clamped to 0..1 (the same curve as
    /// the default velocity -> attenuation, which `Voice::start`'s `note_gain` follows), so
    /// the cutoff drops by 1000*log10(v/127) cents: 0 at 127, about -300 at 64, -600 at 32.
    pub(crate) const DEFAULT_VELOCITY_TO_FILTER: Modulator = Modulator {
        src: 0x0502,
        dest: DEST_INITIAL_FILTER_FC,
        amount: -2400,
        amount_src: 0,
        transform: 0,
    };

    fn new<R: Read>(reader: &mut R) -> Result<Self, SoundFontError> {
        Ok(Self {
            src: BinaryReader::read_u16(reader)?,
            dest: BinaryReader::read_u16(reader)?,
            amount: BinaryReader::read_i16(reader)?,
            amount_src: BinaryReader::read_u16(reader)?,
            transform: BinaryReader::read_u16(reader)?,
        })
    }

    /// Every record of a pmod or imod chunk, the terminator included (zones index into it).
    /// A malformed chunk reads as empty: upstream skipped these chunks, so a SoundFont that
    /// loaded before must still load.
    pub(crate) fn read_from_chunk<R: Read>(
        reader: &mut R,
        size: usize,
    ) -> Result<Vec<Modulator>, SoundFontError> {
        if size % 10 != 0 {
            BinaryReader::discard_data(reader, size)?;
            return Ok(Vec::new());
        }
        let mut modulators = Vec::with_capacity(size / 10);
        for _ in 0..size / 10 {
            modulators.push(Modulator::new(reader)?);
        }
        Ok(modulators)
    }

    fn is_velocity(src: u16) -> bool {
        src & 0x80 == 0 && src & 0x7F == SRC_VELOCITY && (src >> 10) <= 3
    }

    /// A velocity -> initial filter cutoff modulator this synth can play: its source is
    /// note-on velocity, its amount source none or velocity, its transform linear or
    /// absolute value.
    pub(crate) fn is_velocity_to_filter(&self) -> bool {
        self.dest == DEST_INITIAL_FILTER_FC
            && Modulator::is_velocity(self.src)
            && (self.amount_src == 0 || Modulator::is_velocity(self.amount_src))
            && (self.transform == 0 || self.transform == 2)
    }

    /// Whether `other` stands for the same modulator (SF2 section 9.5: same source, amount
    /// source, destination and transform), so it replaces this one. The default velocity ->
    /// filter modulator is also replaced by a negative linear velocity source, with no
    /// amount source or a velocity switch: FluidSynth's and Polyphone's form of that
    /// default, which SoundFonts made with them use to switch it off (amount 0).
    fn same_as(&self, other: &Modulator) -> bool {
        let default_like = |m: &Modulator| {
            m.dest == DEST_INITIAL_FILTER_FC
                && (m.src == 0x0502 || m.src == 0x0102)
                && (m.amount_src == 0 || m.amount_src == 0x0D02 || m.amount_src == 0x0C02)
                && m.transform == 0
        };
        (default_like(self) && default_like(other))
            || (self.src == other.src
                && self.dest == other.dest
                && self.amount_src == other.amount_src
                && self.transform == other.transform)
    }

    /// Add a zone's velocity -> filter modulators to `list`: one that is the same as a
    /// modulator already there replaces it (a local zone's replaces the global zone's, and
    /// an instrument's replaces the default); any other is added.
    pub(crate) fn merge_velocity_to_filter(list: &mut Vec<Modulator>, zone: &[Modulator]) {
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
            if x >= 0.5_f32 {
                1_f32
            } else {
                -1_f32
            }
        } else {
            // Bipolar: the curve on each half, mirrored about the middle.
            let s = 2_f32 * x - 1_f32;
            if s >= 0_f32 {
                curve(s)
            } else {
                -curve(-s)
            }
        }
    }

    fn concave(x: f32) -> f32 {
        if x >= 1_f32 {
            return 1_f32;
        }
        (-(40_f32 / 96_f32) * (1_f32 - x).log10()).clamp(0_f32, 1_f32)
    }

    /// This modulator's output, in cents, for a note-on velocity. Pure arithmetic (it runs
    /// in `Voice::start`, on the audio thread).
    pub(crate) fn velocity_value(&self, velocity: i32) -> f32 {
        let v = self.amount as f32
            * Modulator::source(self.src, velocity)
            * Modulator::source(self.amount_src, velocity);
        if self.transform == 2 {
            v.abs()
        } else {
            v
        }
    }
}

