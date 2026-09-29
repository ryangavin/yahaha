//! The live rack: restored after a stop, migrated from plugin-parts.json, a corrupt or newer
//! file, and what sets `modified`. Offline sessions with a live rack file of their own.

use super::{FILE, MAX_WAIT_NS, NEW_NAME, OLD_FILE, QUIET_NS, RESTORED_NAME};
use crate::api::*;
use crate::racks::{ControlTarget, SoundRef};
use crate::session::{Options, Session};
use std::path::{Path, PathBuf};

/// A fresh folder for one test: the style in `styles/`, the live rack in `app/`.
fn dir(test: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("yahaha-live-rack-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("app")).unwrap();
    d
}

fn live_path(d: &Path) -> PathBuf {
    d.join("app").join(FILE)
}

fn chunk(id: &[u8], body: &[u8]) -> Vec<u8> {
    let mut v = id.to_vec();
    v.extend_from_slice(&(body.len() as u32).to_be_bytes());
    v.extend_from_slice(body);
    v
}

/// An offline session on a tiny synthetic style (one Main A bar, written here, so these
/// tests need no corpus), keeping its live rack in `d/app` when `live` is set.
fn session_with(d: &Path, live: bool) -> Session {
    let mut trk = vec![0x00, 0xFF, 0x06, 4];
    trk.extend_from_slice(b"SFF2");
    trk.extend_from_slice(&[0x00, 0xFF, 0x06, 6]);
    trk.extend_from_slice(b"Main A");
    trk.extend_from_slice(&[0x00, 0x9B, 60, 100, 0x83, 0x00, 0x8B, 60, 0, 0x00, 0xFF, 0x2F, 0]);
    let mut bytes = chunk(b"MThd", &[0, 0, 0, 1, 0, 96]);
    bytes.extend(chunk(b"MTrk", &trk));
    bytes.extend(chunk(b"CASM", &chunk(b"CSEG", &chunk(b"Sdec", b"Main A"))));
    let style = d.join("styles/Test.sty");
    std::fs::create_dir_all(style.parent().unwrap()).unwrap();
    std::fs::write(&style, bytes).unwrap();
    let live_rack = live.then(|| live_path(d));
    Session::offline(Options { paths: vec![style], live_rack, ..Options::default() }).unwrap()
}

fn session(d: &Path) -> Session {
    session_with(d, true)
}

/// The keyboard parts as the app shows them: on, program, volume, octave, pan and sends.
fn parts(s: &Session) -> Vec<(bool, u8, u8, i8, u8, u8, u8, u8)> {
    s.state().keyboard_parts.iter().map(|p| (p.on, p.program, p.volume, p.octave, p.pan, p.reverb, p.chorus, p.variation)).collect()
}

fn live(s: &Session) -> LiveRackState {
    s.state().live_rack.clone()
}

/// A plugin state for a plugin that isn't installed: no real plugin's settings.
const FAKE_PLUGIN: &str = "aumu zzzz fake";
fn fake_state() -> String {
    base64_encode(b"fake plugin settings, edited")
}

/// Right 3 plays a plugin with settings of its own (a bare plugin: no library sound). It
/// isn't installed, so it stays on the part as failed, keeping its settings.
fn put_fake_plugin(s: &Session) {
    let mut rack = s.capture_rack("With a plugin");
    rack.parts[2].sound = SoundRef::Plugin { component: FAKE_PLUGIN.into() };
    rack.parts[2].edited_state = Some(fake_state());
    s.apply_rack(&rack);
}

#[test]
fn stop_then_start_restores_the_parts_the_mix_and_a_plugins_state() {
    let d = dir("restore");
    let a = session(&d);
    assert_eq!(live(&a), LiveRackState { name: NEW_NAME.into(), id: None, modified: false }, "no live rack yet: a new one");
    a.send(PartsCmd::SetPartVoice { part: 0, program: 4 }).unwrap();
    a.send(PartsCmd::SetPartVolume { part: 0, volume: 77 }).unwrap();
    a.send(PartsCmd::SetPartPan { part: 0, pan: 30 }).unwrap();
    a.send(PartsCmd::SetPartSend { part: 0, send: PartSend::Reverb, value: 90 }).unwrap();
    a.send(PartsCmd::SetPartOctave { part: 0, octave: 1 }).unwrap();
    a.send(PartsCmd::SetPartOn { part: 1, on: true }).unwrap();
    a.send(PartsCmd::SetPartVolume { part: 1, volume: 50 }).unwrap();
    a.send(PartsCmd::SetPartSend { part: 3, send: PartSend::Chorus, value: 44 }).unwrap();
    a.send(ChordCmd::SetSplit { note: 60 }).unwrap();
    a.send(ChordCmd::SetTranspose { keyboard: 3, master: 0 }).unwrap();
    a.send(HarmonyArpCmd::SetHarmonyType { index: 2 }).unwrap();
    a.send(HarmonyArpCmd::SetHarmonyArpOn { on: true }).unwrap();
    put_fake_plugin(&a);
    let before = (parts(&a), a.state().chord.split, a.state().chord.transpose_keyboard, a.state().harmony_arp.clone());
    let rack = a.capture_rack("");
    assert!(live(&a).modified);
    a.stop();
    assert!(live_path(&d).exists(), "stopping saves the live rack");

    let b = session(&d);
    let after = (parts(&b), b.state().chord.split, b.state().chord.transpose_keyboard, b.state().harmony_arp.clone());
    assert_eq!(after, before, "the parts sound and mix as before the stop");
    assert_eq!(live(&b), LiveRackState { name: NEW_NAME.into(), id: None, modified: true }, "still unsaved");
    if cfg!(feature = "plugins") {
        let again = b.capture_rack("");
        assert_eq!(again.parts[2].sound, SoundRef::Plugin { component: FAKE_PLUGIN.into() });
        assert_eq!(again.parts[2].edited_state, Some(fake_state()), "the plugin's own settings come back");
        assert_eq!(again.parts, rack.parts, "every part as it was");
    }

    // A saved rack's name and id come back too, unmodified, and nothing that settles after
    // the start counts as a change.
    b.live_rack_clean("Ballad", Some("r1".into()));
    b.stop();
    let c = session(&d);
    c.advance(MAX_WAIT_NS);
    assert_eq!(live(&c), LiveRackState { name: "Ballad".into(), id: Some("r1".into()), modified: false });
    drop(c);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_change_autosaves_once_things_are_quiet() {
    let d = dir("autosave");
    let a = session(&d);
    a.send(PartsCmd::SetPartVolume { part: 0, volume: 33 }).unwrap();
    a.advance(QUIET_NS / 2);
    assert!(!live_path(&d).exists(), "not while changes are still coming");
    a.advance(QUIET_NS);
    // Written on the live-rack thread.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let saved = loop {
        let text = std::fs::read_to_string(live_path(&d)).unwrap_or_default();
        if text.contains("\"volume\": 33") || std::time::Instant::now() > deadline {
            break text;
        }
        std::thread::yield_now();
    };
    assert!(saved.contains("\"volume\": 33"), "saved without a stop: {saved}");
    assert!(saved.contains("\"format\": \"yahaha.liveRack\""));
    drop(a);
    let _ = std::fs::remove_dir_all(&d);
}

/// The first start after racks came in: plugin-parts.json and today's parts become the live
/// rack "Restored"; the old file stays and isn't read again.
#[test]
fn plugin_parts_json_becomes_the_live_rack_restored() {
    let d = dir("migrate");
    let old = d.join("app").join(OLD_FILE);
    let fixture = format!(r#"{{"parts":[null,{{"id":"{FAKE_PLUGIN}","state":"{}"}},null,null]}}"#, fake_state());
    std::fs::write(&old, &fixture).unwrap();

    let a = session(&d);
    assert_eq!(live(&a), LiveRackState { name: RESTORED_NAME.into(), id: None, modified: true }, "saved nowhere else, so modified");
    if cfg!(feature = "plugins") {
        let plugin = a.state().keyboard_parts[1].plugin.clone().expect("Right 2's plugin from plugin-parts.json");
        assert_eq!(plugin.id, FAKE_PLUGIN);
    }
    a.send(PartsCmd::SetPartVolume { part: 0, volume: 91 }).unwrap();
    a.stop();
    assert!(live_path(&d).exists());
    assert_eq!(std::fs::read_to_string(&old).unwrap(), fixture, "the old file is left as it was");

    // From now on the live rack is what's read: plugin-parts.json, even changed, is not.
    std::fs::write(&old, r#"{"parts":[null,null,null,null]}"#).unwrap();
    let b = session(&d);
    assert_eq!(live(&b).name, RESTORED_NAME);
    assert_eq!(b.state().keyboard_parts[0].volume, 91);
    if cfg!(feature = "plugins") {
        let r = b.capture_rack("");
        assert_eq!(r.parts[1].sound, SoundRef::Plugin { component: FAKE_PLUGIN.into() });
        assert_eq!(r.parts[1].edited_state, Some(fake_state()), "the plugin's state came over");
    }
    drop(b);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_corrupt_live_rack_is_moved_aside_and_the_session_starts_on_defaults() {
    let d = dir("corrupt");
    let fresh = parts(&session_with(&d, false));
    std::fs::write(live_path(&d), "{ not json").unwrap();
    let a = session(&d);
    let st = a.state();
    assert_eq!(parts(&a), fresh, "the defaults");
    assert_eq!(st.live_rack.name, NEW_NAME);
    let msg = st.message.clone().expect("it says so");
    assert!(msg.error && msg.text.contains("live rack could not be read"), "{msg:?}");
    let bak = live_path(&d).with_extension("json.bak");
    assert_eq!(std::fs::read_to_string(&bak).unwrap(), "{ not json", "kept for the user");
    a.stop();
    assert!(session(&d).state().message.is_none(), "the next start reads the new save");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_live_rack_from_a_newer_yahaha_is_left_alone() {
    let d = dir("newer");
    let newer = r#"{"format":"yahaha.liveRack","version":99,"rack":{}}"#;
    std::fs::write(live_path(&d), newer).unwrap();
    let a = session(&d);
    assert!(a.state().message.as_ref().is_some_and(|m| m.error && m.text.contains("newer")));
    a.send(PartsCmd::SetPartVolume { part: 0, volume: 12 }).unwrap();
    a.advance(MAX_WAIT_NS);
    a.stop();
    assert_eq!(std::fs::read_to_string(live_path(&d)).unwrap(), newer, "never saved over");
    let _ = std::fs::remove_dir_all(&d);
}

/// Each kind of change to the live rack sets `modified` at once; the same value again, or
/// time passing, doesn't.
#[test]
fn every_kind_of_change_sets_modified() {
    let d = dir("modified");
    type Change = Box<dyn Fn(&Session)>;
    let cmd = |c: AppCmd| -> Change { Box::new(move |s: &Session| s.send(c.clone()).unwrap()) };
    #[cfg_attr(not(feature = "plugins"), allow(unused_mut))]
    let mut changes: Vec<(&str, Change)> = vec![
        ("level", cmd(PartsCmd::SetPartVolume { part: 0, volume: 11 }.into())),
        ("pan", cmd(PartsCmd::SetPartPan { part: 1, pan: 10 }.into())),
        ("send", cmd(PartsCmd::SetPartSend { part: 2, send: PartSend::Variation, value: 70 }.into())),
        ("octave", cmd(PartsCmd::SetPartOctave { part: 3, octave: -1 }.into())),
        ("part switch", cmd(PartsCmd::SetPartOn { part: 2, on: true }.into())),
        ("sound", cmd(PartsCmd::SetPartVoice { part: 0, program: 40 }.into())),
        ("bend range", cmd(ControllersCmd::SetBendRange { part: 0, semitones: 12 }.into())),
        ("split", cmd(ChordCmd::SetSplit { note: 48 }.into())),
        ("transpose", cmd(ChordCmd::SetTranspose { keyboard: -2, master: 0 }.into())),
        ("harmony/arp switch", cmd(HarmonyArpCmd::SetHarmonyArpOn { on: true }.into())),
        ("harmony type", cmd(HarmonyArpCmd::SetHarmonyType { index: 5 }.into())),
        (
            "controller map",
            Box::new(|s: &Session| {
                s.inner.lock().rack_controls.knobs[6] = ControlTarget::SplitPoint;
                s.advance(1);
            }),
        ),
    ];
    #[cfg(feature = "plugins")]
    changes.push((
        "plugin edit",
        Box::new(|s: &Session| {
            s.inner.lock().plugins.channels[crate::parts::CHANNEL[2] as usize].as_mut().unwrap().edited = true;
            s.advance(1);
        }),
    ));
    for (what, change) in &changes {
        let s = session_with(&d, false);
        if *what == "plugin edit" {
            // Right 3 plays a library sound on a plugin (not installed: kept as failed).
            put_fake_plugin(&s);
            let tag = crate::patches::SoundTag { id: "saved:mine".into(), name: "Mine".into() };
            s.inner.lock().adopt_channel_sound(crate::parts::CHANNEL[2], Some(tag));
        }
        s.live_rack_clean("Ballad", Some("r1".into()));
        s.advance(QUIET_NS);
        assert!(!live(&s).modified, "{what}: not before the change");
        // The same values again are no change.
        s.send(PartsCmd::SetPartVolume { part: 0, volume: s.state().keyboard_parts[0].volume }).unwrap();
        assert!(!live(&s).modified, "{what}: setting a value to what it is changes nothing");
        change(&s);
        assert!(live(&s).modified, "{what} sets modified");
        assert_eq!((live(&s).name.as_str(), live(&s).id.as_deref()), ("Ballad", Some("r1")), "{what}: still the same rack");
    }
    let _ = std::fs::remove_dir_all(&d);
}
