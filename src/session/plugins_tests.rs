//! Plugin parts through an offline session, with Apple's DLSMusicDevice (every Mac has
//! it) and the real audio callback (`Session::offline_audio` / `render`).

use super::super::{Options, Port, Session};
use crate::api::{PluginCmd, PluginStatus, PartsCmd};
use std::path::Path;
use std::time::{Duration, Instant};

const DLS: &str = "aumu dls  appl";

fn session() -> Option<Session> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
    if !p.exists() {
        eprintln!("corpus missing; skipping");
        return None;
    }
    Some(Session::offline(Options { paths: vec![p], ..Options::default() }).unwrap())
}

fn energy(l: &[f32], r: &[f32]) -> f64 {
    l.iter().chain(r).map(|x| (*x as f64).powi(2)).sum()
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
    let Some(s) = session() else { return };
    assert!(!s.state().plugins.available);
    assert!(s.send(PluginCmd::SetPartPlugin { part: 0, id: DLS.into(), state: None }).is_err());
    assert!(s.state().keyboard_parts[0].plugin.is_none());
}

/// Right 1 on DLSMusicDevice, with no SoundFont at all: every sound here is the plugin's.
/// It plays, the meter moves, its CC7 is its level, its state saves and restores, and
/// clearing it gives the part back to the (here absent) SoundFont.
#[test]
fn a_keyboard_part_plays_an_audio_unit() {
    let Some(s) = session() else { return };
    s.offline_audio(None, 48_000).unwrap();
    assert!(s.state().plugins.available);
    assert!(s.send(PluginCmd::SetPartPlugin { part: 0, id: "aumu nope nope".into(), state: None }).is_err(), "not installed");
    s.send(PluginCmd::SetPartPlugin { part: 0, id: DLS.into(), state: None }).unwrap();
    let p = s.state().keyboard_parts[0].plugin.clone().unwrap();
    assert_eq!((p.status, p.name.as_str()), (PluginStatus::Loading, "DLSMusicDevice"));
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
    let Some(s) = session() else { return };
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("soundfonts");
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
    let Some(s) = session() else { return };
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
    let Some(s) = session() else { return };
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
    let Some(s) = session() else { return };
    s.offline_audio(None, 48_000).unwrap();
    let mut saved = super::Saved::default();
    saved.parts[2] = Some(super::PluginVoice { id: "aumu Nope Gone".into(), state: None });
    s.inner.lock().restore_saved(saved);
    s.advance(1_000_000);
    let p = s.state().keyboard_parts[2].plugin.clone().unwrap();
    assert_eq!((p.status, p.name.as_str()), (PluginStatus::Failed, "aumu Nope Gone"));
}

/// The start-up restore of a plugin that isn't installed (uninstalled, licence missing,
/// an AUv3 whose app moved) keeps the part's saved choice and state as `failed`, so it
/// survives the next save; and a restore never falls back to loading in process
/// (#105 review B2, B3).
#[test]
fn a_restore_keeps_a_missing_plugin_and_never_falls_back_in_process() {
    let Some(s) = session() else { return };
    s.offline_audio(None, 48_000).unwrap();
    let mut saved = super::Saved::default();
    saved.parts[0] = Some(super::PluginVoice { id: "aumu Nope Gone".into(), state: Some(vec![1, 2, 3]) });
    saved.parts[3] = Some(super::PluginVoice { id: DLS.into(), state: None });
    {
        let mut ctl = s.inner.lock();
        ctl.restore_saved(saved);
        let ch = crate::parts::CHANNEL[3] as usize;
        assert!(!ctl.plugins.channels[ch].as_ref().unwrap().allow_fallback, "no in-process fallback at start-up");
    }
    assert_eq!(wait_playing(&s, 3), PluginStatus::Playing);
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
    let Some(s) = session() else { return };
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
    let Some(s) = session() else { return };
    s.finish_indexing();
    s.offline_audio(None, 48_000).unwrap();
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
    use crate::patches::{PatchDefaults, PatchSource};
    let Some(s) = session() else { return };
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
        source: PatchSource::Plugin { component_id: DLS.into(), state: state.clone() },
        defaults: PatchDefaults { volume: Some(90), ..PatchDefaults::default() },
    };
    s.send(SoundLibraryCmd::CreatePatch { patch: fields }).unwrap();
    let id = s.state().sound_library.last_added.clone().unwrap();
    assert!(s.state().sound_library.patches.iter().any(|p| p.patch.id == id && p.available));

    s.send(SoundLibraryCmd::SetPartPatch { part: 0, id: Some(id.clone()) }).unwrap();
    let r1 = |s: &Session| s.state().keyboard_parts[0].clone();
    assert_eq!((r1(&s).patch.as_deref(), r1(&s).voice_name.as_str(), r1(&s).volume), (Some(id.as_str()), "DLS Keys", 90));
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

/// A plugin patch auditions like a SoundFont one (#109): with the band stopped, its plugin
/// loads on channel 16 (the audition channel), plays the phrase through the rack, and goes
/// when the audition ends; nothing is left on the channel.
#[test]
fn a_plugin_patch_auditions_on_channel_16() {
    use crate::api::{PatchFields, SoundLibraryCmd, TransportCmd};
    use crate::patches::{PatchDefaults, PatchSource};
    let Some(s) = session() else { return };
    s.offline_audio(None, 48_000).unwrap();
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
    let ch16 = |s: &Session| s.inner.lock().channel_plugin(15).map(|p| p.status);
    s.send(SoundLibraryCmd::AuditionPatch { id: id.clone() }).unwrap();
    assert_eq!(s.state().sound_library.auditioning.as_deref(), Some(id.as_str()));
    assert_eq!(ch16(&s), Some(PluginStatus::Loading));
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
    let Some(s) = session() else { return };
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
