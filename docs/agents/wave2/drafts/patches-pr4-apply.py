def edit(p, pairs):
    s = open(p).read()
    for a, b in pairs:
        assert s.count(a) == 1, (p, a)
        s = s.replace(a, b)
    open(p, 'w').write(s)


# 1. Prune empty style-map entries; 2. recycle font ids.
edit('src/session/sound_library.rs', [
("""    /// The font id of a SoundFont file (a new one if it has none yet).
    pub(super) fn font_id(&mut self, file: &str) -> Option<u8> {
        if let Some(i) = self.fonts.iter().position(|f| f == file) {
            return Some(i as u8);
        }
        if self.fonts.len() >= MAX_FONTS {
            return None;
        }
        self.fonts.push(file.to_string());
        Some(self.fonts.len() as u8 - 1)
    }
""", """    /// The font id of a SoundFont file (a new one if it has none yet). When all
    /// `MAX_FONTS` ids are taken, the id of a SoundFont nothing uses any more is given
    /// again (`font_in_use`): auditioning presets of many SoundFonts never runs out.
    pub(super) fn font_id(&mut self, file: &str) -> Option<u8> {
        if let Some(i) = self.fonts.iter().position(|f| f == file) {
            return Some(i as u8);
        }
        if self.fonts.len() < MAX_FONTS {
            self.fonts.push(file.to_string());
            return Some(self.fonts.len() as u8 - 1);
        }
        let free = (0..self.fonts.len()).find(|&i| !self.font_in_use(&self.fonts[i]))?;
        self.fonts[free] = file.to_string();
        Some(free as u8)
    }

    /// A SoundFont's id may still be read somewhere: the rack playing or the one loading
    /// has the font (their `slot_of`), or a patch or the audition plays it (the route
    /// table). An id none of these knows can name another file safely.
    fn font_in_use(&self, file: &str) -> bool {
        let has = |fonts: &[String]| fonts.iter().any(|f| f == file);
        has(&self.rack_fonts)
            || self.loading.as_deref().is_some_and(has)
            || self.audition.as_ref().is_some_and(|a| a.font.as_deref() == Some(file))
            || self.lib.patches.iter().any(|p| matches!(&p.source, PatchSource::SoundFont { file: f, .. } if f == file))
    }
"""),
("""    fn sound_library_changed(&mut self) {
        let avail = self.avail_fonts();""", """    fn sound_library_changed(&mut self) {
        // A style's own map left with no rules (all cleared through `map_mut`) goes.
        self.sound.lib.style_maps.retain(|_, m| !m.is_empty());
        let avail = self.avail_fonts();"""),
])

# 3. Skip idle extra-font synthesizers.
edit('src/synth.rs', [
("""    /// Channels routed to a library patch (bit = channel).
    mapped: u16,
}""", """    /// Channels routed to a library patch (bit = channel).
    mapped: u16,
    /// Per extra synthesizer: how many frames it has rendered silence with no channel on
    /// it. Past `IDLE_FRAMES` it is not rendered until a channel routes to it again.
    quiet: Vec<u32>,
}

/// An extra synthesizer no channel plays is rendered until its output (reverb and chorus
/// tails included) has been below `IDLE_LEVEL` for this long, then skipped.
const IDLE_FRAMES: u32 = 4800;
const IDLE_LEVEL: f32 = 1e-6;"""),
("""            ch_slot: [0; 16],
            mapped: 0,
        };""", """            ch_slot: [0; 16],
            mapped: 0,
            quiet: Vec::new(),
        };"""),
("""        let (l, r) = (&mut self.tmp_l[..n], &mut self.tmp_r[..n]);
        for s in std::iter::once(&mut self.player).chain(self.extra.iter_mut()) {
            s.render(l, r);
            for k in 0..n {
                left[k] += l[k];
                right[k] += r[k];
            }
        }""", """        let (l, r) = (&mut self.tmp_l[..n], &mut self.tmp_r[..n]);
        // The extra synthesizers some channel plays (slot k+1 = extra[k]).
        let mut used = 0u64;
        for &s in &self.ch_slot {
            if s != 0 && s != NO_SLOT {
                used |= 1 << ((s - 1) & 63);
            }
        }
        for (i, s) in std::iter::once(&mut self.player).chain(self.extra.iter_mut()).enumerate() {
            let quiet = if i == 0 { None } else { self.quiet.get_mut(i - 1) };
            let played = i == 0 || used >> ((i - 1) & 63) & 1 == 1;
            if let Some(q) = quiet.as_deref()
                && !played
                && *q >= IDLE_FRAMES
            {
                continue;
            }
            s.render(l, r);
            let mut peak = 0f32;
            for k in 0..n {
                left[k] += l[k];
                right[k] += r[k];
                peak = peak.max(l[k].abs()).max(r[k].abs());
            }
            if let Some(q) = quiet {
                *q = if played || peak >= IDLE_LEVEL { 0 } else { q.saturating_add(n as u32) };
            }
        }"""),
])
edit('src/synth/routing.rs', [
("""            *slot = rack.extra.len() as u8 + 1;
            rack.extra.push(s);""", """            *slot = rack.extra.len() as u8 + 1;
            rack.extra.push(s);
            rack.quiet.push(0);"""),
])
print("ok")
