//! Tests of `library` (in yahaha-sff) that read the facade's `src/` and the layer crates'
//! `crates/*/src`.

use std::path::{Path, PathBuf};

/// The `.rs` files directly in `dir`, and with `deep`, in its subfolders too.
fn rust_files(dir: &Path, deep: bool, out: &mut Vec<PathBuf>) {
    for p in std::fs::read_dir(dir).unwrap().flatten().map(|e| e.path()) {
        if p.is_dir() {
            if deep {
                rust_files(&p, deep, out);
            }
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// No module walks folders on its own: a hand-rolled walk is how corpus tests came to
/// see only `.sty` files, or only one folder. Everything goes through `style_files` (or
/// `files_in` and `sound_font_files`), in `crates/yahaha-sff/src/library.rs`. Scans the
/// facade's top-level `src/` files and every layer crate's `src/`, recursively.
#[test]
fn only_the_library_walks_folders() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_files(&root.join("src"), false, &mut files);
    let crates: Vec<PathBuf> = std::fs::read_dir(root.join("crates")).unwrap().flatten().map(|e| e.path().join("src")).filter(|p| p.is_dir()).collect();
    assert!(crates.len() >= 5, "found only {crates:?}");
    for src in &crates {
        rust_files(src, true, &mut files);
    }
    let needle = ["read", "_dir("].concat();
    for f in files {
        let rel = f.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        if rel == "crates/yahaha-sff/src/library.rs" || rel == "src/library_tests.rs" {
            continue;
        }
        let n = std::fs::read_to_string(&f).unwrap().matches(&needle).count();
        let allowed = match rel.as_str() {
            // One folder of saved data files (`.rack.json`, `.looper.json`), and a file
            // name's spelling on disk (`existing_file`), never styles.
            "crates/yahaha-core/src/data_files.rs" => 2,
            // Multi Pad bank files (`.pad`), never styles.
            "crates/yahaha-engine/src/multipad/library.rs" => 1,
            // A plugin's own user presets, never styles.
            "crates/yahaha-synth/src/plugin/presets.rs" => 1,
            _ => 0,
        };
        assert!(n <= allowed, "{rel} lists folders itself ({n}x); use library::style_files");
    }
}
