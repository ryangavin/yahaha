//! A keyboard part's plugin in a Registration Memory (#104), through an offline session
//! with Apple's DLSMusicDevice (every Mac has it) and the real audio callback.

use crate::api::{LibraryCmd, PartsCmd, PluginCmd, PluginStatus, RegistrationCmd};
use crate::session::{Options, Port, Session};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const DLS: &str = "aumu dls  appl";

/// SlowWalker and BubblyDub, a data folder, and the audio callback on `sf2` (None: no
/// SoundFont, so only a plugin makes a sound).
fn session(test: &str, sf2: Option<&Path>) -> Option<(Session, PathBuf)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let a = root.join("corpus/MOX_v2/SlowWalker.T552.sty");
    let b = root.join("corpus/MOX_v2/BubblyDub.T552.sty");
    if !a.exists() || !b.exists() {
        eprintln!("corpus missing; skipping");
        return None;
    }
    let dir = std::env::temp_dir().join(format!("yahaha-plugreg-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let s = Session::offline(Options { paths: vec![a, b], data_dir: Some(dir.clone()), ..Options::default() }).unwrap();
    s.finish_indexing();
    s.offline_audio(sf2, 48_000).unwrap();
    Some((s, dir))
}

fn energy(l: &[f32], r: &[f32]) -> f64 {
    l.iter().chain(r).map(|x| (*x as f64).powi(2)).sum()
}

/// Pump until the part's plugin has finished loading; None when it has none.
fn wait_loaded(s: &Session, part: usize) -> Option<PluginStatus> {
    let t0 = Instant::now();
    loop {
        s.advance(1_000_000);
        match s.state().keyboard_parts[part].plugin.as_ref().map(|p| p.status) {
            Some(PluginStatus::Loading) if t0.elapsed() < Duration::from_secs(20) => std::thread::sleep(Duration::from_millis(5)),
            x => return x,
        }
    }
}

/// Pump until no plugin state read is running (a Memorize's fill lands the pump after).
fn wait_reads(s: &Session) {
    let t0 = Instant::now();
    while s.inner.lock().plugin_state_reads_pending() && t0.elapsed() < Duration::from_secs(10) {
        s.advance(1_000_000);
        std::thread::sleep(Duration::from_millis(2));
    }
    s.advance(1_000_000);
    s.advance(1_000_000);
}

/// A C5 on Right 1: the energy it makes.
fn c5(s: &Session) -> f64 {
    s.render(960);
    s.midi_in(Port::Keys, &[0x90, 72, 110]);
    let (l, r) = s.render(9600);
    s.midi_in(Port::Keys, &[0x80, 72, 0]);
    s.render(24_000);
    energy(&l, &r)
}

/// Button `b`'s stored Right 1 voice.
fn stored_voice(s: &Session, b: usize) -> Value {
    let ctl = s.inner.lock();
    ctl.reg.bank.memories[b].as_ref().unwrap().sections["parts"]["parts"][0]["voice"].clone()
}

/// The owner's report (#104): after a recall of a GM button and then a plugin button,
/// Right 1 showed its plugin Playing, on, at 100, and a C5 was silent. With no SoundFont
/// only the plugin can sound: the GM button is silent, the plugin button sounds, every
/// time, whether the buttons are pressed with time between them or at once.
#[test]
fn a_plugin_button_after_a_gm_button_sounds() {
    let Some((s, dir)) = session("gm-then-plugin", None) else { return };
    s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();
    assert_eq!(c5(&s), 0.0, "the GM button: no SoundFont, silence");
    s.send(PluginCmd::SetPartPlugin { part: 0, id: DLS.into(), state: None }).unwrap();
    assert_eq!(wait_loaded(&s, 0), Some(PluginStatus::Playing));
    assert!(c5(&s) > 1e-3, "the plugin sounds");
    s.send(RegistrationCmd::MemorizeRegist { index: 1 }).unwrap();
    wait_reads(&s);
    for round in 0..4 {
        let at_once = round % 2 == 1;
        s.send(RegistrationCmd::RecallRegist { index: 0 }).unwrap();
        if !at_once {
            assert_eq!(wait_loaded(&s, 0), None, "round {round}: the GM button clears the plugin");
            assert_eq!(c5(&s), 0.0, "round {round}: GM: silence");
        }
        s.send(RegistrationCmd::RecallRegist { index: 1 }).unwrap();
        assert_eq!(wait_loaded(&s, 0), Some(PluginStatus::Playing), "round {round}");
        let st = s.state();
        let p = &st.keyboard_parts[0];
        assert!(p.on && p.volume > 0, "round {round}: on, with a level");
        assert!(c5(&s) > 1e-3, "round {round}: the plugin button's Right 1 sounds");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The same with a SoundFont, another style, voice, level, octave and transpose on the
/// GM button (the recall waits for its style), and a SoundFont library patch on Right 1.
#[test]
fn a_plugin_button_after_a_gm_button_with_a_style_and_a_patch_sounds() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("soundfonts");
    let Some(sf2) = crate::library::sound_font_files(&dir).into_iter().map(|f| dir.join(f)).min_by_key(|p| p.metadata().map(|m| m.len()).unwrap_or(u64::MAX)) else {
        eprintln!("no SoundFont; skipping");
        return;
    };
    let Some((s, data)) = session("gm-style-patch", Some(&sf2)) else { return };
    let id = s.library_list().entries.iter().find(|e| e.path.contains("BubblyDub")).unwrap().id;
    s.send(LibraryCmd::LoadStyle { id }).unwrap();
    s.advance(1_000_000);
    s.send(PartsCmd::SetPartVoice { part: 0, program: 48 }).unwrap();
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 90 }).unwrap();
    s.send(PartsCmd::SetPartOctave { part: 0, octave: 1 }).unwrap();
    s.send(crate::api::ChordCmd::SetTranspose { keyboard: 2, master: -1 }).unwrap();
    let fields = crate::api::PatchFields {
        name: "Strings".into(),
        category: Default::default(),
        tags: vec![],
        favourite: false,
        source: crate::patches::PatchSource::SoundFont { file: sf2.file_name().unwrap().to_string_lossy().into(), bank: 0, program: 48 },
        defaults: crate::patches::PatchDefaults::default(),
    };
    s.send(crate::api::SoundLibraryCmd::CreatePatch { patch: fields }).unwrap();
    let patch = s.state().sound_library.last_added.clone().unwrap();
    s.send(crate::api::SoundLibraryCmd::SetPartPatch { part: 0, id: Some(patch.clone()) }).unwrap();
    s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();

    let id = s.library_list().entries.iter().find(|e| e.path.contains("SlowWalker")).unwrap().id;
    s.send(LibraryCmd::LoadStyle { id }).unwrap();
    s.advance(1_000_000);
    s.send(PluginCmd::SetPartPlugin { part: 0, id: DLS.into(), state: None }).unwrap();
    assert_eq!(wait_loaded(&s, 0), Some(PluginStatus::Playing));
    assert!(s.state().keyboard_parts[0].patch.is_none(), "the plugin ends the patch");
    s.send(RegistrationCmd::MemorizeRegist { index: 1 }).unwrap();
    wait_reads(&s);
    for round in 0..3 {
        s.send(RegistrationCmd::RecallRegist { index: 0 }).unwrap();
        s.advance(1_000_000);
        assert_eq!(wait_loaded(&s, 0), None, "round {round}");
        assert_eq!(s.state().keyboard_parts[0].patch.as_deref(), Some(patch.as_str()), "round {round}: its patch");
        assert!(c5(&s) > 1e-3, "round {round}: the patch sounds");
        s.send(RegistrationCmd::RecallRegist { index: 1 }).unwrap();
        s.advance(1_000_000);
        assert_eq!(wait_loaded(&s, 0), Some(PluginStatus::Playing), "round {round}");
        assert!(s.state().keyboard_parts[0].patch.is_none(), "round {round}");
        assert!(c5(&s) > 1e-3, "round {round}: the plugin button's Right 1 sounds");
    }
    let _ = std::fs::remove_dir_all(&data);
}

/// Memorize stores the plugin's id, name and state as it is at Memorize (read off the
/// control thread, then filled in); Regist Bank Info names it; a recall brings it back
/// with that state, and a recall of what already plays reloads nothing.
#[test]
fn a_registration_stores_and_recalls_a_parts_plugin() {
    let Some((s, dir)) = session("store-recall", None) else { return };
    s.send(PartsCmd::SetPartVoice { part: 0, program: 5 }).unwrap();
    s.send(PluginCmd::SetPartPlugin { part: 0, id: DLS.into(), state: None }).unwrap();
    assert_eq!(wait_loaded(&s, 0), Some(PluginStatus::Playing));
    assert_eq!(s.inner.lock().part_plugin_voice(0).unwrap().1, None, "no state saved yet");
    s.send(RegistrationCmd::MemorizeRegist { index: 2 }).unwrap();
    let v = stored_voice(&s, 2);
    assert_eq!((v["kind"].as_str(), v["id"].as_str(), v["name"].as_str(), v["program"].as_u64()), (Some("plugin"), Some(DLS), Some("DLSMusicDevice"), Some(5)));
    assert!(v.get("state").is_none(), "at first, the state saved before (none)");
    wait_reads(&s);
    let state = stored_voice(&s, 2)["state"].as_str().map(str::to_string).expect("the state read at Memorize is filled in");
    assert!(state.len() > 100);
    assert_eq!(s.state().registration.buttons[2].voices[0].name, "DLSMusicDevice");

    // Back to the GM voice, then the button: the plugin, with the stored state.
    s.send(PluginCmd::ClearPartPlugin { part: 0 }).unwrap();
    s.send(PartsCmd::SetPartVoice { part: 0, program: 20 }).unwrap();
    s.send(RegistrationCmd::RecallRegist { index: 2 }).unwrap();
    assert!(s.state().keyboard_parts[0].plugin.is_some(), "loading (or preloaded: playing)");
    assert_eq!(wait_loaded(&s, 0), Some(PluginStatus::Playing));
    assert_eq!(s.inner.lock().part_plugin_voice(0), Some((DLS.to_string(), Some(state.clone()))));
    assert_eq!(s.state().keyboard_parts[0].program, 5, "the GM voice underneath");
    assert!(c5(&s) > 1e-3);
    // Again: it already plays it, so nothing reloads.
    s.send(RegistrationCmd::RecallRegist { index: 2 }).unwrap();
    assert_eq!(s.state().keyboard_parts[0].plugin.as_ref().map(|p| p.status), Some(PluginStatus::Playing));

    // A plugin that isn't installed: the part plays its GM voice, and the recall says so.
    // (Refused from the plugin list, which a live session scans at start: the load itself
    // never scans on the control thread.)
    s.inner.lock().start_plugin_scan(false);
    let t0 = Instant::now();
    while s.state().plugins.scanning || s.state().plugins.list.is_empty() {
        assert!(t0.elapsed() < Duration::from_secs(60), "the plugin scan did not finish");
        s.advance(1_000_000);
        std::thread::sleep(Duration::from_millis(5));
    }
    {
        let mut ctl = s.inner.lock();
        let m = ctl.reg.bank.memories[2].as_mut().unwrap();
        m.sections.get_mut("parts").unwrap()["parts"][0]["voice"]["id"] = Value::String("aumu Nope Gone".into());
    }
    s.send(RegistrationCmd::RecallRegist { index: 2 }).unwrap();
    s.advance(1_000_000);
    assert!(s.state().keyboard_parts[0].plugin.is_none(), "not left on the old plugin");
    assert!(s.state().message.as_ref().is_some_and(|m| m.error && m.text.contains("GM voice")), "{:?}", s.state().message);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A GM button leaves a part's library plugin patch to the `patch` recall, and a part
/// playing a plugin patch stores the patch, not a plugin voice.
#[test]
fn a_plugin_patch_is_stored_as_the_patch() {
    use crate::api::{PatchFields, SoundLibraryCmd};
    use crate::patches::{PatchDefaults, PatchSource};
    let Some((s, dir)) = session("plugin-patch", None) else { return };
    let fields = PatchFields {
        name: "DLS Keys".into(),
        category: Default::default(),
        tags: vec![],
        favourite: false,
        source: PatchSource::Plugin { component_id: DLS.into(), state: String::new() },
        defaults: PatchDefaults::default(),
    };
    s.send(SoundLibraryCmd::CreatePatch { patch: fields }).unwrap();
    let id = s.state().sound_library.last_added.clone().unwrap();
    s.send(SoundLibraryCmd::SetPartPatch { part: 0, id: Some(id.clone()) }).unwrap();
    assert_eq!(wait_loaded(&s, 0), Some(PluginStatus::Playing));
    s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();
    wait_reads(&s);
    assert_eq!(stored_voice(&s, 0)["kind"].as_str(), Some("gm"));
    s.send(PartsCmd::SetPartVoice { part: 0, program: 0 }).unwrap();
    assert!(s.state().keyboard_parts[0].plugin.is_none());
    s.send(RegistrationCmd::RecallRegist { index: 0 }).unwrap();
    assert_eq!(s.state().keyboard_parts[0].patch.as_deref(), Some(id.as_str()));
    assert_eq!(wait_loaded(&s, 0), Some(PluginStatus::Playing), "the patch's plugin");
    assert!(c5(&s) > 1e-3);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Pump until `n` preloaded instances are ready (or 20 s).
fn wait_warm(s: &Session, n: usize) -> usize {
    let t0 = Instant::now();
    loop {
        s.advance(1_000_000);
        let k = s.inner.lock().warm_ready();
        if k == n || t0.elapsed() > Duration::from_secs(20) {
            return k;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Selecting a bank preloads its buttons' plugins (with their states) off the real-time
/// threads, so a recall plays at the next pump instead of loading; the pool refills what a
/// press used, and a bank without plugins lets them go.
#[test]
fn a_selected_banks_plugins_are_preloaded() {
    let Some((s, dir)) = session("warm", None) else { return };
    s.send(PluginCmd::SetPartPlugin { part: 0, id: DLS.into(), state: None }).unwrap();
    assert_eq!(wait_loaded(&s, 0), Some(PluginStatus::Playing));
    s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();
    wait_reads(&s);
    s.send(RegistrationCmd::SaveRegistBank { name: Some("Warm".into()), overwrite: false }).unwrap();
    let path = s.state().registration.banks.iter().find(|b| b.name == "Warm").unwrap().path.clone();
    s.send(PluginCmd::ClearPartPlugin { part: 0 }).unwrap();
    s.send(RegistrationCmd::NewRegistBank).unwrap();
    assert_eq!(wait_warm(&s, 0), 0, "an empty bank keeps nothing warm");

    s.send(RegistrationCmd::SelectRegistBank { path }).unwrap();
    assert_eq!(wait_warm(&s, 1), 1, "the bank's plugin is preloaded");
    assert!(s.state().keyboard_parts[0].plugin.is_none(), "preloaded, not playing");
    let used = s.inner.lock().plugins.warm.used;
    s.send(RegistrationCmd::RecallRegist { index: 0 }).unwrap();
    assert_eq!(s.inner.lock().plugins.warm.used, used + 1, "the recall takes the preloaded instance");
    s.advance(1_000_000);
    assert_eq!(s.state().keyboard_parts[0].plugin.as_ref().map(|p| p.status), Some(PluginStatus::Playing), "the recall plays at the next pump");
    let stored = stored_voice(&s, 0)["state"].as_str().map(str::to_string);
    assert_eq!(s.inner.lock().part_plugin_voice(0).unwrap().1, stored, "with the button's state");
    assert!(c5(&s) > 1e-3);
    assert_eq!(wait_warm(&s, 1), 1, "the pool refills");
    s.send(RegistrationCmd::NewRegistBank).unwrap();
    assert_eq!(wait_warm(&s, 0), 0);
    assert!(s.inner.lock().plugins.warm.entries.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

/// What a bank preloads: each plugin voice once per part a single button plays it on, in
/// button order; the pool keeps at most `MAX_WARM`.
#[test]
fn a_banks_plugin_voices() {
    use crate::registration::{Bank, Memory};
    use serde_json::json;
    let voice = |id: &str, state: Option<&str>| match state {
        Some(s) => json!({ "kind": "plugin", "id": id, "state": s }),
        None => json!({ "kind": "plugin", "id": id }),
    };
    let button = |voices: Vec<Value>| {
        let parts: Vec<Value> = voices.into_iter().map(|v| json!({ "on": true, "voice": v, "volume": 100, "octave": 0 })).collect();
        let mut m = Memory::default();
        m.sections.insert("parts".into(), json!({ "parts": parts }));
        Some(m)
    };
    let mut b = Bank::new("b");
    b.memories[0] = button(vec![voice("aumu a", None), voice("aumu a", None), json!({ "kind": "gm", "program": 1 })]);
    b.memories[3] = button(vec![voice("aumu a", None), voice("aumu b", Some("AQID"))]);
    b.memories[5] = button(vec![voice("aumu b", Some("AQID")), voice("aumu b", Some("not base64!"))]);
    let v = super::bank_plugin_voices(&b);
    let ids: Vec<(&str, Option<&[u8]>)> = v.iter().map(|v| (v.id.as_str(), v.state.as_deref())).collect();
    assert_eq!(ids, [("aumu a", None), ("aumu a", None), ("aumu b", Some(&[1u8, 2, 3][..]))]);
    for i in 0..10 {
        b.memories[i] = button(vec![voice(&format!("aumu {i}"), None)]);
    }
    assert_eq!(super::bank_plugin_voices(&b).len(), 10);
    let Some((s, dir)) = session("warm-cap", None) else { return };
    s.inner.lock().warm_plugins(super::bank_plugin_voices(&b));
    assert!(s.inner.lock().plugins.warm.entries.is_empty(), "unknown plugins are not loaded");
    let dls: Vec<crate::session::PluginVoice> = (0..10).map(|_| crate::session::PluginVoice { id: DLS.into(), state: None }).collect();
    s.inner.lock().warm_plugins(dls);
    assert_eq!(s.inner.lock().plugins.warm.entries.len(), crate::session::plugins::pool::MAX_WARM, "bounded");
    let _ = std::fs::remove_dir_all(&dir);
}
