//! A style's XG Drum Setup on the built-in synth (#239).
//!
//! A style's SInt tunes its drum kits note by note with XG Drum Setup SysEx
//! (`F0 43 1n 4C 3s rr pp vv F7`: setup s, note rr, parameter pp). The port passes the SysEx
//! on, so an XG instrument applies it itself. The built-in synth's ring takes 3-byte
//! messages only, so the engine thread's `live::Out` turns each drum setup SysEx into a
//! 3-byte *drum message* ([`encode`]). The audio thread keeps the setups
//! ([`DrumSetups::observe`]). Each note's settings ([`DrumSetups::note`], [`NoteSettings`])
//! are played on upstream rustysynth's primitives (#346 step 5):
//! - its **sends** by a lane of the part with those sends (rack.rs), at the note-on;
//! - everything else by a **kit** derived from the part's drum kit with the settings baked
//!   into its generators (kit.rs), built off the audio thread from [`DrumSetups::kit_params`].
//!   The style's kits are built as it loads ([`prebuilds`]); a setup that changes later
//!   plays from once its kit is built.
//!
//! Either way a note keeps what it started with: a change never moves a note already
//! sounding.
//!
//! Applied:
//! - **Level** (02) as gain, on the CC7 curve (gain = (level / [`DEFAULT_LEVEL`])²).
//! - **Pitch coarse / fine** (00/01) in semitones and cents.
//! - **Pan** (04), in place of the kit's own pan (0 = random per note; the kit's own pan
//!   plays then, as a kit can't draw one per note).
//! - **Reverb / Chorus / Variation send** (05/06/07), scaling the part's send into the
//!   effect bus (value / 127).
//! - **Filter cutoff / resonance** (0B/0C), and **EG attack / decay 1 / decay 2**
//!   (0D/0E/0F: attack, decay and release times).
//!
//! Not applied: Alternate Group, Key Assign and Rcv Note Off/On (no corpus style sets them).
//!
//! The state follows the Data List's rules (DL "MIDI Parameter Change table (DRUM
//! SETUP)"):
//! - A part plays Drum Setup 1 or 2 when its XG Part Mode is DRUMS1 or DRUMS2
//!   (`F0 43 1n 4C 08 pp 07 02|03 F7`). Part 10 is DRUMS1 until told otherwise.
//! - A program change on a part that plays a drum setup initializes that setup.
//! - A Drum Setup Reset, or an XG/GM/GS system reset, initializes the setups.

use super::Msg;

/// Drum Setup parameter addresses (DL).
const COARSE: u8 = 0x00;
const FINE: u8 = 0x01;
const LEVEL: u8 = 0x02;
const PAN: u8 = 0x04;
const REVERB: u8 = 0x05;
const CHORUS: u8 = 0x06;
const VARIATION: u8 = 0x07;
const CUTOFF: u8 = 0x0B;
const RESONANCE: u8 = 0x0C;
const ATTACK: u8 = 0x0D;
const DECAY1: u8 = 0x0E;
const DECAY2: u8 = 0x0F;
/// The parameters a drum message carries (00-0F).
pub const PARAMS: usize = 16;
/// The parameters a kit bakes in (kit.rs): all but the sends.
const KIT_PARAMS: [u8; 9] = [COARSE, FINE, LEVEL, PAN, CUTOFF, RESONANCE, ATTACK, DECAY1, DECAY2];
/// XG Multi Part parameter address of the Part Mode.
const PART_MODE: u8 = 0x07;
/// Part Mode values that select Drum Setup 1 and 2.
const DRUMS1: u8 = 0x02;
const DRUMS2: u8 = 0x03;
/// No drum setup on the part / no value set.
pub const NONE: u8 = 0xFF;
/// The drum setups a Genos has (DL: "n: Drum Setup Number (0 – 1)").
const SETUPS: usize = 2;

/// Drum messages on the synth ring. Their first byte is below 80H, which no MIDI message
/// starts with: `[PARAM | setup << 4 | parameter, note, value]`, `[PART, part, mode]`,
/// `[SETUP_RESET, setup, 0]` and `[SYSTEM_RESET, 0, 0]`.
const PARAM: u8 = 0x40;
const PART: u8 = 0x60;
const SETUP_RESET: u8 = 0x61;
const SYSTEM_RESET: u8 = 0x62;

/// The Level a note plays at when the setup does not set one. The Data List gives the XG
/// default as "depends on the note", and nothing lists it. Decision: 100. It is the value
/// style authors set most (117 of 1021 corpus Level messages), and their levels spread
/// both ways around it (72–127, mean ≈ 100), as edits of a default in the middle would.
/// 127 is set explicitly 46 times, so it isn't the default for those notes.
pub const DEFAULT_LEVEL: u8 = 100;
/// XG's centre for the signed parameters (pitch, filter, EG): 40H = 0.
const CENTRE: i32 = 0x40;
/// EG rate and filter cutoff steps per octave. Decision: 16, so the ±63 range spans about
/// ±4 octaves (a 16x faster or slower envelope, a cutoff 16x higher or lower). The DL gives
/// only the range; the corpus decays sit at +5..+16 (a slightly shorter drum).
const STEPS_PER_OCTAVE: f32 = 16.0;
/// Decibels of resonance per step. Decision: 0.2, so the range spans about ±12 dB.
const RESONANCE_DB_PER_STEP: f32 = 0.2;
/// The send buses a note's sends scale (reverb, chorus, variation).
pub const SENDS: usize = 3;

/// A drum note's own settings, as its drum setup gives them. [`NoteSettings::NEUTRAL`]
/// plays the kit as it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoteSettings {
    /// Gain (linear).
    pub gain: f32,
    /// Semitones added to the pitch (whatever the kit's scale tuning).
    pub tune: f32,
    /// The pan (SoundFont units, -50 left .. 50 right) in place of the kit's; `None` keeps
    /// the kit's.
    pub pan: Option<f32>,
    /// The part's send into each send bus (reverb, chorus, variation) scaled by these.
    pub sends: [f32; SENDS],
    /// Filter cutoff scaled by this factor.
    pub cutoff: f32,
    /// Decibels added to the filter's resonance.
    pub resonance_db: f32,
    /// The volume envelope's attack, decay and release times scaled by these factors.
    pub attack: f32,
    pub decay: f32,
    pub release: f32,
}

impl NoteSettings {
    /// A note as the kit plays it.
    pub const NEUTRAL: NoteSettings =
        NoteSettings { gain: 1.0, tune: 0.0, pan: None, sends: [1.0; SENDS], cutoff: 1.0, resonance_db: 0.0, attack: 1.0, decay: 1.0, release: 1.0 };

    /// The settings a note's parameters (`NONE`: not set) give, its pan (if set) `pan`
    /// (1-127: random pan already drawn; 0: the kit's own).
    fn of(p: &[u8; PARAMS], pan: Option<i32>) -> NoteSettings {
        let get = |a: u8| (p[a as usize] != NONE).then_some(p[a as usize] as i32);
        let signed = |a: u8| get(a).map_or(0, |v| v - CENTRE);
        let octaves = |a: u8| 2f32.powf(signed(a) as f32 / STEPS_PER_OCTAVE);
        let mut n = NoteSettings::NEUTRAL;
        if let Some(l) = get(LEVEL) {
            let g = l as f32 / DEFAULT_LEVEL as f32;
            n.gain = g * g;
        }
        n.tune = signed(COARSE) as f32 + signed(FINE) as f32 / 100.0;
        n.pan = pan.filter(|&v| v > 0).map(|v| (v - CENTRE) as f32 * 50.0 / 64.0);
        for (i, a) in [REVERB, CHORUS, VARIATION].into_iter().enumerate() {
            if let Some(v) = get(a) {
                n.sends[i] = v as f32 / 127.0;
            }
        }
        n.cutoff = octaves(CUTOFF);
        n.resonance_db = signed(RESONANCE) as f32 * RESONANCE_DB_PER_STEP;
        // A higher rate is a shorter time.
        n.attack = 1.0 / octaves(ATTACK);
        n.decay = 1.0 / octaves(DECAY1);
        n.release = 1.0 / octaves(DECAY2);
        n
    }

    /// The settings a kit bakes in for a note's parameters ([`DrumSetups::kit_params`]):
    /// random pan leaves the kit's own.
    pub fn baked(p: &[u8; PARAMS]) -> NoteSettings {
        NoteSettings::of(p, (p[PAN as usize] != NONE).then_some(p[PAN as usize] as i32))
    }

    /// The same settings as SoundFont generator offsets, exactly (kit.rs).
    pub fn steps(p: &[u8; PARAMS]) -> KeySteps {
        let signed = |a: u8| if p[a as usize] == NONE { 0 } else { p[a as usize] as i32 - CENTRE };
        KeySteps {
            level: (p[LEVEL as usize] != NONE).then_some(p[LEVEL as usize]),
            coarse: signed(COARSE),
            fine: signed(FINE),
            pan: (p[PAN as usize] != NONE && p[PAN as usize] > 0).then(|| ((p[PAN as usize] as i32 - CENTRE) as f32 * 500.0 / 64.0).round() as i32),
            cutoff_cents: signed(CUTOFF) * CENTS_PER_STEP,
            resonance_cb: signed(RESONANCE) * RESONANCE_CB_PER_STEP,
            attack_tc: -signed(ATTACK) * CENTS_PER_STEP,
            decay_tc: -signed(DECAY1) * CENTS_PER_STEP,
            release_tc: -signed(DECAY2) * CENTS_PER_STEP,
        }
    }
}

/// Cents (and timecents) per cutoff or EG step: an octave (1200) in `STEPS_PER_OCTAVE`.
const CENTS_PER_STEP: i32 = 75;
/// Centibels of resonance per step (`RESONANCE_DB_PER_STEP`).
const RESONANCE_CB_PER_STEP: i32 = 2;

/// A note's settings as SoundFont generator amounts ([`NoteSettings::steps`]): the same
/// numbers [`NoteSettings`] has, in the units the generators take.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeySteps {
    /// The Level (gain (level / 100)²), if set.
    pub level: Option<u8>,
    /// Semitones and cents added (coarse and fine tune).
    pub coarse: i32,
    pub fine: i32,
    /// The pan in place of the kit's, in 0.1 % (SoundFont pan units), if set.
    pub pan: Option<i32>,
    /// Cents added to the filter cutoff, centibels to its resonance.
    pub cutoff_cents: i32,
    pub resonance_cb: i32,
    /// Timecents added to the volume envelope's attack, decay and release.
    pub attack_tc: i32,
    pub decay_tc: i32,
    pub release_tc: i32,
}

/// A drum setup's parameters as a kit bakes them ([`DrumSetups::kit_params`]): per note,
/// the parameters in `KIT_PARAMS` that change the sound, the rest `NONE`.
pub type KitParams = [[u8; PARAMS]; 128];

/// The drum message that stands for `m` on the synth ring, if `m` is SysEx that changes
/// a drum setup: a Drum Setup parameter, a Part Mode, a Drum Setup Reset or a system reset.
#[inline]
pub fn encode(m: &[u8]) -> Option<Msg> {
    match *m {
        [0xF0, 0x43, d, 0x4C, s @ (0x30 | 0x31), note, p, v, 0xF7] if d & 0xF0 == 0x10 && note < 0x80 && (p as usize) < PARAMS => {
            Some([PARAM | (s - 0x30) << 4 | p, note, v])
        }
        [0xF0, 0x43, d, 0x4C, 0x08, part, PART_MODE, mode, 0xF7] if d & 0xF0 == 0x10 && part < 16 => Some([PART, part, mode]),
        [0xF0, 0x43, d, 0x4C, 0x00, 0x00, 0x7D, s, ..] if d & 0xF0 == 0x10 && (s as usize) < SETUPS => Some([SETUP_RESET, s, 0]),
        [0xF0, ..] if yahaha_sff::sff::is_reset(m) => Some([SYSTEM_RESET, 0, 0]),
        _ => None,
    }
}

/// A drum message (`encode`), not a MIDI message.
#[inline]
pub fn is_drum_msg(m: &Msg) -> bool {
    (PARAM..=SYSTEM_RESET).contains(&m[0])
}

/// The drum setups as the built-in synth plays them. No allocation: fixed tables, updated
/// message by message on the audio thread.
#[derive(Clone)]
pub struct DrumSetups {
    /// Each setup's parameters per note (`NONE`: not set).
    params: [[[u8; PARAMS]; 128]; SETUPS],
    /// Notes with any parameter set, per setup (bit per note).
    set: [u128; SETUPS],
    /// The setup each channel's part plays (`NONE`: not a drum-setup part).
    setup_of: [u8; 16],
    /// Random pan (Pan 0): a xorshift state.
    rng: u32,
    /// Counts the changes (for the kits to know when to look again).
    changes: u32,
}

impl Default for DrumSetups {
    fn default() -> Self {
        Self::new()
    }
}

impl DrumSetups {
    pub const fn new() -> DrumSetups {
        let mut setup_of = [NONE; 16];
        // DL Multi Part PART MODE default: part10 = 02 (DRUMS1), other parts 00.
        setup_of[9] = 0;
        DrumSetups { params: [[[NONE; PARAMS]; 128]; SETUPS], set: [0; SETUPS], setup_of, rng: 0x9E37_79B9, changes: 0 }
    }

    fn clear(&mut self, setup: usize) {
        if let Some(t) = self.params.get_mut(setup)
            && self.set[setup] != 0
        {
            *t = [[NONE; PARAMS]; 128];
            self.set[setup] = 0;
            self.changes = self.changes.wrapping_add(1);
        }
    }

    /// Follow a message on the synth ring: a drum message, or a program change that
    /// initializes its part's setup. Any other message is ignored.
    #[inline]
    pub fn observe(&mut self, m: &Msg) {
        match m[0] {
            PARAM..=0x5F => {
                let setup = ((m[0] >> 4) & 1) as usize;
                let note = (m[1] & 0x7F) as usize;
                self.params[setup][note][(m[0] & 0x0F) as usize] = m[2] & 0x7F;
                self.set[setup] |= 1 << note;
                self.changes = self.changes.wrapping_add(1);
            }
            PART => {
                self.setup_of[(m[1] & 0x0F) as usize] = match m[2] {
                    DRUMS1 => 0,
                    DRUMS2 => 1,
                    _ => NONE,
                };
                self.changes = self.changes.wrapping_add(1);
            }
            SETUP_RESET => self.clear(m[1] as usize),
            SYSTEM_RESET => *self = DrumSetups { rng: self.rng, changes: self.changes.wrapping_add(1), ..DrumSetups::new() },
            st if st & 0xF0 == 0xC0 => self.clear(self.setup_of[(st & 0x0F) as usize] as usize),
            _ => {}
        }
    }

    /// Changes so far: when it moves, a part's kit may have to change.
    #[inline]
    pub fn changes(&self) -> u32 {
        self.changes
    }

    /// The drum setup the part on `ch` plays, if any.
    #[inline]
    pub fn setup_of(&self, ch: u8) -> Option<usize> {
        let s = self.setup_of[ch as usize & 15] as usize;
        (s < SETUPS).then_some(s)
    }

    /// Setup `setup`'s parameters a kit bakes in, into `out` ([`KitParams`]); false (and
    /// `out` all `NONE`) if none changes the sound: a kit would play as the kit itself.
    #[inline]
    pub fn kit_params(&self, setup: usize, out: &mut KitParams) -> bool {
        let mut any = false;
        for (k, o) in out.iter_mut().enumerate() {
            *o = [NONE; PARAMS];
            if setup >= SETUPS || self.set[setup] >> k & 1 == 0 {
                continue;
            }
            let p = &self.params[setup][k];
            for a in KIT_PARAMS {
                let v = p[a as usize];
                let neutral = match a {
                    LEVEL => v == DEFAULT_LEVEL,
                    // Random pan: the kit's own (a kit can't draw one per note).
                    PAN => v == 0,
                    _ => v == CENTRE as u8,
                };
                if v != NONE && !neutral {
                    o[a as usize] = v;
                    any = true;
                }
            }
        }
        any
    }

    /// The settings a note-on (`m`) on its channel starts with, if its part plays a drum
    /// setup that sets any for the note.
    #[inline]
    pub fn note(&mut self, m: &Msg) -> Option<NoteSettings> {
        if m[0] & 0xF0 != 0x90 || m[2] == 0 {
            return None;
        }
        let setup = self.setup_of[(m[0] & 0x0F) as usize] as usize;
        let key = (m[1] & 0x7F) as usize;
        if setup >= SETUPS || self.set[setup] >> key & 1 == 0 {
            return None;
        }
        let p = self.params[setup][key];
        let pan = (p[PAN as usize] != NONE).then(|| match p[PAN as usize] {
            0 => self.random_pan(),
            v => v as i32,
        });
        Some(NoteSettings::of(&p, pan))
    }

    /// A pan value 1-127 for XG's random pan (Pan 0).
    fn random_pan(&mut self) -> i32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        1 + (x % 127) as i32
    }
}

/// A kit to build as a style loads (kit.rs): part `ch` will play setup `params` on the
/// voice its program change (`msb`, `program`) selects.
#[derive(Clone)]
pub struct Prebuild {
    pub ch: u8,
    pub msb: u8,
    pub program: u8,
    pub params: KitParams,
}

/// The kits a style's setup (`msgs`: its SInt as the engine sends it, each a MIDI message
/// or SysEx) leaves its drum parts playing, as the synth will see them from a fresh start:
/// each drum-setup part whose setup changes the sound, with its program change. Not on the
/// audio thread (it allocates).
pub fn prebuilds<'a>(msgs: impl IntoIterator<Item = &'a [u8]>) -> Vec<Prebuild> {
    let mut d = DrumSetups::new();
    let (mut msb, mut program) = ([0u8; 16], [None::<u8>; 16]);
    for m in msgs {
        match *m {
            [0xF0, ..] => {
                if let Some(x) = encode(m) {
                    d.observe(&x);
                }
            }
            [st, 0, v] if st & 0xF0 == 0xB0 => msb[(st & 0x0F) as usize] = v,
            [st, p, ..] if st & 0xF0 == 0xC0 => {
                program[(st & 0x0F) as usize] = Some(p);
                d.observe(&[st, p, 0]);
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for ch in 0..16u8 {
        let (Some(setup), Some(program)) = (d.setup_of(ch), program[ch as usize]) else { continue };
        let mut params = [[NONE; PARAMS]; 128];
        if d.kit_params(setup, &mut params) && !out.iter().any(|p: &Prebuild| p.ch == ch && p.params == params) {
            out.push(Prebuild { ch, msb: msb[ch as usize], program, params });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn param(setup: u8, note: u8, p: u8, v: u8) -> Msg {
        encode(&[0xF0, 0x43, 0x10, 0x4C, 0x30 + setup, note, p, v, 0xF7]).unwrap()
    }

    fn part_mode(part: u8, mode: u8) -> Msg {
        encode(&[0xF0, 0x43, 0x10, 0x4C, 0x08, part, PART_MODE, mode, 0xF7]).unwrap()
    }

    fn with(msgs: &[Msg]) -> DrumSetups {
        let mut d = DrumSetups::new();
        for m in msgs {
            assert!(is_drum_msg(m) || m[0] >= 0x80);
            d.observe(m);
        }
        d
    }

    #[test]
    fn a_note_starts_with_its_setups_settings() {
        // The corpus layout: Rhythm 1 (ch 9) on DRUMS2, Rhythm 2 (ch 10) on DRUMS1.
        let mut d = with(&[
            part_mode(8, DRUMS2),
            part_mode(9, DRUMS1),
            param(0, 42, LEVEL, 80),
            param(0, 42, COARSE, 0x42),
            param(0, 42, FINE, 0x40 - 25),
            param(0, 42, PAN, 0x40 + 32),
            param(0, 42, REVERB, 0),
            param(0, 42, CUTOFF, 0x40 - 16),
            param(0, 42, RESONANCE, 0x40 + 10),
            param(0, 42, DECAY1, 0x40 + 16),
            param(0, 42, DECAY2, 0x40 - 16),
            param(0, 42, ATTACK, 0x40),
            param(1, 36, LEVEL, 120),
        ]);
        let n = d.note(&[0x99, 42, 100]).expect("tuned note");
        assert!((n.gain - 0.64).abs() < 1e-6);
        assert!((n.tune - 1.75).abs() < 1e-6);
        assert_eq!(n.pan, Some(25.0));
        assert_eq!(n.sends, [0.0, 1.0, 1.0]);
        assert!((n.cutoff - 0.5).abs() < 1e-6);
        assert!((n.resonance_db - 2.0).abs() < 1e-6);
        assert!((n.decay - 0.5).abs() < 1e-6 && (n.release - 2.0).abs() < 1e-6 && n.attack == 1.0);
        let k = d.note(&[0x98, 36, 100]).expect("setup 2's note");
        assert!((k.gain - 1.44).abs() < 1e-6);
        assert_eq!((k.tune, k.pan, k.sends, k.cutoff), (0.0, None, [1.0; 3], 1.0));
        // Other notes, the same note on the other setup's part, melodic parts, note-offs:
        // nothing.
        assert_eq!(d.note(&[0x99, 36, 100]), None);
        assert_eq!(d.note(&[0x98, 42, 100]), None);
        assert_eq!(d.note(&[0x9B, 42, 100]), None);
        assert_eq!(d.note(&[0x99, 42, 0]), None);
        assert_eq!(d.note(&[0x89, 42, 64]), None);
    }

    /// The generator amounts a kit bakes are the settings' numbers exactly: a cutoff or EG
    /// step is 75 cents (16 an octave), a resonance step 2 centibels (0.2 dB), the pan the
    /// same fraction of the range.
    #[test]
    fn key_steps_are_the_settings_in_generator_units() {
        let mut p = [NONE; PARAMS];
        for (a, v) in [(LEVEL, 80), (COARSE, 0x42), (FINE, 0x40 - 25), (PAN, 0x40 + 32), (CUTOFF, 0x40 - 16), (RESONANCE, 0x40 + 10), (DECAY1, 0x40 + 16), (DECAY2, 0x40 - 16)] {
            p[a as usize] = v;
        }
        let (n, s) = (NoteSettings::baked(&p), NoteSettings::steps(&p));
        assert_eq!(s, KeySteps { level: Some(80), coarse: 2, fine: -25, pan: Some(250), cutoff_cents: -1200, resonance_cb: 20, attack_tc: 0, decay_tc: -1200, release_tc: 1200 });
        assert!((n.tune - (s.coarse as f32 + s.fine as f32 / 100.0)).abs() < 1e-6);
        assert_eq!(n.pan.map(|x| x * 10.0), s.pan.map(|x| x as f32));
        assert!((n.cutoff - (s.cutoff_cents as f32 / 1200.0).exp2()).abs() < 1e-6);
        assert!((n.resonance_db * 10.0 - s.resonance_cb as f32).abs() < 1e-4);
        assert!((n.decay - (s.decay_tc as f32 / 1200.0).exp2()).abs() < 1e-6 && (n.release - (s.release_tc as f32 / 1200.0).exp2()).abs() < 1e-6);
        // Random pan: the kit's own.
        p[PAN as usize] = 0;
        assert_eq!((NoteSettings::baked(&p).pan, NoteSettings::steps(&p).pan), (None, None));
    }

    /// A kit bakes only what changes the sound: sends, neutral values and random pan are
    /// left out, and a setup with nothing else needs no kit.
    #[test]
    fn kit_params_leave_out_what_the_kit_plays_anyway() {
        let d = with(&[
            param(0, 38, REVERB, 0),
            param(0, 38, LEVEL, DEFAULT_LEVEL),
            param(0, 38, CUTOFF, 0x40),
            param(0, 40, PAN, 0),
            param(0, 42, LEVEL, 90),
            param(0, 42, CHORUS, 10),
            param(0, 44, PAN, 0x40),
        ]);
        let mut out = [[0u8; PARAMS]; 128];
        assert!(d.kit_params(0, &mut out));
        assert_eq!(out[38], [NONE; PARAMS]);
        assert_eq!(out[40], [NONE; PARAMS]);
        let mut want = [NONE; PARAMS];
        want[LEVEL as usize] = 90;
        assert_eq!(out[42], want);
        assert_eq!(out[44][PAN as usize], 0x40, "centre replaces the kit's own pan");
        let sends_only = with(&[param(0, 38, REVERB, 0), param(0, 36, LEVEL, DEFAULT_LEVEL)]);
        assert!(!sends_only.kit_params(0, &mut out));
        assert!(out.iter().all(|k| *k == [NONE; PARAMS]));
        assert!(!d.kit_params(1, &mut out), "the other setup");
    }

    /// The changes counter moves with every change, and a program change on a part with
    /// nothing set leaves it.
    #[test]
    fn changes_are_counted() {
        let mut d = DrumSetups::new();
        let c = d.changes();
        d.observe(&[0xC9, 0, 0]);
        assert_eq!(d.changes(), c, "nothing to initialize");
        d.observe(&param(0, 38, LEVEL, 50));
        assert_ne!(d.changes(), c);
        let c = d.changes();
        d.observe(&[0xC9, 0, 0]);
        assert_ne!(d.changes(), c);
        assert_eq!((d.setup_of(9), d.setup_of(8)), (Some(0), None));
    }

    /// A style's setup gives its drum parts' kits: the part mode, the program change and
    /// the setup after it (a program change before the setup initializes nothing set
    /// later).
    #[test]
    fn a_styles_setup_gives_its_kits() {
        let sysex = |m: Msg| -> Vec<u8> {
            match m[0] {
                PART => vec![0xF0, 0x43, 0x10, 0x4C, 0x08, m[1], PART_MODE, m[2], 0xF7],
                _ => vec![0xF0, 0x43, 0x10, 0x4C, 0x30 + ((m[0] >> 4) & 1), m[1], m[0] & 0x0F, m[2], 0xF7],
            }
        };
        let msgs: Vec<Vec<u8>> = vec![
            sysex(part_mode(8, DRUMS2)),
            vec![0xB8, 0, 127],
            vec![0xC8, 25, 0],
            vec![0xB9, 0, 127],
            vec![0xC9, 0, 0],
            sysex(param(1, 36, LEVEL, 90)),
            sysex(param(0, 38, REVERB, 20)),
            vec![0x99, 38, 100],
        ];
        let p = prebuilds(msgs.iter().map(|m| &m[..]));
        assert_eq!(p.len(), 1, "setup 1 (ch 10) sets only a send");
        assert_eq!((p[0].ch, p[0].msb, p[0].program), (8, 127, 25));
        assert_eq!(p[0].params[36][LEVEL as usize], 90);
    }

    #[test]
    fn part_ten_plays_drum_setup_1_by_default() {
        let mut d = with(&[param(0, 38, LEVEL, 50)]);
        assert!(d.note(&[0x99, 38, 100]).is_some());
        assert_eq!(d.note(&[0x98, 38, 100]), None);
        d.observe(&part_mode(9, 0));
        assert_eq!(d.note(&[0x99, 38, 100]), None);
    }

    #[test]
    fn a_program_change_or_a_reset_initializes_the_setup() {
        let mut d = with(&[part_mode(8, DRUMS2), param(0, 38, LEVEL, 50), param(1, 38, LEVEL, 50)]);
        // A program change on ch 9 initializes setup 2 only.
        d.observe(&[0xC8, 25, 0]);
        assert_eq!(d.note(&[0x98, 38, 100]), None);
        assert!(d.note(&[0x99, 38, 100]).is_some());
        // A program change on a part without a setup changes nothing.
        d.observe(&[0xC3, 0, 0]);
        assert!(d.note(&[0x99, 38, 100]).is_some());
        // Drum Setup Reset (setup 1).
        d.observe(&encode(&[0xF0, 0x43, 0x10, 0x4C, 0x00, 0x00, 0x7D, 0x00, 0xF7]).unwrap());
        assert_eq!(d.note(&[0x99, 38, 100]), None);
        // XG System On resets the setups and the part modes.
        d.observe(&param(1, 38, LEVEL, 50));
        d.observe(&encode(&[0xF0, 0x43, 0x10, 0x4C, 0x00, 0x00, 0x7E, 0x00, 0xF7]).unwrap());
        d.observe(&param(1, 38, LEVEL, 50));
        assert_eq!(d.note(&[0x98, 38, 100]), None);
    }

    #[test]
    fn random_pan_stays_in_range() {
        let mut d = with(&[param(0, 38, PAN, 0)]);
        let pans: Vec<f32> = (0..200).map(|_| d.note(&[0x99, 38, 100]).unwrap().pan.unwrap()).collect();
        assert!(pans.iter().all(|p| (-50.0..=50.0).contains(p)));
        assert!(pans.iter().any(|&p| p < -10.0) && pans.iter().any(|&p| p > 10.0));
    }

    #[test]
    fn only_drum_setup_sysex_is_encoded() {
        for m in [
            &[0xF0, 0x43, 0x10, 0x4C, 0x30, 0x80, LEVEL, 10, 0xF7][..],
            &[0xF0, 0x43, 0x10, 0x4C, 0x30, 36, 0x70, 10, 0xF7],
            &[0xF0, 0x43, 0x10, 0x4C, 0x00, 0x00, 0x7D, 9, 0xF7],
            &[0xF0, 0x43, 0x10, 0x4C, 0x02, 0x01, 0x00, 0x01, 0x10, 0xF7],
            &[0xF0, 0x43, 0x10, 0x4C, 0x08, 0x09, 0x0B, 100, 0xF7],
            &[0xF0],
            &[],
            &[0x99, 36, 100],
        ] {
            assert_eq!(encode(m), None, "{m:02X?}");
        }
    }
}
