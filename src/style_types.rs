//! Style data types shared below the SFF parser: MIDI events, CASM channel rules and
//! section ids.
//!
//! These live here, not in `sff`, so that `theory` and `library` can use them without
//! importing the parser (the layering is core → sff → ...). `sff` re-exports every item,
//! so the old `crate::sff::...` paths keep working.

// ---------------------------------------------------------------------------
// MIDI events
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum Ev {
    NoteOn { ch: u8, key: u8, vel: u8 },
    NoteOff { ch: u8, key: u8 },
    PolyAt { ch: u8, key: u8, val: u8 },
    Cc { ch: u8, cc: u8, val: u8 },
    Pc { ch: u8, prog: u8 },
    ChanAt { ch: u8, val: u8 },
    Bend { ch: u8, val: u16 },
    /// Full sysex message including the leading F0.
    Sysex(Vec<u8>),
    Meta { ty: u8, data: Vec<u8> },
}

impl Ev {
    pub fn channel(&self) -> Option<u8> {
        match *self {
            Ev::NoteOn { ch, .. }
            | Ev::NoteOff { ch, .. }
            | Ev::PolyAt { ch, .. }
            | Ev::Cc { ch, .. }
            | Ev::Pc { ch, .. }
            | Ev::ChanAt { ch, .. }
            | Ev::Bend { ch, .. } => Some(ch),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// CASM
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ntr {
    RootTrans,
    RootFixed,
    Guitar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ntt {
    Bypass,
    Melody,
    Chord,
    /// Old SFF1 "Bass" table; equivalent to Melody with Bass On.
    Bass,
    MelodicMinor,
    MelodicMinor5,
    HarmonicMinor,
    HarmonicMinor5,
    NaturalMinor,
    NaturalMinor5,
    Dorian,
    Dorian5,
    GuitarAllPurpose,
    GuitarStroke,
    GuitarArpeggio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rtr {
    Stop,
    PitchShift,
    PitchShiftToRoot,
    Retrigger,
    RetriggerToRoot,
    NoteGenerator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Zone {
    pub ntr: Ntr,
    pub ntt: Ntt,
    pub high_key: u8,
    pub lo: u8,
    pub hi: u8,
    pub rtr: Rtr,
    /// NTT Bass On: this zone follows slash chords. SFF2 stores it per zone (bit 7 of
    /// each zone's NTT byte); SFF1 and Cntt set every zone alike.
    pub bass_on: bool,
}

/// Channel rule for one source channel within one or more sections (a Ctab/Ctb2 record).
#[allow(dead_code)] // editable/sff2 kept for dump/debugging
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChannelRule {
    pub src_ch: u8,
    pub name: String,
    /// Destination accompaniment channel, 8..=15 (MIDI channels 9..16).
    pub dest_ch: u8,
    pub editable: bool,
    /// Bit n set = chord root n (C=0) plays.
    pub note_mute: u16,
    /// Bit n set = chord type n (see `theory::ChordType` ids) plays.
    pub chord_mute: u64,
    pub autostart: bool,
    pub src_root: u8,
    pub src_type: u8,
    /// Notes below `mid_lo` use zones[0], above `mid_hi` use zones[2].
    pub mid_lo: u8,
    pub mid_hi: u8,
    pub zones: [Zone; 3],
    pub sff2: bool,
}

impl ChannelRule {
    pub fn zone_for(&self, key: u8) -> &Zone {
        if key < self.mid_lo {
            &self.zones[0]
        } else if key > self.mid_hi {
            &self.zones[2]
        } else {
            &self.zones[1]
        }
    }

    /// Rule used when a style has no CASM data for a channel: channels 9..16 play
    /// through with the conventional table for their part, source chord CMaj7.
    pub fn default_for(ch: u8) -> ChannelRule {
        let (ntr, ntt, bass) = match ch {
            8 | 9 => (Ntr::RootFixed, Ntt::Bypass, false),
            10 => (Ntr::RootTrans, Ntt::Melody, true),
            11 | 12 | 13 => (Ntr::RootFixed, Ntt::Chord, false),
            _ => (Ntr::RootTrans, Ntt::Melody, false),
        };
        let z = Zone { ntr, ntt, high_key: 6, lo: 0, hi: 127, rtr: Rtr::PitchShift, bass_on: bass };
        ChannelRule {
            src_ch: ch,
            name: String::new(),
            dest_ch: ch,
            editable: true,
            note_mute: 0x0FFF,
            chord_mute: (1u64 << 34) - 1,
            autostart: false,
            src_root: 0,
            src_type: 2,
            mid_lo: 0,
            mid_hi: 127,
            zones: [z; 3],
            sff2: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Sections
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SectionId {
    Intro(u8),  // 0..=3 (A..D)
    Main(u8),   // 0..=3
    Fill(u8),   // 0..=3 (AA..DD)
    Break,      // "Fill In BA"
    Ending(u8), // 0..=3
}

impl SectionId {
    pub fn parse(s: &str) -> Option<SectionId> {
        let letter = |c: &str| -> Option<u8> {
            match c {
                "A" => Some(0),
                "B" => Some(1),
                "C" => Some(2),
                "D" => Some(3),
                _ => None,
            }
        };
        let s = s.trim();
        if let Some(r) = s.strip_prefix("Intro ") {
            return letter(r).map(SectionId::Intro);
        }
        if let Some(r) = s.strip_prefix("Main ") {
            return letter(r).map(SectionId::Main);
        }
        if let Some(r) = s.strip_prefix("Ending ") {
            return letter(r).map(SectionId::Ending);
        }
        if let Some(r) = s.strip_prefix("Fill In ") {
            return match r {
                "AA" => Some(SectionId::Fill(0)),
                "BB" => Some(SectionId::Fill(1)),
                "CC" => Some(SectionId::Fill(2)),
                "DD" => Some(SectionId::Fill(3)),
                "BA" => Some(SectionId::Break),
                _ => None,
            };
        }
        None
    }

    pub fn name(&self) -> String {
        const L: [char; 4] = ['A', 'B', 'C', 'D'];
        match *self {
            SectionId::Intro(i) => format!("Intro {}", L[i as usize]),
            SectionId::Main(i) => format!("Main {}", L[i as usize]),
            SectionId::Fill(i) => format!("Fill In {0}{0}", L[i as usize]),
            SectionId::Break => "Fill In BA".into(),
            SectionId::Ending(i) => format!("Ending {}", L[i as usize]),
        }
    }
}
