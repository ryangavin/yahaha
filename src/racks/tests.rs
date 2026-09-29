//! The rack file: round trip, newer versions refused and never saved over, unknown fields
//! kept.

use super::*;
use crate::api::{ArpQuantize, ArpVelocityMode, HarmonyArpMode, HarmonyAssign, HarmonySpeed};
use serde_json::json;

fn temp_dir(test: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("yahaha-racks-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn part(sound: SoundRef, volume: u8) -> RackPart {
    RackPart {
        on: true,
        sound,
        edited_state: None,
        fallback_program: None,
        volume,
        pan: 64,
        reverb: 40,
        chorus: 0,
        variation: 0,
        octave: 0,
        tone: ToneReg::default(),
        bend_range: 2,
        eq: PartEq::FLAT,
        insert: PartInsert::OFF,
        other: Map::new(),
    }
}

fn sample() -> Rack {
    let font = |program| SoundRef::Font { file: "GeneralUser.sf2".into(), bank: 0, program };
    let mut plugin = part(SoundRef::Library { id: "warm-pad".into() }, 90);
    plugin.edited_state = Some("AAEC".into());
    plugin.fallback_program = Some(88);
    plugin.tone = ToneReg { cutoff: Some(80), xg: vec![[8, 0x0E, 3]], ..ToneReg::default() };
    plugin.eq = PartEq { low_gain: -3, low_freq: 125, high_gain: 4, high_freq: 8_000 };
    plugin.insert = PartInsert { effect: crate::fx::InsertEffect::Rotary, on: true, amount: 90 };
    Rack {
        format: FORMAT.into(),
        version: VERSION,
        id: new_id(),
        name: "Ballad".into(),
        parts: [part(font(0), 100), plugin, part(SoundRef::Plugin { component: "aumu abcd manu".into() }, 70), part(font(32), 110)],
        split: 54,
        harmony_arp: HarmonyArpReg {
            on: true,
            mode: HarmonyArpMode::Harmony,
            harmony_type: "Duet".into(),
            arp_pattern: "Up Oct".into(),
            volume: 90,
            speed: HarmonySpeed::Eighth,
            assign: HarmonyAssign::Auto,
            chord_note_only: false,
            touch_limit: 1,
            arp_quantize: ArpQuantize::Off,
            arp_hold: false,
            arp_velocity: ArpVelocityMode::Original,
            arp_fixed_velocity: 100,
            arp_keep_key_on: false,
        },
        transpose: -2,
        controls: ControlMap::default(),
        other: Map::new(),
    }
}

#[test]
fn a_rack_round_trips_through_its_file() {
    let dir = temp_dir("round-trip");
    let r = sample();
    let path = path_for(&dir, &r.name);
    assert_eq!(path.file_name().unwrap(), "Ballad.rack.json");
    r.save(&path).unwrap();
    assert_eq!(Rack::load(&path).unwrap(), r);
    assert_eq!(list(&dir), [path.clone()]);
    assert_eq!(name_of(&path), "Ballad");
    // No temporary file is left behind.
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
    let v: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!((v["format"].as_str(), v["version"].as_u64()), (Some(FORMAT), Some(1)));
    assert_eq!(v["parts"][1]["sound"], json!({ "kind": "library", "id": "warm-pad" }));
    assert_eq!(v["parts"][1]["editedState"], "AAEC");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_newer_rack_is_refused_and_never_saved_over() {
    let dir = temp_dir("newer");
    std::fs::create_dir_all(&dir).unwrap();
    let path = path_for(&dir, "Ballad");
    let mut v = serde_json::to_value(sample()).unwrap();
    v["version"] = json!(VERSION + 1);
    let newer = serde_json::to_string_pretty(&v).unwrap();
    std::fs::write(&path, &newer).unwrap();
    let e = Rack::load(&path).unwrap_err();
    assert!(format!("{e:#}").contains("newer"), "{e:#}");
    assert!(sample().save(&path).is_err(), "saving over a newer rack is refused");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), newer, "the newer file is untouched");
    // Another format too.
    std::fs::write(&path, r#"{"format":"yahaha.registration-bank","version":1}"#).unwrap();
    assert!(Rack::load(&path).is_err());
    assert!(sample().save(&path).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

/// #247: a rack saved before the part EQ (no `eq`) reads as flat and is written back
/// exactly as it was; a part's EQ is saved and read back.
#[test]
fn a_rack_without_part_eq_loads_flat_and_is_unchanged() {
    let dir = temp_dir("old-eq");
    let _ = std::fs::create_dir_all(&dir);
    let mut old = serde_json::to_value(sample()).unwrap();
    let parts = old["parts"].as_array_mut().unwrap();
    assert_eq!(parts[1]["eq"], json!({ "lowGain": -3, "lowFreq": 125, "highGain": 4, "highFreq": 8000 }), "saved");
    assert!(parts[0].get("eq").is_none(), "a flat EQ is left out");
    for p in parts.iter_mut() {
        p.as_object_mut().unwrap().remove("eq");
    }
    let path = dir.join("Old.rack.json");
    let text = serde_json::to_string_pretty(&old).unwrap();
    std::fs::write(&path, &text).unwrap();
    let r = Rack::load(&path).unwrap();
    assert!(r.parts.iter().all(|p| p.eq == PartEq::FLAT && p.other.is_empty()));
    r.save(&path).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&std::fs::read_to_string(&path).unwrap()).unwrap(), old, "written back unchanged");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A rack saved before the insert slot (no `insert`) reads as off and is written back
/// exactly as it was; a part's slot is saved and read back, and one off at its defaults
/// is left out.
#[test]
fn a_rack_without_an_insert_slot_loads_off_and_is_unchanged() {
    let dir = temp_dir("old-insert");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("Old.rack.json");
    sample().save(&path).unwrap();
    assert_eq!(Rack::load(&path).unwrap().parts[1].insert, sample().parts[1].insert, "read back");
    let mut old = serde_json::to_value(sample()).unwrap();
    let parts = old["parts"].as_array_mut().unwrap();
    assert_eq!(parts[1]["insert"], json!({ "effect": "rotary", "on": true, "amount": 90 }), "saved");
    assert!(parts[0].get("insert").is_none(), "an off slot is left out");
    for p in parts.iter_mut() {
        p.as_object_mut().unwrap().remove("insert");
    }
    std::fs::write(&path, serde_json::to_string_pretty(&old).unwrap()).unwrap();
    let r = Rack::load(&path).unwrap();
    assert!(r.parts.iter().all(|p| p.insert == PartInsert::OFF && p.other.is_empty()));
    r.save(&path).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&std::fs::read_to_string(&path).unwrap()).unwrap(), old, "written back unchanged");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn unknown_fields_round_trip() {
    let dir = temp_dir("unknown");
    let path = path_for(&dir, "Ballad");
    let mut v = serde_json::to_value(sample()).unwrap();
    v["inserts"] = json!([{ "fx": "tape" }]);
    v["parts"][2]["macros"] = json!({ "cutoff": 3 });
    v["controls"]["knobs"][5] = json!({ "kind": "pluginMacro", "part": 0, "param": 7 });
    let r = Rack::from_json(&v.to_string()).unwrap();
    assert_eq!(r.other["inserts"], json!([{ "fx": "tape" }]));
    assert_eq!(r.parts[2].other["macros"], json!({ "cutoff": 3 }));
    let newer = json!({ "kind": "pluginMacro", "part": 0, "param": 7 });
    assert_eq!(r.controls.knobs[5], ControlTarget::Unknown(newer.clone()), "a target this build doesn't know is kept");
    r.save(&path).unwrap();
    let back: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(back["controls"]["knobs"][5], newer, "and written back verbatim");
    assert_eq!(back["inserts"], json!([{ "fx": "tape" }]));
    assert_eq!(back["parts"][2]["macros"], json!({ "cutoff": 3 }));
    assert_eq!(Rack::load(&path).unwrap(), r);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_default_controller_map_is_the_parts_page() {
    let m = ControlMap::default();
    let level = |part| ControlTarget::PartLevel { part };
    assert_eq!(m.faders, [level(0), level(1), level(2), level(3)]);
    assert_eq!(m.knobs[..4], [level(0), level(1), level(2), level(3)]);
    assert_eq!(m.knobs[4..], [ControlTarget::HarmonyVolume, ControlTarget::MetronomeVolume, ControlTarget::None, ControlTarget::Tempo]);
    // A map saved before it could be edited (none on knobs 5-8) reads as today's default;
    // any other is kept as saved.
    let mut v = serde_json::to_value(&m).unwrap();
    assert_eq!(v["version"], json!(1), "a map is written with its version");
    for k in 4..8 {
        v["knobs"][k] = json!({ "kind": "none" });
    }
    // Written by this build (marked): set that way on purpose, so kept, through a rack
    // file's save and load too.
    let edited: ControlMap = serde_json::from_value(v.clone()).unwrap();
    assert_eq!(edited.knobs[4..], [ControlTarget::None, ControlTarget::None, ControlTarget::None, ControlTarget::None]);
    let dir = temp_dir("map-none");
    let path = path_for(&dir, "Edited");
    let mut r = sample();
    r.controls = edited.clone();
    r.save(&path).unwrap();
    assert_eq!(Rack::load(&path).unwrap().controls, edited, "an edited map survives save and load");
    let _ = std::fs::remove_dir_all(&dir);
    // Unmarked (an older build's): migrated.
    v.as_object_mut().unwrap().remove("version");
    assert_eq!(serde_json::from_value::<ControlMap>(v.clone()).unwrap(), m, "the first default reads as today's");
    v["knobs"][6] = json!({ "kind": "tempo" });
    assert_eq!(serde_json::from_value::<ControlMap>(v).unwrap().knobs[4..], [ControlTarget::None, ControlTarget::None, ControlTarget::Tempo, ControlTarget::None]);
    // A rack without a map gets it; a short list is padded with none.
    let mut v = serde_json::to_value(sample()).unwrap();
    v.as_object_mut().unwrap().remove("controls");
    assert_eq!(Rack::from_json(&v.to_string()).unwrap().controls, m);
    v["controls"] = json!({ "faders": [{ "kind": "splitPoint" }], "knobs": [{ "kind": "harmonyArp" }, { "kind": "partPan", "part": 3 }] });
    let r = Rack::from_json(&v.to_string()).unwrap();
    assert_eq!(r.controls.faders, [ControlTarget::SplitPoint, ControlTarget::None, ControlTarget::None, ControlTarget::None]);
    assert_eq!(r.controls.knobs[..3], [ControlTarget::HarmonyArp, ControlTarget::PartPan { part: 3 }, ControlTarget::None]);
}

#[test]
fn rack_ids_are_unique() {
    let a = new_id();
    assert_ne!(a, new_id());
}
