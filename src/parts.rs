//! Keyboard parts, the Genos model: Right 1, Right 2, Right 3 and Left (OM p.48).
//!
//! Each part has a voice, a volume (its CC7), an octave shift and on/off. The Right parts
//! that are on sound together (that is layering) on the right of the split; Left sounds
//! left of it. Every part has its own channel, the same on the `yahaha` port and in the
//! built-in synth, so what the port carries is exactly what sounds:
//!
//!   Right 1 = ch 1, Left = ch 2 (as before the parts model), Right 2 = ch 3, Right 3 = ch 4.
//!
//! The Launchkey faders have two pages, like the Genos Mixer's Panel and Style tabs (OM
//! p.90): Panel = faders 1-4 on Right 1, Right 2, Right 3, Left; Style = the 8 Style parts.
//!
//! Plain atomics: the input thread (notes, faders), the engine thread (CC7 out), the UI
//! thread (OTS, keys) and the synth all read them; none of them waits.

use crate::engine::{Takeover, HW_UNKNOWN};
use std::sync::atomic::{AtomicBool, AtomicI8, AtomicU8, Ordering::{Acquire, Relaxed, Release}};

pub const RIGHT1: usize = 0;
pub const RIGHT2: usize = 1;
pub const RIGHT3: usize = 2;
pub const LEFT: usize = 3;
pub const COUNT: usize = 4;
pub const NAMES: [&str; COUNT] = ["Right 1", "Right 2", "Right 3", "Left"];
/// Each part's MIDI channel (0-based) on the port and in the synth.
pub const CHANNEL: [u8; COUNT] = [0, 2, 3, 1];
/// Default voices (GM programs): Grand Piano, Strings, Brass Section; Left Strings.
pub const DEFAULT_PROGRAMS: [u8; COUNT] = [0, 48, 61, 48];

/// The part on MIDI channel `ch`, if it is a keyboard part's.
pub fn part_of_channel(ch: u8) -> Option<usize> {
    CHANNEL.iter().position(|&c| c == ch)
}

/// `Parts::level`: the volume bits and the pickup bit.
const VOLUME: u8 = 0x7F;
const PICKED: u8 = 0x80;

fn pack(volume: u8, picked: bool) -> u8 {
    volume & VOLUME | (picked as u8) << 7
}

/// What the Launchkey faders 1-8 control (the button under the master fader toggles it).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FaderPage {
    /// Faders 1-4: Right 1, Right 2, Right 3, Left. 5-8 unused.
    #[default]
    Panel,
    /// Faders 1-8: the Style parts.
    Style,
}

pub struct Parts {
    /// GM program per part.
    pub program: [AtomicU8; COUNT],
    pub on: [AtomicBool; COUNT],
    /// The part's volume (its CC7, sent unchanged on its channel) in bits 0-6, and in bit 7
    /// whether its Panel fader controls it (soft takeover). One atomic, so a fader move and
    /// an OTS recall on other threads can never mix one's volume with the other's pickup.
    level: [AtomicU8; COUNT],
    /// Octave shift, -2..=2.
    pub octave: [AtomicI8; COUNT],
    /// The part the voice keys (9/0, Voice -/+ pads) edit.
    pub selected: AtomicU8,
    /// A voice changed: the synth sends the parts' programs again.
    pub changed: AtomicBool,
    /// Manual Bass in effect: the Left part sounds, on the Style's Bass voice (OM p.51).
    pub manual_bass: AtomicBool,
    /// GM program for the current Style's Bass part (see `synth::style_bass_program`).
    pub bass_program: AtomicU8,
    /// Main A-D recall One Touch Settings 1-4.
    pub ots_link: AtomicBool,
    /// Which OTS was applied last (0 = none, 1..=4).
    pub ots_applied: AtomicU8,
    /// `FaderPage::Panel` = 0, `Style` = 1.
    fader_page: AtomicU8,
    /// Where each Launchkey fader 1-8 physically is (`HW_UNKNOWN` until it moves). Faders
    /// are shared by both pages, so a page switch needs this for soft takeover.
    pub fader_hw: [AtomicU8; 8],
    /// The faders went to the Style page: the engine rebinds the Style parts' takeover to
    /// `rebind_hw` on its next wake (`take_rebind`). A flag rather than a command, so a full
    /// ring can never lose it.
    rebind: AtomicBool,
    /// The physical fader positions when the faders went to the Style page.
    rebind_hw: [AtomicU8; 8],
    /// The part soloed (`NO_SOLO`: none): only it sounds, whatever the on/off switches say.
    solo: AtomicU8,
    /// Each part's pan and reverb/chorus sends (CC10, 91, 93; `NO_FX` = not set) as a sound
    /// library patch set them (#103), and the parts whose values the engine thread still
    /// has to send (bit = part).
    fx: [[AtomicU8; 3]; COUNT],
    fx_dirty: AtomicU8,
}

/// `Parts::fx`: not set.
const NO_FX: u8 = 0xFF;
/// The controllers `Parts::fx` holds: pan, reverb send, chorus send.
const FX_CC: [u8; 3] = [10, 91, 93];

/// `Parts::solo`: no part soloed.
pub const NO_SOLO: u8 = 255;

impl Default for Parts {
    fn default() -> Parts {
        Parts::new()
    }
}

impl Parts {
    pub fn new() -> Parts {
        Parts {
            program: DEFAULT_PROGRAMS.map(AtomicU8::new),
            on: [true, false, false, false].map(AtomicBool::new),
            level: [const { AtomicU8::new(100) }; COUNT],
            octave: [const { AtomicI8::new(0) }; COUNT],
            selected: AtomicU8::new(RIGHT1 as u8),
            changed: AtomicBool::new(true),
            manual_bass: AtomicBool::new(false),
            bass_program: AtomicU8::new(33),
            ots_link: AtomicBool::new(false),
            ots_applied: AtomicU8::new(0),
            fader_page: AtomicU8::new(0),
            fader_hw: [const { AtomicU8::new(HW_UNKNOWN) }; 8],
            rebind: AtomicBool::new(false),
            rebind_hw: [const { AtomicU8::new(HW_UNKNOWN) }; 8],
            solo: AtomicU8::new(NO_SOLO),
            fx: [const { [const { AtomicU8::new(NO_FX) }; 3] }; COUNT],
            fx_dirty: AtomicU8::new(0),
        }
    }

    /// A part's pan, reverb and chorus sends from a sound library patch (None: leave it).
    /// The engine thread sends them as CCs on the part's channel, to the port and the synth.
    pub fn set_fx(&self, part: usize, fx: [Option<u8>; 3]) {
        let part = part % COUNT;
        for (a, v) in self.fx[part].iter().zip(fx) {
            if let Some(v) = v {
                a.store(v.min(127), Relaxed);
            }
        }
        self.fx_dirty.fetch_or(1 << part, Release);
    }

    /// Engine thread: send the pan and sends set since the last call.
    pub fn send_fx(&self, out: &mut impl FnMut(&[u8])) {
        let dirty = self.fx_dirty.swap(0, Acquire);
        if dirty == 0 {
            return;
        }
        for p in (0..COUNT).filter(|p| dirty & 1 << p != 0) {
            for (a, cc) in self.fx[p].iter().zip(FX_CC) {
                let v = a.load(Relaxed);
                if v != NO_FX {
                    out(&[0xB0 | CHANNEL[p], cc, v]);
                }
            }
        }
    }

    /// The part soloed, if any.
    pub fn solo(&self) -> Option<usize> {
        let s = self.solo.load(Relaxed);
        (s != NO_SOLO).then_some(s as usize & 3)
    }

    /// Solo a part (only it sounds, even if switched off), or end the solo. Notes already
    /// sounding keep their note-offs (`live::Keys`).
    pub fn set_solo(&self, part: Option<usize>) {
        self.solo.store(part.map_or(NO_SOLO, |p| (p & 3) as u8), Relaxed);
    }

    /// The part sounds for the keys: the soloed part alone, else when it is on.
    pub fn audible(&self, part: usize) -> bool {
        match self.solo() {
            Some(s) => s == part,
            None => self.is_on(part),
        }
    }

    /// The left hand plays the Left part: `left_sounds`, or Left soloed; not while another
    /// part is soloed.
    pub fn left_audible(&self) -> bool {
        match self.solo() {
            Some(s) => s == LEFT,
            None => self.left_sounds(),
        }
    }

    pub fn is_on(&self, part: usize) -> bool {
        self.on[part].load(Relaxed)
    }

    /// Bitmask of the parts that are on (bit = part index).
    pub fn on_mask(&self) -> u8 {
        (0..COUNT).filter(|&p| self.is_on(p)).fold(0, |m, p| m | 1 << p)
    }

    /// The left hand sounds: Left is on, or Manual Bass plays the bass with it.
    pub fn left_sounds(&self) -> bool {
        self.is_on(LEFT) || self.manual_bass.load(Relaxed)
    }

    /// Bitmask of the parts that sound: `on_mask`, with Left lit under Manual Bass too.
    /// What the LEDs and the screen show.
    pub fn sounding_mask(&self) -> u8 {
        self.on_mask() | (self.left_sounds() as u8) << LEFT
    }

    /// Bitmask of the parts the keys play now: `sounding_mask`, except that a solo leaves
    /// the soloed part alone (switched off or not). Where the pedals and wheels go
    /// (`Controllers::sync`).
    pub fn audible_mask(&self) -> u8 {
        match self.solo() {
            Some(s) => 1 << s,
            None => self.sounding_mask(),
        }
    }

    /// Turn a part on or off. Refused for Left while Manual Bass is in effect (false): the
    /// left hand sounds the bass then whatever Left's switch says, so a flip would change
    /// nothing audible or visible.
    pub fn toggle(&self, part: usize) -> bool {
        if part & 3 == LEFT && self.manual_bass.load(Relaxed) {
            return false;
        }
        self.on[part & 3].fetch_xor(true, Relaxed);
        true
    }

    /// The octave shift the part plays at. Under Manual Bass the left hand plays the Style's
    /// Bass voice at the pitch it is played: Left's octave belongs to its own voice (OTS set
    /// it +1/+2 for pads and strings) and is not applied to the bass.
    pub fn octave_of(&self, part: usize) -> i8 {
        if part == LEFT && self.manual_bass.load(Relaxed) {
            0
        } else {
            self.octave[part].load(Relaxed).clamp(-2, 2)
        }
    }

    /// The part's volume (its CC7).
    pub fn volume(&self, part: usize) -> u8 {
        self.level[part].load(Relaxed) & VOLUME
    }

    pub fn select(&self, part: usize) {
        self.selected.store((part & 3) as u8, Relaxed);
    }

    pub fn selected(&self) -> usize {
        self.selected.load(Relaxed) as usize & 3
    }

    /// Set a part's voice (GM program).
    pub fn set_program(&self, part: usize, program: u8) {
        self.program[part & 3].store(program & 127, Relaxed);
        self.changed.store(true, Release);
    }

    /// Previous/next voice for the selected part.
    pub fn step_program(&self, delta: i32) {
        let p = self.selected();
        let v = (self.program[p].load(Relaxed) as i32 + delta).rem_euclid(128) as u8;
        self.program[p].store(v, Relaxed);
        self.changed.store(true, Release);
    }

    /// The program the part's channel plays: under Manual Bass, Left plays the Style's Bass voice.
    pub fn channel_program(&self, part: usize) -> u8 {
        if part == LEFT && self.manual_bass.load(Relaxed) {
            self.bass_program.load(Relaxed)
        } else {
            self.program[part].load(Relaxed)
        }
    }

    pub fn set_manual_bass(&self, on: bool) {
        self.manual_bass.store(on, Relaxed);
        self.changed.store(true, Release);
    }

    /// A new Style is loaded: its Bass voice is what Manual Bass plays.
    pub fn set_bass_program(&self, prog: u8) {
        self.bass_program.store(prog, Relaxed);
        self.changed.store(true, Release);
    }

    /// Set a part's volume from software (OTS): its Panel fader has to pick it up first.
    pub fn set_volume(&self, part: usize, v: u8) {
        let v = v.min(127);
        let hw = self.fader_hw[part].load(Relaxed);
        self.level[part].store(pack(v, Takeover::at(hw, v).picked()), Relaxed);
    }

    /// Panel fader `part` moved from `prev` to `v` (input thread). Soft takeover as for the
    /// Style faders; true if it now controls the part and set its volume.
    pub fn hw_fader(&self, part: usize, prev: u8, v: u8) -> bool {
        let v = v.min(127);
        let mut ok = false;
        // Judged and stored against one reading of volume + pickup: if an OTS recall set
        // the volume meanwhile, the move is judged again against the new level.
        let _ = self.level[part].fetch_update(Relaxed, Relaxed, |l| {
            let mut t = Takeover::resume(prev, l & PICKED != 0);
            ok = t.hardware(l & VOLUME, v);
            Some(pack(if ok { v } else { l & VOLUME }, t.picked()))
        });
        ok
    }

    /// The part's Panel fader has moved but not yet picked the volume up.
    pub fn waiting(&self, part: usize) -> bool {
        self.fader_hw[part].load(Relaxed) != HW_UNKNOWN && self.level[part].load(Relaxed) & PICKED == 0
    }

    pub fn fader_page(&self) -> FaderPage {
        if self.fader_page.load(Relaxed) == 0 {
            FaderPage::Panel
        } else {
            FaderPage::Style
        }
    }

    /// Switch the fader page. The physical faders now control other values: each picks its
    /// new value up only once it gets there. The Panel side is rebound here; the Style side
    /// by the engine (`take_rebind`, then `Engine::faders_at`).
    pub fn set_fader_page(&self, page: FaderPage) {
        let hw = self.fader_hw.each_ref().map(|a| a.load(Relaxed));
        match page {
            FaderPage::Panel => {
                for (level, h) in self.level.iter().zip(hw) {
                    let _ = level.fetch_update(Relaxed, Relaxed, |l| Some(pack(l & VOLUME, Takeover::at(h, l & VOLUME).picked())));
                }
            }
            FaderPage::Style => {
                for (a, h) in self.rebind_hw.iter().zip(hw) {
                    a.store(h, Relaxed);
                }
                self.rebind.store(true, Release);
            }
        }
        self.fader_page.store(page as u8, Relaxed);
    }

    pub fn toggle_fader_page(&self) -> FaderPage {
        let page = match self.fader_page() {
            FaderPage::Panel => FaderPage::Style,
            FaderPage::Style => FaderPage::Panel,
        };
        self.set_fader_page(page);
        page
    }

    /// Engine thread: where the faders were when they last went to the Style page, once.
    pub fn take_rebind(&self) -> Option<[u8; 8]> {
        self.rebind.swap(false, Acquire).then(|| self.rebind_hw.each_ref().map(|a| a.load(Relaxed)))
    }

    /// Load a One Touch Setting into Right 1-3 and Left: voice, on/off, volume, octave.
    /// Drum-kit voices (bank MSB 126/127) keep the part's current voice.
    pub fn apply_ots(&self, ots: &crate::sff::Ots, number: u8) {
        for (p, part) in ots.parts.iter().enumerate() {
            if let Some((_, _, pc)) = part.voice.filter(|v| v.0 < 126) {
                self.program[p].store(pc, Relaxed);
            }
            self.on[p].store(part.on, Relaxed);
            self.set_volume(p, part.volume);
            self.octave[p].store(part.octave, Relaxed);
        }
        self.selected.store(RIGHT1 as u8, Relaxed);
        self.ots_applied.store(number, Relaxed);
        self.changed.store(true, Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels_keep_right_1_and_left_where_they_were() {
        assert_eq!(CHANNEL[RIGHT1], 0);
        assert_eq!(CHANNEL[LEFT], 1);
        assert_eq!((CHANNEL[RIGHT2], CHANNEL[RIGHT3]), (2, 3));
        for (p, &ch) in CHANNEL.iter().enumerate() {
            assert_eq!(part_of_channel(ch), Some(p));
        }
        assert_eq!(part_of_channel(4), None);
        let parts = Parts::new();
        assert_eq!(parts.on_mask(), 0b0001, "Right 1 alone at start, as on the Genos");
        assert_eq!(parts.fader_page(), FaderPage::Panel);
    }

    #[test]
    fn ots_fills_all_four_parts() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return;
        }
        let style = crate::sff::Style::load(&p).unwrap();
        let parts = Parts::new();
        parts.select(LEFT);
        parts.apply_ots(&style.ots[0], 1);
        assert_eq!(parts.on_mask(), 0b1011, "Right 1 + Right 2 + Left");
        assert_eq!(parts.program[RIGHT1].load(Relaxed), 80);
        assert_eq!(parts.program[RIGHT2].load(Relaxed), 94);
        assert_eq!(parts.program[LEFT].load(Relaxed), 52);
        assert_eq!(parts.volume(LEFT), 40);
        assert_eq!(parts.selected(), RIGHT1);
        assert_eq!(parts.ots_applied.load(Relaxed), 1);
        for (i, o) in style.ots[0].parts.iter().enumerate() {
            assert_eq!(parts.volume(i), o.volume);
            assert_eq!(parts.octave[i].load(Relaxed), o.octave);
        }
    }

    #[test]
    fn manual_bass_gives_left_the_bass_voice() {
        let parts = Parts::new();
        parts.set_bass_program(35);
        assert!(!parts.left_sounds());
        assert_eq!(parts.channel_program(LEFT), 48);
        assert_eq!(parts.sounding_mask(), 0b0001);
        parts.set_manual_bass(true);
        assert!(parts.left_sounds(), "Manual Bass sounds the left hand with Left off");
        assert_eq!(parts.channel_program(LEFT), 35);
        assert_eq!(parts.channel_program(RIGHT1), 0, "only Left is affected");
        assert_eq!((parts.on_mask(), parts.sounding_mask()), (0b0001, 0b1001), "the LEDs show Left sounding");
    }

    #[test]
    fn voice_keys_edit_the_selected_part() {
        let parts = Parts::new();
        parts.select(RIGHT3);
        parts.changed.store(false, Relaxed);
        parts.step_program(1);
        assert_eq!(parts.program[RIGHT3].load(Relaxed), 62);
        assert!(parts.changed.load(Relaxed));
        parts.step_program(-63);
        assert_eq!(parts.program[RIGHT3].load(Relaxed), 127, "wraps");
        assert_eq!(parts.program[RIGHT1].load(Relaxed), 0);
        parts.toggle(RIGHT3);
        parts.toggle(LEFT);
        assert_eq!(parts.on_mask(), 0b1101);
    }

    /// Under Manual Bass the left hand plays the bass at the pitch played, whatever octave
    /// an OTS gave Left, and Left's switch is refused (it would change nothing).
    #[test]
    fn manual_bass_ignores_left_octave_and_switch() {
        let parts = Parts::new();
        parts.octave[LEFT].store(1, Relaxed);
        parts.octave[RIGHT1].store(1, Relaxed);
        assert_eq!(parts.octave_of(LEFT), 1);
        parts.set_manual_bass(true);
        assert_eq!((parts.octave_of(LEFT), parts.octave_of(RIGHT1)), (0, 1));
        assert!(!parts.toggle(LEFT));
        assert!(!parts.is_on(LEFT));
        assert!(parts.toggle(RIGHT2), "the Right parts still switch");
        parts.set_manual_bass(false);
        assert_eq!(parts.octave_of(LEFT), 1, "Left's own voice gets its octave back");
        assert!(parts.toggle(LEFT));
        assert!(parts.is_on(LEFT));
    }

    /// A fader moving on the input thread while an OTS recall sets the level on the UI
    /// thread: the recall is never overwritten by a move judged against the old level.
    #[test]
    fn fader_move_never_overwrites_a_concurrent_recall() {
        for _ in 0..200 {
            let parts = std::sync::Arc::new(Parts::new());
            parts.set_volume(RIGHT1, 10);
            parts.fader_hw[RIGHT1].store(10, Relaxed);
            parts.set_fader_page(FaderPage::Panel);
            assert!(!parts.waiting(RIGHT1));
            let stop = std::sync::Arc::new(AtomicBool::new(false));
            let mover = {
                let (parts, stop) = (parts.clone(), stop.clone());
                std::thread::spawn(move || {
                    let mut v = 5;
                    while !stop.load(Relaxed) {
                        v = if v >= 15 { 5 } else { v + 1 };
                        let prev = parts.fader_hw[RIGHT1].swap(v, Relaxed);
                        parts.hw_fader(RIGHT1, prev, v);
                    }
                })
            };
            std::thread::yield_now();
            parts.set_volume(RIGHT1, 90);
            std::thread::yield_now();
            stop.store(true, Relaxed);
            mover.join().unwrap();
            assert_eq!(parts.volume(RIGHT1), 90, "a fader between 5 and 15 never reaches 90");
            assert!(parts.waiting(RIGHT1));
        }
    }

    /// Panel faders: soft takeover after an OTS recall and across page switches.
    #[test]
    fn panel_faders_take_over_softly() {
        let parts = Parts::new();
        // A fader that never reported must reach the value first.
        let mv = |f: usize, v: u8| {
            let prev = parts.fader_hw[f].swap(v, Relaxed);
            parts.hw_fader(f, prev, v)
        };
        assert!(!mv(RIGHT2, 30));
        assert!(parts.waiting(RIGHT2));
        assert!(mv(RIGHT2, 110), "crossed 100: picked up");
        assert_eq!(parts.volume(RIGHT2), 110);
        assert!(mv(RIGHT2, 60));
        assert!(!parts.waiting(RIGHT2));
        // OTS moves it: the fader waits for the new level.
        parts.set_volume(RIGHT2, 90);
        assert!(parts.waiting(RIGHT2));
        assert!(!mv(RIGHT2, 70));
        assert_eq!(parts.volume(RIGHT2), 90);
        assert!(mv(RIGHT2, 89));
        // On the Style page the fader goes to 10; back on Panel it must pick 89 up again.
        parts.set_fader_page(FaderPage::Style);
        let mut hw = [HW_UNKNOWN; 8];
        hw[RIGHT2] = 89;
        assert_eq!(parts.take_rebind(), Some(hw), "the engine rebinds the Style faders once");
        assert_eq!(parts.take_rebind(), None);
        parts.fader_hw[RIGHT2].store(10, Relaxed);
        assert_eq!(parts.toggle_fader_page(), FaderPage::Panel);
        assert_eq!(parts.take_rebind(), None, "nothing for the engine going to Panel");
        assert!(parts.waiting(RIGHT2));
        assert!(!mv(RIGHT2, 12), "no jump from 89 to 12");
        assert_eq!(parts.volume(RIGHT2), 89);
        // A fader already at its value on the new page keeps control.
        parts.fader_hw[RIGHT1].store(101, Relaxed);
        parts.set_fader_page(FaderPage::Panel);
        assert!(!parts.waiting(RIGHT1));
        assert!(mv(RIGHT1, 105));
    }
}
