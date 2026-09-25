use crate::error::SoundFontError;
use crate::generator::Generator;
use crate::modulator::Modulator;
use crate::zone_info::ZoneInfo;

#[non_exhaustive]
pub(crate) struct Zone {
    pub(crate) generators: Vec<Generator>,
    // yahaha: the zone's modulators (only velocity -> filter is played).
    pub(crate) modulators: Vec<Modulator>,
}

impl Zone {
    pub(crate) fn empty() -> Self {
        Self {
            generators: Vec::new(),
            modulators: Vec::new(),
        }
    }

    fn new(info: &ZoneInfo, generators: &[Generator], modulators: &[Modulator]) -> Self {
        let mut segment: Vec<Generator> = Vec::new();

        for i in 0..info.generator_count {
            segment.push(generators[(info.generator_index + i) as usize]);
        }

        // yahaha: a zone's modulators, or none if the list doesn't hold them all.
        let start = info.modulator_index.max(0) as usize;
        let end = start + info.modulator_count.max(0) as usize;
        let modulators = modulators.get(start..end).unwrap_or(&[]).to_vec();

        Self {
            generators: segment,
            modulators,
        }
    }

    pub(crate) fn create(
        infos: &[ZoneInfo],
        generators: &[Generator],
        modulators: &[Modulator],
    ) -> Result<Vec<Zone>, SoundFontError> {
        if infos.len() <= 1 {
            return Err(SoundFontError::ZoneNotFound);
        }

        // The last one is the terminator.
        let count = infos.len() - 1;

        let mut zones: Vec<Zone> = Vec::new();
        for info in infos.iter().take(count) {
            zones.push(Zone::new(info, generators, modulators));
        }

        Ok(zones)
    }
}
