//! Instrument plugins (#91): a MIDI channel played by an Audio Unit instead of the
//! SoundFont. docs/plugin-hosting.md, "Phase 2 as built".
//!
//! The session-level API, for the keyboard parts here and for the sound library's program
//! map (#103) on the Style channels:
//!
//! - [`Control::assign_channel_plugin`]`(ch, PluginVoice)`: start loading the plugin for
//!   channel `ch` (0-15) on a thread of its own and return at once. The channel keeps
//!   playing what it plays (its SoundFont, or a previous plugin) until the load is done;
//!   the pump then hands the instance to the audio thread's rack and routes the channel to
//!   it (`route::Source::Plugin`), with a 5 ms crossfade from a previous plugin, or
//!   All Notes Off on the SoundFont side. A load that fails or times out (20 s) leaves the
//!   channel on the SoundFont and shows the error ([`Control::channel_plugin`]).
//! - [`Control::clear_channel_plugin`]`(ch)`: back to the SoundFont (a 5 ms fade out).
//! - [`Control::route_channel_sound_font`]`(ch, font)`: route a channel to SoundFont `font`
//!   (clearing any plugin there).
//! - [`Control::channel_plugin`]`(ch)`: where the channel's plugin is (`PartPlugin`:
//!   loading / playing / failed / muted, the error, CPU).
//!
//! Without the `plugins` feature (or without the built-in synth) `assign_channel_plugin`
//! fails with a message and nothing else changes.
//!
//! **Load mode (Decision).** Third-party AUv2 instruments load out of process (Apple's
//! AUHostingService): a crash there silences the part instead of taking the arranger down,
//! for 5-12 µs of IPC per 64-frame block (docs/plugin-hosting.md, "Measured"). Apple's
//! own units load in process; AUv3 extensions run out of process as macOS decides. A
//! third-party plugin that fails out of process is retried in process once.
//!
//! **Persistence.** A live session keeps the keyboard parts' plugins (id and state) in
//! `~/Library/Application Support/yahaha/plugin-parts.json` and loads them again at start.

use super::Control;
use crate::api::{base64_decode, base64_encode, CmdError, PartPlugin, PluginCmd, PluginsState};
use crate::parts;
use serde::{Deserialize, Serialize};
#[cfg(feature = "plugins")]
use std::path::PathBuf;

/// What a channel plays when it plays a plugin: which one and its preset.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PluginVoice {
    /// "aumu dls  appl" (the phase-1 `PluginId` string form).
    pub id: String,
    /// The plugin's full state (ClassInfo bytes); None = its default preset.
    #[serde(default, with = "b64opt")]
    pub state: Option<Vec<u8>>,
}

mod b64opt {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(v: &Option<Vec<u8>>, s: S) -> Result<S::Ok, S::Error> {
        match v {
            Some(b) => s.serialize_some(&crate::api::base64_encode(b)),
            None => s.serialize_none(),
        }
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<u8>>, D::Error> {
        Ok(Option::<String>::deserialize(d)?.and_then(|s| crate::api::base64_decode(&s)))
    }
}

/// The saved keyboard parts' plugins.
#[cfg(feature = "plugins")]
#[derive(Debug, Default, Serialize, Deserialize)]
struct Saved {
    /// By part index (0 = Right 1, 1 = Right 2, 2 = Right 3, 3 = Left).
    parts: [Option<PluginVoice>; parts::COUNT],
}

#[cfg(feature = "plugins")]
fn saved_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support/yahaha/plugin-parts.json"))
}

#[cfg(feature = "plugins")]
mod imp {
    use super::*;
    use crate::api::{PluginEntry, PluginStatus};
    use crate::plugin::{
        dispose_later, EditorTarget, LoadConfig, LoadHandle, LoadMode, LoadProgress, PluginFormat, PluginHost, PluginId, PluginInfo,
        PluginStats, RackEvent, Swap, DEFAULT_FADE_FRAMES,
    };
    use crate::route::Source;
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::time::Duration;

    /// Apple's manufacturer code: its units load in process.
    const APPLE: u32 = u32::from_be_bytes(*b"appl");

    pub(crate) struct ChannelPlugin {
        pub(crate) voice: PluginVoice,
        pub(crate) info: Option<PluginInfo>,
        pub(crate) load: Option<LoadHandle>,
        pub(crate) mode: LoadMode,
        pub(crate) status: PluginStatus,
        pub(crate) stage: Option<String>,
        pub(crate) error: Option<String>,
        pub(crate) editor: Option<EditorTarget>,
        pub(crate) stats: Option<Arc<PluginStats>>,
        pub(crate) out_of_process: bool,
        pub(crate) cpu: f32,
        /// (total ns, frames) at the last CPU reading.
        pub(crate) last: (u64, u64),
    }

    #[derive(Default)]
    pub(crate) struct PluginCtl {
        pub(crate) host: Option<PluginHost>,
        pub(crate) list: Vec<PluginInfo>,
        pub(crate) scan_rx: Option<mpsc::Receiver<Result<Vec<PluginInfo>, String>>>,
        pub(crate) channels: [Option<ChannelPlugin>; 16],
        /// A channel whose plugin was playing and is loading another: the one playing.
        pub(crate) playing: [Option<(PluginVoice, Option<PluginInfo>, Option<EditorTarget>, Option<Arc<PluginStats>>, bool)>; 16],
        pub(crate) stats_ns: u64,
        /// Save the parts' plugins at the next pump (live sessions only).
        pub(crate) dirty: bool,
    }

    fn stage_name(p: &LoadProgress) -> Option<String> {
        Some(match p {
            LoadProgress::Queued => "queued",
            LoadProgress::Instantiating => "instantiating",
            LoadProgress::Initializing => "initializing",
            LoadProgress::RestoringState => "restoringState",
            _ => return None,
        }.into())
    }

    impl PluginCtl {
        pub(crate) fn host(&mut self) -> PluginHost {
            self.host.get_or_insert_with(PluginHost::with_default_cache).clone()
        }
    }

    impl Control {
        fn plugin_ready(&self) -> Result<(), String> {
            match &self.synth {
                Some(s) if s.plugins.is_some() => Ok(()),
                Some(_) => Err("the synth has no plugin rack".into()),
                None => Err("plugins play through the built-in synth, which is off".into()),
            }
        }

        /// Start loading `voice` for channel `ch` (0-15); see the module docs. Returns at
        /// once: the pump finishes the job. Err if it can't even start (no synth, an
        /// unknown plugin id).
        pub(crate) fn assign_channel_plugin(&mut self, ch: u8, voice: PluginVoice) -> Result<(), String> {
            let ch = ch & 15;
            self.plugin_ready()?;
            let id = PluginId::parse(&voice.id).ok_or_else(|| format!("{:?} is not a plugin id", voice.id))?;
            let host = self.plugins.host();
            let info = host.info(&id).map_err(|e| format!("{e:#}"))?;
            let mode = if id.manufacturer == APPLE || info.format == PluginFormat::Au3 { LoadMode::Auto } else { LoadMode::OutOfProcess };
            let rate = self.synth.as_ref().map_or(48_000, |s| s.info.sample_rate) as f64;
            let cfg = LoadConfig { sample_rate: rate, max_frames: crate::synth::PLUGIN_MAX_BLOCK as u32, state: voice.state.clone(), mode, timeout: Duration::from_secs(20) };
            let load = host.load_async(&id, cfg).map_err(|e| format!("{e:#}"))?;
            // What plays now keeps playing until the new one is ready.
            if let Some(prev) = self.plugins.channels[ch as usize].take()
                && prev.status == PluginStatus::Playing
            {
                self.plugins.playing[ch as usize] = Some((prev.voice, prev.info, prev.editor, prev.stats, prev.out_of_process));
            }
            self.plugins.channels[ch as usize] = Some(ChannelPlugin {
                voice,
                info: Some(info),
                load: Some(load),
                mode,
                status: PluginStatus::Loading,
                stage: Some("queued".into()),
                error: None,
                editor: None,
                stats: None,
                out_of_process: false,
                cpu: 0.0,
                last: (0, 0),
            });
            Ok(())
        }

        /// Channel `ch` back to its SoundFont (a short fade out), cancelling a load.
        pub(crate) fn clear_channel_plugin(&mut self, ch: u8) {
            let ch = ch & 15;
            let had = self.plugins.channels[ch as usize].take().is_some() | self.plugins.playing[ch as usize].take().is_some();
            if let Some(sy) = self.synth.as_mut() {
                if had && let Some(link) = sy.plugins.as_mut() {
                    link.clear(ch, DEFAULT_FADE_FRAMES);
                }
                if sy.control.routes.source(ch) == Source::Plugin {
                    sy.control.routes.set(ch, Source::SoundFont(0));
                }
            }
        }

        /// The editor handle of channel `ch`'s playing plugin (for the app's main thread).
        pub(crate) fn channel_editor(&self, ch: u8) -> Option<EditorTarget> {
            let c = self.plugins.channels[(ch & 15) as usize].as_ref()?;
            if c.status == PluginStatus::Playing { c.editor.clone() } else { self.plugins.playing[(ch & 15) as usize].as_ref()?.2.clone() }
        }

        /// Read channel `ch`'s playing plugin's state into its voice (and save it).
        pub(crate) fn save_channel_state(&mut self, ch: u8) -> Result<(), String> {
            let c = self.plugins.channels[(ch & 15) as usize].as_mut().ok_or("the part plays its SoundFont voice")?;
            let ed = c.editor.clone().ok_or("the plugin is not playing yet")?;
            c.voice.state = Some(ed.state().map_err(|e| format!("{e:#}"))?);
            self.plugins.dirty = true;
            Ok(())
        }

        pub(crate) fn channel_plugin_state(&self, ch: u8) -> Option<PartPlugin> {
            let c = self.plugins.channels[(ch & 15) as usize].as_ref()?;
            let (name, manufacturer) = c.info.as_ref().map(|i| (i.name.clone(), i.manufacturer.clone())).unwrap_or_default();
            let overruns = c.stats.as_ref().map_or(0, |s| s.overruns.load(std::sync::atomic::Ordering::Relaxed));
            Some(PartPlugin {
                id: c.voice.id.clone(),
                name,
                manufacturer,
                status: c.status,
                stage: c.stage.clone(),
                error: c.error.clone(),
                out_of_process: c.out_of_process,
                cpu: c.cpu,
                overruns,
                editor: c.status == PluginStatus::Playing,
            })
        }

        pub(crate) fn plugins_list(&self) -> PluginsState {
            PluginsState {
                available: self.plugin_ready().is_ok(),
                scanning: self.plugins.scan_rx.is_some(),
                list: self
                    .plugins
                    .list
                    .iter()
                    .map(|p| PluginEntry {
                        id: p.id.to_string(),
                        name: p.name.clone(),
                        manufacturer: p.manufacturer.clone(),
                        version: p.version_string(),
                        format: match p.format {
                            PluginFormat::Au2 => "AUv2",
                            PluginFormat::Au3 => "AUv3",
                        }
                        .into(),
                        last_error: p.last_load.as_ref().and_then(|l| l.error.clone()),
                    })
                    .collect(),
            }
        }

        /// Scan (from the cache: instant when nothing changed) on a thread.
        pub(crate) fn start_plugin_scan(&mut self, rescan: bool) {
            if self.plugins.scan_rx.is_some() {
                return;
            }
            let host = self.plugins.host();
            let (tx, rx) = mpsc::channel();
            let ok = std::thread::Builder::new().name("plugin-scan".into()).spawn(move || {
                let r = if rescan { host.rescan() } else { host.scan() };
                let _ = tx.send(r.map_err(|e| format!("{e:#}")));
            });
            if ok.is_ok() {
                self.plugins.scan_rx = Some(rx);
            }
        }

        /// Loads finishing, rack events, the CPU readout, the saved parts.
        pub(crate) fn pump_plugins(&mut self, now: u64) {
            if let Some(rx) = &self.plugins.scan_rx {
                match rx.try_recv() {
                    Ok(Ok(list)) => {
                        self.plugins.list = list;
                        self.plugins.scan_rx = None;
                    }
                    Ok(Err(e)) => {
                        self.plugins.scan_rx = None;
                        self.say(format!("plugin scan: {e}"), true);
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                    Err(mpsc::TryRecvError::Disconnected) => self.plugins.scan_rx = None,
                }
            }
            for ch in 0..16u8 {
                self.pump_channel_load(ch);
            }
            let events = match self.synth.as_mut().and_then(|s| s.plugins.as_mut()) {
                Some(link) => link.poll(),
                None => Vec::new(),
            };
            for e in events {
                if let RackEvent::Fault { channel, error } = e
                    && let Some(c) = self.plugins.channels[channel as usize].as_mut()
                    && c.status == PluginStatus::Playing
                {
                    c.status = PluginStatus::Muted;
                    c.error = Some(format!("{error}"));
                    let name = c.info.as_ref().map_or_else(|| c.voice.id.clone(), |i| i.name.clone());
                    self.say(format!("{name} stopped working; the part is muted. Choose it again or go back to the SoundFont voice."), true);
                }
            }
            // CPU once a second.
            if now.saturating_sub(self.plugins.stats_ns) >= 1_000_000_000 {
                self.plugins.stats_ns = now;
                let rate = self.synth.as_ref().map_or(48_000, |s| s.info.sample_rate) as f64;
                for c in self.plugins.channels.iter_mut().flatten() {
                    if let Some(s) = &c.stats {
                        use std::sync::atomic::Ordering::Relaxed;
                        let (t, f) = (s.total_ns.load(Relaxed), s.frames.load(Relaxed));
                        let (dt, df) = (t.saturating_sub(c.last.0), f.saturating_sub(c.last.1));
                        c.last = (t, f);
                        c.cpu = if df > 0 { ((dt as f64 / 1e9) / (df as f64 / rate)) as f32 } else { 0.0 };
                    }
                }
            }
            if self.plugins.dirty && self.offline.is_none() {
                self.plugins.dirty = false;
                self.save_plugin_parts();
            }
        }

        fn pump_channel_load(&mut self, ch: u8) {
            let Some(c) = self.plugins.channels[ch as usize].as_mut() else { return };
            let Some(load) = c.load.as_mut() else { return };
            let res = match load.take() {
                None => {
                    c.stage = stage_name(&load.progress());
                    return;
                }
                Some(r) => r,
            };
            c.load = None;
            c.stage = None;
            match res {
                Ok(inst) => {
                    let stats = inst.stats();
                    let editor = inst.editor_target();
                    let oop = inst.out_of_process();
                    let Some(link) = self.synth.as_mut().and_then(|s| s.plugins.as_mut()) else { return };
                    match link.assign(ch, inst, Swap::default()) {
                        Ok(()) => {
                            if let Some(sy) = &self.synth {
                                sy.control.routes.set(ch, Source::Plugin);
                            }
                            let c = self.plugins.channels[ch as usize].as_mut().unwrap();
                            c.status = PluginStatus::Playing;
                            c.error = None;
                            c.editor = Some(editor);
                            c.last = (stats.total_ns.load(std::sync::atomic::Ordering::Relaxed), stats.frames.load(std::sync::atomic::Ordering::Relaxed));
                            c.stats = Some(stats);
                            c.out_of_process = oop;
                            self.plugins.playing[ch as usize] = None;
                            self.plugins.dirty |= parts::part_of_channel(ch).is_some();
                        }
                        Err(inst) => {
                            dispose_later(inst);
                            self.channel_load_failed(ch, "the audio thread did not take the plugin".into());
                        }
                    }
                }
                Err(e) => {
                    let msg = format!("{e:#}");
                    // A third-party plugin that won't run in Apple's host: once in process.
                    if c.mode == LoadMode::OutOfProcess && !msg.contains("within") {
                        let voice = c.voice.clone();
                        if let Some(id) = PluginId::parse(&voice.id) {
                            let rate = self.synth.as_ref().map_or(48_000, |s| s.info.sample_rate) as f64;
                            let cfg = LoadConfig { sample_rate: rate, max_frames: crate::synth::PLUGIN_MAX_BLOCK as u32, state: voice.state.clone(), mode: LoadMode::InProcess, timeout: Duration::from_secs(20) };
                            if let Ok(h) = self.plugins.host().load_async(&id, cfg) {
                                let c = self.plugins.channels[ch as usize].as_mut().unwrap();
                                c.load = Some(h);
                                c.mode = LoadMode::InProcess;
                                c.stage = Some("queued".into());
                                return;
                            }
                        }
                    }
                    self.channel_load_failed(ch, msg);
                }
            }
        }

        fn channel_load_failed(&mut self, ch: u8, msg: String) {
            // Whatever played before (a previous plugin) is cleared: the part goes back to
            // its SoundFont voice, and shows why.
            let prev = self.plugins.playing[ch as usize].take();
            if prev.is_some()
                && let Some(link) = self.synth.as_mut().and_then(|s| s.plugins.as_mut())
            {
                link.clear(ch, DEFAULT_FADE_FRAMES);
            }
            if let Some(sy) = &self.synth
                && sy.control.routes.source(ch) == Source::Plugin
            {
                sy.control.routes.set(ch, Source::SoundFont(0));
            }
            let Some(c) = self.plugins.channels[ch as usize].as_mut() else { return };
            c.status = PluginStatus::Failed;
            c.error = Some(msg.clone());
            let name = c.info.as_ref().map_or_else(|| c.voice.id.clone(), |i| i.name.clone());
            self.say(format!("{name} didn't load: {msg}"), true);
        }

        /// The parts' plugins as saved (their last saved state).
        fn saved_parts(&self) -> Saved {
            let mut s = Saved::default();
            for p in 0..parts::COUNT {
                s.parts[p] = self.plugins.channels[parts::CHANNEL[p] as usize]
                    .as_ref()
                    .filter(|c| c.status != PluginStatus::Failed)
                    .map(|c| c.voice.clone());
            }
            s
        }

        fn save_plugin_parts(&self) {
            let Some(path) = saved_path() else { return };
            let s = self.saved_parts();
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Ok(json) = serde_json::to_string_pretty(&s) {
                let tmp = path.with_extension("json.tmp");
                if std::fs::write(&tmp, json).is_ok() {
                    let _ = std::fs::rename(&tmp, &path);
                }
            }
        }

        /// A live session's start: the plugin list, and the parts' saved plugins.
        pub(crate) fn restore_plugin_parts(&mut self) {
            if self.plugin_ready().is_err() {
                return;
            }
            self.start_plugin_scan(false);
            let Some(saved) = saved_path().and_then(|p| std::fs::read(p).ok()).and_then(|b| serde_json::from_slice::<Saved>(&b).ok()) else { return };
            for (p, v) in saved.parts.into_iter().enumerate() {
                if let Some(v) = v
                    && let Err(e) = self.assign_channel_plugin(parts::CHANNEL[p], v)
                {
                    self.say(format!("{}: {e}", parts::NAMES[p]), true);
                }
            }
        }

        /// Stopping: read the playing plugins' states and save them (with a deadline: a
        /// plugin that hangs reading its state does not hold up quitting).
        pub(crate) fn save_plugin_states_on_stop(&mut self) {
            if self.offline.is_some() {
                return;
            }
            let mut saved = self.saved_parts();
            let targets: Vec<(usize, EditorTarget)> = (0..parts::COUNT)
                .filter_map(|p| {
                    let c = self.plugins.channels[parts::CHANNEL[p] as usize].as_ref()?;
                    (c.status == PluginStatus::Playing).then(|| c.editor.clone()).flatten().map(|e| (p, e))
                })
                .collect();
            if !targets.is_empty() {
                let (tx, rx) = mpsc::channel();
                let _ = std::thread::Builder::new().name("plugin-save".into()).spawn(move || {
                    for (p, e) in targets {
                        if let Ok(s) = e.state() {
                            let _ = tx.send((p, s));
                        }
                    }
                });
                let deadline = std::time::Instant::now() + Duration::from_secs(2);
                while let Ok((p, s)) = rx.recv_timeout(deadline.saturating_duration_since(std::time::Instant::now())) {
                    if let Some(v) = saved.parts[p].as_mut() {
                        v.state = Some(s);
                    }
                }
            }
            let Some(path) = saved_path() else { return };
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Ok(json) = serde_json::to_string_pretty(&saved) {
                let _ = std::fs::write(path, json);
            }
        }
    }
}

#[cfg(feature = "plugins")]
pub(crate) use imp::PluginCtl;

/// Without the plugin host: nothing to keep.
#[cfg(not(feature = "plugins"))]
#[derive(Default)]
pub(crate) struct PluginCtl;

#[cfg(not(feature = "plugins"))]
impl Control {
    pub(crate) fn assign_channel_plugin(&mut self, _ch: u8, _voice: PluginVoice) -> Result<(), String> {
        Err("this build has no plugin host (the `plugins` feature)".into())
    }
    pub(crate) fn clear_channel_plugin(&mut self, _ch: u8) {}
    pub(crate) fn channel_plugin_state(&self, _ch: u8) -> Option<PartPlugin> {
        None
    }
    pub(crate) fn plugins_list(&self) -> PluginsState {
        PluginsState::default()
    }
    pub(crate) fn start_plugin_scan(&mut self, _rescan: bool) {}
    pub(crate) fn pump_plugins(&mut self, _now: u64) {}
    pub(crate) fn restore_plugin_parts(&mut self) {}
    pub(crate) fn save_plugin_states_on_stop(&mut self) {}
    pub(crate) fn save_channel_state(&mut self, _ch: u8) -> Result<(), String> {
        Err("this build has no plugin host".into())
    }
}

impl Control {
    /// Route channel `ch` to SoundFont `font` (0 = the synth's), clearing a plugin there.
    /// For the sound library's program map (#103).
    #[allow(dead_code)]
    pub(crate) fn route_channel_sound_font(&mut self, ch: u8, font: u8) {
        self.clear_channel_plugin(ch);
        if let Some(sy) = &self.synth {
            sy.control.routes.set(ch & 15, crate::route::Source::SoundFont(font));
        }
    }

    /// Channel `ch`'s plugin, if it has one (loading, playing, failed or muted).
    #[allow(dead_code)]
    pub(crate) fn channel_plugin(&self, ch: u8) -> Option<PartPlugin> {
        self.channel_plugin_state(ch)
    }

    pub(super) fn plugins_cmd(&mut self, c: PluginCmd) -> Result<(), CmdError> {
        let ch = |part: u8| parts::CHANNEL[(part & 3) as usize];
        match c {
            PluginCmd::SetPartPlugin { part, id, state } => {
                let state = match state.as_deref().filter(|s| !s.is_empty()) {
                    Some(s) => match base64_decode(s) {
                        Some(b) => Some(b),
                        None => return self.fail("the plugin state is not base64"),
                    },
                    None => None,
                };
                if let Err(e) = self.assign_channel_plugin(ch(part), PluginVoice { id, state }) {
                    return self.fail(e);
                }
            }
            PluginCmd::ClearPartPlugin { part } => {
                self.clear_channel_plugin(ch(part));
                self.mark_plugins_dirty();
            }
            PluginCmd::SavePartPluginState { part } => {
                if let Err(e) = self.save_channel_state(ch(part)) {
                    return self.fail(e);
                }
            }
            PluginCmd::RescanPlugins => self.start_plugin_scan(true),
        }
        Ok(())
    }

    fn mark_plugins_dirty(&mut self) {
        #[cfg(feature = "plugins")]
        {
            self.plugins.dirty = true;
        }
    }

    pub(super) fn plugins_state(&self) -> PluginsState {
        self.plugins_list()
    }

    /// The saved-voice form of a part's plugin, as `SetPartPlugin` takes it back (the id
    /// and the base64 state), for a client that stores voices itself.
    #[allow(dead_code)]
    pub(crate) fn part_plugin_voice(&self, part: usize) -> Option<(String, Option<String>)> {
        #[cfg(feature = "plugins")]
        {
            let c = self.plugins.channels[parts::CHANNEL[part & 3] as usize].as_ref()?;
            Some((c.voice.id.clone(), c.voice.state.as_deref().map(base64_encode)))
        }
        #[cfg(not(feature = "plugins"))]
        {
            let _ = (part, base64_encode as fn(&[u8]) -> String);
            None
        }
    }
}
