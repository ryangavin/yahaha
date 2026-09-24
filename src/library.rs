//! The style library behind the browser and ←/→: a recursive scan of the folders given to
//! `yahaha play`, a light index (name, tempo, time signature, sections) built on a
//! background thread, and the one order both use: folder, then name.
//!
//! Nothing here runs on the engine or MIDI threads. The UI thread owns the `Library` and
//! drains index results from a channel between frames.

use crate::sff::{SectionId, Summary};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

/// Style file extensions, matched case-insensitively.
pub const EXTENSIONS: [&str; 7] = ["sty", "prs", "sst", "bcs", "pcs", "pst", "fps"];

/// Dot-files and dot-folders (`.Trashes`, `.git`, AppleDouble `._x.sty`) are never styles.
fn is_hidden(p: &Path) -> bool {
    p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('.'))
}

pub fn is_style(p: &Path) -> bool {
    !is_hidden(p) && p.extension().and_then(|x| x.to_str()).is_some_and(|x| EXTENSIONS.iter().any(|e| x.eq_ignore_ascii_case(e)))
}

/// Every style file under `root`, recursively, in path order: every extension in
/// [`EXTENSIONS`], any case, hidden files and folders skipped. The one scan behind the
/// browser, the oracle and every corpus test, so none of them sees a partial corpus.
pub fn style_files(root: &Path) -> Vec<PathBuf> {
    let mut v = walk(root, &mut std::collections::HashSet::new());
    v.sort();
    v
}

/// Every style under the checkout's `corpus/` (empty when there is none), in path order.
#[cfg(test)]
pub fn corpus_styles() -> Vec<PathBuf> {
    style_files(&Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus"))
}

/// The files directly in `dir` (not its folders), in path order: for tests that check what
/// a folder of fixtures holds (tests/reference), so they need not list folders themselves.
#[cfg(test)]
pub fn files_in(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_file()).collect();
    v.sort();
    v
}

/// The style files under `root`. Symlinked folders are followed, once each (`seen` is
/// shared across roots), so a link loop can't hang the scan.
fn walk(root: &Path, seen: &mut std::collections::HashSet<PathBuf>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
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
            } else if q.is_file() && is_style(&q) {
                out.push(q);
            }
        }
    }
    out
}

/// What the index knows about a file.
#[derive(Debug, Clone, PartialEq)]
pub enum Info {
    /// Not indexed yet.
    Pending,
    Ok(Summary),
    /// It doesn't parse; the row shows why.
    Err(String),
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub path: PathBuf,
    /// Folder relative to the scanned root, `/`-separated; the category. Empty at the root.
    pub folder: String,
    pub info: Info,
    stem: String,
    // Lowercased copies for sorting and filtering.
    stem_lc: String,
    name_lc: String,
    folder_lc: String,
}

impl Entry {
    fn new(path: PathBuf, folder: String) -> Entry {
        let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
        let folder_lc = folder.to_lowercase();
        let name_lc = stem.to_lowercase();
        Entry { path, folder, info: Info::Pending, stem_lc: name_lc.clone(), stem, name_lc, folder_lc }
    }

    /// The SFF name marker, or the file stem when there is none (or it isn't indexed yet).
    pub fn name(&self) -> &str {
        match &self.info {
            Info::Ok(s) if !s.name.is_empty() => &s.name,
            _ => &self.stem,
        }
    }

    /// Case-insensitive substring match on the style name, the file name or the folder.
    /// `query_lc` is lowercase. The file name never changes, so a row that matched by it
    /// stays in the list when the index brings in the style name.
    pub fn matches(&self, query_lc: &str) -> bool {
        self.name_lc.contains(query_lc) || self.stem_lc.contains(query_lc) || self.folder_lc.contains(query_lc)
    }
}

pub struct Library {
    entries: Vec<Entry>,
    /// Entry ids in display order.
    order: Vec<usize>,
}

impl Library {
    /// Find every style under `paths`: folders recursively, files as given. With one folder,
    /// categories are relative to it; with several, each starts with its folder's name.
    pub fn scan(paths: &[PathBuf]) -> Library {
        let dirs = paths.iter().filter(|p| p.is_dir()).count();
        let mut entries = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for root in paths {
            if !root.is_dir() {
                // A named file that's missing becomes an error row; a pipe or device is
                // skipped, since opening one can block forever.
                if root.is_file() || !root.exists() {
                    entries.push(Entry::new(root.clone(), String::new()));
                }
                continue;
            }
            let prefix = if dirs > 1 { root.file_name().map(|n| n.to_string_lossy().to_string()) } else { None };
            for q in walk(root, &mut seen) {
                let rel = q.parent().and_then(|d| d.strip_prefix(root).ok()).unwrap_or(Path::new(""));
                let parts = prefix.iter().cloned().chain(rel.components().map(|c| c.as_os_str().to_string_lossy().to_string()));
                let folder = parts.collect::<Vec<_>>().join("/");
                entries.push(Entry::new(q, folder));
            }
        }
        Library::from_entries(entries)
    }

    fn from_entries(entries: Vec<Entry>) -> Library {
        let mut lib = Library { order: (0..entries.len()).collect(), entries };
        lib.sort();
        lib
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entry(&self, id: usize) -> &Entry {
        &self.entries[id]
    }

    /// Entry ids, folder then name.
    pub fn order(&self) -> &[usize] {
        &self.order
    }

    /// Record what the index (or a failed load) found. Call `sort` afterwards, since the
    /// name can change.
    pub fn set_info(&mut self, id: usize, info: Info) {
        let e = &mut self.entries[id];
        e.info = info;
        e.name_lc = e.name().to_lowercase();
    }

    pub fn sort(&mut self) {
        let entries = &self.entries;
        self.order.sort_by(|&a, &b| {
            let (a, b) = (&entries[a], &entries[b]);
            (&a.folder_lc, &a.name_lc, &a.path).cmp(&(&b.folder_lc, &b.name_lc, &b.path))
        });
    }

    /// Position of an entry in display order.
    pub fn position(&self, id: usize) -> usize {
        self.order.iter().position(|&i| i == id).unwrap_or(0)
    }

    /// The next style from `id` in display order (wrapping), skipping files known not to
    /// parse. Returns `id` itself when there is nowhere else to go.
    pub fn step(&self, id: usize, d: i8) -> usize {
        let n = self.order.len();
        let mut pos = self.position(id);
        for _ in 1..n {
            pos = if d > 0 { (pos + 1) % n } else { (pos + n - 1) % n };
            let next = self.order[pos];
            if !matches!(self.entries[next].info, Info::Err(_)) {
                return next;
            }
        }
        id
    }

    /// Entry ids matching `query` (case-insensitive substring of name or folder), in
    /// display order. An empty query matches everything.
    pub fn filter(&self, query: &str) -> Vec<usize> {
        let q = query.to_lowercase();
        self.order.iter().copied().filter(|&i| self.entries[i].matches(&q)).collect()
    }

    /// Index every entry on a background thread, in display order so the top of the list
    /// fills first. Results arrive as (entry id, info); drain them with `apply`.
    pub fn spawn_indexer(&self) -> mpsc::Receiver<(usize, Info)> {
        let jobs: Vec<(usize, PathBuf)> = self.order.iter().map(|&i| (i, self.entries[i].path.clone())).collect();
        let (tx, rx) = mpsc::channel();
        // Normal priority, like the UI. If the thread can't start, the list keeps file
        // names and loading still works.
        let _ = std::thread::Builder::new().name("yahaha-index".into()).spawn(move || {
            for (id, path) in jobs {
                if tx.send((id, index_one(&path))).is_err() {
                    return; // the UI has gone
                }
            }
        });
        rx
    }

    /// Take whatever the indexer has finished. Returns how many entries changed.
    pub fn apply(&mut self, rx: &mpsc::Receiver<(usize, Info)>) -> usize {
        let mut n = 0;
        while let Ok((id, info)) = rx.try_recv() {
            // A failed load already marked the entry; the index doesn't clear that.
            if id < self.entries.len() && !matches!(self.entries[id].info, Info::Err(_)) {
                self.set_info(id, info);
                n += 1;
            }
        }
        if n > 0 {
            self.sort();
        }
        n
    }

    /// How many entries are still waiting for the indexer.
    pub fn pending(&self) -> usize {
        self.entries.iter().filter(|e| e.info == Info::Pending).count()
    }
}

/// Index one file. Never panics: a file that doesn't parse becomes an error row.
pub fn index_one(path: &Path) -> Info {
    // Only regular files: opening a named pipe blocks until a writer appears.
    if path.exists() && !path.is_file() {
        return Info::Err("not a regular file".into());
    }
    match std::panic::catch_unwind(|| Summary::load(path)) {
        Ok(Ok(s)) => Info::Ok(s),
        Ok(Err(e)) => Info::Err(format!("{e:#}")),
        Err(_) => Info::Err("parser panicked".into()),
    }
}

/// Short section list for the browser, e.g. "Main ABCD · Intro ABC · Ending ABC · Fill ABCD · Break".
pub fn sections_text(sections: &[SectionId]) -> String {
    const L: [char; 4] = ['A', 'B', 'C', 'D'];
    let group = |label: &str, f: fn(&SectionId) -> Option<u8>| -> Option<String> {
        let letters: String = sections.iter().filter_map(f).map(|i| L[i as usize & 3]).collect();
        (!letters.is_empty()).then(|| format!("{label} {letters}"))
    };
    let mut parts: Vec<String> = [
        group("Main", |s| if let SectionId::Main(i) = s { Some(*i) } else { None }),
        group("Intro", |s| if let SectionId::Intro(i) = s { Some(*i) } else { None }),
        group("Ending", |s| if let SectionId::Ending(i) = s { Some(*i) } else { None }),
        group("Fill", |s| if let SectionId::Fill(i) = s { Some(*i) } else { None }),
    ]
    .into_iter()
    .flatten()
    .collect();
    if sections.contains(&SectionId::Break) {
        parts.push("Break".into());
    }
    parts.join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A fresh, empty directory under the system temp dir.
    fn temp_dir(tag: &str) -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let d = std::env::temp_dir().join(format!("yahaha-lib-{}-{tag}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn chunk(id: &[u8], body: &[u8]) -> Vec<u8> {
        let mut v = id.to_vec();
        v.extend_from_slice(&(body.len() as u32).to_be_bytes());
        v.extend_from_slice(body);
        v
    }

    fn meta(ty: u8, data: &[u8]) -> Vec<u8> {
        let mut v = vec![0x00, 0xFF, ty, data.len() as u8];
        v.extend_from_slice(data);
        v
    }

    /// A minimal style: name marker (if any), tempo, time signature, then `sections`, each
    /// one bar with a note. A trailing junk chunk shows the index never needs it.
    fn style(name: &str, bpm: u32, ts: (u8, u8), sections: &[&str]) -> Vec<u8> {
        let mut trk = Vec::new();
        if !name.is_empty() {
            trk.extend(meta(0x03, name.as_bytes()));
        }
        let us = 60_000_000 / bpm;
        trk.extend(meta(0x51, &[(us >> 16) as u8, (us >> 8) as u8, us as u8]));
        trk.extend(meta(0x58, &[ts.0, ts.1.trailing_zeros() as u8, 24, 8]));
        trk.extend(meta(0x06, b"SFF2"));
        for s in sections {
            trk.extend(meta(0x06, s.as_bytes()));
            trk.extend_from_slice(&[0x00, 0x9B, 60, 100, 0x83, 0x00, 0x8B, 60, 0]);
        }
        trk.extend_from_slice(&[0x00, 0xFF, 0x2F, 0]);
        let mut out = chunk(b"MThd", &[0, 0, 0, 1, 0, 96]);
        out.extend(chunk(b"MTrk", &trk));
        out.extend(chunk(b"CASM", b"\xFF\xFF garbage the index never reads"));
        out
    }

    fn write(root: &Path, rel: &str, bytes: &[u8]) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }

    /// Index synchronously, like the background thread does.
    fn index_all(lib: &mut Library) {
        let rx = lib.spawn_indexer();
        let mut got = 0;
        while got < lib.len() {
            let (id, info) = rx.recv_timeout(std::time::Duration::from_secs(10)).expect("indexer result");
            lib.set_info(id, info);
            got += 1;
        }
        lib.sort();
    }

    fn names(lib: &Library, ids: &[usize]) -> Vec<String> {
        ids.iter().map(|&i| format!("{}|{}", lib.entry(i).folder, lib.entry(i).name())).collect()
    }

    #[test]
    fn scan_is_recursive_and_matches_every_extension_in_any_case() {
        let root = temp_dir("scan");
        let s = style("", 120, (4, 4), &["Main A"]);
        for (i, ext) in EXTENSIONS.iter().enumerate() {
            let ext = if i % 2 == 0 { ext.to_uppercase() } else { ext.to_string() };
            write(&root, &format!("d{i}/deeper/f{i}.{ext}"), &s);
        }
        write(&root, "top.Sty", &s);
        write(&root, "readme.txt", b"no");
        write(&root, "song.mid", &s);
        write(&root, "._top.sty", b"AppleDouble");
        let lib = Library::scan(std::slice::from_ref(&root));
        assert_eq!(lib.len(), EXTENSIONS.len() + 1);
        let top = lib.order().iter().map(|&i| lib.entry(i)).find(|e| e.name() == "top").unwrap();
        assert_eq!(top.folder, "");
        let deep = lib.order().iter().map(|&i| lib.entry(i)).find(|e| e.name() == "f3").unwrap();
        assert_eq!(deep.folder, "d3/deeper");
        let _ = std::fs::remove_dir_all(root);
    }

    /// The helper every corpus test and tool enumerates styles with finds a style in any
    /// container, not only `.sty`, in any subfolder.
    #[test]
    fn style_files_finds_every_style_type_in_subfolders() {
        let root = temp_dir("style_files");
        let s = style("Same", 100, (4, 4), &["Main A"]);
        write(&root, "a.sty", &s);
        write(&root, "Registrations/b.prs", &s);
        write(&root, "Registrations/deeper/c.SST", &s);
        write(&root, "notes.txt", b"no");
        write(&root, ".hidden/d.prs", &s);
        let got: Vec<String> = style_files(&root)
            .iter()
            .map(|p| p.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/"))
            .collect();
        assert_eq!(got, ["Registrations/b.prs", "Registrations/deeper/c.SST", "a.sty"]);
        let _ = std::fs::remove_dir_all(root);
    }

    /// No module walks folders on its own: a hand-rolled walk is how corpus tests came to
    /// see only `.sty` files, or only one folder. Everything goes through `style_files`
    /// (`main.rs` only looks for a `.sf2` in `soundfonts/`).
    #[test]
    fn only_the_library_walks_folders() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let needle = ["read", "_dir("].concat();
        for f in std::fs::read_dir(&src).unwrap().flatten().map(|e| e.path()) {
            let name = f.file_name().unwrap().to_string_lossy().to_string();
            if !name.ends_with(".rs") || name == "library.rs" {
                continue;
            }
            let text = std::fs::read_to_string(&f).unwrap();
            let n = text.matches(&needle).count();
            let allowed = if name == "main.rs" { 1 } else { 0 };
            assert!(n <= allowed, "src/{name} lists folders itself ({n}x); use library::style_files");
        }
    }

    #[test]
    fn index_reads_name_tempo_timesig_and_sections() {
        let root = temp_dir("index");
        write(&root, "Ballads/slow.sty", &style("Slow Ballad", 72, (6, 8), &["Intro A", "Main A", "Main B", "Fill In BA", "Ending A"]));
        write(&root, "Ballads/noname.sty", &style("", 90, (3, 4), &["Main A"]));
        let mut lib = Library::scan(std::slice::from_ref(&root));
        assert!(lib.order().iter().all(|&i| lib.entry(i).info == Info::Pending));
        index_all(&mut lib);
        assert_eq!(names(&lib, lib.order()), ["Ballads|noname", "Ballads|Slow Ballad"]);
        let Info::Ok(s) = &lib.entry(lib.order()[1]).info else { panic!("indexed") };
        assert!((s.bpm - 72.0).abs() < 0.1);
        assert_eq!(s.timesig, (6, 8));
        assert_eq!(sections_text(&s.sections), "Main AB · Intro A · Ending A · Break");
        let Info::Ok(s) = &lib.entry(lib.order()[0]).info else { panic!("indexed") };
        assert_eq!(s.timesig, (3, 4));
        assert_eq!(lib.pending(), 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn sorted_by_folder_then_name_from_the_marker() {
        let root = temp_dir("sort");
        let st = |n: &str| style(n, 120, (4, 4), &["Main A"]);
        // File names sort the other way from the SFF names.
        write(&root, "Pop/a.sty", &st("Zebra Pop"));
        write(&root, "Pop/b.sty", &st("apple pop"));
        write(&root, "Jazz/z.sty", &st("Swing"));
        write(&root, "Jazz/Latin/y.sty", &st("Bossa"));
        write(&root, "loose.prs", &st("Loose"));
        let mut lib = Library::scan(std::slice::from_ref(&root));
        index_all(&mut lib);
        assert_eq!(
            names(&lib, lib.order()),
            ["|Loose", "Jazz|Swing", "Jazz/Latin|Bossa", "Pop|apple pop", "Pop|Zebra Pop"]
        );
        // ←/→ walk the same order, wrapping.
        let o = lib.order().to_vec();
        assert_eq!(lib.step(o[0], 1), o[1]);
        assert_eq!(lib.step(o[4], 1), o[0]);
        assert_eq!(lib.step(o[0], -1), o[4]);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn broken_files_become_error_rows_and_are_skipped_by_step() {
        let root = temp_dir("errors");
        write(&root, "a.sty", &style("A", 120, (4, 4), &["Main A"]));
        write(&root, "b.sty", b"MThd garbage");
        write(&root, "c.sty", &[]);
        write(&root, "d.sty", &style("D no sections", 120, (4, 4), &[]));
        let mut good = style("E", 120, (4, 4), &["Main A"]);
        good.truncate(30); // cut inside the track
        write(&root, "e.sty", &good);
        write(&root, "f.sty", &style("F", 120, (4, 4), &["Main B"]));
        let mut lib = Library::scan(std::slice::from_ref(&root));
        index_all(&mut lib);
        let o = lib.order().to_vec();
        let errs: Vec<&str> = o.iter().filter(|&&i| matches!(lib.entry(i).info, Info::Err(_))).map(|&i| lib.entry(i).name()).collect();
        assert_eq!(errs, ["b", "c", "d", "e"]);
        // Error rows show the file stem and a reason.
        let Info::Err(why) = &lib.entry(o[3]).info else { panic!() };
        assert!(why.contains("no section markers"), "{why}");
        // Stepping from A goes straight to F and back.
        let a = o.iter().copied().find(|&i| lib.entry(i).name() == "A").unwrap();
        let f = o.iter().copied().find(|&i| lib.entry(i).name() == "F").unwrap();
        assert_eq!(lib.step(a, 1), f);
        assert_eq!(lib.step(f, 1), a);
        assert_eq!(lib.step(a, -1), f);
        // A missing file is an error row too, not a crash.
        assert!(matches!(index_one(&root.join("gone.sty")), Info::Err(_)));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn several_roots_prefix_the_category() {
        let a = temp_dir("rootA");
        let b = temp_dir("rootB");
        write(&a, "Sub/x.sty", &style("X", 120, (4, 4), &["Main A"]));
        write(&b, "y.sty", &style("Y", 120, (4, 4), &["Main A"]));
        let single = b.join("y.sty");
        let lib = Library::scan(&[a.clone(), b.clone(), single]);
        let folders: Vec<&str> = lib.order().iter().map(|&i| lib.entry(i).folder.as_str()).collect();
        let an = a.file_name().unwrap().to_string_lossy().to_string();
        let bn = b.file_name().unwrap().to_string_lossy().to_string();
        assert_eq!(folders, ["", &format!("{an}/Sub"), &bn]);
        let _ = std::fs::remove_dir_all(a);
        let _ = std::fs::remove_dir_all(b);
    }

    #[test]
    fn hidden_folders_and_non_regular_files_are_skipped() {
        let root = temp_dir("hidden");
        let st = style("S", 120, (4, 4), &["Main A"]);
        write(&root, "Pop/ok.sty", &st);
        write(&root, ".Trashes/501/deleted.sty", &st);
        write(&root, "Pop/.git/objects/x.sty", &st);
        // A folder named like a style is still a folder.
        std::fs::create_dir_all(root.join("Pop/folder.sty")).unwrap();
        let fifo = root.join("Pop/pipe.sty");
        let made = std::process::Command::new("mkfifo").arg(&fifo).status().is_ok_and(|s| s.success());
        let lib = Library::scan(std::slice::from_ref(&root));
        let found: Vec<String> = lib.order().iter().map(|&i| format!("{}|{}", lib.entry(i).folder, lib.entry(i).name())).collect();
        assert_eq!(found, ["Pop|ok"]);
        if made {
            // Named directly, a pipe is skipped too; the index refuses it without opening it.
            assert_eq!(Library::scan(std::slice::from_ref(&fifo)).len(), 0);
            assert_eq!(index_one(&fifo), Info::Err("not a regular file".into()));
        }
        let _ = std::fs::remove_dir_all(root);
    }

    fn lib_of(items: &[(&str, &str)]) -> Library {
        Library::from_entries(items.iter().map(|(folder, stem)| Entry::new(PathBuf::from(format!("/x/{folder}/{stem}.sty")), folder.to_string())).collect())
    }

    #[test]
    fn filter_is_case_insensitive_on_name_and_folder() {
        let mut lib = lib_of(&[("Jazz", "CoolSwing"), ("Pop", "8BeatModern"), ("Latin", "BossaNova"), ("Jazz/Latin", "Samba")]);
        let f = |lib: &Library, q: &str| names(lib, &lib.filter(q));
        assert_eq!(f(&lib, "").len(), 4);
        assert_eq!(f(&lib, "SWING"), ["Jazz|CoolSwing"]);
        assert_eq!(f(&lib, "latin"), ["Jazz/Latin|Samba", "Latin|BossaNova"]);
        assert_eq!(f(&lib, "jazz"), ["Jazz|CoolSwing", "Jazz/Latin|Samba"]);
        assert_eq!(f(&lib, "beatmod"), ["Pop|8BeatModern"]);
        assert!(f(&lib, "polka").is_empty());
        // Backspace: a shorter query widens again.
        assert_eq!(f(&lib, "bos"), ["Latin|BossaNova"]);
        assert_eq!(f(&lib, "bo").len(), 1);
        assert_eq!(f(&lib, "b").len(), 3); // 8Beat, Bossa, Samba
        // The name from the index is what's matched once it arrives.
        let id = lib.filter("8beat")[0];
        lib.set_info(id, Info::Ok(Summary { name: "Modern Pop Groove".into(), bpm: 100.0, timesig: (4, 4), sections: vec![] }));
        lib.sort();
        assert_eq!(f(&lib, "groove"), ["Pop|Modern Pop Groove"]);
        // The file name still matches, so the row doesn't drop out while indexing runs.
        assert_eq!(f(&lib, "8beat"), ["Pop|Modern Pop Groove"]);
    }

    #[test]
    fn corpus_indexes_without_errors() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
        if !root.exists() {
            return;
        }
        let mut lib = Library::scan(&[root]);
        index_all(&mut lib);
        for &i in lib.order() {
            let e = lib.entry(i);
            // The index agrees with the full parser on every file.
            match (&e.info, crate::sff::Style::load(&e.path)) {
                (Info::Ok(s), Ok(full)) => {
                    assert_eq!(s.name, full.name);
                    assert_eq!(s.timesig, full.timesig);
                    assert_eq!(s.sections, full.sections.keys().copied().collect::<Vec<_>>());
                }
                (Info::Err(_), Err(_)) => {}
                (i, f) => panic!("{}: index {i:?} vs full parse {:?}", e.path.display(), f.map(|s| s.name)),
            }
        }
    }
}
