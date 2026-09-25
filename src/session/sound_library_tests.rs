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
    assert_eq!(p.source, PatchSource::SoundFont { file: OTHER.into(), bank: 0, program: 50 }, "the mapped patch's sound");
    assert_eq!(p.defaults.volume, Some(st.keyboard_parts[1].volume));
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
    let styled = prep.setups[0].mix;
    prep.setups[0].mix_set = 0b1111_0000;
    sl.prepare(&mut prep, "x");
    for (part, &style_level) in styled.iter().enumerate() {
        if prep.setups[0].voices[8 + part].is_none() {
            continue;
        }
        let want = if part >= 4 { style_level } else { 77 };
        assert_eq!(prep.setups[0].mix[part], want, "part {part}");
    }
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
    for _ in 0..300 {
        s.advance(10 * MS);
        let loading = s.inner.lock().sound.loading.is_some();
        if loading && !was_loading {
            loads += 1;
        }
        was_loading = loading;
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let fonts = s.inner.lock().sound.rack_fonts.clone();
    assert_eq!(fonts, [SF2, OTHER], "the good extra SoundFont plays");
    assert!(loads <= 2, "the loader is not started again and again ({loads})");
    let msg = s.state().message.clone().unwrap();
    assert!(msg.error && msg.text.contains("bad.sf2"), "{msg:?}");
    // The file changes (fixed): it is tried again, and loads.
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(data.join("sf").join("bad.sf2"), patches::sf2::tiny_gm_sound_font()).unwrap();
    for _ in 0..300 {
        s.advance(10 * MS);
        if s.inner.lock().sound.rack_fonts.len() == 3 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert_eq!(s.inner.lock().sound.rack_fonts.len(), 3, "the fixed SoundFont loads");
    let _ = std::fs::remove_dir_all(&data);
}

/// B5 (review of #106): a merge import never saves over a library file a newer yahaha
/// wrote; a replace import keeps it as `.bak`.
#[test]
fn a_merge_import_keeps_a_newer_file() {
    let Some((_, data)) = session("b5", &["SlowWalker.T552.sty"], false) else { return };
    let file = data.join(patches::FILE_NAME);
    let newer = r#"{"version": 2, "patches": [], "fancyNewThing": [1,2,3]}"#;
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
    assert!(std::fs::read_to_string(&file).unwrap().contains("\"version\": 1"));
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
        defaults: PatchDefaults::default(),
    });
    assert_eq!(sl.font_id("new.sf2"), Some(4), "the first id nothing uses");
    assert_eq!(sl.font_id("new.sf2"), Some(4));
    assert_eq!(sl.font_id("f0.sf2"), Some(0));
    // Every id in use: none to give.
    sl.rack_fonts = (0..MAX_FONTS).map(|i| if i == 4 { "new.sf2".to_string() } else { format!("f{i}.sf2") }).collect();
    assert_eq!(sl.font_id("other.sf2"), None);
}
