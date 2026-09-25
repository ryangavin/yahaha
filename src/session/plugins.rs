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
//!   All Notes Off on the SoundFont side. A load that fails or times out (20 s) leaves a
//!   plugin that was playing there playing; otherwise the channel plays its SoundFont and
//!   [`Control::channel_plugin`] shows `Failed` with the error, keeping the choice (saved,
//!   retryable) until it is picked again or cleared.
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
//! plugin the system refuses to host out of process (a typed status, see
//! `plugin::may_retry_in_process`) is retried in process once; never one that timed out or
//! crashed its host, and never during the start-up restore.
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
pub(crate) struct Saved {
    /// By part index (0 = Right 1, 1 = Right 2, 2 = Right 3, 3 = Left).
    pub(crate) parts: [Option<PluginVoice>; parts::COUNT],
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
        dispose_later, EditorTarget, InstanceRef, LoadConfig, LoadHandle, LoadMode, LoadProgress, PluginFormat, PluginHost, PluginId, PluginInfo,
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
        /// A load the system refuses to host out of process may be retried in process
        /// (`plugin::may_retry_in_process`). Never for the start-up restore.
        pub(crate) allow_fallback: bool,
        /// The system refused to host it out of process, so it was loaded in process
        /// instead (a crash in it would take yahaha down): the app shows it.
        pub(crate) fell_back: bool,
    }

    impl ChannelPlugin {
        /// A plugin that is not loaded: a failed pick or restore. It keeps the voice (and
        /// its saved state) so the part's choice survives until `clearPartPlugin`.
        fn failed(voice: PluginVoice, info: Option<PluginInfo>, error: String) -> ChannelPlugin {
            ChannelPlugin {
                voice,
                info,
                load: None,
                mode: LoadMode::Auto,
                status: PluginStatus::Failed,
                stage: None,
                error: Some(error),
                editor: None,
                stats: None,
                out_of_process: false,
                cpu: 0.0,
                last: (0, 0),
                allow_fallback: false,
                fell_back: false,
            }
        }

        fn name(&self) -> String {
            self.info.as_ref().map_or_else(|| self.voice.id.clone(), |i| i.name.clone())
        }
    }

    /// The largest plugin state accepted (a sampler's full state can be several MB).
    pub(crate) const MAX_STATE_BYTES: usize = 64 << 20;

    #[derive(Default)]
    pub(crate) struct PluginCtl {
        pub(crate) host: Option<PluginHost>,
        pub(crate) list: Vec<PluginInfo>,
        pub(crate) scan_rx: Option<mpsc::Receiver<Result<Vec<PluginInfo>, String>>>,
        pub(crate) channels: [Option<ChannelPlugin>; 16],
        /// A channel whose plugin was playing and is loading another: the one playing.
        pub(crate) playing: [Option<ChannelPlugin>; 16],
        pub(crate) stats_ns: u64,
        /// The last autosave of the parts' plugin states.
        pub(crate) autosave_ns: u64,
        /// Save the parts' plugins at the next pump (live sessions only).
        pub(crate) dirty: bool,
        /// Plugin state reads running on `plugin-state` threads (the autosave and
        /// `savePartPluginState`): an out-of-process plugin's state is an XPC round trip and
        /// a sampler's can be MBs, so the control thread never waits for one.
        pub(crate) state_reads: Vec<mpsc::Receiver<StateRead>>,
    }

    /// A plugin state read on a `plugin-state` thread.
    pub(crate) struct StateRead {
        ch: u8,
        /// The instance read (it applies only if the channel still plays it).
        inst: InstanceRef,
        state: Result<Vec<u8>, String>,
        /// Say it in the message line if it fails (an explicit save, not the autosave).
        report: bool,
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
            self.assign_channel_plugin_with(ch, voice, true)
        }

        fn assign_channel_plugin_with(&mut self, ch: u8, mut voice: PluginVoice, allow_fallback: bool) -> Result<(), String> {
            let ch = ch & 15;
            self.plugin_ready()?;
            // Picking a failed plugin again (the app sends no state) retries it with the
            // state it kept: a restore that timed out, or a plugin reinstalled since, comes
            // back as it was saved instead of fresh. Back to the SoundFont first starts it fresh.
            if voice.state.is_none()
                && let Some(prev) = self.plugins.channels[ch as usize].as_ref()
                && prev.status == PluginStatus::Failed
                && prev.voice.id == voice.id
            {
                voice.state = prev.voice.state.clone();
            }
            if voice.state.as_ref().is_some_and(|s| s.len() > MAX_STATE_BYTES) {
                return Err(format!("the plugin state is larger than {} MB", MAX_STATE_BYTES >> 20));
            }
            let id = PluginId::parse(&voice.id).ok_or_else(|| format!("{:?} is not a plugin id", voice.id))?;
            let host = self.plugins.host();
            let info = host.info(&id).map_err(|e| format!("{e:#}"))?;
            let mode = if id.manufacturer == APPLE || info.format == PluginFormat::Au3 { LoadMode::Auto } else { LoadMode::OutOfProcess };
            let rate = self.synth.as_ref().map_or(48_000, |s| s.info.sample_rate) as f64;
            let cfg = LoadConfig { sample_rate: rate, max_frames: crate::synth::PLUGIN_MAX_BLOCK as u32, state: voice.state.clone(), mode, timeout: Duration::from_secs(20) };
            let load = host.load_async(&id, cfg).map_err(|e| format!("{e:#}"))?;
            // What is in the rack now (playing, or muted by a fault) stays there until the
            // new one is ready: it keeps playing, and comes back if the new one fails. A
            // quick re-pick while loading keeps the one from before the first pick.
            if let Some(prev) = self.plugins.channels[ch as usize].take()
                && matches!(prev.status, PluginStatus::Playing | PluginStatus::Muted)
                && self.plugins.playing[ch as usize].is_none()
            {
                self.plugins.playing[ch as usize] = Some(prev);
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
                allow_fallback,
                fell_back: false,
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
            if c.status == PluginStatus::Playing {
                c.editor.clone()
            } else {
                self.plugins.playing[(ch & 15) as usize].as_ref().filter(|p| p.status == PluginStatus::Playing)?.editor.clone()
            }
        }

        /// Read channel `ch`'s playing plugin's state into its voice (and save it). The
        /// read runs on a thread of its own; the pump applies it.
        pub(crate) fn save_channel_state(&mut self, ch: u8) -> Result<(), String> {
            let c = self.plugins.channels[(ch & 15) as usize].as_ref().ok_or("the part plays its SoundFont voice")?;
            let ed = c.editor.clone().ok_or("the plugin is not playing yet")?;
            self.read_states(vec![(ch & 15, ed)], true);
            Ok(())
        }

        /// Read these instances' states on a `plugin-state` thread; the pump applies them.
        fn read_states(&mut self, targets: Vec<(u8, EditorTarget)>, report: bool) {
            let (tx, rx) = mpsc::channel();
            let ok = std::thread::Builder::new().name("plugin-state".into()).spawn(move || {
                for (ch, e) in targets {
                    let state = e.state().map_err(|e| format!("{e:#}"));
                    let inst = e.instance();
                    // The editor handle goes first: if it held the unit's last reference,
                    // the unit is disposed of here, not on the control thread.
                    drop(e);
                    if tx.send(StateRead { ch, inst, state, report }).is_err() {
                        return;
                    }
                }
            });
            match ok {
                Ok(_) => self.plugins.state_reads.push(rx),
                Err(e) if report => self.say(format!("could not read the plugin's settings: {e}"), true),
                Err(_) => {}
            }
        }

        /// Plugin states read since the last pump: into the channels' voices (to be saved),
        /// if the channel still plays the instance read.
        fn pump_state_reads(&mut self) {
            let mut done = Vec::new();
            self.plugins.state_reads.retain(|rx| loop {
                match rx.try_recv() {
                    Ok(r) => done.push(r),
                    Err(mpsc::TryRecvError::Empty) => break true,
                    Err(mpsc::TryRecvError::Disconnected) => break false,
                }
            });
            for r in done {
                let Some(c) = self.plugins.channels[r.ch as usize].as_mut() else { continue };
                if c.status != PluginStatus::Playing || !c.editor.as_ref().is_some_and(|e| r.inst.is(e)) {
                    continue;
                }
                match r.state {
                    Ok(s) => {
                        if c.voice.state.as_ref() != Some(&s) {
                            c.voice.state = Some(s);
                            self.plugins.dirty = true;
                        }
                    }
                    Err(e) if r.report => {
                        let name = c.name();
                        self.say(format!("{name}: could not read its settings ({e})"), true);
                    }
                    Err(_) => {}
                }
            }
        }

        /// Plugin state reads still running (their states land at a later pump).
        pub(crate) fn plugin_state_reads_pending(&self) -> bool {
            !self.plugins.state_reads.is_empty()
        }

        pub(crate) fn channel_plugin_state(&self, ch: u8) -> Option<PartPlugin> {
            let c = self.plugins.channels[(ch & 15) as usize].as_ref()?;
            // A plugin that isn't installed (a failed restore) has no info: its id names it.
            let (name, manufacturer) = (c.name(), c.info.as_ref().map(|i| i.manufacturer.clone()).unwrap_or_default());
            let overruns = c.stats.as_ref().map_or(0, |s| s.overruns.load(std::sync::atomic::Ordering::Relaxed));
            Some(PartPlugin {
                id: c.voice.id.clone(),
                name,
                manufacturer,
                status: c.status,
                stage: c.stage.clone(),
                error: c.error.clone(),
                out_of_process: c.out_of_process,
                in_process_fallback: c.fell_back,
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
                let RackEvent::Fault { channel, error } = e else { continue };
                let ch = channel as usize;
                // The instance in the rack: the channel's, or the one playing while a
                // replacement loads.
                let c = match self.plugins.channels[ch].as_mut() {
                    Some(c) if c.status == PluginStatus::Playing => Some(c),
                    _ => self.plugins.playing[ch].as_mut().filter(|p| p.status == PluginStatus::Playing),
                };
                if let Some(c) = c {
                    c.status = PluginStatus::Muted;
                    c.error = Some(format!("{error}"));
                    let name = c.name();
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
            // Every 30 s (live), the keyboard parts' plugin settings as the editor left
            // them, so a crash or a window closed with the red button loses little. Read
            // on a thread (not while the last reads are still running).
            self.pump_state_reads();
            if self.offline.is_none() && now.saturating_sub(self.plugins.autosave_ns) >= 30_000_000_000 && self.plugins.state_reads.is_empty() {
                self.plugins.autosave_ns = now;
                let targets: Vec<(u8, EditorTarget)> = parts::CHANNEL
                    .iter()
                    .filter_map(|&ch| {
                        let c = self.plugins.channels[ch as usize].as_ref()?;
                        (c.status == PluginStatus::Playing).then(|| c.editor.clone()).flatten().map(|e| (ch, e))
                    })
                    .collect();
                if !targets.is_empty() {
                    self.read_states(targets, false);
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
                    // Only a plugin the system refuses to host out of process at all is
                    // tried once in process; never one that crashed or hung there, and
                    // never during the start-up restore (#105 review B3).
                    if c.mode == LoadMode::OutOfProcess && c.allow_fallback && crate::plugin::may_retry_in_process(&e) {
                        let voice = c.voice.clone();
                        if let Some(id) = PluginId::parse(&voice.id) {
                            let rate = self.synth.as_ref().map_or(48_000, |s| s.info.sample_rate) as f64;
                            let cfg = LoadConfig { sample_rate: rate, max_frames: crate::synth::PLUGIN_MAX_BLOCK as u32, state: voice.state.clone(), mode: LoadMode::InProcess, timeout: Duration::from_secs(20) };
                            if let Ok(h) = self.plugins.host().load_async(&id, cfg) {
                                let c = self.plugins.channels[ch as usize].as_mut().unwrap();
                                c.load = Some(h);
                                c.mode = LoadMode::InProcess;
                                c.fell_back = true;
                                c.stage = Some("queued".into());
                                let name = c.name();
                                self.say(format!("{name} can't run in its own process; loading it inside yahaha instead (if it crashes, yahaha goes with it)"), false);
                                return;
                            }
                        }
                    }
                    self.channel_load_failed(ch, msg);
                }
            }
        }

        fn channel_load_failed(&mut self, ch: u8, msg: String) {
            let Some(failed) = self.plugins.channels[ch as usize].take() else { return };
            let name = failed.name();
            match self.plugins.playing[ch as usize].take() {
                // A working plugin was playing: it keeps the part (and stays saved).
                Some(prev) if prev.status == PluginStatus::Playing => {
                    self.say(format!("{name} didn't load ({msg}); {} keeps playing", prev.name()), true);
                    self.plugins.channels[ch as usize] = Some(prev);
                }
                // Nothing that plays: back to the SoundFont (clearing a faulted instance),
                // keeping the failed choice so it can be retried and is not forgotten.
                prev => {
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
                    self.say(format!("{name} didn't load: {msg}"), true);
                    self.plugins.channels[ch as usize] = Some(ChannelPlugin::failed(failed.voice, failed.info, msg));
                }
            }
            self.plugins.dirty |= parts::part_of_channel(ch).is_some();
        }

        /// The parts' plugins as saved (their last saved state).
        pub(crate) fn saved_parts(&self) -> Saved {
            let mut s = Saved::default();
            for p in 0..parts::COUNT {
                // Whatever its status: only `clearPartPlugin` forgets a part's plugin. A
                // loading replacement is saved as the choice; the one it replaces is kept
                // in `playing` and comes back to the channel if the load fails.
                s.parts[p] = self.plugins.channels[parts::CHANNEL[p] as usize].as_ref().map(|c| c.voice.clone());
            }
            s
        }

        fn save_plugin_parts(&self) {
            write_saved(&self.saved_parts());
        }

        /// A live session's start: the plugin list, and the parts' saved plugins.
        pub(crate) fn restore_plugin_parts(&mut self) {
            if self.plugin_ready().is_err() {
                return;
            }
            self.start_plugin_scan(false);
            let Some(path) = saved_path() else { return };
            let Ok(bytes) = std::fs::read(&path) else { return };
            let saved = match serde_json::from_slice::<Saved>(&bytes) {
                Ok(s) => s,
                Err(e) => {
                    // Keep the file for the user (it may be recoverable) rather than
                    // overwriting it with nothing on the next save.
                    let bak = path.with_extension("json.bak");
                    let _ = std::fs::rename(&path, &bak);
                    self.say(format!("the saved plugin parts could not be read ({e}); moved to {}", bak.display()), true);
                    return;
                }
            };
            self.restore_saved(saved);
        }

        /// Load the saved parts' plugins (the start-up restore).
        pub(crate) fn restore_saved(&mut self, saved: Saved) {
            for (p, v) in saved.parts.into_iter().enumerate() {
                let Some(v) = v else { continue };
                let ch = parts::CHANNEL[p];
                // No in-process fallback here: a plugin that crashed its host process last
                // time must not take yahaha down at every start.
                if let Err(e) = self.assign_channel_plugin_with(ch, v.clone(), false) {
                    self.say(format!("{}: {e}", parts::NAMES[p]), true);
                    // Kept (not forgotten) so a reinstalled plugin can be picked again with
                    // its saved state.
                    self.plugins.channels[ch as usize] = Some(ChannelPlugin::failed(v, None, e));
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
            write_saved(&saved);
        }
    }

    /// Write the saved parts atomically (a temporary file, then a rename), so a crash mid
    /// write never leaves a truncated file.
    fn write_saved(s: &Saved) {
        let Some(path) = saved_path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string_pretty(s) {
            let tmp = path.with_extension("json.tmp");
            if std::fs::write(&tmp, json).is_ok() {
                let _ = std::fs::rename(&tmp, &path);
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
    pub(crate) fn plugin_state_reads_pending(&self) -> bool {
        false
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
                    // 64 MB of state (base64 is 4/3 of it): refuse before decoding.
                    Some(s) if s.len() > (64usize << 20) / 3 * 4 + 4 => return self.fail("the plugin state is larger than 64 MB"),
                    Some(s) => match base64_decode(s) {
                        Some(b) => Some(b),
                        None => return self.fail("the plugin state is not base64"),
                    },
                    None => None,
                };
                // A plugin picked here ends the part's own library patch (#103).
                self.sound_library_part_plugin(part as usize, true);
                if let Err(e) = self.assign_channel_plugin(ch(part), PluginVoice { id, state }) {
                    return self.fail(e);
                }
            }
            PluginCmd::ClearPartPlugin { part } => {
                self.sound_library_part_plugin(part as usize, false);
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

    pub(super) fn mark_plugins_dirty(&mut self) {
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

#[cfg(all(test, feature = "plugins"))]
#[path = "plugins_tests.rs"]
mod tests;
