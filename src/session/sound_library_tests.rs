//! The sound library through an offline session: commands, state, persistence, the route
//! table the synth reads, and the style hand-off.

use super::SoundLib;
use crate::api::*;
use crate::patches;
use crate::session::{Options, Session};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering::Relaxed;

const MS: u64 = 1_000_000;
/// The tiny SoundFonts the tests make (`patches::sf2::tiny_gm_sound_font`): the synth's,
/// and a second one for library patches.
const SF2: &str = "Test.sf2";
const OTHER: &str = "Other.sf2";

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// A fresh data folder with a SoundFont folder in it (`<data>/sf`) holding the two tiny
/// test SoundFonts, whatever SoundFonts the checkout has.
fn folder(tag: &str) -> PathBuf {
    let data = std::env::temp_dir().join(format!("yahaha-sl-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data);
    std::fs::create_dir_all(data.join("sf")).unwrap();
    for f in [SF2, OTHER] {
        std::fs::write(data.join("sf").join(f), patches::sf2::tiny_gm_sound_font()).unwrap();
    }
    data
}

/// An offline session on `styles` (corpus/MOX_v2 file names) with a fresh data folder,
/// and the test SoundFont folder when `sf2` (so patches there are playable). The first
/// style is loaded explicitly (the library sorts its entries).
fn session(tag: &str, styles: &[&str], sf2: bool) -> Option<(Session, PathBuf)> {
    let paths: Vec<PathBuf> = styles.iter().map(|s| root().join("corpus/MOX_v2").join(s)).collect();
    if paths.iter().any(|p| !p.exists()) {
        eprintln!("corpus missing; skipping");
        return None;
    }
    let data = folder(tag);
    let opts = Options { paths, data_dir: Some(data.clone()), sf2: sf2.then(|| data.join("sf").join(SF2)), ..Options::default() };
    let s = Session::offline(opts).unwrap();
    load(&s, styles[0]);
    Some((s, data))
}

/// Load the style whose path contains `name`, and let the engine take it over.
fn load(s: &Session, name: &str) {
    let id = s.library_list().entries.iter().find(|e| e.path.contains(name)).unwrap().id;
    s.send(LibraryCmd::LoadStyle { id }).unwrap();
    s.advance(10 * MS);
    assert!(s.state().style.path.contains(name));
}

fn fields(name: &str, bank: u16, program: u8) -> PatchFields {
    PatchFields {
        name: name.into(),
        category: patches::Category::guess(bank, program),
        tags: vec![],
        favourite: false,
        source: PatchSource::SoundFont { file: SF2.into(), bank, program },
    }
}

fn add(s: &Session, name: &str, bank: u16, program: u8) -> String {
    s.send(SoundLibraryCmd::CreatePatch { patch: fields(name, bank, program) }).unwrap();
    s.state().sound_library.last_added.clone().unwrap()
}

#[test]
fn rules_resolve_the_styles_programs_and_persist() {
    let Some((s, data)) = session("rules", &["SlowWalker.T552.sty"], true) else { return };
    let st = s.state();
    assert!(st.sound_library.patches.is_empty());
    assert_eq!(st.sound_library.families.len(), 16);
    assert_eq!(st.sound_library.categories.len(), 13);
    assert_eq!(st.sound_library.style_key, "SlowWalker.T552.sty");
    let usage = &st.sound_library.usage;
    assert!(!usage.is_empty(), "the style sends programs");
    assert!(usage.iter().all(|u| u.rule == RuleKind::Fallback && u.patch.is_none()), "no map yet: everything falls back");
    assert!(usage.iter().all(|u| (9..=16).contains(&u.channel)));

    let bass = add(&s, "My Bass", 0, 34);
    let kit = add(&s, "My Kit", 128, 0);
    s.send(SoundLibraryCmd::SetFamilyRule { family: 4, patch: Some(bass.clone()), style: false }).unwrap();
    s.send(SoundLibraryCmd::SetDrumRule { patch: Some(kit.clone()), style: false }).unwrap();
    let st = s.state();
    let sl = &st.sound_library;
    assert_eq!(sl.map.families[4].as_deref(), Some(bass.as_str()));
    for u in &sl.usage {
        if u.drums {
            assert_eq!((u.patch.as_deref(), u.rule, u.plays.as_str()), (Some(kit.as_str()), RuleKind::Drums, "My Kit"), "{u:?}");
        } else if (32..40).contains(&u.gm_program) {
            assert_eq!((u.patch.as_deref(), u.rule), (Some(bass.as_str()), RuleKind::Family), "{u:?}");
        } else {
            assert_eq!(u.rule, RuleKind::Fallback, "{u:?}");
        }
    }
    assert!(sl.usage.iter().any(|u| u.channel == 11 && u.patch.as_deref() == Some(bass.as_str())), "the Bass part plays My Bass");
    assert!(sl.patches.iter().all(|p| p.available), "the SoundFont is in the folder");

    // The table the synth reads: bank 0 (the first style's), program 33 and the drums.
    let routes = s.inner.shared.routes.clone();
    let cur = routes.current.load(Relaxed);
    let r = routes.bank_route(cur, 33).expect("program 33 routes");
    assert_eq!((r.bank, r.program), (0, 34));
    assert!(matches!(r.source, patches::Source::SoundFont(_)));
    assert_eq!(routes.bank_drum(cur).map(|r| r.bank), Some(128));
    assert_eq!(routes.bank_route(cur, 40), None, "Strings: no rule");

    // Saved; a new session reads it back.
    let file = data.join(patches::FILE_NAME);
    assert_eq!(sl.file.as_deref(), Some(file.display().to_string().as_str()));
    drop(s);
    let (s2, _) = {
        let opts = Options { paths: vec![root().join("corpus/MOX_v2/SlowWalker.T552.sty")], data_dir: Some(data.clone()), ..Options::default() };
        (Session::offline(opts).unwrap(), ())
    };
    let st2 = s2.state();
    assert_eq!(st2.sound_library.patches.len(), 2);
    assert_eq!(st2.sound_library.map.drums.as_deref(), Some(kit.as_str()));
    // Deleting a patch drops the rules naming it.
    s2.send(SoundLibraryCmd::DeletePatch { id: bass.clone() }).unwrap();
    assert_eq!(s2.state().sound_library.map.families[4], None);
    assert!(s2.send(SoundLibraryCmd::DeletePatch { id: bass }).is_err());
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn edits_reorder_duplicate_and_refuse_bad_input() {
    let Some((s, data)) = session("edit", &["SlowWalker.T552.sty"], false) else { return };
    let a = add(&s, "Pad A", 0, 89);
    let b = add(&s, "Pad A", 0, 90);
    assert_ne!(a, b, "ids stay unique");
    s.send(SoundLibraryCmd::DuplicatePatch { id: a.clone() }).unwrap();
    let dup = s.state().sound_library.last_added.clone().unwrap();
    let order = |s: &Session| s.state().sound_library.patches.iter().map(|p| p.patch.id.clone()).collect::<Vec<_>>();
    assert_eq!(order(&s), [a.clone(), dup.clone(), b.clone()]);
    s.send(SoundLibraryCmd::MovePatch { id: b.clone(), to: 0 }).unwrap();
    assert_eq!(order(&s), [b.clone(), a.clone(), dup.clone()]);
    s.send(SoundLibraryCmd::SetPatchFavourite { id: a.clone(), favourite: true }).unwrap();
    let mut f = fields("Warm Pad", 0, 89);
    f.tags = vec!["warm".into()];
    s.send(SoundLibraryCmd::UpdatePatch { id: a.clone(), patch: f }).unwrap();
    let st = s.state();
    let p = st.sound_library.patches.iter().find(|p| p.patch.id == a).unwrap();
    assert_eq!((p.patch.name.as_str(), p.patch.tags.len()), ("Warm Pad", 1));
    assert!(!p.available && p.note.as_deref().unwrap().contains("not in the SoundFont folder"), "no SoundFont folder here");
    // Bad input is refused and nothing changes.
    assert!(s.send(SoundLibraryCmd::SetFamilyRule { family: 16, patch: None, style: false }).is_err());
    assert!(s.send(SoundLibraryCmd::SetProgramOverride { program: 5, patch: Some("nope".into()), style: false }).is_err());
    assert!(s.send(SoundLibraryCmd::BrowseSoundFont { file: Some("../x.sf2".into()) }).is_err());
    s.send(SoundLibraryCmd::SetProgramOverride { program: 5, patch: Some(a.clone()), style: false }).unwrap();
    assert_eq!(s.state().sound_library.map.overrides.len(), 1);
    s.send(SoundLibraryCmd::SetProgramOverride { program: 5, patch: None, style: false }).unwrap();
    assert!(s.state().sound_library.map.overrides.is_empty());
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_styles_own_map_wins_and_stays_with_the_style() {
    let Some((s, data)) = session("style", &["SlowWalker.T552.sty", "BubblyDub.T552.sty"], true) else { return };
    let g = add(&s, "Global Bass", 0, 33);
    let own = add(&s, "Slow Bass", 0, 35);
    s.send(SoundLibraryCmd::SetFamilyRule { family: 4, patch: Some(g.clone()), style: false }).unwrap();
    s.send(SoundLibraryCmd::SetFamilyRule { family: 4, patch: Some(own.clone()), style: true }).unwrap();
    let st = s.state();
    let key = st.sound_library.style_key.clone();
    assert_eq!(st.sound_library.style_map.families[4].as_deref(), Some(own.as_str()));
    let bass = st.sound_library.usage.iter().find(|u| u.channel == 11).unwrap();
    assert_eq!((bass.patch.as_deref(), bass.from_style), (Some(own.as_str()), true));
    let routes = s.inner.shared.routes.clone();
    let cur = routes.current.load(Relaxed);
    assert_eq!(routes.bank_route(cur, 33).map(|r| r.program), Some(35), "the style's rule in its bank");

    // Another style: it gets the other bank, written with the global map, before the
    // engine takes it over.
    let other = s.library_list().entries.iter().find(|e| e.path.contains("BubblyDub")).unwrap().id;
    s.send(LibraryCmd::LoadStyle { id: other }).unwrap();
    s.advance(10 * MS);
    let st = s.state();
    assert_ne!(st.sound_library.style_key, key);
    assert!(st.sound_library.style_map.is_empty());
    let cur2 = routes.current.load(Relaxed);
    assert_ne!(cur2, cur, "the other bank");
    assert_eq!(routes.bank_route(cur2, 33).map(|r| r.program), Some(33), "the global rule");
    assert!(st.sound_library.usage.iter().filter(|u| (32..40).contains(&u.gm_program) && !u.drums).all(|u| u.patch.as_deref() == Some(g.as_str())));
    // Back: its own map again, in bank 0.
    let first = s.library_list().entries.iter().find(|e| e.path.contains("SlowWalker")).unwrap().id;
    s.send(LibraryCmd::LoadStyle { id: first }).unwrap();
    s.advance(10 * MS);
    let cur3 = routes.current.load(Relaxed);
    assert_eq!(routes.bank_route(cur3, 33).map(|r| r.program), Some(35));
    assert_eq!(s.state().sound_library.style_map.families[4].as_deref(), Some(own.as_str()));
    s.send(SoundLibraryCmd::ClearStyleMap).unwrap();
    assert!(s.state().sound_library.style_map.is_empty());
    assert_eq!(routes.bank_route(cur3, 33).map(|r| r.program), Some(33));
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_keyboard_part_takes_its_patch_and_keeps_its_mix() {
    let Some((s, data)) = session("part", &["SlowWalker.T552.sty"], true) else { return };
    s.send(SoundLibraryCmd::CreatePatch { patch: fields("Stage Piano", 0, 1) }).unwrap();
    let id = s.state().sound_library.last_added.clone().unwrap();
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 55 }).unwrap();
    s.send(PartsCmd::SetPartOctave { part: 0, octave: -1 }).unwrap();
    s.send(SoundLibraryCmd::SetPartPatch { part: 0, id: Some(id.clone()) }).unwrap();
    let st = s.state();
    let r1 = &st.keyboard_parts[0];
    assert_eq!((r1.patch.as_deref(), r1.voice_name.as_str(), r1.volume, r1.octave), (Some(id.as_str()), "Stage Piano", 55, -1));
    let routes = s.inner.shared.routes.clone();
    assert_eq!(routes.part(0).map(|r| r.program), Some(1));
    // A GM voice picked for the part: its own patch goes.
    s.send(PartsCmd::SetPartVoice { part: 0, program: 0 }).unwrap();
    assert_eq!(s.state().keyboard_parts[0].patch, None);
    assert_eq!(routes.part(0), None);
    // A GM voice goes through the map like a Style part's.
    s.send(SoundLibraryCmd::SetFamilyRule { family: 0, patch: Some(id.clone()), style: false }).unwrap();
    assert_eq!(s.state().keyboard_parts[0].voice_name, "Stage Piano");
    // Save the part's sound as a new patch.
    s.send(SoundLibraryCmd::SavePartAsPatch { part: 1, name: Some("My Strings".into()) }).unwrap();
    let st = s.state();
    let p = st.sound_library.patches.last().unwrap();
    assert_eq!(p.patch.name, "My Strings");
    assert_eq!(p.patch.source, PatchSourceView::SoundFont { file: SF2.into(), bank: 0, program: 48 });
    let _ = std::fs::remove_dir_all(&data);
}

/// One save makes one record (docs/racks.md "Saving"): Save on a part playing a GM voice
/// makes one sound named after the voice, which the part then plays, so saving it again
/// updates that sound rather than adding copies. The same for a voice the map sends to a
/// patch: the first Save copies it once.
#[test]
fn saving_a_part_again_and_again_makes_one_record() {
    let Some((s, data)) = session("save-once", &["SlowWalker.T552.sty"], true) else { return };
    let n = s.state().sound_library.patches.len();
    let program = s.state().keyboard_parts[1].program;
    for _ in 0..3 {
        s.send(SoundLibraryCmd::SaveSound { part: 1 }).unwrap();
    }
    let st = s.state();
    assert_eq!(st.sound_library.patches.len(), n + 1, "one record for three saves");
    let p = &st.sound_library.patches.last().unwrap().patch;
    assert_eq!((p.name.as_str(), &p.source), (gm_name(program), &PatchSourceView::SoundFont { file: SF2.into(), bank: 0, program }));
    assert_eq!(st.keyboard_parts[1].patch.as_deref(), Some(p.id.as_str()), "the part plays the new sound");
    assert_eq!(s.inner.shared.routes.part(1).map(|r| r.program), Some(program));

    // A GM voice the map sends to a patch: Save copies it once.
    let mapped = add(&s, "Lush Strings", 0, 50);
    let family = s.state().keyboard_parts[2].program / 8;
    s.send(SoundLibraryCmd::SetFamilyRule { family, patch: Some(mapped.clone()), style: false }).unwrap();
    let n = s.state().sound_library.patches.len();
    s.send(SoundLibraryCmd::SaveSound { part: 2 }).unwrap();
    s.send(SoundLibraryCmd::SaveSound { part: 2 }).unwrap();
    let st = s.state();
    assert_eq!(st.sound_library.patches.len(), n + 1);
    let copy = &st.sound_library.patches.last().unwrap().patch;
    assert_ne!(copy.id, mapped);
    assert_eq!((copy.name.as_str(), st.keyboard_parts[2].patch.as_deref()), ("Lush Strings", Some(copy.id.as_str())));
    let _ = std::fs::remove_dir_all(&data);
}

/// `savePartAsPatch` saves what the part plays (#109): a GM voice the map sends to a patch
/// is saved as that patch, not as the raw GM program.
#[test]
fn saving_a_part_saves_the_patch_the_map_plays() {
    let Some((s, data)) = session("save-mapped", &["SlowWalker.T552.sty"], true) else { return };
    let strings = s.state().keyboard_parts[1].program;
    let mut f = fields("Lush Strings", 0, 50);
    f.source = PatchSource::SoundFont { file: OTHER.into(), bank: 0, program: 50 };
    f.tags = vec!["warm".into()];
    s.send(SoundLibraryCmd::CreatePatch { patch: f }).unwrap();
    let id = s.state().sound_library.last_added.clone().unwrap();
    s.send(SoundLibraryCmd::SetFamilyRule { family: strings / 8, patch: Some(id.clone()), style: false }).unwrap();
    assert_eq!(s.state().keyboard_parts[1].voice_name, "Lush Strings");
    s.send(SoundLibraryCmd::SavePartAsPatch { part: 1, name: None }).unwrap();
    let st = s.state();
    let p = &st.sound_library.patches.last().unwrap().patch;
    assert_ne!(p.id, id);
    assert_eq!((p.name.as_str(), &p.tags), ("Lush Strings", &vec!["warm".to_string()]));
    assert_eq!(p.source, PatchSourceView::SoundFont { file: OTHER.into(), bank: 0, program: 50 }, "the mapped patch's sound");
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn import_export_and_browse() {
    let Some((s, data)) = session("io", &["SlowWalker.T552.sty"], true) else { return };
    let a = add(&s, "A", 0, 1);
    s.send(SoundLibraryCmd::SetFamilyRule { family: 0, patch: Some(a.clone()), style: false }).unwrap();
    s.send(SoundLibraryCmd::ExportSoundLibrary { path: None }).unwrap();
    let out = data.join("sound-library-export.json");
    assert!(out.exists());
    s.send(SoundLibraryCmd::ImportSoundLibrary { path: out.display().to_string(), replace: false, maps: false }).unwrap();
    assert_eq!(s.state().sound_library.patches.len(), 2, "the import adds a copy under a new id");
    s.send(SoundLibraryCmd::ImportSoundLibrary { path: out.display().to_string(), replace: true, maps: false }).unwrap();
    assert_eq!(s.state().sound_library.patches.len(), 1);
    assert!(s.send(SoundLibraryCmd::ImportSoundLibrary { path: data.join("none.json").display().to_string(), replace: false, maps: false }).is_err());
    // Browse the SoundFont's presets and add one.
    s.send(SoundLibraryCmd::BrowseSoundFont { file: Some(SF2.into()) }).unwrap();
    let b = s.state().sound_library.browse.clone().unwrap();
    assert!(b.presets.len() > 128 && b.error.is_none());
    let p = b.presets.iter().find(|p| p.bank == 0 && p.program == 33).unwrap().clone();
    s.send(SoundLibraryCmd::AddPresetAsPatch { file: SF2.into(), bank: 0, program: 33, name: None }).unwrap();
    let st = s.state();
    let added = st.sound_library.patches.last().unwrap();
    assert_eq!((added.patch.name.as_str(), added.patch.category), (p.name.as_str(), patches::Category::Bass));
    s.send(SoundLibraryCmd::BrowseSoundFont { file: None }).unwrap();
    assert!(s.state().sound_library.browse.is_none());
    s.send(SoundLibraryCmd::SetPortSendsMapped { on: true }).unwrap();
    assert!(s.state().sound_library.port_sends_mapped && s.inner.shared.routes.port_mapped.load(Relaxed));
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn auditions_wait_for_a_stopped_band_and_end() {
    let Some((s, data)) = session("aud", &["SlowWalker.T552.sty"], true) else { return };
    let a = add(&s, "A", 0, 1);
    s.send(SoundLibraryCmd::AuditionPatch { id: a.clone() }).unwrap();
    assert_eq!(s.state().sound_library.auditioning.as_deref(), Some(a.as_str()));
    s.advance(4000 * MS);
    assert_eq!(s.state().sound_library.auditioning, None, "it ends by itself");
    s.send(SoundLibraryCmd::AuditionPreset { file: SF2.into(), bank: 128, program: 0 }).unwrap();
    assert_eq!(s.state().sound_library.auditioning.as_deref(), Some("preset"));
    s.send(SoundLibraryCmd::StopPatchAudition).unwrap();
    assert_eq!(s.state().sound_library.auditioning, None);
    s.send(TransportCmd::StartStop).unwrap();
    s.advance(10 * MS);
    assert!(s.send(SoundLibraryCmd::AuditionPatch { id: a }).is_err(), "not while the band plays");
    let _ = std::fs::remove_dir_all(&data);
}

/// A library file from a newer yahaha is not loaded and never saved over.
#[test]
fn a_newer_library_file_is_left_alone() {
    let Some((_, data)) = session("newer", &["SlowWalker.T552.sty"], false) else { return };
    std::fs::create_dir_all(&data).unwrap();
    let file = data.join(patches::FILE_NAME);
    std::fs::write(&file, r#"{"version": 99}"#).unwrap();
    let opts = Options { paths: vec![root().join("corpus/MOX_v2/SlowWalker.T552.sty")], data_dir: Some(data.clone()), ..Options::default() };
    let s = Session::offline(opts).unwrap();
    assert!(s.state().message.as_ref().unwrap().text.contains("newer"));
    s.send(SoundLibraryCmd::CreatePatch { patch: fields("X", 0, 0) }).unwrap();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), r#"{"version": 99}"#);
    let _ = std::fs::remove_dir_all(&data);
}

/// The generated style's parts (channels 9-16): bank MSB, program, and the CC7 its setup
/// sets (None: none, so a map rule's level may fill in).
const GEN_PARTS: [(u8, u8, Option<u8>); 8] = [
    (127, 0, None),     // Rhythm 1
    (127, 0, None),     // Rhythm 2
    (0, 33, None),      // Bass
    (0, 0, None),       // Chord 1
    (0, 25, None),      // Chord 2
    (0, 89, Some(45)),  // Pad
    (0, 61, Some(50)),  // Phrase 1
    (0, 73, None),      // Phrase 2
];

/// A minimal SFF2 style made here (no Yamaha data): its channel setup (GEN_PARTS) and one
/// bar of Main A, every part playing a note on beat 1.
fn gen_style_bytes() -> Vec<u8> {
    fn chunk(id: &[u8], body: &[u8]) -> Vec<u8> {
        let mut v = id.to_vec();
        v.extend_from_slice(&(body.len() as u32).to_be_bytes());
        v.extend_from_slice(body);
        v
    }
    let text = |s: &str| [&[0xFF, 0x06, s.len() as u8][..], s.as_bytes()].concat();
    let mut trk = Vec::new();
    for e in [text("SFF2"), text("SInt")] {
        trk.push(0);
        trk.extend(e);
    }
    for (i, &(msb, program, cc7)) in GEN_PARTS.iter().enumerate() {
        let ch = 8 + i as u8;
        trk.extend_from_slice(&[0, 0xB0 | ch, 0, msb, 0, 0xB0 | ch, 32, 0, 0, 0xC0 | ch, program]);
        if let Some(v) = cc7 {
            trk.extend_from_slice(&[0, 0xB0 | ch, 7, v]);
        }
    }
    // A bar (384 ticks at 96 PPQ) later: Main A, a quarter note on every part.
    trk.extend_from_slice(&[0x83, 0x00]);
    trk.extend(text("Main A"));
    for ch in 8..16u8 {
        trk.extend_from_slice(&[0, 0x90 | ch, 60, 100]);
    }
    for (i, ch) in (8..16u8).enumerate() {
        trk.extend_from_slice(&[if i == 0 { 0x60 } else { 0 }, 0x80 | ch, 60, 0]);
    }
    trk.extend_from_slice(&[0x82, 0x20, 0xFF, 0x2F, 0]);
    let mut out = chunk(b"MThd", &[0, 0, 0, 1, 0, 96]);
    out.extend(chunk(b"MTrk", &trk));
    out
}

/// A version 2 library (sounds with `defaults`) for the generated style: a kit, bass and
/// pad sound with volumes on global rules, a keys sound with a volume on the style's own
/// rule (`Gen.sty`), and a guitar sound with none.
const V2_LIBRARY: &str = r#"{
  "version": 2,
  "patches": [
    {"id": "kit", "name": "Kit", "source": {"kind": "soundFont", "file": "Test.sf2", "bank": 128, "program": 0}, "defaults": {"volume": 90, "octave": 0}},
    {"id": "bass", "name": "Bass", "source": {"kind": "soundFont", "file": "Test.sf2", "bank": 0, "program": 33}, "defaults": {"volume": 77, "pan": 20, "reverb": 10, "chorus": 5, "octave": -1}},
    {"id": "keys", "name": "Keys", "source": {"kind": "soundFont", "file": "Test.sf2", "bank": 0, "program": 4}, "defaults": {"volume": 66, "octave": 1}},
    {"id": "pad", "name": "Pad", "source": {"kind": "soundFont", "file": "Test.sf2", "bank": 0, "program": 89}, "defaults": {"volume": 55, "octave": 0}},
    {"id": "gtr", "name": "Guitar", "source": {"kind": "soundFont", "file": "Test.sf2", "bank": 0, "program": 25}, "defaults": {"volume": null, "octave": 0}}
  ],
  "map": {"families": [null, null, null, "gtr", "bass", null, null, null, null, null, null, "pad", null, null, null, null], "drums": "kit"},
  "styleMaps": {"Gen.sty": {"overrides": [{"program": 0, "patch": "keys"}]}}
}"#;

/// The Style part levels (CC7) the generated style gets from `lib` under style key `key`.
fn gen_style_levels(lib_dir: &Path, key: &str) -> [u8; 8] {
    let style = crate::sff::parse(&gen_style_bytes()).unwrap();
    let mut prep = crate::engine::Prepared::new(&style);
    let mut sl = SoundLib::open(Some(lib_dir));
    assert_eq!(sl.load_error(), None);
    sl.prepare(&mut prep, key);
    assert!(prep.setups.iter().all(|s| s.mix == prep.setups[0].mix));
    prep.setups[0].mix
}

/// Style part levels don't change (docs/racks.md "Migration"): a version 2 library's
/// sound volumes now come from the map rules that name the sounds, so every Style part
/// the style sets no level for plays at the level its sound's `defaults.volume` gave it
/// (global and per-style rules), and one the style sets keeps the style's.
#[test]
fn a_version_2_library_keeps_every_style_part_level() {
    let dir = folder("v2-levels");
    std::fs::write(dir.join(patches::FILE_NAME), V2_LIBRARY).unwrap();
    // The levels version 2 gave: the kit's 90 on both drum parts, the bass's 77, the keys'
    // 66 by the style's own override, no level on the guitar's rule (the GM default 100),
    // the pad's 55 losing to the style's own 45, and the style's 50.
    assert_eq!(gen_style_levels(&dir, "Gen.sty"), [90, 90, 77, 66, 100, 45, 50, 100]);
    // Another style: no style rule, so Chord 1 has the GM default.
    assert_eq!(gen_style_levels(&dir, "Other.sty"), [90, 90, 77, 100, 100, 45, 50, 100]);
    // The file itself is not touched by reading it.
    assert_eq!(std::fs::read_to_string(dir.join(patches::FILE_NAME)).unwrap(), V2_LIBRARY);
    let _ = std::fs::remove_dir_all(&dir);
}

/// An offline session on the generated style (loaded), with the test SoundFonts, in a
/// fresh data folder `folder(tag)` (made by the caller, so it may hold a library file).
fn gen_session(data: &Path) -> Session {
    let style = data.join("styles/Gen.sty");
    std::fs::create_dir_all(style.parent().unwrap()).unwrap();
    std::fs::write(&style, gen_style_bytes()).unwrap();
    let opts = Options { paths: vec![style], data_dir: Some(data.to_path_buf()), sf2: Some(data.join("sf").join(SF2)), ..Options::default() };
    let s = Session::offline(opts).unwrap();
    load(&s, "Gen.sty");
    s
}

/// The first save of a migrated library copies the version 2 file to
/// `sound-library.v2.json` first, once; the file is then version 3, every sound kept with
/// no defaults, and its volumes on the rules.
#[test]
fn the_version_2_file_is_backed_up_once_before_the_first_save() {
    let data = folder("v2-backup");
    let file = data.join(patches::FILE_NAME);
    let bak = data.join("sound-library.v2.json");
    std::fs::write(&file, V2_LIBRARY).unwrap();
    {
        let s = gen_session(&data);
        let st = s.state();
        assert_eq!(st.sound_library.patches.len(), 5, "every sound kept");
        assert_eq!((st.sound_library.map.family_volumes[4], st.sound_library.map.drums_volume), (Some(77), Some(90)));
        assert_eq!(st.sound_library.style_map.overrides[0].volume, Some(66));
        s.send(SoundLibraryCmd::SetPatchFavourite { id: "bass".into(), favourite: true }).unwrap();
        assert_eq!(std::fs::read_to_string(&bak).unwrap(), V2_LIBRARY, "the version 2 file, as it was");
        let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(saved["version"], 3);
        assert_eq!(saved["patches"].as_array().unwrap().len(), 5);
        assert!(saved["patches"].as_array().unwrap().iter().all(|p| p.get("defaults").is_none()));
        assert_eq!(saved["map"]["familyVolumes"][4], 77);
        assert_eq!(saved["styleMaps"]["Gen.sty"]["overrides"][0]["volume"], 66);
        s.send(SoundLibraryCmd::SetPatchFavourite { id: "bass".into(), favourite: false }).unwrap();
        assert_eq!(std::fs::read_to_string(&bak).unwrap(), V2_LIBRARY, "written once");
    }
    // Next boot: a version 3 file, nothing more to back up.
    let s = gen_session(&data);
    s.send(SoundLibraryCmd::SetPatchFavourite { id: "pad".into(), favourite: true }).unwrap();
    assert_eq!(std::fs::read_to_string(&bak).unwrap(), V2_LIBRARY);
    assert!(!data.join("sound-library.v3.json").exists());
    let _ = std::fs::remove_dir_all(&data);
}

/// A sound is the raw instrument: assigning one leaves the part's mix alone, saving the
/// part over its own sound leaves the sound's record as it was (a SoundFont sound has no
/// state to take), and an audition plays at the fixed neutral level.
#[test]
fn saving_or_assigning_a_sound_never_touches_mix() {
    let data = folder("no-mix");
    let s = gen_session(&data);
    let id = add(&s, "My Piano", 0, 0);
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 33 }).unwrap();
    s.send(PartsCmd::SetPartOctave { part: 0, octave: 2 }).unwrap();
    s.send(PartsCmd::SetPartPan { part: 0, pan: 20 }).unwrap();
    s.send(SoundLibraryCmd::SetPartPatch { part: 0, id: Some(id.clone()) }).unwrap();
    let r1 = |s: &Session| {
        let k = s.state().keyboard_parts[0].clone();
        (k.patch, k.volume, k.octave, k.pan)
    };
    assert_eq!(r1(&s), (Some(id.clone()), 33, 2, 20), "the part keeps its mix");
    let record = |s: &Session| s.state().sound_library.patches.iter().find(|p| p.patch.id == id).unwrap().patch.clone();
    let before = record(&s);
    let text_before = std::fs::read_to_string(data.join(patches::FILE_NAME)).unwrap();
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 110 }).unwrap();
    s.send(PartsCmd::SetPartOctave { part: 0, octave: -2 }).unwrap();
    s.send(SoundLibraryCmd::SaveSound { part: 0 }).unwrap();
    assert_eq!(s.state().sound_library.patches.len(), 1, "saved over its own sound");
    assert_eq!(record(&s), before, "the record is unchanged");
    assert_eq!(std::fs::read_to_string(data.join(patches::FILE_NAME)).unwrap(), text_before, "and so is the file");
    assert!(!text_before.contains("defaults") && !text_before.contains("volume"));
    // Save as…: a new sound, with no mix either; the part keeps its own.
    s.send(SoundLibraryCmd::SaveSoundAs { part: 0, name: Some("Copy".into()) }).unwrap();
    assert!(!std::fs::read_to_string(data.join(patches::FILE_NAME)).unwrap().contains("volume"));
    assert_eq!((r1(&s).1, r1(&s).2), (110, -2));
    // An audition plays at the neutral level, whatever the part's.
    s.send(SoundLibraryCmd::AuditionPatch { id: id.clone() }).unwrap();
    assert_eq!(s.inner.lock().sound.audition.as_ref().map(|a| a.volume), Some(100));
    s.send(SoundLibraryCmd::StopPatchAudition).unwrap();
    let _ = std::fs::remove_dir_all(&data);
}

/// The hand-over race (#109): a style chosen after the engine has taken over the style
/// handed to it before, but before the control side has promoted that one, must not reuse
/// (rewrite) the bank the engine now plays from. The snapshot already shows the takeover.
#[test]
fn a_style_chosen_during_the_hand_over_gets_the_other_bank() {
    let Some((s, data)) = session("race", &["SlowWalker.T552.sty", "BubblyDub.T552.sty"], true) else { return };
    let g = add(&s, "Global Bass", 0, 33);
    let own = add(&s, "Dub Bass", 0, 35);
    s.send(SoundLibraryCmd::SetFamilyRule { family: 4, patch: Some(g), style: false }).unwrap();
    let id = |name: &str| s.library_list().entries.iter().find(|e| e.path.contains(name)).unwrap().id;
    let (slow, dub) = (id("SlowWalker"), id("BubblyDub"));
    load(&s, "BubblyDub");
    s.send(SoundLibraryCmd::SetFamilyRule { family: 4, patch: Some(own), style: true }).unwrap();
    load(&s, "SlowWalker");
    let routes = s.inner.shared.routes.clone();
    let slow_bank = routes.current.load(Relaxed);
    assert_eq!(routes.bank_route(slow_bank, 33).map(|r| r.program), Some(33));
    let dub_bank;
    {
        let mut guard = s.inner.lock();
        let ctl = &mut *guard;
        // BubblyDub handed over; the (stopped) engine takes it over at its next step, and its
        // snapshot arrives, but no pump has promoted it yet.
        ctl.apply(LibraryCmd::LoadStyle { id: dub }.into()).unwrap();
        dub_bank = ctl.sound.pending.as_ref().unwrap().0;
        assert_ne!(dub_bank, slow_bank);
        let o = ctl.offline.as_mut().unwrap();
        let now = o.now;
        o.engine.step(now);
        while let Ok(sn) = ctl.snap_rx.pop() {
            ctl.snap = sn;
        }
        assert_eq!(routes.bank_route(dub_bank, 33).map(|r| r.program), Some(35), "BubblyDub plays its own rule");
        // SlowWalker chosen right then.
        ctl.apply(LibraryCmd::LoadStyle { id: slow }.into()).unwrap();
        assert_eq!(routes.bank_route(dub_bank, 33).map(|r| r.program), Some(35), "the bank BubblyDub plays from is not rewritten");
        assert_eq!(ctl.sound.pending.as_ref().unwrap().0, slow_bank, "SlowWalker gets the other bank");
    }
    s.advance(10 * MS);
    assert!(s.state().style.path.contains("SlowWalker"));
    let cur = routes.current.load(Relaxed);
    assert_eq!((cur, routes.bank_route(cur, 33).map(|r| r.program)), (slow_bank, Some(33)));
    let _ = std::fs::remove_dir_all(&data);
}

/// B2 (review of #106): at a style change the per-channel routes follow the NEW style's
/// setup voices: `info` is the new style's before the routes are synced.
#[test]
fn channel_routes_follow_the_new_styles_voices() {
    let Some((s, data)) = session("b2", &["SlowWalker.T552.sty", "BubblyDub.T552.sty"], true) else { return };
    let voices = |name: &str| {
        let p = root().join("corpus/MOX_v2").join(name);
        crate::engine::Prepared::new(&crate::sff::Style::load(&p).unwrap()).setups[0].voices
    };
    let (va, vb) = (voices("SlowWalker.T552.sty"), voices("BubblyDub.T552.sty"));
    let fam = |v: Option<(u8, u8, u8)>, ch: u8| {
        v.filter(|v| !patches::is_drum(ch, v.0)).map(|(m, _, p)| patches::family_of(patches::map_program(ch, m, p)))
    };
    let ch = (10..16u8).find(|&c| fam(va[c as usize], c).is_some() && fam(va[c as usize], c) != fam(vb[c as usize], c)).expect("a channel whose family differs");
    let id = add(&s, "Extra", 0, 0);
    s.send(SoundLibraryCmd::SetFamilyRule { family: fam(va[ch as usize], ch).unwrap() as u8, patch: Some(id.clone()), style: false }).unwrap();
    assert_eq!(s.inner.lock().sound.synced[ch as usize].as_deref(), Some(id.as_str()), "SlowWalker's channel {} on the patch", ch + 1);
    load(&s, "BubblyDub");
    let want = s.inner.lock().channel_patch(ch);
    assert_eq!(s.inner.lock().sound.synced[ch as usize], want, "synced from BubblyDub's voices");
    assert_ne!(want.as_deref(), Some(id.as_str()));
    let _ = std::fs::remove_dir_all(&data);
}

/// B3 (review of #106): a patch naming a SoundFont that can't be read neither blocks the
/// other extra SoundFonts nor sets the loader going again and again.
#[test]
#[cfg(feature = "slow-tests")]
fn a_bad_soundfont_does_not_loop_or_block_the_others() {
    let Some((s, data)) = session("b3", &["SlowWalker.T552.sty"], true) else { return };
    std::fs::write(data.join("sf").join("bad.sf2"), b"RIFF\x04\x00\x00\x00sfbk").unwrap();
    s.offline_audio(Some(&data.join("sf").join(SF2)), 48_000).unwrap();
    for (name, file) in [("Good", OTHER), ("Bad", "bad.sf2")] {
        let mut f = fields(name, 0, 0);
        f.source = PatchSource::SoundFont { file: file.into(), bank: 0, program: 0 };
        s.send(SoundLibraryCmd::CreatePatch { patch: f }).unwrap();
    }
    let mut loads = 0;
    let mut was_loading = false;
    let mut pump = || {
        s.advance(10 * MS);
        let loading = s.inner.lock().sound.loading.is_some();
        if loading && !was_loading {
            loads += 1;
        }
        was_loading = loading;
        loading
    };
    // Until the loader settles with the good extra SoundFont in (a loop never settles)...
    let t0 = std::time::Instant::now();
    while (pump() || s.inner.lock().sound.rack_fonts != [SF2, OTHER]) && t0.elapsed() < std::time::Duration::from_secs(10) {
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    // ...then a pump would start it again at once if the bad one were retried.
    for _ in 0..20 {
        pump();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let fonts = s.inner.lock().sound.rack_fonts.clone();
    assert_eq!(fonts, [SF2, OTHER], "the good extra SoundFont plays");
    assert!(loads <= 2, "the loader is not started again and again ({loads})");
    let msg = s.state().message.clone().unwrap();
    assert!(msg.error && msg.text.contains("bad.sf2"), "{msg:?}");
    // The file changes (fixed; its size differs, so no wait for a new modification time):
    // it is tried again, and loads.
    std::fs::write(data.join("sf").join("bad.sf2"), patches::sf2::tiny_gm_sound_font()).unwrap();
    let t0 = std::time::Instant::now();
    while t0.elapsed() < std::time::Duration::from_secs(10) {
        s.advance(10 * MS);
        if s.inner.lock().sound.rack_fonts.len() == 3 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert_eq!(s.inner.lock().sound.rack_fonts.len(), 3, "the fixed SoundFont loads");
    let _ = std::fs::remove_dir_all(&data);
}

/// D5/O7: the library exports as a bundle (metadata, maps, every plugin sound's state,
/// SoundFonts by file name) and imports into another data folder. A SoundFont that folder
/// lacks is reported and its sounds are kept.
#[test]
fn a_bundle_round_trips_into_another_data_folder() {
    let Some((a, data_a)) = session("bundle-a", &["SlowWalker.T552.sty"], true) else { return };
    let Some((b, data_b)) = session("bundle-b", &["SlowWalker.T552.sty"], true) else { return };
    let piano = add(&a, "Piano", 0, 0);
    let state = crate::api::base64_encode(b"sampler deluxe state");
    let fake = patches::Patch {
        id: "deluxe".into(),
        name: "Deluxe Keys".into(),
        category: patches::Category::guess(0, 4),
        tags: vec!["mine".into()],
        favourite: true,
        source: patches::PatchSource::plugin("aumu Smp7 Fake", state.clone()),
    };
    let gone = patches::Patch { id: "gone".into(), name: "Gone Pad".into(), source: patches::PatchSource::SoundFont { file: "Gone.sf2".into(), bank: 0, program: 88 }, ..fake.clone() };
    a.inner.lock().sound.lib.patches.extend([fake.clone(), gone.clone()]);
    a.send(SoundLibraryCmd::SetFamilyRule { family: 0, patch: Some("deluxe".into()), style: false }).unwrap();
    let out = data_a.join("share").join("bundle.json");
    a.send(SoundLibraryCmd::ExportSoundLibrary { path: Some(out.display().to_string()) }).unwrap();
    let text = std::fs::read_to_string(&out).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["kind"], patches::store::BUNDLE_KIND);
    assert_eq!(v["fonts"], serde_json::json!(["Gone.sf2", SF2]), "fonts by file name");
    assert!(!data_a.join("share").join(SF2).exists(), "no font is copied");

    b.send(SoundLibraryCmd::ImportSoundLibrary { path: out.display().to_string(), replace: false, maps: true }).unwrap();
    let msg = b.state().message.clone().unwrap();
    assert!(msg.error && msg.text.contains("Gone.sf2") && !msg.text.contains(SF2), "the missing font is reported: {msg:?}");
    let lib = b.inner.lock().sound.lib.clone();
    assert_eq!(lib.patches.len(), 3, "every sound is kept, the missing font's too");
    let got = lib.patches.iter().find(|p| p.name == "Deluxe Keys").unwrap();
    assert_eq!(got.source, fake.source, "the plugin sound's state comes over unchanged");
    assert!(got.favourite && got.tags == fake.tags);
    assert_eq!(lib.map.families[0].as_deref(), Some(got.id.as_str()), "the map comes with it");
    assert!(lib.patches.iter().any(|p| p.name == "Gone Pad"));
    assert!(lib.patches.iter().any(|p| p.name == "Piano"), "{piano}");
    // Written to the other folder's library file.
    let saved = patches::SoundLibrary::load(&data_b.join(patches::FILE_NAME)).unwrap();
    assert_eq!(saved.patches.len(), 3);
    // A replace import takes the bundle's library as it is.
    b.send(SoundLibraryCmd::ImportSoundLibrary { path: out.display().to_string(), replace: true, maps: false }).unwrap();
    assert_eq!(b.inner.lock().sound.lib.patches.len(), 3);
    assert!(patches::SoundLibrary::read_bundle(r#"{"kind":"something-else"}"#).is_err());
    let _ = std::fs::remove_dir_all(&data_a);
    let _ = std::fs::remove_dir_all(&data_b);
}

/// B5 (review of #106): a merge import never saves over a library file a newer yahaha
/// wrote; a replace import keeps it as `.bak`.
#[test]
fn a_merge_import_keeps_a_newer_file() {
    let Some((_, data)) = session("b5", &["SlowWalker.T552.sty"], false) else { return };
    let file = data.join(patches::FILE_NAME);
    let newer = r#"{"version": 4, "patches": [], "fancyNewThing": [1,2,3]}"#;
    std::fs::write(&file, newer).unwrap();
    let small = data.join("share.json");
    std::fs::write(&small, r#"[{"id":"x","name":"X","source":{"kind":"soundFont","file":"a.sf2","bank":0,"program":1}}]"#).unwrap();
    let opts = Options { paths: vec![root().join("corpus/MOX_v2/SlowWalker.T552.sty")], data_dir: Some(data.clone()), ..Options::default() };
    let s = Session::offline(opts).unwrap();
    s.send(SoundLibraryCmd::ImportSoundLibrary { path: small.display().to_string(), replace: false, maps: false }).unwrap();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), newer, "a merge import leaves the newer file alone");
    assert_eq!(s.state().sound_library.patches.len(), 1, "the import is in the session");
    s.send(SoundLibraryCmd::ImportSoundLibrary { path: small.display().to_string(), replace: true, maps: false }).unwrap();
    assert_eq!(std::fs::read_to_string(file.with_extension("json.bak")).unwrap(), newer, "kept beside");
    assert!(std::fs::read_to_string(&file).unwrap().contains("\"version\": 3"));
    let _ = std::fs::remove_dir_all(&data);
}

/// A style's own map left with no rules (#109) is dropped from the library, not kept as
/// an empty entry.
#[test]
fn a_styles_map_cleared_rule_by_rule_goes() {
    let Some((s, data)) = session("prune", &["SlowWalker.T552.sty"], true) else { return };
    let own = add(&s, "Slow Bass", 0, 35);
    s.send(SoundLibraryCmd::SetFamilyRule { family: 4, patch: Some(own.clone()), style: true }).unwrap();
    let key = s.state().sound_library.style_key.clone();
    assert!(s.inner.lock().sound.lib.style_maps.contains_key(&key));
    s.send(SoundLibraryCmd::SetFamilyRule { family: 4, patch: None, style: true }).unwrap();
    assert!(!s.inner.lock().sound.lib.style_maps.contains_key(&key), "no empty entry left");
    // Clearing a rule the style never had leaves no entry either.
    s.send(SoundLibraryCmd::SetFamilyRule { family: 5, patch: None, style: true }).unwrap();
    assert!(s.inner.lock().sound.lib.style_maps.is_empty());
    let _ = std::fs::remove_dir_all(&data);
}

/// Font ids are recycled once all `MAX_FONTS` are taken (#109): an id whose SoundFont
/// nothing uses names the new file; one the rack, a load or a patch still uses is never
/// given away.
#[test]
fn font_ids_are_recycled_only_when_unused() {
    use crate::patches::route::MAX_FONTS;
    let mut sl = SoundLib::open(None);
    for i in 0..MAX_FONTS {
        assert_eq!(sl.font_id(&format!("f{i}.sf2")), Some(i as u8));
    }
    assert_eq!(sl.font_id("f3.sf2"), Some(3), "a known file keeps its id");
    sl.rack_fonts = vec!["f0.sf2".into(), "f1.sf2".into()];
    sl.loading = Some(vec!["f2.sf2".into()]);
    sl.lib.patches.push(patches::Patch {
        id: "p".into(),
        name: "P".into(),
        category: patches::Category::Piano,
        tags: vec![],
        favourite: false,
        source: PatchSource::SoundFont { file: "f3.sf2".into(), bank: 0, program: 0 },
    });
    assert_eq!(sl.font_id("new.sf2"), Some(4), "the first id nothing uses");
    assert_eq!(sl.font_id("new.sf2"), Some(4));
    assert_eq!(sl.font_id("f0.sf2"), Some(0));
    // Every id in use: none to give.
    sl.rack_fonts = (0..MAX_FONTS).map(|i| if i == 4 { "new.sf2".to_string() } else { format!("f{i}.sf2") }).collect();
    assert_eq!(sl.font_id("other.sf2"), None);
}

/// A backup that can't be written blocks the save: the older file stays as it was.
#[cfg(unix)]
#[test]
fn a_failed_backup_leaves_the_older_file_untouched() {
    let data = folder("v2-backup-fail");
    let file = data.join(patches::FILE_NAME);
    std::fs::write(&file, V2_LIBRARY).unwrap();
    // The backup's path is a link into a folder that doesn't exist: the copy fails.
    std::os::unix::fs::symlink(data.join("missing/x.json"), data.join("sound-library.v2.json")).unwrap();
    let s = gen_session(&data);
    let _ = s.send(SoundLibraryCmd::SetPatchFavourite { id: "bass".into(), favourite: true });
    assert_eq!(std::fs::read_to_string(&file).unwrap(), V2_LIBRARY, "not saved over");
    drop(s);
    let _ = std::fs::remove_dir_all(&data);
}

/// A version 1 file and a bare patch list are both backed up as `sound-library.v1.json`.
#[test]
fn version_1_and_bare_files_back_up_as_v1() {
    for (tag, text) in [("v1-obj", r#"{"version": 1, "patches": []}"#), ("v1-bare", "[]")] {
        let data = folder(tag);
        std::fs::write(data.join(patches::FILE_NAME), text).unwrap();
        let s = gen_session(&data);
        add(&s, "Bass", 0, 33);
        assert_eq!(std::fs::read_to_string(data.join("sound-library.v1.json")).unwrap(), text, "{tag}");
        drop(s);
        let _ = std::fs::remove_dir_all(&data);
    }
}

/// `updatePatch` with a plugin source and no state (as a client sends back the state's
/// source, which shows only `hasState`) keeps the stored state for the same plugin and
/// origin (#436); another factory preset of the plugin starts with none.
#[test]
fn an_update_without_state_keeps_the_stored_plugin_state() {
    let data = folder("keep-state");
    let s = gen_session(&data);
    let source = |state: &str, number| PatchSource::Plugin { component_id: "aumu Smp7 Fake".into(), state: state.into(), origin: PluginOrigin::Factory { number } };
    let plugin_fields = |name: &str, state: &str, number| PatchFields { name: name.into(), category: patches::Category::EPiano, tags: vec![], favourite: false, source: source(state, number) };
    s.send(SoundLibraryCmd::CreatePatch { patch: plugin_fields("Keys", "c2FtcGxlcg==", 1) }).unwrap();
    let id = s.state().sound_library.last_added.clone().unwrap();
    let stored = |s: &Session| s.inner.lock().sound.lib.patches.iter().find(|p| p.id == id).unwrap().source.clone();
    s.send(SoundLibraryCmd::UpdatePatch { id: id.clone(), patch: plugin_fields("Renamed", "", 1) }).unwrap();
    assert_eq!(stored(&s), source("c2FtcGxlcg==", 1), "the stored blob is unchanged");
    let st = s.state();
    let p = st.sound_library.patches.iter().find(|p| p.patch.id == id).unwrap();
    assert_eq!(p.patch.name, "Renamed");
    assert!(matches!(p.patch.source, PatchSourceView::Plugin { has_state: true, .. }));
    assert!(std::fs::read_to_string(data.join(patches::FILE_NAME)).unwrap().contains("c2FtcGxlcg=="), "and so is the file");
    // Another factory preset of the same plugin: the old preset's state doesn't carry over.
    s.send(SoundLibraryCmd::UpdatePatch { id: id.clone(), patch: plugin_fields("Renamed", "", 2) }).unwrap();
    assert_eq!(stored(&s), source("", 2));
    drop(s);
    let _ = std::fs::remove_dir_all(&data);
}

/// Each patch's number as the state shows it, by patch id, after checking that `number_of`
/// and `at_number` agree with the state and round-trip for every number, and that the
/// numbers are exactly 1..=n.
fn numbers_by_id(s: &Session) -> std::collections::HashMap<String, u32> {
    let st = s.state();
    let ctl = s.inner.lock();
    let lib = &ctl.sound.lib;
    let n = st.sound_library.patches.len() as u32;
    let mut seen: Vec<u32> = st.sound_library.patches.iter().map(|p| p.number).collect();
    seen.sort_unstable();
    assert_eq!(seen, (1..=n).collect::<Vec<_>>());
    for p in &st.sound_library.patches {
        assert_eq!(super::number_of(lib, &p.patch.id), Some(p.number), "{}", p.patch.name);
        assert_eq!(super::at_number(lib, p.number).map(|x| x.id.as_str()), Some(p.patch.id.as_str()));
    }
    for k in 1..=n {
        let id = &super::at_number(lib, k).unwrap().id;
        assert_eq!(super::number_of(lib, id), Some(k));
    }
    assert_eq!(super::number_of(lib, "nope"), None);
    assert!(super::at_number(lib, 0).is_none() && super::at_number(lib, n + 1).is_none());
    st.sound_library.patches.iter().map(|p| (p.patch.id.clone(), p.number)).collect()
}

/// Sound numbers (docs/eyes-free.md): with no favourites they follow the Library's order,
/// category (the Genos order) then name ignoring case, not the order the sounds were
/// added in; starring a sound makes it number 1 and moves what came before it down one,
/// and unstarring puts every number back, at once in the state.
#[test]
fn sound_numbers_put_favourites_first_then_the_librarys_order() {
    let data = folder("numbers");
    let s = gen_session(&data);
    // Added in the reverse of their numbers: index order would number them 1, 2, 3, 4.
    let pad = add(&s, "Zither Pad", 0, 88);
    let sax = add(&s, "Alto Sax", 0, 65);
    let upright = add(&s, "Upright Piano", 0, 0);
    let bright = add(&s, "bright Piano", 0, 1);
    let n = numbers_by_id(&s);
    assert_eq!([n[&bright], n[&upright], n[&sax], n[&pad]], [1, 2, 3, 4]);

    // A favourite comes first; the sounds before it move down one, those after it stay.
    s.send(SoundLibraryCmd::SetPatchFavourite { id: sax.clone(), favourite: true }).unwrap();
    let n = numbers_by_id(&s);
    assert_eq!([n[&sax], n[&bright], n[&upright], n[&pad]], [1, 2, 3, 4]);

    // Favourites among themselves keep the Library's order too.
    s.send(SoundLibraryCmd::SetPatchFavourite { id: upright.clone(), favourite: true }).unwrap();
    let n = numbers_by_id(&s);
    assert_eq!([n[&upright], n[&sax], n[&bright], n[&pad]], [1, 2, 3, 4]);

    // Unstarring both gives back the numbers without favourites.
    for id in [&sax, &upright] {
        s.send(SoundLibraryCmd::SetPatchFavourite { id: id.clone(), favourite: false }).unwrap();
    }
    let n = numbers_by_id(&s);
    assert_eq!([n[&bright], n[&upright], n[&sax], n[&pad]], [1, 2, 3, 4]);

    // Moving a patch in the library doesn't renumber: the Library doesn't show that order.
    s.send(SoundLibraryCmd::MovePatch { id: pad.clone(), to: 0 }).unwrap();
    assert_eq!(numbers_by_id(&s)[&pad], 4);

    // Removing a sound renumbers those after it.
    s.send(SoundLibraryCmd::DeletePatch { id: upright.clone() }).unwrap();
    let n = numbers_by_id(&s);
    assert_eq!([n[&bright], n[&sax], n[&pad]], [1, 2, 3]);
    drop(s);
    let _ = std::fs::remove_dir_all(&data);
}

/// Sounds with the same category and name (a duplicate) still get a number each, in the
/// library's order, and round-trip.
#[test]
fn duplicate_sounds_get_a_number_each() {
    let data = folder("numbers-dup");
    let s = gen_session(&data);
    let a = add(&s, "Piano", 0, 0);
    let b = add(&s, "Piano", 0, 0);
    let n = numbers_by_id(&s);
    assert_eq!([n[&a], n[&b]], [1, 2]);
    drop(s);
    let _ = std::fs::remove_dir_all(&data);
}
