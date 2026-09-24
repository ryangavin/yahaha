# Plugin hosting (#35): options report, prototype, integration plan

Goal: Genos-class sounds for the keyboard parts (Right 1-3, Left), and optionally for the
Style parts, played through the user's own instrument plugins instead of the built-in
SoundFont synth (rustysynth, `src/synth.rs`).

This is a spike. It has three parts: the options (below), a working Audio Unit prototype behind the
`plugins` cargo feature (`src/plugin/`), and a plan for wiring it into the Session and
synth (at the end). Nothing in the default build changes.

## Recommendation

**Use Audio Units first, hosted in-process from the audio callback we already have.** Call the
AudioToolbox C API for scanning, loading, MIDI and rendering, and use `objc2` for AUv3
instantiation and editor windows. Add **CLAP via `clack-host`** as the second format if
users ask for plugins that ship without AU. Treat **VST3** as the last format: on macOS almost
every commercial VST3 instrument also ships as an AU. Do not build an out-of-process host
now. AUv3 extensions already run out of process, and the cost of a separate host is only
worth paying later if crashing AUv2 plugins become a real problem.

Why AU:
- yahaha is macOS-only. On macOS, AU is the format every instrument vendor ships. On this
  machine it includes Kontakt 6/7/8, Serum 2, FM8, Reaktor, EZdrummer and the Virus editor,
  plus Apple's DLSMusicDevice, AUSampler and AUMIDISynth, which are always present.
- The OS provides it and it is stable. There is no SDK to vendor or license, and the scan
  is one system call. The prototype adds **zero dependencies**.
- It works today. The prototype loads, plays, saves and restores the state of, and opens
  the editor of third-party instruments (below).

## Prototype results (this machine, M-series Mac, MOTU Model 16, 48 kHz / 64 frames)

| Instrument | Load | Plays | State (ClassInfo) | Editor |
|---|---|---|---|---|
| Apple DLSMusicDevice | 7 ms | yes, peak -22 dBFS | 916 B, save+restore OK | own Cocoa view, 518x243 |
| Xfer Serum 2 | 94-205 ms | yes, peak -15 dBFS | 23 KB, save+restore OK | own Cocoa view, 1190x740 |
| NI FM8 | 431 ms | yes, peak -27 dBFS | 2 KB, save+restore OK | not tried |
| NI Kontakt 8 | 0.7-3.2 s | silent: no library loaded (expected) | 4 KB, save+restore OK | **crashes** inside `KontaktFormManager::init` (see findings) |

The plugin part is rendered in the same cpal callback as rustysynth and adds no extra
buffering: the latency is the device buffer (64 frames, 1.3 ms), the same as the built-in
synth.

### Findings

- **Load times vary by three orders of magnitude.** Kontakt can take seconds. A
  Registration recall can never load a plugin on demand: instances have to be created
  ahead of time, off the audio thread, and swapped in.
- **Kontakt 8's editor crashes** when this bare CLI process opens it (EXC_BAD_ACCESS in
  Kontakt's own `KontaktFormManager::init`, called from `uiViewForAudioUnit:withSize:`).
  Serum 2 and DLS open fine. The likely cause is that Kontakt's Qt UI expects a real
  `NSApplication` (`[NSApp run]`, a main bundle with an Info.plist), not a hand-pumped event
  loop in an unbundled binary. The desktop app would provide that. Re-test there before
  concluding anything. Headless Kontakt (no editor) loads, renders and saves state fine.
- **Opening a heavy editor causes audio underruns** at 64 frames: the plugin's UI
  initialisation competes with its render. Production should open editors only on user
  request, and should expect to raise the buffer to 128/256 frames when heavy plugins are
  active.
- **Plugins disagree about CC7.** DLS honours it; many synths ignore it or map it to a
  macro. That is why the host applies part volume itself (see the mixer section).
- A component flagged `RequiresAsyncInstantiation` (some AUv3s) cannot be created with the
  synchronous `AudioComponentInstanceNew`. None of the instruments installed here needed
  it. Production needs the async path (`AudioComponentInstantiate` or
  `AUAudioUnit.instantiate`), which means objc2 blocks.

## How to run the prototype

```sh
cargo build --release --features plugins
./target/release/yahaha plugin-test --list                  # installed instrument AUs
./target/release/yahaha plugin-test                         # Apple DLSMusicDevice
./target/release/yahaha plugin-test "Serum 2"               # any name substring, or "aumu Xf2X XFER"
./target/release/yahaha plugin-test "Serum 2" --gui         # also open its editor; loops until you close it
```

Options: `--channel N` (the MIDI channel that goes to the plugin; default 1 = Right 1),
`--sf2 file | --no-sf2` (the backing SoundFont; by default the first `.sf2` in `./soundfonts`),
`--loops N`, `--seconds N`, `--audio-out N` (first channel of the output pair; Model 16
defaults to 11/12, as the built-in synth does).

The phrase is four bars at 100 bpm: C Am F G, then C. The melody plays on the plugin
channel; strings (MIDI ch 2) and drums (ch 10) play on rustysynth in the same callback. On
the last pass the plugin part's CC7 fades out over the final bar, to demonstrate host-side
volume. At the end the command prints the plugin part's peak level, so a silent plugin
shows up without anyone having to listen.

Tests (they use DLSMusicDevice, which is always installed): `cargo test --features plugins plugin`.
They cover the FFI struct layouts, the scan, an offline render that is silent before
note-on and sounding after it, a state round trip, and the `PartGain` curve.

### Code map

- `src/plugin/ffi.rs`: hand-written AudioToolbox, CoreFoundation and objc-runtime
  declarations, with a test that checks the struct sizes against the C headers.
- `src/plugin/au.rs`: `scan()`, `find()`, and `Instrument` (`load`, `midi`, `render`,
  `latency_seconds`, `save_state`, `restore_state`).
- `src/plugin/gui.rs`: the editor window. It uses the plugin's Cocoa view
  (`kAudioUnitProperty_CocoaUI`) if it has one, and CoreAudioKit's `AUGenericView` otherwise.
- `src/plugin/mod.rs`: `PartGain`, the host side of the mixer rule.
- `src/plugin/cli.rs`: `yahaha plugin-test`.

## The options

### 1. Audio Units (AUv2 and AUv3) through AudioToolbox / AVFoundation

| | |
|---|---|
| Maturity | Apple's native format since 10.2. AUv3 (app extensions) since 10.11. Every major macOS DAW hosts it. |
| Rust access | **Raw C API** (as in this prototype): `AudioComponentFindNext`, `AudioComponentInstanceNew`, `AudioUnitRender`, `MusicDeviceMIDIEvent`, `kAudioUnitProperty_ClassInfo`. **`objc2-audio-toolbox` / `objc2-avf-audio` / `objc2-core-audio-kit` 0.3.2**: generated from the SDK headers by the objc2 project, very healthy (about 2M recent downloads for audio-toolbox), and they cover `AUAudioUnit`, `AVAudioUnitComponentManager` and `AUGenericView` with typed blocks. `coreaudio-sys` / `coreaudio-rs` (RustAudio) are bindgen'd C APIs, widely used through cpal. **`rack` 0.4** (sinkingsugar) is a young AU+VST3 host built on a C++ shim, with little adoption (about 1k recent downloads). Worth reading, not worth depending on. |
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

## Integration plan (not done in this spike)

### Per-part routing

Each keyboard part (`parts::CHANNEL`: Right 1 = ch 0, Left = ch 1, Right 2 = ch 2, Right 3 = ch 3)
gets a **voice source**: either the SoundFont program it has today, or a **plugin
voice**. A plugin voice is the component's identity (type/subtype/manufacturer and version),
a display name and a state blob.

- **Audio thread (`synth.rs` callback).** Add a fixed array `[Option<Box<PluginSlot>>; 16]`
  indexed by MIDI channel, where a slot holds an `Instrument`, a `PartGain` and scratch
  buffers. When `apply` drains the rings, a message for a channel with a slot goes to that
  slot (`PartGain::take`, otherwise `Instrument::midi`). Every other channel goes to the
  player or band synth exactly as now. After `player_synth.render`, render each active slot
  and add it into the same `left` / `right` before master gain and `soft_clip`. Channels
  8-15 (the Style parts) can use the same array later, which is the "optionally the Style
  parts" case. Style parts would also need a Yamaha-voice-to-plugin-preset map, because a
  Style's bank/program changes mean nothing to a plugin.
- **Loading.** The audio thread never loads, initialises, restores state or disposes an
  instance. A control thread (the Session's) builds the `Instrument`, restores its state,
  and sends `Box<PluginSlot>` to the callback over an `rtrb` ring. The callback swaps it
  into the channel and sends the old box back on a second ring, so it is dropped off the
  audio thread. Before the swap the callback sends All Notes Off (CC123) to the outgoing
  instrument, so no note hangs. This replaces today's `parts.changed` / `sync_player`
  program push for plugin parts.
- **Session / API.** `AppCmd::SetPartVoice { part, voice: VoiceRef }`, where `VoiceRef` is
  either `Gm(program)` or `Plugin { code, state }`. `AppCmd::OpenPluginEditor { part }` is
  honoured by the desktop app on its main thread. `AppState` shows each part's voice name and
  a plugin/SoundFont badge. The scan result is cached, with the component version used as
  the cache key, and exposed so the UI can offer a voice list.
- **Registration Memory.** Each part stores its `VoiceRef`, so a plugin voice stores the code,
  version and the `save_state()` blob (base64 in the registration JSON). Because Kontakt can
  take seconds to load, recall must be instant from a **preloaded pool**. When a
  Registration bank is selected, instantiate and restore every plugin voice its 10 buttons
  use, off-thread. A button press is then only a swap. Where two buttons share a plugin
  with different states, a `restore_state` on a warm instance can replace a fresh load, but
  that call is not RT-safe, so it happens off-thread on an instance that is not playing
  (double-buffer: one playing, one being prepared).

### Mixer rule: CC7 is the only per-part volume

The built-in synth has one rule: a part's level is its channel's CC7 (the mixer fader),
CC11 and velocity, on the GM curves, and the master fader is the only non-MIDI gain. Plugin
parts keep that rule, but the host enforces it. `PartGain` swallows CC7 and CC11 on a
plugin channel and applies `((CC7/127)·(CC11/127))²` to the plugin's output, ramped over
one buffer. This is the same curve rustysynth uses, so a fader at 100 sounds the same on
either engine, and plugins that ignore CC7 or remap it still obey the mixer. Velocity
still goes to the plugin, where it belongs (it shapes timbre, not only level). Master gain
and the soft clipper stay after the sum, unchanged. Pan (CC10) has the same problem;
handling it host-side with constant-power pan is a small follow-up. Every plugin should
also be **level-normalised once** when it is added (a trim stored with the plugin voice,
set from a reference note so a fresh plugin voice sits near its SoundFont equivalent),
because Serum 2 and FM8 differ by 12 dB at the same settings (measured above).

### Latency and CPU

- **In-process adds no buffering.** The plugin renders in the same callback, so the
  input-to-sound latency stays at one device buffer (1.3 ms at 64 frames / 48 kHz) plus
  the plugin's own lookahead. `Instrument::latency_seconds()` reports that lookahead; all
  instruments tested here report 0. For live play, do not compensate a part's latency by
  delaying the other parts: that adds latency to everything. Show it in the UI instead.
- **MIDI timing.** Today (and in the prototype) messages are applied at the start of the
  buffer, so they are quantised to the buffer (1.3 ms). `MusicDeviceMIDIEvent`'s offset
  argument allows sample-accurate timing once the input thread timestamps messages (CoreMIDI
  host time converted to a frame offset from the callback's timestamp). This is worth doing
  for the Style parts, where many notes land together.
- **CPU.** One heavy instrument (a Kontakt piano) can use most of a 1.3 ms cycle. Options,
  simplest first: (1) raise the buffer to 128/256 when plugins are active, which the UI can
  offer as a "latency" setting; (2) render plugin parts on worker threads in the same audio
  workgroup (`os_workgroup` via `kAudioOutputUnitProperty_OSWorkgroup`; `src/rt.rs` only sets
  time-constraint policy today) and join before mixing; (3) for Style parts only, render one buffer
  ahead, since the engine schedules them and can send them early.
- **Sample-rate and device changes.** The AU must be uninitialised, reconfigured and
  reinitialised off-thread, then swapped back in. The same swap path as loading works.

### Remaining effort (AU route)

| Work | Estimate |
|---|---|
| Switch the FFI to `objc2-audio-toolbox` / `objc2-avf-audio`; async AUv3 instantiation; cached scan | 3-4 days |
| Plugin slots in the synth callback, swap rings, `PartGain` wired to the channels, CC123 on swap | 2-3 days |
| Session/API: `SetPartVoice`, voice list, plugin badge in `AppState`, TUI voice picker entry | 2-3 days |
| Registration: `VoiceRef` storage, preload pool, double-buffered recall | 3-4 days |
| Editor windows in the desktop app (AUv2 Cocoa + AUv3 view controller + generic fallback); Kontakt re-test there | 2-3 days |
| Sample-accurate MIDI offsets; buffer-size setting; level-normalise trim; hardening with Kontakt / Serum / Omnisphere-class plugins | 3-5 days |
| **Total** | **about 3-4 weeks** for keyboard parts; Style parts add about 1 week (voice mapping UI, CPU) |

CLAP would add about 1.5-2 weeks and VST3 about 3-4 weeks on top of this, reusing the slots,
swap rings, `PartGain`, Registration storage and editor hosting.

## Sources

- VST 3.8 SDK moves to MIT: [KVR](https://www.kvraudio.com/news/steinberg-moves-vst-3-sdk-to-mit-open-source-license-asio-now-gplv3-65179),
  [Steinberg licensing page](https://steinbergmedia.github.io/vst3_dev_portal/pages/VST+3+Licensing/Index.html),
  [Steinberg forum announcement](https://forums.steinberg.net/t/vst-3-8-0-sdk-released/1011988)
- clack: <https://github.com/prokopyl/clack>
- rack: <https://github.com/sinkingsugar/rack>
- Crate versions and download counts are from crates.io, 2026-09-23.
