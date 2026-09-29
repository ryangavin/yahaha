//! The app API's JSON wire format (docs/app-api.md) is a contract with the app's
//! TypeScript types and its fixtures: however `AppCmd` and `AppState` are organised in
//! Rust, what goes over the wire must not change. These tests take one of every command
//! and the fixtures in tests/fixtures, and check that they go through the Rust types and
//! come back out byte for byte. Nothing here reads docs/: CI checks separately that
//! EVERY_CMD names exactly the commands docs/app-api.md documents, and
//! examples/api_doc_check.rs checks the doc's JSON examples (AGENTS.md, Checks).

use serde_json::Value;
use yahaha::api::{AppState, LibraryList};
use yahaha::{AppCmd, Event};

const STATE: &str = include_str!("fixtures/state.json");
const LIBRARY: &str = include_str!("fixtures/library.json");

/// One of every command, in the exact form `serde_json::to_string` writes it (the tag
/// first, then the fields in declaration order). Keep one command per line, starting
/// `r#"{"type":"<name>"`: CI's docs check reads the names from these lines.
const EVERY_CMD: &[&str] = &[
    // Sections and transport
    r#"{"type":"intro","index":1}"#,
    r#"{"type":"main","index":2}"#,
    r#"{"type":"break"}"#,
    r#"{"type":"fill","delta":1}"#,
    r#"{"type":"ending","index":0}"#,
    r#"{"type":"startStop"}"#,
    r#"{"type":"stop"}"#,
    r#"{"type":"toggleSyncStart"}"#,
    r#"{"type":"toggleSyncStop"}"#,
    r#"{"type":"toggleAutoFill"}"#,
    r#"{"type":"toggleStopAcmp"}"#,
    r#"{"type":"setStopAcmp","mode":"fixed"}"#,
    r#"{"type":"fillUp"}"#,
    r#"{"type":"fillDown"}"#,
    r#"{"type":"fillSelf"}"#,
    r#"{"type":"fillBreak"}"#,
    r#"{"type":"toggleHalfBarFill"}"#,
    r#"{"type":"setHalfBarFill","on":true}"#,
    r#"{"type":"tapTempo"}"#,
    r#"{"type":"tempoUp"}"#,
    r#"{"type":"tempoDown"}"#,
    r#"{"type":"resetTempo"}"#,
    r#"{"type":"toggleAcmp"}"#,
    r#"{"type":"setAcmp","on":false}"#,
    r#"{"type":"toggleUnison"}"#,
    r#"{"type":"setUnison","on":true}"#,
    r#"{"type":"setUnisonHeld","on":true}"#,
    r#"{"type":"setUnisonType","unisonType":"melody"}"#,
    r#"{"type":"toggleFade"}"#,
    r#"{"type":"sectionReset"}"#,
    r#"{"type":"toggleRetrigger"}"#,
    r#"{"type":"setTempo","bpm":480}"#,
    r#"{"type":"toggleStylePart","part":5}"#,
    r#"{"type":"setStylePartVolume","part":2,"volume":90}"#,
    r#"{"type":"setStylePartSend","part":3,"send":"chorus","value":70}"#,
    r#"{"type":"resetStylePartSends","part":null}"#,
    r#"{"type":"resetStylePartSends","part":5}"#,
    r#"{"type":"setStyleVolume","volume":80}"#,
    r#"{"type":"setMultiPadVolume","volume":70}"#,
    r#"{"type":"setStyleSolo","part":3}"#,
    r#"{"type":"setStyleSolo","part":null}"#,
    r#"{"type":"styleTrackMute","order":"b","value":64}"#,
    // Chord detection, split, transpose
    r#"{"type":"setFingering","fingering":"aiFullKeyboard"}"#,
    r#"{"type":"setFingering","fingering":"fingered"}"#,
    r#"{"type":"nextFingering"}"#,
    r#"{"type":"setUpper","on":true}"#,
    r#"{"type":"toggleUpper"}"#,
    r#"{"type":"setManualBass","on":false}"#,
    r#"{"type":"toggleManualBass"}"#,
    r#"{"type":"setSplit","note":60}"#,
    r#"{"type":"moveSplit","delta":-1}"#,
    r#"{"type":"setTranspose","keyboard":2,"master":-1}"#,
    r#"{"type":"stepTranspose","keyboard":1,"master":0}"#,
    r#"{"type":"resetTranspose"}"#,
    r#"{"type":"setChordSettle","ms":10}"#,
    r#"{"type":"setLeftHold","on":true}"#,
    r#"{"type":"toggleLeftHold"}"#,
    // Keyboard parts
    r#"{"type":"setPartOn","part":1,"on":true}"#,
    r#"{"type":"togglePart","part":3}"#,
    r#"{"type":"selectPart","part":2}"#,
    r#"{"type":"setPartVoice","part":0,"program":48}"#,
    r#"{"type":"stepVoice","delta":-1}"#,
    r#"{"type":"setPartVolume","part":1,"volume":64}"#,
    r#"{"type":"setPartOctave","part":2,"octave":-2}"#,
    r#"{"type":"setPartPan","part":3,"pan":20}"#,
    r#"{"type":"setPartSend","part":0,"send":"reverb","value":64}"#,
    r#"{"type":"setPartSend","part":1,"send":"chorus","value":10}"#,
    r#"{"type":"setPartSend","part":2,"send":"variation","value":30}"#,
    r#"{"type":"setPartSolo","part":1}"#,
    // Mixer, Launchkey pages, synth
    r#"{"type":"setFaderPage","page":"style"}"#,
    r#"{"type":"toggleFaderPage"}"#,
    r#"{"type":"setFaderLayer","layer":"reverb"}"#,
    r#"{"type":"stepFaderLayer","delta":1}"#,
    r#"{"type":"setPadPage","page":"otsParts"}"#,
    r#"{"type":"cyclePadPage","delta":1}"#,
    r#"{"type":"setMasterVolume","volume":100}"#,
    r#"{"type":"setSynthMuted","on":true}"#,
    r#"{"type":"toggleSynthMute"}"#,
    r#"{"type":"setAudioOutput","first":2}"#,
    r#"{"type":"nextAudioOutput"}"#,
    r#"{"type":"panic"}"#,
    r#"{"type":"clearMessage"}"#,
    // Settings
    r#"{"type":"setMidiInputs","all":false,"names":["Launchkey 49 MK4 LKMK4 MIDI Out"]}"#,
    r#"{"type":"setPaletteLeds","on":true}"#,
    r#"{"type":"setAudioBuffer","frames":128}"#,
    r#"{"type":"rescanLibrary"}"#,
    // Style settings
    r#"{"type":"setMainTiming","timing":"immediate"}"#,
    r#"{"type":"setIntroEndingTiming","timing":"endOfSection"}"#,
    r#"{"type":"setSyncStopWindow","ms":800}"#,
    r#"{"type":"setFadeInTime","ms":5000}"#,
    r#"{"type":"setFadeOutTime","ms":12500}"#,
    r#"{"type":"setFadeHoldTime","ms":2000}"#,
    r#"{"type":"setSectionReset","on":false}"#,
    r#"{"type":"setRetriggerRate","rate":16}"#,
    r#"{"type":"stepRetriggerRate","delta":-1}"#,
    r#"{"type":"setSwing","amount":50}"#,
    r#"{"type":"stepSwing","delta":-2}"#,
    r#"{"type":"setSwingGrid","grid":16}"#,
    r#"{"type":"setSectionTempo","on":false}"#,
    // One Touch Settings and styles
    r#"{"type":"recallOts","index":3}"#,
    r#"{"type":"setOtsLink","on":true}"#,
    r#"{"type":"toggleOtsLink"}"#,
    r#"{"type":"setOtsLinkTiming","timing":"mainChange"}"#,
    r#"{"type":"loadStyle","id":7}"#,
    r#"{"type":"queueStyle","id":8}"#,
    r#"{"type":"loadStylePath","path":"/tmp/x.sty"}"#,
    r#"{"type":"stepStyle","delta":-1}"#,
    r#"{"type":"auditionStyle","id":3}"#,
    r#"{"type":"stopAudition"}"#,
    // Style change behaviour
    r#"{"type":"setTempoChange","rule":"lock"}"#,
    r#"{"type":"setPartsChange","rule":"reset"}"#,
    r#"{"type":"setSectionSet","section":2}"#,
    r#"{"type":"setSectionSet","section":null}"#,
    r#"{"type":"toggleStyleTempoLock"}"#,
    r#"{"type":"toggleStyleTempoHold"}"#,
    // iReal Pro chart player
    r#"{"type":"importCharts","text":"irealb://..."}"#,
    r#"{"type":"importChartFile","path":"/tmp/playlist.html"}"#,
    r#"{"type":"selectChart","playlist":0,"song":3}"#,
    r#"{"type":"stepChart","delta":1}"#,
    r#"{"type":"removeChartPlaylist","playlist":1}"#,
    r#"{"type":"setChartMode","on":true}"#,
    r#"{"type":"toggleChartMode"}"#,
    r#"{"type":"setChartChoruses","choruses":3}"#,
    r#"{"type":"setChartLoop","range":[8,16]}"#,
    r#"{"type":"setChartLoop","range":null}"#,
    r#"{"type":"setChartIntro","index":0}"#,
    r#"{"type":"setChartEnding","index":null}"#,
    r#"{"type":"setChartAutoStyle","on":false}"#,
    // Registration Memory
    r#"{"type":"pressRegist","index":0}"#,
    r#"{"type":"recallRegist","index":9}"#,
    r#"{"type":"memorizeRegist","index":2}"#,
    r#"{"type":"toggleRegistMemory"}"#,
    r#"{"type":"pressSnapshot","slot":7}"#,
    r#"{"type":"stepSnapshotBank","delta":-1}"#,
    r#"{"type":"selectSnapshotBank","bank":1}"#,
    r#"{"type":"setMemorizeGroup","group":"voice","on":false}"#,
    r#"{"type":"clearRegist","index":3}"#,
    r#"{"type":"renameRegist","index":3,"name":"Verse"}"#,
    r#"{"type":"stepRegistBank","delta":1}"#,
    r#"{"type":"selectRegistBank","path":"banks/Gig.regist.json"}"#,
    r#"{"type":"newRegistBank"}"#,
    r#"{"type":"saveRegistBank","name":"Gig"}"#,
    r#"{"type":"saveRegistBank","name":null}"#,
    r#"{"type":"saveRegistBank","name":"Gig","overwrite":true}"#,
    r#"{"type":"setFreeze","on":true}"#,
    r#"{"type":"toggleFreeze"}"#,
    r#"{"type":"setFreezeGroup","group":"tempo","on":true}"#,
    r#"{"type":"setRegistSequence","steps":[2,0,5],"end":"next"}"#,
    r#"{"type":"setRegistSequenceOn","on":true}"#,
    r#"{"type":"toggleRegistSequence"}"#,
    r#"{"type":"stepRegistSequence","delta":-1}"#,
    r#"{"type":"stepRegist","delta":1}"#,
    // Playlist
    r#"{"type":"newPlaylist"}"#,
    r#"{"type":"loadPlaylist","path":"lists/Friday.playlist.json"}"#,
    r#"{"type":"savePlaylist","name":"Friday"}"#,
    r#"{"type":"savePlaylist","name":"Friday","overwrite":true}"#,
    r#"{"type":"addPlaylistRecord","record":{"name":"Opener","kind":"bank","path":"banks/Gig.regist.json","regist":0}}"#,
    r#"{"type":"addPlaylistRecord","record":{"name":"Blues","kind":"style","path":"styles/x.sty"}}"#,
    r#"{"type":"addCurrentBank"}"#,
    r#"{"type":"addCurrentStyle"}"#,
    r#"{"type":"appendPlaylist","path":"lists/Other.playlist.json"}"#,
    r#"{"type":"setPlaylistRecord","index":1,"record":{"name":"Ballad","kind":"bank","path":"banks/Gig.regist.json"}}"#,
    r#"{"type":"movePlaylistRecord","index":2,"delta":-1}"#,
    r#"{"type":"deletePlaylistRecord","index":0}"#,
    r#"{"type":"setPlaylistSort","sort":"aToZ"}"#,
    r#"{"type":"loadPlaylistRecord","index":4}"#,
    r#"{"type":"stepPlaylist","delta":1}"#,
    // Chord Looper
    r#"{"type":"looperRec"}"#,
    r#"{"type":"looperOnOff"}"#,
    r#"{"type":"selectLooperMemory","index":2}"#,
    r#"{"type":"storeLooperMemory","index":7}"#,
    r#"{"type":"clearLooperMemory","index":0}"#,
    r#"{"type":"newLooperBank"}"#,
    r#"{"type":"saveLooperBank","name":"Songs","overwrite":true}"#,
    r#"{"type":"saveLooperBank","name":null}"#,
    r#"{"type":"loadLooperBank","path":"/data/ChordLooper/Songs.looper.json"}"#,
    // Metronome
    r#"{"type":"toggleMetronome"}"#,
    r#"{"type":"setMetronome","on":true}"#,
    r#"{"type":"setMetronomeVolume","volume":70}"#,
    r#"{"type":"setMetronomeBell","on":false}"#,
    // Multi Pads
    r#"{"type":"loadMultiPad","id":2}"#,
    r#"{"type":"loadMultiPadPath","path":"/tmp/Demo.pad"}"#,
    r#"{"type":"clearMultiPad"}"#,
    r#"{"type":"triggerMultiPad","pad":0}"#,
    r#"{"type":"stopMultiPad","pad":3}"#,
    r#"{"type":"stopAllMultiPads"}"#,
    r#"{"type":"armMultiPad","pad":1}"#,
    r#"{"type":"setMultiPadRepeat","pad":2,"on":false}"#,
    r#"{"type":"setMultiPadChordMatch","pad":1,"on":true}"#,
    r#"{"type":"setMultiPadSynchroStop","styleStop":true,"ending":false}"#,
    // Controllers
    r#"{"type":"setPedal","pedal":1,"cc":66,"function":"fillUp","controlType":"toggle","reverse":true,"range":"full"}"#,
    r#"{"type":"setPedal","pedal":2,"cc":null,"function":"none","controlType":"holdA","reverse":false,"range":"upper"}"#,
    r#"{"type":"learnPedal","pedal":0}"#,
    r#"{"type":"learnPedal","pedal":null}"#,
    r#"{"type":"setPartControllers","part":3,"sustain":false,"pitchBend":true,"modulation":false}"#,
    r#"{"type":"setBendRange","part":0,"semitones":12}"#,
    r#"{"type":"triggerFunction","function":"ots2"}"#,
    // Instrument plugins
    r#"{"type":"setPartPlugin","part":0,"id":"aumu dls  appl","state":null}"#,
    r#"{"type":"setPartPlugin","part":1,"id":"aumu Xf2X XFER","state":"YnBsaXN0MDA="}"#,
    r#"{"type":"setPartPluginPreset","part":2,"id":"aumu Nik2 -NI-","preset":"f:3"}"#,
    r#"{"type":"clearPartPlugin","part":0}"#,
    r#"{"type":"savePartPluginState","part":3}"#,
    r#"{"type":"rescanPlugins"}"#,
    r#"{"type":"setPluginInProcess","id":"aumu dls  appl","inProcess":true}"#,
    r#"{"type":"reloadPartPlugin","part":null}"#,
    r#"{"type":"reloadPartPlugin","part":2}"#,
    r#"{"type":"markPluginSeen","id":"aumu Smp7 Fake"}"#,
    // Keyboard Harmony / Arpeggio
    r#"{"type":"toggleHarmonyArp"}"#,
    r#"{"type":"setHarmonyArpOn","on":true}"#,
    r#"{"type":"setHarmonyType","index":20}"#,
    r#"{"type":"setArpPattern","index":3}"#,
    r#"{"type":"stepHarmonyArpType","delta":-1}"#,
    r#"{"type":"setHarmonyVolume","volume":90}"#,
    r#"{"type":"setHarmonySpeed","speed":"1/12"}"#,
    r#"{"type":"setHarmonyAssign","assign":"right2"}"#,
    r#"{"type":"setChordNoteOnly","on":true}"#,
    r#"{"type":"setTouchLimit","velocity":64}"#,
    r#"{"type":"setArpQuantize","quantize":"sixteenth"}"#,
    r#"{"type":"setArpHold","on":true}"#,
    r#"{"type":"toggleArpHold"}"#,
    r#"{"type":"setArpPedalHold","on":true}"#,
    r#"{"type":"toggleArpPedalHold"}"#,
    r#"{"type":"setArpVelocity","mode":"fixed","velocity":90}"#,
    r#"{"type":"setArpKeepKeyOn","on":false}"#,
    // Sound library
    r#"{"type":"createPatch","patch":{"name":"My Bass","category":"bass","tags":["warm"],"favourite":false,"source":{"kind":"soundFont","file":"GeneralUser-GS.sf2","bank":0,"program":33}}}"#,
    r#"{"type":"updatePatch","id":"keys","patch":{"name":"Keys","category":"ePiano","tags":[],"favourite":true,"source":{"kind":"plugin","componentId":"aumu dls  appl","state":"AAE="}}}"#,
    r#"{"type":"deletePatch","id":"my-bass"}"#,
    r#"{"type":"duplicatePatch","id":"my-bass"}"#,
    r#"{"type":"movePatch","id":"my-bass","to":0}"#,
    r#"{"type":"setPatchFavourite","id":"my-bass","favourite":true}"#,
    r#"{"type":"saveSound","part":1}"#,
    r#"{"type":"saveSoundAs","part":2,"name":"Stage Piano"}"#,
    r#"{"type":"saveSoundAs","part":0,"name":null}"#,
    r#"{"type":"savePartAsPatch","part":0,"name":"Stage Piano"}"#,
    r#"{"type":"savePartAsPatch","part":3,"name":null}"#,
    r#"{"type":"addPresetAsPatch","file":"GeneralUser-GS.sf2","bank":128,"program":0,"name":null}"#,
    r#"{"type":"auditionPatch","id":"my-bass"}"#,
    r#"{"type":"auditionPreset","file":"GeneralUser-GS.sf2","bank":0,"program":4}"#,
    r#"{"type":"stopPatchAudition"}"#,
    r#"{"type":"setFamilyRule","family":4,"patch":"my-bass","style":false}"#,
    r#"{"type":"setFamilyRule","family":11,"patch":null,"style":true}"#,
    r#"{"type":"setProgramOverride","program":4,"patch":"keys","style":false}"#,
    r#"{"type":"setDrumRule","patch":"kit","style":true}"#,
    r#"{"type":"clearStyleMap"}"#,
    r#"{"type":"setPartPatch","part":1,"id":"my-bass"}"#,
    r#"{"type":"setPartPatch","part":0,"id":null}"#,
    r#"{"type":"setPortSendsMapped","on":true}"#,
    r#"{"type":"browseSoundFont","file":"GeneralUser-GS.sf2"}"#,
    r#"{"type":"browseSoundFont","file":null}"#,
    r#"{"type":"importSoundLibrary","path":"/tmp/lib.json","replace":false,"maps":true}"#,
    r#"{"type":"exportSoundLibrary","path":null}"#,
    r#"{"type":"exportSoundPreset","id":"deluxe-keys","overwrite":true}"#,
    // Parameter Lock
    r#"{"type":"setParamLock","item":"splitPoint","on":true}"#,
    r#"{"type":"setParamLock","item":"fingeringType","on":false}"#,
    // Sound catalog
    r#"{"type":"setSoundFavourite","id":"sf:GeneralUser-GS.sf2:0:0","on":true}"#,
    r#"{"type":"auditionSound","id":"au:aumu Xf2X XFER"}"#,
    r#"{"type":"stopSoundAudition"}"#,
    r#"{"type":"assignSound","part":0,"id":"saved:warm-pad"}"#,
    r#"{"type":"replacePartSound","part":1,"id":"saved:warm-pad"}"#,
    r#"{"type":"setSoundCategory","id":"au:aumu Xf2X XFER","category":"pad"}"#,
    r#"{"type":"listPluginPresets","id":"au:aumu Nik2 -NI-"}"#,
    r#"{"type":"addToMySounds","id":"sf:GM.sf2:0:5"}"#,
    r#"{"type":"savePartAsPluginPreset","part":0,"name":"Upright","category":"piano"}"#,
    r#"{"type":"savePartAsPluginPreset","part":1,"name":"Upright","category":"piano","overwrite":true}"#,
    // Style Dynamics Control, Touch, Accent
    r#"{"type":"setDynamicsControl","on":false}"#,
    r#"{"type":"setDynamics","level":90}"#,
    r#"{"type":"stepDynamics","delta":-8}"#,
    r#"{"type":"setDynamicsTouch","on":true}"#,
    r#"{"type":"toggleDynamicsTouch"}"#,
    r#"{"type":"setAccent","on":true}"#,
    r#"{"type":"toggleAccent"}"#,
    r#"{"type":"setAccentThreshold","velocity":110}"#,
    r#"{"type":"setAccentMode","mode":"fill"}"#,
    r#"{"type":"setAccentSource","source":"both"}"#,
    // Knob Assign pages (#197)
    r#"{"type":"setKnobPage","page":"parts"}"#,
    r#"{"type":"stepKnobPage","delta":-1}"#,
    r#"{"type":"turnKnob","knob":3,"delta":-2}"#,
    r#"{"type":"resetKnob","knob":3}"#,
    r#"{"type":"setEffectType","block":"reverb","effect":"plate"}"#,
    r#"{"type":"setEffectType","block":"variation","effect":"pingPong"}"#,
    r#"{"type":"setEffectReturn","block":"chorus","level":90}"#,
    r#"{"type":"setBandSend","block":"variation","level":100}"#,
    r#"{"type":"setPadSend","block":"chorus","level":40}"#,
    r#"{"type":"setEffectParam","block":"reverb","param":"reverbTime","value":35}"#,
    r#"{"type":"setEffectParam","block":"variation","param":"delayFeedback","value":60}"#,
    r#"{"type":"setEffectParam","block":"chorus","param":"chorusDepth","value":30}"#,
    r#"{"type":"setFollowStyle","block":"variation","on":false}"#,
    r#"{"type":"setInsertsOn","on":false}"#,
    r#"{"type":"setPartInsertOn","part":3,"on":false}"#,
    r#"{"type":"setPartInsertAmount","part":3,"amount":100}"#,
    r#"{"type":"setRotaryFast","on":true}"#,
];

#[test]
fn every_command_round_trips_byte_for_byte() {
    for json in EVERY_CMD {
        let cmd: AppCmd = serde_json::from_str(json).unwrap_or_else(|e| panic!("{json}: {e}"));
        assert_eq!(serde_json::to_string(&cmd).unwrap(), *json);
        // And through a serde_json::Value, as the Tauri shell receives it.
        let v: Value = serde_json::from_str(json).unwrap();
        let cmd2: AppCmd = serde_json::from_value(v.clone()).unwrap();
        assert_eq!(cmd2, cmd);
        assert_eq!(serde_json::to_value(&cmd2).unwrap(), v);
    }
}

#[test]
fn bad_commands_are_refused() {
    for bad in [
        r#"{"type":"noSuchCommand"}"#,
        r#"{"index":1}"#,
        r#"{"type":"main"}"#,
        r#"{"type":"main","index":"one"}"#,
        r#"{"type":"setFingering","fingering":"nope"}"#,
        r#"{"type":"setMidiInputs","all":false}"#,
        r#"{"type":"setPedal","pedal":0,"cc":64,"function":"noSuchFunction"}"#,
        r#"{"type":"triggerFunction"}"#,
        r#"{"type":"setParamLock","item":"masterEq","on":true}"#,
        r#"{"type":"setDynamics","level":300}"#,
        r#"{"type":"setKnobPage","page":"type9"}"#,
        r#"{"type":"setEffectType","block":"delay","effect":"hall"}"#,
        r#"{"type":"setEffectReturn","block":"reverb","level":300}"#,
        r#"[1,2]"#,
        r#""startStop""#,
    ] {
        assert!(serde_json::from_str::<AppCmd>(bad).is_err(), "{bad} was accepted");
    }
    let e = serde_json::from_str::<AppCmd>(r#"{"type":"noSuchCommand"}"#).unwrap_err().to_string();
    assert!(e.contains("noSuchCommand"), "{e}");
}

#[test]
fn state_fixture_round_trips_byte_for_byte() {
    let st: AppState = serde_json::from_str(STATE).unwrap();
    assert_eq!(serde_json::to_string_pretty(&st).unwrap() + "\n", STATE);
    // Semantically: serde_json's default float parsing is not exact, so a tempo such as
    // 90.99995298335763 comes back one ulp off. Both sides parse it the same way here.
    let lib: LibraryList = serde_json::from_str(LIBRARY).unwrap();
    let v: Value = serde_json::from_str(LIBRARY).unwrap();
    assert_eq!(serde_json::to_value(&lib).unwrap(), v);
    // Everything but those floats byte for byte: the same keys, in the same order.
    let out = serde_json::to_string_pretty(&lib).unwrap() + "\n";
    let strip = |s: &str| s.lines().filter(|l| !l.trim_start().starts_with("\"tempo\"")).collect::<Vec<_>>().join("\n");
    assert_eq!(strip(&out), strip(LIBRARY));
}


#[test]
fn events_keep_their_form() {
    for (e, json) in [
        (Event::StateChanged { version: 3 }, r#"{"type":"stateChanged","version":3}"#),
        (Event::LibraryChanged { revision: 2 }, r#"{"type":"libraryChanged","revision":2}"#),
        (Event::SoundsChanged { revision: 4 }, r#"{"type":"soundsChanged","revision":4}"#),
        (Event::Stopped, r#"{"type":"stopped"}"#),
    ] {
        assert_eq!(serde_json::to_string(&e).unwrap(), json);
        assert_eq!(serde_json::from_str::<Event>(json).unwrap(), e);
    }
}
