//! Session tests: the API through an offline session (no CoreMIDI, no audio), driven by
//! its virtual clock.

use super::*;
use crate::launchkey::{Level, PAD_DOWN_CC, SHIFT_CC};

fn style(name: &str) -> Option<PathBuf> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2").join(name);
    if p.exists() {
        Some(p)
    } else {
        eprintln!("corpus missing; skipping");
        None
    }
}

fn offline(name: &str) -> Option<Session> {
    let p = style(name)?;
    Some(Session::offline(Options { paths: vec![p], ..Options::default() }).unwrap())
}

const MS: u64 = 1_000_000;

fn keys(s: &Session, on: bool, notes: &[u8]) {
    for &n in notes {
        s.midi_in(Port::Keys, &[if on { 0x90 } else { 0x80 }, n, if on { 100 } else { 0 }]);
    }
}

/// Every command, for the serialization tests.
fn all_cmds() -> Vec<AppCmd> {
    use AppCmd::*;
    vec![
        Intro { index: 1 },
        Main { index: 2 },
        Break,
        Ending { index: 0 },
        StartStop,
        Stop,
        ToggleSyncStart,
        ToggleSyncStop,
        ToggleAutoFill,
        ToggleStopAcmp,
        TapTempo,
        TempoUp,
        TempoDown,
        ToggleStylePart { part: 5 },
        SetStylePartVolume { part: 2, volume: 90 },
        SetFingering { fingering: Fingering::AiFullKeyboard },
        NextFingering,
        SetUpper { on: true },
        ToggleUpper,
        SetManualBass { on: false },
        ToggleManualBass,
        SetSplit { note: 60 },
        MoveSplit { delta: -1 },
        SetTranspose { keyboard: 2, master: -1 },
        StepTranspose { keyboard: 1, master: 0 },
        ResetTranspose,
        SetPartOn { part: 1, on: true },
        TogglePart { part: 3 },
        SelectPart { part: 2 },
        SetPartVoice { part: 0, program: 4 },
        StepVoice { delta: -1 },
        SetPartVolume { part: 3, volume: 64 },
        SetPartOctave { part: 1, octave: -1 },
        SetFaderPage { page: FaderPage::Style },
        ToggleFaderPage,
        SetPadPage { page: Page::OtsParts },
        CyclePadPage { delta: 1 },
        SetMasterVolume { volume: 110 },
        RecallOts { index: 3 },
        SetOtsLink { on: true },
        ToggleOtsLink,
        LoadStyle { id: 7 },
        LoadStylePath { path: "/styles/Funk.sty".into() },
        StepStyle { delta: 1 },
        SetSynthMuted { on: true },
        ToggleSynthMute,
        SetAudioOutput { first: 2 },
        NextAudioOutput,
        Panic,
        ClearMessage,
    ]
}

#[test]
fn commands_serialize_as_tagged_camel_case() {
    for c in all_cmds() {
        let j = serde_json::to_string(&c).unwrap();
        assert_eq!(serde_json::from_str::<AppCmd>(&j).unwrap(), c, "{j}");
    }
    let j = |c: AppCmd| serde_json::to_string(&c).unwrap();
    assert_eq!(j(AppCmd::Main { index: 1 }), r#"{"type":"main","index":1}"#);
    assert_eq!(j(AppCmd::StartStop), r#"{"type":"startStop"}"#);
    assert_eq!(j(AppCmd::SetStylePartVolume { part: 2, volume: 90 }), r#"{"type":"setStylePartVolume","part":2,"volume":90}"#);
    assert_eq!(j(AppCmd::SetFingering { fingering: Fingering::FingeredOnBass }), r#"{"type":"setFingering","fingering":"fingeredOnBass"}"#);
    assert_eq!(j(AppCmd::SetPadPage { page: Page::ChordSetup }), r#"{"type":"setPadPage","page":"chordSetup"}"#);
    let e = serde_json::to_string(&Event::StateChanged { version: 3 }).unwrap();
    assert_eq!(e, r#"{"type":"stateChanged","version":3}"#);
}

#[test]
fn app_state_round_trips_through_json() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    s.send(AppCmd::RecallOts { index: 0 }).unwrap();
    keys(&s, true, &[45, 48, 52, 55]);
    s.advance(700 * MS);
    let st = s.state();
    assert!(st.transport.running);
    let j = serde_json::to_string_pretty(&*st).unwrap();
    let back: AppState = serde_json::from_str(&j).unwrap();
    assert_eq!(back, *st);
    // Every field is there, camelCase.
    for k in ["\"keyboardParts\"", "\"styleParts\"", "\"lamps\"", "\"faderPage\"", "\"syncStopAvailable\"", "\"transposeKeyboard\""] {
        assert!(j.contains(k), "{k}");
    }
    let lib = s.library_list();
    let j = serde_json::to_string(&lib).unwrap();
    assert_eq!(serde_json::from_str::<LibraryList>(&j).unwrap(), lib);
}

/// Print a state as JSON (for docs/app-api.md): `cargo test --release print_state_json -- --ignored --nocapture`.
#[test]
#[ignore]
fn print_state_json() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    s.send(AppCmd::RecallOts { index: 0 }).unwrap();
    keys(&s, true, &[45, 48, 52, 55]);
    s.advance(2300 * MS);
    s.send(AppCmd::Main { index: 1 }).unwrap();
    println!("{}", serde_json::to_string_pretty(&s.state_now()).unwrap());
}

#[test]
fn sync_start_sections_and_position() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    let st = s.state();
    assert!(!st.transport.running && st.transport.sync_start);
    assert_eq!(st.transport.section, None);
    assert_eq!((st.transport.bar, st.transport.beat), (1, 1));
    assert!(st.style.sections.iter().any(|n| n == "Main A"));
    assert!(st.io.offline);

    // A chord in the left hand starts the style (Sync Start).
    keys(&s, true, &[36, 40, 43]);
    let st = s.state();
    assert!(st.transport.running);
    assert_eq!(st.chord.name.as_deref(), Some("C"));
    assert_eq!(st.transport.section.as_deref(), Some("Main A"));
    let v = s.version();
    let bar = (60e9 / st.style.tempo * st.transport.beats_per_bar as f64) as u64;
    s.advance(bar + bar / 8);
    let st = s.state();
    assert!(s.version() > v);
    assert_eq!((st.transport.bar, st.transport.beat), (2, 1));
    assert!(!s.take_output().is_empty(), "the band played");

    // Main B with Auto Fill on: Fill In BB first, then Main B.
    s.send(AppCmd::Main { index: 1 }).unwrap();
    assert_eq!(s.state().transport.queued.as_deref(), Some("Fill In BB"));
    s.advance(2 * bar);
    let st = s.state();
    assert_eq!(st.transport.section.as_deref(), Some("Main B"));
    assert_eq!(st.transport.main, 1);
    // Its lamp is lit on page 1 (the pad at note 113).
    let lamp = st.transport.lamps.iter().find(|p| p.note == 113).unwrap();
    assert_eq!((lamp.label.as_str(), lamp.level), ("MAIN B", Level::Bright));
    assert_eq!(lamp.action, Some(AppCmd::Main { index: 1 }));

    let t = st.transport.tempo;
    s.send(AppCmd::TempoUp).unwrap();
    assert!(s.state().transport.tempo > t);
    s.send(AppCmd::StartStop).unwrap();
    assert!(!s.state().transport.running);
    s.send(AppCmd::Panic).unwrap();
}

#[test]
fn chord_settings_split_and_transpose() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    s.send(AppCmd::SetFingering { fingering: Fingering::FullKeyboard }).unwrap();
    let st = s.state();
    assert_eq!(st.chord.fingering, Fingering::FullKeyboard);
    assert_eq!(st.chord.fingering_name, "Full Keyboard");
    assert!(!st.transport.sync_stop_available);
    s.send(AppCmd::NextFingering).unwrap();
    assert_eq!(s.state().chord.fingering, Fingering::AiFullKeyboard);

    // Upper turns Manual Bass on: the Style's Bass part is muted, Left plays the bass.
    s.send(AppCmd::SetUpper { on: true }).unwrap();
    let st = s.state();
    assert!(st.chord.upper && st.chord.manual_bass && st.chord.manual_bass_active);
    assert!(st.transport.sync_stop_available, "Upper is Fingered*");
    assert!(st.mixer.style_parts[2].muted_by_manual_bass && !st.mixer.style_parts[2].on);
    assert!(st.keyboard_parts[3].plays_bass && st.keyboard_parts[3].sounding);
    // Left can't be switched then: refused, with a message.
    let err = s.send(AppCmd::TogglePart { part: 3 }).unwrap_err();
    assert!(matches!(err, CmdError::Failed(_)));
    assert!(s.state().message.as_ref().is_some_and(|m| m.error && m.text.contains("Manual Bass")));
    s.send(AppCmd::ClearMessage).unwrap();
    assert_eq!(s.state().message, None);
    s.send(AppCmd::ToggleManualBass).unwrap();
    assert!(!s.state().chord.manual_bass_active);
    s.send(AppCmd::ToggleUpper).unwrap();
    let st = s.state();
    assert!(!st.chord.upper && !st.chord.manual_bass_active);
    // Manual Bass is Upper-only.
    s.send(AppCmd::SetManualBass { on: true }).unwrap();
    assert!(!s.state().chord.manual_bass);

    assert_eq!((s.state().chord.split, s.state().chord.split_name.as_str()), (54, "F#2"));
    s.send(AppCmd::MoveSplit { delta: 2 }).unwrap();
    assert_eq!(s.state().chord.split_name, "Ab2");
    s.send(AppCmd::SetSplit { note: 200 }).unwrap();
    assert_eq!(s.state().chord.split, 96);

    s.send(AppCmd::StepTranspose { keyboard: 1, master: -2 }).unwrap();
    s.send(AppCmd::StepTranspose { keyboard: 1, master: 0 }).unwrap();
    let st = s.state();
    assert_eq!((st.chord.transpose_keyboard, st.chord.transpose_master), (2, -2));
    s.send(AppCmd::SetTranspose { keyboard: 20, master: 0 }).unwrap();
    assert_eq!(s.state().chord.transpose_keyboard, 12);
    s.send(AppCmd::ResetTranspose).unwrap();
    assert_eq!(s.state().chord.transpose_keyboard, 0);
}

/// Transpose reaches the notes you play: a key sounds shifted on its part's channel.
#[test]
fn transpose_and_parts_reach_the_keys() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    s.take_output();
    s.send(AppCmd::SetTranspose { keyboard: 2, master: 0 }).unwrap();
    s.send(AppCmd::SetPartOn { part: 1, on: true }).unwrap();
    s.send(AppCmd::SetPartOctave { part: 1, octave: 1 }).unwrap();
    s.take_output();
    keys(&s, true, &[72]);
    let out = s.take_output();
    assert!(out.contains(&[0x90, 74, 100]), "{out:?}");
    assert!(out.contains(&[0x92, 86, 100]), "Right 2 an octave up: {out:?}");
}

#[test]
fn keyboard_parts_mixer_and_pages() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    let st = s.state();
    assert_eq!(st.keyboard_parts.len(), 4);
    assert_eq!(st.keyboard_parts.iter().map(|p| p.channel).collect::<Vec<_>>(), vec![1, 3, 4, 2]);
    assert!(st.keyboard_parts[0].on && st.keyboard_parts[0].selected);

    s.send(AppCmd::SelectPart { part: 1 }).unwrap();
    s.send(AppCmd::SetPartVoice { part: 1, program: 40 }).unwrap();
    s.send(AppCmd::StepVoice { delta: 1 }).unwrap();
    s.send(AppCmd::SetPartVolume { part: 1, volume: 70 }).unwrap();
    s.send(AppCmd::SetPartOctave { part: 1, octave: 5 }).unwrap();
    let p = s.state().keyboard_parts[1].clone();
    assert!(p.selected);
    assert_eq!((p.program, p.voice_name.as_str(), p.volume, p.octave), (41, "Viola", 70, 2));
    // The engine sends the volume as the part's CC7.
    assert!(s.take_output().contains(&[0xB2, 7, 70]));

    s.send(AppCmd::SetStylePartVolume { part: 5, volume: 33 }).unwrap();
    s.send(AppCmd::ToggleStylePart { part: 6 }).unwrap();
    let st = s.state();
    assert_eq!(st.mixer.style_parts[5].volume, 33);
    assert!(!st.mixer.style_parts[6].on);
    assert_eq!(st.mixer.style_parts[2].name, "Bass");
    assert_eq!(st.mixer.style_parts[2].channel, 11);
    assert!(st.mixer.style_parts[2].voice.is_some());

    assert_eq!(st.mixer.fader_page, FaderPage::Panel);
    s.send(AppCmd::ToggleFaderPage).unwrap();
    assert_eq!(s.state().mixer.fader_page, FaderPage::Style);
    s.send(AppCmd::SetFaderPage { page: FaderPage::Panel }).unwrap();
    assert_eq!(s.state().mixer.fader_page, FaderPage::Panel);
    assert_eq!(st.mixer.master, None, "no synth offline");
    assert!(s.send(AppCmd::SetMasterVolume { volume: 90 }).is_err());

    assert_eq!(st.pads.page, Page::Sections);
    s.send(AppCmd::CyclePadPage { delta: -1 }).unwrap();
    let st = s.state();
    assert_eq!((st.pads.page, st.pads.page_number, st.pads.page_count), (Page::OtsParts, 3, 3));
    assert_eq!(st.pads.pads.len(), 16);
    assert_eq!(st.pads.pads[0].label, "OTS 1");
    assert_eq!(st.pads.pads[0].action, Some(AppCmd::RecallOts { index: 0 }));
    s.send(AppCmd::SetPadPage { page: Page::ChordSetup }).unwrap();
    assert_eq!(s.state().pads.pads[1].action, Some(AppCmd::SetFingering { fingering: Fingering::Fingered }));
}

#[test]
fn ots_and_ots_link() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    let st = s.state();
    assert!(st.ots.settings.len() >= 2);
    assert_eq!(st.ots.settings[0].name, "OTS 1");
    assert_eq!(st.ots.applied, 0);
    s.send(AppCmd::RecallOts { index: 0 }).unwrap();
    let st = s.state();
    assert_eq!(st.ots.applied, 1);
    assert_eq!(st.keyboard_parts.iter().map(|p| p.on).collect::<Vec<_>>(), vec![true, true, false, true]);
    assert_eq!(st.keyboard_parts[0].program, 80);

    // OTS Link: Main B recalls OTS 2.
    s.send(AppCmd::SetOtsLink { on: true }).unwrap();
    assert_eq!(s.state().ots.applied, 1, "Main A: OTS 1");
    s.send(AppCmd::Main { index: 1 }).unwrap();
    let st = s.state();
    assert!(st.ots.link);
    assert_eq!(st.ots.applied, 2);
    s.send(AppCmd::ToggleOtsLink).unwrap();
    assert!(!s.state().ots.link);
}

#[test]
fn library_style_loading() {
    let Some(p) = style("SlowWalker.T552.sty") else { return };
    let dir = p.parent().unwrap().to_path_buf();
    let s = Session::offline(Options { paths: vec![dir], ..Options::default() }).unwrap();
    s.finish_indexing();
    let st = s.state();
    let lib = s.library_list();
    assert_eq!(lib.entries.len(), st.library.count);
    assert!(st.library.count > 3);
    assert_eq!(st.library.pending, 0);
    assert_eq!(lib.revision, st.library.revision);
    let first = st.style.clone();
    s.send(AppCmd::StepStyle { delta: 1 }).unwrap();
    let st = s.state();
    assert_ne!(st.style.id, first.id);
    assert_eq!(st.library.position, 1);
    let e = lib.entries.iter().find(|e| e.status == "ok" && e.id != st.style.id).unwrap();
    s.send(AppCmd::LoadStyle { id: e.id }).unwrap();
    assert_eq!(s.state().style.id, e.id);
    s.send(AppCmd::LoadStylePath { path: p.display().to_string() }).unwrap();
    assert!(s.state().style.path.ends_with("SlowWalker.T552.sty"));
    // A file that doesn't load: an error, a message, and the entry is marked.
    let rx = s.subscribe();
    let bad = std::env::temp_dir().join(format!("yahaha-bad-{}.sty", std::process::id()));
    std::fs::write(&bad, b"not a style").unwrap();
    let r = s.send(AppCmd::LoadStylePath { path: bad.display().to_string() });
    let _ = std::fs::remove_file(&bad);
    assert!(matches!(r, Err(CmdError::Failed(_))));
    assert!(s.state().style.path.ends_with("SlowWalker.T552.sty"));
    assert!(s.state().message.as_ref().is_some_and(|m| m.error));
    let evs: Vec<Event> = rx.try_iter().collect();
    assert!(evs.iter().any(|e| matches!(e, Event::LibraryChanged { .. })), "{evs:?}");
    assert!(evs.iter().any(|e| matches!(e, Event::StateChanged { .. })), "{evs:?}");
    assert!(s.library_list().entries.iter().any(|e| e.status == "error" && e.path == bad.display().to_string()));
}

/// The Launchkey runs the same commands as the app: pages, pads, fader buttons.
#[test]
fn launchkey_pads_are_commands() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    // Pad Bank ▼: page 2; pad 97 = Fingered.
    s.midi_in(Port::Pads, &[0xB0, PAD_DOWN_CC, 127]);
    assert_eq!(s.state().pads.page, Page::ChordSetup);
    s.midi_in(Port::Pads, &[0x90, 97, 100]);
    assert_eq!(s.state().chord.fingering, Fingering::Fingered);
    // Pad 103 = Upper; 114 = Split -.
    s.midi_in(Port::Pads, &[0x90, 103, 100, 0x90, 114, 100]);
    let st = s.state();
    assert!(st.chord.upper);
    assert_eq!(st.chord.split, 53);
    // Page 3: pad 113 = Right 2 on; Shift + ▼ = OTS Link.
    s.midi_in(Port::Pads, &[0xB0, PAD_DOWN_CC, 127]);
    s.midi_in(Port::Pads, &[0x90, 113, 100]);
    assert!(s.state().keyboard_parts[1].on);
    s.midi_in(Port::Pads, &[0xB0, SHIFT_CC, 127, 0xB0, PAD_DOWN_CC, 127, 0xB0, SHIFT_CC, 0]);
    assert!(s.state().ots.link);
    // Fader buttons on the Panel page: part on/off; Shift + button selects.
    s.midi_in(Port::Pads, &[0xB0, 39, 127]);
    assert!(s.state().keyboard_parts[2].on);
    s.midi_in(Port::Pads, &[0xB0, SHIFT_CC, 127, 0xB0, 38, 127, 0xB0, SHIFT_CC, 0]);
    assert!(s.state().keyboard_parts[1].selected);
    // The master fader button: the Style page.
    s.midi_in(Port::Pads, &[0xB0, 45, 127]);
    assert_eq!(s.state().mixer.fader_page, FaderPage::Style);
    // Page 1 pads are engine buttons: Start/Stop.
    s.send(AppCmd::SetPadPage { page: Page::Sections }).unwrap();
    s.midi_in(Port::Pads, &[0x90, 119, 100]);
    assert!(s.state().transport.running);
    // What a pad's `action` says is what pressing it does.
    let stop = s.state().pads.pads.iter().find(|p| p.note == 119).unwrap().action.clone().unwrap();
    s.send(stop).unwrap();
    assert!(!s.state().transport.running);
    // Unmapped controls show up for diagnosis.
    s.midi_in(Port::Pads, &[0xB0, 51, 127]);
    assert_eq!(s.state().io.unmapped, "unmapped CC 51 = 127");
}

/// Style faders move the Style parts (soft takeover); a software move makes the fader
/// pick the part up again.
#[test]
fn style_faders_and_software_volume() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    s.send(AppCmd::SetFaderPage { page: FaderPage::Style }).unwrap();
    let v0 = s.state().mixer.style_parts[0].volume;
    s.midi_in(Port::Pads, &[0xB0, 5, v0]);
    s.midi_in(Port::Pads, &[0xB0, 5, 50]);
    assert_eq!(s.state().mixer.style_parts[0].volume, 50);
    s.send(AppCmd::SetStylePartVolume { part: 0, volume: 100 }).unwrap();
    let st = s.state();
    assert_eq!(st.mixer.style_parts[0].volume, 100);
    s.midi_in(Port::Pads, &[0xB0, 5, 40]);
    let st = s.state();
    assert_eq!(st.mixer.style_parts[0].volume, 100, "no jump");
    assert!(st.mixer.style_parts[0].waiting);
}

#[test]
fn versions_and_events() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    let rx = s.subscribe();
    let v = s.version();
    assert_eq!(s.state().version, v);
    s.send(AppCmd::ClearMessage).unwrap();
    assert_eq!(s.version(), v, "nothing changed, same version");
    s.send(AppCmd::MoveSplit { delta: 1 }).unwrap();
    assert_eq!(s.version(), v + 1);
    assert_eq!(rx.try_recv(), Ok(Event::StateChanged { version: v + 1 }));
    assert!(rx.try_recv().is_err());
    s.stop();
    assert_eq!(rx.try_recv(), Ok(Event::Stopped));
}

#[test]
fn an_empty_library_is_an_error() {
    let dir = std::env::temp_dir().join(format!("yahaha-empty-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let r = Session::offline(Options { paths: vec![dir.clone()], ..Options::default() });
    let _ = std::fs::remove_dir(&dir);
    assert!(r.is_err());
}

/// Every Launchkey pad (each page), button and fader button does exactly what the command
/// it stands for does: the same state and the same output, from several starting states.
#[test]
fn launchkey_hardware_matches_its_commands() {
    use crate::launchkey::{FUNCTION_CC, PAD_UP_CC, PLAY_CC, SCENE_CC, STOP_CC, TRACK_LEFT_CC, TRACK_RIGHT_CC};
    let Some(p) = style("SlowWalker.T552.sty") else { return };
    let mk = |setup: u8| {
        let s = Session::offline(Options { paths: vec![p.clone()], ..Options::default() }).unwrap();
        match setup {
            0 => {}
            1 => {
                s.send(AppCmd::SetOtsLink { on: true }).unwrap();
                s.send(AppCmd::SetUpper { on: true }).unwrap();
                keys(&s, true, &[60, 64, 67]);
                s.advance(700 * MS);
            }
            _ => {
                s.send(AppCmd::RecallOts { index: 1 }).unwrap();
                keys(&s, true, &[36, 40, 43]);
                s.advance(1300 * MS);
                s.send(AppCmd::Main { index: 2 }).unwrap();
            }
        }
        s
    };
    let norm = |s: &Session| {
        let mut st = (*s.state()).clone();
        (st.version, st.io.last_control) = (0, 0);
        st.io.unmapped.clear();
        // When the state last changed is not what it is: read the clock now.
        st.surface.clock = st.surface.clock.at(ns_to_ms(s.now()));
        (st, s.take_output())
    };
    let mut bad = Vec::new();
    // `hw` on one session, `cmd` on a twin: they must end up the same.
    let mut pair = |setup: u8, what: String, prep: &dyn Fn(&Session), hw: &[&[u8]], cmd: Option<AppCmd>| {
        let (a, b) = (mk(setup), mk(setup));
        prep(&a);
        prep(&b);
        a.take_output();
        b.take_output();
        for m in hw {
            a.midi_in(Port::Pads, m);
        }
        if let Some(c) = cmd {
            let _ = b.send(c);
        }
        if norm(&a) != norm(&b) {
            bad.push(format!("setup {setup}: {what}"));
        }
    };
    for setup in 0..3 {
        for page in Page::ALL {
            let prep = move |s: &Session| s.send(AppCmd::SetPadPage { page }).unwrap();
            let pads = {
                let s = mk(setup);
                prep(&s);
                s.state().pads.pads.clone()
            };
            for pad in pads {
                pair(setup, format!("{page:?} pad {} {:?}", pad.note, pad.action), &prep, &[&[0x90, pad.note, 100]], pad.action);
            }
        }
        let page2 = |s: &Session| s.send(AppCmd::SetPadPage { page: Page::ChordSetup }).unwrap();
        for (cc, shift, cmd) in [
            (PLAY_CC, false, AppCmd::StartStop),
            (STOP_CC, false, AppCmd::Stop),
            (SCENE_CC, false, AppCmd::TempoUp),
            (FUNCTION_CC, false, AppCmd::TempoDown),
            (TRACK_LEFT_CC, false, AppCmd::StepStyle { delta: -1 }),
            (TRACK_RIGHT_CC, false, AppCmd::StepStyle { delta: 1 }),
            (PAD_UP_CC, true, AppCmd::TogglePart { part: 3 }),
            (PAD_DOWN_CC, true, AppCmd::ToggleOtsLink),
            (PAD_UP_CC, false, AppCmd::CyclePadPage { delta: -1 }),
            (PAD_DOWN_CC, false, AppCmd::CyclePadPage { delta: 1 }),
        ] {
            let btn = [0xB0, cc, 127];
            let hw: Vec<&[u8]> = if shift { vec![&[0xB0, SHIFT_CC, 127], &btn, &[0xB0, SHIFT_CC, 0]] } else { vec![&btn] };
            pair(setup, format!("CC {cc} shift {shift}"), &page2, &hw, Some(cmd));
        }
        for i in 0..9u8 {
            for shift in [false, true] {
                for fp in [FaderPage::Panel, FaderPage::Style] {
                    let cmd = match (i, fp, shift) {
                        (8, _, _) => Some(AppCmd::ToggleFaderPage),
                        (0..=3, FaderPage::Panel, true) => Some(AppCmd::SelectPart { part: i }),
                        (0..=3, FaderPage::Panel, false) => Some(AppCmd::TogglePart { part: i }),
                        (_, FaderPage::Panel, _) => None,
                        (_, FaderPage::Style, _) => Some(AppCmd::ToggleStylePart { part: i }),
                    };
                    let btn = [0xB0, 37 + i, 127];
                    let hw: Vec<&[u8]> = if shift { vec![&[0xB0, SHIFT_CC, 127], &btn, &[0xB0, SHIFT_CC, 0]] } else { vec![&btn] };
                    let prep = move |s: &Session| s.send(AppCmd::SetFaderPage { page: fp }).unwrap();
                    pair(setup, format!("fader button {i} shift {shift} {fp:?}"), &prep, &hw, cmd);
                }
            }
        }
    }
    assert!(bad.is_empty(), "hardware and command differ:\n{}", bad.join("\n"));
}

/// A client can send any delta: no overflow, and the page wraps as it should.
#[test]
fn cycle_pad_page_takes_any_delta() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    s.send(AppCmd::SetPadPage { page: Page::OtsParts }).unwrap();
    s.send(AppCmd::CyclePadPage { delta: 127 }).unwrap(); // 2 + 127 = 129 = 0 mod 3
    assert_eq!(s.state().pads.page, Page::Sections);
    s.send(AppCmd::CyclePadPage { delta: -128 }).unwrap(); // 0 - 128 = 1 mod 3
    assert_eq!(s.state().pads.page, Page::ChordSetup);
}

/// While the library indexes, `library_list()` is labelled with the revision its entries
/// are, and the state's library status describes that same list.
#[test]
fn library_list_revision_matches_its_entries() {
    let Some(p) = style("SlowWalker.T552.sty") else { return };
    let root = p.parent().unwrap().parent().unwrap().to_path_buf();
    let s = Session::offline(Options { paths: vec![root], ..Options::default() }).unwrap();
    for _ in 0..5000 {
        s.send(AppCmd::ClearMessage).unwrap(); // the control side applies index results
        let st = s.state();
        let l = s.library_list();
        if l.revision == st.library.revision {
            let pending = l.entries.iter().filter(|e| e.status == "pending").count();
            assert_eq!((pending, l.entries.len()), (st.library.pending, st.library.count), "revision {}", l.revision);
        }
        if st.library.pending == 0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_micros(100));
    }
    s.finish_indexing();
    assert_eq!(s.state().library.pending, 0);
}

/// Live (real CoreMIDI, no Launchkey, no synth): what `send` applied on the control side
/// is in `state()` as soon as it returns, and a second, concurrent `stop` is harmless.
#[test]
fn live_send_is_visible_at_once() {
    let Some(p) = style("SlowWalker.T552.sty") else { return };
    let opts = Options { paths: vec![p], no_pads: true, inputs: vec!["(no such input)".into()], ..Options::default() };
    let s = match Session::start(opts) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            eprintln!("no CoreMIDI ({e}); skipping");
            return;
        }
    };
    s.send(AppCmd::SetSplit { note: 70 }).unwrap();
    assert_eq!(s.state().chord.split, 70);
    let rx = s.subscribe();
    let s2 = s.clone();
    let t = std::thread::spawn(move || s2.stop());
    s.stop();
    t.join().unwrap();
    assert!(rx.try_iter().any(|e| e == Event::Stopped));
}

/// Every button the state describes does, pressed on the hardware (with Shift where it has
/// a Shift layer), exactly what its `action` / `shiftAction` does; one without an action
/// does nothing. On every pad page, on both fader pages.
#[test]
fn launchkey_buttons_are_what_the_state_says() {
    let Some(p) = style("SlowWalker.T552.sty") else { return };
    let mk = |page: Page, fp: FaderPage| {
        let s = Session::offline(Options { paths: vec![p.clone()], ..Options::default() }).unwrap();
        s.send(AppCmd::SetPadPage { page }).unwrap();
        s.send(AppCmd::SetFaderPage { page: fp }).unwrap();
        s.take_output();
        s
    };
    let norm = |s: &Session| {
        let mut st = (*s.state()).clone();
        (st.version, st.io.last_control) = (0, 0);
        st.io.unmapped.clear();
        // When the state last changed is not what it is: read the clock now.
        st.surface.clock = st.surface.clock.at(ns_to_ms(s.now()));
        (st, s.take_output())
    };
    let mut bad = Vec::new();
    let mut n = 0;
    for page in Page::ALL {
        for fp in [FaderPage::Panel, FaderPage::Style] {
            let buttons = mk(page, fp).state().surface.controls.clone();
            assert_eq!(buttons.len(), 17);
            for b in buttons.iter() {
                for shift in [false, true] {
                    let cmd = if shift { b.shift_action.clone() } else { b.action.clone() };
                    let (a, c) = (mk(page, fp), mk(page, fp));
                    let press = [0xB0, b.cc, 127];
                    if shift {
                        a.midi_in(Port::Pads, &[0xB0, SHIFT_CC, 127]);
                        assert!(a.state().surface.shift);
                        a.midi_in(Port::Pads, &press);
                        a.midi_in(Port::Pads, &[0xB0, SHIFT_CC, 0]);
                    } else {
                        a.midi_in(Port::Pads, &press);
                    }
                    if let Some(cmd) = cmd {
                        let _ = c.send(cmd);
                        n += 1;
                    }
                    if norm(&a) != norm(&c) {
                        bad.push(format!("{page:?} {fp:?} {} shift {shift}", b.id));
                    }
                }
            }
        }
    }
    assert!(bad.is_empty(), "described and pressed differ:\n{}", bad.join("\n"));
    assert!(n > 60, "{n}");
}

/// What the buttons say and how they are lit.
#[test]
fn launchkey_button_descriptions() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    let b = |s: &Session, id: &str| s.state().surface.controls.iter().find(|b| b.id == id).cloned().unwrap();
    let up = b(&s, "padBankUp");
    assert_eq!((up.action, up.label.as_str(), up.level, up.colour), (None, "", Level::Off, Some(0)), "first page: nowhere up");
    assert_eq!((up.shift_label.as_str(), up.shift_action), ("LEFT", Some(AppCmd::TogglePart { part: 3 })));
    let down = b(&s, "padBankDown");
    assert_eq!(down.action, Some(AppCmd::SetPadPage { page: Page::ChordSetup }));
    assert_eq!((down.colour, down.level), (Some(3), Level::Bright), "white on page 1");
    assert_eq!(down.shift_action, Some(AppCmd::ToggleOtsLink));
    // One style: the Track buttons go nowhere and are dark.
    let tl = b(&s, "trackPrev");
    assert_eq!((tl.action, tl.level, tl.shift_action), (None, Level::Off, None));
    let play = b(&s, "play");
    assert_eq!((play.action.clone(), play.colour, play.shift_action, play.shift_label.as_str()), (Some(AppCmd::StartStop), None, Some(AppCmd::StartStop), "PLAY"));
    assert_eq!(b(&s, "scene").action, Some(AppCmd::TempoUp));
    // Panel faders: Right 1 on (blue), Right 2 off (dim blue), 5-8 do nothing.
    let f1 = b(&s, "faderButton1");
    assert_eq!((f1.label.as_str(), f1.action, f1.shift_action), ("RIGHT 1", Some(AppCmd::TogglePart { part: 0 }), Some(AppCmd::SelectPart { part: 0 })));
    assert_eq!((f1.level, f1.rgb), (Level::Bright, [0, 0, 127]));
    assert_eq!(b(&s, "faderButton2").level, Level::Dim);
    let f5 = b(&s, "faderButton5");
    assert_eq!((f5.action, f5.level), (None, Level::Off));
    assert_eq!(b(&s, "masterButton").label, "PANEL");
    // Style page: the Style parts' mutes, green.
    s.send(AppCmd::ToggleFaderPage).unwrap();
    s.send(AppCmd::ToggleStylePart { part: 5 }).unwrap();
    let f6 = b(&s, "faderButton6");
    assert_eq!((f6.label.as_str(), f6.action), ("PAD", Some(AppCmd::ToggleStylePart { part: 5 })));
    assert_eq!(f6.level, Level::Dim, "muted");
    assert_eq!((b(&s, "faderButton1").level, b(&s, "faderButton1").rgb), (Level::Bright, [0, 127, 0]));
    assert_eq!(b(&s, "masterButton").label, "STYLE");
    // Page 3: ▼ goes nowhere, ▲ back to page 2, both pink.
    s.send(AppCmd::SetPadPage { page: Page::OtsParts }).unwrap();
    assert_eq!(b(&s, "padBankDown").action, None);
    let up = b(&s, "padBankUp");
    assert_eq!(up.action, Some(AppCmd::SetPadPage { page: Page::ChordSetup }));
    assert_eq!(up.rgb, [127, 0, 70]);
    // Shift is mirrored while held.
    assert!(!s.state().surface.shift);
    s.midi_in(Port::Pads, &[0xB0, SHIFT_CC, 127]);
    assert!(s.state().surface.shift);
    s.midi_in(Port::Pads, &[0xB0, SHIFT_CC, 0]);
    assert!(!s.state().surface.shift);
}

/// Track ◀ / ▶ neighbours are where `StepStyle` goes, skipping files that don't load.
#[test]
fn track_neighbours() {
    let Some(p) = style("SlowWalker.T552.sty") else { return };
    let s = Session::offline(Options { paths: vec![p.parent().unwrap().to_path_buf()], ..Options::default() }).unwrap();
    s.finish_indexing();
    let st = s.state();
    let (prev, next) = (st.surface.track_prev.clone().unwrap(), st.surface.track_next.clone().unwrap());
    let lib = s.library();
    assert_eq!(next.id, lib.step(st.style.id, 1));
    assert_eq!(prev.id, lib.step(st.style.id, -1));
    assert_eq!(next.name, lib.entry(next.id).name());
    assert!(s.state().surface.controls.iter().any(|b| b.id == "trackNext" && b.action == Some(AppCmd::StepStyle { delta: 1 })));
    s.send(AppCmd::StepStyle { delta: 1 }).unwrap();
    assert_eq!(s.state().style.id, next.id);
    assert_eq!(s.state().surface.track_prev.as_ref().map(|n| n.id), Some(st.style.id));
    s.send(AppCmd::StepStyle { delta: -1 }).unwrap();
    s.send(AppCmd::StepStyle { delta: -1 }).unwrap();
    assert_eq!(s.state().style.id, prev.id);
    // A file that doesn't load is skipped once it is known not to.
    let bad = std::env::temp_dir().join(format!("yahaha-neighbour-{}.sty", std::process::id()));
    std::fs::write(&bad, b"not a style").unwrap();
    let _ = s.send(AppCmd::LoadStylePath { path: bad.display().to_string() });
    let _ = std::fs::remove_file(&bad);
    let st = s.state();
    for n in [&st.surface.track_prev, &st.surface.track_next] {
        assert!(n.as_ref().is_some_and(|n| n.path != bad.display().to_string()));
    }
    // One style: nowhere to go.
    let one = offline("SlowWalker.T552.sty").unwrap();
    assert_eq!((one.state().surface.track_prev.clone(), one.state().surface.track_next.clone()), (None, None));
}

/// The clock in the state extrapolates to what the engine reports later, and the LED clock
/// is the one the pads flash on.
#[test]
fn beat_clock_extrapolates() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    let ms = |s: &Session| ns_to_ms(s.now());
    let c = s.state().surface.clock.clone();
    assert!(!c.running);
    assert_eq!((c.bar, c.beat, c.phase), (1, 1, 0.0));
    assert_eq!(c.beats_per_bar, 4.0);
    keys(&s, true, &[36, 40, 43]);
    s.advance(300 * MS);
    let st = s.state();
    let c = st.surface.clock.clone();
    assert!(c.running);
    // `atMs` is when the state last changed (the chord, at 0): time alone changes nothing.
    assert!(c.at_ms <= ms(&s));
    assert_eq!((c.bar, c.beat), (st.transport.bar, st.transport.beat));
    // Predict ahead from a state alone, then check against the engine.
    for ahead in [2_300 * MS, 5_100 * MS] {
        let later = ns_to_ms(s.now() + ahead);
        let want = s.state().surface.clock.at(later);
        s.advance(ahead);
        let st = s.state();
        assert_eq!((want.bar, want.beat), (st.transport.bar, st.transport.beat), "{ahead}");
        let now = s.state_now().surface.clock;
        assert_eq!((now.at_ms, now.bar, now.beat), (later, st.transport.bar, st.transport.beat));
        assert!((now.phase - want.phase).abs() < 1e-6);
    }
    // Time passing alone publishes nothing new (within a beat, the transport is the same).
    let (v, before) = (s.version(), s.state().clone());
    s.advance(MS);
    if s.state().transport == before.transport {
        assert_eq!(s.version(), v);
    }
    // The pads' flash clock.
    let st = s.state();
    assert!((st.surface.clock.led_beats(ms(&s)) - s.beats()).abs() < 1e-6);
    // A tempo change re-anchors both clocks; they carry on from where they were.
    let led = st.surface.clock.led_beats(ms(&s));
    s.send(AppCmd::TempoUp).unwrap();
    let c2 = s.state().surface.clock.clone();
    assert!(c2.tempo > before.surface.clock.tempo);
    assert!((c2.led_beats(ms(&s)) - led).abs() < 1e-6);
    let later = ns_to_ms(s.now() + 1_000 * MS);
    let w = c2.at(later);
    s.advance(1_000 * MS);
    let st = s.state();
    assert_eq!((w.bar, w.beat), (st.transport.bar, st.transport.beat));
}

/// Physical fader positions sit next to the values; a software master move makes the
/// master fader wait again (masterWaiting was left stale).
#[test]
fn fader_positions_and_master_takeover() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    let st = s.state();
    assert!(st.surface.faders.iter().all(|f| f.position.is_none()));
    assert_eq!(st.surface.faders.len(), 9);
    assert_eq!((st.surface.faders[8].set.clone(), st.keyboard_parts[1].fader), (None, None), "no synth: no master");
    s.midi_in(Port::Pads, &[0xB0, 6, 30]); // fader 2 on Panel: Right 2, far from 100
    let st = s.state();
    let f = &st.surface.faders[1];
    assert_eq!((f.label.as_str(), f.value, f.waiting, f.position), ("RIGHT 2", Some(100), true, Some(30)));
    assert_eq!(f.set, Some(AppCmd::SetPartVolume { part: 1, volume: 0 }));
    assert_eq!((st.surface.faders[5].label.as_str(), st.surface.faders[5].set.clone()), ("", None), "fader 6 unused on Panel");
    assert_eq!(st.keyboard_parts[1].fader, Some(30));
    assert_eq!(st.mixer.style_parts[1].fader, Some(30), "the same physical fader");
    assert!(st.keyboard_parts[1].waiting && st.keyboard_parts[1].volume == 100);
    // The Style page shows the same physical fader with the Style part's level.
    s.send(AppCmd::SetFaderPage { page: FaderPage::Style }).unwrap();
    let f = s.state().surface.faders[1].clone();
    assert_eq!((f.label.as_str(), f.position, f.set), ("RHYTHM 2", Some(30), Some(AppCmd::SetStylePartVolume { part: 1, volume: 0 })));
    s.send(AppCmd::SetFaderPage { page: FaderPage::Panel }).unwrap();

    // A synth control, as a live session with the synth has.
    let ctl = Arc::new(SynthControl::new(0));
    {
        let mut c = s.inner.lock();
        c.offline.as_mut().unwrap().input.set_synth(Some(ctl.clone()));
        c.synth = Some(SynthRef {
            info: SynthInfo { name: "test".into(), sample_rate: 48000, buffer: None, device: "none".into(), channels: 2 },
            control: ctl.clone(),
        });
    }
    s.midi_in(Port::Pads, &[0xB0, 13, 100]); // at unity: picks up
    s.midi_in(Port::Pads, &[0xB0, 13, 90]);
    let st = s.state();
    assert_eq!((st.mixer.master, st.surface.faders[8].position, st.mixer.master_waiting), (Some(90), Some(90), false));
    assert_eq!((st.surface.faders[8].label.as_str(), st.surface.faders[8].value), ("MASTER", Some(90)));
    s.send(AppCmd::SetMasterVolume { volume: 40 }).unwrap();
    let st = s.state();
    assert_eq!((st.mixer.master, st.mixer.master_waiting, st.surface.faders[8].waiting), (Some(40), true, true), "the fader at 90 must come to 40");
    s.midi_in(Port::Pads, &[0xB0, 13, 80]);
    assert_eq!(s.state().mixer.master, Some(40), "no jump");
    s.midi_in(Port::Pads, &[0xB0, 13, 41]);
    let st = s.state();
    assert_eq!((st.mixer.master, st.mixer.master_waiting), (Some(41), false));
    // Set to where the fader already is: it keeps control.
    s.send(AppCmd::SetMasterVolume { volume: 42 }).unwrap();
    assert!(!s.state().mixer.master_waiting);
}

/// Palette-LED mode: each pad carries what the hardware was sent.
#[test]
fn palette_leds_are_described() {
    use crate::launchkey::Anim;
    let Some(p) = style("SlowWalker.T552.sty") else { return };
    let s = Session::offline(Options { paths: vec![p], palette_leds: true, ..Options::default() }).unwrap();
    let st = s.state();
    assert!(st.pads.palette_leds);
    let pad = |st: &AppState, n: u8| st.pads.pads.iter().find(|p| p.note == n).unwrap().palette.clone().unwrap();
    let stop = pad(&st, 119);
    assert_eq!((stop.mode, stop.colour, stop.level), (Anim::Solid, 7, Level::Dim), "dim red while stopped");
    let sync = pad(&st, 99);
    assert_eq!((sync.mode, sync.level), (Anim::Pulse, Level::Bright));
    keys(&s, true, &[36, 40, 43]);
    s.send(AppCmd::Main { index: 1 }).unwrap();
    let b = pad(&s.state(), 113);
    assert_eq!((b.mode, b.flash_level), (Anim::Flash, Some(Level::Bright)));
    assert!(st.transport.lamps.iter().all(|p| p.palette.is_some()));
    let rgb = offline("SlowWalker.T552.sty").unwrap();
    assert!(rgb.state().pads.pads.iter().all(|p| p.palette.is_none()));
}
