//! The data folder's small JSON files: atomic writes, file names from user names, and
//! listing a folder by extension, saving under a name. Shared by Chord Looper banks, racks
//! and the other data files; no dependencies inside the crate.

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

/// Write via a temporary file (flushed to disk) and a rename, so a crash or power loss
/// never leaves half a file. It blocks on the disk: never call it from a real-time thread.
pub fn write_atomic(path: &Path, text: &str) -> Result<()> {
    use std::io::Write;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    let tmp = path.with_extension("tmp");
    let mut file = std::fs::File::create(&tmp).with_context(|| format!("writing {}", tmp.display()))?;
    file.write_all(text.as_bytes()).and_then(|()| file.sync_all()).with_context(|| format!("writing {}", tmp.display()))?;
    drop(file);
    std::fs::rename(&tmp, path).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// The file `file` (a name from `file_name`) names in `dir`, as it is on disk, or None if
/// there is none. On a case-insensitive file system (APFS, the Mac's default; NTFS)
/// "gig.looper.json" opens the existing "Gig.looper.json": this returns that entry, so a
/// save compares with, and renames, the file that is really there.
pub fn existing_file(dir: &Path, file: &str) -> Option<PathBuf> {
    let want = dir.join(file);
    if !want.exists() {
        return None;
    }
    let names: Vec<String> = std::fs::read_dir(dir)
        .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned())).collect())
        .unwrap_or_default();
    if names.iter().any(|n| n == file) {
        return Some(want);
    }
    let lower = file.to_lowercase();
    Some(names.into_iter().find(|n| n.to_lowercase() == lower).map_or(want, |n| dir.join(n)))
}

/// Where a save as `file` in `dir` goes, given the file in use (`own`): Ok(path), after
/// renaming an existing file whose name differs only in case (the same file on a
/// case-insensitive file system) to the spelling asked for; `SaveClash::Exists` when it
/// belongs to something else and `overwrite` is not set.
pub fn save_target(dir: &Path, file: &str, own: Option<&Path>, overwrite: bool) -> Result<PathBuf, SaveClash> {
    let path = dir.join(file);
    let Some(existing) = existing_file(dir, file) else { return Ok(path) };
    if own != Some(existing.as_path()) && !overwrite {
        return Err(SaveClash::Exists);
    }
    if existing != path {
        std::fs::rename(&existing, &path).map_err(|e| SaveClash::Rename(e.to_string()))?;
    }
    Ok(path)
}

/// Why `save_target` refused.
#[derive(Debug)]
pub enum SaveClash {
    /// Another file has that name.
    Exists,
    /// Renaming the file to the new case failed.
    Rename(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_and_listing() {
        assert_eq!(file_name("My/Set: 1", ".looper.json"), "My_Set_ 1.looper.json");
        assert_eq!(file_name("  ", ".looper.json"), "Untitled.looper.json");
        let dir = std::env::temp_dir().join(format!("yahaha-data-files-list-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write_atomic(&dir.join("b.looper.json"), "{}").unwrap();
        write_atomic(&dir.join("A.looper.json"), "{}").unwrap();
        std::fs::write(dir.join("notes.txt"), "x").unwrap();
        let names: Vec<String> = list_files(&dir, ".looper.json").iter().map(|p| file_stem(p, ".looper.json")).collect();
        assert_eq!(names, ["A", "b"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Saving over your own file is fine; over another one needs `overwrite`. On a
    /// case-insensitive file system your own file saved in another case is renamed to it.
    #[test]
    fn save_target_refuses_another_file_unless_overwriting() {
        let dir = std::env::temp_dir().join(format!("yahaha-data-files-save-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let gig = dir.join("Gig.looper.json");
        write_atomic(&gig, "{}").unwrap();
        assert_eq!(save_target(&dir, "New.looper.json", None, false).unwrap(), dir.join("New.looper.json"));
        assert!(matches!(save_target(&dir, "Gig.looper.json", None, false), Err(SaveClash::Exists)));
        assert_eq!(save_target(&dir, "Gig.looper.json", None, true).unwrap(), gig);
        assert_eq!(save_target(&dir, "Gig.looper.json", Some(&gig), false).unwrap(), gig);
        if existing_file(&dir, "gig.looper.json").is_some() {
            // Case-insensitive: "gig" is the file "Gig", renamed when it is your own.
            assert!(matches!(save_target(&dir, "gig.looper.json", None, false), Err(SaveClash::Exists)));
            assert_eq!(save_target(&dir, "gig.looper.json", Some(&gig), false).unwrap(), dir.join("gig.looper.json"));
            assert_eq!(list_files(&dir, ".looper.json"), [dir.join("gig.looper.json")]);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
