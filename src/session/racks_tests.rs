//! Racks and the session: capture, write, read and apply, through offline sessions.

use crate::api::*;
use crate::patches::PatchSource;
use crate::racks::{self, ControlTarget, Rack, SoundRef};
use crate::session::testing::session_in as session;
use crate::session::Session;

fn data_dir(test: &str) -> std::path::PathBuf {
    crate::session::testing::data_dir(&format!("rack-{test}"))
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

/// #247: `setPartEq` sets a part's channel-strip EQ (clamped), the state shows it, a rack
/// captures and applies it, and a rack without one (an OTS's XG EQ in its voice settings
/// included) plays flat.
#[test]
fn a_rack_carries_the_part_eq() {
    let (d1, d2) = (data_dir("eq-from"), data_dir("eq-to"));
    let a = session(&d1);
    let b = session(&d2);
    let eq = PartEq { low_gain: -5, low_freq: 160, high_gain: 30, high_freq: 6_300 };
    a.send(PartsCmd::SetPartEq { part: 2, eq }).unwrap();
    let want = PartEq { high_gain: 12, ..eq };
    assert_eq!(a.state().keyboard_parts[2].eq, want, "clamped");
    assert_eq!(a.state().keyboard_parts[0].eq, PartEq::FLAT);
    let rack = a.capture_rack("Tone");
    assert_eq!(rack.parts[2].eq, want);
    assert!(b.apply_rack(&rack).is_empty());
    assert_eq!(b.state().keyboard_parts[2].eq, want, "applied");
    // A rack with no EQ: flat, even with an XG part EQ in its voice settings.
    let mut old = rack.clone();
    old.parts[2].eq = PartEq::FLAT;
    old.parts[2].tone.xg = vec![[0x08, 0x72, 0x4C]];
    assert!(b.apply_rack(&old).is_empty());
    assert_eq!(b.state().keyboard_parts[2].eq, PartEq::FLAT);
    let _ = std::fs::remove_dir_all(&d1);
    let _ = std::fs::remove_dir_all(&d2);
}

/// A keyboard part's insert slot: its three commands set it (the amount at most 127), the
/// state shows it, a rack captures and applies it, and a rack without one turns it off.
#[test]
fn a_rack_carries_the_insert_slot() {
    let (d1, d2) = (data_dir("insert-from"), data_dir("insert-to"));
    let a = session(&d1);
    let b = session(&d2);
    a.send(PartsCmd::SetKeyboardInsertEffect { part: 3, effect: InsertEffect::AutoWah }).unwrap();
    a.send(PartsCmd::SetKeyboardInsertOn { part: 3, on: true }).unwrap();
    a.send(PartsCmd::SetKeyboardInsertAmount { part: 3, amount: 200 }).unwrap();
    let want = PartInsert { effect: InsertEffect::AutoWah, on: true, amount: 127 };
    assert_eq!(a.state().keyboard_parts[3].insert, want);
    assert_eq!(a.state().keyboard_parts[0].insert, PartInsert::OFF);
    let rack = a.capture_rack("Wah");
    assert_eq!(rack.parts[3].insert, want);
    assert!(b.apply_rack(&rack).is_empty());
    assert_eq!(b.state().keyboard_parts[3].insert, want, "applied");
    let mut old = rack.clone();
    old.parts[3].insert = PartInsert::OFF;
    assert!(b.apply_rack(&old).is_empty());
    assert_eq!(b.state().keyboard_parts[3].insert, PartInsert::OFF, "a rack without one");
    let _ = std::fs::remove_dir_all(&d1);
    let _ = std::fs::remove_dir_all(&d2);
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

/// Every keyboard part's strip (EQ, compressor, both inserts, sends 4-6), the added sends
/// and an override of send 1, set through the strip commands.
fn set_strips(s: &Session) {
    use crate::fx::{InsertType, SendKind};
    let cmds = [
        StripCmd::SetStripEq { strip: 1, eq: PartEq { low_gain: -4, low_freq: 150, high_gain: 5, high_freq: 8_000 } },
        StripCmd::SetStripCompressorOn { strip: 0, on: true },
        StripCmd::SetStripCompressorPreset { strip: 0, preset: CompPreset::Loud },
        StripCmd::SetStripCompressorParam { strip: 0, param: PartCompParam::Makeup, value: 12 },
        StripCmd::SetStripCompressorOn { strip: 3, on: true },
        StripCmd::SetStripInsertKind { strip: 2, slot: 0, kind: InsertType::Rotary },
        StripCmd::SetStripInsertOn { strip: 2, slot: 0, on: true },
        StripCmd::SetStripInsertSetting { strip: 2, slot: 0, setting: 0, value: 100 },
        StripCmd::SetStripInsertSetting { strip: 2, slot: 0, setting: 2, value: 30 },
        StripCmd::SetStripInsertKind { strip: 1, slot: 1, kind: InsertType::Phaser },
        StripCmd::SetStripInsertOn { strip: 1, slot: 1, on: true },
        StripCmd::SetStripInsertSetting { strip: 1, slot: 1, setting: 1, value: 300 },
        StripCmd::AddSend { kind: SendKind::Phaser },
        StripCmd::AddSend { kind: SendKind::Stage },
        StripCmd::SetSendReturn { send: 4, level: 80 },
        StripCmd::SetStripSend { strip: 0, send: 3, level: 70 },
        StripCmd::SetStripSend { strip: 3, send: 4, level: 20 },
        StripCmd::SetRackSendOverride { send: 0, on: true },
        StripCmd::SetSendKind { send: 0, kind: SendKind::Room },
        StripCmd::SetSendReturn { send: 0, level: 90 },
    ];
    for c in cmds {
        s.send(c.clone()).unwrap_or_else(|e| panic!("{c:?}: {e:?}"));
    }
}

/// Capture then apply into another session: each keyboard part's strip, the added sends
/// and the override come back exactly.
#[test]
fn a_rack_carries_the_strips_and_send_effects() {
    let (d1, d2) = (data_dir("strips-from"), data_dir("strips-to"));
    let a = session(&d1);
    let b = session(&d2);
    set_strips(&a);
    let rack = a.capture_rack("Strips");
    assert_eq!(rack.parts[0].strip.comp.map(|c| (c.on, c.preset, c.makeup)), Some((true, CompPreset::Loud, 12)));
    assert_eq!(rack.parts[1].strip.comp, None, "a default compressor isn't saved");
    assert_eq!(rack.parts[2].strip.inserts[0].values[..3], [100, crate::fx::InsertType::Rotary.defaults()[1], 30]);
    assert_eq!(rack.parts[1].strip.inserts[1].values[1], 300);
    assert_eq!((rack.parts[0].strip.sends[3], rack.parts[3].strip.sends[4]), (70, 20));
    assert_eq!(rack.sends.added.len(), 2);
    let over = rack.sends.override_[0].as_ref().expect("send 1 overridden");
    assert_eq!((over.kind.clone(), over.return_level), (crate::fx::SendKind::Room, 90));
    assert_eq!(rack.sends.override_[1..], [None, None]);

    let problems = b.apply_rack(&rack);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(contents(&b.capture_rack("Strips")), contents(&rack), "captured again: the same rack");
    let (sa, sb) = (a.state(), b.state());
    for p in 0..4 {
        assert_eq!(sb.keyboard_parts[p].strip, sa.keyboard_parts[p].strip, "part {p}");
    }
    assert_eq!(sb.effects.sends, sa.effects.sends);
    assert!(sb.effects.sends[0].set_by_rack);
    let _ = std::fs::remove_dir_all(&d1);
    let _ = std::fs::remove_dir_all(&d2);
}

/// A rack saved before the strips (no `strip`, no `sends`) plays the keyboard parts flat
/// and drops the added sends and the override; the Style parts' strips stay.
#[test]
fn a_rack_from_before_the_strips_loads_flat() {
    use crate::fx::InsertType;
    let d = data_dir("strips-v1");
    let s = session(&d);
    set_strips(&s);
    s.send(StripCmd::SetStripInsertKind { strip: 7, slot: 1, kind: InsertType::Rotary }).unwrap();
    s.send(StripCmd::SetStripCompressorOn { strip: 7, on: true }).unwrap();
    let rack = s.capture_rack("Strips");

    // As a file saved before them: no `strip` on any part, no `sends`.
    let mut v: serde_json::Value = serde_json::from_str(&rack.to_json()).unwrap();
    assert!(v["sends"].is_object() && v["parts"][0]["strip"].is_object(), "the new rack writes them");
    v.as_object_mut().unwrap().remove("sends");
    for p in v["parts"].as_array_mut().unwrap() {
        p.as_object_mut().unwrap().remove("strip");
    }
    let old = Rack::from_json(&v.to_string()).unwrap();
    assert!(old.sends.is_empty());

    let problems = s.apply_rack(&old);
    assert!(problems.is_empty(), "{problems:?}");
    let st = s.state();
    for (p, kp) in st.keyboard_parts.iter().enumerate() {
        let strip = &kp.strip;
        assert!(!strip.comp.on, "part {p}: compressor off");
        assert_eq!(strip.inserts[1].kind, InsertType::None, "part {p}: insert 2 empty");
        assert_eq!(strip.sends[3..], [0, 0, 0], "part {p}: no sends 4-6");
    }
    assert_eq!(st.keyboard_parts[2].strip.inserts[0].kind, InsertType::Rotary, "insert 1 is the part's insert");
    assert_eq!(st.keyboard_parts[2].strip.inserts[0].settings[2].value, InsertType::Rotary.defaults()[2], "its later settings at their defaults");
    assert_eq!(st.effects.sends.len(), 3, "no added sends");
    assert!(st.effects.sends.iter().all(|s| !s.set_by_rack), "no override");
    assert_eq!(st.effects.blocks[0].effect, FxType::Hall, "send 1 back to the style's");
    let style = &st.mixer.style_parts[3].strip;
    assert_eq!((style.inserts[1].kind.clone(), style.comp.on), (InsertType::Rotary, true), "the Style strip stays");

    // The same rack as a value (strip and sends at their defaults) loads the same way.
    set_strips(&s);
    let mut flat = rack.clone();
    for p in &mut flat.parts {
        p.strip = Default::default();
    }
    flat.sends = Default::default();
    assert!(s.apply_rack(&flat).is_empty());
    assert_eq!(s.state().keyboard_parts, st.keyboard_parts);
    assert_eq!(s.state().effects.sends, st.effects.sends);
    let _ = std::fs::remove_dir_all(&d);
}
