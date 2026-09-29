//! Test helpers: an offline session that needs no corpus.

use super::{Options, Session};
use std::path::{Path, PathBuf};

/// A fresh data folder for test `test` (emptied first).
pub(crate) fn data_dir(test: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("yahaha-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn chunk(id: &[u8], body: &[u8]) -> Vec<u8> {
    let mut v = id.to_vec();
    v.extend_from_slice(&(body.len() as u32).to_be_bytes());
    v.extend_from_slice(body);
    v
}

/// An offline session on a tiny synthetic style written at run time (one Main A bar), with
/// its data folder at `data`, so a test runs without the git-ignored corpus: capturing and
/// applying keyboard parts needs no real style.
pub(crate) fn session(data: &Path) -> Session {
    let mut trk = vec![0x00, 0xFF, 0x06, 4];
    trk.extend_from_slice(b"SFF2");
    trk.extend_from_slice(&[0x00, 0xFF, 0x06, 6]);
    trk.extend_from_slice(b"Main A");
    trk.extend_from_slice(&[0x00, 0x9B, 60, 100, 0x83, 0x00, 0x8B, 60, 0, 0x00, 0xFF, 0x2F, 0]);
    let mut bytes = chunk(b"MThd", &[0, 0, 0, 1, 0, 96]);
    bytes.extend(chunk(b"MTrk", &trk));
    bytes.extend(chunk(b"CASM", &chunk(b"CSEG", &chunk(b"Sdec", b"Main A"))));
    let style = data.join("styles/Test.sty");
    std::fs::create_dir_all(style.parent().unwrap()).unwrap();
    std::fs::write(&style, bytes).unwrap();
    Session::offline(Options { paths: vec![style], data_dir: Some(data.to_path_buf()), ..Options::default() }).unwrap()
}
