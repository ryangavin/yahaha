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
use crate::api::{base64_decode, base64_encode, CmdError, PartPlugin, PluginCmd, PluginStatus, PluginsState};
use crate::parts;
use serde::{Deserialize, Serialize};
#[cfg(feature = "plugins")]
use std::path::PathBuf;

/// What a channel plays when it plays a plugin: which one and its preset.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PluginVoice {
    /// "aumu dls  appl" (the phase-1 `PluginId` string form).
    pub id: String,
    /// The plugin's full state (ClassInfo bytes); None = its default preset (or `preset`).
    #[serde(default, with = "b64opt")]
    pub state: Option<Vec<u8>>,
    /// The preset picked in the Sound Browser (AU presets), if any. A factory preset with
    /// no `state` yet loads by number; once the state is read it restores from that.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<VoicePreset>,
    /// The library Sound it plays (docs/sound-browser.md), if known. A plugin-parts.json
    /// written before sounds had ids has none: a part that played a preset gets its sound
    /// when it plays ([`crate::patches::SoundLibrary::add_plugin_preset`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sound: Option<crate::patches::SoundTag>,
}

/// A plugin preset a part plays: its catalog key and name.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct VoicePreset {
    /// `f:<number>` (a factory preset) or `u:<path>` (an `.aupreset`).
    pub key: String,
    pub name: String,
}

impl VoicePreset {
    /// A factory preset's number.
    pub fn factory_number(&self) -> Option<i32> {
        self.key.strip_prefix("f:")?.parse().ok()
    }
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
    use crate::api::PluginEntry;
    use crate::plugin::{
        dispose_later, EditorTarget, InstanceRef, LoadConfig, LoadHandle, LoadMode, LoadProgress, PluginFormat, PluginHost, PluginId, PluginInfo,
        PluginStats, RackEvent, Swap, DEFAULT_FADE_FRAMES,
    };
    use crate::route::Source;
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::time::Duration;

    /// The factory preset a voice loads by number: only while it has no state of its own.
    fn factory_preset(v: &PluginVoice) -> Option<(i32, String)> {
        let p = v.preset.as_ref().filter(|_| v.state.is_none())?;
        Some((p.factory_number()?, p.name.clone()))
    }

    /// A `listPluginPresets` running: the plugin, and the listing thread's answer.
    pub(crate) type PresetListing = (String, mpsc::Receiver<Result<PluginInfo, String>>);

    /// What a `savePartAsPluginPreset` thread hands back: the file written, the state it
    /// holds, and the plugin as the cache now lists it.
    type SaveResult = Result<(crate::plugin::UserPreset, Vec<u8>, Option<PluginInfo>), String>;

    /// A `savePartAsPluginPreset` running on a `plugin-preset` thread.
    pub(crate) struct PresetSave {
        ch: u8,
        inst: InstanceRef,
        category: crate::api::PatchCategory,
        rx: mpsc::Receiver<SaveResult>,
    }

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
        /// Its overruns over the last few seconds (the live readout).
        pub(crate) recent: OverrunWindow,
        /// A load the system refuses to host out of process may be retried in process
        /// (`plugin::may_retry_in_process`). Never for the start-up restore.
        pub(crate) allow_fallback: bool,
        /// The system refused to host it out of process, so it was loaded in process
        /// instead (a crash in it would take yahaha down): the app shows it.
        pub(crate) fell_back: bool,
        /// The fingerprint of the plugin's own state as its sound left it: the first read
        /// after it loaded (or after a Save), taken on a `plugin-state` thread. None until
        /// then, or when the voice names no sound (docs/sound-browser.md, "Edited").
        pub(crate) sound_fp: Option<u64>,
        /// The plugin's state no longer matches the sound it was loaded from. It stays set
        /// until a Save, Save as… or another sound.
        pub(crate) edited: bool,
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
                recent: OverrunWindow::default(),
                allow_fallback: false,
                fell_back: false,
                sound_fp: None,
                edited: false,
            }
        }

        fn name(&self) -> String {
            self.info.as_ref().map_or_else(|| self.voice.id.clone(), |i| i.name.clone())
        }
    }

    /// How many seconds the live overrun readout (`PartPlugin::recent_overruns`) covers.
    pub(crate) const OVERRUN_WINDOW_SECS: usize = 10;

    /// A plugin's overruns per second over the last `OVERRUN_WINDOW_SECS` seconds, from its
    /// running total (read once a second on the control thread).
    #[derive(Clone, Debug, Default, PartialEq)]
    pub(crate) struct OverrunWindow {
        /// The total at the last reading.
        total: u64,
        secs: [u32; OVERRUN_WINDOW_SECS],
        at: usize,
    }

    impl OverrunWindow {
        pub(crate) fn starting_at(total: u64) -> OverrunWindow {
            OverrunWindow { total, ..OverrunWindow::default() }
        }

        /// A new reading of the running total: one second more.
        pub(crate) fn tick(&mut self, total: u64) {
            self.secs[self.at] = total.saturating_sub(self.total).min(u32::MAX as u64) as u32;
            self.at = (self.at + 1) % OVERRUN_WINDOW_SECS;
            self.total = total;
        }

        /// The overruns in the window.
        pub(crate) fn count(&self) -> u32 {
            self.secs.iter().fold(0u32, |a, &n| a.saturating_add(n))
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
        /// Plugins preloaded for the Registration bank's buttons (plugins/pool.rs).
        pub(crate) warm: super::pool::WarmPool,
        /// Preset listings running (`listPluginPresets`).
        pub(crate) listing: Vec<PresetListing>,
        /// Saves as a user preset running (`savePartAsPluginPreset`).
        pub(crate) preset_saves: Vec<PresetSave>,
    }

    /// A plugin state read on a `plugin-state` thread.
    pub(crate) struct StateRead {
        ch: u8,
        /// The instance read (it applies only if the channel still plays it).
        inst: InstanceRef,
        state: Result<Vec<u8>, String>,
        /// The state's fingerprint ([`state_fingerprint`]), taken on the reading thread.
        fp: u64,
        /// Say it in the message line if it fails (an explicit save, not the autosave).
        report: bool,
    }

    /// A plugin state's fingerprint: what the "edited" check compares, so it never holds
    /// or compares whole states. Taken on the `plugin-state` thread that read the state,
    /// never on the control or audio threads.
    pub(crate) fn state_fingerprint(state: &[u8]) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::hash::DefaultHasher::new();
        state.hash(&mut h);
        h.finish()
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

    /// Where a plugin loads: in process if the player chose that (`setPluginInProcess`),
    /// else Apple's units and AUv3s as macOS decides (`Auto`), and everything else in its
    /// own process.
    pub(crate) fn load_mode(info: &PluginInfo) -> LoadMode {
        if info.in_process && info.can_run_in_process() {
            LoadMode::InProcess
        } else if info.id.manufacturer == APPLE || info.format == PluginFormat::Au3 {
            LoadMode::Auto
        } else {
            LoadMode::OutOfProcess
        }
    }

    impl PluginCtl {
        pub(crate) fn host(&mut self) -> PluginHost {
            self.host.get_or_insert_with(PluginHost::with_default_cache).clone()
        }
    }

    impl Control {
        pub(crate) fn plugin_ready(&self) -> Result<(), String> {
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
                && prev.voice.preset == voice.preset
            {
                voice.state = prev.voice.state.clone();
            }
            if voice.state.as_ref().is_some_and(|s| s.len() > MAX_STATE_BYTES) {
                return Err(format!("the plugin state is larger than {} MB", MAX_STATE_BYTES >> 20));
            }
            // A plugin preloaded for a Registration button plays at once; else it loads now.
            let (info, mode, load) = match self.take_warm(&voice) {
                Some(w) => (w.info, w.mode, w.load),
                None => self.start_load(&voice)?,
            };
            // What is in the rack now (playing, or muted by a fault) stays there until the
            // new one is ready: it keeps playing, and comes back if the new one fails. A
            // quick re-pick while loading keeps the one from before the first pick.
            if let Some(prev) = self.plugins.channels[ch as usize].take()
                && matches!(prev.status, PluginStatus::Playing | PluginStatus::Muted)
                && self.plugins.playing[ch as usize].is_none()
            {
                self.plugins.playing[ch as usize] = Some(prev);
            }
            // A recalled or restored state that isn't its sound's is edited from the start.
            let edited = self.sound_state_differs(voice.sound.as_ref(), voice.state.as_deref());
            self.plugins.channels[ch as usize] = Some(ChannelPlugin {
                voice,
                info,
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
                recent: OverrunWindow::default(),
                allow_fallback,
                fell_back: false,
                sound_fp: None,
                edited,
            });
            Ok(())
        }

        /// Start loading `voice` on a thread of its own, as a part plays it ([`load_mode`]).
        ///
        /// The plugin is checked against the scanned list, never by scanning here: a stale
        /// scan cache means a full component scan, and this runs on the control thread under
        /// the Session lock. Before the first scan is in, the load thread looks the plugin up
        /// (and chooses its mode); one that is not installed fails there, through the handle.
        /// Once the list is in, an id missing from it is refused at once. The info is None
        /// until the load thread has it (`LoadHandle::info`).
        pub(crate) fn start_load(&mut self, voice: &PluginVoice) -> Result<(Option<PluginInfo>, LoadMode, LoadHandle), String> {
            let id = PluginId::parse(&voice.id).ok_or_else(|| format!("{:?} is not a plugin id", voice.id))?;
            let info = self.plugins.list.iter().find(|p| p.id == id).cloned();
            if info.is_none() && self.plugins.scan_rx.is_none() && !self.plugins.list.is_empty() {
                return Err(format!("no instrument Audio Unit {id} is installed (rescan the plugins if it was just installed)"));
            }
            let (mode, choose_mode) = match &info {
                Some(i) => (load_mode(i), None),
                None => (LoadMode::Auto, Some(load_mode as fn(&PluginInfo) -> LoadMode)),
            };
            let cfg = LoadConfig {
                sample_rate: self.plugin_rate(),
                max_frames: crate::synth::PLUGIN_MAX_BLOCK as u32,
                state: voice.state.clone(),
                mode,
                timeout: Duration::from_secs(20),
                choose_mode,
                factory_preset: factory_preset(voice),
            };
            let load = self.plugins.host().load_async(&id, cfg).map_err(|e| format!("{e:#}"))?;
            Ok((info, mode, load))
        }

        /// The rate plugins load at: the synth's.
        pub(crate) fn plugin_rate(&self) -> f64 {
            self.synth.as_ref().map_or(48_000, |s| s.info.sample_rate) as f64
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

        /// Keep `voice` on channel `ch` as failed with `error` (after a load that could not
        /// start): the choice survives, saved and retryable, as a failed start-up restore's
        /// does. The channel plays its SoundFont meanwhile.
        pub(crate) fn keep_failed_channel_plugin(&mut self, ch: u8, voice: PluginVoice, error: String) {
            self.plugins.channels[(ch & 15) as usize] = Some(ChannelPlugin::failed(voice, None, error));
            self.plugins.dirty = true;
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
                    let fp = state.as_deref().map_or(0, state_fingerprint);
                    let inst = e.instance();
                    // The editor handle goes first: if it held the unit's last reference,
                    // the unit is disposed of here, not on the control thread.
                    drop(e);
                    if tx.send(StateRead { ch, inst, state, fp, report }).is_err() {
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
            let mut captures = Vec::new();
            for r in done {
                let Some(c) = self.plugins.channels[r.ch as usize].as_mut() else { continue };
                if c.status != PluginStatus::Playing || !c.editor.as_ref().is_some_and(|e| r.inst.is(e)) {
                    continue;
                }
                match r.state {
                    Ok(s) => {
                        if c.voice.state.as_ref() != Some(&s) {
                            c.voice.state = Some(s.clone());
                            self.plugins.dirty = true;
                        }
                        // Edited: the first read after the sound loaded is its baseline
                        // (a plugin's own serialization, not the stored bytes); a later
                        // read that differs means the editor changed it.
                        if c.voice.sound.is_some() {
                            match c.sound_fp {
                                None => c.sound_fp = Some(r.fp),
                                Some(fp) => c.edited |= fp != r.fp,
                            }
                        }
                        // A factory preset's first play: its sound keeps the state.
                        if let Some(tag) = c.voice.sound.clone() {
                            captures.push((tag, s));
                        }
                    }
                    Err(e) if r.report => {
                        let name = c.name();
                        self.say(format!("{name}: could not read its settings ({e})"), true);
                    }
                    Err(_) => {}
                }
            }
            for (tag, st) in captures {
                self.capture_sound_state(&tag, &st);
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
                recent_overruns: c.recent.count(),
                editor: c.status == PluginStatus::Playing,
                preset: c.voice.preset.as_ref().map(|p| p.name.clone()),
                preset_key: c.voice.preset.as_ref().map(|p| p.key.clone()),
            })
        }

        /// The Sound channel `ch`'s plugin plays, and whether it is edited (O3).
        pub(crate) fn channel_sound(&self, ch: u8) -> Option<(Option<crate::patches::SoundTag>, bool)> {
            let c = self.plugins.channels[(ch & 15) as usize].as_ref()?;
            Some((c.voice.sound.clone(), c.edited && c.voice.sound.is_some()))
        }

        /// Channel `ch`'s plugin now plays `tag` as it is (a Save or Save as…): not edited,
        /// and the next state read is the new baseline.
        pub(crate) fn adopt_channel_sound(&mut self, ch: u8, tag: Option<crate::patches::SoundTag>) {
            let Some(c) = self.plugins.channels[(ch & 15) as usize].as_mut() else { return };
            c.voice.sound = tag;
            c.sound_fp = None;
            c.edited = false;
            self.plugins.dirty |= parts::part_of_channel(ch).is_some();
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
                        in_process: p.in_process,
                        can_run_in_process: p.can_run_in_process(),
                    })
                    .collect(),
            }
        }

        /// The player's "run in process" override for plugin `id`, saved in the scan cache.
        /// It applies from the plugin's next load; a part playing it now keeps running where
        /// it is. What the Registration bank preloaded of it loads again in the new mode
        /// (`rewarm_plugin`, #176).
        pub(crate) fn set_plugin_in_process(&mut self, id: &str, on: bool) -> Result<(), String> {
            let pid = PluginId::parse(id).ok_or_else(|| format!("{id:?} is not a plugin id"))?;
            let info = self.plugins.host().set_in_process(&pid, on).map_err(|e| format!("{e:#}"))?;
            let mut changed = true;
            if let Some(p) = self.plugins.list.iter_mut().find(|p| p.id == pid) {
                changed = p.in_process != info.in_process;
                p.in_process = info.in_process;
            }
            if changed {
                self.rewarm_plugin(&pid.to_string());
            }
            let playing = self.plugins.channels.iter().flatten().any(|c| c.voice.id == id && c.status == PluginStatus::Playing);
            if playing {
                let r#where = if on { "inside yahaha" } else { "in its own process" };
                self.say(format!("{} runs {where} from its next load (the next start, or pick it again)", info.name), false);
            }
            Ok(())
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

        /// The voice that plays preset `key` of plugin `id` (a catalog key, `f:<n>` or
        /// `u:<path>`): a factory preset loads by number; an `.aupreset` file (only one the
        /// scan listed for the plugin) is read here as the state it is.
        pub(crate) fn preset_voice(&self, id: &str, key: &str) -> Result<PluginVoice, String> {
            let pid = PluginId::parse(id).ok_or_else(|| format!("{id:?} is not a plugin id"))?;
            let info = self.plugins.list.iter().find(|p| p.id == pid).ok_or_else(|| format!("no instrument Audio Unit {id} is installed"))?;
            if let Some(n) = key.strip_prefix("f:").and_then(|n| n.parse::<i32>().ok()) {
                let p = info.factory_presets.iter().flatten().find(|p| p.number == n).ok_or_else(|| format!("{} has no preset {key}", info.name))?;
                return Ok(PluginVoice { id: id.to_string(), state: None, preset: Some(VoicePreset { key: key.to_string(), name: p.name.clone() }), sound: None });
            }
            let path = key.strip_prefix("u:").ok_or_else(|| format!("{key:?} is not a preset key"))?;
            let p = info.user_presets.iter().find(|p| p.path.to_str() == Some(path)).ok_or_else(|| format!("{} has no preset file {path}", info.name))?;
            let len = std::fs::metadata(&p.path).map(|m| m.len()).map_err(|e| format!("{}: {e}", p.path.display()))?;
            if len as usize > MAX_STATE_BYTES {
                return Err(format!("the preset is larger than {} MB", MAX_STATE_BYTES >> 20));
            }
            let bytes = std::fs::read(&p.path).map_err(|e| format!("{}: {e}", p.path.display()))?;
            Ok(PluginVoice { id: id.to_string(), state: Some(bytes), preset: Some(VoicePreset { key: key.to_string(), name: p.name.clone() }), sound: None })
        }

        /// Every plugin's presets as the catalog lists them: factory presets (once read),
        /// then `.aupreset` files.
        pub(crate) fn plugin_preset_lists(&self) -> Vec<crate::api::PluginPresetList> {
            use crate::api::{PluginPresetEntry, PluginPresetList};
            self.plugins
                .list
                .iter()
                .filter(|p| p.factory_presets.is_some() || !p.user_presets.is_empty())
                .map(|p| PluginPresetList {
                    plugin: p.id.to_string(),
                    listed: p.factory_presets.is_some(),
                    presets: p
                        .factory_presets
                        .iter()
                        .flatten()
                        .map(|f| PluginPresetEntry { key: format!("f:{}", f.number), name: f.name.clone(), folder: None })
                        .chain(p.user_presets.iter().map(|u| PluginPresetEntry { key: format!("u:{}", u.path.display()), name: u.name.clone(), folder: u.folder.clone() }))
                        .collect(),
                })
                .collect()
        }

        /// `listPluginPresets`: read plugin `id`'s factory presets on a thread (loading an
        /// instance once if no load has read them yet). Nothing to do when they are known.
        pub(crate) fn list_plugin_presets(&mut self, id: &str) -> Result<(), String> {
            let pid = PluginId::parse(id).ok_or_else(|| format!("{id:?} is not a plugin id"))?;
            let Some(info) = self.plugins.list.iter().find(|p| p.id == pid) else {
                return Err(format!("no instrument Audio Unit {id} is installed"));
            };
            if info.factory_presets.is_some() || self.plugins.listing.iter().any(|(l, _)| l == id) {
                return Ok(());
            }
            let cfg = LoadConfig {
                sample_rate: self.plugin_rate(),
                max_frames: crate::synth::PLUGIN_MAX_BLOCK as u32,
                mode: load_mode(info),
                timeout: Duration::from_secs(20),
                ..LoadConfig::default()
            };
            let host = self.plugins.host();
            let (tx, rx) = mpsc::channel();
            std::thread::Builder::new()
                .name("plugin-presets".into())
                .spawn(move || {
                    let _ = tx.send(host.list_presets(&pid, cfg).map_err(|e| format!("{e:#}")));
                })
                .map_err(|e| format!("could not start the preset listing: {e}"))?;
            self.plugins.listing.push((id.to_string(), rx));
            Ok(())
        }

        /// Preset listings running (`listPluginPresets`), by plugin id.
        pub(crate) fn plugin_presets_listing(&self) -> Vec<String> {
            self.plugins.listing.iter().map(|(id, _)| id.clone()).collect()
        }

        /// The list's copy of plugin `id` from the host's cache (which a load just gave its
        /// factory presets).
        fn refresh_listed_presets(&mut self, id: &str) {
            let Some(pid) = PluginId::parse(id) else { return };
            let Some(i) = self.plugins.list.iter().position(|p| p.id == pid) else { return };
            if self.plugins.list[i].factory_presets.is_some() {
                return;
            }
            if let Some(fresh) = self.plugins.host().cached(&pid) {
                self.plugins.list[i] = fresh;
            }
        }

        /// `savePartAsPluginPreset`: read the part's plugin state, write it as an
        /// `.aupreset`, list it, file it under `category`, and make it the part's preset.
        /// The read and the write run on a thread; the pump finishes.
        pub(crate) fn save_part_as_preset(&mut self, ch: u8, name: &str, category: crate::api::PatchCategory, overwrite: bool) -> Result<(), String> {
            let name = name.trim();
            if name.is_empty() {
                return Err("name the preset".into());
            }
            let c = self.plugins.channels[(ch & 15) as usize].as_ref().ok_or("the part plays its SoundFont voice, not a plugin")?;
            let ed = c.editor.clone().filter(|_| c.status == PluginStatus::Playing).ok_or("the plugin is not playing yet")?;
            let pid = PluginId::parse(&c.voice.id).ok_or("the part's plugin has no valid id")?;
            let (host, name) = (self.plugins.host(), name.to_string());
            // Refused at once (the app asks "Replace?" and sends `overwrite`); the write
            // checks again, in case the file appeared meanwhile.
            if !overwrite && host.user_preset_exists(&pid, &name) {
                return Err(crate::plugin::presets::PresetExists(crate::plugin::presets::safe_name(&name)).to_string());
            }
            let inst = ed.instance();
            let (tx, rx) = mpsc::channel();
            std::thread::Builder::new()
                .name("plugin-preset".into())
                .spawn(move || {
                    let r = ed.state().and_then(|state| {
                        let saved = host.save_user_preset(&pid, &name, &state, overwrite)?;
                        Ok((saved, state, host.cached(&pid)))
                    });
                    // The editor handle goes here: if it held the unit's last reference, the
                    // unit is disposed of on the dispose thread, not the control thread.
                    drop(ed);
                    let _ = tx.send(r.map_err(|e| format!("{e:#}")));
                })
                .map_err(|e| format!("could not start saving the preset: {e}"))?;
            self.plugins.preset_saves.push(PresetSave { ch: ch & 15, inst, category, rx });
            Ok(())
        }

        /// Write a plugin sound's `state` as the user preset `name` of its plugin (the
        /// `.aupreset` Save as preset writes, in the folder Logic reads). An existing preset
        /// of that name is refused unless `overwrite` (the app asks Replace/Cancel, #307).
        /// The file is small and written at once; no instance is needed.
        pub(crate) fn export_state_as_preset(&mut self, component_id: &str, name: &str, state: &[u8], overwrite: bool) -> Result<std::path::PathBuf, String> {
            let name = name.trim();
            if name.is_empty() {
                return Err("name the preset".into());
            }
            let pid = PluginId::parse(component_id).ok_or("the sound's plugin has no valid id")?;
            let host = self.plugins.host();
            if host.cached(&pid).is_none() {
                return Err("the sound's plugin is not installed".into());
            }
            if !overwrite && host.user_preset_exists(&pid, name) {
                return Err(crate::plugin::presets::PresetExists(crate::plugin::presets::safe_name(name)).to_string());
            }
            host.save_user_preset(&pid, name, state, overwrite).map(|p| p.path).map_err(|e| format!("{e:#}"))
        }

        /// Preset listings and saves that finished.
        fn pump_presets(&mut self) {
            let mut done = Vec::new();
            self.plugins.listing.retain(|(id, rx)| match rx.try_recv() {
                Ok(r) => {
                    done.push((id.clone(), r));
                    false
                }
                Err(mpsc::TryRecvError::Empty) => true,
                Err(mpsc::TryRecvError::Disconnected) => false,
            });
            for (id, r) in done {
                match r {
                    Ok(info) => {
                        if let Some(p) = self.plugins.list.iter_mut().find(|p| p.id == info.id) {
                            *p = info;
                        }
                    }
                    Err(e) => self.say(format!("{id}: could not list its presets ({e})"), true),
                }
            }
            let mut saves = Vec::new();
            self.plugins.preset_saves.retain(|s| match s.rx.try_recv() {
                Ok(r) => {
                    saves.push((s.ch, s.inst.clone(), s.category, r));
                    false
                }
                Err(mpsc::TryRecvError::Empty) => true,
                Err(mpsc::TryRecvError::Disconnected) => false,
            });
            for (ch, inst, category, r) in saves {
                let (saved, state, info) = match r {
                    Ok(x) => x,
                    Err(e) => {
                        self.say(format!("The preset was not saved: {e}"), true);
                        continue;
                    }
                };
                let key = format!("u:{}", saved.path.display());
                if let Some(info) = info
                    && let Some(p) = self.plugins.list.iter_mut().find(|p| p.id == info.id)
                {
                    let id = info.id.to_string();
                    *p = info;
                    self.set_preset_category(crate::api::plugin_preset_id(&id, &key), category);
                }
                // The part plays it now, if it still plays the instance saved.
                if let Some(c) = self.plugins.channels[ch as usize].as_mut()
                    && c.editor.as_ref().is_some_and(|e| inst.is(e))
                {
                    c.voice.preset = Some(VoicePreset { key, name: saved.name.clone() });
                    c.voice.state = Some(state);
                    self.plugins.dirty = true;
                }
                self.say(format!("Saved the preset “{}” ({})", saved.name, saved.path.display()), false);
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
            self.pump_presets();
            self.pump_warm();
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
                        c.recent.tick(s.overruns.load(Relaxed));
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
            // Looked up on the load thread (the plugin was not in the list yet): its name
            // and the mode it loaded in.
            if c.info.is_none() {
                c.info = load.info();
            }
            if let Some(m) = load.mode() {
                c.mode = m;
            }
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
                            c.recent = OverrunWindow::starting_at(stats.overruns.load(std::sync::atomic::Ordering::Relaxed));
                            c.stats = Some(stats);
                            c.out_of_process = oop;
                            self.plugins.playing[ch as usize] = None;
                            self.plugins.dirty |= parts::part_of_channel(ch).is_some();
                            // A preset picked by number: read the state it gives, so the part
                            // (and a Registration memorized now) keeps the sound itself.
                            // A keyboard part's preset is a library Sound (the
                            // plugin-parts.json migration, and a preset picked now).
                            if parts::part_of_channel(ch).is_some() {
                                let c = self.plugins.channels[ch as usize].as_mut().unwrap();
                                let mut voice = std::mem::take(&mut c.voice);
                                let category = c.info.as_ref().map_or(crate::patches::Category::SynthLead, |i| crate::api::plugin_category(&i.name, &i.manufacturer));
                                self.plugins.dirty |= self.link_voice_sound(&mut voice, category);
                                self.plugins.channels[ch as usize].as_mut().unwrap().voice = voice;
                            }
                            let c = self.plugins.channels[ch as usize].as_ref().unwrap();
                            let id = c.voice.id.clone();
                            // A keyboard part's sound: that first read is also the "edited"
                            // check's baseline.
                            let baseline = parts::part_of_channel(ch).is_some() && c.voice.sound.is_some();
                            if (c.voice.preset.is_some() && c.voice.state.is_none()) || baseline {
                                let e = c.editor.clone().unwrap();
                                self.read_states(vec![(ch, e)], false);
                            }
                            // Its load read the factory presets, if they were not known.
                            self.refresh_listed_presets(&id);
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
                            let cfg = LoadConfig { sample_rate: rate, max_frames: crate::synth::PLUGIN_MAX_BLOCK as u32, state: voice.state.clone(), mode: LoadMode::InProcess, timeout: Duration::from_secs(20), choose_mode: None, factory_preset: factory_preset(&voice) };
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
    pub(crate) fn keep_failed_channel_plugin(&mut self, _ch: u8, _voice: PluginVoice, _error: String) {}
    pub(crate) fn channel_plugin_state(&self, _ch: u8) -> Option<PartPlugin> {
        None
    }
    pub(crate) fn plugin_state_reads_pending(&self) -> bool {
        false
    }
    pub(crate) fn channel_sound(&self, _ch: u8) -> Option<(Option<crate::patches::SoundTag>, bool)> {
        None
    }
    pub(crate) fn adopt_channel_sound(&mut self, _ch: u8, _tag: Option<crate::patches::SoundTag>) {}
    pub(crate) fn warm_plugins(&mut self, _want: Vec<PluginVoice>) {}
    pub(crate) fn plugins_list(&self) -> PluginsState {
        PluginsState::default()
    }
    pub(crate) fn start_plugin_scan(&mut self, _rescan: bool) {}
    pub(crate) fn set_plugin_in_process(&mut self, _id: &str, _on: bool) -> Result<(), String> {
        Err("this build has no plugin host".into())
    }
    pub(crate) fn pump_plugins(&mut self, _now: u64) {}
    pub(crate) fn restore_plugin_parts(&mut self) {}
    pub(crate) fn save_plugin_states_on_stop(&mut self) {}
    pub(crate) fn save_channel_state(&mut self, _ch: u8) -> Result<(), String> {
        Err("this build has no plugin host".into())
    }
    pub(crate) fn preset_voice(&self, _id: &str, _key: &str) -> Result<PluginVoice, String> {
        Err("this build has no plugin host".into())
    }
    pub(crate) fn plugin_preset_lists(&self) -> Vec<crate::api::PluginPresetList> {
        Vec::new()
    }
    pub(crate) fn plugin_presets_listing(&self) -> Vec<String> {
        Vec::new()
    }
    pub(crate) fn list_plugin_presets(&mut self, _id: &str) -> Result<(), String> {
        Err("this build has no plugin host".into())
    }
    pub(crate) fn save_part_as_preset(&mut self, _ch: u8, _name: &str, _category: crate::api::PatchCategory, _overwrite: bool) -> Result<(), String> {
        Err("this build has no plugin host".into())
    }
    pub(crate) fn export_state_as_preset(&mut self, _component_id: &str, _name: &str, _state: &[u8], _overwrite: bool) -> Result<std::path::PathBuf, String> {
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
                if let Err(e) = self.assign_channel_plugin(ch(part), PluginVoice { id, state, preset: None, sound: None }) {
                    return self.fail(e);
                }
            }
            PluginCmd::SetPartPluginPreset { part, id, preset } => {
                let voice = match self.preset_voice(&id, &preset) {
                    Ok(v) => v,
                    Err(e) => return self.fail(e),
                };
                self.sound_library_part_plugin(part as usize, true);
                if let Err(e) = self.assign_channel_plugin(ch(part), voice) {
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
            PluginCmd::SetPluginInProcess { id, in_process } => {
                if let Err(e) = self.set_plugin_in_process(&id, in_process) {
                    return self.fail(e);
                }
            }
            PluginCmd::ReloadPartPlugin { part } => {
                let part = part.map_or_else(|| self.shared.parts.selected(), |p| (p & 3) as usize);
                if let Err(e) = self.reload_part_plugin(part) {
                    return self.fail(e);
                }
            }
        }
        Ok(())
    }

    /// The selected part's plugin stopped working or failed to load: the Launchkey's reload
    /// button (Panel fader button 6) lights.
    pub(super) fn selected_plugin_fault(&self) -> bool {
        let ch = parts::CHANNEL[self.shared.parts.selected() & 3];
        self.channel_plugin_state(ch).is_some_and(|p| matches!(p.status, PluginStatus::Muted | PluginStatus::Failed))
    }

    /// Load part `part`'s plugin again with its saved voice (id and preset), if it stopped
    /// working or failed to load. `reloadPartPlugin`.
    fn reload_part_plugin(&mut self, part: usize) -> Result<(), String> {
        let name = parts::NAMES[part];
        let Some(p) = self.channel_plugin_state(parts::CHANNEL[part]) else {
            return Err(format!("{name} plays its SoundFont voice; there is no plugin to reload"));
        };
        match p.status {
            PluginStatus::Muted | PluginStatus::Failed => {}
            PluginStatus::Playing => return Err(format!("{name}'s {} is playing; nothing to reload", p.name)),
            PluginStatus::Loading => return Err(format!("{name}'s {} is still loading", p.name)),
        }
        #[cfg(feature = "plugins")]
        {
            let voice = self.plugins.channels[parts::CHANNEL[part] as usize].as_ref().map(|c| c.voice.clone()).unwrap_or_default();
            self.assign_channel_plugin(parts::CHANNEL[part], voice)?;
            self.say(format!("{name}: loading {} again", p.name), false);
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

#[cfg(feature = "plugins")]
#[path = "plugin_pool.rs"]
pub(crate) mod pool;

#[cfg(all(test, feature = "plugins"))]
#[path = "plugins_tests.rs"]
mod tests;
