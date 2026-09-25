//! The app API's JSON wire format (docs/app-api.md) is a contract with the app's
//! TypeScript types and its fixtures: however `AppCmd` and `AppState` are organised in
//! Rust, what goes over the wire must not change. These tests read the documented forms
//! and check that they go through the Rust types and come back out byte for byte.

use serde_json::Value;
use yahaha::api::{AppState, LibraryList};
use yahaha::{AppCmd, Event};

const DOC: &str = include_str!("../docs/app-api.md");
const STATE: &str = include_str!("../docs/fixtures/state.json");
const LIBRARY: &str = include_str!("../docs/fixtures/library.json");

/// One of every command, in the exact form `serde_json::to_string` writes it (the tag
/// first, then the fields in declaration order).
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
    r#"{"type":"toggleFade"}"#,
    r#"{"type":"sectionReset"}"#,
    r#"{"type":"toggleRetrigger"}"#,
    r#"{"type":"setTempo","bpm":480}"#,
    r#"{"type":"toggleStylePart","part":5}"#,
    r#"{"type":"setStylePartVolume","part":2,"volume":90}"#,
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
    // Keyboard parts
    r#"{"type":"setPartOn","part":1,"on":true}"#,
    r#"{"type":"togglePart","part":3}"#,
    r#"{"type":"selectPart","part":2}"#,
    r#"{"type":"setPartVoice","part":0,"program":48}"#,
    r#"{"type":"stepVoice","delta":-1}"#,
    r#"{"type":"setPartVolume","part":1,"volume":64}"#,
    r#"{"type":"setPartOctave","part":2,"octave":-2}"#,
    r#"{"type":"setPartSolo","part":1}"#,
    // Mixer, Launchkey pages, synth
    r#"{"type":"setFaderPage","page":"style"}"#,
    r#"{"type":"toggleFaderPage"}"#,
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
    r#"{"type":"setSoundFont","file":"FluidR3_GM.sf2"}"#,
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
    r#"{"type":"clearPartPlugin","part":0}"#,
    r#"{"type":"savePartPluginState","part":3}"#,
    r#"{"type":"rescanPlugins"}"#,
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
    r#"{"type":"createPatch","patch":{"name":"My Bass","category":"bass","tags":["warm"],"favourite":false,"source":{"kind":"soundFont","file":"GeneralUser-GS.sf2","bank":0,"program":33},"defaults":{"volume":100,"pan":null,"reverb":20,"chorus":null,"octave":-1}}}"#,
    r#"{"type":"updatePatch","id":"keys","patch":{"name":"Keys","category":"ePiano","tags":[],"favourite":true,"source":{"kind":"plugin","componentId":"aumu dls  appl","state":"AAE="},"defaults":{"volume":null,"pan":64,"reverb":null,"chorus":null,"octave":0}}}"#,
    r#"{"type":"deletePatch","id":"my-bass"}"#,
    r#"{"type":"duplicatePatch","id":"my-bass"}"#,
    r#"{"type":"movePatch","id":"my-bass","to":0}"#,
    r#"{"type":"setPatchFavourite","id":"my-bass","favourite":true}"#,
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
    // Parameter Lock
    r#"{"type":"setParamLock","item":"splitPoint","on":true}"#,
    r#"{"type":"setParamLock","item":"fingeringType","on":false}"#,
];

fn type_of(json: &str) -> String {
    let v: Value = serde_json::from_str(json).unwrap();
    v["type"].as_str().unwrap().to_string()
}

/// The command names in the doc's AppCmd tables (the first column of each row).
fn documented_cmds() -> Vec<String> {
    let start = DOC.find("\n## AppCmd").expect("AppCmd section");
    let end = start + DOC[start..].find("\n### Result").expect("Result section");
    let mut names = Vec::new();
    for line in DOC[start..end].lines().filter(|l| l.starts_with("| `")) {
        let first = line[2..].split('|').next().unwrap();
        for name in first.split('`').skip(1).step_by(2) {
            names.push(name.to_string());
        }
    }
    names
}

/// Every `{"type": ...}` object written out in the doc (inline examples and the example
/// state's actions), as its text.
fn documented_objects() -> Vec<&'static str> {
    let mut out = Vec::new();
    let b = DOC.as_bytes();
    let mut i = 0;
    while let Some(off) = DOC[i..].find('{') {
        let s = i + off;
        let rest = DOC[s + 1..].trim_start();
        if rest.starts_with("\"type\"") {
            let mut depth = 0;
            let mut j = s;
            while j < b.len() {
                match b[j] {
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            let text = &DOC[s..=j];
            if serde_json::from_str::<Value>(text).is_ok() {
                out.push(text);
            }
        }
        i = s + 1;
    }
    out
}

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
fn the_command_list_is_the_documented_one() {
    let mut doc = documented_cmds();
    doc.sort();
    doc.dedup();
    let mut ours: Vec<String> = EVERY_CMD.iter().map(|j| type_of(j)).collect();
    ours.sort();
    ours.dedup();
    assert_eq!(ours, doc, "EVERY_CMD must list exactly the commands docs/app-api.md documents");
}

#[test]
fn documented_examples_round_trip() {
    let objs = documented_objects();
    assert!(objs.len() >= 10, "found {} examples", objs.len());
    let mut cmds = 0;
    for text in objs {
        let v: Value = serde_json::from_str(text).unwrap();
        if let Ok(cmd) = serde_json::from_value::<AppCmd>(v.clone()) {
            assert_eq!(serde_json::to_value(&cmd).unwrap(), v, "{text}");
            cmds += 1;
        } else {
            let e: Event = serde_json::from_value(v.clone()).unwrap_or_else(|e| panic!("{text}: neither AppCmd nor Event: {e}"));
            assert_eq!(serde_json::to_value(e).unwrap(), v, "{text}");
        }
    }
    assert!(cmds >= 10);
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
fn example_state_in_the_doc_round_trips() {
    let start = DOC.find("## Example `AppState`").unwrap();
    let body = &DOC[start..];
    let s = body.find("```json\n").unwrap() + "```json\n".len();
    let e = s + body[s..].find("```").unwrap();
    let v: Value = serde_json::from_str(&body[s..e]).unwrap();
    let st: AppState = serde_json::from_value(v.clone()).unwrap();
    assert_eq!(serde_json::to_value(&st).unwrap(), v);
}

#[test]
fn events_keep_their_form() {
    for (e, json) in [
        (Event::StateChanged { version: 3 }, r#"{"type":"stateChanged","version":3}"#),
        (Event::LibraryChanged { revision: 2 }, r#"{"type":"libraryChanged","revision":2}"#),
        (Event::Stopped, r#"{"type":"stopped"}"#),
    ] {
        assert_eq!(serde_json::to_string(&e).unwrap(), json);
        assert_eq!(serde_json::from_str::<Event>(json).unwrap(), e);
    }
}
