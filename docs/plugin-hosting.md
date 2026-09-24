# Plugin hosting (#35)

Goal: Genos-class sounds for the keyboard parts (Right 1-3, Left), and optionally for the
Style parts, played through the user's own instrument plugins instead of the built-in
SoundFont synth (rustysynth, `src/synth.rs`).

Status:

- **Spike** (PR #79): options report (below) and an Audio Unit prototype. Recommendation:
  Audio Units first, through the objc2 bindings.
- **Phase 1** (this document's first half): the library layer in `src/plugin/`, behind
  the `plugins` cargo feature, off by default. Scan (cached), async load with timeout
  (AUv2 and AUv3, in or out of process), instances with sample-accurate MIDI and state as
  bytes, a real-time rack that applies the part's CC7/CC11 and swaps instances glitch-free,
  editor windows, and `yahaha plugin-test` to measure it all. Nothing in the engine,
  session, API or built-in synth uses it yet, and the default build is unchanged.
- **Phase 2** (plan at the end): wiring it into the Session, the synth callback, the app
  and Registration Memory.

## Phase 1: the library

```sh
cargo test --features plugins plugin                        # unit tests (Apple's DLSMusicDevice)
cargo test --features plugins --test plugin_rack_no_alloc   # the rack's audio path never allocates
cargo build --release --features plugins
./target/release/yahaha plugin-test --list                  # installed instruments (cached scan)
./target/release/yahaha plugin-test "Serum 2" --bench       # load, state, CPU, swap latency; no audio device
./target/release/yahaha plugin-test "Serum 2" --swap-to FM8 # play the phrase, swap to a preloaded FM8 at bar 3
./target/release/yahaha plugin-test "Kontakt 8" --oop --gui # out of process, with its editor
```

Other options: `--rescan`, `--oop` (load out of process), `--timeout S`, `--channel N`,
`--sf2 file | --no-sf2`, `--loops N`, `--seconds N`, `--audio-out N`.

### API

All of it is `yahaha::plugin::*`.

**`PluginHost`** (cheap to clone; share between the control thread and the app)

| Call | What it does |
|---|---|
| `PluginHost::new(cache_path)`, `with_default_cache()` | Cache in `~/Library/Caches/yahaha/plugins.json` by default. |
| `scan() -> Vec<PluginInfo>` | Every instrument AU (AUv2 and AUv3), sorted by vendor and name. Served from the cache while a fingerprint of every component's (type, subtype, manufacturer, flags, version) matches; any install, removal or update rescans. 22 ms live, 0.01 ms cached on this Mac (14 instruments). |
| `rescan()` | Ignore the cache (keeps the per-plugin load records). |
| `find(query)`, `info(&id)` | By id ("aumu Xf2X XFER") or name substring. |
| `load_async(&id, LoadConfig) -> LoadHandle` | Loads on its own thread. `LoadConfig { sample_rate, max_frames, state, mode, timeout }`; `mode` is `Auto` (AUv2 in process, AUv3 out of process), `InProcess` or `OutOfProcess`. |
| `load(&id, cfg)` | `load_async` + `wait`, for tools and tests. |

`PluginInfo { id: PluginId, name, manufacturer, version, format: AUv2 / AUv3, requires_async,
can_load_in_process, sandbox_safe, last_load: Option<LoadRecord> }`. `PluginId` is the
(type, subtype, manufacturer) triple; it serialises as three numbers and prints and parses as
"aumu Xf2X XFER". `last_load` is the plugin's last load time or its error ("timed out after
20.0 s"), so the browser can flag a bad plugin before the user picks it.

**`LoadHandle`**: `progress()` (`Queued`, `Instantiating`, `Initializing`,
`RestoringState`, `Ready`, `Failed(msg)`, `TimedOut(d)`), `take() -> Option<Result<PluginInstance>>`
(non-blocking), `wait()`, `elapsed()`. Dropping it cancels. A load that passes its deadline
reports `TimedOut`; its thread (stuck inside the plugin, which cannot be killed) is
abandoned and disposes of the instance itself if the plugin ever returns. The load also
renders a little silence (`prime`) so a plugin's first-render set-up happens off the audio
thread, and runs inside an Objective-C `@try` so an `NSException` fails the load instead of
aborting yahaha.

**`PluginInstance`** (`Send`; one per loaded plugin)

| Call | Thread |
|---|---|
| `midi([u8; 3], offset)` | Audio. One `MusicDeviceMIDIEvent`; `offset` is the frame in the next render (sample-accurate, tested). |
| `render(&mut left, &mut right) -> Result<(), RenderError>` | Audio. Any length; sliced to `max_frames`. The `AudioBufferList` is on the stack. An OSStatus or NaN / infinity zeroes the output and returns the error. |
| `stats() -> Arc<PluginStats>` | Any. Relaxed atomics written by `render`: blocks, mean / max / last ns, overruns (render > `budget_permille` of the block, default 50%), deadline misses (> 100%), errors. `snapshot(rate)` gives µs and CPU share. |
| `get_state() -> Vec<u8>`, `set_state(&[u8])` | Control thread, not the main thread (below), instance not playing. `kAudioUnitProperty_ClassInfo` as a binary plist: the plugin's full preset. |
| `reconfigure(rate, max_frames)` | Control; for device changes. |
| `latency_seconds()`, `load_times()`, `out_of_process()`, `info()` | Any. |
| `editor_target() -> EditorTarget` | Any; `Send`. Hand it to the main thread for the editor. |

**`PluginRack` / `RackControl`** (`rack(max_block, sample_rate)` makes the pair; all memory
is allocated there)

- One slot per MIDI channel, so it fits the synth callback as it is: keyboard parts are
  channels 0-3 (`parts::CHANNEL`), Style parts 8-15.
- Audio thread, per callback: `begin_block()` (apply assigns / clears), `midi(m, offset) ->
  bool` for each message (true = a plugin slot took it; otherwise send it to rustysynth as
  now), `render_add(&mut l, &mut r)` (mixes every slot in, before master gain and the soft
  clipper). None of these allocate or lock (`tests/plugin_rack_no_alloc.rs` counts).
- Control thread: `assign(channel, instance, Swap { fade_frames, trim })`, `clear(channel,
  fade)`, `poll()` (events, and drops retired instances here), `poll_events()` /
  `take_retired()` (to keep a warm pool). Events: `Swapped { channel, latency }`,
  `Cleared`, `Fault { channel, error }`, `Overrun { channel, render_us, block_us }` (at most
  one per slot per second).
- **Mixer rule.** The rack keeps CC7 / CC11 (and their LSBs, CC39 / CC43) and applies
  `((vol14/16383)·(expr14/16383))²` to the plugin's output, ramped over the block: the exact
  curve and 14-bit arithmetic rustysynth uses (`voice.rs`: `ve * ve`), power-on 100 / 127,
  CC121 resets expression. The plugin never sees CC7 or CC11, so a fader at 100 is the same
  level on either engine, whatever the plugin would have done with CC7. Velocity still goes
  to the plugin. `Swap::trim` is a per-voice linear trim for level normalisation.
- **Swaps.** An assign takes effect at the next block boundary. The part's controllers
  (everything but volume, bank select, RPN/NRPN and channel mode messages) and pitch bend
  are replayed into the incoming instance, so modulation, pan, sustain and bend carry over.
  The outgoing instance gets Sustain off + All Notes Off and keeps rendering while it fades
  out over `fade_frames` (default 240 = 5 ms) as the new one fades in; then it goes back to
  the control side, never disposed of on the audio thread. The CC7/CC11 gain belongs to the
  slot, so it carries over. `fade_frames: 0` hands over at the boundary with no fade.
- **Faults.** A render error or non-finite output mutes the slot: it keeps owning its
  channel (the part goes quiet instead of jumping to the SoundFont), reports `Fault` once,
  and renders nothing until the control side assigns or clears. A MIDI error (an
  out-of-process plugin whose process died fails there first) does the same.

**Editor** (`plugin::editor`, main thread only, `MainThreadMarker`)

- `open_editor(mtm, &EditorTarget) -> Editor` picks the plugin's AUv2 Cocoa view (in
  process), else its view controller (`kAudioUnitProperty_RequestViewController`: AUv3, and
  every out-of-process unit, as a remote view), else CoreAudioKit's `AUGenericView`.
  `Editor::{kind, size, is_open, focus, close}`; `close_editor(editor)`; dropping closes.
  The editor holds its own reference to the Audio Unit, so swapping the part out while its
  window is open is safe.
- The desktop app owns `NSApplication`, so it only calls these from its main thread
  (`app.run_on_main_thread`). Tools without an app call `prepare_app(mtm)` once and
  `pump_events(mtm, dur)` in their loop, and `run_main_loop(dur)` while they wait for loads.

**Code map**: `sys.rs` is the thin unsafe core over `objc2-audio-toolbox` /
`objc2-core-foundation` (instance ownership, render over a stack `AudioBufferList`, MIDI,
ClassInfo state, sync and async instantiation, silent input callbacks, the `@try` guard);
`scan.rs` the ids, infos and cache; `host.rs` `PluginHost` and loads; `instance.rs`
`PluginInstance` and stats; `rack.rs` the rack; `editor.rs` the windows (AppKit through
`objc2-app-kit` / `objc2-core-audio-kit`); `cli.rs` `plugin-test`; `tests.rs` the DLS tests.
The spike's hand-written FFI (`ffi.rs`, `au.rs`, `gui.rs`) is gone; the only hand-typed call
left is `uiViewForAudioUnit:withSize:` (a typed `objc_msgSend`, because objc2's encoding check
knows `AudioUnit` under a different SDK name).

Why no `objc2-avf-audio` / `AUAudioUnit`: `AudioComponentInstantiate` already creates AUv3
units (and out-of-process AUv2 units) asynchronously and hands back the same v2 handle, so
one render path (`AudioUnitRender` / `MusicDeviceMIDIEvent`), one state path (ClassInfo)
and one editor path (`RequestViewController`) serve every format. The `AUAudioUnit`
render-block API would be a second implementation of each; it is worth adding only if an
AUv3 instrument turns out to need something the bridge does not give (none is installed
here to test; the out-of-process AUv2 runs exercise the same code).

### Measured (M1 Max, macOS 26.5, 48 kHz)

`yahaha plugin-test <name> --bench`. Render is a six-note chord held for 2 s per block size;
figures are mean / p99 / max render time per block. Swap is 40 preload-then-swap hand-overs
on a real-time (time-constraint) thread rendering 64-frame blocks with a chord held and a
5 ms crossfade; "assign → playing" runs from the `assign()` call to the start of the block
that swapped (half a block on average, at most one).

| | DLSMusicDevice | Serum 2 | FM8 | Kontakt 8 (no library) |
|---|---|---|---|---|
| Load, in process | 9 ms | 119 ms (second instance 64 ms) | 174 ms (second 7 ms) | 0.6-1.3 s (second 36 ms) |
| Load, out of process | 339 ms (second 4 ms) | 423 ms (second 67 ms) | 384 ms (second 12 ms) | 4.5 s (second 45 ms) |
| State size; get / set | 916 B; 28 / 3 ms; byte-identical | 23 KB; 0.8 / 17 ms | 2 KB; 0.3 / 0.4 ms | 4 KB; 0.3 / 326 ms |
| Render 64 frames (1333 µs) | 6.3 / 6.5 / 6.7 µs (0.5%) | 29.7 / 61.5 / 179 µs (2.2%) | 10.9 / 11.9 / 39 µs (0.8%) | 4.9 / 5.7 / 6.5 µs (silent) |
| Render 128 frames | 12.7 / 15.8 / 78 µs | 56.1 / 108 / 284 µs | 23.0 / 68.5 / 145 µs | 5.0 / 6.2 / 7.5 µs |
| Render 256 frames | 26.2 / 74.4 / 132 µs | 106 / 189 / 239 µs | 45.1 / 90.0 / 265 µs | 5.4 / 6.0 / 36 µs |
| Render 64, out of process | 11.9 / 42.9 / 109 µs | 37.5 / 95.2 / 175 µs | 22.6 / 63.6 / 121 µs | 11.2 / 38.7 / 74 µs |
| `assign()` call | 3.3 µs | 6.6 µs | 2.9 µs | 2.4 µs |
| Assign → playing, mean / max | 0.89 / 1.34 ms | 0.75 / 1.33 ms | 0.86 / 1.33 ms | 0.80 / 1.33 ms |
| Rack block, steady (mean / p99 / max) | 23 / 88 / 150 µs | 64 / 379 / **1607** µs | 20 / 66 / 93 µs | 12 / 26 / 33 µs |
| Rack block, swap + crossfade | 42 / 150 / 153 µs | 99 / 375 / 452 µs | 23 / 68 / 98 µs | 22 / 48 / 208 µs |

Live (`plugin-test "Serum 2" --swap-to FM8 --loops 2`, Model 16 at 64 frames, rustysynth
backing in the same callback): two swaps at 0.77 ms mean / 0.93 ms max from assign to
playing; Serum 1.4% and FM8 0.6% of real time; no overruns, deadline misses or errors.

### Findings

- **Preload, then swap.** Loads take 9 ms to 4.5 s; a swap of a preloaded instance costs the
  control thread 3-7 µs and reaches the speakers within one block. So recall must never
  load: preload, then assign. A second instance of a plugin already loaded is much faster
  (Serum 64 ms, FM8 7 ms, Kontakt 36 ms), which makes a small warm pool cheap.
- **Out of process is cheap to render and it isolates crashes.** AUv2 units load fine in the
  AUHostingService (`LoadMode::OutOfProcess`, macOS 11+). The IPC adds about 5-12 µs per
  64-frame block and 0.2-0.3 s to the first load of a plugin (then the service is warm).
  Kontakt 8 shows why it matters: in process, a Kontakt background thread (`ProductScan`)
  crashed yahaha with SIGSEGV when the tool exited; out of process it loaded, rendered, saved
  state and opened its editor (a remote view controller, 1010x647) without trouble. The
  spike's Kontakt editor crash was in process; out of process, the editor opens from the
  CLI. Recommendation for phase 2: out of process by default for third-party
  AUv2 instruments, with in process as a per-plugin option for the lightest ones.
- **Kontakt deadlocks restoring state on the main thread.** Its `ClassInfo` setter waits for
  work it queues on the main thread. The Session's control thread (not main) must do all
  `get_state` / `set_state` calls while the main thread runs its loop, which the app does
  anyway; `plugin-test` does the same (`off_main`, `run_main_loop`).
- **Instruments with audio inputs fail out of process** unless the host feeds the inputs:
  FM8 (it has an FM input bus) rendered `NoConnection` (-10876) until `sys.rs` installed a
  silent input callback on every input bus. In process FM8 did not care.
- **Serum 2 misses a deadline now and then on note-on**: one 64-frame block of 1.6 ms among
  1446 while five notes started together, and never in the live run with the phrase. The
  rack's stats and `Overrun` events make this visible per plugin; at 64 frames a user with
  Serum parts may want 128.
- **State is bytes, but not canonical bytes.** Serum and Kontakt re-save a restored state a
  few bytes longer (timestamps and the like), so compare states by restoring them, not by
  bytes. DLS round-trips byte-identical.
- **CC7.** Unchanged from the spike: plugins disagree about CC7, so the rack owns it.

## The spike's options report

### 1. Audio Units (AUv2 and AUv3) through AudioToolbox / AVFoundation

| | |
|---|---|
| Maturity | Apple's native format since 10.2. AUv3 (app extensions) since 10.11. Every major macOS DAW hosts it. |
| Rust access | **Raw C API** (as in the spike's prototype): `AudioComponentFindNext`, `AudioComponentInstanceNew`, `AudioUnitRender`, `MusicDeviceMIDIEvent`, `kAudioUnitProperty_ClassInfo`. **`objc2-audio-toolbox` / `objc2-avf-audio` / `objc2-core-audio-kit` 0.3.2**: generated from the SDK headers by the objc2 project, very healthy (about 2M recent downloads for audio-toolbox), and they cover `AUAudioUnit`, `AVAudioUnitComponentManager` and `AUGenericView` with typed blocks. `coreaudio-sys` / `coreaudio-rs` (RustAudio) are bindgen'd C APIs, widely used through cpal. **`rack` 0.4** (sinkingsugar) is a young AU+VST3 host built on a C++ shim, with little adoption (about 1k recent downloads). Worth reading, not worth depending on. |
| Licence | Apple system frameworks; nothing to vendor. objc2 crates are Zlib/Apache/MIT. |
| Real time | `AudioUnitRender` and `MusicDeviceMIDIEvent` are designed to be called from the IO thread. The host side needs no allocation (the prototype builds the `AudioBufferList` on the stack over caller buffers). Whether the plugin itself allocates is up to the plugin. `MusicDeviceMIDIEvent` takes a sample offset, so MIDI can be sample-accurate. AUv3 in-process uses `AUAudioUnit.renderBlock` / `scheduleMIDIEventBlock`: fetch them once off-thread and call them from the callback with no Objective-C messaging. |
| Editors | AUv2: `kAudioUnitProperty_CocoaUI` gives a view factory bundle, and `uiViewForAudioUnit:withSize:` returns an NSView. AUv3: `requestViewController`. Either one hosts in an NSWindow, or inside the desktop app's window. `AUGenericView` is the fallback. |
| State | `kAudioUnitProperty_ClassInfo` gets or sets a CFPropertyList with the complete preset (`fullState` in AUv3). Serialised as a binary plist it is just bytes, so it fits in Registration Memory. Proven here with round trips of 0.9-23 KB. |
| Crash isolation | AUv2 runs in-process: a crashing plugin kills yahaha. AUv3 runs out of process by default on macOS (lower risk, with some IPC cost). |
| Effort to production | **About 2-3 weeks** (see the plan below). |

### 2. VST3 via the `vst3` crate or the VST3 SDK

| | |
|---|---|
| Maturity | Steinberg's format, the standard on Windows. On macOS it is usually shipped alongside AU. |
| Rust access | **`vst3` 0.3.0** (coupler-rs, Dec 2025, about 35k recent downloads, MIT/Apache): bindings generated from the SDK's COM-style interfaces (`IPluginFactory`, `IComponent`, `IAudioProcessor`, `IEditController`, `IPlugView`), usable for plugins and hosts. No safe host layer: the host has to implement `IHostApplication`, `IComponentHandler`, the parameter-change and event-list queues, bus activation and the connection-point plumbing between component and controller. `vst3-sys` (RustAudio) is abandoned (no longer on crates.io). The C++ SDK's `hosting` helpers could be wrapped with `cxx`, but that means a C++ build. |
| Licence | **Since VST 3.8.0 (October 2025) the SDK is MIT-licensed** (VSTGUI and the mda examples are BSD-style). The old dual GPLv3 / proprietary Steinberg agreement no longer applies, so a public MIT repo like yahaha can use it or generate bindings from it freely, keeping the copyright notice. (ASIO moved to GPLv3 at the same time, which does not matter on macOS.) |
| Real time | `IAudioProcessor::process` is RT; events come in an `IEventList` and parameters in an `IParameterChanges` that the host must fill without allocating (pre-allocated pools). More host work than AU. |
| Editors | `IPlugView::attached(nsview, "NSView")`, sized through `IPlugFrame`. |
| State | `IComponent::getState` / `setState` plus `IEditController::getState` / `setState` over an `IBStream` the host implements. Two blobs, not one. |
| Effort to production | **About 3-4 weeks** on top of the shared host infrastructure. Most of it is COM plumbing and testing against real plugins' quirks. |

### 3. CLAP via clack

| | |
|---|---|
| Maturity | Open format (Bitwig/u-he, 2022). Adoption is growing (u-he, Surge XT, Vital, FabFilter, Arturia's newer releases) but it is still a minority among the big sample-library instruments (Kontakt, Spitfire). None of the plugins installed here ship as CLAP. |
| Rust access | **`clack-host` 0.2.0** (prokopyl, published Sept 2026 after years as a git dependency; MIT/Apache; about 20k recent downloads) over **`clap-sys` 0.5**. Safe, low-level, and designed for both plugins and hosts. CLAP's threading model (main thread vs audio thread) is explicit, so clack can enforce it in types. |
| Licence | CLAP is MIT; clack is MIT/Apache. |
| Real time | The cleanest of the three: `process()` takes sample-timestamped input events in one sorted list, and the spec forbids allocation in the audio thread. |
| Editors | `clap.gui` extension: `set_parent` with an NSView, or floating windows. |
| State | `clap.state` extension: `save` / `load` over a stream, one blob. |
| Effort to production | **About 1.5-2 weeks** on top of the shared host infrastructure. It is the easiest API, but it reaches the fewest of the plugins this user owns. |

### 4. Out-of-process host

A helper process (`yahaha-pluginhost`) loads the plugins and exchanges audio and MIDI with
yahaha over shared-memory ring buffers, woken each cycle by a semaphore or Mach port.
Bitwig, Reaper (optional) and Logic (for AUv3) all work this way.

| | |
|---|---|
| Pros | A crashing or hanging plugin cannot take down the arranger, which on stage matters more than anywhere. Formats can be mixed behind one protocol. The main binary's build and licensing stay clean. |
| Cons | Every audio cycle becomes an IPC round trip on the RT path: one extra buffer of latency, or a tight synchronous hand-off with a real-time-priority helper thread (Mach time-constraint policy, as `src/rt.rs` already does). Editors live in the other process, so the desktop app cannot embed them. Plugin state has to be marshalled across. Harder to debug. |
| Effort | **About 4-6 weeks** before plugin-specific work: the protocol, shared memory, watchdog and restart logic, plus everything in options 1-3 inside the helper. |
| Verdict | Defer. AUv3 already gives out-of-process isolation for free. Revisit if in-process AUv2 crashes turn out to be a real problem in practice. The in-process `Instrument` API (`midi` / `render` / `save_state`) is the interface such a helper would implement, so nothing built now is wasted. |

## Phase 2: wiring plan

Phase 1 touched nothing outside `src/plugin/`. Phase 2 needs the hotspot files (`synth.rs`,
`session`, `api`, `live`, the app), so it starts after the hotspot refactor lands and CLAIMs
each file on the board. In order:

### 1. The synth callback (`src/synth.rs`)

- `synth::start` builds a `plugin::rack(8192, rate)` next to the SoundFont `Rack` and moves
  the `PluginRack` into the callback; the `RackControl` goes to the Session (a new field on
  `Synth`, like `SynthControl`).
- In the callback: `plugin_rack.begin_block()` first; in the consumer loop, `if
  !plugin_rack.midi(m, 0) { shadow.note(&m); apply_rack(...) }`; after `rack.render`,
  `plugin_rack.render_add(&mut left2, &mut right2)` into zeroed scratch, scaled by
  `master_gain(master)` (rustysynth applies the master inside its render, the plugin rack
  does not) and added to `left` / `right` before the soft clipper. Meters: the rack's
  per-slot output peak goes into `ctl.peaks[channel]`, so the Mixer shows plugin parts like
  any other (a small `render_add` variant that reports per-slot peaks).
- A plugin channel's messages then never reach rustysynth, so `sync_player_rack` and
  `Shadow` keep working for the SoundFont parts unchanged. The part's GM program is still
  pushed to rustysynth by `sync_player_rack` (it does not go through the rings), but its
  CC7 / CC11 were swallowed by the rack while the plugin played: on clear, the Session
  re-sends the part's current volume and expression so the SoundFont voice comes back at
  the fader's level.
- Sample-accurate MIDI: the input thread already timestamps messages; converting the host
  time to a frame offset in the coming buffer and passing it to `midi(m, offset)` is the one
  change needed for plugin parts (rustysynth would need its own per-offset rendering).
- Device changes (sample rate, buffer): `PluginInstance::reconfigure` off-thread on
  instances removed from the rack, then reassign.

### 2. Session commands (`src/session`, `src/api`)

- `VoiceRef`: `Gm { program }` (today's voices) or `Plugin { id: PluginId, name, state:
  base64 bytes, trim_db }`. A part's voice becomes a `VoiceRef`.
- New `AppCmd`s:
  - `SetPartPlugin { part, id, state? }`: load (async) with the part's rate/buffer, show
    `Loading` in state, assign when ready (`Swap::default()`), fall back to the GM voice and
    report on error / timeout.
  - `ClearPartPlugin { part }`: back to the part's GM voice.
  - `SavePartPluginState { part }`: `get_state` into the part's `VoiceRef` (the Session asks
    after the editor closes and before a Registration write).
  - `OpenPluginEditor { part }` / `ClosePluginEditor { part }`: forwarded to the app's main
    thread with the part's `EditorTarget` (a Tauri command, since the Session has no
    main-thread access).
  - `RescanPlugins`.
- New `AppState` fields: `plugins: { scanning, list: [{ id, name, manufacturer, version,
  format, lastLoad }] }` (from the cache, instant at start-up), and per part `voice.kind:
  "gm" | "plugin"`, `voice.plugin: { id, name, loading: progress | null, error, cpu,
  overruns }` (from `PluginStats` snapshots once a second, not per state push).
- The Session owns the `PluginHost`, the `RackControl` and in-flight `LoadHandle`s; its
  pump polls them (`take`, `poll`) and turns rack events into state (a `Fault` marks the
  part "muted: plugin failed" with a Retry).
- Load mode policy: `Auto` for AUv3; out of process for third-party AUv2 by default, with a
  per-plugin "run in process" setting stored in the cache file (see the findings).
- All state calls run on the Session's control thread, never the main thread (Kontakt).

### 3. App UI (`app/`)

- **Plugin browser** in the part's voice picker: a second tab "Plugins" next to the GM list,
  from `state.plugins.list`, grouped by manufacturer, with the AUv3 badge and a warning
  icon for plugins whose `lastLoad.error` is set ("timed out after 20 s last time").
  Picking one sends `SetPartPlugin`; the part shows a spinner with the progress stage until
  `Ready`.
- **Editor button** on each keyboard part (Parts drawer and Mixer channel strip) when the
  part plays a plugin: a Tauri command that runs `open_editor` on the main thread
  (`app.run_on_main_thread`) and keeps the `Editor` in a main-thread map keyed by part;
  pressing again focuses it; closing the part's plugin closes the window. On close, send
  `SavePartPluginState`.
- Mixer: plugin parts get a "plugin" badge and the CPU / overrun readout in the channel
  tooltip. The fader is the same CC7 as always.
- Settings: an Audio "buffer size" choice (64 / 128 / 256) for heavy plugins, and "Rescan
  plugins".

### 4. Registration Memory

- A Registration stores each part's `VoiceRef` (plugin id, version, name, base64 state,
  trim). Sizes seen: 1-23 KB per part.
- **Recall must never load.** When a Registration bank is selected, the Session preloads
  every plugin voice its buttons use, with their states, off-thread (`load_async` with
  `LoadConfig::state`), into a per-bank pool keyed by (button, part). A button press is then
  `assign` only: under one block to the speakers, 3-7 µs on the control thread.
- Two buttons using the same plugin with different presets get two instances (a second
  instance loads in 7-64 ms once the first is warm). A pool budget (count and total load
  time) keeps a bank with 10 Kontakt buttons from preloading forever; beyond it, the
  button recalls with a visible "loading" state instead of silently lagging.
- The retired instance goes back into the pool (`take_retired`), not dropped, so pressing
  the previous button again is also only a swap. Its state is re-set off-thread to the
  button's stored state before it is eligible again (the user may have tweaked it).
- Unknown plugin on recall (uninstalled, or a Registration from another machine): fall back
  to the part's GM voice and show the plugin's stored name with a warning.

### Remaining effort (AU route)

| Work | Estimate |
|---|---|
| ~~objc2 migration, async AUv3 instantiation, cached scan, rack, state, editor windows~~ | done (phase 1) |
| Synth callback wiring, meters, master gain, CC123 on swap (done in the rack) | 1-2 days |
| Session / API: `VoiceRef`, commands, state, load polling, fault handling | 2-3 days |
| App: plugin browser tab, editor button and main-thread editor map, Mixer badge, buffer setting | 3-4 days |
| Registration: `VoiceRef` storage, per-bank preload pool, recall as swap | 3-4 days |
| Sample-accurate offsets from the input thread; level-normalise trim; Omnisphere-class hardening | 2-3 days |
| **Total** | **about 2-3 weeks** for keyboard parts; Style parts add about 1 week (Yamaha voice → plugin preset map, CPU) |

CLAP would add about 1.5-2 weeks and VST3 about 3-4 weeks on top of this, reusing the rack,
`PartGain`, the preload pool, Registration storage and editor hosting.

## Sources

- VST 3.8 SDK moves to MIT: [KVR](https://www.kvraudio.com/news/steinberg-moves-vst-3-sdk-to-mit-open-source-license-asio-now-gplv3-65179),
  [Steinberg licensing page](https://steinbergmedia.github.io/vst3_dev_portal/pages/VST+3+Licensing/Index.html),
  [Steinberg forum announcement](https://forums.steinberg.net/t/vst-3-8-0-sdk-released/1011988)
- clack: <https://github.com/prokopyl/clack>
- rack: <https://github.com/sinkingsugar/rack>
- Crate versions and download counts are from crates.io, 2026-09-23.
