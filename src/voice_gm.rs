//! The GM program for a Genos voice on a Genos-only bank: MSB 8 (MegaVoice, S.Art!,
//! S.Art2!, #228), MSB 9 (the Ensemble parts' S.Art! and S.Art2! voices) and MSB 104
//! (#270).
//!
//! Yamaha's GM/XG banks (MSB 0) follow GM numbering. Bank 8 does not. The Genos Data List's
//! Voice List numbers it by instrument: NylonGuitar is PC# 1, SteelGuitar PC# 2,
//! CleanGuitar PC# 4, ElectricBass PC# 18, the MegaVoice pop choirs PC# 101-126. Played as
//! GM numbers, those are pianos, strings and sound effects.
//!
//! Every bank 8 voice with one program number is the same kind of instrument, whatever its
//! LSB: the S.Art! and S.Art2! variations (LSB 32 and up) share the MegaVoice's number.
//! So the table maps the program number alone. That also keeps it usable where only the
//! MSB is known (the synth and the program map follow bank MSB, not LSB).
//!
//! Decision: a few numbers hold two kinds of instrument. The one with more voices wins:
//! - PC# 21: the 70s electric pianos and Clavis (16 voices) over ActiveBassSlap. On the
//!   Bass part, the synth's bass rule still gives a GM bass.
//! - PC# 74: the flute over the trombone (one each).
//! - PC# 121: PopHooLegato2 over MagicBell (one each).
//!
//! **Bank 9** is the Voice List's "EnsemblePart" category: copies of bank 8's S.Art! and
//! S.Art2! voices (Seattle and Kino strings, the choirs, trumpets, saxes, woodwind) that
//! the Ensemble Voices are built from. They keep bank 8's program numbers, so bank 9's
//! table matches bank 8's row for row, except where bank 9 holds other voices on a number:
//! - PC# 49: only JazzViolin and CelticViolin (bank 8 adds its string sections): Violin.
//! - PC# 74: Trombone and TromboneShake over OrchestralFlute (two to one): Trombone.
//!
//! **Bank 104** (Live!, Cool!, S.Art! and Regular voices: the CFX and C7 grands, Suitcase
//! and DX pianos, theatre and pipe organs, accordions, orchestral brass, synths) already
//! follows GM numbering: each of its Data List voices sits in its number's GM family (CFX
//! ConcertGrand PC# 1, DX Sweet PC# 6, ActiveBassSlap PC# 37, OrchTrumpets PC# 57,
//! Applause PC# 127). Its program plays as it is.
//!
//! A bank 8 or 9 number the Data List does not use plays as before (the number as a GM
//! program).

/// The MegaVoice / S.Art! bank.
pub const MSB: u8 = 8;
/// The Ensemble parts' S.Art! / S.Art2! bank.
pub const ENSEMBLE_MSB: u8 = 9;
/// The Genos bank that follows GM numbering.
pub const GM_NUMBERED_MSB: u8 = 104;

/// (Data List PC#, 1-128; GM program, 0-127).
#[rustfmt::skip]
const PROGRAMS: [(u8, u8); 63] = [
    (1, 24),    // NylonGuitar, FlamencoGuitar, Spanish, ConcertGuitar: Nylon Guitar
    (2, 25),    // SteelGuitar, SteelAcoustic*, 12String*, D-Folk*, Resonator: Steel Guitar
    (3, 25),    // HiStringGuitar, 12StringGuitar: Steel Guitar
    (4, 27),    // CleanGuitar, Solid*, SingleCoil*, 50s/60sVintage, PedalSteel: Clean Guitar
    (5, 29),    // OverdriveGuitar, HeavyRock, Blues: Overdriven Guitar
    (6, 30),    // DistortionGuitar, GuitarHero, RockLegend: Distortion Guitar
    (7, 26),    // JazzGuitar, SemiAcoustic: Jazz Guitar
    (8, 27),    // 60sShadowLead, 60sBalladGuitar, 60sVintage*: Clean Guitar
    (13, 25),   // Mandolin: Steel Guitar (GM has no mandolin)
    (14, 24),   // UkleleThumbDown, Ukulele: Nylon Guitar
    (17, 32),   // AcousticBass: Acoustic Bass
    (18, 33),   // ElectricBass, VintageRound/Flat, ActiveBassFing*: Finger Bass
    (19, 34),   // PickBass, VintagePick, ActiveBassPick*: Pick Bass
    (20, 35),   // FretlessBass: Fretless Bass
    (21, 4),    // 70sSuitcase*, 70sVintageEP, Clavi (and ActiveBassSlap): Electric Piano 1
    (30, 16),   // WhiterBars, ProgRockOrgan, ClassicBars: Drawbar Organ
    (31, 16),   // AllBarsOut: Drawbar Organ
    (40, 52),   // Haa, Wah, Baa, Daa: Choir Aahs
    (41, 53),   // Ooh, Doo, Yoo, Ahh-OohAuto: Voice Oohs
    (44, 40),   // Seattle1stViolins, Orchestral1stViolin: Violin
    (45, 40),   // Seattle2ndViolins, Orchestral2ndViolin: Violin
    (46, 41),   // SeattleViolas, OrchestralViola: Viola
    (47, 42),   // SeattleCellos, OrchestralCello: Cello
    (48, 43),   // SeattleBasses, KinoStringsBasses: Contrabass
    (49, 48),   // SmallStrings, ClassicalStrings, Kino*, StudioStrings, JazzViolin: Strings
    (50, 49),   // LargeStrings, SeattleStrings, Kino*, ConcertStrings: Slow Strings
    (51, 42),   // ClassicalCello, PopCello: Cello
    (52, 52),   // MaleVoiceChoir, BoysChoir*: Choir Aahs
    (55, 52),   // GospelChoir, GospelVocals*: Choir Aahs
    (56, 53),   // Shoo-Bee-Doo-Bah, PopVocals, JazzScat*: Voice Oohs
    (57, 61),   // Brass, PopHorns1/2, BrassShake, BigBandBrass: Brass Section
    (61, 60),   // SoftOrchHorns, WarmOrchHorns, MutedHorns: French Horn
    (63, 56),   // Flugelhorn: Trumpet
    (65, 56),   // Trumpet, BrightTrumpet, MuteTrumpet: Trumpet
    (66, 56),   // SoftTrumpet, ClassicTrumpet, BigBandTrumpet: Trumpet
    (67, 65),   // CleanAltoSax, SoftAltoSax: Alto Sax
    (69, 68),   // PopOboe, ClassicalOboe: Oboe
    (71, 70),   // PopBassoon, ClassicalBassoon: Bassoon
    (74, 73),   // OrchestralFlute (and Trombone): Flute
    (75, 73),   // ClassicalFlute, JazzFlute: Flute
    (81, 66),   // BreathyTenorSax, SmoothTenorSax, TenorSax: Tenor Sax
    (82, 67),   // BaritoneSax, FunkBaritoneSax, BigBandBaritone: Baritone Sax
    (83, 66),   // TenorSax, Saxophone, RockSax, SmoothSaxes: Tenor Sax
    (84, 65),   // AltoSax, FunkAltoSax, BigBandAltoSax: Alto Sax
    (85, 64),   // PopSopranoSax, BalladSopranoSax: Soprano Sax
    (93, 71),   // Clarinet, BalladClarinet, RomanceClarinet: Clarinet
    (101, 52),  // PopHaa: Choir Aahs
    (102, 52),  // PopDaa: Choir Aahs
    (103, 52),  // PopBaa: Choir Aahs
    (104, 53),  // PopShoo: Voice Oohs
    (105, 22),  // Harmonica, BluesHarmonica: Harmonica
    (106, 53),  // PopHoo: Voice Oohs
    (107, 53),  // PopDoo: Voice Oohs
    (108, 53),  // PopBee: Voice Oohs
    (109, 109), // IrishPipesAir, IrishPipesDance: Bagpipe
    (111, 53),  // PopHee: Voice Oohs
    (113, 6),   // Harpsichord: Harpsichord
    (114, 16),  // JazzRotary, RockOrgan: Drawbar Organ
    (116, 52),  // PopHaaLegato2: Choir Aahs
    (121, 53),  // PopHooLegato2 (and MagicBell): Voice Oohs
    (123, 80),  // BPF, Wobble, RampBass, SquarePluck: Square Lead
    (124, 81),  // SquareStack, FourStack, NuLine: Saw Lead
    (126, 53),  // PopHeeLegato2: Voice Oohs
];

/// Bank 9 (EnsemblePart): (Data List PC#, 1-128; GM program, 0-127).
#[rustfmt::skip]
const ENSEMBLE: [(u8, u8); 28] = [
    (7, 26),    // SemiAcoustic, JazzArtistGuitar: Jazz Guitar
    (40, 52),   // Haa, BoysHaa, GirlsHaa, Wah, Baa, Daa: Choir Aahs
    (41, 53),   // Ooh, BoysOoh, GirlsOoh, Doo, BoysDoo, GirlsDoo, Yoo: Voice Oohs
    (44, 40),   // Seattle1stViolins, Orchestral1stViolin, Classic1stVns, KinoStringsViolins: Violin
    (45, 40),   // Seattle2ndViolins, Orchestral2ndViolin, Classic2ndVns: Violin
    (46, 41),   // SeattleViolas, OrchestralViola, ClassicViolas, KinoStringsViolas: Viola
    (47, 42),   // SeattleCellos, OrchestralCello, ClassicCellos, KinoStringsCellos: Cello
    (48, 43),   // SeattleC.Basses, KinoStringsBasses: Contrabass
    (49, 40),   // JazzViolin, CelticViolin: Violin (bank 8: Strings)
    (50, 49),   // KinoStrings, KinoStringsVnVc, 1Oct, 2Oct, Low, WarmVc, Slow: Slow Strings
    (51, 42),   // ClassicalCello, PopCello: Cello
    (52, 52),   // BoysChoirOoh, BoysChoirAah (p, f): Choir Aahs
    (63, 56),   // Flugelhorn: Trumpet
    (65, 56),   // Trumpet, BigBandTrumpet, MuteTrumpet: Trumpet
    (66, 56),   // SoftTrumpet, ClassicTrumpet, Trumpet1/2, Trumpet1/2Shake: Trumpet
    (67, 65),   // CleanAltoSax, SoftAltoSax: Alto Sax
    (69, 68),   // MOR Oboe, PopOboe, ClassicalOboe: Oboe
    (71, 70),   // PopBassoon, ClassicalBassoon: Bassoon
    (74, 57),   // Trombone, TromboneShake (and OrchestralFlute): Trombone (bank 8: Flute)
    (75, 73),   // ClassicalFlute, JazzFlute: Flute
    (81, 66),   // SmoothTenorSax, TenorSax, TenorSaxGrowl, TenorSaxShake: Tenor Sax
    (82, 67),   // FunkBaritoneSax, BaritoneSax, BigBandBaritone, BaritoneSaxGrowl: Baritone Sax
    (83, 66),   // Saxophone, SmoothSaxesOctave, BigBandTenorSax: Tenor Sax
    (84, 65),   // FunkAltoSax, AltoSax, BigBandAltoSax, AltoSaxGrowl: Alto Sax
    (85, 64),   // SopranoSax, BalladSopranoSax, SopranoSaxGrowl: Soprano Sax
    (93, 71),   // Clarinet: Clarinet
    (105, 22),  // Harmonica: Harmonica
    (109, 109), // IrishPipesAir: Bagpipe
];

/// The GM program (0-127) a voice on bank `msb` with program `program` (0-127, the Data
/// List's PC# - 1) plays as: banks 8 and 9 by the Data List's tables, bank 104 as it is.
/// None for another bank, or for a bank 8 or 9 number the Data List does not use.
pub fn gm_program(msb: u8, program: u8) -> Option<u8> {
    let table: &[(u8, u8)] = match msb {
        MSB => &PROGRAMS,
        ENSEMBLE_MSB => &ENSEMBLE,
        GM_NUMBERED_MSB => return (program < 128).then_some(program),
        _ => return None,
    };
    let pc = program.checked_add(1)?;
    table.iter().find(|p| p.0 == pc).map(|p| p.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_data_lists_voices_map_to_their_instrument() {
        // The #228 examples, as the Style sends them (0-based).
        assert_eq!(gm_program(8, 0), Some(24), "NylonGuitar: Nylon Guitar, not Acoustic Grand Piano");
        assert_eq!(gm_program(8, 1), Some(25), "SteelGuitar");
        assert_eq!(gm_program(8, 3), Some(27), "CleanGuitar");
        assert_eq!(gm_program(8, 6), Some(26), "JazzGuitar");
        assert_eq!(gm_program(8, 4), Some(29), "OverdriveGuitar");
        assert_eq!(gm_program(8, 5), Some(30), "DistortionGuitar");
        assert_eq!(gm_program(8, 16), Some(32), "AcousticBass");
        assert_eq!(gm_program(8, 17), Some(33), "ElectricBass");
        assert_eq!(gm_program(8, 19), Some(35), "FretlessBass");
        assert_eq!(gm_program(8, 48), Some(48), "SmallStrings");
        assert_eq!(gm_program(8, 100), Some(52), "PopHaa: Choir Aahs, not an FX program");
        assert_eq!(gm_program(8, 33), None, "not a bank 8 number");
        assert_eq!(gm_program(8, 127), None);
    }

    #[test]
    fn the_tables_are_well_formed() {
        for table in [&PROGRAMS[..], &ENSEMBLE[..]] {
            for (i, p) in table.iter().enumerate() {
                assert!(!table[..i].iter().any(|q| q.0 == p.0), "PC# {} twice", p.0);
                assert!((1..=128).contains(&p.0) && p.1 <= 127);
            }
        }
    }

    #[test]
    fn ensemble_voices_map_like_their_bank_8_twins() {
        // The #270 examples, 0-based: Seattle1stViolins (9/32/44) plays a violin, not
        // Pizzicato; TenorSax (9/66/81) a tenor sax, not a Saw Lead; Haa (9/32/40) a
        // choir, not a viola.
        assert_eq!(gm_program(9, 43), Some(40), "Seattle1stViolins");
        assert_eq!(gm_program(9, 80), Some(66), "TenorSax");
        assert_eq!(gm_program(9, 39), Some(52), "Haa");
        assert_eq!(gm_program(9, 64), Some(56), "Trumpet");
        assert_eq!(gm_program(9, 108), Some(109), "IrishPipesAir");
        // Every bank 9 number is a bank 8 number and maps the same way, except the two
        // where bank 9 holds other voices.
        for &(pc, gm) in &ENSEMBLE {
            match pc {
                49 => assert_eq!(gm, 40, "JazzViolin, CelticViolin"),
                74 => assert_eq!(gm, 57, "Trombone, TromboneShake"),
                _ => assert_eq!(gm_program(8, pc - 1), Some(gm), "PC# {pc}"),
            }
        }
        assert_eq!(gm_program(8, 48), Some(48), "bank 8 PC# 49 stays Strings");
        assert_eq!(gm_program(8, 73), Some(73), "bank 8 PC# 74 stays Flute");
        assert_eq!(gm_program(9, 0), None, "not a bank 9 number");
    }

    #[test]
    fn bank_104_is_gm_numbered() {
        // (0-based program, a Data List voice there): each is in its number's GM family.
        for (prog, voice) in [
            (0, "CFX ConcertGrand"),
            (5, "DX Sweet"),
            (17, "EuroAccomp1"),
            (21, "CajunAccordion"),
            (36, "ActiveBassSlap"),
            (56, "OrchTrumpets"),
            (60, "OrchestralHorns"),
            (73, "OrchestralFlute"),
            (126, "Applause1"),
        ] {
            assert_eq!(gm_program(104, prog), Some(prog), "{voice}");
        }
        assert_eq!(gm_program(0, 5), None, "the GM/XG banks are not mapped here");
        assert_eq!(gm_program(109, 5), None);
    }
}
