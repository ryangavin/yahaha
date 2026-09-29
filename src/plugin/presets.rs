//! An Audio Unit's presets: its factory presets (`kAudioUnitProperty_FactoryPresets`, read
//! from a loaded instance) and the user presets on disk, the `.aupreset` files Logic,
//! MainStage and GarageBand write to
//! `~/Library/Audio/Presets/<Manufacturer>/<Plugin>/` (and `/Library/Audio/Presets/...`
//! for every user).
//!
//! An `.aupreset` is the unit's `kAudioUnitProperty_ClassInfo` dictionary as a property
//! list, so loading one is restoring a state (`PluginInstance::set_state`), and saving the
//! current state as one ([`write_user_preset`]) makes a file Logic reads too.
//!
//! Everything here runs off the real-time threads: the scan thread lists the files, a load
//! thread reads the factory presets and applies them.

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::scan::{PluginId, PluginInfo};

/// One of the unit's own presets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactoryPreset {
    /// Its `presetNumber` (what `kAudioUnitProperty_PresentPreset` takes).
    pub number: i32,
    pub name: String,
}

/// The unit's factory presets as the browser lists them, from what
/// `kAudioUnitProperty_FactoryPresets` gave (number, name; None for a preset with no name):
/// names trimmed, placeholders dropped (no name, a blank one, a bracketed or punctuation-only
/// one such as "<disabled>" or "---", or a slot word such as "empty"), and each name kept
/// once (the first). Many units pad their bank with such slots; they are not sounds.
pub fn real_factory_presets(raw: impl IntoIterator<Item = (i32, Option<String>)>) -> Vec<FactoryPreset> {
    let mut seen = std::collections::HashSet::new();
    raw.into_iter()
        .filter_map(|(number, name)| {
            let name = name?.trim().to_string();
            (!is_placeholder(&name) && seen.insert(name.clone())).then_some(FactoryPreset { number, name })
        })
        .collect()
}

/// A factory preset name that names no sound: blank, bracketed ("<disabled>", "[empty]",
/// "(none)"), without a letter or digit ("---"), or a slot word.
fn is_placeholder(name: &str) -> bool {
    const SLOTS: [&str; 7] = ["empty", "disabled", "none", "unused", "n/a", "null", "untitled"];
    let inner = name.trim_matches(|c: char| "<>[]()-_*. ".contains(c)).to_lowercase();
    !name.chars().any(char::is_alphanumeric) || (name.starts_with('<') && name.ends_with('>')) || SLOTS.contains(&inner.as_str())
}

/// An `.aupreset` file for the unit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserPreset {
    /// The file name without `.aupreset`: what Logic shows.
    pub name: String,
    pub path: PathBuf,
    /// The sub-folder it is in under the plugin's folder ("Pianos"), if any: a folder is a
    /// preset category in Logic's menu, and a hint for the browser's category.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
}

/// The unit an `.aupreset` is for, and the name inside it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AuPresetHeader {
    pub name: Option<String>,
    pub id: Option<PluginId>,
}

/// The user preset folders: `~/Library/Audio/Presets` then `/Library/Audio/Presets`.
pub fn default_preset_roots() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(h) = std::env::var_os("HOME") {
        v.push(PathBuf::from(h).join("Library/Audio/Presets"));
    }
    v.push(PathBuf::from("/Library/Audio/Presets"));
    v
}

/// The folder a unit's user presets live in under `root`: `<Manufacturer>/<Plugin>`.
pub fn plugin_preset_dir(root: &Path, info: &PluginInfo) -> PathBuf {
    root.join(safe_name(&info.manufacturer)).join(safe_name(&info.name))
}

/// A name as a file or folder name: no path separators, no leading dot.
pub fn safe_name(s: &str) -> String {
    let t: String = s.chars().map(|c| if matches!(c, '/' | ':' | '\\') || c.is_control() { '-' } else { c }).collect();
    let t = t.trim().trim_start_matches('.').trim().to_string();
    if t.is_empty() { "Untitled".into() } else { t }
}

/// Read an `.aupreset`'s header: which unit it is for, and its name. XML is read here; a
/// binary property list is converted to XML first. None if it is not a property list with
/// a dictionary at the top.
pub fn parse_aupreset(bytes: &[u8]) -> Option<AuPresetHeader> {
    let text = if bytes.starts_with(b"bplist") {
        String::from_utf8(super::sys::plist_to_xml(bytes)?).ok()?
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    };
    parse_xml_header(&text)
}

/// The top-level dictionary's `name`, `type`, `subtype` and `manufacturer` of an XML
/// property list (a nested dictionary's keys are skipped).
pub(crate) fn parse_xml_header(xml: &str) -> Option<AuPresetHeader> {
    let start = xml.find("<dict>")? + "<dict>".len();
    let mut rest = &xml[start..];
    let mut depth = 0usize;
    let mut key: Option<String> = None;
    let (mut name, mut kind, mut subtype, mut manufacturer) = (None, None, None, None);
    loop {
        let lt = rest.find('<')?;
        rest = &rest[lt..];
        let gt = rest.find('>')?;
        let tag = &rest[1..gt];
        let after = &rest[gt + 1..];
        if let Some(t) = tag.strip_prefix('/') {
            if matches!(t.trim(), "dict" | "array") {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            rest = after;
            continue;
        }
        if tag.starts_with('!') || tag.starts_with('?') {
            rest = after;
            continue;
        }
        let self_closing = tag.ends_with('/');
        let tname = tag.trim_end_matches('/').split_whitespace().next().unwrap_or("");
        if matches!(tname, "dict" | "array") {
            if !self_closing {
                depth += 1;
            }
            key = None;
            rest = after;
            continue;
        }
        if self_closing {
            key = None;
            rest = after;
            continue;
        }
        let close = format!("</{tname}>");
        let end = after.find(&close)?;
        let body = unescape(&after[..end]);
        rest = &after[end + close.len()..];
        if depth > 0 {
            continue;
        }
        if tname == "key" {
            key = Some(body);
            continue;
        }
        match (key.take().as_deref(), tname) {
            (Some("name"), "string") => name = Some(body),
            (Some("type"), "integer") => kind = int_u32(&body),
            (Some("subtype"), "integer") => subtype = int_u32(&body),
            (Some("manufacturer"), "integer") => manufacturer = int_u32(&body),
            _ => {}
        }
    }
    let id = match (kind, subtype, manufacturer) {
        (Some(kind), Some(subtype), Some(manufacturer)) => Some(PluginId { kind, subtype, manufacturer }),
        _ => None,
    };
    Some(AuPresetHeader { name, id })
}

/// A plist integer as a four-character code: written signed or unsigned.
fn int_u32(s: &str) -> Option<u32> {
    let v: i64 = s.trim().parse().ok()?;
    (i32::MIN as i64..=u32::MAX as i64).contains(&v).then_some(v as u32)
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

/// Where the user preset `name` of `info` is (or would be) under `root`.
pub fn user_preset_path(root: &Path, info: &PluginInfo, name: &str) -> PathBuf {
    plugin_preset_dir(root, info).join(format!("{}.aupreset", safe_name(name)))
}

/// A save refused because a preset of that name exists (and `overwrite` was not set).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetExists(pub String);

impl std::fmt::Display for PresetExists {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "a preset called {} already exists: save under another name, or replace it", self.0)
    }
}

impl std::error::Error for PresetExists {}

/// The largest `.aupreset` read while listing (a sampler's can embed its samples' paths,
/// rarely more than a few MB).
const MAX_PRESET_BYTES: u64 = 64 << 20;

/// Every `.aupreset` for `info` under the roots' `<Manufacturer>/<Plugin>` folders (and
/// their sub-folders, a level or two deep), sorted by folder then name. A file that names
/// another unit is left out; one that can't be read is listed by its file name, since
/// Logic lists it that way too.
pub fn user_presets(info: &PluginInfo, roots: &[PathBuf]) -> Vec<UserPreset> {
    let mut out = Vec::new();
    for root in roots {
        let dir = plugin_preset_dir(root, info);
        walk(&dir, None, 0, info.id, &mut out);
    }
    out.sort_by_key(|p| (p.folder.clone().unwrap_or_default().to_lowercase(), p.name.to_lowercase()));
    out.dedup_by(|a, b| a.path == b.path);
    out
}

fn walk(dir: &Path, folder: Option<&str>, depth: usize, id: PluginId, out: &mut Vec<UserPreset>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let path = e.path();
        let fname = e.file_name().to_string_lossy().into_owned();
        if fname.starts_with('.') {
            continue;
        }
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_dir() {
            if depth < 3 {
                let sub = match folder {
                    Some(f) => format!("{f}/{fname}"),
                    None => fname.clone(),
                };
                walk(&path, Some(&sub), depth + 1, id, out);
            }
            continue;
        }
        let Some(stem) = fname.strip_suffix(".aupreset") else { continue };
        let fits = e.metadata().map(|m| m.len() <= MAX_PRESET_BYTES).unwrap_or(false);
        let header = if fits { std::fs::read(&path).ok().and_then(|b| parse_aupreset(&b)) } else { None };
        if header.as_ref().and_then(|h| h.id).is_some_and(|i| i != id) {
            continue;
        }
        out.push(UserPreset { name: stem.to_string(), path, folder: folder.map(str::to_string) });
    }
}

/// Write `state` (the unit's ClassInfo, as `PluginInstance::get_state` reads it) as
/// `<root>/<Manufacturer>/<Plugin>/<name>.aupreset`, in XML as Logic writes it. The
/// dictionary is written unchanged. A preset of the same name is replaced only with
/// `overwrite` (the files are shared with Logic and MainStage). Returns it.
pub fn write_user_preset(root: &Path, info: &PluginInfo, name: &str, state: &[u8], overwrite: bool) -> Result<UserPreset> {
    if !overwrite && user_preset_path(root, info, name).exists() {
        return Err(anyhow!(PresetExists(safe_name(name))));
    }
    let dir = plugin_preset_dir(root, info);
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let xml = super::sys::plist_to_xml(state).ok_or_else(|| anyhow!("the plugin's settings are not a property list"))?;
    let name = safe_name(name);
    let path = dir.join(format!("{name}.aupreset"));
    let tmp = dir.join(format!(".{name}.aupreset.{}.tmp", std::process::id()));
    std::fs::write(&tmp, xml).with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, &path).with_context(|| format!("replacing {}", path.display()))?;
    Ok(UserPreset { name, path, folder: None })
}

/// Whether two plugin states are the same settings: the same bytes, or the same property
/// list (an `.aupreset` file holds the XML form of the state it was written from).
pub fn same_settings(a: &[u8], b: &[u8]) -> bool {
    a == b || matches!((super::sys::plist_to_xml(a), super::sys::plist_to_xml(b)), (Some(x), Some(y)) if x == y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::PluginFormat;

    pub(crate) const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>data</key>
	<data>
	AAAAAA==
	</data>
	<key>vstdata</key>
	<dict>
		<key>name</key>
		<string>not this one</string>
		<key>type</key>
		<integer>1</integer>
	</dict>
	<key>manufacturer</key>
	<integer>1180789605</integer>
	<key>name</key>
	<string>Grand &amp; Strings</string>
	<key>subtype</key>
	<integer>1315531570</integer>
	<key>type</key>
	<integer>1635085685</integer>
	<key>version</key>
	<integer>0</integer>
	<key>empty</key>
	<true/>
</dict>
</plist>
"#;

    fn info() -> PluginInfo {
        PluginInfo {
            id: PluginId::parse("aumu Nik2 -NI-").unwrap(),
            name: "Kontakt 7".into(),
            manufacturer: "Native Instruments".into(),
            version: 0x70000,
            format: PluginFormat::Au2,
            requires_async: false,
            can_load_in_process: false,
            sandbox_safe: false,
            last_load: None,
            in_process: false,
            factory_presets: None,
            user_presets: Vec::new(),
        }
    }

    #[test]
    fn placeholder_factory_presets_are_never_listed() {
        let raw = [
            (0, Some("Init".to_string())),
            (1, Some("  Bright Grand ".into())),
            (2, None),
            (3, Some(String::new())),
            (4, Some("   ".into())),
            (5, Some("<disabled>".into())),
            (6, Some("<Empty Slot>".into())),
            (7, Some("---".into())),
            (8, Some("[Empty]".into())),
            (9, Some("empty".into())),
            (10, Some("Bright Grand".into())),
            (11, Some("Init".into())),
            (12, Some("Brass Stabs (2)".into())),
        ];
        let got: Vec<_> = real_factory_presets(raw).into_iter().map(|p| (p.number, p.name)).collect();
        assert_eq!(got, [(0, "Init".to_string()), (1, "Bright Grand".into()), (12, "Brass Stabs (2)".into())]);
    }

    #[test]
    fn an_aupreset_header_reads_the_top_level_keys_only() {
        let h = parse_aupreset(SAMPLE.as_bytes()).unwrap();
        assert_eq!(h.name.as_deref(), Some("Grand & Strings"));
        let id = h.id.unwrap();
        assert_eq!(id.to_string(), "aumu Nik2 Fake");
        assert!(parse_aupreset(b"not a plist").is_none());
        // No ids: still a preset (listed by its file name), for no unit in particular.
        let h = parse_xml_header("<plist><dict><key>name</key><string>x</string></dict></plist>").unwrap();
        assert_eq!((h.name.as_deref(), h.id), (Some("x"), None));
        // Negative (signed) codes read as their unsigned bits.
        let h = parse_xml_header("<dict><key>type</key><integer>-1</integer><key>subtype</key><integer>1</integer><key>manufacturer</key><integer>2</integer></dict>").unwrap();
        assert_eq!(h.id.unwrap().kind, u32::MAX);
    }

    #[test]
    fn a_binary_aupreset_reads_like_the_xml_one() {
        let data = objc2_core_foundation::CFData::from_bytes(SAMPLE.as_bytes());
        let plist = unsafe { objc2_core_foundation::CFPropertyListCreateWithData(None, Some(&data), 0, std::ptr::null_mut(), std::ptr::null_mut()) }.unwrap();
        let bin = unsafe { objc2_core_foundation::CFPropertyListCreateData(None, Some(&plist), objc2_core_foundation::CFPropertyListFormat::BinaryFormat_v1_0, 0, std::ptr::null_mut()) }
            .unwrap()
            .to_vec();
        assert!(bin.starts_with(b"bplist"));
        assert_eq!(parse_aupreset(&bin), parse_aupreset(SAMPLE.as_bytes()));
    }

    #[test]
    fn user_presets_are_listed_by_folder_and_written_back() {
        let root = std::env::temp_dir().join(format!("yahaha-presets-{}-{:?}", std::process::id(), std::thread::current().id()));
        let _ = std::fs::remove_dir_all(&root);
        let info = info();
        let dir = plugin_preset_dir(&root, &info);
        std::fs::create_dir_all(dir.join("Pianos")).unwrap();
        let mine = SAMPLE.replace("1180789605", &(u32::from_be_bytes(*b"-NI-") as i64).to_string());
        std::fs::write(dir.join("Pianos/Upright.aupreset"), &mine).unwrap();
        std::fs::write(dir.join("Arco Strings.aupreset"), &mine).unwrap();
        // Another unit's preset in the folder, a stray file, a hidden one.
        std::fs::write(dir.join("Other.aupreset"), SAMPLE).unwrap();
        std::fs::write(dir.join("notes.txt"), "x").unwrap();
        std::fs::write(dir.join(".hidden.aupreset"), &mine).unwrap();
        let list = user_presets(&info, std::slice::from_ref(&root));
        let names: Vec<(&str, Option<&str>)> = list.iter().map(|p| (p.name.as_str(), p.folder.as_deref())).collect();
        assert_eq!(names, [("Arco Strings", None), ("Upright", Some("Pianos"))]);

        // Saving writes a standard XML .aupreset there, and it lists.
        let saved = write_user_preset(&root, &info, "My: Piano/1", mine.as_bytes(), false).unwrap();
        // The same name again: refused unless replacing, and the file is untouched.
        let before = std::fs::read(&saved.path).unwrap();
        let e = write_user_preset(&root, &info, "My: Piano/1", b"<plist><dict/></plist>", false).unwrap_err();
        assert!(e.downcast_ref::<PresetExists>().is_some(), "{e}");
        assert_eq!(std::fs::read(&saved.path).unwrap(), before, "the existing file is untouched");
        write_user_preset(&root, &info, "My: Piano/1", mine.as_bytes(), true).unwrap();
        assert_eq!(saved.name, "My- Piano-1");
        let back = std::fs::read(&saved.path).unwrap();
        assert!(back.starts_with(b"<?xml"));
        assert_eq!(parse_aupreset(&back).unwrap().id, Some(info.id));
        assert!(user_presets(&info, std::slice::from_ref(&root)).iter().any(|p| p.name == "My- Piano-1"));
        assert!(write_user_preset(&root, &info, "bad", b"not a plist", true).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }
}
