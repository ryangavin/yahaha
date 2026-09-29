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
        strip: StripReg::default(),
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
        sends: RackSends::default(),
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

/// A v1 rack written before the channel strip (the mixer rework), as that build wrote it.
fn old_rack_json() -> Value {
    let part = |reverb: u8, chorus: u8, variation: u8| {
        json!({
            "on": true,
            "sound": { "kind": "font", "file": "GeneralUser.sf2", "bank": 0, "program": 0 },
            "volume": 100, "pan": 64, "reverb": reverb, "chorus": chorus, "variation": variation,
            "octave": 0, "bendRange": 2
        })
    };
    let mut rotary = part(50, 10, 5);
    rotary["insert"] = json!({ "effect": "rotary", "on": true, "amount": 90 });
    json!({
        "format": "yahaha.rack", "version": 1, "id": "r1", "name": "Old",
        "parts": [rotary, part(40, 0, 0), part(0, 0, 0), part(20, 30, 40)],
        "split": 54,
        "harmonyArp": {
            "on": false, "mode": "harmony", "harmonyType": "Duet", "arpPattern": "Up Oct", "volume": 90,
            "speed": "1/8", "assign": "auto", "chordNoteOnly": false, "touchLimit": 1, "arpQuantize": "off",
            "arpHold": false, "arpVelocity": "original", "arpFixedVelocity": 100, "arpKeepKeyOn": false
        },
        "transpose": 0
    })
}

/// A rack saved before the strip reads with the strip mirroring the older fields: sends
/// 1-3 the reverb, chorus and variation sends, insert 1 the old slot, nothing else.
#[test]
fn an_old_rack_loads_with_the_strip_mirroring_its_fields() {
    let r = Rack::from_json(&old_rack_json().to_string()).unwrap();
    let s = &r.parts[0].strip;
    assert_eq!(s.sends, [50, 10, 5, 0, 0, 0]);
    let mut rotary = InsertSlot::of(InsertType::Rotary);
    rotary.values[0] = 90;
    assert_eq!(s.inserts[0], rotary);
    assert!(s.inserts[1].is_default());
    assert_eq!(s.comp, None);
    assert_eq!(r.parts[3].strip.sends, [20, 30, 40, 0, 0, 0]);
    assert!(r.parts[3].strip.inserts[0].is_default(), "an old slot off at its defaults is an empty insert 1");
    assert!(r.sends.is_empty());
    assert!(r.parts.iter().all(|p| p.other.is_empty()) && r.other.is_empty());
}

/// A rack saved before the strip is written back byte for byte: no "strip" or "sends".
#[test]
fn an_old_rack_writes_back_byte_identical() {
    let text = sample().to_json();
    assert!(!text.contains("\"strip\"") && !text.contains("\"sends\""), "{text}");
    let r = Rack::from_json(&text).unwrap();
    assert_eq!(r.parts[1].strip.inserts[0].values[0], 90, "normalized on load");
    assert_eq!(r.to_json(), text);
    // And the inline fixture: loaded and written, it says the same (and no strip).
    let old = old_rack_json();
    let back: Value = serde_json::from_str(&Rack::from_json(&old.to_string()).unwrap().to_json()).unwrap();
    let mut want = old.clone();
    want["controls"] = serde_json::to_value(ControlMap::default()).unwrap();
    assert_eq!(back, want);
}

/// A rack using every new field saves and loads equal; unknown kinds come back by name.
#[test]
fn a_rack_with_every_strip_field_round_trips() {
    let dir = temp_dir("strip");
    let path = path_for(&dir, "Strip");
    let mut r = sample();
    let p = &mut r.parts[1];
    p.strip.comp = Some(PartComp::of(true, crate::fx::master::CompPreset::Punchy));
    let mut phaser = InsertSlot::of(InsertType::Phaser);
    phaser.values[1] = 120;
    p.strip.inserts[1] = phaser;
    // Insert 1 is the rotary `insert` says, with its second value of its own.
    p.normalize();
    p.strip.inserts[0].values[1] = 30;
    p.strip.sends[3..].copy_from_slice(&[11, 22, 33]);
    let mut odd = r.parts[2].clone();
    odd.strip.inserts[1] = InsertSlot { kind: InsertType::Unknown("ringModulator".into()), on: true, values: [1, 2, 3, 4] };
    r.parts[2] = odd;
    r.sends.added = vec![SendSlot::of(SendKind::Phaser), SendSlot { kind: SendKind::Unknown("shimmer".into()), params: [1, 2, 3, 4, 5, 6], return_level: 70 }];
    r.sends.override_[1] = Some(SendSlot::of(SendKind::Flanger));
    r.save(&path).unwrap();
    let back = Rack::load(&path).unwrap();
    assert_eq!(back, r);
    assert_eq!(back.parts[1].strip, r.parts[1].strip, "the strip itself, mirrors included");
    assert_eq!(back.parts[1].strip.inserts[0].values[..2], [90, 30], "insert 1's own values kept");
    assert_eq!(back.parts[2].strip.inserts[1].kind, InsertType::Unknown("ringModulator".into()));
    assert_eq!(back.sends.added[1].kind, SendKind::Unknown("shimmer".into()));
    let v: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(v["parts"][2]["strip"]["inserts"][1]["kind"], "ringModulator");
    assert_eq!(v["sends"]["added"][1]["kind"], "shimmer");
    assert_eq!(v["sends"]["override"], json!([null, SendSlot::of(SendKind::Flanger), null]));
    assert!(v["parts"][0].get("strip").is_none(), "a part with nothing new has no strip");
    // An insert 1 of a kind this build doesn't know, beside an off `insert`, is kept.
    let mut v = v;
    v["parts"][3]["strip"] = json!({ "inserts": [{ "kind": "tapeEcho", "on": true, "values": [5, 6, 7, 8] }, {}], "sends": [0, 0, 0, 0, 0, 0] });
    let r = Rack::from_json(&v.to_string()).unwrap();
    assert_eq!(r.parts[3].strip.inserts[0].kind, InsertType::Unknown("tapeEcho".into()));
    assert_eq!(r.parts[3].strip.sends[..3], [40, 0, 0], "sends 1-3 are the older fields'");
    let again: Value = serde_json::from_str(&r.to_json()).unwrap();
    assert_eq!(again["parts"][3]["strip"]["inserts"][0]["kind"], "tapeEcho");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The older fields win over the strip's mirrors of them, on load and in comparisons.
#[test]
fn the_older_fields_are_the_source_of_truth() {
    let mut v = old_rack_json();
    v["parts"][0]["strip"] = json!({ "inserts": [{ "kind": "distortion", "on": false, "values": [1, 2, 3, 0] }, {}], "sends": [1, 2, 3, 4, 5, 6] });
    let r = Rack::from_json(&v.to_string()).unwrap();
    let s = &r.parts[0].strip;
    assert_eq!(s.sends, [50, 10, 5, 4, 5, 6]);
    assert_eq!((s.inserts[0].kind.clone(), s.inserts[0].on, s.inserts[0].values[0]), (InsertType::Rotary, true, 90));
    assert_eq!(s.inserts[0].values[1..3], InsertType::Rotary.defaults()[1..3], "a new kind: its defaults");
    // A part whose strip isn't normalized yet equals its normalized self.
    let mut p = sample().parts[1].clone();
    let q = { let mut q = p.clone(); q.normalize(); q };
    assert_ne!(p.strip, q.strip);
    assert_eq!(p, q);
    p.strip.sends[4] = 1;
    assert_ne!(p, q);
}

#[test]
fn an_added_send_list_is_at_most_three() {
    let mut v = serde_json::to_value(sample()).unwrap();
    v["sends"] = json!({ "added": [{ "kind": "hall" }, { "kind": "room" }, { "kind": "plate" }, { "kind": "phaser" }] });
    let r = Rack::from_json(&v.to_string()).unwrap();
    assert_eq!(r.sends.added.iter().map(|s| s.kind.clone()).collect::<Vec<_>>(), [SendKind::Hall, SendKind::Room, SendKind::Plate]);
    assert_eq!(r.sends.added[0].return_level, crate::fx::RETURN_UNITY);
}

#[test]
fn the_mixer_targets_round_trip_and_are_checked() {
    let targets = [
        (ControlTarget::PartInsertOn { part: 1, slot: 1 }, json!({ "kind": "partInsertOn", "part": 1, "slot": 1 })),
        (ControlTarget::PartInsertSetting { part: 3, slot: 0, setting: 2 }, json!({ "kind": "partInsertSetting", "part": 3, "slot": 0, "setting": 2 })),
        (ControlTarget::PartSend { part: 0, send: 5 }, json!({ "kind": "partSend", "part": 0, "send": 5 })),
        (ControlTarget::PartDelay { part: 2 }, json!({ "kind": "partDelay", "part": 2 })),
        (ControlTarget::RotaryFast, json!({ "kind": "rotaryFast" })),
    ];
    for (t, j) in &targets {
        assert_eq!(serde_json::to_value(t).unwrap(), *j);
        assert_eq!(serde_json::from_value::<ControlTarget>(j.clone()).unwrap(), *t);
        assert!(t.is_known(), "{t:?}");
    }
    for t in [
        ControlTarget::PartInsertOn { part: 4, slot: 0 },
        ControlTarget::PartInsertOn { part: 0, slot: 2 },
        ControlTarget::PartInsertSetting { part: 0, slot: 0, setting: 4 },
        ControlTarget::PartInsertSetting { part: 0, slot: 2, setting: 0 },
        ControlTarget::PartSend { part: 0, send: 6 },
        ControlTarget::PartSend { part: 4, send: 0 },
        ControlTarget::PartDelay { part: 4 },
    ] {
        assert!(!t.is_known(), "{t:?}");
    }
    // Through a rack file's map.
    let mut r = sample();
    for (k, (t, _)) in targets.iter().enumerate() {
        r.controls.set(RackControl::Knob, k as u8, t.clone()).unwrap();
    }
    assert!(r.controls.set(RackControl::Fader, 0, ControlTarget::PartSend { part: 0, send: 6 }).is_err());
    assert_eq!(Rack::from_json(&r.to_json()).unwrap().controls, r.controls);
}
