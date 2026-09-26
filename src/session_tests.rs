//! Session tests: the API through an offline session (no CoreMIDI, no audio), driven by
//! its virtual clock.

use super::*;
use crate::launchkey::{Level, PAD_DOWN_CC, SHIFT_CC};
use crate::library;
use crate::parts::FaderPage;
use crate::synth::{SynthControl, SynthInfo};

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
    vec![
        AppCmd::Transport(TransportCmd::Intro { index: 1 }),
        AppCmd::Transport(TransportCmd::Main { index: 2 }),
        AppCmd::Transport(TransportCmd::Break),
        AppCmd::Transport(TransportCmd::Ending { index: 0 }),
        AppCmd::Transport(TransportCmd::StartStop),
        AppCmd::Transport(TransportCmd::Stop),
        AppCmd::Transport(TransportCmd::ToggleSyncStart),
        AppCmd::Transport(TransportCmd::ToggleSyncStop),
        AppCmd::Transport(TransportCmd::ToggleAutoFill),
        AppCmd::Transport(TransportCmd::ToggleStopAcmp),
        AppCmd::Transport(TransportCmd::TapTempo),
        AppCmd::Transport(TransportCmd::TempoUp),
        AppCmd::Transport(TransportCmd::TempoDown),
        AppCmd::Mixer(MixerCmd::ToggleStylePart { part: 5 }),
        AppCmd::Mixer(MixerCmd::SetStylePartVolume { part: 2, volume: 90 }),
        AppCmd::Chord(ChordCmd::SetFingering { fingering: Fingering::AiFullKeyboard }),
        AppCmd::Chord(ChordCmd::NextFingering),
        AppCmd::Chord(ChordCmd::SetUpper { on: true }),
        AppCmd::Chord(ChordCmd::ToggleUpper),
        AppCmd::Chord(ChordCmd::SetManualBass { on: false }),
        AppCmd::Chord(ChordCmd::ToggleManualBass),
        AppCmd::Chord(ChordCmd::SetSplit { note: 60 }),
        AppCmd::Chord(ChordCmd::MoveSplit { delta: -1 }),
        AppCmd::Chord(ChordCmd::SetTranspose { keyboard: 2, master: -1 }),
        AppCmd::Chord(ChordCmd::StepTranspose { keyboard: 1, master: 0 }),
        AppCmd::Chord(ChordCmd::ResetTranspose),
        AppCmd::Chord(ChordCmd::SetChordSettle { ms: 10 }),
        AppCmd::Parts(PartsCmd::SetPartOn { part: 1, on: true }),
        AppCmd::Parts(PartsCmd::TogglePart { part: 3 }),
        AppCmd::Parts(PartsCmd::SelectPart { part: 2 }),
        AppCmd::Parts(PartsCmd::SetPartVoice { part: 0, program: 4 }),
        AppCmd::Parts(PartsCmd::StepVoice { delta: -1 }),
        AppCmd::Parts(PartsCmd::SetPartVolume { part: 3, volume: 64 }),
        AppCmd::Parts(PartsCmd::SetPartOctave { part: 1, octave: -1 }),
        AppCmd::Mixer(MixerCmd::SetFaderPage { page: FaderPage::Style }),
        AppCmd::Mixer(MixerCmd::ToggleFaderPage),
        AppCmd::Pads(PadsCmd::SetPadPage { page: Page::OtsParts }),
        AppCmd::Pads(PadsCmd::CyclePadPage { delta: 1 }),
        AppCmd::Mixer(MixerCmd::SetMasterVolume { volume: 110 }),
        AppCmd::Ots(OtsCmd::RecallOts { index: 3 }),
        AppCmd::Ots(OtsCmd::SetOtsLink { on: true }),
        AppCmd::Ots(OtsCmd::ToggleOtsLink),
        AppCmd::Library(LibraryCmd::LoadStyle { id: 7 }),
        AppCmd::Library(LibraryCmd::LoadStylePath { path: "/styles/Funk.sty".into() }),
        AppCmd::Library(LibraryCmd::StepStyle { delta: 1 }),
        AppCmd::Mixer(MixerCmd::SetSynthMuted { on: true }),
        AppCmd::Mixer(MixerCmd::ToggleSynthMute),
        AppCmd::Settings(SettingsCmd::SetAudioOutput { first: 2 }),
        AppCmd::Settings(SettingsCmd::NextAudioOutput),
        AppCmd::System(SystemCmd::Panic),
        AppCmd::System(SystemCmd::ClearMessage),
        AppCmd::Transport(TransportCmd::ToggleFade),
        AppCmd::Transport(TransportCmd::SectionReset),
        AppCmd::Transport(TransportCmd::ToggleRetrigger),
        AppCmd::StyleSettings(StyleSettingsCmd::SetMainTiming { timing: crate::engine::MainTiming::Immediate }),
        AppCmd::StyleSettings(StyleSettingsCmd::SetFadeOutTime { ms: 1200 }),
        AppCmd::StyleSettings(StyleSettingsCmd::StepRetriggerRate { delta: 1 }),
        AppCmd::Transport(TransportCmd::SetTempo { bpm: 480 }),
        AppCmd::Mixer(MixerCmd::SetStyleSolo { part: Some(3) }),
        AppCmd::Mixer(MixerCmd::SetPartSolo { part: None }),
        AppCmd::Mixer(MixerCmd::StyleTrackMute { order: TrackMuteOrder::B, value: 64 }),
        AppCmd::Looper(LooperCmd::LooperRec),
        AppCmd::Looper(LooperCmd::LooperOnOff),
        AppCmd::Looper(LooperCmd::SelectLooperMemory { index: 1 }),
        AppCmd::Looper(LooperCmd::StoreLooperMemory { index: 1 }),
        AppCmd::Looper(LooperCmd::ClearLooperMemory { index: 1 }),
        AppCmd::Looper(LooperCmd::NewLooperBank),
        AppCmd::Metronome(MetronomeCmd::ToggleMetronome),
        AppCmd::Metronome(MetronomeCmd::SetMetronome { on: true }),
        AppCmd::Metronome(MetronomeCmd::SetMetronomeVolume { volume: 64 }),
        AppCmd::Metronome(MetronomeCmd::SetMetronomeBell { on: false }),
        AppCmd::Dynamics(DynamicsCmd::SetDynamics { level: 90 }),
        AppCmd::Dynamics(DynamicsCmd::ToggleAccent),
        AppCmd::Knobs(KnobsCmd::StepKnobPage { delta: 1 }),
        AppCmd::Knobs(KnobsCmd::TurnKnob { knob: 0, delta: -3 }),
    ]
}

#[test]
fn commands_serialize_as_tagged_camel_case() {
    for c in all_cmds() {
        let j = serde_json::to_string(&c).unwrap();
        assert_eq!(serde_json::from_str::<AppCmd>(&j).unwrap(), c, "{j}");
    }
    let j = |c: AppCmd| serde_json::to_string(&c).unwrap();
    assert_eq!(j(AppCmd::Transport(TransportCmd::Main { index: 1 })), r#"{"type":"main","index":1}"#);
    assert_eq!(j(AppCmd::Transport(TransportCmd::StartStop)), r#"{"type":"startStop"}"#);
    assert_eq!(j(AppCmd::Mixer(MixerCmd::SetStylePartVolume { part: 2, volume: 90 })), r#"{"type":"setStylePartVolume","part":2,"volume":90}"#);
    assert_eq!(j(AppCmd::Chord(ChordCmd::SetFingering { fingering: Fingering::FingeredOnBass })), r#"{"type":"setFingering","fingering":"fingeredOnBass"}"#);
    assert_eq!(j(AppCmd::Pads(PadsCmd::SetPadPage { page: Page::ChordSetup })), r#"{"type":"setPadPage","page":"chordSetup"}"#);
    let e = serde_json::to_string(&Event::StateChanged { version: 3 }).unwrap();
    assert_eq!(e, r#"{"type":"stateChanged","version":3}"#);
}

#[test]
fn app_state_round_trips_through_json() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    s.send(OtsCmd::RecallOts { index: 0 }).unwrap();
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
    s.send(OtsCmd::RecallOts { index: 0 }).unwrap();
    keys(&s, true, &[45, 48, 52, 55]);
    s.advance(2300 * MS);
    s.send(TransportCmd::Main { index: 1 }).unwrap();
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
    s.send(TransportCmd::Main { index: 1 }).unwrap();
    assert_eq!(s.state().transport.queued.as_deref(), Some("Fill In BB"));
    s.advance(2 * bar);
    let st = s.state();
    assert_eq!(st.transport.section.as_deref(), Some("Main B"));
    assert_eq!(st.transport.main, 1);
    // Its lamp is lit on page 1 (the pad at note 113).
    let lamp = st.transport.lamps.iter().find(|p| p.note == 113).unwrap();
    assert_eq!((lamp.label.as_str(), lamp.level), ("MAIN B", Level::Bright));
    assert_eq!(lamp.action, Some(AppCmd::Transport(TransportCmd::Main { index: 1 })));

    let t = st.transport.tempo;
    s.send(TransportCmd::TempoUp).unwrap();
    assert!(s.state().transport.tempo > t);
    s.send(TransportCmd::StartStop).unwrap();
    assert!(!s.state().transport.running);
    s.send(SystemCmd::Panic).unwrap();
}

#[test]
fn chord_settings_split_and_transpose() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    s.send(ChordCmd::SetFingering { fingering: Fingering::FullKeyboard }).unwrap();
    let st = s.state();
    assert_eq!(st.chord.fingering, Fingering::FullKeyboard);
    assert_eq!(st.chord.fingering_name, "Full Keyboard");
    assert!(!st.transport.sync_stop_available);
    s.send(ChordCmd::NextFingering).unwrap();
    assert_eq!(s.state().chord.fingering, Fingering::AiFullKeyboard);

    // Upper turns Manual Bass on: the Style's Bass part is muted, Left plays the bass.
    s.send(ChordCmd::SetUpper { on: true }).unwrap();
    let st = s.state();
    assert!(st.chord.upper && st.chord.manual_bass && st.chord.manual_bass_active);
    assert!(st.transport.sync_stop_available, "Upper is Fingered*");
    assert!(st.mixer.style_parts[2].muted_by_manual_bass && !st.mixer.style_parts[2].on);
    assert!(st.keyboard_parts[3].plays_bass && st.keyboard_parts[3].sounding);
    // Left can't be switched then: refused, with a message.
    let err = s.send(PartsCmd::TogglePart { part: 3 }).unwrap_err();
    assert!(matches!(err, CmdError::Failed(_)));
    assert!(s.state().message.as_ref().is_some_and(|m| m.error && m.text.contains("Manual Bass")));
    s.send(SystemCmd::ClearMessage).unwrap();
    assert_eq!(s.state().message, None);
    s.send(ChordCmd::ToggleManualBass).unwrap();
    assert!(!s.state().chord.manual_bass_active);
    s.send(ChordCmd::ToggleUpper).unwrap();
    let st = s.state();
    assert!(!st.chord.upper && !st.chord.manual_bass_active);
    // Manual Bass is Upper-only.
    s.send(ChordCmd::SetManualBass { on: true }).unwrap();
    assert!(!s.state().chord.manual_bass);

    assert_eq!((s.state().chord.split, s.state().chord.split_name.as_str()), (54, "F#2"));
    s.send(ChordCmd::MoveSplit { delta: 2 }).unwrap();
    assert_eq!(s.state().chord.split_name, "Ab2");
    s.send(ChordCmd::SetSplit { note: 200 }).unwrap();
    assert_eq!(s.state().chord.split, 96);

    s.send(ChordCmd::StepTranspose { keyboard: 1, master: -2 }).unwrap();
    s.send(ChordCmd::StepTranspose { keyboard: 1, master: 0 }).unwrap();
    let st = s.state();
    assert_eq!((st.chord.transpose_keyboard, st.chord.transpose_master), (2, -2));
    s.send(ChordCmd::SetTranspose { keyboard: 20, master: 0 }).unwrap();
    assert_eq!(s.state().chord.transpose_keyboard, 12);
    s.send(ChordCmd::ResetTranspose).unwrap();
    assert_eq!(s.state().chord.transpose_keyboard, 0);

    // The chord-settle window: the default, clamped to its range, and 0.
    assert_eq!(s.state().chord.settle_ms, crate::engine::CHORD_SETTLE_DEFAULT_MS);
    s.send(ChordCmd::SetChordSettle { ms: 500 }).unwrap();
    assert_eq!(s.state().chord.settle_ms, crate::engine::CHORD_SETTLE_MAX_MS);
    s.send(ChordCmd::SetChordSettle { ms: 0 }).unwrap();
    assert_eq!(s.state().chord.settle_ms, 0);
}

/// Transpose reaches the notes you play: a key sounds shifted on its part's channel.
#[test]
fn transpose_and_parts_reach_the_keys() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    s.take_output();
    s.send(ChordCmd::SetTranspose { keyboard: 2, master: 0 }).unwrap();
    s.send(PartsCmd::SetPartOn { part: 1, on: true }).unwrap();
    s.send(PartsCmd::SetPartOctave { part: 1, octave: 1 }).unwrap();
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

    s.send(PartsCmd::SelectPart { part: 1 }).unwrap();
    s.send(PartsCmd::SetPartVoice { part: 1, program: 40 }).unwrap();
    s.send(PartsCmd::StepVoice { delta: 1 }).unwrap();
    s.send(PartsCmd::SetPartVolume { part: 1, volume: 70 }).unwrap();
    s.send(PartsCmd::SetPartOctave { part: 1, octave: 5 }).unwrap();
    let p = s.state().keyboard_parts[1].clone();
    assert!(p.selected);
    assert_eq!((p.program, p.voice_name.as_str(), p.volume, p.octave), (41, "Viola", 70, 2));
    // The engine sends the volume as the part's CC7.
    assert!(s.take_output().contains(&[0xB2, 7, 70]));

    s.send(MixerCmd::SetStylePartVolume { part: 5, volume: 33 }).unwrap();
    s.send(MixerCmd::ToggleStylePart { part: 6 }).unwrap();
    let st = s.state();
    assert_eq!(st.mixer.style_parts[5].volume, 33);
    assert!(!st.mixer.style_parts[6].on);
    assert_eq!(st.mixer.style_parts[2].name, "Bass");
    assert_eq!(st.mixer.style_parts[2].channel, 11);
    assert!(st.mixer.style_parts[2].voice.is_some());

    assert_eq!(st.mixer.fader_page, FaderPage::Panel);
    s.send(MixerCmd::ToggleFaderPage).unwrap();
    assert_eq!(s.state().mixer.fader_page, FaderPage::Style);
    s.send(MixerCmd::SetFaderPage { page: FaderPage::Panel }).unwrap();
    assert_eq!(s.state().mixer.fader_page, FaderPage::Panel);
    assert_eq!(st.mixer.master, None, "no synth offline");
    assert!(s.send(MixerCmd::SetMasterVolume { volume: 90 }).is_err());

    assert_eq!(st.pads.page, Page::Sections);
    s.send(PadsCmd::CyclePadPage { delta: -3 }).unwrap();
    let st = s.state();
    assert_eq!((st.pads.page, st.pads.page_number, st.pads.page_count), (Page::OtsParts, 3, 5));
    assert_eq!(st.pads.pads.len(), 16);
    assert_eq!(st.pads.pads[0].label, "OTS 1");
    assert_eq!(st.pads.pads[0].action, Some(AppCmd::Ots(OtsCmd::RecallOts { index: 0 })));
    s.send(PadsCmd::SetPadPage { page: Page::ChordSetup }).unwrap();
    assert_eq!(s.state().pads.pads[1].action, Some(AppCmd::Chord(ChordCmd::SetFingering { fingering: Fingering::Fingered })));
}

/// Pan and the reverb/chorus sends (#198): the part's CC10/91/93 on its own channel, to
/// the port and the synth, and in the state. Before anything sets them the state shows the
/// power-on values.
#[test]
fn part_pan_and_sends() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    let left = &s.state().keyboard_parts[crate::parts::LEFT];
    assert_eq!((left.pan, left.reverb, left.chorus), (64, 40, 0));
    s.take_output();
    s.send(PartsCmd::SetPartPan { part: 3, pan: 20 }).unwrap();
    s.send(PartsCmd::SetPartSend { part: 3, send: PartSend::Reverb, value: 90 }).unwrap();
    s.send(PartsCmd::SetPartSend { part: 1, send: PartSend::Chorus, value: 200 }).unwrap();
    let out = s.take_output();
    assert!(out.contains(&[0xB1, 10, 20]), "Left is ch 2: {out:?}");
    assert!(out.contains(&[0xB1, 91, 90]), "{out:?}");
    assert!(out.contains(&[0xB2, 93, 127]), "Right 2 is ch 3, clamped: {out:?}");
    assert!(!out.contains(&[0xB1, 93, 0]), "a send not set is not sent: {out:?}");
    let st = s.state();
    let (l, r2) = (&st.keyboard_parts[3], &st.keyboard_parts[1]);
    assert_eq!((l.pan, l.reverb, l.chorus), (20, 90, 0));
    assert_eq!((r2.pan, r2.reverb, r2.chorus), (64, 40, 127));
}

#[test]
fn ots_and_ots_link() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    let st = s.state();
    assert!(st.ots.settings.len() >= 2);
    assert_eq!(st.ots.settings[0].name, "OTS 1");
    assert_eq!(st.ots.applied, 0);
    s.take_output();
    s.send(OtsCmd::RecallOts { index: 0 }).unwrap();
    // Its pan and reverb/chorus sends go out on the parts' channels too (#198).
    let fx = crate::sff::Style::load(&style("SlowWalker.T552.sty").unwrap()).unwrap().ots[0].parts[0].fx;
    let out = s.take_output();
    for (cc, v) in crate::parts::FX_CC.into_iter().zip(fx) {
        if let Some(v) = v {
            assert!(out.contains(&[0xB0, cc, v]), "Right 1 CC{cc} {v}: {out:?}");
        }
    }
    assert!(fx[0].is_some(), "SlowWalker's OTS 1 sets Right 1's pan");
    let st = s.state();
    assert_eq!(st.ots.applied, 1);
    assert_eq!(st.keyboard_parts.iter().map(|p| p.on).collect::<Vec<_>>(), vec![true, true, false, true]);
    assert_eq!(st.keyboard_parts[0].program, 80);

    // OTS Link: Main B recalls OTS 2.
    s.send(OtsCmd::SetOtsLink { on: true }).unwrap();
    assert_eq!(s.state().ots.applied, 1, "Main A: OTS 1");
    s.send(TransportCmd::Main { index: 1 }).unwrap();
    let st = s.state();
    assert!(st.ots.link);
    assert_eq!(st.ots.applied, 2);
    s.send(OtsCmd::ToggleOtsLink).unwrap();
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
    s.send(LibraryCmd::StepStyle { delta: 1 }).unwrap();
    let st = s.state();
    assert_ne!(st.style.id, first.id);
    assert_eq!(st.library.position, 1);
    let e = lib.entries.iter().find(|e| e.status == "ok" && e.id != st.style.id).unwrap();
    s.send(LibraryCmd::LoadStyle { id: e.id }).unwrap();
    assert_eq!(s.state().style.id, e.id);
    s.send(LibraryCmd::LoadStylePath { path: p.display().to_string() }).unwrap();
    assert!(s.state().style.path.ends_with("SlowWalker.T552.sty"));
    // A file that doesn't load: an error, a message, and the entry is marked.
    let rx = s.subscribe();
    let bad = std::env::temp_dir().join(format!("yahaha-bad-{}.sty", std::process::id()));
    std::fs::write(&bad, b"not a style").unwrap();
    let r = s.send(LibraryCmd::LoadStylePath { path: bad.display().to_string() });
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
    s.send(PadsCmd::SetPadPage { page: Page::Sections }).unwrap();
    s.midi_in(Port::Pads, &[0x90, 119, 100]);
    assert!(s.state().transport.running);
    // What a pad's `action` says is what pressing it does.
    let stop = s.state().pads.pads.iter().find(|p| p.note == 119).unwrap().action.clone().unwrap();
    s.send(stop).unwrap();
    assert!(!s.state().transport.running);
    // Unmapped controls show up for diagnosis.
    s.midi_in(Port::Pads, &[0xB0, 53, 127]);
    assert_eq!(s.state().io.unmapped, "unmapped CC 53 = 127");
    // Knob 1 (an encoder on channel 16) turns Dynamics; the encoder page ▼ steps the
    // Knob Assign page.
    s.midi_in(Port::Pads, &[0xBF, 21, 67]);
    assert_eq!(s.state().dynamics.level, 70);
    s.midi_in(Port::Pads, &[0xB0, 52, 127]);
    assert_eq!(s.state().knobs.page_name, "Parts");
}

/// Style faders move the Style parts (soft takeover); a software move makes the fader
/// pick the part up again.
#[test]
fn style_faders_and_software_volume() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    s.send(MixerCmd::SetFaderPage { page: FaderPage::Style }).unwrap();
    let v0 = s.state().mixer.style_parts[0].volume;
    s.midi_in(Port::Pads, &[0xB0, 5, v0]);
    s.midi_in(Port::Pads, &[0xB0, 5, 50]);
    assert_eq!(s.state().mixer.style_parts[0].volume, 50);
    s.send(MixerCmd::SetStylePartVolume { part: 0, volume: 100 }).unwrap();
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
    s.send(SystemCmd::ClearMessage).unwrap();
    assert_eq!(s.version(), v, "nothing changed, same version");
    s.send(ChordCmd::MoveSplit { delta: 1 }).unwrap();
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
        // The index thread would otherwise land in one twin and not the other under load.
        s.finish_indexing();
        match setup {
            0 => {}
            1 => {
                s.send(OtsCmd::SetOtsLink { on: true }).unwrap();
                s.send(ChordCmd::SetUpper { on: true }).unwrap();
                keys(&s, true, &[60, 64, 67]);
                s.advance(700 * MS);
            }
            _ => {
                s.send(OtsCmd::RecallOts { index: 1 }).unwrap();
                keys(&s, true, &[36, 40, 43]);
                s.advance(1300 * MS);
                s.send(TransportCmd::Main { index: 2 }).unwrap();
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
        let (na, nb) = (norm(&a), norm(&b));
        if na != nb {
            let (ja, jb) = (serde_json::to_value(&na.0).unwrap(), serde_json::to_value(&nb.0).unwrap());
            let mut d = Vec::new();
            if let (Some(oa), Some(ob)) = (ja.as_object(), jb.as_object()) {
                for (k, v) in oa {
                    if ob.get(k) != Some(v) {
                        d.push(format!("{k}: {v} VS {:?}", ob.get(k)));
                    }
                }
            }
            if na.1 != nb.1 {
                d.push(format!("output {:?} VS {:?}", na.1, nb.1));
            }
            bad.push(format!("setup {setup}: {what}: {}", d.join(" | ")));
        }
    };
    for setup in 0..3 {
        for page in Page::ALL {
            let prep = move |s: &Session| s.send(PadsCmd::SetPadPage { page }).unwrap();
            let pads = {
                let s = mk(setup);
                prep(&s);
                s.state().pads.pads.clone()
            };
            for pad in pads {
                pair(setup, format!("{page:?} pad {} {:?}", pad.note, pad.action), &prep, &[&[0x90, pad.note, 100]], pad.action);
            }
        }
        let page2 = |s: &Session| s.send(PadsCmd::SetPadPage { page: Page::ChordSetup }).unwrap();
        for (cc, shift, cmd) in [
            (PLAY_CC, false, AppCmd::Transport(TransportCmd::StartStop)),
            (STOP_CC, false, AppCmd::Transport(TransportCmd::Stop)),
            (SCENE_CC, false, AppCmd::Transport(TransportCmd::TempoUp)),
            (FUNCTION_CC, false, AppCmd::Transport(TransportCmd::TempoDown)),
            (TRACK_LEFT_CC, false, AppCmd::Library(LibraryCmd::StepStyle { delta: -1 })),
            (TRACK_RIGHT_CC, false, AppCmd::Library(LibraryCmd::StepStyle { delta: 1 })),
            (PAD_UP_CC, true, AppCmd::Parts(PartsCmd::TogglePart { part: 3 })),
            (PAD_DOWN_CC, true, AppCmd::Ots(OtsCmd::ToggleOtsLink)),
            (PAD_UP_CC, false, AppCmd::Pads(PadsCmd::CyclePadPage { delta: -1 })),
            (PAD_DOWN_CC, false, AppCmd::Pads(PadsCmd::CyclePadPage { delta: 1 })),
        ] {
            let btn = [0xB0, cc, 127];
            let hw: Vec<&[u8]> = if shift { vec![&[0xB0, SHIFT_CC, 127], &btn, &[0xB0, SHIFT_CC, 0]] } else { vec![&btn] };
            pair(setup, format!("CC {cc} shift {shift}"), &page2, &hw, Some(cmd));
        }
        for i in 0..9u8 {
            for shift in [false, true] {
                for fp in [FaderPage::Panel, FaderPage::Style] {
                    let cmd = match (i, fp, shift) {
                        (8, _, _) => Some(AppCmd::Mixer(MixerCmd::ToggleFaderPage)),
                        (0..=3, FaderPage::Panel, true) => Some(AppCmd::Parts(PartsCmd::SelectPart { part: i })),
                        (0..=3, FaderPage::Panel, false) => Some(AppCmd::Parts(PartsCmd::TogglePart { part: i })),
                        (4, FaderPage::Panel, _) => Some(AppCmd::HarmonyArp(HarmonyArpCmd::ToggleHarmonyArp)),
                        (5, FaderPage::Panel, _) => Some(AppCmd::Plugins(crate::api::PluginCmd::ReloadPartPlugin { part: None })),
                        (6, FaderPage::Panel, _) => Some(AppCmd::Chord(ChordCmd::ToggleLeftHold)),
                        (7, FaderPage::Panel, false) => Some(AppCmd::Looper(crate::api::LooperCmd::LooperOnOff)),
                        (7, FaderPage::Panel, true) => Some(AppCmd::Looper(crate::api::LooperCmd::LooperRec)),
                        (_, FaderPage::Panel, _) => None,
                        (_, FaderPage::Style, _) => Some(AppCmd::Mixer(MixerCmd::ToggleStylePart { part: i })),
                    };
                    let btn = [0xB0, 37 + i, 127];
                    let hw: Vec<&[u8]> = if shift { vec![&[0xB0, SHIFT_CC, 127], &btn, &[0xB0, SHIFT_CC, 0]] } else { vec![&btn] };
                    let prep = move |s: &Session| s.send(MixerCmd::SetFaderPage { page: fp }).unwrap();
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
    s.send(PadsCmd::SetPadPage { page: Page::OtsParts }).unwrap();
    s.send(PadsCmd::CyclePadPage { delta: 127 }).unwrap(); // 2 + 127 = 129 = 4 mod 5
    assert_eq!(s.state().pads.page, Page::MultiPads);
    s.send(PadsCmd::CyclePadPage { delta: -128 }).unwrap(); // 4 - 128 = -124 = 1 mod 5
    assert_eq!(s.state().pads.page, Page::ChordSetup);
    s.send(PadsCmd::CyclePadPage { delta: -2 }).unwrap();
    assert_eq!(s.state().pads.page, Page::MultiPads);
}

/// While the library indexes, `library_list()` is labelled with the revision its entries
/// are, and the state's library status describes that same list.
#[test]
fn library_list_revision_matches_its_entries() {
    let Some(p) = style("SlowWalker.T552.sty") else { return };
    let root = p.parent().unwrap().parent().unwrap().to_path_buf();
    let s = Session::offline(Options { paths: vec![root], ..Options::default() }).unwrap();
    for _ in 0..5000 {
        s.send(SystemCmd::ClearMessage).unwrap(); // the control side applies index results
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
    s.send(ChordCmd::SetSplit { note: 70 }).unwrap();
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
        s.send(PadsCmd::SetPadPage { page }).unwrap();
        s.send(MixerCmd::SetFaderPage { page: fp }).unwrap();
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
    assert_eq!((up.shift_label.as_str(), up.shift_action), ("LEFT", Some(AppCmd::Parts(PartsCmd::TogglePart { part: 3 }))));
    let down = b(&s, "padBankDown");
    assert_eq!(down.action, Some(AppCmd::Pads(PadsCmd::SetPadPage { page: Page::ChordSetup })));
    assert_eq!((down.colour, down.level), (Some(3), Level::Bright), "white on page 1");
    assert_eq!(down.shift_action, Some(AppCmd::Ots(OtsCmd::ToggleOtsLink)));
    // One style: the Track buttons go nowhere and are dark.
    let tl = b(&s, "trackPrev");
    assert_eq!((tl.action, tl.level, tl.shift_action), (None, Level::Off, None));
    let play = b(&s, "play");
    assert_eq!((play.action.clone(), play.colour, play.shift_action, play.shift_label.as_str()), (Some(AppCmd::Transport(TransportCmd::StartStop)), None, Some(AppCmd::Transport(TransportCmd::SectionReset)), "RESET"));
    let stop = b(&s, "stop");
    assert_eq!((stop.shift_action, stop.shift_label.as_str()), (Some(AppCmd::Transport(TransportCmd::ToggleFade)), "FADE"));
    assert_eq!(b(&s, "scene").shift_action, Some(AppCmd::StyleSettings(StyleSettingsCmd::StepRetriggerRate { delta: 1 })));
    assert_eq!(b(&s, "scene").action, Some(AppCmd::Transport(TransportCmd::TempoUp)));
    assert_eq!((b(&s, "scene").label.as_str(), b(&s, "function").label.as_str()), ("TEMPO +", "TEMPO -"));
    // Panel faders: Right 1 on (blue), Right 2 off (dim blue).
    let f1 = b(&s, "faderButton1");
    assert_eq!((f1.label.as_str(), f1.action, f1.shift_action), ("RIGHT 1", Some(AppCmd::Parts(PartsCmd::TogglePart { part: 0 })), Some(AppCmd::Parts(PartsCmd::SelectPart { part: 0 }))));
    assert_eq!((f1.level, f1.rgb), (Level::Bright, [0, 0, 127]));
    assert_eq!(b(&s, "faderButton2").level, Level::Dim);
    // Button 5: HARMONY/ARPEGGIO, dim purple while off, bright while on. Button 6 reloads
    // the selected part's plugin (dark while there is nothing to reload); 7 is Left Hold;
    // 8 is the Chord Looper (ON/OFF, Shift: REC/STOP; dark with nothing recorded).
    let f5 = b(&s, "faderButton5");
    assert_eq!((f5.label.as_str(), f5.action, f5.level), ("HARM/ARP", Some(AppCmd::HarmonyArp(HarmonyArpCmd::ToggleHarmonyArp)), Level::Dim));
    s.send(HarmonyArpCmd::ToggleHarmonyArp).unwrap();
    assert_eq!((b(&s, "faderButton5").level, b(&s, "faderButton5").rgb), (Level::Bright, [90, 0, 127]));
    s.send(HarmonyArpCmd::ToggleHarmonyArp).unwrap();
    let f6 = b(&s, "faderButton6");
    assert_eq!((f6.label.as_str(), f6.action, f6.level), ("PLUGIN", Some(AppCmd::Plugins(crate::api::PluginCmd::ReloadPartPlugin { part: None })), Level::Off));
    let f7 = b(&s, "faderButton7");
    assert_eq!((f7.label.as_str(), f7.action, f7.level), ("L HOLD", Some(AppCmd::Chord(ChordCmd::ToggleLeftHold)), Level::Dim));
    s.send(ChordCmd::ToggleLeftHold).unwrap();
    assert_eq!((b(&s, "faderButton7").level, b(&s, "faderButton7").rgb), (Level::Bright, [127, 60, 0]));
    s.send(ChordCmd::ToggleLeftHold).unwrap();
    let f8 = b(&s, "faderButton8");
    assert_eq!((f8.label.as_str(), f8.action, f8.shift_action, f8.level), ("LOOPER", Some(AppCmd::Looper(LooperCmd::LooperOnOff)), Some(AppCmd::Looper(LooperCmd::LooperRec)), Level::Off));
    assert_eq!(b(&s, "masterButton").label, "PANEL");
    // Style page: the Style parts' mutes, green.
    s.send(MixerCmd::ToggleFaderPage).unwrap();
    s.send(MixerCmd::ToggleStylePart { part: 5 }).unwrap();
    let f6 = b(&s, "faderButton6");
    assert_eq!((f6.label.as_str(), f6.action), ("PAD", Some(AppCmd::Mixer(MixerCmd::ToggleStylePart { part: 5 }))));
    assert_eq!(f6.level, Level::Dim, "muted");
    assert_eq!((b(&s, "faderButton1").level, b(&s, "faderButton1").rgb), (Level::Bright, [0, 127, 0]));
    assert_eq!(b(&s, "masterButton").label, "STYLE");
    // Manual Bass (with Upper) mutes the Style's Bass part: its button dims, as on the
    // hardware.
    s.send(ChordCmd::SetUpper { on: true }).unwrap();
    s.send(ChordCmd::SetManualBass { on: true }).unwrap();
    let f3 = b(&s, "faderButton3");
    assert_eq!((f3.level, f3.rgb), (Level::Dim, [0, 127, 0]), "Manual Bass");
    s.send(ChordCmd::SetManualBass { on: false }).unwrap();
    assert_eq!(b(&s, "faderButton3").level, Level::Bright);
    // Page 3: ▼ to page 4 (Registration), ▲ back to page 2, both pink.
    s.send(PadsCmd::SetPadPage { page: Page::OtsParts }).unwrap();
    assert_eq!(b(&s, "padBankDown").action, Some(AppCmd::Pads(PadsCmd::SetPadPage { page: Page::Registration })));
    let up = b(&s, "padBankUp");
    assert_eq!(up.action, Some(AppCmd::Pads(PadsCmd::SetPadPage { page: Page::ChordSetup })));
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
    assert!(s.state().surface.controls.iter().any(|b| b.id == "trackNext" && b.action == Some(AppCmd::Library(LibraryCmd::StepStyle { delta: 1 }))));
    s.send(LibraryCmd::StepStyle { delta: 1 }).unwrap();
    assert_eq!(s.state().style.id, next.id);
    assert_eq!(s.state().surface.track_prev.as_ref().map(|n| n.id), Some(st.style.id));
    s.send(LibraryCmd::StepStyle { delta: -1 }).unwrap();
    s.send(LibraryCmd::StepStyle { delta: -1 }).unwrap();
    assert_eq!(s.state().style.id, prev.id);
    // A file that doesn't load is skipped once it is known not to.
    let bad = std::env::temp_dir().join(format!("yahaha-neighbour-{}.sty", std::process::id()));
    std::fs::write(&bad, b"not a style").unwrap();
    let _ = s.send(LibraryCmd::LoadStylePath { path: bad.display().to_string() });
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
    s.send(TransportCmd::TempoUp).unwrap();
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
    assert_eq!(f.set, Some(AppCmd::Parts(PartsCmd::SetPartVolume { part: 1, volume: 0 })));
    assert_eq!((st.surface.faders[5].label.as_str(), st.surface.faders[5].set.clone()), ("", None), "fader 6 unused on Panel");
    assert_eq!(st.keyboard_parts[1].fader, Some(30));
    assert_eq!(st.mixer.style_parts[1].fader, Some(30), "the same physical fader");
    assert!(st.keyboard_parts[1].waiting && st.keyboard_parts[1].volume == 100);
    // The Style page shows the same physical fader with the Style part's level.
    s.send(MixerCmd::SetFaderPage { page: FaderPage::Style }).unwrap();
    let f = s.state().surface.faders[1].clone();
    assert_eq!((f.label.as_str(), f.position, f.set), ("RHYTHM 2", Some(30), Some(AppCmd::Mixer(MixerCmd::SetStylePartVolume { part: 1, volume: 0 }))));
    s.send(MixerCmd::SetFaderPage { page: FaderPage::Panel }).unwrap();

    // A synth control, as a live session with the synth has.
    let ctl = Arc::new(SynthControl::new(0));
    {
        let mut c = s.inner.lock();
        c.offline.as_mut().unwrap().input.set_synth(Some(ctl.clone()));
        c.synth = Some(SynthRef {
            info: SynthInfo { name: "test".into(), sample_rate: 48000, buffer: None, device: "none".into(), channels: 2 },
            control: ctl.clone(),
            swap: None,
            plugins: None,
            thread: None,
        });
    }
    s.midi_in(Port::Pads, &[0xB0, 13, 100]); // at unity: picks up
    s.midi_in(Port::Pads, &[0xB0, 13, 90]);
    let st = s.state();
    assert_eq!((st.mixer.master, st.surface.faders[8].position, st.mixer.master_waiting), (Some(90), Some(90), false));
    assert_eq!((st.surface.faders[8].label.as_str(), st.surface.faders[8].value), ("MASTER", Some(90)));
    s.send(MixerCmd::SetMasterVolume { volume: 40 }).unwrap();
    let st = s.state();
    assert_eq!((st.mixer.master, st.mixer.master_waiting, st.surface.faders[8].waiting), (Some(40), true, true), "the fader at 90 must come to 40");
    s.midi_in(Port::Pads, &[0xB0, 13, 80]);
    assert_eq!(s.state().mixer.master, Some(40), "no jump");
    s.midi_in(Port::Pads, &[0xB0, 13, 41]);
    let st = s.state();
    assert_eq!((st.mixer.master, st.mixer.master_waiting), (Some(41), false));
    // Set to where the fader already is: it keeps control.
    s.send(MixerCmd::SetMasterVolume { volume: 42 }).unwrap();
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
    s.send(TransportCmd::Main { index: 1 }).unwrap();
    let b = pad(&s.state(), 113);
    assert_eq!((b.mode, b.flash_level), (Anim::Flash, Some(Level::Bright)));
    assert!(st.transport.lamps.iter().all(|p| p.palette.is_some()));
    let rgb = offline("SlowWalker.T552.sty").unwrap();
    assert!(rgb.state().pads.pads.iter().all(|p| p.palette.is_none()));
}

/// A style change while playing, to a style of another resolution (SlowWalker is 960 ticks
/// per quarter, TickingAway 480): it waits for the next bar line, as on a Genos, then the
/// band carries on in the same section at the same bar position, at the same tempo, and
/// the clock runs on as if nothing had changed.
#[test]
fn style_change_keeps_tempo_across_resolutions() {
    let Some(p) = style("SlowWalker.T552.sty") else { return };
    let Some(other) = style("TickingAway.T162.sty") else { return };
    let s = Session::offline(Options { paths: vec![p], ..Options::default() }).unwrap();
    keys(&s, true, &[36, 40, 43]);
    s.advance(1_000 * MS);
    let c = s.state_now().surface.clock;
    let (t0, tempo) = (ns_to_ms(s.now()), c.tempo);
    let first = s.state().style.clone();
    s.send(LibraryCmd::LoadStylePath { path: other.display().to_string() }).unwrap();
    let st = s.state();
    assert_eq!(st.style.path, first.path, "the style waits for the bar line");
    assert!(st.preview.queued.is_some_and(|id| id != first.id));
    // Past the next bar line.
    let bar_ms = c.beats_per_bar * 60e3 / tempo;
    let to_bar = bar_ms - (c.at(t0).beat as f64 - 1.0 + c.at(t0).phase) * 60e3 / tempo;
    s.advance((to_bar + 30.0) as u64 * MS);
    let st = s.state_now();
    assert!(st.style.path.ends_with("TickingAway.T162.sty"));
    assert_eq!(st.preview.queued, None);
    assert_eq!(st.transport.section.as_deref(), Some("Main A"));
    assert_eq!(st.transport.tempo, tempo);
    // 5 beats later, less a hair so no beat boundary is at stake: the clock never jumped.
    let ahead = (5.0 * 60e3 / tempo - 20.0) as u64 * MS;
    let t1 = ns_to_ms(s.now());
    s.advance(ahead);
    let st = s.state_now();
    let want = c.at(t1 + ns_to_ms(ahead));
    assert_eq!((st.transport.bar, st.transport.beat), (want.bar, want.beat), "the bar position carries on");
}

/// The hardware Track ◀/▶ LEDs are re-sent when the library gains its second style, as the
/// mirror shows them.
#[test]
fn track_leds_follow_the_library() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    let snap = s.inner.lock().snap;
    let mut leds = Leds::new(PacketSink::new(crate::rt::Target::Null), false);
    let pnl = Panel::default();
    leds.update(&snap, &[true; 16], &pnl, false, FaderPage::Panel, false, 0.0);
    let n = leds.out.sent;
    leds.update(&snap, &[true; 16], &pnl, false, FaderPage::Panel, false, 0.0);
    assert_eq!(leds.out.sent, n, "nothing changed, nothing sent");
    leds.update(&snap, &[true; 16], &pnl, false, FaderPage::Panel, true, 0.0);
    assert!(leds.out.sent > n, "Track LEDs re-sent");
}

// ---------------------------------------------------------------------------
// Engine NEEDs batch (m3/engine-needs): preview, queue, library, voices, keys, settings.
// ---------------------------------------------------------------------------

/// A session on a folder of three corpus styles.
fn library_session() -> Option<Session> {
    let p = style("SlowWalker.T552.sty")?;
    let dir = p.parent().unwrap();
    let paths = ["SlowWalker.T552.sty", "TickingAway.T162.sty", "CoolRevibed.T552.sty"].map(|n| dir.join(n)).to_vec();
    if !paths.iter().all(|p| p.exists()) {
        return None;
    }
    let s = Session::offline(Options { paths, ..Options::default() }).unwrap();
    s.finish_indexing();
    Some(s)
}

/// Another style than the loaded one (its library id).
fn other_style(s: &Session) -> usize {
    let cur = s.state().style.id;
    s.library_list().entries.iter().find(|e| e.id != cur && e.status == "ok").unwrap().id
}

/// Advance in 10 ms steps until `f` holds (or 10 s pass on the virtual clock).
fn advance_until(s: &Session, mut f: impl FnMut(&AppState) -> bool) -> bool {
    for _ in 0..1000 {
        if f(&s.state()) {
            return true;
        }
        s.advance(10 * MS);
    }
    f(&s.state())
}

/// Wait in real time (a background thread) for `f`, advancing the offline clock.
fn wait_for(s: &Session, mut f: impl FnMut(&AppState) -> bool) -> bool {
    let t0 = std::time::Instant::now();
    while t0.elapsed() < std::time::Duration::from_secs(60) {
        s.advance(MS);
        if f(&s.state()) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    false
}

#[test]
fn style_preview_plays_four_bars_and_leaves_the_setup_alone() {
    let Some(s) = library_session() else { return };
    s.send(MixerCmd::SetStylePartVolume { part: 2, volume: 33 }).unwrap();
    s.send(OtsCmd::RecallOts { index: 0 }).unwrap();
    let before = s.state();
    let id = other_style(&s);
    s.take_output();
    s.send(PreviewCmd::AuditionStyle { id }).unwrap();
    let st = s.state();
    let a = st.preview.audition.clone().expect("preview playing");
    assert_eq!((a.id, a.bar, a.bars, a.chord.as_deref()), (id, 1, 4, Some("C")));
    assert!(!st.transport.running, "the band stays stopped");
    assert_eq!(st.style, before.style, "the loaded style stays");
    assert!(s.take_output().iter().any(|m| m[0] & 0xF0 == 0x90 && m[0] & 0x0F >= 8), "the preview plays on the band's channels");
    assert!(advance_until(&s, |st| st.preview.audition.as_ref().is_some_and(|a| a.bar == 2)));
    assert_eq!(s.state().preview.audition.as_ref().unwrap().chord.as_deref(), Some("Am"));
    assert!(advance_until(&s, |st| st.preview.audition.is_none()), "it stops by itself");
    let st = s.state();
    assert!(!st.transport.running);
    assert_eq!((&st.style, &st.keyboard_parts, &st.ots), (&before.style, &before.keyboard_parts, &before.ots));
    assert_eq!(st.mixer.style_parts[2].volume, 33, "the mixer stays");
    // Afterwards the loaded style's setup is back on its channels: its fader levels.
    let out = s.take_output();
    assert!(out.contains(&[0xBA, 7, 33]), "the Bass level goes out again");
}

#[test]
fn style_preview_is_refused_while_the_band_plays_and_ends_on_start() {
    let Some(s) = library_session() else { return };
    let id = other_style(&s);
    // A Sync Start chord ends the preview and starts the band.
    s.send(PreviewCmd::AuditionStyle { id }).unwrap();
    assert!(s.state().preview.audition.is_some());
    keys(&s, true, &[36, 40, 43]);
    let st = s.state();
    assert!(st.transport.running && st.preview.audition.is_none());
    let r = s.send(PreviewCmd::AuditionStyle { id });
    assert!(matches!(r, Err(CmdError::Failed(_))), "{r:?}");
    s.send(TransportCmd::Stop).unwrap();
    // StopAudition and START/STOP end it too.
    s.send(PreviewCmd::AuditionStyle { id }).unwrap();
    s.send(PreviewCmd::StopAudition).unwrap();
    assert!(s.state().preview.audition.is_none());
    s.send(PreviewCmd::AuditionStyle { id }).unwrap();
    s.send(TransportCmd::StartStop).unwrap();
    let st = s.state();
    assert!(st.transport.running && st.preview.audition.is_none());
}

#[test]
fn queue_style_waits_for_the_bar_line_and_loads_at_once_when_stopped() {
    let Some(s) = library_session() else { return };
    let first = s.state().style.id;
    let id = other_style(&s);
    // Stopped: the same as LoadStyle.
    s.send(LibraryCmd::QueueStyle { id }).unwrap();
    let st = s.state();
    assert_eq!((st.style.id, st.preview.queued), (id, None));
    s.send(LibraryCmd::LoadStyle { id: first }).unwrap();
    // Playing, past the bar's first beat (Next Bar: within it, the style changes at
    // once): the next bar line.
    keys(&s, true, &[36, 40, 43]);
    assert!(advance_until(&s, |st| st.transport.beat >= 2));
    s.send(LibraryCmd::QueueStyle { id }).unwrap();
    let st = s.state();
    assert_eq!((st.style.id, st.preview.queued), (first, Some(id)));
    let section = st.transport.section.clone();
    assert!(advance_until(&s, |st| st.style.id == id));
    let st = s.state();
    assert_eq!(st.preview.queued, None);
    assert_eq!(st.transport.section, section, "the section carries on");
    // StepStyle while playing waits too, from the style waiting.
    assert!(advance_until(&s, |st| st.transport.beat >= 2));
    s.send(LibraryCmd::StepStyle { delta: 1 }).unwrap();
    let waiting = s.state().preview.queued.expect("a style waits");
    assert_ne!(waiting, id);
    // Stopping first loads it then.
    s.send(TransportCmd::Stop).unwrap();
    let st = s.state();
    assert_eq!((st.style.id, st.preview.queued), (waiting, None));
}

#[test]
fn library_entries_carry_the_sff_format_and_the_voice_list() {
    let Some(s) = library_session() else { return };
    let lib = s.library_list();
    assert!(lib.entries.iter().all(|e| matches!(e.format.as_deref(), Some("SFF1" | "SFF2"))), "{:?}", lib.entries);
    let loaded = lib.entries.iter().find(|e| e.id == s.state().style.id).unwrap();
    assert_eq!(loaded.format.as_deref(), Some(s.state().style.format.as_str()));
    assert_eq!(lib.voices.len(), 128);
    assert_eq!((lib.voices[0].program, lib.voices[0].name.as_str()), (0, "Grand Piano"));
    assert!(lib.voices.iter().enumerate().all(|(i, v)| v.program as usize == i && v.bank_msb == 0 && v.name == gm_name(v.program)));
}

#[test]
fn rescan_adds_and_drops_files_keeping_ids() {
    let Some(src) = style("SlowWalker.T552.sty") else { return };
    let dir = std::env::temp_dir().join(format!("yahaha-rescan-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("Sub")).unwrap();
    std::fs::copy(&src, dir.join("A.sty")).unwrap();
    std::fs::copy(src.with_file_name("TickingAway.T162.sty"), dir.join("Sub/B.sty")).unwrap();
    let s = Session::offline(Options { paths: vec![dir.clone()], ..Options::default() }).unwrap();
    s.finish_indexing();
    let st = s.state();
    assert_eq!((st.library.count, st.library.roots.clone(), st.library.scanning), (2, vec![dir.display().to_string()], false));
    let ids: Vec<(usize, String)> = s.library_list().entries.iter().map(|e| (e.id, e.path.clone())).collect();
    std::fs::copy(src.with_file_name("CoolRevibed.T552.sty"), dir.join("Sub/C.sty")).unwrap();
    s.send(LibraryCmd::RescanLibrary).unwrap();
    // A three-file scan can finish before we look, so either it's still walking or it has
    // already merged the new file; it must not be neither (the rescan never started).
    let st = s.state();
    assert!(st.library.scanning || st.library.count == 3, "rescan did not start: {:?}", st.library);
    assert!(wait_for(&s, |st| !st.library.scanning && st.library.count == 3 && st.library.pending == 0));
    let lib = s.library_list();
    for (id, path) in &ids {
        assert!(lib.entries.iter().any(|e| e.id == *id && e.path == *path), "{path} keeps id {id}");
    }
    let c = lib.entries.iter().find(|e| e.path.ends_with("C.sty")).unwrap();
    assert_eq!((c.folder.as_str(), c.status.as_str()), ("Sub", "ok"));
    std::fs::remove_file(dir.join("Sub/B.sty")).unwrap();
    s.send(LibraryCmd::RescanLibrary).unwrap();
    assert!(wait_for(&s, |st| !st.library.scanning && st.library.count == 2));
    assert!(!s.library_list().entries.iter().any(|e| e.path.ends_with("B.sty")));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_keyboard_strip_sees_held_keys_parts_chord_and_detection() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    s.send(PartsCmd::SetPartOn { part: 1, on: true }).unwrap(); // Right 2 layered on Right 1
    keys(&s, true, &[36, 40, 43, 72]);
    let k = s.state().keyboard.clone();
    let held: Vec<(u8, Zone, Vec<u8>)> = k.held.iter().map(|h| (h.note, h.zone, h.parts.clone())).collect();
    assert_eq!(held, vec![
        (36, Zone::Left, vec![]),
        (40, Zone::Left, vec![]),
        (43, Zone::Left, vec![]),
        (72, Zone::Right, vec![0, 1]),
    ], "Lower, Left off: the left hand only gives the chord; the right plays Right 1 + 2");
    assert_eq!((k.left_split, k.detection), (54, [0, 54]));
    assert_eq!((k.chord_tones.clone(), k.chord_bass), (vec![0, 4, 7], Some(0)), "C");
    s.send(ChordCmd::SetUpper { on: true }).unwrap();
    assert_eq!(s.state().keyboard.detection, [55, 127], "Upper: above the split");
    s.send(ChordCmd::SetUpper { on: false }).unwrap();
    s.send(ChordCmd::SetFingering { fingering: Fingering::FullKeyboard }).unwrap();
    assert_eq!(s.state().keyboard.detection, [0, 127], "Full Keyboard: every key");
    keys(&s, false, &[36, 40, 43, 72]);
    assert!(s.state().keyboard.held.is_empty());
    // The section's length for the lead band (the chord started the band): the Main's
    // bars playing, none stopped.
    assert!(s.state().transport.running);
    assert!(s.state().transport.section_bars.is_some_and(|b| b >= 1));
    s.send(TransportCmd::Stop).unwrap();
    assert_eq!(s.state().transport.section_bars, None);
}

#[test]
fn palette_leds_switch_at_runtime() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    assert!(!s.state().pads.palette_leds);
    s.send(SettingsCmd::SetPaletteLeds { on: true }).unwrap();
    let st = s.state();
    assert!(st.pads.palette_leds && st.pads.pads.iter().all(|p| p.palette.is_some()));
    // The hardware gets every pad again, in the new mode.
    let mut leds = Leds::new(PacketSink::new(crate::rt::Target::Null), false);
    let snap = s.inner.lock().snap;
    let pnl = Panel::default();
    leds.update(&snap, &[true; 16], &pnl, false, FaderPage::Panel, false, 0.0);
    let n = leds.out.sent;
    leds.set_palette(true);
    leds.update(&snap, &[true; 16], &pnl, false, FaderPage::Panel, false, 0.0);
    assert!(leds.out.sent > n, "the pads in palette colours");
    // Back to RGB: every pad again, though its colour hasn't changed since RGB was last on.
    let n = leds.out.sent;
    leds.set_palette(false);
    leds.update(&snap, &[true; 16], &pnl, false, FaderPage::Panel, false, 0.0);
    assert!(leds.out.sent > n, "every pad re-sent");
    s.send(SettingsCmd::SetPaletteLeds { on: false }).unwrap();
    assert!(s.state().pads.pads.iter().all(|p| p.palette.is_none()));
}

#[test]
fn midi_input_choice() {
    let names: Vec<String> =
        ["yahaha", "Launchkey 49 MK4 LKMK4 MIDI Out", "Launchkey 49 MK4 LKMK4 DAW Out", "Roland A-88", "IAC Driver Bus 1"].map(String::from).to_vec();
    assert_eq!(choose_keys(&names, false, &[]), vec![1], "default: the Launchkey's keys");
    assert_eq!(choose_keys(&names, true, &[]), vec![1, 3, 4], "all: never yahaha, never the DAW port");
    assert_eq!(choose_keys(&names, false, &["Roland".into(), "IAC".into()]), vec![3, 4]);
    assert_eq!(choose_keys(&names[3..], false, &[]), vec![0, 1], "no Launchkey: every source");
    // Offline there are no sources, but the setting is kept and shown.
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    assert!(!s.state().io.all_inputs);
    s.send(SettingsCmd::SetMidiInputs { all: true, names: vec![] }).unwrap();
    assert!(s.state().io.all_inputs);
}

#[test]
fn sound_font_switch_needs_the_synth_and_a_file_in_its_folder() {
    let Some(p) = style("SlowWalker.T552.sty") else { return };
    let sf_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("soundfonts");
    let fonts = library::sound_font_files(&sf_dir);
    let sf2 = fonts.first().map(|f| sf_dir.join(f));
    let s = Session::offline(Options { paths: vec![p], sf2: sf2.clone(), ..Options::default() }).unwrap();
    let st = s.state();
    assert_eq!(st.io.sound_fonts, fonts);
    assert_eq!(st.io.sound_font_file, None, "no synth offline");
    let r = s.send(SettingsCmd::SetSoundFont { file: "x.sf2".into() });
    assert!(matches!(r, Err(CmdError::Failed(_))));
    assert_eq!(s.meters().channels.len(), 0, "no synth, no meters");
    let Some(file) = fonts.first().cloned() else { return };
    // A synth as a live session has one, with the rings its audio thread would drain.
    let (tx, mut rx) = RingBuffer::<Box<synth::Rack>>::new(2);
    let (_old_tx, old) = RingBuffer::<Box<synth::Rack>>::new(4);
    s.inner.lock().synth = Some(SynthRef {
        info: SynthInfo { name: "test".into(), sample_rate: 48000, buffer: None, device: "none".into(), channels: 2 },
        control: Arc::new(SynthControl::new(0)),
        swap: Some(synth::RackSwap { tx, old }),
        plugins: None,
        thread: None,
    });
    for bad in ["../x.sf2", "nope.sf2", "a/b.sf2"] {
        assert!(s.send(SettingsCmd::SetSoundFont { file: bad.into() }).is_err(), "{bad}");
    }
    s.send(SettingsCmd::SetSoundFont { file: file.clone() }).unwrap();
    assert!(s.state().io.sound_font_loading);
    assert!(wait_for(&s, |st| !st.io.sound_font_loading), "loads in the background");
    let st = s.state();
    assert_eq!(st.io.sound_font_file.as_deref(), Some(file.as_str()));
    assert!(rx.pop().is_ok(), "the new rack went to the audio thread");
    let m = s.meters();
    assert_eq!(m.channels.iter().map(|c| c.channel).collect::<Vec<_>>(), vec![1, 2, 3, 4, 9, 10, 11, 12, 13, 14, 15, 16]);
}

/// The audio buffer (#104): 64, 128 or 256 only. Offline it sets the render block; a note
/// held across the change sounds on and releases (nothing sticks). Live, the synth thread
/// reopens the stream and the size it reports is the one shown.
#[test]
fn audio_buffer_changes_keep_notes_and_report_the_size() {
    let Some(p) = style("SlowWalker.T552.sty") else { return };
    let s = Session::offline(Options { paths: vec![p.clone()], ..Options::default() }).unwrap();
    assert!(s.send(SettingsCmd::SetAudioBuffer { frames: 128 }).is_err(), "no synth");
    let sf_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("soundfonts");
    let Some(sf2) = library::sound_font_files(&sf_dir).into_iter().map(|f| sf_dir.join(f)).min_by_key(|p| p.metadata().map(|m| m.len()).unwrap_or(u64::MAX)) else {
        return;
    };
    s.offline_audio(Some(&sf2), 48_000).unwrap();
    let energy = |(l, r): (Vec<f32>, Vec<f32>)| l.iter().chain(&r).map(|x| (*x as f64).powi(2)).sum::<f64>();
    s.midi_in(Port::Keys, &[0x90, 72, 110]);
    assert!(energy(s.render(4800)) > 1e-4);
    for bad in [0, 100, 512] {
        assert!(s.send(SettingsCmd::SetAudioBuffer { frames: bad }).is_err(), "{bad}");
    }
    s.send(SettingsCmd::SetAudioBuffer { frames: 256 }).unwrap();
    assert_eq!(s.state().io.synth.as_ref().unwrap().buffer_frames, Some(256));
    assert!(energy(s.render(4800)) > 1e-4, "the held note plays on");
    s.midi_in(Port::Keys, &[0x80, 72, 0]);
    // Two seconds for the note, four more for the reverb tail (Hall, RT60 2.4 s; #204).
    s.render(6 * 48_000);
    assert!(energy(s.render(4800)) < 1e-6, "and releases");

    // Live: the synth thread answers with the size the device took.
    let s = Session::offline(Options { paths: vec![p], ..Options::default() }).unwrap();
    let (tx, rx) = std::sync::mpsc::channel::<SynthMsg>();
    let t = std::thread::spawn(move || {
        while let Ok(SynthMsg::Buffer(n, reply)) = rx.recv() {
            let _ = reply.send(Ok(Some(n.max(128))));
        }
    });
    s.inner.lock().synth = Some(SynthRef {
        info: SynthInfo { name: "test".into(), sample_rate: 48000, buffer: Some(128), device: "none".into(), channels: 2 },
        control: Arc::new(SynthControl::new(0)),
        swap: None,
        plugins: None,
        thread: Some(tx.clone()),
    });
    s.send(SettingsCmd::SetAudioBuffer { frames: 256 }).unwrap();
    assert_eq!(s.state().io.synth.as_ref().unwrap().buffer_frames, Some(256));
    s.send(SettingsCmd::SetAudioBuffer { frames: 64 }).unwrap();
    assert_eq!(s.state().io.synth.as_ref().unwrap().buffer_frames, Some(128), "the nearest the device allows");
    let _ = tx.send(SynthMsg::Stop);
    s.inner.lock().synth = None;
    t.join().unwrap();
}

#[test]
fn new_state_and_commands_serialize_as_documented() {
    use serde_json::json;
    for (cmd, want) in [
        (AppCmd::Preview(PreviewCmd::AuditionStyle { id: 3 }), json!({"type": "auditionStyle", "id": 3})),
        (AppCmd::Preview(PreviewCmd::StopAudition), json!({"type": "stopAudition"})),
        (AppCmd::Library(LibraryCmd::QueueStyle { id: 4 }), json!({"type": "queueStyle", "id": 4})),
        (AppCmd::Library(LibraryCmd::RescanLibrary), json!({"type": "rescanLibrary"})),
        (AppCmd::Settings(SettingsCmd::SetSoundFont { file: "A.sf2".into() }), json!({"type": "setSoundFont", "file": "A.sf2"})),
        (AppCmd::Settings(SettingsCmd::SetMidiInputs { all: false, names: vec!["K".into()] }), json!({"type": "setMidiInputs", "all": false, "names": ["K"]})),
        (AppCmd::Settings(SettingsCmd::SetPaletteLeds { on: true }), json!({"type": "setPaletteLeds", "on": true})),
    ] {
        assert_eq!(serde_json::to_value(&cmd).unwrap(), want);
        assert_eq!(serde_json::from_value::<AppCmd>(want).unwrap(), cmd);
    }
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    let v = serde_json::to_value(&*s.state()).unwrap();
    assert_eq!(v["preview"], json!({"audition": null, "queued": null}));
    assert_eq!(v["keyboard"]["held"], json!([]));
    assert_eq!(v["keyboard"]["detection"], json!([0, 54]));
    assert!(v["transport"].get("sectionBars").is_some());
    for k in ["sources", "allInputs", "soundFonts", "soundFontFile", "soundFontLoading"] {
        assert!(v["io"].get(k).is_some(), "io.{k}");
    }
    for k in ["roots", "scanning"] {
        assert!(v["library"].get(k).is_some(), "library.{k}");
    }
    assert!(v["pads"].get("paletteLeds").is_some());
}

/// A synthetic chart (no real song): two sections.
const TEST_CHART: &str = "irealbook://Test Tune=Doe John=Bossa Nova=C=n=*A[C^7 |D-7 G7 ]*B[F^7 |G7 Z";

#[test]
fn chart_player_imports_selects_and_plays() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    // Nothing imported: chart mode refuses to turn on.
    assert!(s.send(ChartCmd::ToggleChartMode).is_err());
    assert!(s.send(ChartCmd::SetChartMode { on: true }).is_err());
    assert!(!s.state().chart.on);
    assert!(s.send(ChartCmd::ImportCharts { text: "no links here".into() }).is_err());
    s.send(ChartCmd::ImportCharts { text: format!("<html><a href=\"{}\">x</a></html>", TEST_CHART.replace(' ', "%20")) }).unwrap();
    let st = s.state();
    let c = &st.chart;
    assert_eq!(c.playlists.len(), 1);
    assert_eq!(c.playlists[0].name, "Test Tune");
    assert_eq!(c.selected, Some([0, 0]));
    let song = c.song.as_ref().unwrap();
    assert_eq!(song.info.style, "Bossa Nova");
    let names: Vec<Vec<&str>> = song.bars.iter().map(|b| b.chords.iter().map(|c| c.name.as_str()).collect()).collect();
    assert_eq!(names, [vec!["Cmaj7"], vec!["Dm7", "G7"], vec!["Fmaj7"], vec!["G7"]]);
    assert_eq!(song.sections.iter().map(|x| (x.label.as_str(), x.start, x.bars)).collect::<Vec<_>>(), [("A", 0, 2), ("B", 2, 2)]);
    assert_eq!(song.bars[2].main, 1);
    assert!(!c.on && c.bar.is_none());
    assert!(st.message.as_ref().is_some_and(|m| m.text.contains("Imported 1 song")));

    s.send(ChartCmd::SetChartMode { on: true }).unwrap();
    s.send(ChartCmd::SetChartIntro { index: None }).unwrap();
    s.send(ChartCmd::SetChartLoop { range: Some([0, 4]) }).unwrap();
    assert!(s.send(ChartCmd::SetChartLoop { range: Some([2, 9]) }).is_err());
    s.send(TransportCmd::StartStop).unwrap();
    let st = s.state();
    assert!(st.transport.running);
    assert_eq!(st.chart.bar, Some(0));
    assert_eq!(st.chord.name.as_deref(), Some("Cmaj7"));
    let bar = (60e9 / st.transport.tempo * st.transport.beats_per_bar as f64) as u64;
    s.advance(bar + bar / 8);
    let st = s.state();
    assert_eq!(st.chart.bar, Some(1));
    assert_eq!(st.chord.name.as_deref(), Some("Dm7"));
    // The left hand takes over until the next bar line.
    keys(&s, true, &[36, 39, 43]);
    s.advance(20 * MS); // the chord settles
    let st = s.state();
    assert!(st.chart.overridden);
    assert_eq!(st.chord.name.as_deref(), Some("Cm"));
    keys(&s, false, &[36, 39, 43]);
    s.advance(bar);
    let st = s.state();
    assert_eq!(st.chart.bar, Some(2));
    assert!(!st.chart.overridden);
    assert_eq!(st.chord.name.as_deref(), Some("Fmaj7"));
    assert_eq!(st.transport.section.as_deref(), Some("Main B"));
    // Keyboard transpose moves the chart.
    s.send(ChordCmd::SetTranspose { keyboard: 2, master: 0 }).unwrap();
    s.advance(20 * MS); // the change settles
    assert_eq!(s.state().chord.name.as_deref(), Some("Gmaj7"));
    // The loop goes round.
    s.advance(2 * bar - 20 * MS);
    assert_eq!(s.state().chart.bar, Some(0));
    s.send(TransportCmd::StartStop).unwrap();
    assert_eq!(s.state().chart.bar, None);
    // Choruses expand the form.
    s.send(ChartCmd::SetChartChoruses { choruses: 3 }).unwrap();
    let st = s.state();
    assert_eq!(st.chart.song.as_ref().unwrap().bars.len(), 12);
    assert_eq!(st.chart.choruses, 3);
    // Fewer choruses: a loop past the new end goes.
    s.send(ChartCmd::SetChartLoop { range: Some([8, 12]) }).unwrap();
    s.send(ChartCmd::SetChartChoruses { choruses: 1 }).unwrap();
    assert_eq!(s.state().chart.loop_range, None);
    s.send(ChartCmd::SetChartLoop { range: Some([0, 2]) }).unwrap();
    s.send(ChartCmd::SetChartChoruses { choruses: 2 }).unwrap();
    assert_eq!(s.state().chart.loop_range, Some([0, 2]));
    s.send(ChartCmd::RemoveChartPlaylist { playlist: 0 }).unwrap();
    let st = s.state();
    assert!(st.chart.song.is_none() && !st.chart.on && st.chart.playlists.is_empty());
}

/// Only one of the chart player and the Chord Looper gives the chords: ON/OFF arming a
/// loop turns chart mode off; chart mode on stops the loop.
#[test]
fn chart_mode_and_the_chord_looper_take_turns() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    s.send(ChartCmd::ImportCharts { text: TEST_CHART.into() }).unwrap();
    s.send(LooperCmd::LooperRec).unwrap();
    keys(&s, true, &[36, 40, 43]); // C: starts the band and the recording
    s.advance(20 * MS);
    s.send(ChartCmd::SetChartMode { on: true }).unwrap();
    assert_eq!(s.state().looper.mode, LooperMode::Recording, "chart mode leaves a recording alone");
    s.advance(bar_ns(&s));
    keys(&s, false, &[36, 40, 43]);
    s.send(LooperCmd::LooperOnOff).unwrap();
    let st = s.state();
    assert!(!st.chart.on, "arming the loop turns chart mode off");
    assert_eq!(st.looper.mode, LooperMode::LoopArmed);
    s.send(ChartCmd::SetChartMode { on: true }).unwrap();
    let st = s.state();
    assert!(st.chart.on);
    assert_eq!(st.looper.mode, LooperMode::Off, "chart mode on stops the loop");
}

/// #110: ON/OFF straight after a memory is selected, with chart mode on, before any
/// snapshot shows the memory's sequence: one press arms the loop, and chart mode goes off
/// (the engine decides, on its own state, and reports it).
#[test]
fn looper_on_off_right_after_selecting_a_memory_with_chart_mode_on() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    s.send(ChartCmd::ImportCharts { text: TEST_CHART.into() }).unwrap();
    // Record one bar of C and keep it in memory 1.
    s.send(LooperCmd::LooperRec).unwrap();
    keys(&s, true, &[36, 40, 43]);
    s.advance(bar_ns(&s));
    keys(&s, false, &[36, 40, 43]);
    s.send(LooperCmd::LooperOnOff).unwrap();
    s.send(LooperCmd::LooperOnOff).unwrap();
    assert_eq!(s.state().looper.mode, LooperMode::Off);
    s.send(LooperCmd::StoreLooperMemory { index: 0 }).unwrap();
    // The engine's sequence emptied (as in a fresh engine), and chart mode on.
    {
        let mut ctl = s.inner.lock();
        ctl.looper.empty_engine_seq();
    }
    s.settle();
    s.send(ChartCmd::SetChartMode { on: true }).unwrap();
    let st = s.state();
    assert!(st.chart.on && !st.looper.has_data);
    // Select the memory and press ON/OFF at once: no snapshot in between.
    {
        let mut ctl = s.inner.lock();
        ctl.apply(LooperCmd::SelectLooperMemory { index: 0 }.into()).unwrap();
        assert!(!ctl.snap.looper.has_data, "the snapshot has not seen the memory yet");
        ctl.apply(LooperCmd::LooperOnOff.into()).unwrap();
    }
    s.settle();
    let st = s.state();
    assert_eq!(st.looper.mode, LooperMode::LoopArmed, "one press arms the loop");
    assert!(!st.chart.on, "chart mode went off for it");
    assert_eq!(st.message.as_ref().map(|m| m.text.as_str()), Some("Chart mode off: the Chord Looper plays"));
    // The session's chart settings follow: a later chart setting keeps chart mode off.
    s.send(ChartCmd::SetChartIntro { index: None }).unwrap();
    let st = s.state();
    assert!(!st.chart.on);
    assert_eq!(st.looper.mode, LooperMode::LoopArmed);
}

#[test]
fn chart_commands_serialize_as_documented() {
    use serde_json::json;
    for (cmd, want) in [
        (AppCmd::Chart(ChartCmd::ImportCharts { text: "irealb://x".into() }), json!({"type": "importCharts", "text": "irealb://x"})),
        (AppCmd::Chart(ChartCmd::SelectChart { playlist: 0, song: 2 }), json!({"type": "selectChart", "playlist": 0, "song": 2})),
        (AppCmd::Chart(ChartCmd::SetChartLoop { range: Some([4, 12]) }), json!({"type": "setChartLoop", "range": [4, 12]})),
        (AppCmd::Chart(ChartCmd::SetChartIntro { index: None }), json!({"type": "setChartIntro", "index": null})),
    ] {
        assert_eq!(serde_json::to_value(&cmd).unwrap(), want);
        assert_eq!(serde_json::from_value::<AppCmd>(want).unwrap(), cmd);
    }
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    let v = serde_json::to_value(&*s.state()).unwrap();
    assert_eq!(v["chart"]["loop"], json!(null));
    assert_eq!(v["chart"]["intro"], json!(0));
    assert_eq!(v["chart"]["autoStyle"], json!(true));
}

/// Style settings reach the engine and the state; the fade, Retrigger and Section Reset
/// buttons show in `transport`.
#[test]
fn style_settings_fade_and_retrigger_through_the_session() {
    use crate::engine::FadeState;
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    assert_eq!(s.state().style_settings, StyleSettingsState::default());
    s.send(StyleSettingsCmd::SetFadeInTime { ms: 300 }).unwrap();
    s.send(StyleSettingsCmd::SetFadeOutTime { ms: 60_000 }).unwrap();
    s.send(StyleSettingsCmd::SetRetriggerRate { rate: 12 }).unwrap();
    s.send(StyleSettingsCmd::SetSyncStopWindow { ms: 700 }).unwrap();
    let st = s.state().style_settings.clone();
    assert_eq!((st.fade_in_ms, st.fade_out_ms, st.retrigger_rate, st.sync_stop_window_ms), (300, 20_000, 8, 700), "clamped");
    // Armed while stopped, then the Sync Start chord fades in.
    s.send(TransportCmd::ToggleFade).unwrap();
    assert_eq!(s.state().transport.fade, FadeState::Armed);
    keys(&s, true, &[36, 40, 43]);
    assert_eq!(s.state().transport.fade, FadeState::FadingIn);
    s.advance(400 * MS);
    assert_eq!(s.state().transport.fade, FadeState::Off);
    // The Style parts' CC7 went to the synth: silence first, the fader value at the end;
    // no Master Volume, and the keyboard parts' levels untouched.
    let out = s.take_output();
    let mixer = s.state().mixer.style_parts.clone();
    let part = (0..8).max_by_key(|&p| mixer[p].volume).unwrap();
    let full = mixer[part].volume;
    let vols: Vec<u8> = out.iter().filter(|m| m[0] == 0xB0 | (8 + part as u8) && m[1] == 7).map(|m| m[2]).collect();
    let rise = &vols[vols.iter().position(|&v| v == 0).expect("silence first")..];
    assert_eq!(rise.last(), Some(&full), "{vols:?}");
    assert!(out.iter().all(|m| m[0] != 0xF0));
    for ch in 0..8u8 {
        let kbd: Vec<u8> = out.iter().filter(|m| m[0] == 0xB0 | ch && m[1] == 7).map(|m| m[2]).collect();
        assert!(kbd.windows(2).all(|w| w[0] == w[1]), "channel {ch} faded: {kbd:?}");
    }
    s.send(TransportCmd::ToggleRetrigger).unwrap();
    assert!(s.state().transport.retrigger);
    // Section Reset: back to bar 1, beat 1.
    assert!(advance_until(&s, |st| st.transport.bar >= 2));
    s.send(TransportCmd::SectionReset).unwrap();
    let t = s.state().transport.clone();
    assert_eq!((t.bar, t.beat), (1, 1));
}

/// The length of a bar at the state's tempo and time signature.
fn bar_ns(s: &Session) -> u64 {
    let t = &s.state().transport;
    (60e9 / t.tempo * t.beats_per_bar as f64) as u64
}

/// Chord Looper through the API: REC while stopped arms Sync Start, the first chord starts
/// the band and the recording; ON/OFF loops it from the next bar; memories keep it.
#[test]
fn chord_looper_records_loops_and_keeps_memories() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    s.send(TransportCmd::ToggleSyncStart).unwrap(); // off: REC turns it back on
    assert!(!s.state().transport.sync_start);
    s.send(LooperCmd::LooperRec).unwrap();
    let st = s.state();
    assert_eq!(st.looper.mode, LooperMode::RecArmed);
    assert!(st.transport.sync_start);
    // A memory can't be chosen while recording.
    assert!(s.send(LooperCmd::SelectLooperMemory { index: 0 }).is_err());
    let bar = bar_ns(&s);
    keys(&s, true, &[36, 40, 43]); // C
    assert_eq!(s.state().looper.mode, LooperMode::Recording);
    assert!(s.state().transport.running);
    s.advance(bar);
    keys(&s, false, &[36, 40, 43]);
    keys(&s, true, &[33, 36, 40]); // Am, on bar 2
    s.advance(bar - bar / 4);
    assert_eq!(s.state().looper.bar, Some(2));
    s.send(LooperCmd::LooperOnOff).unwrap();
    assert_eq!(s.state().looper.mode, LooperMode::LoopArmed);
    s.advance(bar / 2);
    let st = s.state();
    assert_eq!(st.looper.mode, LooperMode::Looping);
    assert_eq!(st.looper.bars, 2);
    let names: Vec<_> = st.looper.chords.iter().map(|c| (c.bar, c.beat, c.chord.as_str())).collect();
    assert_eq!(names, [(1, 1.0, "C"), (2, 1.0, "Am")]);
    assert_eq!(st.looper.memory, None);
    s.send(LooperCmd::StoreLooperMemory { index: 2 }).unwrap();
    let st = s.state();
    assert_eq!(st.looper.memory, Some(2));
    assert_eq!(st.looper.memories[2].name.as_deref(), Some("CLD_001"));
    assert_eq!(st.looper.memories[2].bars, 2);
    assert!(st.looper.memories[0].name.is_none());
    // ON/OFF: the loop stops at once.
    s.send(LooperCmd::LooperOnOff).unwrap();
    let st = s.state();
    assert_eq!(st.looper.mode, LooperMode::Off);
    assert!(st.looper.has_data);
    s.send(LooperCmd::NewLooperBank).unwrap();
    assert!(s.state().looper.memories.iter().all(|m| m.name.is_none()));
    assert!(s.send(LooperCmd::StoreLooperMemory { index: 9 }).is_ok(), "index wraps, the sequence is still there");
}

#[test]
fn solo_track_mute_tempo_and_metronome() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    s.send(MixerCmd::SetStyleSolo { part: Some(2) }).unwrap();
    assert_eq!(s.state().mixer.style_solo, Some(2));
    s.send(MixerCmd::SetStyleSolo { part: None }).unwrap();
    assert_eq!(s.state().mixer.style_solo, None);
    s.send(MixerCmd::StyleTrackMute { order: TrackMuteOrder::A, value: 0 }).unwrap();
    let on: Vec<_> = s.state().mixer.style_parts.iter().map(|p| p.on).collect();
    assert_eq!(on, [false, true, false, false, false, false, false, false]);
    s.send(MixerCmd::StyleTrackMute { order: TrackMuteOrder::A, value: 127 }).unwrap();
    assert!(s.state().mixer.style_parts.iter().all(|p| p.on));

    // Keyboard solo: Right 2 alone sounds, though it is off.
    s.send(MixerCmd::SetPartSolo { part: Some(1) }).unwrap();
    let st = s.state();
    assert_eq!(st.mixer.part_solo, Some(1));
    let sounding: Vec<_> = st.keyboard_parts.iter().map(|p| p.sounding).collect();
    assert_eq!(sounding, [false, true, false, false]);
    assert!(!st.keyboard_parts[1].on);
    s.take_output();
    keys(&s, true, &[72]);
    let out = s.take_output();
    let (r1, r2) = (crate::parts::CHANNEL[crate::parts::RIGHT1], crate::parts::CHANNEL[crate::parts::RIGHT2]);
    assert!(out.iter().any(|m| m[0] == 0x90 | r2 && m[1] == 72), "{out:?}");
    assert!(!out.iter().any(|m| m[0] == 0x90 | r1), "{out:?}");
    keys(&s, false, &[72]);
    s.send(MixerCmd::SetPartSolo { part: None }).unwrap();
    s.take_output();
    keys(&s, true, &[72]);
    assert!(s.take_output().iter().any(|m| m[0] == 0x90 | r1));

    s.send(TransportCmd::SetTempo { bpm: 500 }).unwrap();
    assert_eq!(s.state().transport.tempo, 500.0);
    s.send(TransportCmd::SetTempo { bpm: 1 }).unwrap();
    assert_eq!(s.state().transport.tempo, 5.0);
    s.send(TransportCmd::SetTempo { bpm: 120 }).unwrap();

    s.send(MetronomeCmd::ToggleMetronome).unwrap();
    s.send(MetronomeCmd::SetMetronomeVolume { volume: 200 }).unwrap();
    let m = s.state().metronome.clone();
    assert!(m.on && m.bell && !m.audible);
    assert_eq!(m.volume, 127);
    s.take_output();
    s.advance(2_000 * MS);
    let clicks = s.take_output().iter().filter(|m| m[0] == crate::click::CLICK).count();
    assert_eq!(clicks, 4, "120 BPM, stopped: a click every 500 ms");
}

/// Chart mode with OTS Link on (At Main Section Change, the default) and Half Bar Fill
/// In on (#92 vs #98): the chart's chords never recall an OTS or start a fill; only its
/// section change does, and the OTS comes when Main B starts (not during its fill).
#[test]
fn chart_chords_trigger_no_ots_or_fill() {
    let Some(s) = offline("SlowWalker.T552.sty") else { return };
    s.send(ChartCmd::ImportCharts { text: TEST_CHART.into() }).unwrap();
    s.send(ChartCmd::SetChartMode { on: true }).unwrap();
    s.send(ChartCmd::SetChartIntro { index: None }).unwrap();
    s.send(TransportCmd::SetHalfBarFill { on: true }).unwrap();
    s.send(OtsCmd::SetOtsLink { on: true }).unwrap();
    s.advance(10 * MS);
    assert_eq!(s.state().ots.link_timing, OtsLinkTiming::MainChange);
    s.send(TransportCmd::StartStop).unwrap();
    s.advance(5 * MS);
    let st = s.state();
    assert!(st.transport.running);
    assert_eq!((st.transport.section.as_deref(), st.ots.applied), (Some("Main A"), 1));
    let sounds = |st: &AppState| st.keyboard_parts.iter().map(|p| (p.on, p.program, p.volume, p.octave)).collect::<Vec<_>>();
    let before = sounds(&st);
    let mut chords = std::collections::BTreeSet::new();
    // Bars 0-1 (section A: Cmaj7, then Dm7 G7, with Main B's fill in bar 1): OTS 1 throughout.
    for _ in 0..15_000 / 5 {
        let st = s.state();
        if st.transport.section.as_deref() == Some("Main B") {
            break;
        }
        chords.extend(st.chord.name.clone());
        if st.chart.bar == Some(0) {
            assert_eq!(st.transport.section.as_deref(), Some("Main A"), "a chart chord started no fill");
        }
        assert_eq!((st.ots.applied, sounds(&st)), (1, before.clone()), "no OTS before Main B ({:?})", st.transport.section);
        s.advance(5 * MS);
    }
    assert!(chords.len() >= 3, "the chart's chords played: {chords:?}");
    let st = s.state();
    assert_eq!((st.transport.section.as_deref(), st.ots.applied), (Some("Main B"), 2), "OTS 2 as Main B starts");
}

/// The Launchkey's Ending pads (100-102, with the pad's pressure) queue the Ending and
/// change nothing the band sends before its bar line (#129): the output matches a session
/// where no pad was pressed, 5 ms at a time, up to the Ending's start.
#[test]
fn an_ending_pad_changes_nothing_before_the_ending() {
    let Some(p) = style("NightCruiser.S930.STY") else { return };
    for pad in [100u8, 101, 102] {
        let band = || {
            let s = Session::offline(Options { paths: vec![p.clone()], ..Options::default() }).unwrap();
            keys(&s, true, &[48, 52, 55]);
            s.send(TransportCmd::StartStop).unwrap();
            let bar = 4 * (60e9 / s.state().transport.tempo) as u64;
            s.advance(bar + bar * 4 / 10);
            s.take_output();
            s
        };
        let (a, b) = (band(), band());
        for m in [[0x90, pad, 100], [0xA0, pad, 90], [0xD0, 90, 0], [0x80, pad, 0]] {
            a.midi_in(Port::Pads, &m);
        }
        assert!(a.state().transport.queued.as_deref().is_some_and(|q| q.starts_with("Ending")), "pad {pad} queued an Ending");
        assert_eq!(a.take_output(), Vec::<[u8; 3]>::new(), "pad {pad}: nothing sent at the press");
        let mut steps = 0;
        while !a.state().transport.section.as_deref().is_some_and(|s| s.starts_with("Ending")) {
            a.advance(5 * MS);
            b.advance(5 * MS);
            if a.state().transport.section.as_deref().is_some_and(|s| s.starts_with("Ending")) {
                break;
            }
            assert_eq!(a.take_output(), b.take_output(), "pad {pad}: the Main plays on unchanged ({steps} steps after the press)");
            steps += 1;
            assert!(steps < 1000, "pad {pad}: the Ending never started");
        }
        assert!(steps > 100, "pad {pad}: the Ending waited for the bar line ({steps} steps)");
    }
}
