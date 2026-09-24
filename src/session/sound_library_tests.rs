//! The sound library through an offline session: commands, state, persistence, the route
//! table the synth reads, and the style hand-off.

use super::SoundLib;
use crate::api::*;
use crate::patches;
use crate::session::{Options, Session};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering::Relaxed;

const MS: u64 = 1_000_000;
const SF2: &str = "GeneralUser-GS.sf2";

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// An offline session on `styles` (corpus/MOX_v2 file names) with a fresh data folder, and
/// the checkout's SoundFont folder when `sf2` (so patches there are playable).
fn session(tag: &str, styles: &[&str], sf2: bool) -> Option<(Session, PathBuf)> {
    let paths: Vec<PathBuf> = styles.iter().map(|s| root().join("corpus/MOX_v2").join(s)).collect();
    if paths.iter().any(|p| !p.exists()) || (sf2 && !root().join("soundfonts").join(SF2).exists()) {
        eprintln!("corpus or SoundFont missing; skipping");
        return None;
    }
    let data = std::env::temp_dir().join(format!("yahaha-sl-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data);
    let opts = Options {
        paths,
        data_dir: Some(data.clone()),
        sf2: sf2.then(|| root().join("soundfonts").join(SF2)),
        ..Options::default()
    };
    Some((Session::offline(opts).unwrap(), data))
}

fn fields(name: &str, bank: u16, program: u8) -> PatchFields {
    PatchFields {
        name: name.into(),
        category: patches::Category::guess(bank, program),
        tags: vec![],
        favourite: false,
        source: PatchSource::SoundFont { file: SF2.into(), bank, program },
        defaults: PatchDefaults::default(),
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
    let r = routes.bank_route(0, 33).expect("program 33 routes");
    assert_eq!((r.bank, r.program), (0, 34));
    assert!(matches!(r.source, patches::Source::SoundFont(_)));
    assert_eq!(routes.bank_drum(0).map(|r| r.bank), Some(128));
    assert_eq!(routes.bank_route(0, 40), None, "Strings: no rule");

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
    f.defaults.volume = Some(250);
    s.send(SoundLibraryCmd::UpdatePatch { id: a.clone(), patch: f }).unwrap();
    let st = s.state();
    let p = st.sound_library.patches.iter().find(|p| p.patch.id == a).unwrap();
    assert_eq!((p.patch.name.as_str(), p.patch.defaults.volume, p.patch.tags.len()), ("Warm Pad", Some(127), 1));
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
    assert_eq!(routes.current.load(Relaxed), 0);
    assert_eq!(routes.bank_route(0, 33).map(|r| r.program), Some(35), "the style's rule in its bank");

    // Another style: it gets the other bank, written with the global map, before the
    // engine takes it over.
    let other = s.library_list().entries.iter().find(|e| e.path.contains("BubblyDub")).unwrap().id;
    s.send(LibraryCmd::LoadStyle { id: other }).unwrap();
    s.advance(10 * MS);
    let st = s.state();
    assert_ne!(st.sound_library.style_key, key);
    assert!(st.sound_library.style_map.is_empty());
    assert_eq!(routes.current.load(Relaxed), 1);
    assert_eq!(routes.bank_route(1, 33).map(|r| r.program), Some(33), "the global rule");
    assert!(st.sound_library.usage.iter().filter(|u| (32..40).contains(&u.gm_program) && !u.drums).all(|u| u.patch.as_deref() == Some(g.as_str())));
    // Back: its own map again, in bank 0.
    let first = s.library_list().entries.iter().find(|e| e.path.contains("SlowWalker")).unwrap().id;
    s.send(LibraryCmd::LoadStyle { id: first }).unwrap();
    s.advance(10 * MS);
    assert_eq!(routes.current.load(Relaxed), 0);
    assert_eq!(s.state().sound_library.style_map.families[4].as_deref(), Some(own.as_str()));
    s.send(SoundLibraryCmd::ClearStyleMap).unwrap();
    assert!(s.state().sound_library.style_map.is_empty());
    assert_eq!(routes.bank_route(0, 33).map(|r| r.program), Some(33));
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_keyboard_part_takes_its_patch_and_defaults() {
    let Some((s, data)) = session("part", &["SlowWalker.T552.sty"], true) else { return };
    let mut f = fields("Stage Piano", 0, 1);
    f.defaults = PatchDefaults { volume: Some(80), pan: Some(40), reverb: Some(30), chorus: None, octave: 1 };
    s.send(SoundLibraryCmd::CreatePatch { patch: f }).unwrap();
    let id = s.state().sound_library.last_added.clone().unwrap();
    s.send(SoundLibraryCmd::SetPartPatch { part: 0, id: Some(id.clone()) }).unwrap();
    let st = s.state();
    let r1 = &st.keyboard_parts[0];
    assert_eq!((r1.patch.as_deref(), r1.voice_name.as_str(), r1.volume, r1.octave), (Some(id.as_str()), "Stage Piano", 80, 1));
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
    assert_eq!(p.patch.source, PatchSource::SoundFont { file: SF2.into(), bank: 0, program: 48 });
    assert_eq!(p.patch.defaults.volume, Some(st.keyboard_parts[1].volume));
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

/// A Style part the style sets no level for takes its patch's CC7 (the mixer shows it);
/// one the style sets keeps the style's.
#[test]
fn a_patch_volume_fills_in_where_the_style_sets_none() {
    let p = root().join("corpus/MOX_v2/SlowWalker.T552.sty");
    if !p.exists() {
        return;
    }
    let style = crate::sff::Style::load(&p).unwrap();
    let mut prep = crate::engine::Prepared::new(&style);
    let mut sl = SoundLib::open(None);
    let pat = crate::patches::Patch {
        id: "b".into(),
        name: "B".into(),
        category: patches::Category::Bass,
        tags: vec![],
        favourite: false,
        source: PatchSource::SoundFont { file: SF2.into(), bank: 0, program: 33 },
        defaults: PatchDefaults { volume: Some(77), ..PatchDefaults::default() },
    };
    sl.lib.patches.push(pat);
    for f in 0..16 {
        sl.lib.map.set_family(f, Some("b".into()));
    }
    sl.lib.map.drums = Some("b".into());
    let styled = prep.mix;
    prep.mix_set = 0b1111_0000;
    sl.prepare(&mut prep, "x");
    for part in 0..8 {
        if prep.voices[8 + part].is_none() {
            continue;
        }
        let want = if part >= 4 { styled[part] } else { 77 };
        assert_eq!(prep.mix[part], want, "part {part}");
    }
}
