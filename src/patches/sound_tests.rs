//! The unified Sound model, its migrations, and the GM map's data shape
//! (docs/sound-browser.md).

use super::tests::library;
use super::*;

#[test]
fn sound_ids_round_trip() {
    let f = SoundId::parse("sf:Fluid R3:GM.sf2:128:0").unwrap();
    assert_eq!(f, SoundId::Font(FontPreset::new("Fluid R3:GM.sf2", 128, 0)));
    assert_eq!(f.to_string(), "sf:Fluid R3:GM.sf2:128:0");
    assert_eq!(SoundId::parse("saved:my-bass"), Some(SoundId::Library("my-bass".into())));
    // A bare id is a library patch id (what rules and part patches store).
    assert_eq!(SoundId::parse("my-bass").unwrap().to_string(), "saved:my-bass");
    assert_eq!(SoundId::parse("sf:x.sf2:0:128"), None);
    assert_eq!(SoundId::parse("au:aumu dls  appl"), None);
    assert_eq!(SoundId::parse(""), None);
    assert_eq!(PluginOrigin::from_preset_key("f:12"), Some(PluginOrigin::Factory { number: 12 }));
    assert_eq!(PluginOrigin::from_preset_key("u:/a/b.aupreset"), Some(PluginOrigin::File { path: "/a/b.aupreset".into() }));
    assert_eq!(PluginOrigin::from_preset_key("x"), None);
}

/// A version 1 library (before plugin sounds had an origin) reads as it is: its plugin
/// patches are `user` sounds, its rules (global and per-style) unchanged; it is written
/// back as version 2 with no `origin` on them, and an origin round-trips.
#[test]
fn a_version_1_library_migrates_unchanged() {
    let v1 = r#"{"version": 1, "patches": [
        {"id": "keys", "name": "Keys", "source": {"kind": "plugin", "componentId": "aumu:abcd:manu", "state": "00ff"}},
        {"id": "bass", "name": "Bass", "source": {"kind": "soundFont", "file": "x.sf2", "bank": 0, "program": 33}}],
        "map": {"families": [null,null,null,null,"bass",null,null,null,null,null,null,null,null,null,null,null], "drums": null, "overrides": [{"program": 4, "patch": "keys"}]},
        "styleMaps": {"Cool8Beat.S910.sty": {"drums": "bass"}}}"#;
    let mut lib = SoundLibrary::from_json(v1).unwrap();
    assert_eq!(lib.version, 2);
    assert_eq!(lib.patches[0].source, PatchSource::plugin("aumu:abcd:manu", "00ff"));
    assert!(!lib.patches[0].awaits_capture());
    assert_eq!((lib.map.families[4].as_deref(), lib.map.override_of(4)), (Some("bass"), Some("keys")));
    assert_eq!(lib.style_maps["Cool8Beat.S910.sty"].drums.as_deref(), Some("bass"));
    let json: serde_json::Value = serde_json::from_str(&lib.to_json()).unwrap();
    assert_eq!(json["version"], 2);
    assert!(json["patches"][0]["source"].get("origin").is_none());
    lib.add_plugin_preset("aumu:abcd:manu", PluginOrigin::File { path: "/p/Warm.aupreset".into() }, "Warm", Category::Pad, Some("aa".into())).unwrap();
    let back = SoundLibrary::from_json(&lib.to_json()).unwrap();
    assert_eq!(back, lib);
    assert_eq!(serde_json::to_value(&back.patches[2].source).unwrap()["origin"], serde_json::json!({"kind": "file", "path": "/p/Warm.aupreset"}));
}

/// A factory preset is one plugin sound, found again by its origin; its state is empty
/// until it first plays, then captured once (later reads never overwrite it).
#[test]
fn a_factory_preset_is_captured_the_first_time_it_plays() {
    let mut lib = SoundLibrary::default();
    let f = PluginOrigin::Factory { number: 3 };
    let id = lib.add_plugin_preset("aumu Smp7 Fake", f.clone(), "Brass Stabs", Category::Brass, None).unwrap();
    assert_eq!(lib.add_plugin_preset("aumu Smp7 Fake", f.clone(), "Other name", Category::Pad, None), Some(id.clone()), "the same preset is the same sound");
    assert_eq!(lib.patches.len(), 1);
    assert!(lib.patch(&id).unwrap().awaits_capture());
    assert!(lib.capture_state(&id, "c3RhdGU="));
    assert!(!lib.patch(&id).unwrap().awaits_capture());
    assert!(!lib.capture_state(&id, "ZWRpdGVk"), "an edit is the user's to save");
    assert_eq!(lib.patch(&id).unwrap().source, PatchSource::Plugin { component_id: "aumu Smp7 Fake".into(), state: "c3RhdGU=".into(), origin: f });
    // A user sound never awaits a capture, and two made in yahaha are two sounds.
    let u = lib.add_plugin_preset("aumu Smp7 Fake", PluginOrigin::User, "Mine", Category::Pad, None).unwrap();
    assert!(!lib.capture_state(&u, "eA=="));
    assert_ne!(lib.add_plugin_preset("aumu Smp7 Fake", PluginOrigin::User, "Mine", Category::Pad, None).unwrap(), u);
}

/// plugin-parts.json from before sounds had ids: a part's voice (a factory preset by key,
/// no state yet) reads with no sound, and becomes the library's sound for that preset.
#[test]
fn plugin_parts_json_migrates_to_a_sound() {
    let old = r#"{"id": "aumu Smp7 Fake", "state": null, "preset": {"key": "f:1", "name": "Bright Grand"}}"#;
    let v: crate::session::PluginVoice = serde_json::from_str(old).unwrap();
    assert_eq!(v.sound, None);
    let mut lib = SoundLibrary::default();
    let p = v.preset.clone().unwrap();
    let id = lib.add_plugin_preset(&v.id, PluginOrigin::from_preset_key(&p.key).unwrap(), &p.name, Category::Piano, None).unwrap();
    let tag = lib.patch(&id).unwrap().tag();
    assert_eq!(tag, SoundTag { id: "saved:bright-grand".into(), name: "Bright Grand".into() });
    let v = crate::session::PluginVoice { sound: Some(tag), ..v };
    let json = serde_json::to_value(&v).unwrap();
    assert_eq!(json["sound"], serde_json::json!({"id": "saved:bright-grand", "name": "Bright Grand"}));
    // A voice with no sound writes none (older builds read the file as before).
    let bare = crate::session::PluginVoice { id: "x".into(), ..Default::default() };
    assert!(serde_json::to_value(&bare).unwrap().get("sound").is_none());
}

/// A Registration from before sounds had ids: its plugin voice reads with no sound, and
/// the library's sound with exactly that state is found for it.
#[test]
fn registration_plugin_voices_migrate() {
    use crate::registration::VoiceRef;
    let mut lib = library();
    let old: VoiceRef = serde_json::from_str(r#"{"kind":"plugin","id":"aumu:abcd:manu","name":"Keys (AU)","state":"00ff","program":4}"#).unwrap();
    let VoiceRef::Plugin { id, state, sound, .. } = &old else { panic!("a plugin voice") };
    assert_eq!(*sound, None);
    assert_eq!(lib.tag_for_state(id, state.as_deref().unwrap()), Some(SoundTag { id: "saved:keys".into(), name: "Keys (AU)".into() }));
    assert_eq!(lib.tag_for_state(id, "ffff"), None, "an edited state is no sound of the library's");
    assert_eq!(lib.tag_for_state(id, ""), None);
    lib.patches.clear();
    assert_eq!(lib.tag_for_state("aumu:abcd:manu", "00ff"), None);
    // Written back without a sound, it is the old record exactly.
    assert!(serde_json::to_value(&old).unwrap().get("sound").is_none());
}

/// OTS voices (a style's One Touch Settings, and the OTS a Registration keeps) are GM
/// voices: the record is unchanged, and they resolve through the map as before.
#[test]
fn ots_voices_migrate_unchanged() {
    use crate::registration::VoiceRef;
    let ots: VoiceRef = serde_json::from_str(r#"{"kind":"gm","program":33,"bankMsb":0,"bankLsb":0}"#).unwrap();
    assert_eq!(ots, VoiceRef::gm(33));
    assert_eq!(serde_json::to_string(&ots).unwrap(), r#"{"kind":"gm","program":33,"bankMsb":0,"bankLsb":0}"#);
    let lib = library();
    let r = resolve_gm(&lib, None, &AutoFill::default(), false, ots.program().unwrap());
    assert_eq!((r.sound.as_deref(), r.layer), (Some("saved:bass"), Layer::Family));
}

fn preset(bank: u16, program: u8) -> sf2::Preset {
    sf2::Preset { bank, program, name: format!("{bank}:{program}") }
}

#[test]
fn auto_fill_prefers_the_most_gm_complete_font() {
    let full: Vec<sf2::Preset> = (0..128).map(|p| preset(0, p)).chain([preset(128, 0), preset(128, 25)]).collect();
    let small = vec![preset(0, 0), preset(0, 33), preset(8, 120), preset(128, 16)];
    let fonts = vec![("a-small.sf2".to_string(), small.clone()), ("b-full.sf2".to_string(), full[1..].to_vec())];
    let auto = AutoFill::build(&fonts);
    assert_eq!(auto.programs.len(), 128);
    assert_eq!(auto.get(false, 33), Some(&FontPreset::new("b-full.sf2", 0, 33)), "the fuller font first");
    assert_eq!(auto.get(false, 0), Some(&FontPreset::new("a-small.sf2", 0, 0)), "a program the fuller one lacks");
    assert_eq!(auto.get(true, 9), Some(&FontPreset::new("b-full.sf2", 128, 0)));
    // Only on a variation bank: that preset; nowhere: none.
    let auto = AutoFill::build(&[("s.sf2".to_string(), small)]);
    assert_eq!(auto.get(false, 120), Some(&FontPreset::new("s.sf2", 8, 120)));
    assert_eq!(auto.get(false, 1), None);
    assert_eq!(auto.get(true, 0), Some(&FontPreset::new("s.sf2", 128, 16)), "no kit 0: the lowest kit");
    assert_eq!(AutoFill::build(&[]).get(false, 0), None);
}

/// Every layer, style over global, auto last; font resolutions carry their provenance
/// (file, bank, program), plugin sounds none.
#[test]
fn the_gm_map_resolves_by_layer_with_font_provenance() {
    let lib = library();
    let auto = AutoFill::build(&[("auto.sf2".to_string(), (0..128).map(|p| preset(0, p)).chain([preset(128, 0)]).collect())]);
    let style = Some("Cool8Beat.S910.sty");
    let r = resolve_gm(&lib, None, &auto, false, 33);
    assert_eq!((r.sound.as_deref(), r.layer, r.from_style), (Some("saved:bass"), Layer::Family, false));
    assert_eq!(r.font, Some(FontPreset::new("GeneralUser-GS.sf2", 0, 33)));
    let r = resolve_gm(&lib, style, &auto, false, 33);
    assert_eq!((r.sound.as_deref(), r.layer, r.from_style, r.font), (Some("saved:keys"), Layer::Override, true, None));
    assert_eq!(resolve_gm(&lib, None, &auto, false, 4).layer, Layer::Override);
    let r = resolve_gm(&lib, style, &auto, true, 0);
    assert_eq!((r.layer, r.font), (Layer::Drums, Some(FontPreset::new("GeneralUser-GS.sf2", 128, 25))));
    let r = resolve_gm(&lib, None, &auto, false, 40);
    assert_eq!((r.sound.as_deref(), r.layer, r.font), (Some("sf:auto.sf2:0:40"), Layer::Auto, Some(FontPreset::new("auto.sf2", 0, 40))));
    assert_eq!(resolve_gm(&lib, None, &AutoFill::default(), false, 40).layer, Layer::None);
    // The map page: drums first, then 128 programs with their family and rules.
    let rows = gm_map_rows(&lib, style, &auto);
    assert_eq!(rows.len(), 129);
    assert_eq!((rows[0].program, rows[0].family_rule.as_deref()), (None, Some("kit")));
    let r33 = &rows[34];
    assert_eq!((r33.program, r33.family, r33.override_rule.as_deref(), r33.family_rule.as_deref()), (Some(33), Some(4), Some("keys"), Some("bass")));
    assert_eq!(rows[41].resolved.layer, Layer::Auto);
}
