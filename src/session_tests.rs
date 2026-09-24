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
    println!("{}", serde_json::to_string_pretty(&*s.state()).unwrap());
}

#[test]
fn sync_start_sections_and_position() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    let st = s.state();
    assert!(!st.transport.running && st.transport.sync_start);
    assert_eq!(st.transport.section, None);
    assert_eq!((st.transport.bar, st.transport.beat), (1, 1));
    assert!(st.style.sections.iter().any(|n| n == "Main A"));
    assert_eq!(st.io.offline, true);

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
