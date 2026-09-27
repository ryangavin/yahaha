//! The GM map's auto-fill from the scanned fonts (D3, D4): the main font, the fill, the
//! routes it writes off-thread, and an old default sound set that no longer counts.

use super::*;
use crate::patches::sf2::tiny_sound_font;
use crate::patches::{Layer, Route};
use crate::session::{Options, Session};

/// A data folder with a SoundFont folder (`<data>/sf`): `Main.sf2` has GM programs 0-119
/// and a kit, `Tail.sf2` programs 120-127 only, `Junk.sf2` doesn't parse.
fn folder(tag: &str) -> PathBuf {
    let data = std::env::temp_dir().join(format!("yahaha-gma-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data);
    let sf = data.join("sf");
    std::fs::create_dir_all(&sf).unwrap();
    let main: Vec<(u16, u8, &str)> = (0..120).map(|p| (0, p, "GM")).chain([(128, 0, "Kit")]).collect();
    let tail: Vec<(u16, u8, &str)> = (120..128).map(|p| (0, p, "SFX")).collect();
    std::fs::write(sf.join("Main.sf2"), tiny_sound_font(&main)).unwrap();
    std::fs::write(sf.join("Tail.sf2"), tiny_sound_font(&tail)).unwrap();
    std::fs::write(sf.join("Junk.sf2"), b"not a soundfont").unwrap();
    data
}

#[test]
fn the_most_gm_complete_font_is_main_and_the_others_fill_its_gaps() {
    let data = folder("build");
    let sf = data.join("sf");
    let files = crate::library::sound_font_files(&sf);
    let (auto, best) = build(Some(&sf), &files, None);
    assert_eq!(best.as_deref(), Some("Main.sf2"));
    assert_eq!(auto.get(false, 0).map(|f| f.file.as_str()), Some("Main.sf2"));
    assert_eq!(auto.get(false, 121).map(|f| (f.file.as_str(), f.bank, f.program)), Some(("Tail.sf2", 0, 121)));
    assert_eq!(auto.get(true, 0).map(|f| (f.file.as_str(), f.bank)), Some(("Main.sf2", 128)));
    // No folder, no fonts: nothing to fill from.
    assert_eq!(build(None, &files, None), (AutoFill::default(), None));
    // The `--sf2` pin fills what it has; the others only its gaps.
    let (auto, best) = build(Some(&sf), &files, Some("Tail.sf2"));
    assert_eq!(best.as_deref(), Some("Main.sf2"));
    assert_eq!(auto.get(false, 121).map(|f| f.file.as_str()), Some("Tail.sf2"));
    assert_eq!(auto.get(false, 0).map(|f| f.file.as_str()), Some("Main.sf2"));
    assert_eq!(auto.get(true, 0).map(|f| f.file.as_str()), Some("Main.sf2"));
    let _ = std::fs::remove_dir_all(&data);
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

#[test]
fn the_session_routes_the_auto_fill_and_leaves_the_main_font_unrouted() {
    let data = folder("routes");
    // An old default sound set is ignored: the map decides, and Main is the main font.
    std::fs::write(data.join(FILE_NAME), r#"{"defaultSoundSet":"Tail.sf2"}"#).unwrap();
    let Some(s) = offline(&data) else { return };
    let st = s.state();
    let rows = &st.sound_library.gm_map;
    assert_eq!(rows.len(), 129);
    assert_eq!(rows[0].program, None, "the drums first");
    assert_eq!(rows[0].resolved.layer, Layer::Auto);
    assert_eq!(rows[1].resolved.layer, Layer::Auto);
    assert_eq!(rows[1].resolved.sound.as_deref(), Some("sf:Main.sf2:0:0"));
    assert_eq!(rows[1 + 121].resolved.sound.as_deref(), Some("sf:Tail.sf2:0:121"));
    let mut ctl = s.inner.lock();
    assert_eq!(ctl.sound.native.as_deref(), Some("Main.sf2"));
    let routes = ctl.shared.routes.clone();
    // Tail already has its id (the route uses it): asking again gives the same one.
    let tail = ctl.sound.font_id("Tail.sf2");
    drop(ctl);
    // The main font's own programs and kit need no route (its bank variations still play).
    assert_eq!(routes.bank_route(0, 0), None);
    assert_eq!(routes.bank_drum(0), None);
    // A program only Tail has plays Tail's preset.
    assert!(tail.is_some());
    assert_eq!(routes.bank_route(0, 121), tail.map(|t| Route::sound_font(t, 0, 121)));
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn old_settings_keys_are_kept_when_the_catalog_saves() {
    let data = folder("keys");
    let file = data.join(FILE_NAME);
    std::fs::write(&file, r#"{"defaultSoundSet":"Tail.sf2","fromTheFuture":1}"#).unwrap();
    write_key(Some(&file), "favourites", serde_json::json!(["x"])).unwrap();
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(v["fromTheFuture"], 1);
    assert_eq!(v["favourites"][0], "x");
    let _ = std::fs::remove_dir_all(&data);
}
