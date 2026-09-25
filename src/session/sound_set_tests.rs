//! The default sound set (#117): the Auto pick, the saved choice, and the setting through
//! an offline session.

use super::*;
use crate::api::SettingsCmd;
use crate::patches::sf2::tiny_sound_font;
use crate::session::{Options, Session};

fn preset(bank: u16, program: u8) -> Preset {
    Preset { bank, program, name: String::new() }
}

/// A data folder with a SoundFont folder (`<data>/sf`): `Half.sf2` has 64 GM programs,
/// `Full.sf2` has all 128 and a kit, `Kitless.sf2` all 128 and no kit, `Junk.sf2` doesn't
/// parse.
fn folder(tag: &str) -> PathBuf {
    let data = std::env::temp_dir().join(format!("yahaha-ss-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data);
    let sf = data.join("sf");
    std::fs::create_dir_all(&sf).unwrap();
    let gm: Vec<(u16, u8, &str)> = (0..128).map(|p| (0, p, "GM")).collect();
    let with_kit: Vec<(u16, u8, &str)> = gm.iter().copied().chain([(128, 0, "Kit")]).collect();
    std::fs::write(sf.join("Half.sf2"), tiny_sound_font(&gm[..64])).unwrap();
    std::fs::write(sf.join("Full.sf2"), tiny_sound_font(&with_kit)).unwrap();
    std::fs::write(sf.join("Kitless.sf2"), tiny_sound_font(&gm)).unwrap();
    std::fs::write(sf.join("Junk.sf2"), b"not a soundfont").unwrap();
    data
}

#[test]
fn gm_score_counts_bank_0_programs_then_a_kit() {
    assert_eq!(gm_score(&[]), (0, false));
    // Other banks' variations don't count; a program counts once.
    let v = [preset(0, 0), preset(0, 0), preset(8, 1), preset(0, 5), preset(128, 0)];
    assert_eq!(gm_score(&v), (2, true));
    assert!((128, false) > (127, true), "programs come first");
}

#[test]
fn auto_picks_the_most_gm_complete_font() {
    let data = folder("auto");
    let sf = data.join("sf");
    let files = crate::library::sound_font_files(&sf);
    assert_eq!(auto_pick(&sf, &files).as_deref(), Some("Full.sf2"));
    // A tie goes to the first name; an unreadable font never wins over one that reads.
    let two = vec!["Kitless.sf2".to_string(), "Junk.sf2".into()];
    assert_eq!(auto_pick(&sf, &two).as_deref(), Some("Kitless.sf2"));
    assert_eq!(auto_pick(&sf, &[]), None);
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
fn the_setting_is_saved_and_auto_is_the_default() {
    let data = folder("setting");
    let Some(s) = offline(&data) else { return };
    let io = s.state().io.clone();
    assert_eq!(io.sound_fonts, ["Full.sf2", "Half.sf2", "Junk.sf2", "Kitless.sf2"]);
    assert_eq!(io.default_sound_set, None);
    assert_eq!(io.auto_sound_set.as_deref(), Some("Full.sf2"));

    s.send(SettingsCmd::SetDefaultSoundSet { file: Some("Half.sf2".into()) }).unwrap();
    assert_eq!(s.state().io.default_sound_set.as_deref(), Some("Half.sf2"));
    assert!(s.send(SettingsCmd::SetDefaultSoundSet { file: Some("Nope.sf2".into()) }).is_err());
    assert!(s.send(SettingsCmd::SetDefaultSoundSet { file: Some("../x.sf2".into()) }).is_err());
    assert_eq!(s.state().io.default_sound_set.as_deref(), Some("Half.sf2"));
    drop(s);

    // Saved: the next session starts with it, and keeps what a newer yahaha added.
    let file = data.join(FILE_NAME);
    let mut v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(v["defaultSoundSet"], "Half.sf2");
    v["fromTheFuture"] = serde_json::json!(1);
    std::fs::write(&file, v.to_string()).unwrap();
    let s = offline(&data).unwrap();
    assert_eq!(s.state().io.default_sound_set.as_deref(), Some("Half.sf2"));
    s.send(SettingsCmd::SetDefaultSoundSet { file: None }).unwrap();
    assert_eq!(s.state().io.default_sound_set, None);
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(v["defaultSoundSet"], serde_json::Value::Null);
    assert_eq!(v["fromTheFuture"], 1);
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_saved_choice_that_left_the_folder_falls_back_to_auto() {
    let data = folder("gone");
    std::fs::write(data.join(FILE_NAME), r#"{"defaultSoundSet":"Gone.sf2"}"#).unwrap();
    let files = crate::library::sound_font_files(&data.join("sf"));
    let set = SoundSet::open(Some(&data), Some(&data.join("sf")), &files);
    assert_eq!(set.choice.as_deref(), Some("Gone.sf2"));
    assert_eq!(set.resolve(&files).as_deref(), Some("Full.sf2"));
    // A broken file is Auto too.
    std::fs::write(data.join(FILE_NAME), "{").unwrap();
    assert_eq!(SoundSet::open(Some(&data), Some(&data.join("sf")), &files).choice, None);
    let _ = std::fs::remove_dir_all(&data);
}
