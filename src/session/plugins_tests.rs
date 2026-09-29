//! Plugin parts through an offline session, with Apple's DLSMusicDevice (every Mac has
//! it) and the real audio callback (`Session::offline_audio` / `render`).

use super::super::testing::{self, session};
use super::super::{Options, Port, Session};
use crate::api::{PluginCmd, PluginStatus, PartsCmd};
use std::time::{Duration, Instant};

const DLS: &str = "aumu dls  appl";

fn energy(l: &[f32], r: &[f32]) -> f64 {
    l.iter().chain(r).map(|x| (*x as f64).powi(2)).sum()
}

/// Scan the plugins (the scan cache, on the `plugin-scan` thread) and pump until the list
/// is in.
fn wait_scanned(s: &Session) {
    s.inner.lock().start_plugin_scan(false);
    let t0 = Instant::now();
    while s.state().plugins.scanning || s.state().plugins.list.is_empty() {
        assert!(t0.elapsed() < Duration::from_secs(60), "the plugin scan did not finish");
        s.advance(1_000_000);
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Picking a plugin never scans on the control thread (#104 plug-rt PR 4). Before the
/// first scan is in, the load thread looks the id up: an unknown one fails there, named
/// by its id, and DLS loads (and gets its name and load mode) as usual. Once the list is
/// in, an unknown id is refused at once from it.
#[test]
fn a_plugin_is_looked_up_off_the_control_thread() {
    let s = session();
    s.offline_audio(None, 48_000).unwrap();
    assert!(s.inner.lock().plugins.list.is_empty(), "no scan yet");
    s.send(PluginCmd::SetPartPlugin { part: 0, id: "aumu nope nope".into(), state: None }).unwrap();
    assert_eq!(wait_playing(&s, 0), PluginStatus::Failed);
    let p = s.state().keyboard_parts[0].plugin.clone().unwrap();
    assert!(p.name == "aumu nope nope" && p.error.as_deref().is_some_and(|e| e.contains("no instrument Audio Unit")), "{p:?}");
    s.send(PluginCmd::SetPartPlugin { part: 1, id: DLS.into(), state: None }).unwrap();
    // `send` pumps, so a quick load may already be in: the id names it only while loading.
    let p = s.state().keyboard_parts[1].plugin.clone().unwrap();
    assert!(p.status != PluginStatus::Loading || p.name == DLS, "named by its id until looked up: {p:?}");
    assert_eq!(wait_playing(&s, 1), PluginStatus::Playing);
    assert_eq!(s.state().keyboard_parts[1].plugin.clone().unwrap().name, "DLSMusicDevice");
    {
        let ctl = s.inner.lock();
        let c = ctl.plugins.channels[crate::parts::CHANNEL[1] as usize].as_ref().unwrap();
        let info = c.info.as_ref().expect("looked up on the load thread");
        assert_eq!(c.mode, super::imp::load_mode(info), "the load thread chose the mode");
    }
    wait_scanned(&s);
    assert!(s.send(PluginCmd::SetPartPlugin { part: 2, id: "aumu nope nope".into(), state: None }).is_err(), "refused from the list");
}

/// Pump until Right 1's plugin has finished loading (the load runs on its own thread).
fn wait_playing(s: &Session, part: usize) -> PluginStatus {
    let t0 = Instant::now();
    loop {
        s.advance(1_000_000);
        let st = s.state().keyboard_parts[part].plugin.as_ref().map(|p| p.status);
        match st {
            Some(PluginStatus::Loading) if t0.elapsed() < Duration::from_secs(20) => std::thread::sleep(Duration::from_millis(5)),
            Some(x) => return x,
            None => panic!("no plugin on the part"),
        }
    }
}

#[test]
fn plugins_need_the_synth() {
    let s = session();
    assert!(!s.state().plugins.available);
    assert!(s.send(PluginCmd::SetPartPlugin { part: 0, id: DLS.into(), state: None }).is_err());
    assert!(s.state().keyboard_parts[0].plugin.is_none());
}

/// Right 1 on DLSMusicDevice, with no SoundFont at all: every sound here is the plugin's.
/// It plays, the meter moves, its CC7 is its level, its state saves and restores, and
/// clearing it gives the part back to the (here absent) SoundFont.
#[test]
fn a_keyboard_part_plays_an_audio_unit() {
    let s = session();
    s.offline_audio(None, 48_000).unwrap();
    // Checks for silence below: no reverb tail.
    s.fx_returns_off();
    assert!(s.state().plugins.available);
    wait_scanned(&s);
    assert!(s.send(PluginCmd::SetPartPlugin { part: 0, id: "aumu nope nope".into(), state: None }).is_err(), "not installed");
    s.send(PluginCmd::SetPartPlugin { part: 0, id: DLS.into(), state: None }).unwrap();
    // `send` pumps, so a quick load may already be playing.
    let p = s.state().keyboard_parts[0].plugin.clone().unwrap();
    assert!(matches!(p.status, PluginStatus::Loading | PluginStatus::Playing) && p.name == "DLSMusicDevice", "{p:?}");
    // Keys played while it loads: the part keeps its (here silent) SoundFont voice.
    assert_eq!(wait_playing(&s, 0), PluginStatus::Playing);
    let p = s.state().keyboard_parts[0].plugin.clone().unwrap();
    assert!(p.editor && !p.out_of_process, "Apple's units load in process");
    let _ = s.meters();
    let (l, r) = s.render(4800);
    assert_eq!(energy(&l, &r), 0.0, "nothing played yet");
    s.midi_in(Port::Keys, &[0x90, 72, 110]);
    let (l, r) = s.render(9600);
    let loud = energy(&l, &r);
    assert!(loud > 1e-3, "the plugin sounds: {loud}");
    let m = s.meters();
    let right1 = m.channels.iter().find(|c| c.channel == 1).unwrap().peak;
    assert!(right1 > 1e-3, "Right 1's meter moves: {right1}");
    assert!(m.master[0] > 0.0);
    // The fader is the part's CC7, applied by the host.
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 0 }).unwrap();
    s.render(256);
    let (l, r) = s.render(4800);
    assert_eq!(energy(&l, &r), 0.0, "CC7 0: silent");
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 100 }).unwrap();
    s.midi_in(Port::Keys, &[0x80, 72, 0]);
    // Its state saves and loads back into a fresh instance.
    s.send(PluginCmd::SavePartPluginState { part: 0 }).unwrap();
    let saved = wait_saved(&s, 0);
    assert!(saved.1.as_ref().is_some_and(|b| b.len() > 100), "a state blob");
    s.send(PluginCmd::SetPartPlugin { part: 0, id: saved.0, state: saved.1 }).unwrap();
    assert_eq!(wait_playing(&s, 0), PluginStatus::Playing, "restored with its state");
    s.midi_in(Port::Keys, &[0x90, 67, 110]);
    let (l, r) = s.render(4800);
    assert!(energy(&l, &r) > 1e-3);
    s.midi_in(Port::Keys, &[0x80, 67, 0]);
    // Back to the SoundFont (none here: silence), after the short fade.
    s.send(PluginCmd::ClearPartPlugin { part: 0 }).unwrap();
    assert!(s.state().keyboard_parts[0].plugin.is_none());
    s.render(960);
    s.midi_in(Port::Keys, &[0x90, 72, 110]);
    let (l, r) = s.render(4800);
    assert_eq!(energy(&l, &r), 0.0, "the part is back on its SoundFont voice");
}

/// With a SoundFont too: the part switches from its SoundFont voice to the plugin, and
/// only the plugin plays the notes after that (the SoundFont no longer gets note-ons).
#[test]
fn the_soundfont_voice_hands_over_to_the_plugin() {
    let s = session();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("soundfonts");
    let Some(sf2) = crate::library::sound_font_files(&dir).into_iter().map(|f| dir.join(f)).min_by_key(|p| p.metadata().map(|m| m.len()).unwrap_or(u64::MAX)) else {
        eprintln!("no SoundFont; skipping");
        return;
    };
    s.offline_audio(Some(&sf2), 48_000).unwrap();
    s.midi_in(Port::Keys, &[0x90, 72, 110]);
    let (l, r) = s.render(4800);
    assert!(energy(&l, &r) > 1e-4, "the SoundFont voice plays");
    s.send(PluginCmd::SetPartPlugin { part: 0, id: DLS.into(), state: None }).unwrap();
    assert_eq!(wait_playing(&s, 0), PluginStatus::Playing);
    s.midi_in(Port::Keys, &[0x80, 72, 0]);
    s.render(48_000); // the SoundFont note's release dies away
    // Now mute the plugin with its level: if the SoundFont still got note-ons, it would sound.
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 0 }).unwrap();
    s.render(256);
    let (l, r) = s.render(4800);
    let before = energy(&l, &r); // the SoundFont note's reverb tail, dying away
    s.midi_in(Port::Keys, &[0x90, 60, 110]);
    let (l, r) = s.render(4800);
    let after = energy(&l, &r);
    assert!(after <= before, "only the plugin plays the part's notes (the tail only decays): {after} after, {before} before");
}

/// Pump until the part's plugin voice has a state (it is read on a thread of its own).
fn wait_saved(s: &Session, part: usize) -> (String, Option<String>) {
    let t0 = Instant::now();
    loop {
        s.advance(1_000_000);
        let v = s.inner.lock().part_plugin_voice(part).unwrap();
        if v.1.is_some() || t0.elapsed() > Duration::from_secs(10) {
            return v;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// `savePartPluginState` reads the state off the control thread (#104 review item 5):
/// the command returns before the state is read, and a state read from an instance the part
/// no longer plays is not saved as the new one's.
#[test]
fn a_plugin_state_is_read_off_the_control_thread() {
    let s = session();
    s.offline_audio(None, 48_000).unwrap();
    s.send(PluginCmd::SetPartPlugin { part: 0, id: DLS.into(), state: None }).unwrap();
    assert_eq!(wait_playing(&s, 0), PluginStatus::Playing);
    s.send(PluginCmd::SavePartPluginState { part: 0 }).unwrap();
    // `send` settles an offline session with a pump, which may already have taken a quick
    // read's result (a small state, a fast read thread): pending, or landed at that pump.
    {
        let c = s.inner.lock();
        assert!(!c.plugins.state_reads.is_empty() || c.part_plugin_voice(0).unwrap().1.is_some(), "the read runs on a thread");
    }
    assert!(wait_saved(&s, 0).1.is_some_and(|b| b.len() > 100), "and lands at a pump");
    // A read of the old instance, then the part loads a new one: the read is dropped.
    s.send(PluginCmd::SetPartPlugin { part: 1, id: DLS.into(), state: None }).unwrap();
    assert_eq!(wait_playing(&s, 1), PluginStatus::Playing);
    s.send(PluginCmd::SavePartPluginState { part: 1 }).unwrap();
    s.send(PluginCmd::SetPartPlugin { part: 1, id: DLS.into(), state: None }).unwrap();
    assert_eq!(wait_playing(&s, 1), PluginStatus::Playing);
    let t0 = Instant::now();
    while !s.inner.lock().plugins.state_reads.is_empty() && t0.elapsed() < Duration::from_secs(10) {
        s.advance(1_000_000);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(s.inner.lock().part_plugin_voice(1).unwrap().1, None, "the old instance's state is not the new one's");
    // No plugin: an error at once, nothing read.
    assert!(s.send(PluginCmd::SavePartPluginState { part: 2 }).is_err());
}

fn saved_id(s: &Session, part: usize) -> Option<String> {
    s.inner.lock().saved_parts().parts[part].as_ref().map(|v| v.id.clone())
}

/// A re-pick that fails (here a state blob the plugin rejects) leaves the working plugin
/// playing and saved; a failed pick with nothing playing keeps its choice as `failed`
/// (saved, retryable); only `clearPartPlugin` forgets it (#105 review B2, nonblocking 1).
#[test]
fn a_failed_load_never_loses_the_parts_plugin() {
    let s = session();
    s.offline_audio(None, 48_000).unwrap();
    s.send(PluginCmd::SetPartPlugin { part: 0, id: DLS.into(), state: None }).unwrap();
    assert_eq!(wait_playing(&s, 0), PluginStatus::Playing);
    // "junk" in base64: not a property list, so restoring it fails.
    s.send(PluginCmd::SetPartPlugin { part: 0, id: DLS.into(), state: Some("anVuaw==".into()) }).unwrap();
    assert_eq!(wait_playing(&s, 0), PluginStatus::Playing, "the working plugin keeps the part");
    assert!(s.state().message.as_ref().is_some_and(|m| m.error && m.text.contains("keeps playing")));
    s.midi_in(Port::Keys, &[0x90, 72, 110]);
    let (l, r) = s.render(4800);
    assert!(energy(&l, &r) > 1e-3, "and it still sounds");
    s.midi_in(Port::Keys, &[0x80, 72, 0]);
    assert_eq!(saved_id(&s, 0).as_deref(), Some(DLS));
    // Right 2: a failing first pick is kept as failed, saved, and can be picked again.
    s.send(PluginCmd::SetPartPlugin { part: 1, id: DLS.into(), state: Some("anVuaw==".into()) }).unwrap();
    assert_eq!(wait_playing(&s, 1), PluginStatus::Failed);
    let p = s.state().keyboard_parts[1].plugin.clone().unwrap();
    assert!(p.error.is_some() && p.name == "DLSMusicDevice");
    assert_eq!(saved_id(&s, 1).as_deref(), Some(DLS), "a failed choice stays saved");
    // Picking it again (the app's picker sends no state) retries it with the state it
    // kept, so a restore that timed out or a reinstalled plugin comes back as saved.
    s.send(PluginCmd::SetPartPlugin { part: 1, id: DLS.into(), state: None }).unwrap();
    assert_eq!(wait_playing(&s, 1), PluginStatus::Failed, "retried with its kept state");
    let kept = s.inner.lock().saved_parts().parts[1].as_ref().and_then(|v| v.state.clone());
    assert_eq!(kept.as_deref(), Some(&b"junk"[..]), "the retry did not throw the saved state away");
    s.send(PluginCmd::ClearPartPlugin { part: 1 }).unwrap();
    assert_eq!(saved_id(&s, 1), None, "cleared: forgotten");
    // After the SoundFont, a pick starts it fresh.
    s.send(PluginCmd::SetPartPlugin { part: 1, id: DLS.into(), state: None }).unwrap();
    assert_eq!(wait_playing(&s, 1), PluginStatus::Playing, "fresh");
}

/// A restore of a plugin that isn't installed shows its id as the name, not a blank.
#[test]
fn a_missing_plugin_is_named_by_its_id() {
    let s = session();
    s.offline_audio(None, 48_000).unwrap();
    let mut saved = super::Saved::default();
    saved.parts[2] = Some(super::PluginVoice { id: "aumu Nope Gone".into(), state: None, preset: None, sound: None });
    s.inner.lock().restore_saved(saved);
    assert_eq!(wait_playing(&s, 2), PluginStatus::Failed);
    let p = s.state().keyboard_parts[2].plugin.clone().unwrap();
    assert_eq!((p.status, p.name.as_str()), (PluginStatus::Failed, "aumu Nope Gone"));
}

/// The start-up restore of a plugin that isn't installed (uninstalled, licence missing,
/// an AUv3 whose app moved) keeps the part's saved choice and state as `failed`, so it
/// survives the next save; and a restore never falls back to loading in process
/// (#105 review B2, B3).
#[test]
fn a_restore_keeps_a_missing_plugin_and_never_falls_back_in_process() {
    let s = session();
    s.offline_audio(None, 48_000).unwrap();
    let mut saved = super::Saved::default();
    saved.parts[0] = Some(super::PluginVoice { id: "aumu Nope Gone".into(), state: Some(vec![1, 2, 3]), preset: None, sound: None });
    saved.parts[3] = Some(super::PluginVoice { id: DLS.into(), state: None, preset: None, sound: None });
    {
        let mut ctl = s.inner.lock();
        ctl.restore_saved(saved);
        let ch = crate::parts::CHANNEL[3] as usize;
        assert!(!ctl.plugins.channels[ch].as_ref().unwrap().allow_fallback, "no in-process fallback at start-up");
    }
    assert_eq!(wait_playing(&s, 3), PluginStatus::Playing);
    assert_eq!(wait_playing(&s, 0), PluginStatus::Failed);
    let p = s.state().keyboard_parts[0].plugin.clone().unwrap();
    assert_eq!((p.status, p.id.as_str()), (PluginStatus::Failed, "aumu Nope Gone"));
    let kept = s.inner.lock().saved_parts();
    assert_eq!(kept.parts[0].as_ref().map(|v| (v.id.as_str(), v.state.clone())), Some(("aumu Nope Gone", Some(vec![1, 2, 3]))), "the id and state survive");
    assert_eq!(kept.parts[3].as_ref().map(|v| v.id.as_str()), Some(DLS));
}

/// Goertzel power at `hz` (48 kHz).
fn power_at(x: &[f32], hz: f64) -> f64 {
    let w = 2.0 * std::f64::consts::PI * hz / 48_000.0;
    let (mut s1, mut s2) = (0.0f64, 0.0f64);
    for &v in x {
        let s0 = v as f64 + 2.0 * w.cos() * s1 - s2;
        (s2, s1) = (s1, s0);
    }
    s1 * s1 + s2 * s2 - 2.0 * w.cos() * s1 * s2
}

/// Play with time running: engine deadlines (Echo, the arpeggio) and audio together.
fn play(s: &Session, ms: usize) -> Vec<f32> {
    let mut out = Vec::new();
    for _ in 0..ms / 10 {
        s.advance(10_000_000);
        out.extend(s.render(480).0);
    }
    out
}

/// Keyboard Harmony's added notes (input thread) and the arpeggio's (engine thread) on a
/// keyboard part reach that part's plugin, as its keys do (#100 with #91).
#[test]
fn harmony_and_arpeggio_notes_reach_the_parts_plugin() {
    use crate::api::HarmonyArpCmd;
    let s = session();
    s.finish_indexing();
    s.offline_audio(None, 48_000).unwrap();
    s.send(PluginCmd::SetPartPlugin { part: 0, id: DLS.into(), state: None }).unwrap();
    assert_eq!(wait_playing(&s, 0), PluginStatus::Playing);
    // Harmony: C5 over a C chord adds G4 below it, on Right 1 (the plugin).
    let duet = crate::live::type_index(crate::harmony::HarmonyType::StandardDuet1);
    s.send(HarmonyArpCmd::SetHarmonyType { index: duet }).unwrap();
    let g4 = |on: bool| {
        s.send(HarmonyArpCmd::SetHarmonyArpOn { on }).unwrap();
        for k in [48, 52, 43] {
            s.midi_in(Port::Keys, &[0x90, k, 90]);
        }
        s.midi_in(Port::Keys, &[0x90, 72, 100]);
        let x = play(&s, 300);
        for k in [72, 48, 52, 43] {
            s.midi_in(Port::Keys, &[0x80, k, 0]);
        }
        play(&s, 1500);
        power_at(&x[2400..], 392.0) / power_at(&x[2400..], 523.25)
    };
    let (off, on) = (g4(false), g4(true));
    assert!(on > 20.0 * off, "the harmony note sounds on the plugin: G4/C5 {on} with Harmony, {off} without");
    // Arpeggio with Hold: the pattern plays on the plugin after the keys are released.
    let climb = crate::arp::library::PATTERNS.iter().position(|p| p.name == "Climb 16").unwrap() as u8;
    s.send(HarmonyArpCmd::SetArpPattern { index: climb }).unwrap();
    s.send(HarmonyArpCmd::SetArpHold { on: true }).unwrap();
    let tail = |on: bool| {
        s.send(HarmonyArpCmd::SetHarmonyArpOn { on }).unwrap();
        s.midi_in(Port::Keys, &[0x90, 72, 100]);
        play(&s, 300);
        s.midi_in(Port::Keys, &[0x80, 72, 0]);
        let x = play(&s, 3000);
        energy(&x[96_000..], &[])
    };
    let (off, on) = (tail(false), tail(true));
    assert!(on > 1e-3 && on > 100.0 * off, "the held arpeggio sounds on the plugin: {on}, a released key {off}");
    s.send(HarmonyArpCmd::SetHarmonyArpOn { on: false }).unwrap();
}

/// The band's channel setups, now one per section routing (#64), go to the style parts'
/// channels (9-16) only: section changes never reach a keyboard part's plugin, which
/// plays its key as before afterwards.
#[test]
fn section_setups_never_reach_a_keyboard_parts_plugin() {
    use crate::api::TransportCmd;
    let s = session();
    s.finish_indexing();
    s.offline_audio(None, 48_000).unwrap();
    // The plugin's own sound only: the effect bus's chorus (Right 1's default send, #204)
    // moves the C5 measure from one note to the next.
    s.fx_returns_off();
    s.send(PluginCmd::SetPartPlugin { part: 0, id: DLS.into(), state: None }).unwrap();
    assert_eq!(wait_playing(&s, 0), PluginStatus::Playing);
    // A C5 on Right 1: its level and how much of it is C5.
    let key = |s: &Session| {
        s.midi_in(Port::Keys, &[0x90, 72, 100]);
        let x = play(s, 300);
        s.midi_in(Port::Keys, &[0x80, 72, 0]);
        play(s, 2000);
        let e = energy(&x[4800..], &[]);
        (e, power_at(&x[4800..], 523.25) / e)
    };
    let before = key(&s);
    // The band (no SoundFont: silent) through every Main, with a left-hand chord.
    s.send(TransportCmd::StartStop).unwrap();
    for k in [48, 52, 43] {
        s.midi_in(Port::Keys, &[0x90, k, 90]);
    }
    // Only the C5's reverb tail, dying away, may sound.
    let mut last = f64::MAX;
    for index in [1, 2, 3, 0] {
        s.send(TransportCmd::Main { index }).unwrap();
        let e = energy(&play(&s, 2000), &[]);
        assert!(e <= last && e < before.0 * 1e-4, "Main {index}: nothing reaches Right 1's plugin ({e} after {last})");
        last = e;
    }
    assert!(s.state().transport.running);
    let after = key(&s);
    assert!((after.0 / before.0 - 1.0).abs() < 0.05 && (after.1 / before.1 - 1.0).abs() < 0.05, "the key plays as before: {before:?} then {after:?}");
}

/// A plugin patch is a keyboard part's own patch (#109): picking it from the library loads
/// its plugin with the patch's state (#91's `setPartPlugin` path) and the part shows the
/// patch; leaving the patch (a GM voice, the patch deleted) takes the plugin away, and a
/// plugin picked on the Plugins tab ends the patch instead.
#[test]
fn a_plugin_patch_plays_on_a_keyboard_part() {
    use crate::api::{PatchFields, SoundLibraryCmd};
    use crate::patches::PatchSource;
    let s = session();
    s.offline_audio(None, 48_000).unwrap();
    // A state to store in the patch: DLS's own, read back from a part.
    s.send(PluginCmd::SetPartPlugin { part: 1, id: DLS.into(), state: None }).unwrap();
    assert_eq!(wait_playing(&s, 1), PluginStatus::Playing);
    s.send(PluginCmd::SavePartPluginState { part: 1 }).unwrap();
    let state = wait_saved(&s, 1).1.unwrap();
    s.send(PluginCmd::ClearPartPlugin { part: 1 }).unwrap();
    let fields = PatchFields {
        name: "DLS Keys".into(),
        category: Default::default(),
        tags: vec![],
        favourite: false,
        source: PatchSource::plugin(DLS, state.clone()),
    };
    s.send(SoundLibraryCmd::CreatePatch { patch: fields }).unwrap();
    let id = s.state().sound_library.last_added.clone().unwrap();
    assert!(s.state().sound_library.patches.iter().any(|p| p.patch.id == id && p.available));

    // The part keeps its own level: a sound carries no mix.
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 64 }).unwrap();
    s.send(SoundLibraryCmd::SetPartPatch { part: 0, id: Some(id.clone()) }).unwrap();
    let r1 = |s: &Session| s.state().keyboard_parts[0].clone();
    assert_eq!((r1(&s).patch.as_deref(), r1(&s).voice_name.as_str(), r1(&s).volume), (Some(id.as_str()), "DLS Keys", 64));
    assert_eq!(wait_playing(&s, 0), PluginStatus::Playing);
    assert_eq!(s.inner.lock().part_plugin_voice(0), Some((DLS.to_string(), Some(state.clone()))), "the patch's state");
    s.midi_in(Port::Keys, &[0x90, 72, 110]);
    let (l, r) = s.render(9600);
    assert!(energy(&l, &r) > 1e-3, "the plugin patch sounds");
    s.midi_in(Port::Keys, &[0x80, 72, 0]);
    // Another edit of the library does not reload it.
    s.send(SoundLibraryCmd::SetPatchFavourite { id: id.clone(), favourite: true }).unwrap();
    assert_eq!(r1(&s).plugin.map(|p| p.status), Some(PluginStatus::Playing));

    // A GM voice ends the patch and its plugin.
    s.send(PartsCmd::SetPartVoice { part: 0, program: 0 }).unwrap();
    assert!(r1(&s).patch.is_none() && r1(&s).plugin.is_none());

    // A plugin picked on the Plugins tab ends the patch, and is not taken away after.
    s.send(SoundLibraryCmd::SetPartPatch { part: 0, id: Some(id.clone()) }).unwrap();
    assert_eq!(wait_playing(&s, 0), PluginStatus::Playing);
    s.send(PluginCmd::SetPartPlugin { part: 0, id: DLS.into(), state: None }).unwrap();
    assert!(r1(&s).patch.is_none());
    s.send(SoundLibraryCmd::SetPatchFavourite { id: id.clone(), favourite: false }).unwrap();
    assert!(r1(&s).plugin.is_some(), "the Plugins tab's plugin stays");
    s.send(PluginCmd::ClearPartPlugin { part: 0 }).unwrap();

    // Deleting the patch a part plays takes its plugin away.
    s.send(SoundLibraryCmd::SetPartPatch { part: 2, id: Some(id.clone()) }).unwrap();
    assert!(s.state().keyboard_parts[2].plugin.is_some());
    s.send(SoundLibraryCmd::DeletePatch { id }).unwrap();
    assert!(s.state().keyboard_parts[2].patch.is_none() && s.state().keyboard_parts[2].plugin.is_none());
}

/// `savePartAsPatch` on a part playing a plugin (#109) saves the plugin, with its state
/// read afresh off the control thread (it lands in the patch at a later pump), not the
/// part's GM program; a part playing its own plugin patch saves a copy of that patch.
#[test]
fn saving_a_part_saves_its_plugin_and_its_state_now() {
    use crate::api::SoundLibraryCmd;
    use crate::patches::PatchSource;
    let s = session();
    s.offline_audio(None, 48_000).unwrap();
    s.send(PluginCmd::SetPartPlugin { part: 0, id: DLS.into(), state: None }).unwrap();
    assert_eq!(wait_playing(&s, 0), PluginStatus::Playing);
    assert_eq!(s.inner.lock().part_plugin_voice(0).unwrap().1, None, "no state saved yet");
    s.send(SoundLibraryCmd::SavePartAsPatch { part: 0, name: None }).unwrap();
    let id = s.state().sound_library.last_added.clone().unwrap();
    let source = |s: &Session| s.state().sound_library.patches.iter().find(|p| p.patch.id == id).unwrap().patch.source.clone();
    let state = |src: PatchSource| match src {
        PatchSource::Plugin { component_id, state, .. } => (component_id, state),
        other => panic!("not a plugin patch: {other:?}"),
    };
    let (component, _) = state(source(&s));
    assert_eq!(component, DLS);
    assert_eq!(s.state().sound_library.patches.last().unwrap().patch.name, "DLSMusicDevice");
    // The fresh read lands in the patch.
    let t0 = Instant::now();
    while state(source(&s)).1.is_empty() && t0.elapsed() < Duration::from_secs(10) {
        s.advance(1_000_000);
        std::thread::sleep(Duration::from_millis(2));
    }
    let saved = state(source(&s)).1;
    assert!(saved.len() > 100, "the plugin's state as it is now");
    assert_eq!(s.inner.lock().part_plugin_voice(0).unwrap().1.as_deref(), Some(saved.as_str()));

    // A part playing its own plugin patch: a copy of it, under the name given.
    s.send(SoundLibraryCmd::SetPartPatch { part: 1, id: Some(id.clone()) }).unwrap();
    assert_eq!(wait_playing(&s, 1), PluginStatus::Playing);
    s.send(SoundLibraryCmd::SavePartAsPatch { part: 1, name: Some("DLS Copy".into()) }).unwrap();
    let st = s.state();
    let copy = &st.sound_library.patches.last().unwrap().patch;
    assert_ne!(copy.id, id);
    assert_eq!(copy.name, "DLS Copy");
    assert!(matches!(&copy.source, PatchSource::Plugin { component_id, .. } if component_id == DLS));
}

/// A plugin patch auditions like a SoundFont one (#109): with the band stopped, its plugin
/// loads on channel 16 (the audition channel), plays the phrase through the rack, and goes
/// when the audition ends; nothing is left on the channel.
#[test]
fn a_plugin_patch_auditions_on_channel_16() {
    use crate::api::{PatchFields, SoundLibraryCmd, TransportCmd};
    use crate::patches::PatchSource;
    let s = session();
    s.offline_audio(None, 48_000).unwrap();
    let fields = PatchFields {
        name: "DLS Keys".into(),
        category: Default::default(),
        tags: vec![],
        favourite: false,
        source: PatchSource::plugin(DLS, String::new()),
    };
    s.send(SoundLibraryCmd::CreatePatch { patch: fields }).unwrap();
    let id = s.state().sound_library.last_added.clone().unwrap();
    let ch16 = |s: &Session| s.inner.lock().channel_plugin(15).map(|p| p.status);
    s.send(SoundLibraryCmd::AuditionPatch { id: id.clone() }).unwrap();
    assert_eq!(s.state().sound_library.auditioning.as_deref(), Some(id.as_str()));
    assert!(matches!(ch16(&s), Some(PluginStatus::Loading | PluginStatus::Playing)), "`send` pumps: it may be in already");
    let t0 = Instant::now();
    while ch16(&s) == Some(PluginStatus::Loading) && t0.elapsed() < Duration::from_secs(20) {
        s.advance(1_000_000);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(ch16(&s), Some(PluginStatus::Playing));
    s.advance(1_000_000);
    let (l, r) = s.render(9600);
    assert!(energy(&l, &r) > 1e-3, "the audition sounds through the plugin");
    let m = s.meters();
    assert!(m.channels.iter().find(|c| c.channel == 16).unwrap().peak > 1e-3, "on channel 16");
    // No keyboard part got the plugin.
    assert!(s.state().keyboard_parts.iter().all(|p| p.plugin.is_none()));
    s.advance(4000 * 1_000_000);
    assert_eq!(s.state().sound_library.auditioning, None, "it ends by itself");
    assert_eq!(ch16(&s), None, "and its plugin goes");
    s.render(4800);
    // Stopping it early, and never while the band plays.
    s.send(SoundLibraryCmd::AuditionPatch { id: id.clone() }).unwrap();
    s.send(SoundLibraryCmd::StopPatchAudition).unwrap();
    assert_eq!((s.state().sound_library.auditioning.clone(), ch16(&s)), (None, None));
    s.send(TransportCmd::StartStop).unwrap();
    s.advance(10_000_000);
    assert!(s.send(SoundLibraryCmd::AuditionPatch { id }).is_err(), "not while the band plays");
    assert_eq!(ch16(&s), None);
}

/// The live overrun readout counts the last 10 one-second readings of the running total.
#[test]
fn recent_overruns_cover_the_last_ten_seconds() {
    use super::imp::{OverrunWindow, OVERRUN_WINDOW_SECS};
    let mut w = OverrunWindow::starting_at(5);
    assert_eq!(w.count(), 0, "overruns before this instance's window don't count");
    w.tick(8);
    assert_eq!(w.count(), 3);
    w.tick(8);
    w.tick(9);
    assert_eq!(w.count(), 4);
    // Ten quiet seconds later the readout is back to 0.
    for _ in 0..OVERRUN_WINDOW_SECS {
        w.tick(9);
    }
    assert_eq!(w.count(), 0);
    w.tick(9 + u64::from(u32::MAX) + 10);
    assert_eq!(w.count(), u32::MAX, "saturates");
}

/// Where a plugin loads: the player's override first, then Apple's units and AUv3s as
/// macOS decides, and third-party AUv2s in their own process.
#[test]
fn the_in_process_override_picks_the_load_mode() {
    use crate::plugin::{LoadMode, PluginFormat, PluginId, PluginInfo};
    let info = |id: &str, format: PluginFormat, can_load_in_process: bool, in_process: bool| PluginInfo {
        id: PluginId::parse(id).unwrap(),
        name: "x".into(),
        manufacturer: String::new(),
        version: 1,
        format,
        requires_async: false,
        can_load_in_process,
        sandbox_safe: true,
        last_load: None,
        in_process,
        factory_presets: None,
        user_presets: Vec::new(),
    };
    let mode = super::imp::load_mode;
    assert_eq!(mode(&info("aumu Xf2X XFER", PluginFormat::Au2, false, false)), LoadMode::OutOfProcess);
    assert_eq!(mode(&info("aumu Xf2X XFER", PluginFormat::Au2, false, true)), LoadMode::InProcess);
    assert_eq!(mode(&info(DLS, PluginFormat::Au2, false, false)), LoadMode::Auto);
    assert_eq!(mode(&info(DLS, PluginFormat::Au2, false, true)), LoadMode::InProcess);
    assert_eq!(mode(&info("aumu Ab3X ACME", PluginFormat::Au3, false, false)), LoadMode::Auto);
    assert_eq!(mode(&info("aumu Ab3X ACME", PluginFormat::Au3, false, true)), LoadMode::Auto, "an AUv3 that can't run in process");
    assert_eq!(mode(&info("aumu Ab3X ACME", PluginFormat::Au3, true, true)), LoadMode::InProcess);
}

/// `setPluginInProcess` shows in the plugin list; an unknown id is an error.
#[test]
fn set_plugin_in_process_shows_in_the_list() {
    let s = session();
    s.offline_audio(None, 48_000).unwrap();
    assert!(s.send(PluginCmd::SetPluginInProcess { id: "aumu nope nope".into(), in_process: true }).is_err());
    // An offline session has no plugin list until a scan runs.
    s.send(PluginCmd::RescanPlugins).unwrap();
    let t0 = Instant::now();
    while !s.state().plugins.list.iter().any(|p| p.id == DLS) && t0.elapsed() < Duration::from_secs(20) {
        s.advance(1_000_000);
        std::thread::sleep(Duration::from_millis(5));
    }
    let dls = |s: &Session| s.state().plugins.list.iter().find(|p| p.id == DLS).cloned().unwrap();
    assert!(dls(&s).can_run_in_process);
    let was = dls(&s).in_process;
    s.send(PluginCmd::SetPluginInProcess { id: DLS.into(), in_process: !was }).unwrap();
    assert_eq!(dls(&s).in_process, !was);
    // Put the player's cache back as it was.
    s.send(PluginCmd::SetPluginInProcess { id: DLS.into(), in_process: was }).unwrap();
    assert_eq!(dls(&s).in_process, was);
}

/// #176: a plugin preloaded for a Registration bank (the warm pool) loaded in the old mode
/// before its "run in process" override changed. The override drops it and preloads it
/// again in the new mode, so a button press never hands a part the old one.
#[test]
fn the_in_process_override_refills_the_warm_pool() {
    use crate::session::PluginVoice;
    let s = session();
    s.offline_audio(None, 48_000).unwrap();
    wait_scanned(&s);
    let info = || s.inner.lock().plugins.list.iter().find(|p| p.id.to_string() == DLS).cloned().unwrap();
    let warm = |s: &Session, n: usize| {
        let t0 = Instant::now();
        while s.inner.lock().warm_ready() != n && t0.elapsed() < Duration::from_secs(20) {
            s.advance(1_000_000);
            std::thread::sleep(Duration::from_millis(5));
        }
        s.inner.lock().plugins.warm.entries.iter().map(|w| w.mode).collect::<Vec<_>>()
    };
    s.inner.lock().warm_plugins(vec![PluginVoice { id: DLS.into(), state: None, preset: None, sound: None }]);
    let was = info().in_process;
    let before = warm(&s, 1);
    s.send(PluginCmd::SetPluginInProcess { id: DLS.into(), in_process: !was }).unwrap();
    let now = super::imp::load_mode(&info());
    let after = warm(&s, 1);
    let ready = s.inner.lock().warm_ready();
    // Put the player's cache back as it was.
    s.send(PluginCmd::SetPluginInProcess { id: DLS.into(), in_process: was }).unwrap();
    assert_ne!(before, [now], "the override changes DLS's load mode");
    assert_eq!(after, [now], "the pool holds DLS loaded in the new mode");
    assert_eq!(ready, 1, "and it is ready again");
    assert_eq!(warm(&s, 1), [super::imp::load_mode(&info())], "switched back: the old mode again");
}

/// `reloadPartPlugin` loads a failed (or stopped) plugin again with its kept state, for the
/// selected part when no part is given; the Launchkey's reload button lights while the
/// selected part's plugin needs it. A part without a plugin, or one playing, is refused.
#[test]
fn reload_part_plugin_retries_a_failed_plugin() {
    use crate::api::PartsCmd;
    let s = session();
    s.offline_audio(None, 48_000).unwrap();
    let fault = |s: &Session| s.inner.lock().selected_plugin_fault();
    assert!(s.send(PluginCmd::ReloadPartPlugin { part: None }).is_err(), "Right 1 plays its SoundFont");
    // Right 2: a state the plugin rejects, so the load fails.
    s.send(PluginCmd::SetPartPlugin { part: 1, id: DLS.into(), state: Some("anVuaw==".into()) }).unwrap();
    assert_eq!(wait_playing(&s, 1), PluginStatus::Failed);
    assert!(!fault(&s), "Right 1 is selected");
    s.send(PartsCmd::SelectPart { part: 1 }).unwrap();
    assert!(fault(&s), "the button lights for the selected part");
    s.send(PluginCmd::ReloadPartPlugin { part: None }).unwrap();
    // `send` pumps, so the reload may have failed again already.
    assert!(matches!(s.state().keyboard_parts[1].plugin.as_ref().unwrap().status, PluginStatus::Loading | PluginStatus::Failed));
    assert_eq!(wait_playing(&s, 1), PluginStatus::Failed, "reloaded with its kept state");
    let kept = s.inner.lock().saved_parts().parts[1].as_ref().and_then(|v| v.state.clone());
    assert_eq!(kept.as_deref(), Some(&b"junk"[..]));
    // Right 1 plays DLS: nothing to reload.
    s.send(PluginCmd::SetPartPlugin { part: 0, id: DLS.into(), state: None }).unwrap();
    assert_eq!(wait_playing(&s, 0), PluginStatus::Playing);
    assert!(s.send(PluginCmd::ReloadPartPlugin { part: Some(0) }).is_err());
    s.send(PartsCmd::SelectPart { part: 0 }).unwrap();
    assert!(!fault(&s));
}

/// The Sound Browser (#117): a SoundFont sound assigned to a part that plays a plugin
/// picked for it directly ends that plugin, and it is no longer saved to come back at
/// the next start. A preset from the synth's own font in bank 0 (the part's GM voice),
/// one from another font and a saved SoundFont sound all do (#171 review blocker).
#[test]
fn a_soundfont_sound_from_the_browser_ends_a_picked_plugin() {
    use crate::api::{SoundLibraryCmd, SoundsCmd};
    let p = testing::style_path();
    let data = std::env::temp_dir().join(format!("yahaha-browser-plugin-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data);
    let sf = data.join("sf");
    std::fs::create_dir_all(&sf).unwrap();
    let tiny = crate::patches::sf2::tiny_sound_font;
    std::fs::write(sf.join("A.sf2"), tiny(&[(0, 0, "Piano"), (0, 33, "Finger Bass"), (128, 0, "Standard Kit")])).unwrap();
    std::fs::write(sf.join("B.sf2"), tiny(&[(0, 88, "Warm Pad")])).unwrap();
    let opts = Options { paths: vec![p], data_dir: Some(data.clone()), sound_font_dir: Some(sf.clone()), ..Options::default() };
    let s = Session::offline(opts).unwrap();
    s.offline_audio(Some(&sf.join("A.sf2")), 48_000).unwrap();
    wait_scanned(&s);
    let au = format!("au:{DLS}");
    let pick = |part: u8| {
        s.send(SoundsCmd::AssignSound { part, id: au.clone() }).unwrap();
        assert_eq!(wait_playing(&s, part as usize), PluginStatus::Playing);
        assert_eq!(saved_id(&s, part as usize).as_deref(), Some(DLS));
    };
    let ended = |part: usize, what: &str| {
        assert!(s.state().keyboard_parts[part].plugin.is_none(), "{what}: the plugin ended");
        assert_eq!(saved_id(&s, part), None, "{what}: and is not saved to come back");
    };
    // The synth's own font, bank 0: the part's GM voice.
    pick(0);
    s.send(SoundsCmd::AssignSound { part: 0, id: "sf:A.sf2:0:33".into() }).unwrap();
    let r1 = s.state().keyboard_parts[0].clone();
    assert_eq!((r1.program, r1.patch), (33, None), "the synth's own preset is the part's GM voice");
    ended(0, "own font");
    // Another font's preset, and a saved SoundFont sound.
    pick(1);
    s.send(SoundsCmd::AssignSound { part: 1, id: "sf:B.sf2:0:88".into() }).unwrap();
    ended(1, "another font");
    pick(2);
    s.send(SoundLibraryCmd::AddPresetAsPatch { file: "A.sf2".into(), bank: 0, program: 0, name: Some("Mine".into()) }).unwrap();
    let mine = s.state().sound_library.last_added.clone().unwrap();
    s.send(SoundsCmd::AssignSound { part: 2, id: format!("saved:{mine}") }).unwrap();
    ended(2, "saved sound");
    let _ = std::fs::remove_dir_all(&data);
}

/// #179: selecting a GM voice replaces the part's voice, as on the Genos: Voice −/+ (the
/// Launchkey's and the terminal UI's `stepVoice`), a voice picked by number
/// (`setPartVoice`) and a One Touch Setting that gives the part a voice each end a plugin
/// picked for the part, and it is no longer saved to come back.
#[test]
fn a_gm_voice_selection_ends_a_picked_plugin() {
    use crate::api::OtsCmd;
    let s = session();
    s.offline_audio(None, 48_000).unwrap();
    wait_scanned(&s);
    let pick = |part: usize| {
        s.send(PluginCmd::SetPartPlugin { part: part as u8, id: DLS.into(), state: None }).unwrap();
        assert_eq!(wait_playing(&s, part), PluginStatus::Playing);
        assert_eq!(saved_id(&s, part).as_deref(), Some(DLS));
    };
    let ended = |part: usize, what: &str| {
        s.advance(1_000_000);
        assert!(s.state().keyboard_parts[part].plugin.is_none(), "{what}: the plugin ended");
        assert_eq!(saved_id(&s, part), None, "{what}: and is not saved to come back");
    };
    // Voice −/+ on the selected part.
    pick(1);
    s.send(PartsCmd::SelectPart { part: 1 }).unwrap();
    let was = s.state().keyboard_parts[1].program;
    s.send(PartsCmd::StepVoice { delta: 1 }).unwrap();
    assert_ne!(s.state().keyboard_parts[1].program, was, "Voice + steps the voice");
    ended(1, "Voice +");
    // A voice picked by number.
    pick(2);
    s.send(PartsCmd::SetPartVoice { part: 2, program: 40 }).unwrap();
    ended(2, "setPartVoice");
    // A One Touch Setting that gives the part a voice.
    let ots = s.inner.lock().info.ots.iter().enumerate().find_map(|(i, o)| o.parts.iter().position(|q| q.voice.is_some_and(|v| v.0 < 126)).map(|p| (i, p)));
    let Some((i, p)) = ots else { panic!("SlowWalker has a One Touch Setting with a voice") };
    pick(p);
    s.send(OtsCmd::RecallOts { index: i as u8 }).unwrap();
    ended(p, "One Touch Setting");
}

/// A plugin part feeds the shared effect bus through its sends, as a SoundFont part does
/// (#204): fully sent to the reverb, its note rings on after the release; at return 0 the
/// release is the plugin's own.
#[test]
fn a_plugin_part_feeds_the_effect_bus() {
    let tail = |returns: bool| {
        let s = session();
        s.offline_audio(None, 48_000).unwrap();
        if !returns {
            s.fx_returns_off();
        }
        s.send(PluginCmd::SetPartPlugin { part: 0, id: DLS.into(), state: None }).unwrap();
        assert_eq!(wait_playing(&s, 0), PluginStatus::Playing);
        s.midi_in(Port::Keys, &[0xB0, 91, 127]);
        s.midi_in(Port::Keys, &[0x90, 72, 110]);
        let (l, r) = s.render(9600);
        assert!(energy(&l, &r) > 1e-3, "the plugin sounds");
        s.midi_in(Port::Keys, &[0x80, 72, 0]);
        s.render(24_000);
        let (l, r) = s.render(24_000);
        energy(&l, &r)
    };
    let (wet, dry) = (tail(true), tail(false));
    assert!(wet > dry * 4.0 + 1e-6, "the reverb rings on: {wet} vs {dry}");
}

/// AU presets: an `.aupreset` in the plugin's preset folder is a sound of its own in the
/// catalog, under its plugin; picking it gives the part an instance loaded with it, while
/// another part plays the same plugin plainly. "Save as preset" writes the part's plugin
/// as a new `.aupreset`, files it under the category picked and makes it the part's
/// preset; the part's saved voice keeps the preset, and an old plugin-parts.json (no
/// preset) still reads.
#[test]
fn a_plugin_preset_is_a_sound_of_its_own() {
    use crate::api::{PatchCategory, SoundsCmd};
    use crate::plugin::{presets, LoadConfig, PluginHost, PluginId};
    let p = testing::style_path();
    let data = std::env::temp_dir().join(format!("yahaha-au-presets-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data);
    let root = data.join("Presets");
    // A preset file for DLS, from a DLS instance's own state.
    let host = PluginHost::with_preset_roots(Some(data.join("plugins.json")), vec![root.clone()]);
    let info = host.info(&PluginId::DLS).unwrap();
    let state = host.load(&PluginId::DLS, LoadConfig::default()).unwrap().get_state().unwrap();
    let file = presets::write_user_preset(&root, &info, "Warm Strings", &state, false).unwrap();
    host.rescan().unwrap();

    let opts = Options { paths: vec![p], data_dir: Some(data.join("data")), ..Options::default() };
    let s = Session::offline(opts).unwrap();
    s.offline_audio(None, 48_000).unwrap();
    s.inner.lock().plugins.host = Some(host);
    wait_scanned(&s);
    let key = format!("u:{}", file.path.display());
    let id = format!("au:{DLS}#{key}");
    let cat = s.sound_catalog();
    let e = cat.entries.iter().find(|e| e.id == id).expect("the preset is in the catalog");
    assert_eq!((e.name.as_str(), e.parent.as_deref(), e.category), ("Warm Strings", Some(format!("au:{DLS}").as_str()), PatchCategory::Strings));

    s.send(SoundsCmd::AssignSound { part: 0, id: id.clone() }).unwrap();
    s.send(SoundsCmd::AssignSound { part: 1, id: format!("au:{DLS}") }).unwrap();
    assert_eq!(wait_playing(&s, 0), PluginStatus::Playing);
    assert_eq!(wait_playing(&s, 1), PluginStatus::Playing);
    let (p0, p1) = (s.state().keyboard_parts[0].plugin.clone().unwrap(), s.state().keyboard_parts[1].plugin.clone().unwrap());
    assert_eq!((p0.preset.as_deref(), p0.preset_key.as_deref()), (Some("Warm Strings"), Some(key.as_str())));
    assert_eq!(p1.preset, None);
    assert!(s.inner.lock().saved_parts().parts[0].as_ref().unwrap().state.is_some(), "the file's settings are the part's state");
    assert!(s.send(SoundsCmd::AssignSound { part: 2, id: format!("au:{DLS}#f:999") }).is_err(), "no such preset");

    // Save as preset.
    // The file's name exists already: refused without `overwrite` (Logic's presets are
    // never replaced silently), then replaced when asked to.
    let clash = presets::user_preset_path(&root, &info, "My Organ");
    std::fs::write(&clash, b"logic's own").unwrap();
    s.inner.lock().plugins.host.as_ref().unwrap().rescan().unwrap();
    assert!(s.send(SoundsCmd::SavePartAsPluginPreset { part: 1, name: "My Organ".into(), category: PatchCategory::Organ, overwrite: false }).is_err());
    assert_eq!(std::fs::read(&clash).unwrap(), b"logic's own");
    s.send(SoundsCmd::SavePartAsPluginPreset { part: 1, name: "My Organ".into(), category: PatchCategory::Organ, overwrite: true }).unwrap();
    let t0 = Instant::now();
    while s.state().keyboard_parts[1].plugin.as_ref().and_then(|p| p.preset.clone()).is_none() {
        assert!(t0.elapsed() < Duration::from_secs(20), "the preset was not saved");
        s.advance(1_000_000);
        std::thread::sleep(Duration::from_millis(5));
    }
    let saved = root.join("Apple/DLSMusicDevice/My Organ.aupreset");
    assert!(saved.exists());
    let cat = s.sound_catalog();
    let mine = cat.entries.iter().find(|e| e.name == "My Organ" && e.parent.is_some()).expect("listed under the plugin");
    assert_eq!(mine.category, PatchCategory::Organ);
    assert_eq!(s.state().keyboard_parts[1].plugin.clone().unwrap().preset_key, Some(format!("u:{}", saved.display())));

    // The saved parts keep the preset; an old file without it still reads.
    let json = serde_json::to_string(&s.inner.lock().saved_parts()).unwrap();
    let back: super::Saved = serde_json::from_str(&json).unwrap();
    assert_eq!(back.parts[1].as_ref().unwrap().preset.as_ref().unwrap().name, "My Organ");
    let old: super::Saved = serde_json::from_str(r#"{"parts":[{"id":"aumu dls  appl","state":null},null,null,null]}"#).unwrap();
    assert_eq!(old.parts[0].as_ref().unwrap().preset, None);
    let _ = std::fs::remove_dir_all(&data);
}

/// One save makes one record (docs/racks.md "Saving"): Save as… with an `.aupreset`
/// makes one sound, and the file is only an export: picking it adds nothing, and a map
/// rule naming it gets that same sound. Picking an `.aupreset` adds no library record (the
/// part plays the preset, named by its catalog id); Save makes one sound named after the
/// preset, which the part then plays, so saving again updates it.
#[test]
fn one_save_makes_one_record() {
    use crate::api::{PatchCategory, SoundLibraryCmd, SoundsCmd};
    use crate::plugin::{presets, LoadConfig, PluginHost, PluginId};
    let p = testing::style_path();
    let data = std::env::temp_dir().join(format!("yahaha-one-record-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data);
    let root = data.join("Presets");
    let host = PluginHost::with_preset_roots(Some(data.join("plugins.json")), vec![root.clone()]);
    let info = host.info(&PluginId::DLS).unwrap();
    let state = host.load(&PluginId::DLS, LoadConfig::default()).unwrap().get_state().unwrap();
    let file = presets::write_user_preset(&root, &info, "Warm Strings", &state, false).unwrap();
    host.rescan().unwrap();
    let opts = Options { paths: vec![p], data_dir: Some(data.join("data")), ..Options::default() };
    let s = Session::offline(opts).unwrap();
    s.offline_audio(None, 48_000).unwrap();
    s.inner.lock().plugins.host = Some(host);
    wait_scanned(&s);
    let records = |s: &Session| s.state().sound_library.patches.len();
    let n = records(&s);

    // Save as… with the .aupreset box: one sound; the file is only an export.
    s.send(SoundsCmd::AssignSound { part: 1, id: format!("au:{DLS}") }).unwrap();
    assert_eq!(wait_playing(&s, 1), PluginStatus::Playing);
    s.send(SoundsCmd::SavePartAsPluginPreset { part: 1, name: "My Organ".into(), category: PatchCategory::Organ, overwrite: false }).unwrap();
    s.send(SoundLibraryCmd::SaveSoundAs { part: 1, name: Some("My Organ".into()) }).unwrap();
    let t0 = Instant::now();
    while s.state().keyboard_parts[1].plugin.as_ref().and_then(|p| p.preset.clone()).is_none() {
        assert!(t0.elapsed() < Duration::from_secs(20), "the preset was not saved");
        s.advance(1_000_000);
        std::thread::sleep(Duration::from_millis(5));
    }
    wait_reads(&s);
    assert_eq!(records(&s), n + 1, "Save as… with an .aupreset makes one record");
    let organ = s.state().sound_library.last_added.clone().unwrap();
    let saved = format!("au:{DLS}#u:{}", root.join("Apple/DLSMusicDevice/My Organ.aupreset").display());
    s.send(SoundsCmd::AssignSound { part: 2, id: saved.clone() }).unwrap();
    assert_eq!(wait_playing(&s, 2), PluginStatus::Playing);
    wait_reads(&s);
    assert_eq!(records(&s), n + 1, "picking the exported file adds nothing");
    s.send(SoundLibraryCmd::SetFamilyRule { family: 2, patch: Some(saved), style: false }).unwrap();
    assert_eq!(records(&s), n + 1, "a rule naming the file gets the sound saved with it");
    assert_eq!(s.state().sound_library.map.families[2].as_deref(), Some(organ.as_str()));

    // Pick a preset: no record; the part plays the preset.
    let id = format!("au:{DLS}#u:{}", file.path.display());
    s.send(SoundsCmd::AssignSound { part: 0, id: id.clone() }).unwrap();
    assert_eq!(wait_playing(&s, 0), PluginStatus::Playing);
    wait_reads(&s);
    assert_eq!(records(&s), n + 1, "picking a preset adds no record");
    let tag = s.state().keyboard_parts[0].sound.clone().expect("the part plays the preset");
    assert_eq!((tag.id.as_str(), tag.name.as_str()), (id.as_str(), "Warm Strings"));
    assert!(!s.state().keyboard_parts[0].sound_edited);

    // Save three times: one sound, named after the preset, which the part plays.
    for _ in 0..3 {
        s.send(SoundLibraryCmd::SaveSound { part: 0 }).unwrap();
        wait_reads(&s);
    }
    assert_eq!(records(&s), n + 2, "three saves, one record");
    let mine = s.state().sound_library.patches.last().unwrap().patch.clone();
    assert_eq!(mine.name, "Warm Strings");
    assert_eq!(s.state().keyboard_parts[0].sound.clone().map(|t| t.id), Some(format!("saved:{}", mine.id)));
    let _ = std::fs::remove_dir_all(&data);
}

/// Pump until no plugin state read is running, then twice more (a save fill lands after).
fn wait_reads(s: &Session) {
    let t0 = Instant::now();
    while s.inner.lock().plugin_state_reads_pending() && t0.elapsed() < Duration::from_secs(10) {
        s.advance(1_000_000);
        std::thread::sleep(Duration::from_millis(2));
    }
    s.advance(1_000_000);
    s.advance(1_000_000);
}

/// Now playing and the one save flow (O3): Save as… names the part's plugin sound; an
/// autosave-schedule read whose fingerprint differs from the baseline marks it edited;
/// Save writes the state over the sound, clears the mark and keeps the instance.
#[test]
fn a_part_shows_its_sound_and_when_it_was_edited() {
    use crate::api::SoundLibraryCmd;
    use crate::patches::PatchSource;
    let s = session();
    s.offline_audio(None, 48_000).unwrap();
    s.send(PluginCmd::SetPartPlugin { part: 0, id: DLS.into(), state: None }).unwrap();
    assert_eq!(wait_playing(&s, 0), PluginStatus::Playing);
    let bare = s.state().keyboard_parts[0].clone();
    assert_eq!(bare.sound.map(|t| t.id), Some(format!("au:{DLS}")), "a bare plugin is its catalog row");
    assert_eq!(bare.voice_name, bare.plugin.unwrap().name, "named by the plugin, not its GM voice");
    // Save with no sound is Save as….
    s.send(SoundLibraryCmd::SaveSound { part: 0 }).unwrap();
    let id = s.state().sound_library.last_added.clone().unwrap();
    wait_reads(&s);
    let p0 = s.state().keyboard_parts[0].clone();
    let tag = p0.sound.clone().expect("the part plays the new sound");
    assert_eq!((tag.id.as_str(), tag.name.as_str(), p0.sound_edited), (format!("saved:{id}").as_str(), "DLSMusicDevice", false));
    let state = |s: &Session| match s.state().sound_library.patches.iter().find(|p| p.patch.id == id).unwrap().patch.source.clone() {
        PatchSource::Plugin { state, .. } => state,
        other => panic!("not a plugin sound: {other:?}"),
    };
    assert!(!state(&s).is_empty(), "the fresh read landed in the sound");

    // The editor changes it: the next read's fingerprint is not the baseline's.
    let ch = crate::parts::CHANNEL[0] as usize;
    let editor = |s: &Session| s.inner.lock().plugins.channels[ch].as_ref().unwrap().editor.clone().unwrap();
    let before = editor(&s).instance();
    s.inner.lock().plugins.channels[ch].as_mut().unwrap().sound_fp = Some(1);
    s.inner.lock().save_channel_state(ch as u8).unwrap();
    wait_reads(&s);
    assert!(s.state().keyboard_parts[0].sound_edited, "edited");
    // Save: the same sound, not edited, the same instance.
    s.send(SoundLibraryCmd::SaveSound { part: 0 }).unwrap();
    wait_reads(&s);
    let p0 = s.state().keyboard_parts[0].clone();
    assert_eq!((p0.sound, p0.sound_edited), (Some(tag.clone()), false));
    assert_eq!(s.state().sound_library.patches.iter().filter(|p| matches!(&p.patch.source, PatchSource::Plugin { .. })).count(), 1, "overwritten, not added");
    assert!(before.is(&editor(&s)), "Save doesn't reload the plugin");

    // Save as… on a part playing it as its own patch: the copy plays, on the same instance.
    s.send(SoundLibraryCmd::SetPartPatch { part: 1, id: Some(id.clone()) }).unwrap();
    assert_eq!(wait_playing(&s, 1), PluginStatus::Playing);
    wait_reads(&s);
    assert_eq!(s.state().keyboard_parts[1].sound, Some(tag));
    s.send(SoundLibraryCmd::SaveSoundAs { part: 1, name: Some("Mine".into()) }).unwrap();
    let copy = s.state().sound_library.last_added.clone().unwrap();
    wait_reads(&s);
    let p1 = s.state().keyboard_parts[1].clone();
    assert_eq!((p1.patch.as_deref(), p1.sound.map(|t| t.name), p1.sound_edited), (Some(copy.as_str()), Some("Mine".to_string()), false));
    assert_eq!(p1.plugin.map(|p| p.status), Some(PluginStatus::Playing));
}

/// docs/racks.md, "Saving": with the part's plugin window open, a knob turned there shows
/// as "edited" within about a second (not at the 30-second autosave, which an offline
/// session never runs), and turning it back clears it. With the window closed, nothing
/// reads the state.
#[test]
fn an_edit_in_the_open_plugin_window_shows_at_once_and_undoing_it_clears_it() {
    use crate::api::SoundLibraryCmd;
    let s = session();
    s.offline_audio(None, 48_000).unwrap();
    s.send(PluginCmd::SetPartPlugin { part: 0, id: DLS.into(), state: None }).unwrap();
    assert_eq!(wait_playing(&s, 0), PluginStatus::Playing);
    s.send(SoundLibraryCmd::SaveSound { part: 0 }).unwrap();
    wait_reads(&s);
    assert!(s.state().keyboard_parts[0].sound.is_some() && !s.state().keyboard_parts[0].sound_edited);
    let ch = crate::parts::CHANNEL[0] as usize;
    let editor = s.inner.lock().plugins.channels[ch].as_ref().unwrap().editor.clone().unwrap();
    // DLSMusicDevice's tuning (kMusicDeviceParam_Tuning, global), in cents.
    let tune = |cents: f32| editor.set_parameter(0, 0, 0, cents).unwrap();
    // Pump (half a second of session time a step) until `done`, for at most 5 s of wall time.
    let until = |s: &Session, what: &str, done: &dyn Fn(&Session) -> bool| {
        let t0 = Instant::now();
        while !done(s) {
            assert!(t0.elapsed() < Duration::from_secs(5), "{what}");
            s.advance(250_000_000);
            std::thread::sleep(Duration::from_millis(5));
        }
    };

    // Window closed: an edit isn't seen until something reads the state.
    tune(30.0);
    for _ in 0..8 {
        s.advance(250_000_000);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!s.state().keyboard_parts[0].sound_edited, "no reads while the window is closed");
    assert!(s.inner.lock().plugins.probes.is_empty());
    tune(0.0);

    // The window opens: its first read is the baseline if none was taken yet.
    let window = editor.watch();
    assert!(editor.editor_open());
    until(&s, "the open window's reads start", &|s| s.inner.lock().plugins.channels[ch].as_ref().unwrap().sound_fp.is_some());
    s.advance(1_000_000_000);
    std::thread::sleep(Duration::from_millis(20));
    s.advance(1_000_000_000);
    assert!(!s.state().keyboard_parts[0].sound_edited, "nothing changed yet");

    tune(30.0);
    until(&s, "the edit shows while the window is open", &|s| s.state().keyboard_parts[0].sound_edited);
    tune(0.0);
    until(&s, "undoing the edit clears it", &|s| !s.state().keyboard_parts[0].sound_edited);

    // Closed again: no more reads.
    window.release();
    assert!(!editor.editor_open());
    until(&s, "the last read lands", &|s| s.inner.lock().plugins.probes.is_empty());
    tune(30.0);
    for _ in 0..8 {
        s.advance(250_000_000);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(s.inner.lock().plugins.probes.is_empty() && !s.state().keyboard_parts[0].sound_edited);
}

/// A fingerprint read that ends without a result (its thread died) still frees the part's
/// probe: the part keeps being read, so a later edit in the open window still shows.
#[test]
fn a_probe_that_ends_without_a_result_does_not_stop_the_reads() {
    use crate::api::SoundLibraryCmd;
    let s = session();
    s.offline_audio(None, 48_000).unwrap();
    s.send(PluginCmd::SetPartPlugin { part: 0, id: DLS.into(), state: None }).unwrap();
    assert_eq!(wait_playing(&s, 0), PluginStatus::Playing);
    s.send(SoundLibraryCmd::SaveSound { part: 0 }).unwrap();
    wait_reads(&s);
    let ch = crate::parts::CHANNEL[0];
    let editor = s.inner.lock().plugins.channels[ch as usize].as_ref().unwrap().editor.clone().unwrap();
    let until = |s: &Session, what: &str, done: &dyn Fn(&Session) -> bool| {
        let t0 = Instant::now();
        while !done(s) {
            assert!(t0.elapsed() < Duration::from_secs(5), "{what}");
            s.advance(250_000_000);
            std::thread::sleep(Duration::from_millis(5));
        }
    };
    let _window = editor.watch();
    until(&s, "the open window's reads start", &|s| s.inner.lock().plugins.channels[ch as usize].as_ref().unwrap().sound_fp.is_some());
    until(&s, "no read in flight", &|s| s.inner.lock().plugins.probes.is_empty());

    // A read whose thread dies before it sends anything.
    {
        let mut g = s.inner.lock();
        let (tx, rx) = std::sync::mpsc::channel();
        drop(tx);
        g.plugins.channels[ch as usize].as_mut().unwrap().probe.running = true;
        g.plugins.probes.push((ch, rx));
    }
    editor.set_parameter(0, 0, 0, 30.0).unwrap();
    until(&s, "the edit still shows", &|s| s.state().keyboard_parts[0].sound_edited);
}

/// D5/O7: a plugin sound in the library exports as an `.aupreset` in its plugin's preset
/// folder (where Logic reads it), with #307's overwrite rule. The plugin is the made-up
/// "Sampler Deluxe" from a mock scan cache; no real plugin state is used.
#[test]
fn a_plugin_sound_exports_as_an_aupreset() {
    use crate::api::{base64_encode, SoundLibraryCmd};
    use crate::patches::{Category, Patch, PatchSource};
    use crate::plugin::{presets, PluginFormat, PluginId, PluginInfo};
    let s = session();
    let dir = std::env::temp_dir().join(format!("yahaha-sound-aupreset-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let root = dir.join("Presets");
    let id = PluginId::parse("aumu Smp7 Fake").unwrap();
    let fake = PluginInfo {
        id,
        name: "Sampler Deluxe".into(),
        manufacturer: "Fake Instruments".into(),
        version: 0x10000,
        format: PluginFormat::Au2,
        requires_async: false,
        can_load_in_process: false,
        sandbox_safe: true,
        last_load: None,
        in_process: false,
        factory_presets: None,
        user_presets: Vec::new(),
    };
    let host = crate::plugin::mock_host(&dir.join("plugins.json"), vec![root.clone()], vec![fake.clone()]);
    host.scan().unwrap();
    s.inner.lock().plugins.host = Some(host);
    let state = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>manufacturer</key><integer>{}</integer><key>name</key><string>Deluxe Keys</string><key>subtype</key><integer>{}</integer><key>type</key><integer>{}</integer><key>version</key><integer>0</integer></dict></plist>"#,
        id.manufacturer, id.subtype, id.kind
    );
    let sound = |pid: &str, name: &str, st: &str| Patch {
        id: pid.into(),
        name: name.into(),
        category: Category::guess(0, 4),
        tags: Vec::new(),
        favourite: false,
        source: PatchSource::plugin(id.to_string(), base64_encode(st.as_bytes())),
    };
    let mut font = sound("font", "Font", "");
    font.source = PatchSource::SoundFont { file: "Test.sf2".into(), bank: 0, program: 0 };
    s.inner.lock().sound.lib.patches.extend([sound("deluxe", "Deluxe Keys", &state), sound("empty", "Unplayed", ""), font]);

    s.send(SoundLibraryCmd::ExportSoundPreset { id: "deluxe".into(), overwrite: false }).unwrap();
    let file = presets::user_preset_path(&root, &fake, "Deluxe Keys");
    assert_eq!(file, root.join("Fake Instruments/Sampler Deluxe/Deluxe Keys.aupreset"));
    let h = presets::parse_aupreset(&std::fs::read(&file).unwrap()).unwrap();
    assert_eq!((h.name.as_deref(), h.id), (Some("Deluxe Keys"), Some(id)), "Logic reads the unit and name");
    // #307: an existing preset of that name is never replaced silently.
    std::fs::write(&file, b"logic's own").unwrap();
    assert!(s.send(SoundLibraryCmd::ExportSoundPreset { id: "deluxe".into(), overwrite: false }).is_err());
    assert_eq!(std::fs::read(&file).unwrap(), b"logic's own");
    s.send(SoundLibraryCmd::ExportSoundPreset { id: "deluxe".into(), overwrite: true }).unwrap();
    assert!(presets::parse_aupreset(&std::fs::read(&file).unwrap()).is_some());
    // Nothing to write: no state yet, a SoundFont preset, no such sound.
    for bad in ["empty", "font", "nope"] {
        assert!(s.send(SoundLibraryCmd::ExportSoundPreset { id: bad.into(), overwrite: false }).is_err(), "{bad}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The made-up "Sampler Deluxe" in a mock scan cache (it is not in the registrar, so a
/// load of it fails), as the session's plugin list. No real plugin is loaded.
fn with_fake_sampler(s: &Session, tag: &str) -> std::path::PathBuf {
    use crate::plugin::{PluginFormat, PluginId, PluginInfo};
    let dir = std::env::temp_dir().join(format!("yahaha-listing-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let fake = PluginInfo {
        id: PluginId::parse("aumu Smp7 Fake").unwrap(),
        name: "Sampler Deluxe".into(),
        manufacturer: "Fake Instruments".into(),
        version: 0x10000,
        format: PluginFormat::Au2,
        requires_async: false,
        can_load_in_process: false,
        sandbox_safe: true,
        last_load: None,
        in_process: false,
        factory_presets: None,
        user_presets: Vec::new(),
    };
    let host = crate::plugin::mock_host(&dir.join("plugins.json"), vec![dir.join("Presets")], vec![fake]);
    let list = host.scan().unwrap();
    let mut ctl = s.inner.lock();
    ctl.plugins.host = Some(host);
    ctl.plugins.list = list;
    dir
}

const FAKE: &str = "au:aumu Smp7 Fake";

fn fake_entry(s: &Session) -> crate::api::SoundPluginInfo {
    s.sound_catalog().entries.iter().find(|e| e.id == FAKE).and_then(|e| e.plugin.clone()).expect("the fake sampler's entry")
}

/// Pump until no preset listing runs (and the state says so).
fn wait_listed(s: &Session) {
    let t0 = Instant::now();
    loop {
        s.advance(1_000_000);
        if s.inner.lock().plugins.listing.is_empty() {
            break;
        }
        assert!(t0.elapsed() < Duration::from_secs(40), "the preset listing never ended");
        std::thread::sleep(Duration::from_millis(5));
    }
    s.advance(1_000_000);
    assert!(s.state().sounds.listing_presets.is_empty());
}

/// A listing whose load fails ends: the browser stops waiting, with the reason, and a
/// second ask does not load the plugin again.
#[test]
fn a_preset_listing_that_fails_ends_and_says_why() {
    use crate::api::SoundsCmd;
    let s = session();
    let dir = with_fake_sampler(&s, "fails");
    assert_eq!(fake_entry(&s).presets, None);
    s.send(SoundsCmd::ListPluginPresets { id: FAKE.into() }).unwrap();
    wait_listed(&s);
    let e = fake_entry(&s);
    assert_eq!(e.presets, None);
    assert!(e.presets_error.as_deref().is_some_and(|m| m.contains("registrar")), "{:?}", e.presets_error);
    assert!(s.state().message.as_ref().is_some_and(|m| m.error && m.text.contains("could not list its presets")));
    // Not tried again until the next scan.
    s.send(SoundsCmd::ListPluginPresets { id: FAKE.into() }).unwrap();
    assert!(s.state().sounds.listing_presets.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

/// A listing that never answers, dies without an answer, or answers without a preset list
/// (the plugin changed since the scan) ends too.
#[test]
fn a_preset_listing_without_an_answer_ends() {
    use super::imp::{PresetListing, LISTING_DEADLINE};
    use std::sync::mpsc;
    let s = session();
    let dir = with_fake_sampler(&s, "no-answer");
    let fake = s.inner.lock().plugins.list[0].clone();
    for why in ["no answer after", "stopped without an answer", "no preset list"] {
        s.inner.lock().plugins.listing_failed.clear();
        let (tx, rx) = mpsc::channel();
        let deadline = Instant::now() + LISTING_DEADLINE;
        // The sender is kept alive (no answer yet) or dropped (the thread died).
        let _alive = match why {
            "no answer after" => Some(tx),
            "stopped without an answer" => {
                drop(tx);
                None
            }
            _ => {
                tx.send(Ok(fake.clone())).unwrap();
                None
            }
        };
        s.inner.lock().plugins.listing.push(PresetListing { id: "aumu Smp7 Fake".into(), rx, deadline });
        if _alive.is_some() {
            // Asked again while one runs: no second listing (no second instance).
            s.send(crate::api::SoundsCmd::ListPluginPresets { id: FAKE.into() }).unwrap();
            assert_eq!(s.inner.lock().plugin_presets_listing(), ["aumu Smp7 Fake"]);
            // Then its deadline passes.
            s.inner.lock().plugins.listing[0].deadline = Instant::now();
        }
        wait_listed(&s);
        let e = fake_entry(&s);
        assert!(e.presets_error.as_deref().is_some_and(|m| m.contains(why)), "{why}: {:?}", e.presets_error);
    }
    let _ = std::fs::remove_dir_all(&dir);
}
