//! The data folder's small JSON files: atomic writes, file names from user names, and
//! listing a folder by extension. Shared by Registration banks, playlists, Chord Looper
//! banks and racks; no dependencies inside the crate.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// A file name for a bank, playlist or rack called `name` (characters a file name can't
/// have become `_`).
pub fn file_name(name: &str, ext: &str) -> String {
    let clean: String = name.trim().chars().map(|c| if matches!(c, '/' | '\\' | ':' | '\0') { '_' } else { c }).collect();
    let clean = clean.trim_start_matches('.');
    format!("{}{ext}", if clean.is_empty() { "Untitled" } else { clean })
}

/// The files in `dir` whose names end with `ext` (hidden ones aside), sorted by name.
pub fn list_files(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_file() && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.ends_with(ext) && !n.starts_with('.')))
                .collect()
        })
        .unwrap_or_default();
    v.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
    v
}

/// A file's name without `ext` ("Ballads.regist.json" -> "Ballads").
pub fn file_stem(path: &Path, ext: &str) -> String {
    let n = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    n.strip_suffix(ext).map(str::to_string).unwrap_or(n)
}

/// Write via a temporary file and a rename, so a crash never leaves half a file.
pub fn write_atomic(path: &Path, text: &str) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text).with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, path).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}
