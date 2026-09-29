//! Sound-library tests that need the session's types (`session::PluginVoice`). They moved
//! here from the synth crate's patches/sound_tests.rs in the crate split: the session sits
//! above yahaha-synth (AGENTS.md, Layering).

use crate::patches::{Category, PluginOrigin, SoundLibrary, SoundTag};

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
