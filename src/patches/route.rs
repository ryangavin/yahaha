//! The program map as the real-time threads read it: a table of atomics, written on the
//! control side whenever the map, the library or the style changes, and only read by the
//! audio thread (the built-in synth) and the engine thread (the `yahaha` port, when it
//! sends mapped programs). No allocation, no locks: one atomic load per lookup.
//!
//! There are two banks of the table. The style playing uses one; a style handed to the
//! engine gets the other, written before the hand-off, and the engine names its bank when
//! it takes the style over (`Sink::route_bank`, in the MIDI stream right after the new
//! style's setup), so the synth switches maps exactly where the styles switch.
//!
//! Each entry is a [`Route`] packed into a `u32` (0 = no route: the fallback): which
//! SoundFont (a font id the control side hands out, stable for the session), bank and
//! program.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering::{Acquire, Relaxed, Release}};

/// Font ids (and plugin slots) are 6 bits.
pub const MAX_FONTS: usize = 64;

/// Which sound source a route plays: a SoundFont (by font id), or a plugin slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// SoundFont `k` (a font id the control side hands out, stable for the session).
    SoundFont(u8),
    /// Plugin rack slot `j` (#91). Not played yet: the synth treats it as no route (the
    /// fallback) until plugin hosting reaches the channels.
    Plugin(u8),
}

/// A patch as a channel plays it: its source, and for a SoundFont the bank (128 = drum
/// kits) and program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Route {
    pub source: Source,
    pub bank: u16,
    pub program: u8,
}

const VALID: u32 = 1 << 31;
const PLUGIN: u32 = 1 << 30;

impl Route {
    pub fn sound_font(font: u8, bank: u16, program: u8) -> Route {
        Route { source: Source::SoundFont(font), bank, program }
    }

    /// The SoundFont it plays, if it is a SoundFont route.
    #[inline]
    pub fn font(self) -> Option<u8> {
        match self.source {
            Source::SoundFont(k) => Some(k),
            Source::Plugin(_) => None,
        }
    }

    pub fn pack(self) -> u32 {
        let (kind, id) = match self.source {
            Source::SoundFont(k) => (0, k),
            Source::Plugin(j) => (PLUGIN, j),
        };
        VALID | kind | (id as u32 & 0x3F) << 24 | (self.bank as u32) << 8 | self.program as u32 & 0x7F
    }

    pub fn unpack(v: u32) -> Option<Route> {
        if v & VALID == 0 {
            return None;
        }
        let id = (v >> 24) as u8 & 0x3F;
        let source = if v & PLUGIN != 0 { Source::Plugin(id) } else { Source::SoundFont(id) };
        Some(Route { source, bank: (v >> 8) as u16, program: v as u8 & 0x7F })
    }
}

fn pack(r: Option<Route>) -> u32 {
    r.map_or(0, Route::pack)
}

/// One bank of the table: a route per GM program, the drum route, and a generation that
/// moves whenever the bank is rewritten (the synth then routes its channels again).
pub struct RouteBank {
    prog: [AtomicU32; 128],
    drum: AtomicU32,
    generation: AtomicU32,
}

impl RouteBank {
    fn new() -> RouteBank {
        RouteBank { prog: [const { AtomicU32::new(0) }; 128], drum: AtomicU32::new(0), generation: AtomicU32::new(0) }
    }
}

/// How many banks the table has.
pub const BANKS: usize = 2;

pub struct Routes {
    banks: [RouteBank; BANKS],
    /// A keyboard part's own patch (Right 1, Right 2, Right 3, Left); 0 = none: the part's
    /// GM voice goes through the map like a Style part's.
    part: [AtomicU32; crate::parts::COUNT],
    /// Moves when a part's patch changes.
    parts_gen: AtomicU32,
    /// The `yahaha` port gets the mapped bank and program instead of the style's own.
    pub port_mapped: AtomicBool,
    /// The patch a library audition plays (on the audition channel), 0 = none.
    audition: AtomicU32,
    /// The bank the style the engine plays uses, as the control side last handed it out.
    /// For display and tests; the real-time threads follow `Sink::route_bank`.
    pub current: AtomicU8,
}

impl Default for Routes {
    fn default() -> Routes {
        Routes::new()
    }
}

impl Routes {
    pub fn new() -> Routes {
        Routes {
            banks: [RouteBank::new(), RouteBank::new()],
            part: [const { AtomicU32::new(0) }; crate::parts::COUNT],
            parts_gen: AtomicU32::new(0),
            port_mapped: AtomicBool::new(false),
            audition: AtomicU32::new(0),
            current: AtomicU8::new(0),
        }
    }

    // ----- real-time side: reads only -----

    /// The route for a program change on band channel `ch` (0-based) with bank MSB `msb`,
    /// in bank `bank` of the table: the drum route for a drum part, else the program's.
    #[inline]
    pub fn lookup(&self, bank: u8, ch: u8, msb: u8, program: u8) -> Option<Route> {
        let b = &self.banks[bank as usize % BANKS];
        let v = if super::is_drum(ch, msb) {
            b.drum.load(Relaxed)
        } else {
            b.prog[super::map_program(ch, msb, program) as usize & 127].load(Relaxed)
        };
        Route::unpack(v)
    }

    /// The route for a keyboard part's GM program (not a drum part), in bank `bank`.
    #[inline]
    pub fn lookup_program(&self, bank: u8, program: u8) -> Option<Route> {
        Route::unpack(self.banks[bank as usize % BANKS].prog[program as usize & 127].load(Relaxed))
    }

    #[inline]
    pub fn part(&self, part: usize) -> Option<Route> {
        Route::unpack(self.part[part % crate::parts::COUNT].load(Relaxed))
    }

    #[inline]
    pub fn generation(&self, bank: u8) -> u32 {
        self.banks[bank as usize % BANKS].generation.load(Acquire)
    }

    #[inline]
    pub fn parts_generation(&self) -> u32 {
        self.parts_gen.load(Acquire)
    }

    #[inline]
    pub fn audition(&self) -> Option<Route> {
        Route::unpack(self.audition.load(Relaxed))
    }

    // ----- control side -----

    /// Rewrite bank `bank`: a route per GM program and the drum route.
    pub fn write_bank(&self, bank: u8, prog: &[Option<Route>; 128], drum: Option<Route>) {
        let b = &self.banks[bank as usize % BANKS];
        let mut changed = false;
        for (a, r) in b.prog.iter().zip(prog) {
            changed |= a.swap(pack(*r), Relaxed) != pack(*r);
        }
        changed |= b.drum.swap(pack(drum), Relaxed) != pack(drum);
        if changed {
            // Release: a reader that sees the new generation sees the new routes.
            b.generation.fetch_add(1, Release);
        }
    }

    /// Set the keyboard parts' own patches.
    pub fn set_parts(&self, routes: [Option<Route>; crate::parts::COUNT]) {
        let mut changed = false;
        for (a, r) in self.part.iter().zip(routes) {
            changed |= a.swap(pack(r), Relaxed) != pack(r);
        }
        if changed {
            self.parts_gen.fetch_add(1, Release);
        }
    }

    pub fn set_audition(&self, r: Option<Route>) {
        self.audition.store(pack(r), Relaxed);
    }

    /// Bank `bank`'s route for a program (tests, the state).
    pub fn bank_route(&self, bank: u8, program: u8) -> Option<Route> {
        self.lookup_program(bank, program)
    }

    pub fn bank_drum(&self, bank: u8) -> Option<Route> {
        Route::unpack(self.banks[bank as usize % BANKS].drum.load(Relaxed))
    }
}

/// The synth-ring message that switches the table bank the band's channels use:
/// `[ROUTE_BANK, bank, 0]`. Like `click::CLICK`, a status byte no channel message uses;
/// it never goes out as MIDI.
pub const ROUTE_BANK: u8 = 0xFD;

/// Synth-ring messages for a library audition, from the control side: `[AUDITION, 1, 0]`
/// routes the audition channel to `Routes::audition`, `[AUDITION, 0, 0]` gives it back
/// to the band. Not MIDI either.
pub const AUDITION: u8 = 0xF4;

/// The channel an audition plays on (0-based): Phrase 2's, which the audition borrows
/// while the band is stopped.
pub const AUDITION_CHANNEL: u8 = 15;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_pack_and_unpack() {
        for r in [Route::sound_font(0, 0, 0), Route::sound_font(63, 128, 127), Route::sound_font(3, 8, 33), Route { source: Source::Plugin(5), bank: 0, program: 0 }] {
            assert_eq!(Route::unpack(r.pack()), Some(r));
        }
        assert_eq!(Route::unpack(0), None);
    }

    #[test]
    fn a_rewrite_moves_the_generation_only_when_something_changed() {
        let t = Routes::new();
        let mut prog = [None; 128];
        prog[33] = Some(Route::sound_font(1, 0, 34));
        t.write_bank(1, &prog, None);
        assert_eq!(t.generation(1), 1);
        assert_eq!(t.generation(0), 0);
        t.write_bank(1, &prog, None);
        assert_eq!(t.generation(1), 1, "same routes: no change");
        // Channel 11 (Bass), GM bank: program 33 routes; a Yamaha bank variation of it too.
        assert_eq!(t.lookup(1, 10, 0, 33), prog[33]);
        assert_eq!(t.lookup(1, 10, 8, 33), prog[33]);
        assert_eq!(t.lookup(0, 10, 0, 33), None, "the other bank is untouched");
        t.write_bank(1, &prog, Some(Route::sound_font(0, 128, 25)));
        assert_eq!(t.lookup(1, 9, 0, 0).map(|r| r.program), Some(25), "Rhythm 2 is a drum part");
        assert_eq!(t.lookup(1, 12, 127, 0).map(|r| r.program), Some(25), "a drum kit bank on any part");
    }
}
