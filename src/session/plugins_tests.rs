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
    let saved = s.inner.lock().part_plugin_voice(0).unwrap();
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
