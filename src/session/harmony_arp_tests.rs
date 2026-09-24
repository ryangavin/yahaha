//! Keyboard Harmony / Arpeggio through an offline session: the processor on the input
//! thread, the engine thread's Echo, arpeggio and Strum, and randomized play that must
//! never leave a note sounding.

use crate::api::{ArpQuantize, ArpVelocityMode, ChordCmd, HarmonyArpCmd, HarmonyAssign, HarmonySpeed, LibraryCmd, PartsCmd, SystemCmd, TransportCmd};
use crate::arp::library::PATTERNS;
use crate::harmony::{HarmonyType, ALL_TYPES};
use crate::live::type_index;
use crate::session::{Options, Port, Session};
use std::path::{Path, PathBuf};

const MS: u64 = 1_000_000;

fn style(name: &str) -> Option<PathBuf> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2").join(name);
    if p.exists() {
        Some(p)
    } else {
        eprintln!("corpus missing; skipping");
        None
    }
}

fn session(names: &[&str]) -> Option<Session> {
    let paths = names.iter().map(|n| style(n)).collect::<Option<Vec<_>>>()?;
    let s = Session::offline(Options { paths, ..Options::default() }).unwrap();
    s.finish_indexing();
    Some(s)
}

/// What sounds on the keyboard parts' channels (1-4), from everything played so far, in
/// order: the input thread's messages of a `midi_in` before the engine's.
struct Heard {
    on: [[bool; 128]; 16],
    /// Every note-on, as (channel, note, velocity), since the last `clear`.
    ons: Vec<(u8, u8, u8)>,
    /// Every note-off, as (channel, note).
    offs: Vec<(u8, u8)>,
}

impl Default for Heard {
    fn default() -> Heard {
        Heard { on: [[false; 128]; 16], ons: Vec::new(), offs: Vec::new() }
    }
}

impl Heard {
    fn take(&mut self, s: &Session) {
        let (band, keys) = s.take_output_split();
        for m in keys.iter().chain(&band) {
            let ch = m[0] & 0x0F;
            match m[0] & 0xF0 {
                0x90 if m[2] > 0 => {
                    self.on[ch as usize][m[1] as usize] = true;
                    if ch < 4 {
                        self.ons.push((ch, m[1], m[2]));
                    }
                }
                0x80 | 0x90 => {
                    self.on[ch as usize][m[1] as usize] = false;
                    if ch < 4 {
                        self.offs.push((ch, m[1]));
                    }
                }
                0xB0 if m[1] == 123 || m[1] == 120 => self.on[ch as usize] = [false; 128],
                _ => {}
            }
        }
    }

    fn clear(&mut self) {
        self.ons.clear();
        self.offs.clear();
    }

    /// The keyboard parts' notes still sounding, as (channel, note).
    fn sounding(&self) -> Vec<(u8, u8)> {
        (0..4u8).flat_map(|ch| (0..128u8).filter(move |&n| self.on[ch as usize][n as usize]).map(move |n| (ch, n))).collect()
    }
}

fn key(s: &Session, h: &mut Heard, k: u8, vel: u8) {
    s.midi_in(Port::Keys, &[if vel > 0 { 0x90 } else { 0x80 }, k, vel]);
    h.take(s);
}

fn send(s: &Session, h: &mut Heard, c: impl Into<crate::api::AppCmd>) {
    let _ = s.send(c);
    h.take(s);
}

fn advance(s: &Session, h: &mut Heard, ns: u64) {
    s.advance(ns);
    h.take(s);
}

fn harmony_type(t: HarmonyType) -> HarmonyArpCmd {
    HarmonyArpCmd::SetHarmonyType { index: type_index(t) }
}

fn pattern(name: &str) -> HarmonyArpCmd {
    HarmonyArpCmd::SetArpPattern { index: PATTERNS.iter().position(|p| p.name == name).unwrap() as u8 }
}

/// A C chord in the left hand (the band starts on it: Sync Start).
fn c_chord(s: &Session, h: &mut Heard) {
    for k in [48, 52, 43] {
        key(s, h, k, 90);
    }
}

#[test]
fn harmony_adds_a_note_below_the_melody_and_stops_it() {
    let Some(s) = session(&["SlowWalker.T552.sty"]) else { return };
    let mut h = Heard::default();
    send(&s, &mut h, harmony_type(HarmonyType::StandardDuet1));
    send(&s, &mut h, HarmonyArpCmd::SetHarmonyArpOn { on: true });
    assert!(s.state().harmony_arp.on);
    c_chord(&s, &mut h);
    h.clear();
    key(&s, &mut h, 72, 100);
    // C5 over C: the nearest chord tone below is G4, at Volume 100 of the key's 100.
    assert_eq!(h.ons, [(0, 67, 79), (0, 72, 100)]);
    // A lower right-hand key is not the melody: it plays plainly.
    key(&s, &mut h, 64, 100);
    assert_eq!(h.ons.len(), 3);
    key(&s, &mut h, 64, 0);
    key(&s, &mut h, 72, 0);
    assert!(h.offs.contains(&(0, 67)) && h.offs.contains(&(0, 72)));
    assert!(h.sounding().is_empty());
    // The chord section is never harmonised.
    h.clear();
    key(&s, &mut h, 40, 100);
    assert!(h.ons.iter().all(|&(ch, n, _)| ch != 0 || n == 40), "{:?}", h.ons);
}

/// A harmony note and a key on the same pitch are counted together: releasing one does
/// not cut the other (review #76, item 9).
#[test]
fn a_harmony_note_shared_with_a_key_is_not_cut() {
    let Some(s) = session(&["SlowWalker.T552.sty"]) else { return };
    let mut h = Heard::default();
    send(&s, &mut h, HarmonyArpCmd::SetHarmonyArpOn { on: true });
    c_chord(&s, &mut h);
    key(&s, &mut h, 72, 100); // adds G4 (67)
    key(&s, &mut h, 67, 100); // G4 as a key of its own
    h.clear();
    key(&s, &mut h, 72, 0);
    assert_eq!(h.offs, [(0, 72)], "G4 is still held by its key");
    key(&s, &mut h, 67, 0);
    assert_eq!(h.offs, [(0, 72), (0, 67)]);
    assert!(h.sounding().is_empty());
}

/// With both ACMP and LEFT as yahaha has them (ACMP always on), the harmony follows the
/// Style's chord, whichever the LEFT part is; Chord Note Only leaves passing notes alone.
#[test]
fn harmony_follows_the_style_chord_and_chord_note_only() {
    let Some(s) = session(&["SlowWalker.T552.sty"]) else { return };
    let mut h = Heard::default();
    send(&s, &mut h, HarmonyArpCmd::SetHarmonyArpOn { on: true });
    send(&s, &mut h, PartsCmd::SetPartOn { part: 3, on: true });
    for k in [45, 48, 52] {
        key(&s, &mut h, k, 90); // A minor
    }
    h.clear();
    key(&s, &mut h, 76, 100); // E5 over Am: C5 below
    assert_eq!(h.ons.iter().filter(|o| o.0 == 0).map(|o| o.1).collect::<Vec<_>>(), [72, 76]);
    key(&s, &mut h, 76, 0);
    send(&s, &mut h, HarmonyArpCmd::SetChordNoteOnly { on: true });
    h.clear();
    key(&s, &mut h, 74, 100); // D5 is not in Am: no harmony
    assert_eq!(h.ons, [(0, 74, 100)]);
    key(&s, &mut h, 74, 0);
    // Touch Limit: a soft key gets no harmony.
    send(&s, &mut h, HarmonyArpCmd::SetChordNoteOnly { on: false });
    send(&s, &mut h, HarmonyArpCmd::SetTouchLimit { velocity: 80 });
    h.clear();
    key(&s, &mut h, 76, 60);
    assert_eq!(h.ons, [(0, 76, 60)]);
}

#[test]
fn multi_assign_spreads_the_keys_over_the_right_parts() {
    let Some(s) = session(&["SlowWalker.T552.sty"]) else { return };
    let mut h = Heard::default();
    send(&s, &mut h, PartsCmd::SetPartOn { part: 1, on: true });
    send(&s, &mut h, PartsCmd::SetPartOn { part: 2, on: true });
    send(&s, &mut h, harmony_type(HarmonyType::MultiAssign));
    send(&s, &mut h, HarmonyArpCmd::SetHarmonyArpOn { on: true });
    for k in [60, 64, 67] {
        key(&s, &mut h, k, 100);
    }
    // Right 1 = ch 1, Right 2 = ch 3, Right 3 = ch 4.
    assert_eq!(h.ons, [(0, 60, 100), (2, 64, 100), (3, 67, 100)]);
    for k in [60, 64, 67] {
        key(&s, &mut h, k, 0);
    }
    assert!(h.sounding().is_empty());
}

#[test]
fn echo_repeats_in_time_until_the_key_goes_up() {
    let Some(s) = session(&["SlowWalker.T552.sty"]) else { return };
    let mut h = Heard::default();
    send(&s, &mut h, harmony_type(HarmonyType::Tremolo));
    send(&s, &mut h, HarmonyArpCmd::SetHarmonySpeed { speed: HarmonySpeed::Eighth });
    send(&s, &mut h, HarmonyArpCmd::SetHarmonyArpOn { on: true });
    let bpm = s.state().transport.tempo;
    key(&s, &mut h, 72, 100);
    assert_eq!(h.ons, [(0, 72, 100)], "the struck note at once, from the engine thread");
    advance(&s, &mut h, 2_000 * MS);
    let eighths = (2.0 * bpm / 60.0 * 2.0) as usize;
    let n = h.ons.len();
    assert!((eighths..=eighths + 1).contains(&n), "{n} pulses in 2 s at {bpm} BPM");
    key(&s, &mut h, 72, 0);
    advance(&s, &mut h, 1_000 * MS);
    assert_eq!(h.ons.len(), n, "no repeat after release");
    assert!(h.sounding().is_empty());
}

#[test]
fn strum_notes_come_after_the_melody_and_end_with_it() {
    let Some(s) = session(&["SlowWalker.T552.sty"]) else { return };
    let mut h = Heard::default();
    send(&s, &mut h, harmony_type(HarmonyType::Strum));
    send(&s, &mut h, HarmonyArpCmd::SetHarmonyArpOn { on: true });
    c_chord(&s, &mut h);
    h.clear();
    key(&s, &mut h, 72, 100);
    let at_once = h.ons.clone();
    advance(&s, &mut h, 100 * MS);
    assert!(h.ons.len() > at_once.len(), "the later strings come after: {at_once:?} then {:?}", h.ons);
    key(&s, &mut h, 72, 0);
    assert!(h.sounding().is_empty(), "{:?}", h.sounding());
    // Released before the strum finished: the rest never starts.
    h.clear();
    key(&s, &mut h, 72, 100);
    key(&s, &mut h, 72, 0);
    advance(&s, &mut h, 100 * MS);
    assert!(h.sounding().is_empty());
    assert!(h.ons.len() <= 2, "{:?}", h.ons);
}

/// The arpeggio swallows the right-hand keys and plays its pattern on its own clock with
/// the band stopped, then on the style clock once it starts.
#[test]
fn arpeggio_runs_stopped_and_with_the_band() {
    let Some(s) = session(&["SlowWalker.T552.sty"]) else { return };
    let mut h = Heard::default();
    send(&s, &mut h, pattern("Climb 16"));
    send(&s, &mut h, HarmonyArpCmd::SetHarmonyArpOn { on: true });
    for k in [60, 64, 67] {
        key(&s, &mut h, k, 100);
    }
    assert!(!s.state().transport.running);
    advance(&s, &mut h, 1_000 * MS);
    let bpm = s.state().transport.tempo;
    let sixteenths = (bpm / 60.0 * 4.0) as usize;
    let ch0: Vec<u8> = h.ons.iter().filter(|o| o.0 == 0).map(|o| o.1).collect();
    assert!((sixteenths - 1..=sixteenths + 1).contains(&ch0.len()), "{} notes in 1 s at {bpm} BPM", ch0.len());
    assert_eq!(ch0[..4], [60, 64, 67, 60], "Climb 16 walks up");
    // The band starts (a chord): the pattern restarts with it, on its grid.
    h.clear();
    c_chord(&s, &mut h);
    assert!(s.state().transport.running);
    advance(&s, &mut h, 1_000 * MS);
    let ch0: Vec<u8> = h.ons.iter().filter(|o| o.0 == 0).map(|o| o.1).collect();
    assert_eq!(ch0[..3], [60, 64, 67], "from step 1 at the start: {ch0:?}");
    // Hold: the pattern plays on after release, until Hold goes off.
    send(&s, &mut h, HarmonyArpCmd::SetArpHold { on: true });
    for k in [60, 64, 67] {
        key(&s, &mut h, k, 0);
    }
    h.clear();
    advance(&s, &mut h, 500 * MS);
    assert!(!h.ons.is_empty(), "latched");
    send(&s, &mut h, HarmonyArpCmd::SetArpHold { on: false });
    advance(&s, &mut h, 500 * MS);
    h.clear();
    advance(&s, &mut h, 500 * MS);
    assert!(h.ons.iter().all(|o| o.0 != 0), "stopped: {:?}", h.ons);
    assert!(h.sounding().is_empty());
}

/// Turning the switch off stops the arpeggio at once, notes and all.
#[test]
fn the_switch_off_stops_the_arpeggio_now() {
    let Some(s) = session(&["SlowWalker.T552.sty"]) else { return };
    let mut h = Heard::default();
    send(&s, &mut h, pattern("Gated Pad 16"));
    send(&s, &mut h, HarmonyArpCmd::SetHarmonyArpOn { on: true });
    for k in [60, 64, 67] {
        key(&s, &mut h, k, 100);
    }
    advance(&s, &mut h, 300 * MS);
    send(&s, &mut h, HarmonyArpCmd::ToggleHarmonyArp);
    assert!(h.sounding().is_empty(), "{:?}", h.sounding());
    h.clear();
    advance(&s, &mut h, 300 * MS);
    assert!(h.ons.is_empty());
    for k in [60, 64, 67] {
        key(&s, &mut h, k, 0);
    }
}

/// PANIC with a held arpeggio under the sustain pedal (#93's controllers and this
/// processor meet in the engine's Panic): the arpeggio's notes stop and it plays no more,
/// and the pedal switches are released too.
#[test]
fn panic_stops_a_held_arpeggio_and_the_pedals() {
    let Some(s) = session(&["SlowWalker.T552.sty"]) else { return };
    let mut h = Heard::default();
    send(&s, &mut h, pattern("Gated Pad 16"));
    send(&s, &mut h, HarmonyArpCmd::SetArpHold { on: true });
    send(&s, &mut h, HarmonyArpCmd::SetHarmonyArpOn { on: true });
    s.midi_in(Port::Keys, &[0xB0, 64, 127]);
    h.take(&s);
    assert!(s.state().controllers.sustain);
    for k in [60, 64, 67] {
        key(&s, &mut h, k, 100);
    }
    for k in [60, 64, 67] {
        key(&s, &mut h, k, 0);
    }
    h.clear();
    advance(&s, &mut h, 400 * MS);
    assert!(!h.ons.is_empty(), "Arp Hold: the pattern plays on after the keys go up");
    send(&s, &mut h, SystemCmd::Panic);
    advance(&s, &mut h, MS);
    assert!(h.sounding().is_empty(), "{:?}", h.sounding());
    assert!(!s.state().controllers.sustain, "PANIC releases the pedal switches");
    h.clear();
    advance(&s, &mut h, 600 * MS);
    assert!(h.ons.is_empty(), "the held arpeggio is gone: {:?}", h.ons);
    assert!(s.state().harmony_arp.on, "PANIC leaves the settings as they are");
}

/// A pedal on Arpeggio Hold or Kbd Harmony/Arpeggio (Hold A or Hold B) held down at PANIC
/// or when its keyboard is unplugged (`Cmd::KeysOff`): the reset lets the pedal go, so
/// the switch it held goes where the pedal being up puts it, and the arpeggio stops (RM
/// p.141: "stops when this function is turned off"). Releasing the pedal afterwards
/// changes nothing.
#[test]
fn a_reset_lets_go_of_what_a_hold_pedal_held() {
    use crate::api::ControllersCmd;
    use crate::controllers::{ControlType, Function};
    use crate::live::Cmd;
    let pedal = |function, control_type| ControllersCmd::SetPedal { pedal: 1, cc: Some(66), function, control_type, reverse: false, range: Default::default() };
    let panic = |s: &Session, h: &mut Heard| send(s, h, SystemCmd::Panic);
    let keys_off = |s: &Session, h: &mut Heard| {
        let _ = s.inner.lock().engine_cmd(Cmd::KeysOff);
        s.settle();
        h.take(s);
    };
    for (name, reset) in [("PANIC", &panic as &dyn Fn(&Session, &mut Heard)), ("KeysOff", &keys_off)] {
        let Some(s) = session(&["SlowWalker.T552.sty"]) else { return };
        let mut h = Heard::default();
        // Arpeggio Hold on Hold A, down: the pattern plays on after the keys go up.
        send(&s, &mut h, pattern("Climb 16"));
        send(&s, &mut h, HarmonyArpCmd::SetHarmonyArpOn { on: true });
        send(&s, &mut h, pedal(Function::ArpHold, ControlType::HoldA));
        s.midi_in(Port::Keys, &[0xB0, 66, 127]);
        assert!(s.state().harmony_arp.arp.pedal_hold);
        for k in [60, 64, 67] {
            key(&s, &mut h, k, 100);
        }
        for k in [60, 64, 67] {
            key(&s, &mut h, k, 0);
        }
        h.clear();
        advance(&s, &mut h, 400 * MS);
        assert!(!h.ons.is_empty(), "{name}: held by the pedal");
        reset(&s, &mut h);
        let st = s.state();
        assert!(!st.harmony_arp.arp.pedal_hold, "{name}: the pedal counts as up, so Arpeggio Hold is off");
        assert!(!st.controllers.pedals[1].down);
        s.midi_in(Port::Keys, &[0xB0, 66, 0]);
        h.take(&s);
        h.clear();
        advance(&s, &mut h, 1_000 * MS);
        assert!(h.ons.is_empty(), "{name}: pedal and keys up, nothing plays: {:?}", h.ons);
        assert!(!s.state().harmony_arp.arp.pedal_hold);
        // Hold B, the mirror: up holds it on, so the reset turns it off. As with a Hold B
        // pedal switch, it stays off until the pedal is next pressed and released: a reset
        // never turns anything on.
        send(&s, &mut h, pedal(Function::ArpHold, ControlType::HoldB));
        assert!(s.state().harmony_arp.arp.pedal_hold, "{name}: Hold B, up: on");
        for k in [60, 64, 67] {
            key(&s, &mut h, k, 100);
        }
        for k in [60, 64, 67] {
            key(&s, &mut h, k, 0);
        }
        reset(&s, &mut h);
        assert!(!s.state().harmony_arp.arp.pedal_hold, "{name}: Hold B, up at the reset: off");
        h.clear();
        advance(&s, &mut h, 1_000 * MS);
        assert!(h.ons.is_empty(), "{name}: {:?}", h.ons);
        s.midi_in(Port::Keys, &[0xB0, 66, 127]);
        assert!(!s.state().harmony_arp.arp.pedal_hold);
        s.midi_in(Port::Keys, &[0xB0, 66, 0]);
        assert!(s.state().harmony_arp.arp.pedal_hold, "{name}: pressed and released: on again");
        // Hold B down at the reset (off): released afterwards, no edge, still off.
        s.midi_in(Port::Keys, &[0xB0, 66, 127]);
        reset(&s, &mut h);
        s.midi_in(Port::Keys, &[0xB0, 66, 0]);
        assert!(!s.state().harmony_arp.arp.pedal_hold, "{name}: Hold B down at the reset stays off");
        // The Hold setting is not the pedal's: a reset leaves it.
        send(&s, &mut h, HarmonyArpCmd::SetArpHold { on: true });
        reset(&s, &mut h);
        assert!(s.state().harmony_arp.arp.hold, "{name}: the Hold setting stays");
        send(&s, &mut h, HarmonyArpCmd::SetArpHold { on: false });
        // Kbd Harmony/Arpeggio on Hold A, down: the switch goes off with the reset.
        send(&s, &mut h, pedal(Function::KbdHarmonyArp, ControlType::HoldA));
        assert!(!s.state().harmony_arp.on, "Hold A picked with the pedal up: off");
        s.midi_in(Port::Keys, &[0xB0, 66, 127]);
        assert!(s.state().harmony_arp.on);
        reset(&s, &mut h);
        assert!(!s.state().harmony_arp.on, "{name}: Kbd Harmony/Arpeggio off with the pedal");
        s.midi_in(Port::Keys, &[0xB0, 66, 0]);
        assert!(!s.state().harmony_arp.on);
        // A Toggle pedal switched it: the reset leaves it.
        send(&s, &mut h, pedal(Function::KbdHarmonyArp, ControlType::Toggle));
        s.midi_in(Port::Keys, &[0xB0, 66, 127]);
        assert!(s.state().harmony_arp.on);
        reset(&s, &mut h);
        assert!(s.state().harmony_arp.on, "{name}: a Toggle pedal's switch stays");
        s.midi_in(Port::Keys, &[0xB0, 66, 0]);
        send(&s, &mut h, HarmonyArpCmd::SetHarmonyArpOn { on: false });
        assert!(h.sounding().is_empty(), "{name}: {:?}", h.sounding());
    }
}

/// The Arpeggio Hold pedal (RM p.141) and the Hold setting (RM p.41) are two things: the
/// pedal never changes the setting, and the arpeggio holds while either is on.
#[test]
fn the_hold_pedal_leaves_the_hold_setting() {
    use crate::api::ControllersCmd;
    use crate::controllers::{ControlType, Function};
    let Some(s) = session(&["SlowWalker.T552.sty"]) else { return };
    let mut h = Heard::default();
    let pedal = ControllersCmd::SetPedal { pedal: 1, cc: Some(66), function: Function::ArpHold, control_type: ControlType::HoldA, reverse: false, range: Default::default() };
    send(&s, &mut h, pedal);
    send(&s, &mut h, pattern("Climb 16"));
    send(&s, &mut h, HarmonyArpCmd::SetHarmonyArpOn { on: true });
    send(&s, &mut h, HarmonyArpCmd::SetArpHold { on: true });
    s.midi_in(Port::Keys, &[0xB0, 66, 127]);
    s.midi_in(Port::Keys, &[0xB0, 66, 0]);
    let st = s.state();
    assert!(st.harmony_arp.arp.hold, "a press and release of the pedal leaves the setting on");
    assert!(!st.harmony_arp.arp.pedal_hold);
    // The setting holds the pattern with the pedal up.
    for k in [60, 64, 67] {
        key(&s, &mut h, k, 100);
    }
    for k in [60, 64, 67] {
        key(&s, &mut h, k, 0);
    }
    h.clear();
    advance(&s, &mut h, 500 * MS);
    assert!(!h.ons.is_empty(), "held by the setting");
    // The setting off, the pedal down: held by the pedal, and it stops when the pedal
    // goes up.
    send(&s, &mut h, HarmonyArpCmd::SetArpHold { on: false });
    s.midi_in(Port::Keys, &[0xB0, 66, 127]);
    for k in [62, 65, 69] {
        key(&s, &mut h, k, 100);
    }
    for k in [62, 65, 69] {
        key(&s, &mut h, k, 0);
    }
    h.clear();
    advance(&s, &mut h, 500 * MS);
    assert!(!h.ons.is_empty(), "held by the pedal");
    assert!(!s.state().harmony_arp.arp.hold, "the pedal did not turn the setting on");
    s.midi_in(Port::Keys, &[0xB0, 66, 0]);
    h.take(&s);
    advance(&s, &mut h, 500 * MS);
    h.clear();
    advance(&s, &mut h, 500 * MS);
    assert!(h.ons.iter().all(|o| o.0 != 0), "the pedal up stops it: {:?}", h.ons);
    assert!(h.sounding().is_empty(), "{:?}", h.sounding());
}

/// A style with another resolution takes over at the bar line: the arpeggio re-times
/// (`Arp::set_ppq`) and plays on at the same tempo.
#[test]
fn the_arpeggio_follows_a_style_change_to_another_resolution() {
    let Some(s) = session(&["SlowWalker.T552.sty", "TickingAway.T162.sty"]) else { return };
    let mut h = Heard::default();
    send(&s, &mut h, pattern("Climb 16"));
    send(&s, &mut h, HarmonyArpCmd::SetHarmonyArpOn { on: true });
    c_chord(&s, &mut h);
    for k in [60, 64, 67] {
        key(&s, &mut h, k, 100);
    }
    advance(&s, &mut h, 1_000 * MS);
    let other = s.library_list().entries.iter().find(|e| e.path.contains("TickingAway")).unwrap().id;
    send(&s, &mut h, LibraryCmd::LoadStyle { id: other });
    advance(&s, &mut h, 5_000 * MS);
    assert!(s.state().style.path.contains("TickingAway"));
    h.clear();
    let bpm = s.state().transport.tempo;
    advance(&s, &mut h, 1_000 * MS);
    let n = h.ons.iter().filter(|o| o.0 == 0).count();
    let sixteenths = (bpm / 60.0 * 4.0) as usize;
    assert!((sixteenths - 1..=sixteenths + 1).contains(&n), "{n} notes in 1 s at {bpm} BPM after the change");
    for k in [60, 64, 67] {
        key(&s, &mut h, k, 0);
    }
    advance(&s, &mut h, 1_000 * MS);
    assert!(h.sounding().is_empty());
}

/// A seeded random number generator (xorshift), for the randomized play.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
}

/// Randomized play: keys in both hands, every Harmony type and arpeggio, settings,
/// transpose, parts and octaves changing, the band starting and stopping, style changes
/// across resolutions, PANIC. Once every key is up and Hold and Keep Key On are off,
/// nothing may be left sounding on the keyboard parts.
#[test]
fn randomized_play_leaves_no_note_stuck() {
    let Some(s) = session(&["SlowWalker.T552.sty", "TickingAway.T162.sty"]) else { return };
    let ids: Vec<usize> = s.library_list().entries.iter().map(|e| e.id).collect();
    let mut h = Heard::default();
    for seed in 1..=12u64 {
        let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let mut down = [false; 128];
        send(&s, &mut h, HarmonyArpCmd::SetHarmonyArpOn { on: true });
        for _ in 0..600 {
            match r.below(100) {
                0..=54 => {
                    let k = 36 + r.below(60) as u8;
                    if down[k as usize] && r.below(3) > 0 {
                        down[k as usize] = false;
                        key(&s, &mut h, k, 0);
                    } else {
                        down[k as usize] = true;
                        key(&s, &mut h, k, 1 + r.below(127) as u8);
                    }
                }
                55..=74 => advance(&s, &mut h, r.below(80) * MS),
                75..=79 => {
                    let n = (ALL_TYPES.len() + PATTERNS.len()) as u64;
                    send(&s, &mut h, HarmonyArpCmd::StepHarmonyArpType { delta: (r.below(n) as i8) - 10 });
                }
                80 => send(&s, &mut h, HarmonyArpCmd::ToggleHarmonyArp),
                81 => send(&s, &mut h, HarmonyArpCmd::ToggleArpHold),
                82 => send(&s, &mut h, HarmonyArpCmd::SetArpKeepKeyOn { on: r.below(2) == 0 }),
                83 => {
                    let q = [ArpQuantize::Off, ArpQuantize::Eighth, ArpQuantize::Sixteenth][r.below(3) as usize];
                    send(&s, &mut h, HarmonyArpCmd::SetArpQuantize { quantize: q });
                }
                84 => {
                    let a = [HarmonyAssign::Auto, HarmonyAssign::Multi, HarmonyAssign::Right1, HarmonyAssign::Right2, HarmonyAssign::Right3][r.below(5) as usize];
                    send(&s, &mut h, HarmonyArpCmd::SetHarmonyAssign { assign: a });
                }
                85 => {
                    let m = [ArpVelocityMode::Original, ArpVelocityMode::Thru, ArpVelocityMode::Fixed][r.below(3) as usize];
                    send(&s, &mut h, HarmonyArpCmd::SetArpVelocity { mode: m, velocity: 1 + r.below(127) as u8 });
                }
                86 => send(&s, &mut h, HarmonyArpCmd::SetHarmonyVolume { volume: r.below(128) as u8 }),
                87 => send(&s, &mut h, PartsCmd::TogglePart { part: r.below(4) as u8 }),
                88 => send(&s, &mut h, PartsCmd::SetPartOctave { part: r.below(4) as u8, octave: r.below(5) as i8 - 2 }),
                89 => send(&s, &mut h, ChordCmd::SetTranspose { keyboard: r.below(7) as i8 - 3, master: r.below(3) as i8 - 1 }),
                90..=92 => send(&s, &mut h, TransportCmd::StartStop),
                93 => send(&s, &mut h, LibraryCmd::LoadStyle { id: ids[r.below(ids.len() as u64) as usize] }),
                94 => send(&s, &mut h, HarmonyArpCmd::SetHarmonySpeed { speed: HarmonySpeed::Sixteenth }),
                95 => send(&s, &mut h, HarmonyArpCmd::SetChordNoteOnly { on: r.below(2) == 0 }),
                96 => send(&s, &mut h, HarmonyArpCmd::SetTouchLimit { velocity: 1 + r.below(100) as u8 }),
                97 => send(&s, &mut h, ChordCmd::ToggleUpper),
                98 => send(&s, &mut h, TransportCmd::Main { index: r.below(4) as u8 }),
                _ => {
                    if r.below(4) == 0 {
                        send(&s, &mut h, SystemCmd::Panic);
                    }
                }
            }
        }
        for k in 0..128u8 {
            if down[k as usize] {
                key(&s, &mut h, k, 0);
            }
        }
        send(&s, &mut h, HarmonyArpCmd::SetArpHold { on: false });
        send(&s, &mut h, HarmonyArpCmd::SetArpKeepKeyOn { on: false });
        advance(&s, &mut h, 3_000 * MS);
        assert!(s.state().keyboard.held.is_empty());
        assert_eq!(h.sounding(), [], "seed {seed}: notes left sounding");
        h.clear();
    }
}
