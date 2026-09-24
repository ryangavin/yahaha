p='src/plugin/host.rs'
s=open(p).read()
def rep(old,new):
    global s
    assert old in s, old
    s=s.replace(old,new,1)
rep("""struct LoadState {
    progress: LoadProgress,
    result: Option<Result<PluginInstance>>,
    /// The caller gave up (timeout or cancel): the worker disposes of whatever it makes.
    abandoned: bool,
}""","""struct LoadState {
    progress: LoadProgress,
    result: Option<Result<PluginInstance>>,
    /// The caller gave up (timeout or cancel): the worker disposes of whatever it makes.
    abandoned: bool,
    /// The plugin, once the load thread has looked it up.
    info: Option<PluginInfo>,
}""")
rep("""    host: PluginHost,
    info: PluginInfo,
    taken: bool,
}

impl LoadHandle {
    pub fn info(&self) -> &PluginInfo {
        &self.info
    }
""","""    host: PluginHost,
    id: PluginId,
    taken: bool,
}

impl LoadHandle {
    /// The plugin loading, once the load thread has looked it up (None before, or if it is
    /// not installed).
    pub fn info(&self) -> Option<PluginInfo> {
        self.shared.state.lock().unwrap().info.clone()
    }
""")
rep("""            st.abandoned = true;
            self.host.record(&self.info, None, Some(format!("timed out after {:.1} s", self.timeout.as_secs_f64())));""","""            st.abandoned = true;
            if let Some(info) = &st.info {
                self.host.record(info, None, Some(format!("timed out after {:.1} s", self.timeout.as_secs_f64())));
            }""")
rep("""                Some(Err(anyhow::Error::new(LoadTimedOut(*d)).context(format!("{} did not load", self.info.full_name()))))""","""                let name = st.info.as_ref().map_or_else(|| self.id.to_string(), |i| i.full_name());
                Some(Err(anyhow::Error::new(LoadTimedOut(*d)).context(format!("{name} did not load"))))""")
old=s[s.index("    /// Start loading `id` on a background thread."):s.index("    /// Load and wait (up to the timeout).")]
new='''    /// Start loading `id` on a background thread. Returns at once; follow the handle. The
    /// plugin is looked up on that thread too (a stale scan cache means a full scan), so the
    /// caller never waits for a scan: an id that is not installed fails through the handle.
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
                shared.state.lock().unwrap().info = Some(info.clone());
                let r = sys::guard("loading", || load_blocking(&comp, &info, &cfg, &shared));
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

'''
s=s.replace(old,new)
rep("""fn set_progress(shared: &LoadShared, p: LoadProgress) -> Result<()> {""","""/// Hand the load's result to the handle (or, if the caller gave up, dispose of it here).
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

fn set_progress(shared: &LoadShared, p: LoadProgress) -> Result<()> {""")
open(p,'w').write(s)

p='src/plugin/tests.rs'
s=open(p).read()
rep("""    let bogus = PluginId::parse("aumu zzzz zzzz").unwrap();
    assert!(host().load_async(&bogus, LoadConfig::default()).is_err());""","""    let bogus = PluginId::parse("aumu zzzz zzzz").unwrap();
    // Looked up on the load thread (the caller never waits for a scan): the handle fails.
    let h = host().load_async(&bogus, LoadConfig::default()).unwrap();
    let err = h.wait().err().expect("not installed");
    assert!(format!("{err:#}").contains("no instrument Audio Unit"), "{err:#}");""")
open(p,'w').write(s)

p='src/session/plugins.rs'
s=open(p).read()
rep("""            let id = PluginId::parse(&voice.id).ok_or_else(|| format!("{:?} is not a plugin id", voice.id))?;
            let host = self.plugins.host();
            let info = host.info(&id).map_err(|e| format!("{e:#}"))?;
            let mode = if id.manufacturer == APPLE || info.format == PluginFormat::Au3 { LoadMode::Auto } else { LoadMode::OutOfProcess };""","""            let id = PluginId::parse(&voice.id).ok_or_else(|| format!("{:?} is not a plugin id", voice.id))?;
            // Checked against the scanned list, never by scanning here (a stale cache means a
            // full component scan, under the Session lock). Before the first scan is in, the
            // load thread looks the plugin up and fails the load if it is not installed.
            let info = self.plugins.list.iter().find(|p| p.id == id).cloned();
            if info.is_none() && self.plugins.scan_rx.is_none() && !self.plugins.list.is_empty() {
                return Err(format!("no instrument Audio Unit {id} is installed (rescan the plugins if it was just installed)"));
            }
            let host = self.plugins.host();
            let au3 = info.as_ref().is_some_and(|i| i.format == PluginFormat::Au3);
            let mode = if id.manufacturer == APPLE || au3 { LoadMode::Auto } else { LoadMode::OutOfProcess };""")
rep("""                info: Some(info),
                load: Some(load),""","""                info,
                load: Some(load),""")
rep("""            let Some(load) = c.load.as_mut() else { return };
            let res = match load.take() {""","""            let Some(load) = c.load.as_mut() else { return };
            if c.info.is_none() {
                c.info = load.info();
            }
            let res = match load.take() {""")
open(p,'w').write(s)

p='src/session/plugins_tests.rs'
s=open(p).read()
rep("""    assert!(s.state().plugins.available);
    assert!(s.send(PluginCmd::SetPartPlugin { part: 0, id: "aumu nope nope".into(), state: None }).is_err(), "not installed");
    s.send(PluginCmd::SetPartPlugin { part: 0, id: DLS.into(), state: None }).unwrap();""","""    assert!(s.state().plugins.available);
    wait_scanned(&s);
    assert!(s.send(PluginCmd::SetPartPlugin { part: 0, id: "aumu nope nope".into(), state: None }).is_err(), "not installed");
    s.send(PluginCmd::SetPartPlugin { part: 0, id: DLS.into(), state: None }).unwrap();""")
rep("""/// Pump until Right 1's plugin has finished loading""","""/// Scan the plugins (the scan cache, on the `plugin-scan` thread) and pump until the list
/// is in.
fn wait_scanned(s: &Session) {
    s.inner.lock().start_plugin_scan(false);
    let t0 = Instant::now();
    while s.state().plugins.scanning || s.state().plugins.list.is_empty() {
        assert!(t0.elapsed() < Duration::from_secs(60), "the plugin scan did not finish");
        s.advance(1_000_000);
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Picking a plugin never scans on the control thread (#104 review item 6). Before the
/// first scan is in, the load thread looks the id up: an unknown one fails there, named by
/// its id, and DLS loads (and gets its name) as usual. Once the list is in, an unknown id
/// is refused at once from it.
#[test]
fn a_plugin_is_looked_up_off_the_control_thread() {
    let Some(s) = session() else { return };
    s.offline_audio(None, 48_000).unwrap();
    assert!(s.inner.lock().plugins.list.is_empty(), "no scan yet");
    s.send(PluginCmd::SetPartPlugin { part: 0, id: "aumu nope nope".into(), state: None }).unwrap();
    assert_eq!(wait_playing(&s, 0), PluginStatus::Failed);
    let p = s.state().keyboard_parts[0].plugin.clone().unwrap();
    assert!(p.name == "aumu nope nope" && p.error.as_deref().is_some_and(|e| e.contains("no instrument Audio Unit")), "{p:?}");
    s.send(PluginCmd::SetPartPlugin { part: 1, id: DLS.into(), state: None }).unwrap();
    assert_eq!(wait_playing(&s, 1), PluginStatus::Playing);
    assert_eq!(s.state().keyboard_parts[1].plugin.clone().unwrap().name, "DLSMusicDevice");
    wait_scanned(&s);
    assert!(s.send(PluginCmd::SetPartPlugin { part: 2, id: "aumu nope nope".into(), state: None }).is_err(), "refused from the list");
}

/// Pump until Right 1's plugin has finished loading""")
open(p,'w').write(s)

s=open(p).read()
rep("""    s.inner.lock().restore_saved(saved);
    s.advance(1_000_000);
    let p = s.state().keyboard_parts[2].plugin.clone().unwrap();""","""    s.inner.lock().restore_saved(saved);
    assert_eq!(wait_playing(&s, 2), PluginStatus::Failed);
    let p = s.state().keyboard_parts[2].plugin.clone().unwrap();""")
rep("""    assert_eq!(wait_playing(&s, 3), PluginStatus::Playing);
    let p = s.state().keyboard_parts[0].plugin.clone().unwrap();""","""    assert_eq!(wait_playing(&s, 3), PluginStatus::Playing);
    assert_eq!(wait_playing(&s, 0), PluginStatus::Failed);
    let p = s.state().keyboard_parts[0].plugin.clone().unwrap();""")
open(p,'w').write(s)
