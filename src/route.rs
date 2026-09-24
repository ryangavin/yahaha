//! Per-MIDI-channel sound sources: which engine renders each of the 16 channels.
//!
//! The built-in synth's audio callback renders every channel (the keyboard parts on 0-3,
//! the Multi Pads on 4-7, the Style parts on 8-15) from one of:
//!
//! - [`Source::SoundFont`]`(n)`: the built-in SoundFont synth, font `n` (0 = the synth's
//!   SoundFont; 1-14 are reserved for per-channel SoundFonts, #103), or
//! - [`Source::Plugin`]: the plugin rack's slot for that channel (an Audio Unit
//!   instrument, `src/plugin/`).
//!
//! The whole table is one `u64` (16 channels x 4 bits), so a change is a single atomic
//! store and the audio thread reads a consistent table with a single atomic load per
//! buffer. Writers are non-real-time threads (the Session's control side); the audio
//! thread only reads. [`ChannelRoutes::set`] is a compare-and-swap loop, so concurrent
//! writers never lose each other's channels.
//!
//! What the audio thread does with a route (docs/plugin-hosting.md, "Phase 2 as built"):
//! a channel routed to a plugin whose rack slot has an instance plays on the plugin; its
//! note-ons no longer reach the SoundFont, but everything else (controllers, program,
//! bend) still does, so the SoundFont side stays in step and takes the channel back at
//! the right level. A plugin route whose slot has no instance yet (still loading, or the
//! assign not yet applied) keeps playing the SoundFont, so a part is never silent while a
//! plugin loads.

use std::sync::atomic::{AtomicU64, Ordering::{Acquire, Relaxed, Release}};

/// Where one MIDI channel's sound comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Source {
    /// The built-in SoundFont synth; `0` is the synth's SoundFont, 1-14 are other fonts
    /// (reserved: the audio callback renders every SoundFont channel on font 0 today).
    SoundFont(u8),
    /// The plugin rack's slot for this channel.
    Plugin,
}

/// The highest SoundFont index a route can name.
pub const MAX_FONT: u8 = 14;
const PLUGIN: u64 = 0xF;

impl Source {
    #[inline]
    const fn code(self) -> u64 {
        match self {
            Source::SoundFont(n) => (if n > MAX_FONT { MAX_FONT } else { n }) as u64,
            Source::Plugin => PLUGIN,
        }
    }

    #[inline]
    const fn from_code(c: u64) -> Source {
        if c == PLUGIN { Source::Plugin } else { Source::SoundFont(c as u8) }
    }
}

/// A snapshot of all 16 routes (`Copy`, 8 bytes).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RouteTable(u64);

impl RouteTable {
    /// Every channel on the synth's SoundFont.
    pub const ALL_SOUND_FONT: RouteTable = RouteTable(0);

    /// Channel `ch`'s (0-15) source.
    #[inline]
    pub const fn source(self, ch: u8) -> Source {
        Source::from_code((self.0 >> ((ch as u64 & 15) * 4)) & 0xF)
    }

    /// This table with channel `ch` (0-15) set to `src`.
    #[inline]
    #[must_use]
    pub const fn with(self, ch: u8, src: Source) -> RouteTable {
        let shift = (ch as u64 & 15) * 4;
        RouteTable((self.0 & !(0xF << shift)) | (src.code() << shift))
    }

    /// Bit `ch` set for each channel routed to a plugin.
    #[inline]
    pub const fn plugin_mask(self) -> u16 {
        let mut m = 0u16;
        let mut ch = 0;
        while ch < 16 {
            if (self.0 >> (ch * 4)) & 0xF == PLUGIN {
                m |= 1 << ch;
            }
            ch += 1;
        }
        m
    }

    /// The raw 64-bit word (4 bits per channel, channel 0 lowest).
    pub const fn bits(self) -> u64 {
        self.0
    }
}

/// The live route table, shared between the control side (writes) and the audio thread
/// (reads). Lock-free; see the module docs.
#[derive(Debug, Default)]
pub struct ChannelRoutes(AtomicU64);

impl ChannelRoutes {
    pub const fn new() -> ChannelRoutes {
        ChannelRoutes(AtomicU64::new(0))
    }

    /// The current table. RT-safe: one atomic load (Acquire, pairing with the writers'
    /// Release, so what a writer prepared before routing a channel is visible).
    #[inline]
    pub fn table(&self) -> RouteTable {
        RouteTable(self.0.load(Acquire))
    }

    /// Channel `ch`'s current source.
    #[inline]
    pub fn source(&self, ch: u8) -> Source {
        self.table().source(ch)
    }

    /// Route channel `ch` (0-15) to `src`, leaving the others as they are. Returns the
    /// source it had. For non-real-time threads (a CAS loop).
    pub fn set(&self, ch: u8, src: Source) -> Source {
        let mut cur = self.0.load(Relaxed);
        loop {
            let next = RouteTable(cur).with(ch, src).0;
            match self.0.compare_exchange_weak(cur, next, Release, Relaxed) {
                Ok(_) => return RouteTable(cur).source(ch),
                Err(now) => cur = now,
            }
        }
    }

    /// Replace the whole table at once.
    pub fn store(&self, t: RouteTable) {
        self.0.store(t.0, Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_pack_into_one_word() {
        let t = RouteTable::ALL_SOUND_FONT;
        assert!((0..16).all(|c| t.source(c) == Source::SoundFont(0)));
        let t = t.with(0, Source::Plugin).with(10, Source::SoundFont(3)).with(15, Source::Plugin);
        assert_eq!(t.source(0), Source::Plugin);
        assert_eq!(t.source(10), Source::SoundFont(3));
        assert_eq!(t.source(15), Source::Plugin);
        assert_eq!(t.source(1), Source::SoundFont(0));
        assert_eq!(t.plugin_mask(), 1 | 1 << 15);
        // Out-of-range fonts clamp, never alias the plugin code.
        assert_eq!(t.with(2, Source::SoundFont(200)).source(2), Source::SoundFont(MAX_FONT));
        assert_eq!(t.with(0, Source::SoundFont(0)).plugin_mask(), 1 << 15);
    }

    #[test]
    fn concurrent_writers_keep_each_others_channels() {
        let r = std::sync::Arc::new(ChannelRoutes::new());
        let hs: Vec<_> = (0..8u8)
            .map(|ch| {
                let r = r.clone();
                std::thread::spawn(move || {
                    for i in 0..2000 {
                        r.set(ch, if i % 2 == 0 { Source::Plugin } else { Source::SoundFont(ch % 3) });
                    }
                    r.set(ch, Source::Plugin);
                })
            })
            .collect();
        for h in hs {
            h.join().unwrap();
        }
        assert_eq!(r.table().plugin_mask(), 0xFF);
        assert_eq!(r.set(3, Source::SoundFont(1)), Source::Plugin);
        assert_eq!(r.source(3), Source::SoundFont(1));
    }
}
