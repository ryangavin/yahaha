//! Tests of `library` (in yahaha-sff) that read the facade's own `src/`.

use std::path::Path;

/// No module walks folders on its own: a hand-rolled walk is how corpus tests came to
/// see only `.sty` files, or only one folder. Everything goes through `style_files`
/// (`main.rs` only looks for a `.sf2` in `soundfonts/`).
#[test]
fn only_the_library_walks_folders() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let needle = ["read", "_dir("].concat();
    for f in std::fs::read_dir(&src).unwrap().flatten().map(|e| e.path()) {
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        if !name.ends_with(".rs") || name == "library.rs" || name == "library_tests.rs" {
            continue;
        }
        let text = std::fs::read_to_string(&f).unwrap();
        let n = text.matches(&needle).count();
        // data_files.rs lists one folder of saved data files (`.rack.json`, `.looper.json`),
        // and looks up a file name's spelling on disk (`existing_file`), never styles.
        let allowed = match name.as_str() {
            "main.rs" => 1,
            "data_files.rs" => 2,
            _ => 0,
        };
        assert!(n <= allowed, "src/{name} lists folders itself ({n}x); use library::style_files");
    }
}
