//! The Multi Pad bank list: every `.pad` file under the library's folders, found by the
//! same kind of walk as the style library (recursive, symlinked folders once, hidden files
//! and folders skipped), in folder-then-name order.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// A bank file the library found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BankFile {
    pub path: PathBuf,
    /// The file name without its extension.
    pub name: String,
    /// Folder relative to the scanned root, `/`-separated ("" at the root).
    pub folder: String,
}

fn is_hidden(p: &Path) -> bool {
    p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('.'))
}

/// A Multi Pad bank file: `.pad`, any case.
pub fn is_pad(p: &Path) -> bool {
    !is_hidden(p) && p.extension().and_then(|x| x.to_str()).is_some_and(|x| x.eq_ignore_ascii_case("pad"))
}

/// Every bank under `roots` (folders recursively; a root that is itself a `.pad` file
/// counts), each path once, sorted by folder then name (case-insensitive).
pub fn scan(roots: &[PathBuf]) -> Vec<BankFile> {
    let mut seen = HashSet::new();
    let mut files = HashSet::new();
    let mut out = Vec::new();
    for root in roots {
        if root.is_file() {
            if is_pad(root) && files.insert(root.clone()) {
                let parent = root.parent().unwrap_or(Path::new(""));
                out.push(bank_file(root, parent));
            }
            continue;
        }
        let mut stack = vec![root.clone()];
        while let Some(d) = stack.pop() {
            if !seen.insert(d.canonicalize().unwrap_or_else(|_| d.clone())) {
                continue;
            }
            for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                let q = e.path();
                if q.is_dir() {
                    if !is_hidden(&q) {
                        stack.push(q);
                    }
                } else if q.is_file() && is_pad(&q) && files.insert(q.clone()) {
                    out.push(bank_file(&q, root));
                }
            }
        }
    }
    out.sort_by_cached_key(|b| (b.folder.to_lowercase(), b.name.to_lowercase(), b.path.clone()));
    out
}

pub fn bank_file(path: &Path, root: &Path) -> BankFile {
    let name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let folder = path
        .parent()
        .and_then(|p| p.strip_prefix(root).ok())
        .map(|p| p.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect::<Vec<_>>().join("/"))
        .unwrap_or_default();
    BankFile { path: path.to_path_buf(), name, folder }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_pad_files_in_any_case_below_the_roots_in_folder_then_name_order() {
        let root = std::env::temp_dir().join(format!("yahaha-padlib-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for rel in ["b.pad", "Pads/a.PAD", "Pads/deeper/c.pad", "x.sty", ".hidden.pad", ".dot/d.pad"] {
            let p = root.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, b"x").unwrap();
        }
        let got: Vec<_> = scan(std::slice::from_ref(&root)).into_iter().map(|b| format!("{}|{}", b.folder, b.name)).collect();
        assert_eq!(got, ["|b", "Pads|a", "Pads/deeper|c"]);
        // A root that is a file counts; the same file twice is listed once.
        let f = root.join("b.pad");
        assert_eq!(scan(&[f.clone(), root.clone()]).len(), 3);
        let _ = std::fs::remove_dir_all(&root);
    }
}
