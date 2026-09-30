//! The MIDI input thread must not allocate or free: the keyboard-part note path
//! (src/live/pipeline.rs: note in, transpose, processor, part routing and output) and the
//! chord section's recognition run there for every key. The crate's counting allocator
//! (`alloc_count`, on the test's own thread, which plays the input thread) checks
//! `Input::packet` through both hands, layered parts, a transpose, retriggers, pedals,
//! wheels, the pedals' assignable functions and aftertouch, and on the Launchkey port
//! through the faders, knobs, fader buttons (a tap, swap mode's hold + knob, the Sound hold)
//! and pads.

use crate::alloc_count::{count_here, counts};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use yahaha::live::{FxConfig, FxMode, Input, Out, Shared};
use yahaha::midi::InputHandler;
use yahaha::rt::{PacketSink, Target};
use yahaha::theory::Recognizer;

#[test]
fn keyboard_note_path_does_not_allocate() {
    let _on = count_here();
    let shared = Arc::new(Shared::new(54));
    for p in 0..3 {
        shared.parts.on[p].store(true, Ordering::Relaxed);
    }
    let (tx, mut rx) = rtrb::RingBuffer::new(256);
    let mut input = Input::new(shared.clone(), Recognizer::new(), tx, Out::new(PacketSink::new(Target::Null), None));
    let (actions, mut actions_rx) = rtrb::RingBuffer::new(64);
    input.set_actions(actions);
    // Pedals: 2 runs Start/Stop (an engine button), OTS + (a control-side function, through
    // the actions ring), or Arpeggio Hold / Kbd Harmony/Arpeggio (control-side switches:
    // Hold A sets on the edges, Toggle runs on the press), by round; 3 is a pitch-bend foot
    // controller.
    use yahaha::controllers::{ControlType, Function, PedalSetup, Range};
    let ctl = &shared.controllers;
    ctl.set_pedal(1, PedalSetup { cc: Some(66), function: Function::StartStop, ..PedalSetup::default() });
    ctl.set_pedal(2, PedalSetup { cc: Some(4), function: Function::PitchBend, range: Range::Full, ..PedalSetup::default() });
    // Warm up: nothing sized lazily later on.
    input.packet(1, 0, &[0x90, 60, 100, 0x80, 60, 0]);
    input.end_of_list();
    // With the performance view collecting (`perf`): packets and their latency.
    yahaha::perf::enable();

    let (allocs, frees) = counts();
    input.packet(1, yahaha::rt::host_now(), &[0x90, 62, 100, 0x80, 62, 0]);
    let (mut assigned, mut strikes, mut levels, mut holds, mut rack_faders, mut knobs) = (0, 0, 0, 0, 0, 0);
    let (mut swaps, mut racks) = (0, 0);
    for round in 0..50u8 {
        // Dynamics Touch / Accent on in some rounds: chord-section strikes go to the engine.
        shared.strikes.store(round % 4 < 2, Ordering::Relaxed);
        while let Ok(c) = rx.pop() {
            strikes += matches!(c, yahaha::live::Cmd::Strike(_)) as u32;
            levels += matches!(c, yahaha::live::Cmd::DynamicsLevel(_)) as u32;
            holds += matches!(c, yahaha::live::Cmd::TempoHold(_) | yahaha::live::Cmd::Button(yahaha::engine::Button::TempoReset)) as u32;
        }
        let (function, control_type) = match round % 4 {
            0 => (Function::StartStop, ControlType::HoldA),
            1 => (Function::OtsNext, ControlType::HoldA),
            2 => (Function::ArpHold, ControlType::HoldA),
            _ => (Function::KbdHarmonyArp, ControlType::Toggle),
        };
        ctl.set_pedal(1, PedalSetup { cc: Some(66), function, control_type, ..PedalSetup::default() });
        // Pedal 3 alternates between a pitch-bend and a Dynamics Control foot controller
        // (the level goes to the engine ring).
        let foot = if round % 2 == 0 { Function::PitchBend } else { Function::DynamicsControl };
        ctl.set_pedal(2, PedalSetup { cc: Some(4), function: foot, range: Range::Full, ..PedalSetup::default() });
        while let Ok(a) = actions_rx.pop() {
            match a {
                yahaha::launchkey::Action::RackFader(..) => rack_faders += 1,
                yahaha::launchkey::Action::Knob(..) => knobs += 1,
                yahaha::launchkey::Action::SwapSound { .. } => swaps += 1,
                yahaha::launchkey::Action::QuickRack(_) => racks += 1,
                _ => assigned += 1,
            }
        }
        // The live rack's controller map, by round: every Panel fader on its own part's
        // level (the input thread's takeover), or faders 1 and 4 on another target (an
        // action for the control side) and fader 2 on none.
        use yahaha::parts::FaderRoute::{Control, Off, Own};
        shared.parts.set_rack_faders(if round % 3 == 0 { [Own; 4] } else { [Control, Off, Own, Control] });
        shared.key_shift.store((round % 5) as i8 - 2, Ordering::Relaxed);
        // [ACMP] off in some rounds (#266): no chord section, any key for Sync Start.
        shared.acmp.store(round % 3 != 0, Ordering::Relaxed);
        // Some rounds with a keyboard part soloed (Left, Right 2, none).
        shared.parts.set_solo([None, Some(3), Some(1)][round as usize % 3]);
        // Left on in some rounds, with Left Hold (#202) on in some: its keys re-pedal Left.
        shared.parts.on[3].store(round % 2 == 0, Ordering::Relaxed);
        ctl.set_left_hold(round % 3 != 1);
        // Some rounds with the Chord Looper looping: the left hand plays too.
        shared.looping.store(round % 2 == 1, Ordering::Relaxed);
        // Some rounds in AI Full Keyboard, where a re-struck chord is checked for three
        // notes (#107); the dyad after everything is up at the end of the round is one.
        use yahaha::fingering::Fingering;
        let mode = if round % 3 == 2 { Fingering::AiFullKeyboard } else { Fingering::Fingered };
        shared.fingering.store(mode.to_u8(), Ordering::Relaxed);
        // A left-hand chord, a right-hand melody over layered parts, a retrigger, the
        // sustain pedal, poly aftertouch, then everything up (one note-off as a note-on
        // with velocity 0, one through running status).
        input.packet(1, 0, &[0x90, 36, 90, 40, 90, 43, 90]);
        input.packet(1, 0, &[0x90, 72, 100, 0x90, 76, 80, 0x90, 72, 110]);
        input.packet(1, 0, &[0xB0, 64, 127, 0xA0, 76, 40, 0xE0, 0, 80]);
        input.packet(1, 0, &[0x80, 36, 0, 40, 0, 0x90, 43, 0]);
        input.packet(1, 0, &[0x80, 72, 0, 76, 0, 0xB0, 64, 0]);
        // The wheels, the pedals' functions, a part switched under them, a reset.
        input.packet(1, 0, &[0xB0, 1, round, 0xE0, round, 0x50, 0xB0, 66, 127, 0xB0, 4, round]);
        shared.parts.toggle(1);
        input.packet(1, 0, &[0xB0, 66, 0, 0xB0, 64, 127, 0xB0, 121, 0, 0xB0, 64, 0]);
        input.packet(1, 0, &[0x90, 64, 90, 67, 90, 0x80, 64, 0, 67, 0]);
        // The Launchkey's TEMPO buttons: + held and let go, then − and + together (#263).
        use yahaha::launchkey::{FUNCTION_CC, SCENE_CC};
        use yahaha::live::TAG_PADS;
        input.packet(TAG_PADS, 0, &[0xB0, SCENE_CC, 127, 0xB0, SCENE_CC, 0]);
        input.packet(TAG_PADS, 0, &[0xB0, FUNCTION_CC, 127, 0xB0, SCENE_CC, 127, 0xB0, FUNCTION_CC, 0, 0xB0, SCENE_CC, 0]);
        // The faders in a fader layer: Shift + the master fader's button steps VOL, PAN,
        // REV, CHO, DLY; faders 1 and 3 move a part's pan or send, on either page.
        use yahaha::launchkey::{FADER_BTN_CC, FADER_CC, SHIFT_CC};
        let (f1, mb) = (*FADER_CC.start(), *FADER_BTN_CC.end());
        input.packet(TAG_PADS, 0, &[0xB0, SHIFT_CC, 127, 0xB0, mb, 127, 0xB0, SHIFT_CC, 0]);
        input.packet(TAG_PADS, 0, &[0xB0, f1, round, 0xB0, f1 + 2, 127 - round, 0xB0, f1, 64]);
        // Every Panel fader 1-4 (the layer, stepped each round, is Volume in some), and
        // an encoder: a knob on the Knob Assign page, run by the control side.
        input.packet(TAG_PADS, 0, &[0xB0, f1, round, 0xB0, f1 + 1, round, 0xB0, f1 + 3, 127 - round]);
        input.packet(TAG_PADS, 0, &[0xBF, 21, 65]);
        // The fader buttons' holds (docs/eyes-free.md): a part button tapped; a part button
        // held with knobs 1 and 3 turned (swap mode, on the Panel page); and Sound held with
        // a pad pressed under it (the Racks page's pads), then let go.
        use yahaha::launchkey::SOUND_FADER_BTN;
        let (b1, sound) = (*FADER_BTN_CC.start(), *FADER_BTN_CC.start() + SOUND_FADER_BTN);
        input.packet(TAG_PADS, 0, &[0xB0, b1 + 1, 127, 0xB0, b1 + 1, 0]);
        input.packet(TAG_PADS, 0, &[0xB0, b1, 127, 0xBF, 21, 65, 0xBF, 23, 63, 0xB0, b1, 0]);
        input.packet(TAG_PADS, 0, &[0xB0, sound, 127, 0x90, 96 + round % 8, 100, 0xB0, sound, 0]);
        if round % 7 == 0 {
            input.packet(TAG_PADS, 0, &[0xB0, mb, 127]);
        }
        input.end_of_list();
        ctl.reset(&mut |_| {});
    }
    assert_eq!(counts().0 - allocs, 0, "the input thread allocated");
    assert_eq!(counts().1 - frees, 0, "the input thread freed");
    assert!(assigned > 0, "OTS + went through the actions ring");
    assert!(strikes > 0, "chord-section strikes went to the engine");
    assert!(levels > 0, "the Dynamics Control pedal went to the engine");
    assert!(holds > 0, "the tempo buttons went to the engine");
    assert!(rack_faders > 0, "remapped faders went through the actions ring");
    assert!(knobs > 0, "the knobs went through the actions ring");
    assert!(swaps > 0, "a knob during a part button's hold stepped the part's sound");
    assert!(racks > 0, "a pad under the Sound hold recalled a Quick Rack");
    assert_eq!(shared.layer(), yahaha::launchkey::Layer::None, "every hold was let go");
}

/// The processor slot: every Harmony type (Strum's late notes and the Echo category go to
/// the engine thread's ring), Multi Assign and the arpeggio, switched while keys are down.
#[test]
fn harmony_and_arpeggio_processor_does_not_allocate() {
    let _on = count_here();
    let shared = Arc::new(Shared::new(54));
    for p in 0..3 {
        shared.parts.on[p].store(true, Ordering::Relaxed);
    }
    let (tx, _rx) = rtrb::RingBuffer::new(256);
    let (fx_tx, mut fx_rx) = rtrb::RingBuffer::new(yahaha::live::FX_RING);
    let mut input = Input::new(shared.clone(), Recognizer::new(), tx, Out::new(PacketSink::new(Target::Null), None));
    input.set_fx(fx_tx);
    let configs: Vec<u64> = yahaha::harmony::ALL_TYPES
        .iter()
        .map(|&ty| {
            let mut c = FxConfig { on: true, ..FxConfig::default() };
            c.harmony.ty = ty;
            c.pack()
        })
        .chain([FxConfig { on: true, mode: FxMode::Arpeggio, ..FxConfig::default() }.pack(), FxConfig::default().pack()])
        .collect();
    input.packet(1, 0, &[0x90, 60, 100, 0x80, 60, 0]);
    input.end_of_list();

    let (allocs, frees) = counts();
    for round in 0..4u8 {
        for &w in &configs {
            shared.kbd_fx.store(w, Ordering::Relaxed);
            shared.key_shift.store((round % 3) as i8 - 1, Ordering::Relaxed);
            // A left-hand chord, a right-hand melody with a lower key and a retrigger; the
            // type switches while they are down, then everything goes up.
            input.packet(1, 0, &[0x90, 36, 90, 40, 90, 43, 90]);
            input.packet(1, 0, &[0x90, 72, 100, 0x90, 64, 80, 0x90, 76, 110, 0x90, 72, 100]);
            input.end_of_list();
            while fx_rx.pop().is_ok() {}
        }
        for k in [36u8, 40, 43, 64, 72, 76] {
            input.packet(1, 0, &[0x80, k, 0]);
        }
        input.end_of_list();
        while fx_rx.pop().is_ok() {}
    }
    assert_eq!(counts().0 - allocs, 0, "the input thread allocated");
    assert_eq!(counts().1 - frees, 0, "the input thread freed");
}
