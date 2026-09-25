//! The sound catalog (#117): ids, the plugin category guess, and the catalog through an
//! offline session.

use super::*;
use crate::api::{parse_preset_id, plugin_category, preset_id, SoundSource};
use crate::api::{AppState, SoundLibraryCmd};
use crate::patches::sf2::tiny_sound_font;
use crate::session::{Options, Session};
use std::path::PathBuf;

#[test]
fn preset_ids_round_trip_even_with_a_colon_in_the_file() {
    assert_eq!(preset_id("GM.sf2", 128, 0), "sf:GM.sf2:128:0");
    assert_eq!(parse_preset_id("sf:GM.sf2:128:0"), Some(("GM.sf2", 128, 0)));
    assert_eq!(parse_preset_id("sf:a:b.sf2:8:4"), Some(("a:b.sf2", 8, 4)));
    assert_eq!(parse_preset_id("sf:GM.sf2:0:128"), None);
    assert_eq!(parse_preset_id("au:aumu dls  appl"), None);
}

#[test]
fn plugin_categories_come_from_name_and_maker() {
    assert_eq!(plugin_category("Keyscape", "Spectrasonics"), Category::Piano);
    assert_eq!(plugin_category("Lounge Lizard EP-4", "AAS"), Category::EPiano);
    assert_eq!(plugin_category("B-3 V2", "Arturia"), Category::Organ);
    assert_eq!(plugin_category("Trilian Bass", "Spectrasonics"), Category::Bass);
    assert_eq!(plugin_category("Serum", "Xfer Records"), Category::SynthLead);
}

/// A data folder with a SoundFont folder (`<data>/sf`): `A.sf2` (piano, bass and a kit,
/// the most complete: Auto) and `B.sf2` (one pad).
fn folder(tag: &str) -> PathBuf {
    let data = std::env::temp_dir().join(format!("yahaha-sounds-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data);
    let sf = data.join("sf");
    std::fs::create_dir_all(&sf).unwrap();
    std::fs::write(sf.join("A.sf2"), tiny_sound_font(&[(0, 0, "Piano"), (0, 33, "Finger Bass"), (128, 0, "Standard Kit")])).unwrap();
    std::fs::write(sf.join("B.sf2"), tiny_sound_font(&[(0, 88, "Warm Pad")])).unwrap();
    data
}

fn offline(data: &Path) -> Option<Session> {
    let style = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
    if !style.exists() {
        eprintln!("corpus missing; skipping");
        return None;
    }
    let opts = Options { paths: vec![style], data_dir: Some(data.to_path_buf()), sound_font_dir: Some(data.join("sf")), ..Options::default() };
    Some(Session::offline(opts).unwrap())
}

fn ids(s: &Session) -> Vec<String> {
    s.sound_catalog().entries.iter().map(|e| e.id.clone()).collect()
}

#[test]
fn the_catalog_lists_every_preset_and_saved_sound() {
    let data = folder("list");
    let Some(s) = offline(&data) else { return };
    let st: Arc<AppState> = s.state();
    assert_eq!(st.sounds.count, 4);
    let cat = s.sound_catalog();
    assert_eq!(cat.revision, st.sounds.revision);
    assert_eq!(ids(&s), ["sf:A.sf2:0:0", "sf:A.sf2:0:33", "sf:A.sf2:128:0", "sf:B.sf2:0:88"]);
    let kit = &cat.entries[2];
    assert_eq!((kit.name.as_str(), kit.category, kit.source, kit.detail.as_str()), ("Standard Kit", Category::DrumsPerc, SoundSource::SoundFont, "A.sf2"));
    assert_eq!(cat.entries[3].category, Category::Pad);
    // Unchanged: the same catalog, not a rebuilt one.
    assert!(Arc::ptr_eq(&cat, &s.sound_catalog()));

    // A saved sound joins it, with a new revision.
    s.send(SoundLibraryCmd::AddPresetAsPatch { file: "B.sf2".into(), bank: 0, program: 88, name: Some("My Pad".into()) }).unwrap();
    let st = s.state();
    assert!(st.sounds.revision > cat.revision);
    let last = s.sound_catalog().entries.last().cloned().unwrap();
    assert_eq!((last.name.as_str(), last.source, last.detail.as_str()), ("My Pad", SoundSource::Saved, "B.sf2"));
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn assigning_a_sound_picks_the_route_its_source_has() {
    let data = folder("assign");
    let Some(s) = offline(&data) else { return };
    // The default sound set's preset: the part's GM voice.
    s.send(SoundsCmd::AssignSound { part: 1, id: "sf:A.sf2:0:33".into() }).unwrap();
    let st = s.state();
    assert_eq!((st.keyboard_parts[1].program, st.keyboard_parts[1].patch.clone()), (33, None));
    // Another font's preset: a saved sound, added once.
    s.send(SoundsCmd::AssignSound { part: 0, id: "sf:B.sf2:0:88".into() }).unwrap();
    s.send(SoundsCmd::AssignSound { part: 2, id: "sf:B.sf2:0:88".into() }).unwrap();
    let st = s.state();
    assert_eq!(st.sound_library.patches.len(), 1);
    let id = st.sound_library.patches[0].patch.id.clone();
    assert_eq!(st.keyboard_parts[0].patch.as_deref(), Some(id.as_str()));
    assert_eq!(st.keyboard_parts[2].patch.as_deref(), Some(id.as_str()));
    // A saved sound, by its catalog id.
    s.send(SoundsCmd::AssignSound { part: 3, id: format!("saved:{id}") }).unwrap();
    assert_eq!(s.state().keyboard_parts[3].patch.as_deref(), Some(id.as_str()));
    // Recents: most recent first, once each.
    let cat = s.sound_catalog();
    assert_eq!(cat.recents, [format!("saved:{id}"), "sf:B.sf2:0:88".into(), "sf:A.sf2:0:33".into()]);
    assert!(cat.entries.iter().find(|e| e.id == "sf:A.sf2:0:33").unwrap().recent);
    // Refused: no such sound, no such part.
    assert!(s.send(SoundsCmd::AssignSound { part: 0, id: "sf:B.sf2:0:1".into() }).is_err());
    assert!(s.send(SoundsCmd::AssignSound { part: 4, id: "sf:A.sf2:0:0".into() }).is_err());
    assert!(s.send(SoundsCmd::AssignSound { part: 0, id: "au:nope".into() }).is_err());
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn favourites_and_recents_are_saved() {
    let data = folder("saved");
    let Some(s) = offline(&data) else { return };
    let rev = s.state().sounds.revision;
    s.send(SoundsCmd::SetSoundFavourite { id: "sf:A.sf2:0:0".into(), on: true }).unwrap();
    assert!(s.state().sounds.revision > rev);
    assert!(s.send(SoundsCmd::SetSoundFavourite { id: "sf:A.sf2:0:5".into(), on: true }).is_err());
    s.send(SoundsCmd::AssignSound { part: 0, id: "sf:A.sf2:0:0".into() }).unwrap();
    assert!(s.send(SoundsCmd::SetSoundCategory { id: "sf:A.sf2:0:0".into(), category: Category::Organ }).is_err());
    drop(s);
    let s = offline(&data).unwrap();
    let cat = s.sound_catalog();
    let piano = cat.entries.iter().find(|e| e.id == "sf:A.sf2:0:0").unwrap();
    assert!(piano.favourite && piano.recent);
    // A saved sound's favourite is its patch's.
    s.send(SoundLibraryCmd::AddPresetAsPatch { file: "B.sf2".into(), bank: 0, program: 88, name: None }).unwrap();
    let id = s.state().sound_library.patches[0].patch.id.clone();
    s.send(SoundsCmd::SetSoundFavourite { id: format!("saved:{id}"), on: true }).unwrap();
    s.send(SoundsCmd::SetSoundCategory { id: format!("saved:{id}"), category: Category::Strings }).unwrap();
    let p = &s.state().sound_library.patches[0].patch;
    assert!(p.favourite);
    assert_eq!(p.category, Category::Strings);
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_preset_auditions_and_an_unknown_plugin_is_refused() {
    let data = folder("audition");
    let Some(s) = offline(&data) else { return };
    s.send(SoundsCmd::AuditionSound { id: "sf:B.sf2:0:88".into() }).unwrap();
    assert_eq!(s.state().sounds.auditioning.as_deref(), Some("sf:B.sf2:0:88"));
    s.send(SoundsCmd::StopSoundAudition).unwrap();
    assert_eq!(s.state().sounds.auditioning, None);
    assert!(s.send(SoundsCmd::AuditionSound { id: "au:aumu dls  appl".into() }).is_err());
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn program_map_rules_take_catalog_ids() {
    let data = folder("rules");
    let Some(s) = offline(&data) else { return };
    // A preset of another font: added to the library once, and the rules name it.
    s.send(SoundLibraryCmd::SetFamilyRule { family: 11, patch: Some("sf:B.sf2:0:88".into()), style: false }).unwrap();
    s.send(SoundLibraryCmd::SetProgramOverride { program: 89, patch: Some("sf:B.sf2:0:88".into()), style: true }).unwrap();
    let st = s.state();
    assert_eq!(st.sound_library.patches.len(), 1);
    let id = st.sound_library.patches[0].patch.id.clone();
    assert_eq!(st.sound_library.map.families[11].as_deref(), Some(id.as_str()));
    assert!(st.sound_library.style_map.overrides.iter().any(|o| o.program == 89 && o.patch == id));
    // A saved sound by its catalog id, then none.
    s.send(SoundLibraryCmd::SetDrumRule { patch: Some(format!("saved:{id}")), style: false }).unwrap();
    assert_eq!(s.state().sound_library.map.drums.as_deref(), Some(id.as_str()));
    s.send(SoundLibraryCmd::SetDrumRule { patch: None, style: false }).unwrap();
    assert_eq!(s.state().sound_library.map.drums, None);
    // Refused: sounds the catalog doesn't have; nothing is added.
    assert!(s.send(SoundLibraryCmd::SetDrumRule { patch: Some("sf:B.sf2:0:1".into()), style: false }).is_err());
    assert!(s.send(SoundLibraryCmd::SetDrumRule { patch: Some("au:nope".into()), style: false }).is_err());
    assert_eq!(s.state().sound_library.patches.len(), 1);
    let _ = std::fs::remove_dir_all(&data);
}
