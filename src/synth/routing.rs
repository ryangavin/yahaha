//! Per-channel routes (#103): each MIDI channel of the rack plays from one sound source,
//! the rack's main SoundFont or another one the sound library uses.
//!
//! A [`Rack`] holds one synthesizer per SoundFont in use: the main font's two (the band's
//! and your playing's, as before) and one more per extra font. `slot_of` says which slot
//! plays a font id (`patches::Route`), `ch_slot` which slot each channel plays now.
//!
//! What goes where:
//!
//! - a note-on, a program change and the bank selects: to the channel's slot only;
//! - everything else (note-offs, controllers, pitch bend, pressure): to every slot, so a
//!   channel keeps its level, pan and bend whichever slot it moves to, and a note started
//!   before a route change still gets its note-off.
//!
//! The band's program changes and the keyboard parts' voices go through the program map
//! (`patches::Routes`, read here, written on the control side): a route to a SoundFont the
//! rack has moves the channel to that slot with the patch's bank and program; no route (or
//! a plugin route, until #91 plays them) leaves it on the main font with the voice it
//! played before the library existed.
//!
//! Real-time safe: fixed arrays and atomic loads only.

use super::*;
use crate::patches::route::{AUDITION, AUDITION_CHANNEL, ROUTE_BANK};
use crate::patches::{Route, Routes};

/// `Rack::slot_of`: the font is not in this rack.
pub(super) const NO_SLOT: u8 = 0xFF;

/// Voices per extra SoundFont's synthesizer.
const EXTRA_POLYPHONY: usize = 64;

/// The audio thread's view of the program map: the table, the bank the band's channels
/// use now, and the generations it last routed by.
pub struct Router {
    routes: Arc<Routes>,
    active: u8,
    seen: u32,
    seen_parts: u32,
    /// An audition has the audition channel (the band's own program changes there wait).
    audition: bool,
}

impl Router {
    pub fn new(routes: Arc<Routes>) -> Router {
        Router { routes, active: 0, seen: 0, seen_parts: 0, audition: false }
    }

    /// The table moved since the channels were last routed: (band, keyboard parts).
    fn stale(&mut self) -> (bool, bool) {
        let g = self.routes.generation(self.active);
        let pg = self.routes.parts_generation();
        let r = (g != self.seen, pg != self.seen_parts);
        self.seen = g;
        self.seen_parts = pg;
        r
    }
}

impl Rack {
    /// A rack playing several SoundFonts: `fonts[0]` is the main one (the band's and your
    /// playing's synthesizers), the rest get one synthesizer each. Each is `(font id,
    /// font)`. Slow: call it off the audio thread.
    pub fn with_fonts(fonts: &[(u8, Arc<SoundFont>)], sample_rate: i32) -> Result<Rack> {
        let (main_id, main) = fonts.first().ok_or_else(|| anyhow!("no SoundFont"))?;
        let mut rack = Rack::new(main, sample_rate)?;
        rack.slot_of[*main_id as usize % crate::patches::route::MAX_FONTS] = 0;
        let mut settings = SynthesizerSettings::new(sample_rate);
        settings.maximum_polyphony = EXTRA_POLYPHONY;
        settings.velocity_to_filter = super::velocity_to_filter();
        for (id, font) in fonts.iter().skip(1) {
            let slot = &mut rack.slot_of[*id as usize % crate::patches::route::MAX_FONTS];
            if *slot != NO_SLOT || rack.extra.len() >= 250 {
                continue;
            }
            let mut s = Synthesizer::new(font, &settings).map_err(|e| anyhow!("{e:?}"))?;
            s.process_midi_message(8, 0xB0, 0, 128);
            s.set_internal_effects(false);
            *slot = rack.extra.len() as u8 + 1;
            rack.extra.push(s);
            rack.quiet.push(0);
        }
        Ok(rack)
    }

    /// The slot that plays `r`, if this rack has its SoundFont.
    #[inline]
    fn slot_for(&self, r: Route) -> Option<u8> {
        let s = self.slot_of[r.font()? as usize % crate::patches::route::MAX_FONTS];
        (s != NO_SLOT && (s as usize) <= self.extra.len()).then_some(s)
    }

    /// The synthesizer of `slot` for channel `ch`.
    #[inline]
    fn synth(&mut self, slot: u8, ch: i32) -> &mut Synthesizer {
        match slot {
            0 if parts::part_of_channel(ch as u8).is_some() => &mut self.player,
            0 => &mut self.band,
            k => &mut self.extra[k as usize - 1],
        }
    }

    /// A channel message to the synthesizer(s) that take it (see the module docs).
    #[inline]
    pub(super) fn process(&mut self, ch: i32, st: i32, d1: i32, d2: i32) {
        let routed = match st {
            0x90 => d2 > 0,
            0xC0 => true,
            0xB0 => d1 == 0 || d1 == 32,
            _ => false,
        };
        let slot = self.ch_slot[ch as usize & 15];
        if routed || self.extra.is_empty() {
            self.synth(slot, ch).process_midi_message(ch, st, d1, d2);
            return;
        }
        self.synth(0, ch).process_midi_message(ch, st, d1, d2);
        for s in &mut self.extra {
            s.process_midi_message(ch, st, d1, d2);
        }
    }

    /// Channel `ch` plays `r` from `slot`: its bank and program there.
    fn route_to(&mut self, ch: u8, slot: u8, r: Route) {
        let c = ch as usize & 15;
        self.ch_slot[c] = slot;
        self.mapped |= 1 << c;
        // Channel 10 is rustysynth's percussion channel: its bank numbers start at 128.
        let bank = if ch == 9 { r.bank.saturating_sub(128) } else { r.bank } as i32;
        let s = self.synth(slot, ch as i32);
        s.process_midi_message(ch as i32, 0xB0, 0, bank);
        s.process_midi_message(ch as i32, 0xC0, r.program as i32, 0);
    }

    /// Channel `ch` goes back to the main SoundFont; true if it was routed to a patch (its
    /// bank there has to be set again).
    fn unroute(&mut self, ch: u8) -> bool {
        let c = ch as usize & 15;
        self.ch_slot[c] = 0;
        let was = self.mapped & 1 << c != 0;
        self.mapped &= !(1 << c);
        was
    }

    /// Channel `ch` plays `r` if this rack has its SoundFont; else back on the main font.
    /// True if it routed.
    fn try_route(&mut self, ch: u8, r: Option<Route>) -> bool {
        match r.and_then(|r| Some((r, self.slot_for(r)?))) {
            Some((r, slot)) => {
                self.route_to(ch, slot, r);
                true
            }
            None => false,
        }
    }
}

/// A message to a rack, through the program map when there is one: a band program change
/// plays its route; everything else as `apply_rack`.
#[inline]
pub(super) fn apply_routed(rack: &mut Rack, m: &Msg, bank: &mut [u8; 16], router: Option<&Router>) {
    let ch = m[0] & 0x0F;
    if let Some(r) = router
        && m[0] & 0xF0 == 0xC0
        && ch >= 8
    {
        // An audition holds its channel: the band's voice there waits in the shadow.
        if r.audition && ch == AUDITION_CHANNEL {
            return;
        }
        if rack.try_route(ch, r.routes.lookup(r.active, ch, bank[ch as usize], m[1])) {
            return;
        }
        if rack.unroute(ch) && ch != 8 {
            // Rhythm 1's program change sets its drum bank itself (`translate`).
            rack.process(ch as i32, 0xB0, 0, 0);
        }
    }
    translate(m, bank, |c, st, a, b| rack.process(c, st, a, b));
}

/// The keyboard parts' voices: a part's own patch, else its GM voice through the map
/// (the band's current bank), else the GM voice on the main font.
pub(super) fn sync_parts(rack: &mut Rack, parts: &Parts, router: Option<&Router>) {
    for p in 0..parts::COUNT {
        let ch = parts::CHANNEL[p];
        let prog = parts.channel_program(p);
        if let Some(r) = router {
            // Under Manual Bass, Left plays the Style's Bass voice, not its own patch.
            let own = (!(p == parts::LEFT && parts.manual_bass.load(Relaxed))).then(|| r.routes.part(p)).flatten();
            if rack.try_route(ch, own.or_else(|| r.routes.lookup_program(r.active, prog))) {
                continue;
            }
            if rack.unroute(ch) {
                rack.process(ch as i32, 0xB0, 0, 0);
            }
        }
        rack.process(ch as i32, 0xC0, prog as i32, 0);
    }
}

impl Shadow {
    /// Route the band's channels again from the programs they last got (the map changed,
    /// or the band switched table banks).
    pub(super) fn reroute(&self, rack: &mut Rack, bank: &mut [u8; 16], router: &Router) {
        for ch in 8..16u8 {
            if let Some(p) = self.program[ch as usize] {
                apply_routed(rack, &[0xC0 | ch, p, 0], bank, Some(router));
            }
        }
    }
}

/// Per buffer, before the MIDI: route again whatever the table changed under.
pub(super) fn follow_table(rack: &mut Rack, shadow: &Shadow, bank: &mut [u8; 16], parts: &Parts, router: &mut Router) {
    let (band, keys) = router.stale();
    if band {
        shadow.reroute(rack, bank, router);
    }
    if band || keys {
        sync_parts(rack, parts, Some(router));
    }
}

/// A synth-ring message that is not MIDI but the program map's: a table bank switch from
/// the engine (`ROUTE_BANK`), or an audition starting or ending (`AUDITION`). True if it
/// was one.
pub(super) fn control_msg(m: &Msg, rack: &mut Rack, shadow: &Shadow, bank: &mut [u8; 16], parts: &Parts, router: &mut Router) -> bool {
    match m[0] {
        ROUTE_BANK => {
            let b = m[1] % crate::patches::route::BANKS as u8;
            if b != router.active {
                router.active = b;
                router.seen = router.routes.generation(b);
                shadow.reroute(rack, bank, router);
                sync_parts(rack, parts, Some(router));
            }
            true
        }
        AUDITION => {
            let ch = AUDITION_CHANNEL;
            // Whatever sounds on the channel stops; the audition starts from a clean level.
            rack.process(ch as i32, 0xB0, 123, 0);
            if m[1] != 0 {
                router.audition = true;
                if !rack.try_route(ch, router.routes.audition()) {
                    rack.unroute(ch);
                }
                for (cc, v) in [(7, 100), (11, 127), (10, 64), (1, 0), (64, 0)] {
                    rack.process(ch as i32, 0xB0, cc, v);
                }
                rack.process(ch as i32, 0xE0, 0, 64);
            } else {
                router.audition = false;
                // The band's own setup of the channel again.
                rack.unroute(ch);
                shadow.replay_channel(rack, bank, ch, Some(router));
            }
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patches::route::MAX_FONTS;

    /// Two tiny test SoundFonts (built in code: they run whatever the checkout has).
    fn fonts() -> Option<Vec<(String, Arc<SoundFont>)>> {
        let font = || Arc::new(SoundFont::new(&mut &crate::patches::sf2::tiny_gm_sound_font()[..]).unwrap());
        Some(vec![("A.sf2".into(), font()), ("B.sf2".into(), font())])
    }

    fn peaks() -> [AtomicU32; 16] {
        std::array::from_fn(|_| AtomicU32::new(0))
    }

    fn level(p: &[AtomicU32; 16], ch: usize) -> f32 {
        f32::from_bits(p[ch].swap(0, Relaxed))
    }

    /// A band program change routed to a second SoundFont plays there, metered on its own
    /// channel; the main font's synthesizer stays silent; unmapping brings it back.
    #[test]
    fn a_mapped_program_plays_from_its_soundfont_and_is_metered() {
        let Some(f) = fonts() else { return };
        // The same font twice if the checkout has one: two slots all the same.
        let second = f.get(1).unwrap_or(&f[0]).1.clone();
        let mut rack = Rack::with_fonts(&[(3, f[0].1.clone()), (7, second)], 48_000).unwrap();
        assert_eq!((rack.slot_of[3], rack.slot_of[7], rack.extra.len()), (0, 1, 1));
        assert_eq!(rack.slot_of.iter().filter(|&&s| s != NO_SLOT).count(), 2);
        let routes = Arc::new(Routes::new());
        let mut prog = [None; 128];
        for r in &mut prog[32..40] {
            *r = Some(Route::sound_font(7, 0, 33));
        }
        routes.write_bank(0, &prog, None);
        let mut router = Router::new(routes.clone());
        let parts = Parts::new();
        let shadow = Shadow::new();
        let mut bank = [0u8; 16];
        follow_table(&mut rack, &shadow, &mut bank, &parts, &mut router);
        let p = peaks();
        let (mut l, mut r) = (vec![0f32; 512], vec![0f32; 512]);
        for m in [[0xBA, 0, 8], [0xCA, 35, 0], [0xBA, 7, 100], [0x9A, 40, 110]] {
            apply_routed(&mut rack, &m, &mut bank, Some(&router));
        }
        assert_eq!(rack.ch_slot[10], 1, "Fretless (bank 8) is in the Bass family: the second font");
        for _ in 0..8 {
            rack.render_dry(&mut l, &mut r, &p, None);
        }
        assert!(level(&p, 10) > 1e-3, "the routed bass is metered on ch 11");
        // The note-off reaches the slot that plays the note, wherever the channel is now.
        apply_routed(&mut rack, &[0xBA, 0, 0], &mut bank, Some(&router));
        apply_routed(&mut rack, &[0xCA, 0, 0], &mut bank, Some(&router));
        assert_eq!(rack.ch_slot[10], 0, "an unmapped program: back on the main font");
        apply_routed(&mut rack, &[0x8A, 40, 0], &mut bank, Some(&router));
        for _ in 0..200 {
            rack.render_dry(&mut l, &mut r, &p, None);
        }
        level(&p, 10);
        for _ in 0..4 {
            rack.render_dry(&mut l, &mut r, &p, None);
        }
        let tail = level(&p, 10);
        assert!(tail < 1e-4, "the note ended on the extra font ({tail})");
        let _ = MAX_FONTS;
    }

    /// A route to a SoundFont the rack doesn't have, or to a plugin, plays the fallback.
    #[test]
    fn a_missing_font_or_a_plugin_plays_the_fallback() {
        let Some(f) = fonts() else { return };
        let mut rack = Rack::with_fonts(&[(0, f[0].1.clone())], 48_000).unwrap();
        let routes = Arc::new(Routes::new());
        let mut prog = [None; 128];
        prog[33] = Some(Route::sound_font(9, 0, 33));
        prog[34] = Some(Route { source: crate::patches::Source::Plugin(1), bank: 0, program: 0 });
        prog[35] = Some(Route::sound_font(0, 0, 36));
        routes.write_bank(0, &prog, None);
        let router = Router::new(routes);
        let mut bank = [0u8; 16];
        for (p, want) in [(33, 0), (34, 0), (35, 0)] {
            apply_routed(&mut rack, &[0xCA, p, 0], &mut bank, Some(&router));
            assert_eq!(rack.ch_slot[10], want);
        }
        assert_eq!(rack.mapped & 1 << 10, 1 << 10, "program 35 routes to the main font's program 36");
    }

    /// Switching table banks (a style change) routes the band's channels again from the
    /// programs they have; keyboard parts follow their own patch or the map.
    #[test]
    fn a_bank_switch_and_part_patches_route_again() {
        let Some(f) = fonts() else { return };
        let second = f.get(1).unwrap_or(&f[0]).1.clone();
        let mut rack = Rack::with_fonts(&[(0, f[0].1.clone()), (1, second)], 48_000).unwrap();
        let routes = Arc::new(Routes::new());
        let mut prog = [None; 128];
        prog[48] = Some(Route::sound_font(1, 0, 49));
        routes.write_bank(1, &prog, Some(Route::sound_font(1, 128, 0)));
        let mut router = Router::new(routes.clone());
        let parts = Parts::new();
        let mut shadow = Shadow::new();
        let mut bank = [0u8; 16];
        for m in [[0xCC, 48, 0], [0xC8, 0, 0]] {
            shadow.note(&m);
            apply_routed(&mut rack, &m, &mut bank, Some(&router));
        }
        assert_eq!((rack.ch_slot[12], rack.ch_slot[8]), (0, 0), "bank 0 is empty");
        assert!(control_msg(&[ROUTE_BANK, 1, 0], &mut rack, &shadow, &mut bank, &parts, &mut router));
        assert_eq!((rack.ch_slot[12], rack.ch_slot[8]), (1, 1), "Chord 1's strings and the drums move");
        // Right 1 (Grand Piano) has no rule; Left's Strings (48) take the map's.
        assert_eq!((rack.ch_slot[0], rack.ch_slot[1]), (0, 1));
        routes.set_parts([Some(Route::sound_font(1, 0, 4)), None, None, None]);
        follow_table(&mut rack, &shadow, &mut bank, &parts, &mut router);
        assert_eq!(rack.ch_slot[0], 1, "Right 1's own patch");
        assert!(!control_msg(&[0xB0, 7, 100], &mut rack, &shadow, &mut bank, &parts, &mut router));
    }

    /// An extra SoundFont's synthesizer no channel plays is rendered until its tail has
    /// died away, then skipped; a channel routed to it again brings it back (#109).
    #[test]
    fn an_idle_extra_soundfont_is_not_rendered() {
        let Some(f) = fonts() else { return };
        let mut rack = Rack::with_fonts(&[(0, f[0].1.clone()), (1, f[1].1.clone())], 48_000).unwrap();
        let p = peaks();
        let (mut l, mut r) = (vec![0f32; 512], vec![0f32; 512]);
        rack.route_to(10, 1, Route::sound_font(1, 0, 33));
        rack.process(10, 0x90, 40, 110);
        rack.render_dry(&mut l, &mut r, &p, None);
        assert_eq!(rack.quiet[0], 0, "a slot a channel plays renders");
        rack.process(10, 0x80, 40, 0);
        rack.unroute(10);
        let mut n = 0;
        while rack.quiet[0] < IDLE_FRAMES && n < 4000 {
            rack.render_dry(&mut l, &mut r, &p, None);
            n += 1;
        }
        assert!(n > 1, "the tail is rendered first");
        let idle = rack.quiet[0];
        assert!(idle >= IDLE_FRAMES, "the tail died away ({n} buffers)");
        rack.render_dry(&mut l, &mut r, &p, None);
        assert_eq!(rack.quiet[0], idle, "then it is skipped");
        rack.route_to(10, 1, Route::sound_font(1, 0, 33));
        rack.render_dry(&mut l, &mut r, &p, None);
        assert_eq!(rack.quiet[0], 0, "a channel routed to it again renders it");
        rack.process(10, 0x90, 40, 110);
        rack.render_dry(&mut l, &mut r, &p, None);
        assert!(level(&p, 10) > 1e-3, "and it sounds");
    }
}
