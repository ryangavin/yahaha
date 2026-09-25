//! The installed instruments, and the on-disk scan cache.
//!
//! Enumerating components is cheap (the registrar answers in a few ms), but copying every
//! name is not free and the app wants the list at start-up before anything else runs. So a
//! scan is cached as JSON, keyed by a fingerprint of every instrument's (type, subtype,
//! manufacturer, flags, version): installing, removing or updating a plugin changes the
//! fingerprint and forces a fresh scan. The cache also remembers each plugin's last load
//! (time, or the error / timeout), so the plugin browser can flag a plugin that failed before
//! the user picks it.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use super::sys::{self, Component};

/// An Audio Unit's identity: component type, subtype and manufacturer, the triple
/// `auval -a` prints ("aumu Xf2X XFER"). Stable across versions, so it is what a
/// Registration stores.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PluginId {
    #[serde(rename = "type")]
    pub kind: u32,
    pub subtype: u32,
    pub manufacturer: u32,
}

impl PluginId {
    /// Parse "aumu Xf2X XFER" (three four-character codes, space separated; a code with a
    /// trailing space such as "dls " is written as is, so "aumu dls  appl" works).
    pub fn parse(code: &str) -> Option<PluginId> {
        let b = code.as_bytes();
        if b.len() == 14 && code.is_ascii() && b[4] == b' ' && b[9] == b' ' {
            return Some(PluginId {
                kind: sys::fourcc_parse(&code[0..4])?,
                subtype: sys::fourcc_parse(&code[5..9])?,
                manufacturer: sys::fourcc_parse(&code[10..14])?,
            });
        }
        let mut it = code.split_whitespace();
        let id = PluginId { kind: sys::fourcc_parse(it.next()?)?, subtype: sys::fourcc_parse(it.next()?)?, manufacturer: sys::fourcc_parse(it.next()?)? };
        it.next().is_none().then_some(id)
    }

    /// Apple's DLSMusicDevice ("aumu dls  appl"): always installed, used by the tests.
    pub const DLS: PluginId = PluginId { kind: 0x61756d75, subtype: 0x646c7320, manufacturer: 0x6170706c };
}

impl fmt::Display for PluginId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", sys::fourcc_str(self.kind), sys::fourcc_str(self.subtype), sys::fourcc_str(self.manufacturer))
    }
}

/// Which plugin API the component implements.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginFormat {
    /// A classic AUv2 component bundle. Loads in process by default.
    #[serde(rename = "AUv2")]
    Au2,
    /// An AUv3 app extension. Loads out of process by default (its own sandboxed process;
    /// a crash there cannot take yahaha down).
    #[serde(rename = "AUv3")]
    Au3,
}

/// What happened the last time this plugin (at this version) was loaded.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoadRecord {
    /// Unix seconds.
    pub at: u64,
    /// Total load time, when it loaded.
    pub ms: Option<f64>,
    /// The error, or "timed out after N s".
    pub error: Option<String>,
}

/// One installed instrument.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: PluginId,
    /// The product name ("Serum 2").
    pub name: String,
    /// The vendor ("Xfer Records"). AudioToolbox reports "Vendor: Name"; split here.
    pub manufacturer: String,
    /// 0xMMMMmmDD (major, minor, dot).
    pub version: u32,
    pub format: PluginFormat,
    /// Must be created with `AudioComponentInstantiate` (never `AudioComponentInstanceNew`).
    pub requires_async: bool,
    /// An AUv3 that also allows in-process loading.
    pub can_load_in_process: bool,
    pub sandbox_safe: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_load: Option<LoadRecord>,
    /// The player's override: load it in yahaha's process, not its own (for the lightest
    /// plugins: no IPC per render, but a crash takes yahaha down). Kept across rescans and
    /// updates of the plugin (`PluginHost::set_in_process`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub in_process: bool,
}

impl PluginInfo {
    /// Whether it can run in yahaha's process at all: every AUv2, and an AUv3 that allows it.
    pub fn can_run_in_process(&self) -> bool {
        self.format == PluginFormat::Au2 || self.can_load_in_process
    }

    /// "2.1.4" from 0x00020104.
    pub fn version_string(&self) -> String {
        format!("{}.{}.{}", self.version >> 16, (self.version >> 8) & 0xFF, self.version & 0xFF)
    }

    /// "Xfer Records: Serum 2", as AudioToolbox names it.
    pub fn full_name(&self) -> String {
        if self.manufacturer.is_empty() { self.name.clone() } else { format!("{}: {}", self.manufacturer, self.name) }
    }

    fn from_component(c: &Component) -> PluginInfo {
        let full = c.name();
        let (manufacturer, name) = match full.split_once(": ") {
            Some((m, n)) => (m.trim().to_string(), n.trim().to_string()),
            None => (String::new(), full.trim().to_string()),
        };
        PluginInfo {
            id: PluginId { kind: c.desc[0], subtype: c.desc[1], manufacturer: c.desc[2] },
            name,
            manufacturer,
            version: c.version,
            format: if c.flags & sys::FLAG_IS_V3 != 0 { PluginFormat::Au3 } else { PluginFormat::Au2 },
            requires_async: c.flags & sys::FLAG_REQUIRES_ASYNC != 0,
            can_load_in_process: c.flags & sys::FLAG_CAN_LOAD_IN_PROCESS != 0,
            sandbox_safe: c.flags & sys::FLAG_SANDBOX_SAFE != 0,
            last_load: None,
            in_process: false,
        }
    }
}

/// The cache file's contents.
#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct ScanCache {
    pub schema: u32,
    pub fingerprint: u64,
    pub plugins: Vec<PluginInfo>,
}

pub(crate) const SCHEMA: u32 = 1;

/// A hash of what is installed, without copying any names.
pub(crate) fn fingerprint(components: &[Component]) -> u64 {
    let mut keys: Vec<([u32; 3], u32, u32)> = components.iter().map(|c| (c.desc, c.flags, c.version)).collect();
    keys.sort_unstable();
    let mut h = DefaultHasher::new();
    SCHEMA.hash(&mut h);
    keys.hash(&mut h);
    h.finish()
}

/// Scan the registrar now: every instrument, sorted by vendor then name.
pub(crate) fn scan_live(components: &[Component]) -> Vec<PluginInfo> {
    let mut out: Vec<PluginInfo> = components.iter().map(PluginInfo::from_component).collect();
    out.sort_by_key(|p| (p.manufacturer.to_lowercase(), p.name.to_lowercase()));
    out
}

pub(crate) fn read_cache(path: &Path) -> Option<ScanCache> {
    let bytes = std::fs::read(path).ok()?;
    let c: ScanCache = serde_json::from_slice(&bytes).ok()?;
    (c.schema == SCHEMA).then_some(c)
}

pub(crate) fn write_cache(path: &Path, c: &ScanCache) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    // Write-then-rename, so a crash mid-write never leaves a torn cache.
    // A temporary name of its own per write: two sessions (or a scan and a load recording
    // its time) writing at once must not rename each other's file away.
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = path.with_extension(format!("json.{}.{n}.tmp", std::process::id()));
    std::fs::write(&tmp, serde_json::to_vec_pretty(c)?).with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))?;
    Ok(())
}

/// `~/Library/Caches/yahaha/plugins.json`, the default cache location.
pub fn default_cache_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Caches/yahaha/plugins.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_parse_and_print() {
        let dls = PluginId::parse("aumu dls  appl").unwrap();
        assert_eq!(dls, PluginId::DLS);
        assert_eq!(dls.to_string(), "aumu dls  appl");
        assert_eq!(PluginId::parse("aumu dls appl"), Some(PluginId::DLS), "trailing spaces may be dropped");
        let serum = PluginId::parse("aumu Xf2X XFER").unwrap();
        assert_eq!(serum.to_string(), "aumu Xf2X XFER");
        assert_eq!(PluginId::parse("aumu"), None);
        assert_eq!(PluginId::parse("aumu a b c"), None);
        let json = serde_json::to_string(&dls).unwrap();
        assert_eq!(serde_json::from_str::<PluginId>(&json).unwrap(), dls);
    }

    #[test]
    fn fingerprint_ignores_order_and_sees_versions() {
        let all = sys::instruments();
        let mut rev = all.clone();
        rev.reverse();
        assert_eq!(fingerprint(&all), fingerprint(&rev));
        if let Some(first) = rev.first_mut() {
            first.version += 1;
            assert_ne!(fingerprint(&all), fingerprint(&rev));
        }
    }
}
