//! Racks and the session: capture, write, read and apply, through offline sessions.

use crate::api::*;
use crate::patches::PatchSource;
use crate::racks::{self, ControlTarget, Rack, SoundRef};
use crate::session::{Options, Session};
use std::path::{Path, PathBuf};

fn data_dir(test: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("yahaha-rack-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn chunk(id: &[u8], body: &[u8]) -> Vec<u8> {
    let mut v = id.to_vec();
    v.extend_from_slice(&(body.len() as u32).to_be_bytes());
    v.extend_from_slice(body);
    v
}

/// A session on a tiny synthetic style written at run time (one Main A bar), so these tests
/// run without the git-ignored corpus: capturing and applying keyboard parts needs no real
/// style.
fn session(data: &Path) -> Session {
    let mut trk = vec![0x00, 0xFF, 0x06, 4];
    trk.extend_from_slice(b"SFF2");
    trk.extend_from_slice(&[0x00, 0xFF, 0x06, 6]);
    trk.extend_from_slice(b"Main A");
    trk.extend_from_slice(&[0x00, 0x9B, 60, 100, 0x83, 0x00, 0x8B, 60, 0, 0x00, 0xFF, 0x2F, 0]);
    let mut bytes = chunk(b"MThd", &[0, 0, 0, 1, 0, 96]);
    bytes.extend(chunk(b"MTrk", &trk));
    bytes.extend(chunk(b"CASM", &chunk(b"CSEG", &chunk(b"Sdec", b"Main A"))));
    let style = data.join("styles/Test.sty");
    std::fs::create_dir_all(style.parent().unwrap()).unwrap();
    std::fs::write(&style, bytes).unwrap();
    Session::offline(Options { paths: vec![style], data_dir: Some(data.to_path_buf()), ..Options::default() }).unwrap()
}

/// A SoundFont library patch (a sound carries no mix).
fn add_bass_patch(s: &Session) -> String {
    let source = PatchSource::SoundFont { file: "Other.sf2".into(), bank: 0, program: 33 };
    let patch = PatchFields { name: "My Bass".into(), category: Default::default(), tags: Vec::new(), favourite: false, source };
    s.send(SoundLibraryCmd::CreatePatch { patch }).unwrap();
    let st = s.state();
    st.sound_library.patches.iter().find(|p| p.patch.name == "My Bass").unwrap().patch.id.clone()
}

/// The keyboard parts as the app shows them: on, program, patch, volume, octave, pan and
/// sends.
fn parts(s: &Session) -> Vec<(bool, u8, Option<String>, u8, i8, u8, u8, u8, u8)> {
    s.state().keyboard_parts.iter().map(|p| (p.on, p.program, p.patch.clone(), p.volume, p.octave, p.pan, p.reverb, p.chorus, p.variation)).collect()
}

/// Everything a rack holds but its id.
fn contents(r: &Rack) -> Rack {
    Rack { id: String::new(), ..r.clone() }
}

#[test]
fn capture_write_read_apply_round_trips() {
    let (d1, d2) = (data_dir("rt-from"), data_dir("rt-to"));
    let a = session(&d1);
    let b = session(&d2);
    let bass = add_bass_patch(&a);
    assert_eq!(add_bass_patch(&b), bass, "the same library sound in both");

    a.send(PartsCmd::SetPartVoice { part: 0, program: 4 }).unwrap();
    a.send(PartsCmd::SetPartVolume { part: 0, volume: 77 }).unwrap();
    a.send(PartsCmd::SetPartPan { part: 0, pan: 30 }).unwrap();
    a.send(PartsCmd::SetPartSend { part: 0, send: PartSend::Reverb, value: 90 }).unwrap();
    a.send(PartsCmd::SetPartSend { part: 0, send: PartSend::Variation, value: 12 }).unwrap();
    a.send(PartsCmd::SetPartOctave { part: 0, octave: 1 }).unwrap();
    a.send(PartsCmd::SetPartOn { part: 1, on: true }).unwrap();
    a.send(PartsCmd::SetPartVoice { part: 1, program: 48 }).unwrap();
    a.send(PartsCmd::SetPartVolume { part: 1, volume: 50 }).unwrap();
    // Left plays the library patch, with the rack's own mix.
    a.send(SoundLibraryCmd::SetPartPatch { part: 3, id: Some(bass.clone()) }).unwrap();
    a.send(PartsCmd::SetPartVolume { part: 3, volume: 99 }).unwrap();
    a.send(PartsCmd::SetPartPan { part: 3, pan: 70 }).unwrap();
    a.send(PartsCmd::SetPartSend { part: 3, send: PartSend::Chorus, value: 44 }).unwrap();
    a.send(PartsCmd::SetPartOctave { part: 3, octave: -1 }).unwrap();
    a.send(ControllersCmd::SetBendRange { part: 0, semitones: 7 }).unwrap();
    a.send(ChordCmd::SetSplit { note: 60 }).unwrap();
    a.send(ChordCmd::SetTranspose { keyboard: 3, master: 0 }).unwrap();
    a.send(HarmonyArpCmd::SetHarmonyType { index: 2 }).unwrap();
    a.send(HarmonyArpCmd::SetHarmonyVolume { volume: 66 }).unwrap();
    a.send(HarmonyArpCmd::SetHarmonyArpOn { on: true }).unwrap();
    a.inner.lock().shared.parts.set_tone(0, [Some(90), None, Some(20), None, None, None, None, None, None, None], [(0x08, 0x0E, 1)]);

    let mut rack = a.capture_rack("Ballad");
    assert_eq!(rack.parts[3].sound, SoundRef::Library { id: bass.clone() });
    assert!(matches!(&rack.parts[0].sound, SoundRef::Font { bank: 0, program: 4, .. }));
    assert_eq!((rack.split, rack.transpose), (60, 3));
    // A map of the player's own, to come back too.
    rack.controls.knobs[5] = ControlTarget::SplitPoint;
    rack.controls.faders[2] = ControlTarget::PartChorus { part: 1 };

    let path = racks::path_for(&racks::dir(&d1), &rack.name);
    rack.save(&path).unwrap();
    let read = Rack::load(&path).unwrap();
    assert_eq!(read, rack);

    assert_ne!(parts(&b), parts(&a), "the second session starts elsewhere");
    let problems = b.apply_rack(&read);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(parts(&b), parts(&a), "the parts sound and mix as captured");
    let bl = &b.state().keyboard_parts[3];
    assert_eq!((bl.volume, bl.pan, bl.octave), (99, 70, -1), "the rack's mix");
    let (sa, sb) = (a.state(), b.state());
    assert_eq!((sb.chord.split, sb.chord.transpose_keyboard), (60, 3));
    assert_eq!(sb.harmony_arp, sa.harmony_arp);
    assert_eq!(contents(&b.capture_rack("Ballad")), contents(&rack), "captured again: the same rack");
    let _ = std::fs::remove_dir_all(&d1);
    let _ = std::fs::remove_dir_all(&d2);
}

/// A rack whose plugin isn't installed, and whose library sound is gone: those parts keep
/// their mix, the problem is reported, and everything else applies.
#[test]
fn a_missing_sound_leaves_the_part_failed_with_its_mix() {
    let d = data_dir("missing");
    let s = session(&d);
    let mut rack = s.capture_rack("Missing");
    rack.parts[0].volume = 81;
    rack.parts[1].sound = SoundRef::Plugin { component: "aumu zzzz nope".into() };
    rack.parts[1].fallback_program = Some(48);
    (rack.parts[1].volume, rack.parts[1].pan, rack.parts[1].reverb, rack.parts[1].octave) = (33, 20, 70, 1);
    rack.parts[2].sound = SoundRef::Library { id: "gone".into() };
    (rack.parts[2].volume, rack.parts[2].chorus) = (44, 55);
    rack.split = 50;

    let problems = s.apply_rack(&rack);
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(problems[0].starts_with("Right 2"), "{problems:?}");
    assert!(problems[1].starts_with("Right 3"), "{problems:?}");
    let st = s.state();
    let p = &st.keyboard_parts;
    assert_eq!((p[1].volume, p[1].pan, p[1].reverb, p[1].octave), (33, 20, 70, 1), "Right 2 keeps its mix");
    assert_eq!((p[2].volume, p[2].chorus), (44, 55), "Right 3 keeps its mix");
    assert_eq!(p[0].volume, 81, "the other parts apply");
    assert_eq!(st.chord.split, 50, "and the rest of the rack");
    if cfg!(feature = "plugins") {
        let plugin = p[1].plugin.as_ref().expect("the missing plugin stays on the part");
        assert_eq!((plugin.id.as_str(), plugin.status), ("aumu zzzz nope", PluginStatus::Failed));
        let again = s.capture_rack("Again");
        assert_eq!(again.parts[1].sound, rack.parts[1].sound, "captured again, it is still the rack's");
    }
    let _ = std::fs::remove_dir_all(&d);
}
