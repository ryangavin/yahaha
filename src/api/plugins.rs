//! Plugin instruments (Audio Units) for the keyboard parts (#91, docs/plugin-hosting.md).
//!
//! A part plays its SoundFont voice (`KeyboardPart::program`) or an Audio Unit
//! instrument (`KeyboardPart::plugin`). The plugin's editor window is not a command: it
//! must open on the app's main thread, so the app shell opens it (the Tauri command
//! `openPluginEditor`, docs/app-api.md) with the handle `Session::plugin_editor` gives.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PluginCmd {
    /// Play a keyboard part on an instrument plugin: `id` from `plugins.list` ("aumu dls
    /// appl"), `state` a saved preset (base64, from `KeyboardPart::plugin`'s voice) or
    /// none for the plugin's default. It loads in the background; the part keeps its
    /// SoundFont voice until the plugin is ready, then switches without a click.
    SetPartPlugin {
        part: u8,
        id: String,
        #[serde(default)]
        state: Option<String>,
    },
    /// Back to the part's SoundFont voice.
    ClearPartPlugin { part: u8 },
    /// Save the plugin's current preset (what its editor changed) into the part, so it is
    /// kept across restarts. The app sends it when the editor closes.
    SavePartPluginState { part: u8 },
    /// Scan the installed instruments again (ignoring the cache).
    RescanPlugins,
    /// Run plugin `id` in yahaha's process (`true`) or in its own (`false`, the default for
    /// third-party plugins). In process saves the IPC per render for the lightest plugins,
    /// but a crash in it takes yahaha down. Kept in the scan cache; applies from the
    /// plugin's next load.
    SetPluginInProcess { id: String, in_process: bool },
}

/// Where a part's plugin is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PluginStatus {
    /// Loading in the background; the part still plays its SoundFont voice.
    #[default]
    Loading,
    /// The part plays the plugin.
    Playing,
    /// The load failed or timed out: the part plays its SoundFont voice (`error` says why).
    Failed,
    /// The plugin stopped rendering (it crashed or produced bad audio): the part is silent
    /// until it is chosen again or cleared.
    Muted,
}

/// A keyboard part's plugin.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartPlugin {
    /// "aumu dls  appl".
    pub id: String,
    /// "DLSMusicDevice".
    pub name: String,
    /// "Apple".
    pub manufacturer: String,
    pub status: PluginStatus,
    /// While loading: "queued", "instantiating", "initializing", "restoringState".
    pub stage: Option<String>,
    pub error: Option<String>,
    /// Runs in its own process (a crash there only silences the part).
    pub out_of_process: bool,
    /// Its render time as a share of real time (0.05 = 5% of one core), updated once a
    /// second.
    pub cpu: f32,
    /// Renders slower than half the audio buffer, since it loaded.
    pub overruns: u64,
    /// Its editor window can be opened (the app shell has the plugin host).
    pub editor: bool,
}

/// One installed instrument plugin.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginEntry {
    /// "aumu Xf2X XFER": what `SetPartPlugin` takes.
    pub id: String,
    pub name: String,
    pub manufacturer: String,
    /// "2.1.4".
    pub version: String,
    /// "AUv2" or "AUv3".
    pub format: String,
    /// The last load's error ("timed out after 20.0 s"), so the browser can warn.
    pub last_error: Option<String>,
    /// The player chose to run it in yahaha's process (`SetPluginInProcess`).
    #[serde(default)]
    pub in_process: bool,
    /// It can run in yahaha's process: every AUv2, and an AUv3 that allows it.
    #[serde(default)]
    pub can_run_in_process: bool,
}

/// The plugin host.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginsState {
    /// This build hosts plugins (macOS, the `plugins` feature) and the synth is running.
    pub available: bool,
    /// A rescan is running.
    pub scanning: bool,
    /// The installed instruments, by manufacturer then name (cached scan).
    pub list: Vec<PluginEntry>,
}

/// Base64 (standard alphabet, padded): plugin states in JSON.
pub fn base64_encode(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            out.push(if i <= c.len() { T[(n >> shift & 63) as usize] as char } else { '=' });
        }
    }
    out
}

/// Decode [`base64_encode`]'s form (whitespace ignored). None if it is not base64.
pub fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let val = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32)
    };
    let b: Vec<u8> = s.bytes().filter(|c| !c.is_ascii_whitespace()).collect();
    if b.len() % 4 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(b.len() / 4 * 3);
    for q in b.chunks(4) {
        let pad = q.iter().rev().take_while(|&&c| c == b'=').count();
        if pad > 2 || q[..4 - pad].contains(&b'=') {
            return None;
        }
        let mut n = 0u32;
        for &c in &q[..4 - pad] {
            n = n << 6 | val(c)?;
        }
        n <<= 6 * pad as u32;
        out.extend_from_slice(&[(n >> 16) as u8, (n >> 8) as u8, n as u8][..3 - pad]);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trips() {
        for (raw, enc) in [(&b""[..], ""), (b"f", "Zg=="), (b"fo", "Zm8="), (b"foo", "Zm9v"), (b"foobar", "Zm9vYmFy")] {
            assert_eq!(base64_encode(raw), enc);
            assert_eq!(base64_decode(enc).unwrap(), raw);
        }
        let all: Vec<u8> = (0..=255).collect();
        assert_eq!(base64_decode(&base64_encode(&all)).unwrap(), all);
        assert!(base64_decode("Zm9").is_none());
        assert!(base64_decode("Z=9v").is_none());
        assert!(base64_decode("Zm9$").is_none());
    }
}
