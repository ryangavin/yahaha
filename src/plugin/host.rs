//! `PluginHost`: scan (cached), and load instruments off the audio thread.
//!
//! Loads run on their own thread each. A load has a deadline: if the plugin hangs (in its
//! constructor, `Initialize`, or restoring a preset), the handle reports
//! [`LoadProgress::TimedOut`] and the caller moves on. The hung thread cannot be killed
//! (the plugin owns it); it is abandoned, and if the plugin ever finishes, the instance is
//! disposed of right there. The timeout is recorded in the scan cache, so the browser can
//! flag the plugin next time.

use anyhow::{Result, anyhow, bail};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::instance::{LoadTimes, PluginInstance};
use super::scan::{self, LoadRecord, PluginFormat, PluginId, PluginInfo, ScanCache};
use super::sys::{self, Component};

/// Where to run the plugin.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LoadMode {
    /// AUv2 in process, AUv3 out of process.
    #[default]
    Auto,
    /// In process: lowest render overhead; a crash takes yahaha down. AUv3 only if the
    /// extension allows it.
    InProcess,
    /// In the system's AUHostingService: a crash or hang there only silences the part. Works
    /// for AUv2 too (macOS 11+), at an IPC cost per render.
    OutOfProcess,
}

/// How to load.
#[derive(Clone, Debug)]
pub struct LoadConfig {
    pub sample_rate: f64,
    /// The largest block `render` will be asked for. MIDI offsets must be below the next
    /// render's length, so keep it at least the audio device's buffer.
    pub max_frames: u32,
    /// A state from `get_state` to restore before the instance is handed over.
    pub state: Option<Vec<u8>>,
    pub mode: LoadMode,
    /// Give up after this long (default 20 s: Kontakt with a big library takes several).
    pub timeout: Duration,
    /// Choose the mode from the plugin as the load thread looks it up (instead of `mode`):
    /// a caller that does not know the plugin yet (the scan still running, say) never scans
    /// to find out.
    pub choose_mode: Option<fn(&PluginInfo) -> LoadMode>,
}

impl Default for LoadConfig {
    fn default() -> Self {
        LoadConfig { sample_rate: 48_000.0, max_frames: 4096, state: None, mode: LoadMode::Auto, timeout: Duration::from_secs(20), choose_mode: None }
    }
}

/// A load that passed its deadline, as a typed error inside `anyhow`
/// (`e.chain().any(|c| c.is::<LoadTimedOut>())`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadTimedOut(pub Duration);

impl std::fmt::Display for LoadTimedOut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "timed out after {:.1} s", self.0.as_secs_f64())
    }
}

impl std::error::Error for LoadTimedOut {}

/// Where a load is.
#[derive(Clone, Debug, PartialEq)]
pub enum LoadProgress {
    Queued,
    /// Creating the instance (for AUv3: launching its extension process).
    Instantiating,
    /// Setting the format and `AudioUnitInitialize`.
    Initializing,
    RestoringState,
    /// Done: take the instance with [`LoadHandle::take`] / [`LoadHandle::wait`].
    Ready,
    Failed(String),
    /// The deadline passed. The load thread is abandoned.
    TimedOut(Duration),
}

impl LoadProgress {
    pub fn is_finished(&self) -> bool {
        matches!(self, LoadProgress::Ready | LoadProgress::Failed(_) | LoadProgress::TimedOut(_))
    }
}

struct LoadState {
    progress: LoadProgress,
    result: Option<Result<PluginInstance>>,
    /// The caller gave up (timeout or cancel): the worker disposes of whatever it makes.
    abandoned: bool,
    /// The plugin and the mode it loads in, once the load thread has looked it up.
    info: Option<(PluginInfo, LoadMode)>,
}

struct LoadShared {
    state: Mutex<LoadState>,
    done: Condvar,
}

/// A load in flight. Poll it ([`LoadHandle::progress`], [`LoadHandle::take`]) from a UI or
/// control loop, or block on it ([`LoadHandle::wait`]). Dropping it cancels the load.
pub struct LoadHandle {
    shared: Arc<LoadShared>,
    started: Instant,
    timeout: Duration,
    host: PluginHost,
    id: PluginId,
    taken: bool,
}

impl LoadHandle {
    /// The plugin loading, once the load thread has looked it up (None before that, or if
    /// it is not installed).
    pub fn info(&self) -> Option<PluginInfo> {
        self.shared.state.lock().unwrap().info.as_ref().map(|(i, _)| i.clone())
    }

    /// The mode it loads in (`LoadConfig::mode`, or what `choose_mode` chose), once looked up.
    pub fn mode(&self) -> Option<LoadMode> {
        self.shared.state.lock().unwrap().info.as_ref().map(|(_, m)| *m)
    }

    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    /// Where the load is now. Flips to `TimedOut` once the deadline has passed.
    pub fn progress(&self) -> LoadProgress {
        let mut st = self.shared.state.lock().unwrap();
        self.check_deadline(&mut st);
        st.progress.clone()
    }

    fn check_deadline(&self, st: &mut LoadState) {
        if !st.progress.is_finished() && self.started.elapsed() >= self.timeout {
            st.progress = LoadProgress::TimedOut(self.timeout);
            st.abandoned = true;
            if let Some((info, _)) = &st.info {
                self.host.record(info, None, Some(format!("timed out after {:.1} s", self.timeout.as_secs_f64())));
            }
        }
    }

    /// The instance, if the load has finished (`Some(Err)` when it failed or timed out).
    pub fn take(&mut self) -> Option<Result<PluginInstance>> {
        let mut st = self.shared.state.lock().unwrap();
        self.check_deadline(&mut st);
        match &st.progress {
            LoadProgress::TimedOut(d) => {
                self.taken = true;
                let name = st.info.as_ref().map_or_else(|| self.id.to_string(), |(i, _)| i.full_name());
                Some(Err(anyhow::Error::new(LoadTimedOut(*d)).context(format!("{name} did not load"))))
            }
            p if p.is_finished() => {
                self.taken = true;
                st.result.take()
            }
            _ => None,
        }
    }

    /// Block until the load finishes or times out.
    pub fn wait(mut self) -> Result<PluginInstance> {
        {
            let mut st = self.shared.state.lock().unwrap();
            while !st.progress.is_finished() {
                let left = self.timeout.saturating_sub(self.started.elapsed());
                if left.is_zero() {
                    break;
                }
                st = self.shared.done.wait_timeout(st, left).unwrap().0;
            }
        }
        self.take().unwrap_or_else(|| Err(anyhow!("load did not finish")))
    }
}

impl Drop for LoadHandle {
    fn drop(&mut self) {
        if !self.taken {
            let mut st = self.shared.state.lock().unwrap();
            st.abandoned = true;
            // Drop a finished-but-untaken instance here (off the audio thread by construction).
            st.result = None;
        }
    }
}

struct Inner {
    cache_path: Option<PathBuf>,
    cache: Mutex<Option<ScanCache>>,
}

/// The plugin host: scanning and loading. Cheap to clone (one `Arc`); share it between the
/// Session's control thread and the app.
#[derive(Clone)]
pub struct PluginHost {
    inner: Arc<Inner>,
}

impl PluginHost {
    /// A host whose scan is cached at `cache_path` (None: no cache, always scan).
    pub fn new(cache_path: Option<PathBuf>) -> PluginHost {
        PluginHost { inner: Arc::new(Inner { cache_path, cache: Mutex::new(None) }) }
    }

    /// A host with its cache in `~/Library/Caches/yahaha/plugins.json`.
    pub fn with_default_cache() -> PluginHost {
        PluginHost::new(scan::default_cache_path())
    }

    /// Every installed instrument Audio Unit (AUv2 and AUv3), sorted by vendor and name.
    /// Served from the cache when nothing was installed, removed or updated since it was
    /// written; otherwise rescanned and the cache rewritten (keeping each plugin's last load
    /// record while its version is unchanged).
    pub fn scan(&self) -> Result<Vec<PluginInfo>> {
        let comps = sys::instruments();
        let fp = scan::fingerprint(&comps);
        let mut cache = self.inner.cache.lock().unwrap();
        if cache.is_none() {
            *cache = self.inner.cache_path.as_deref().and_then(scan::read_cache);
        }
        if let Some(c) = cache.as_ref().filter(|c| c.fingerprint == fp) {
            return Ok(c.plugins.clone());
        }
        let mut plugins = scan::scan_live(&comps);
        if let Some(old) = cache.as_ref() {
            for p in &mut plugins {
                p.last_load = old.plugins.iter().find(|o| o.id == p.id && o.version == p.version).and_then(|o| o.last_load.clone());
                // The player's choice outlives an update.
                p.in_process = old.plugins.iter().any(|o| o.id == p.id && o.in_process) && p.can_run_in_process();
            }
        }
        let fresh = ScanCache { schema: scan::SCHEMA, fingerprint: fp, plugins: plugins.clone() };
        if let Some(path) = &self.inner.cache_path {
            scan::write_cache(path, &fresh)?;
        }
        *cache = Some(fresh);
        Ok(plugins)
    }

    /// Scan ignoring the cache (and rewrite it).
    pub fn rescan(&self) -> Result<Vec<PluginInfo>> {
        {
            let mut cache = self.inner.cache.lock().unwrap();
            if cache.is_none() {
                *cache = self.inner.cache_path.as_deref().and_then(scan::read_cache);
            }
            // Keep the load records; only force the fingerprint to miss.
            if let Some(c) = cache.as_mut() {
                c.fingerprint = 0;
            }
        }
        self.scan()
    }

    /// Where the scan cache lives, if anywhere.
    pub fn cache_path(&self) -> Option<&std::path::Path> {
        self.inner.cache_path.as_deref()
    }

    /// The instrument with this id, if installed.
    pub fn info(&self, id: &PluginId) -> Result<PluginInfo> {
        self.scan()?.into_iter().find(|p| p.id == *id).ok_or_else(|| anyhow!("no instrument Audio Unit {id} is installed"))
    }

    /// The first instrument whose id is `query` ("aumu Xf2X XFER") or whose name contains it
    /// (case-insensitive).
    pub fn find(&self, query: &str) -> Result<PluginInfo> {
        let all = self.scan()?;
        if let Some(id) = PluginId::parse(query)
            && let Some(p) = all.iter().find(|p| p.id == id)
        {
            return Ok(p.clone());
        }
        let q = query.to_lowercase();
        all.iter()
            .find(|p| p.full_name().to_lowercase().contains(&q))
            .cloned()
            .ok_or_else(|| anyhow!("no instrument Audio Unit matches {query:?}"))
    }

    /// Set the player's "run in process" override for `id` and save it in the cache.
    /// Returns the plugin as now cached. Err for an AUv3 that only runs out of process.
    pub fn set_in_process(&self, id: &PluginId, on: bool) -> Result<PluginInfo> {
        let info = self.info(id)?;
        if on && !info.can_run_in_process() {
            bail!("{} is an AUv3 that only runs out of process", info.full_name());
        }
        let mut cache = self.inner.cache.lock().unwrap();
        let c = cache.as_mut().ok_or_else(|| anyhow!("no plugin scan"))?;
        let p = c.plugins.iter_mut().find(|p| p.id == *id).ok_or_else(|| anyhow!("no instrument Audio Unit {id} is installed"))?;
        p.in_process = on;
        let out = p.clone();
        if let Some(path) = &self.inner.cache_path {
            scan::write_cache(path, c)?;
        }
        Ok(out)
    }

    fn record(&self, info: &PluginInfo, ms: Option<f64>, error: Option<String>) {
        let at = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let mut cache = self.inner.cache.lock().unwrap();
        let Some(c) = cache.as_mut() else { return };
        let Some(p) = c.plugins.iter_mut().find(|p| p.id == info.id && p.version == info.version) else { return };
        p.last_load = Some(LoadRecord { at, ms, error });
        if let Some(path) = &self.inner.cache_path {
            let _ = scan::write_cache(path, c);
        }
    }

    /// Start loading `id` on a background thread. Returns at once; follow the handle. The
    /// plugin is looked up on that thread too (a stale scan cache means a full component
    /// scan), so the caller never waits for a scan: an id that is not installed fails
    /// through the handle.
    pub fn load_async(&self, id: &PluginId, cfg: LoadConfig) -> Result<LoadHandle> {
        let shared = Arc::new(LoadShared {
            state: Mutex::new(LoadState { progress: LoadProgress::Queued, result: None, abandoned: false, info: None }),
            done: Condvar::new(),
        });
        let id = *id;
        let handle = LoadHandle { shared: shared.clone(), started: Instant::now(), timeout: cfg.timeout, host: self.clone(), id, taken: false };
        let host = self.clone();
        std::thread::Builder::new()
            .name(format!("plugin-load {id}"))
            .spawn(move || {
                let found = host.info(&id).and_then(|info| {
                    let comp = sys::instruments()
                        .into_iter()
                        .find(|c| c.desc == [id.kind, id.subtype, id.manufacturer])
                        .ok_or_else(|| anyhow!("{id} disappeared from the registrar"))?;
                    Ok((info, comp))
                });
                let (info, comp) = match found {
                    Ok(x) => x,
                    Err(e) => return finish(&shared, Err(e)),
                };
                let mode = cfg.choose_mode.map_or(cfg.mode, |f| f(&info));
                shared.state.lock().unwrap().info = Some((info.clone(), mode));
                let r = sys::guard("loading", || load_blocking(&comp, &info, mode, &cfg, &shared));
                match &r {
                    Ok(inst) => host.record(&info, Some(inst.load_times().total().as_secs_f64() * 1000.0), None),
                    Err(e) if !shared.state.lock().unwrap().abandoned => host.record(&info, None, Some(format!("{e:#}"))),
                    Err(_) => {}
                }
                finish(&shared, r);
            })
            .map_err(|e| anyhow!("could not start the load thread: {e}"))?;
        Ok(handle)
    }

    /// Load and wait (up to the timeout). For tools and tests; the app uses `load_async`.
    pub fn load(&self, id: &PluginId, cfg: LoadConfig) -> Result<PluginInstance> {
        self.load_async(id, cfg)?.wait()
    }
}

/// Hand the load's result to the handle (or, if the caller gave up, dispose of it here).
fn finish(shared: &LoadShared, r: Result<PluginInstance>) {
    let mut st = shared.state.lock().unwrap();
    if st.abandoned {
        // Timed out or cancelled: the instance (if any) is disposed of here.
        drop(st);
        drop(r);
        return;
    }
    st.progress = match &r {
        Ok(_) => LoadProgress::Ready,
        Err(e) => LoadProgress::Failed(format!("{e:#}")),
    };
    st.result = Some(r);
    shared.done.notify_all();
}

fn set_progress(shared: &LoadShared, p: LoadProgress) -> Result<()> {
    let mut st = shared.state.lock().unwrap();
    if st.abandoned {
        bail!("abandoned");
    }
    st.progress = p;
    Ok(())
}

/// The load itself, on the load thread.
fn load_blocking(comp: &Component, info: &PluginInfo, mode: LoadMode, cfg: &LoadConfig, shared: &LoadShared) -> Result<PluginInstance> {
    let out_of_process = match mode {
        LoadMode::Auto => info.format == PluginFormat::Au3,
        LoadMode::InProcess => {
            if info.format == PluginFormat::Au3 && !info.can_load_in_process {
                bail!("{} is an AUv3 that only runs out of process", info.full_name());
            }
            false
        }
        LoadMode::OutOfProcess => true,
    };
    set_progress(shared, LoadProgress::Instantiating)?;
    let t0 = Instant::now();
    let mut unit = if out_of_process || info.requires_async || info.format == PluginFormat::Au3 {
        let rx = sys::instantiate_async(comp, out_of_process);
        let left = cfg.timeout.saturating_sub(t0.elapsed());
        rx.recv_timeout(left).map_err(|_| anyhow::Error::new(LoadTimedOut(cfg.timeout)).context("instantiation did not complete"))??
    } else {
        sys::instantiate_sync(comp)?
    };
    let instantiate = t0.elapsed();

    set_progress(shared, LoadProgress::Initializing)?;
    let t1 = Instant::now();
    unit.configure_and_initialize(cfg.sample_rate, cfg.max_frames)?;
    let initialize = t1.elapsed();

    let mut times = LoadTimes { instantiate, initialize, restore: Duration::ZERO };
    let mut inst = PluginInstance::new(unit, info.clone(), cfg.sample_rate, cfg.max_frames, out_of_process, times);
    if let Some(state) = &cfg.state {
        set_progress(shared, LoadProgress::RestoringState)?;
        let t2 = Instant::now();
        inst.set_state(state)?;
        times.restore = t2.elapsed();
        inst = inst.with_load_times(times);
    }
    inst.prime();
    Ok(inst)
}
