//! The sound library's rules and file format.

use super::*;

fn sf(id: &str, program: u8) -> Patch {
    Patch {
        id: id.into(),
        name: id.to_uppercase(),
        category: Category::guess(0, program),
        tags: vec![],
        favourite: false,
        source: PatchSource::SoundFont { file: "GeneralUser-GS.sf2".into(), bank: 0, program },
        defaults: PatchDefaults::default(),
    }
}

fn global() -> ProgramMap {
    let mut m = ProgramMap::default();
    m.set_family(4, Some("bass".into())); // 32-39
    m.set_family(0, Some("piano".into())); // 0-7
    m.set_override(4, Some("rhodes".into()));
    m.set_override(5, Some("rhodes".into()));
    m.drums = Some("kit".into());
    m
}

#[test]
fn family_rules_cover_eight_programs() {
    let g = global();
    for p in 32..40 {
        let r = resolve(&g, None, false, p);
        assert_eq!((r.patch, r.rule, r.from_style), (Some("bass"), RuleKind::Family, false), "program {p}");
    }
    assert_eq!(resolve(&g, None, false, 40).rule, RuleKind::Fallback);
    assert_eq!(resolve(&g, None, false, 40).patch, None);
    assert_eq!(family_of(127), 15);
    assert_eq!(FAMILY_NAMES[family_of(33)], "Bass");
}

#[test]
fn an_override_beats_its_family() {
    let g = global();
    assert_eq!(resolve(&g, None, false, 4).patch, Some("rhodes"));
    assert_eq!(resolve(&g, None, false, 4).rule, RuleKind::Override);
    assert_eq!(resolve(&g, None, false, 0).patch, Some("piano"));
    assert_eq!(resolve(&g, None, false, 6).patch, Some("piano"));
}

#[test]
fn drum_parts_take_the_drum_rule() {
    let g = global();
    // Rhythm 1 and 2 (ch 9, 10) whatever their program; a kit bank on any channel.
    assert!(is_drum(8, 0) && is_drum(9, 0) && is_drum(12, 127) && is_drum(3, 126));
    assert!(!is_drum(10, 0) && !is_drum(10, 8));
    let r = resolve(&g, None, true, 33);
    assert_eq!((r.patch, r.rule), (Some("kit"), RuleKind::Drums));
    let mut no_kit = g.clone();
    no_kit.drums = None;
    assert_eq!(resolve(&no_kit, None, true, 33).rule, RuleKind::Fallback, "a drum part never takes a melodic rule");
}

/// Bank 8 (MegaVoice, S.Art!) numbers its voices by the Genos Data List, not by GM (#228):
/// they look up, and play, their instrument's GM program.
#[test]
fn genos_bank_8_voices_map_to_their_gm_instrument() {
    let (chord1, bass) = (11, 10);
    assert_eq!(map_program(chord1, 8, 0), 24, "NylonGuitar");
    assert_eq!(FAMILY_NAMES[family_of(map_program(chord1, 8, 0))], "Guitar");
    assert_eq!(FAMILY_NAMES[family_of(map_program(chord1, 8, 2))], "Guitar", "SolidGuitar2 (8/2/PC#4)");
    assert_eq!(FAMILY_NAMES[family_of(map_program(chord1, 8, 17))], "Bass", "ElectricBass on a Chord part");
    assert_eq!(FAMILY_NAMES[family_of(map_program(13, 8, 49))], "Ensemble", "SeattleStrings on the Pad: Slow Strings");
    assert_eq!(FAMILY_NAMES[family_of(map_program(14, 8, 100))], "Ensemble", "PopHaa: Choir Aahs");
    assert_eq!(map_program(bass, 8, 18), 34, "PickBass");
    assert_eq!(map_program(bass, 8, 20), 33, "ActiveBassSlap shares PC# 21 with the EPs; the Bass part plays a bass");
    // GM banks are untouched.
    assert_eq!(map_program(chord1, 0, 0), 0);
    assert_eq!(map_program(chord1, 104, 0), 0);
}

/// Bank 9 (the Ensemble parts' S.Art! voices) keeps bank 8's numbering; bank 104 is GM
/// numbered (#270).
#[test]
fn genos_bank_9_and_104_voices_map_to_their_gm_instrument() {
    let (chord1, pad, phrase1) = (11, 13, 14);
    assert_eq!(map_program(phrase1, 9, 80), 66, "TenorSax 9/66/81: Tenor Sax, not Saw Lead");
    assert_eq!(FAMILY_NAMES[family_of(map_program(pad, 9, 39))], "Ensemble", "Haa 9/32/40: Choir Aahs, not a synth bass");
    assert_eq!(map_program(pad, 9, 43), 40, "Seattle1stViolins: Violin, not Pizzicato");
    assert_eq!(map_program(chord1, 9, 6), 26, "SemiAcoustic: Jazz Guitar");
    assert_eq!(map_program(phrase1, 9, 73), 57, "Trombone 9/65/74");
    assert_eq!(map_program(10, 9, 43), 33, "the Bass part rule still applies");
    for prog in [0, 5, 21, 56, 88] {
        assert_eq!(map_program(chord1, 104, prog), prog, "bank 104 is GM numbered");
    }
}

/// Bank 10 holds the Organ Flutes voices at PC# 1-3: organs, not pianos (#272).
#[test]
fn organ_flutes_map_to_an_organ() {
    for prog in 0..3 {
        assert_eq!(map_program(12, 10, prog), 16, "10/x/PC#{}", prog + 1);
        assert_eq!(FAMILY_NAMES[family_of(map_program(12, 10, prog))], "Organ");
    }
    assert_eq!(map_program(14, 109, 56), 56, "bank 109 is GM numbered: OrchTrumpets");
}

#[test]
fn bank_variations_collapse_onto_their_program() {
    // XG/GS variations of Finger Bass (MSB 0 LSB x, or a Yamaha bank that keeps GM
    // numbering) look up program 33; a Genos-only bank on the Bass part is brought to the
    // GM program the synth plays for it first.
    assert_eq!(map_program(10, 0, 33), 33);
    assert_eq!(map_program(10, 8, 33), 33);
    assert_eq!(map_program(10, 104, 3), 33, "a non-bass number on the Bass part plays Finger Bass");
    assert_eq!(map_program(12, 104, 3), 3);
    let t = Routes::new();
    let mut prog = [None; 128];
    prog[33] = Some(Route::sound_font(2, 0, 34));
    t.write_bank(0, &prog, None);
    for msb in [0, 8, 16, 104] {
        assert_eq!(t.lookup(0, 10, msb, 33), prog[33], "MSB {msb}");
    }
}

#[test]
fn style_rules_win_and_unset_ones_fall_through() {
    let g = global();
    let mut s = ProgramMap::default();
    s.set_family(4, Some("synth-bass".into()));
    s.set_override(0, Some("grand".into()));
    // The style's family rule wins over the global family rule.
    let r = resolve(&g, Some(&s), false, 33);
    assert_eq!((r.patch, r.rule, r.from_style), (Some("synth-bass"), RuleKind::Family, true));
    // The style's override for 0.
    assert_eq!(resolve(&g, Some(&s), false, 0).patch, Some("grand"));
    // A rule the style leaves unset falls through to the global map, override first.
    let r = resolve(&g, Some(&s), false, 4);
    assert_eq!((r.patch, r.from_style), (Some("rhodes"), false));
    assert_eq!(resolve(&g, Some(&s), true, 0).patch, Some("kit"));
    // A style rule beats even a global override: the style is the more specific map.
    s.set_family(0, Some("style-piano".into()));
    assert_eq!(resolve(&g, Some(&s), false, 4).patch, Some("style-piano"));
    // Unmapped anywhere: the fallback.
    assert_eq!(resolve(&g, Some(&s), false, 100).rule, RuleKind::Fallback);
}

#[test]
fn overrides_stay_sorted_and_unique() {
    let mut m = ProgramMap::default();
    m.set_override(9, Some("a".into()));
    m.set_override(3, Some("b".into()));
    m.set_override(9, Some("c".into()));
    assert_eq!(m.overrides.iter().map(|o| (o.program, o.patch.as_str())).collect::<Vec<_>>(), [(3, "b"), (9, "c")]);
    m.set_override(3, None);
    assert_eq!(m.overrides.len(), 1);
    m.set_family(2, Some("c".into()));
    m.drums = Some("c".into());
    m.forget("c");
    assert!(m.is_empty());
}

#[test]
fn ids_are_readable_and_unique() {
    let taken = ["my-bass", "my-bass-2"];
    assert_eq!(new_id("My Bass", taken.iter().copied()), "my-bass-3");
    assert_eq!(new_id("E.Piano 1", taken.iter().copied()), "e-piano-1");
    assert_eq!(new_id("!!!", taken.iter().copied()), "patch");
}

fn library() -> SoundLibrary {
    let mut lib = SoundLibrary { patches: vec![sf("bass", 33), sf("piano", 0), sf("rhodes", 4), sf("kit", 0)], ..SoundLibrary::default() };
    lib.patches[3].source = PatchSource::SoundFont { file: "GeneralUser-GS.sf2".into(), bank: 128, program: 25 };
    lib.patches[0].defaults = PatchDefaults { volume: Some(90), pan: Some(64), reverb: Some(20), chorus: None, octave: -1 };
    lib.patches.push(Patch {
        id: "keys".into(),
        name: "Keys (AU)".into(),
        category: Category::EPiano,
        tags: vec!["plugin".into()],
        favourite: true,
        source: PatchSource::Plugin { component_id: "aumu:abcd:manu".into(), state: "00ff".into() },
        defaults: PatchDefaults::default(),
    });
    lib.map = global();
    let mut s = ProgramMap::default();
    s.set_override(33, Some("keys".into()));
    lib.style_maps.insert("Cool8Beat.S910.sty".into(), s);
    lib
}

#[test]
fn a_library_round_trips_through_its_file() {
    let dir = std::env::temp_dir().join(format!("yahaha-patches-{}", std::process::id()));
    let path = dir.join(FILE_NAME);
    let lib = library();
    lib.save(&path).unwrap();
    let back = SoundLibrary::load(&path).unwrap();
    assert_eq!(back, lib);
    let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(json["version"], VERSION);
    assert_eq!(json["patches"][4]["source"], serde_json::json!({"kind": "plugin", "componentId": "aumu:abcd:manu", "state": "00ff"}));
    assert_eq!(json["patches"][0]["source"], serde_json::json!({"kind": "soundFont", "file": "GeneralUser-GS.sf2", "bank": 0, "program": 33}));
    assert_eq!(json["patches"][2]["category"], "ePiano");
    assert_eq!(json["map"]["families"].as_array().unwrap().len(), 16);
    // No file yet: an empty library.
    assert_eq!(SoundLibrary::load(&dir.join("none.json")).unwrap(), SoundLibrary::default());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn old_and_future_versions() {
    // A bare patch list (no version): the patches, empty maps.
    let bare = serde_json::to_string(&vec![sf("bass", 33)]).unwrap();
    let lib = SoundLibrary::from_json(&bare).unwrap();
    assert_eq!((lib.version, lib.patches.len(), lib.map.is_empty()), (VERSION, 1, true));
    // A newer format is refused, not half read.
    let future = r#"{"version": 99, "patches": []}"#;
    assert!(SoundLibrary::from_json(future).unwrap_err().to_string().contains("newer"));
    assert!(SoundLibrary::from_json(r#"{"patches": []}"#).is_err());
    assert!(SoundLibrary::from_json("[1, 2]").is_err());
    // Missing optional fields take their defaults; rules naming missing patches go.
    let v1 = r#"{"version": 1, "patches": [{"id": "b", "name": "B", "source": {"kind": "soundFont", "file": "x.sf2", "bank": 0, "program": 33}}],
                 "map": {"families": [null,null,null,null,"b",null,null,null,null,null,null,null,null,null,null,"gone"], "drums": "gone",
                         "overrides": [{"program": 200, "patch": "b"}, {"program": 72, "patch": "b"}, {"program": 72, "patch": "b"}]}}"#;
    let lib = SoundLibrary::from_json(v1).unwrap();
    assert_eq!(lib.patches[0].category, Category::Piano);
    assert_eq!(lib.map.families[4].as_deref(), Some("b"));
    assert_eq!(lib.map.families[15], None);
    assert_eq!(lib.map.drums, None);
    assert_eq!(lib.map.overrides.iter().map(|o| o.program).collect::<Vec<_>>(), [72]);
}

#[test]
fn normalize_fixes_duplicate_ids_and_ranges() {
    let mut lib = SoundLibrary { patches: vec![sf("a", 1), sf("a", 2), sf("", 3)], ..SoundLibrary::default() };
    lib.patches[0].defaults.octave = 7;
    lib.patches[0].defaults.volume = Some(200);
    lib.normalize();
    let ids: Vec<&str> = lib.patches.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids[0], "a");
    assert_ne!(ids[1], "a");
    assert!(!ids[2].is_empty() && ids[2] != ids[1]);
    assert_eq!(lib.patches[0].defaults.octave, 2);
    assert_eq!(lib.patches[0].defaults.volume, Some(127));
}

#[test]
fn an_import_merges_with_new_ids_and_follows_its_maps() {
    let mut lib = library();
    let mut other = SoundLibrary { patches: vec![sf("bass", 38), sf("pad", 89)], ..SoundLibrary::default() };
    other.map.set_family(11, Some("pad".into()));
    other.map.set_family(4, Some("bass".into()));
    let added = lib.merge(other, true);
    assert_eq!(added, 2);
    assert_eq!(lib.patches.len(), 7);
    let renamed = &lib.patches[5];
    assert_ne!(renamed.id, "bass", "the imported bass got a new id");
    assert_eq!(lib.map.families[4].as_deref(), Some(renamed.id.as_str()), "and the imported rule follows it");
    assert_eq!(lib.map.families[11].as_deref(), Some("pad"));
    // Patches only: the maps stay.
    let mut lib2 = library();
    let mut other = SoundLibrary { patches: vec![sf("pad", 89)], ..SoundLibrary::default() };
    other.map.set_family(0, Some("pad".into()));
    lib2.merge(other, false);
    assert_eq!(lib2.map.families[0].as_deref(), Some("piano"));
}

#[test]
fn plugin_patches_say_why_they_play_the_fallback() {
    let lib = library();
    let fonts = vec!["GeneralUser-GS.sf2".to_string()];
    let want = if cfg!(feature = "plugins") { None } else { Some("needs plugin hosting (#91)") };
    assert_eq!(unavailable_reason(lib.patch("keys").unwrap(), &fonts).as_deref(), want);
    assert_eq!(unavailable_reason(lib.patch("bass").unwrap(), &fonts), None);
    assert!(unavailable_reason(lib.patch("bass").unwrap(), &[]).unwrap().contains("not in the SoundFont folder"));
}

/// Corpus check: styles from every corpus folder with a small map (a few family rules and
/// the drums). Every program each style sends a part resolves (to a patch or the
/// fallback) without a panic, drum parts only ever to the drum rule, and the route table
/// agrees with the resolution.
#[test]
fn corpus_styles_resolve_every_channel() {
    let files = crate::library::corpus_styles();
    if files.is_empty() {
        eprintln!("corpus missing; skipping");
        return;
    }
    let mut lib = library();
    lib.map.set_family(5, Some("piano".into()));
    lib.map.set_family(6, Some("rhodes".into()));
    let t = Routes::new();
    let route_of = |id: &str| match &lib.patch(id).unwrap().source {
        PatchSource::SoundFont { bank, program, .. } => Some(Route::sound_font(0, *bank, *program)),
        PatchSource::Plugin { .. } => None,
    };
    let mut prog = [None; 128];
    for (p, r) in prog.iter_mut().enumerate() {
        *r = resolve(&lib.map, None, false, p as u8).patch.and_then(route_of);
    }
    t.write_bank(0, &prog, resolve(&lib.map, None, true, 0).patch.and_then(route_of));
    // Every tenth style, so each folder is represented and the test stays quick.
    let (mut styles, mut resolved, mut mapped) = (0, 0, 0);
    for f in files.iter().step_by(10) {
        let Ok(style) = crate::sff::Style::load(f) else { continue };
        let prep = crate::engine::Prepared::new(&style);
        let used = prep.program_changes();
        styles += 1;
        for d in 8..16u8 {
            if prep.setups.iter().any(|s| s.voices[d as usize].is_some()) {
                assert!(used.iter().any(|u| u.0 == d), "{}: ch {} has a voice but no program listed", f.display(), d + 1);
            }
        }
        for &(ch, msb, _, pc) in &used {
            assert!((8..16).contains(&ch));
            let drum = is_drum(ch, msb);
            let p = if drum { pc } else { map_program(ch, msb, pc) };
            let r = resolve(&lib.map, None, drum, p);
            if drum {
                assert!(matches!(r.rule, RuleKind::Drums), "{}: ch {} drums", f.display(), ch + 1);
            }
            assert_eq!(t.lookup(0, ch, msb, pc), r.patch.and_then(route_of), "{}: ch {} {msb}/{pc}", f.display(), ch + 1);
            resolved += 1;
            mapped += r.patch.is_some() as usize;
        }
    }
    eprintln!("{styles} styles, {resolved} programs, {mapped} mapped");
    assert!(styles > 0 && mapped > 0);
}

#[test]
fn categories_follow_the_genos_tabs() {
    assert_eq!(Category::ALL.len(), 13);
    assert_eq!(Category::guess(0, 4), Category::EPiano);
    assert_eq!(Category::guess(0, 33), Category::Bass);
    assert_eq!(Category::guess(128, 0), Category::DrumsPerc);
    assert_eq!(Category::guess(0, 53), Category::Choir);
    assert_eq!(serde_json::to_string(&Category::SaxWoodwind).unwrap(), "\"saxWoodwind\"");
    assert_eq!(serde_json::to_string(&Category::EPiano).unwrap(), "\"ePiano\"");
}

/// B4 (review of #106): an import whose patch ids collide in a chain keeps each imported
/// rule on its own patch.
#[test]
fn a_merge_keeps_rules_on_their_patches_when_ids_chain() {
    let p = |id: &str, name: &str, prog: u8| Patch { id: id.into(), name: name.into(), ..sf(id, prog) };
    let mut mine = SoundLibrary { patches: vec![p("bass", "Bass", 1)], ..SoundLibrary::default() };
    let mut theirs = SoundLibrary { patches: vec![p("bass", "Bass", 33), p("bass-2", "Bass 2", 34)], ..SoundLibrary::default() };
    theirs.map.set_family(4, Some("bass".into()));
    theirs.map.set_family(5, Some("bass-2".into()));
    theirs.map.set_override(7, Some("bass-2".into()));
    theirs.map.drums = Some("bass".into());
    assert_eq!(mine.merge(theirs, true), 2);
    let prog_of = |lib: &SoundLibrary, id: &str| match &lib.patch(id).unwrap().source {
        PatchSource::SoundFont { program, .. } => *program,
        _ => 255,
    };
    let ids: Vec<&str> = mine.patches.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids.iter().collect::<std::collections::HashSet<_>>().len(), 3, "{ids:?}");
    assert_eq!(prog_of(&mine, mine.map.families[4].as_deref().unwrap()), 33);
    assert_eq!(prog_of(&mine, mine.map.families[5].as_deref().unwrap()), 34);
    assert_eq!(prog_of(&mine, mine.map.override_of(7).unwrap()), 34);
    assert_eq!(prog_of(&mine, mine.map.drums.as_deref().unwrap()), 33);
    assert_eq!(prog_of(&mine, "bass"), 1, "my own patch is untouched");
}
