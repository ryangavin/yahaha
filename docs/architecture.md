# Architecture

How yahaha is put together, and where a new feature goes. Written for the people (and
agents) adding features in parallel: each feature should mostly add files of its own and
touch the shared "hotspot" files in one or two lines.

The contract with clients (the terminal UI, the desktop app) is [app-api.md](app-api.md).
This document is about the inside.

## Crate layout

| Where | What |
|---|---|
| `src/lib.rs` | The `yahaha` library: engine, runtime, app API. The desktop app depends on it with `default-features = false`. |
| `src/main.rs`, `src/ui.rs` | The `yahaha` binary (feature `tui`): the terminal front panel and the developer tools (`screen`, `state-json`, `bench`, `sim`, ...). |
| `src/api.rs`, `src/api/*` | The app API types: `AppCmd`, `AppState`, `Event`, one module per feature. |
| `src/session.rs`, `src/session/*` | `Session`: owns the runtime, runs commands, builds `AppState`. One module per feature. |
| `src/engine.rs`, `src/engine/*` | `Engine`: the arranger (sections, pattern playback, chord following). Deterministic, allocation-free. |
| `src/live.rs`, `src/live/*` | The real-time threads' code: MIDI input (`Input`, the key pipeline) and the engine loop (`EngineLoop`). |
| `src/synth.rs`, `vendor/rustysynth` | The built-in SoundFont synth (cpal audio thread). |
| `src/midi.rs`, `src/rt.rs` | CoreMIDI, real-time helpers (clock, wakeups, packet sink, histograms). |
| `src/sff.rs`, `src/library.rs` | Style files (SFF1/SFF2) and the style library index. |
| `src/theory.rs`, `src/fingering.rs` | Chords, chord recognition, fingering types. |
| `src/parts.rs`, `src/launchkey.rs` | Keyboard parts (Right 1-3, Left) and the Launchkey mapping. |
| `src/controllers.rs` | Pedals, wheels and the assignable-function table (atomics in `Shared`; the input thread and the engine thread send them to the parts). docs/controllers.md. |
| `src/harmony.rs`, `src/arp/`, `src/multipad/`, `src/plugin/` | Feature libraries not yet wired in (pure, real-time safe). |
| `src/ireal/` | iReal Pro charts (pure); the chart player plays them: `engine/chart.rs`, `session/chart.rs`, `api/chart.rs` (docs/ireal.md). |
| `app/` | The desktop app: Svelte frontend (`app/src`), Tauri shell (`app/src-tauri`). |

## Threads

A live `Session` runs these threads:

| Thread | Code | May |
|---|---|---|
| CoreMIDI receive | `live::Input` (`InputHandler::packet`) | atomics, SPSC ring pushes, `Wakeup::signal`. **No allocation, no locks, no blocking.** |
| Engine | `live::run_engine` → `EngineLoop::step` → `Engine` | real-time policy; waits on a semaphore or a deadline, then spins. **No allocation or freeing, no locks.** |
| Audio | `synth` (cpal callback) | renders from its `[u8; 3]` MIDI ring. **No allocation, no locks.** |
| Control | `session` (`Inner` / `Control`) | locks, allocation, files. Runs Launchkey actions as `AppCmd`s, OTS Link, the Launchkey LEDs, library indexing, SoundFont loads; publishes `AppState`. |
| Clients | whoever calls `Session::send` / `state` | `send` runs the command right there, under the control lock. |

Between the real-time threads and the control side there are only lock-free things:

- **Atomics in `live::Shared`** (split, fingering, transpose, the keyboard parts in
  `parts::Parts`, the held keys, the packed chord word, latency histograms).
- **SPSC rings (`rtrb`)**: `Cmd` to the engine (one ring from the input thread, one from
  the control side); prepared styles and auditions in (`Box<Prepared>`, `Box<Audition>`),
  the old ones back out to be dropped on the control side; `Snapshot`s out; Launchkey
  `Action`s from the input thread to the control side; MIDI bytes to the synth.
- **`rt::Wakeup`** semaphores: `shared.wake` (engine), `shared.ctl_wake` (control).

Anything that must be allocated (a style, a preview, a feature's tables) is built on the
control side, sent in through a ring as a `Box`, and sent back out through another ring
to be dropped there. `tests/engine_no_alloc.rs`, `tests/input_no_alloc.rs` and
`tests/arp_no_alloc.rs` check the rule with a counting allocator.

An **offline** session (`Session::offline`, `session/offline.rs`) has no threads: the engine
runs on a virtual clock that `Session::advance` moves, and `Session::midi_in` plays the
keyboard and the Launchkey. Tests, `yahaha state-json` and the app's dev mode use it.

## Data flow

```
 keyboard / Launchkey (CoreMIDI)
        │
        ▼
 live::Input ──────────────── key pipeline (live/pipeline.rs) ──► Out ──► MIDI port + synth ring
   │   chord word (atomic)     note in → transpose → processor → part routing, Keys, output
   │   Cmd ring (buttons, faders)
   │   Action ring (settings, OTS, style change) ─────────────┐
   ▼                                                           ▼
 live::EngineLoop::step ──► Engine (process, button, set_chord)   session Control
   │  Sink = Out ──► MIDI port + synth ring                       ▲   apply(AppCmd) ◄── clients (send)
   └─ Snapshot ring ───────────────────────────────────────────────┘   pump(): snapshots, actions, OTS Link,
                                                                        LEDs, indexing, SoundFont, inputs
                                                                    build_state() ──► AppState (Arc) + Event
```

- **Input → engine.** Chords are recognized on the input thread and published as one
  atomic word (`Shared::chord`, with a generation); the engine reads it at its next wake.
  Buttons and fader moves go through the `Cmd` ring. The keyboard parts' notes never
  reach the engine: the input thread sends them itself (the key pipeline below).
- **Session → engine.** `Control::engine_cmd` pushes a `Cmd` on the control ring; a new style
  goes in as a `Box<Prepared>`. Settings the input thread reads (split, fingering,
  transpose, parts) are atomics in `Shared`/`Parts`, written by the session handlers.
- **Engine → session.** After each wake the engine pushes a `Snapshot` (a small `Copy`
  struct: running, section, queued, bar, beat, tempo, ...) when it changed, and signals
  `ctl_wake`. The control side keeps the latest (`drain_snapshots`).
- **Session → clients.** `Control::build_state` makes a fresh `AppState` from the snapshot,
  the atomics and its own state, each feature building its field. `Inner::publish`
  compares it with the last one; if it changed, it gets a new `version`, is stored as an
  `Arc<AppState>` (`Session::state` is a cheap clone) and `Event::StateChanged` goes to
  the subscribers. The library list is published the same way (`Event::LibraryChanged`).

## Where a feature plugs in

A feature usually touches several layers. For each, the place to add code and the one
line (if any) that goes into a shared file:

### 1. API: `src/api/<feature>.rs`

The feature's commands (one enum, `#[serde(tag = "type", rename_all = "camelCase",
rename_all_fields = "camelCase")]`) and its state (one struct). Then:

- one line in `app_cmd!` in `src/api.rs` (`/// doc` + `Group(GroupCmd),`), and
- one field in `AppState` (camelCase on the wire),
- `mod`/`pub use` lines at the top of `src/api.rs`.

`AppCmd` is grouped in Rust only: on the wire a command is its group's JSON
(`{"type":"main","index":1}`), and `Session::send` takes a group's command directly
(`send(TransportCmd::Main { index: 1 })`). Command `type` names must be unique across
groups. `tests/api_wire.rs` pins the wire format (every documented command round-trips,
the fixtures round-trip byte for byte): extend it, never loosen it.

### 2. Session handler: `src/session/<feature>.rs`

An `impl Control` block with:

- `<feature>_cmd(&mut self, c: FeatureCmd) -> Result<(), CmdError>`: runs a command.
  One arm in `Control::apply` (session.rs).
- `<feature>_state(&self, v: &View) -> FeatureState`: its part of `AppState`. One field in
  `Control::build_state`.
- `pump_<feature>(&mut self, ...)` if it follows the engine or runs a background job
  (loading, scanning). One call in `Control::pump`, which runs its steps in a fixed order.

Control-side state goes in a field of `Control` (session.rs); keep feature state in one
struct of the feature's own module. To reach the engine: `self.engine_cmd(Cmd::...)`; to
reach the input thread: an atomic in `Shared` or `Parts`, or a ring set up in
`Session::start`.

### 3. Engine: `src/engine/<feature>.rs`, hooks and `Features`

Engine behaviour is split by concern into `impl Engine` blocks: `transport.rs` (buttons,
start/stop, tempo), `sections.rs` (section changes), `playback.rs` (`process`),
`chords.rs` (chord following), `mixer.rs`, `setup.rs`, `mirror.rs`, `style_change.rs`,
`prepared.rs` (the style compiled for playback).

A feature that lives in the engine:

- keeps its state in **one field of `hooks::Features`** (fixed-size, no heap; `Engine::new`
  builds it on the control side);
- adds **one call** to its own function in the **hook** it needs (`src/engine/hooks.rs`):
  `on_start`, `on_stop`, `on_bar`, `on_beat`, `before_section_change`,
  `after_section_change`, `on_chord`, `on_style_loaded`; and returns its next deadline from
  `hook_deadline` if it must act exactly on a tick (a metronome click, an arp step). The
  hooks run at fixed points in a fixed order (the table in hooks.rs; tests there pin it);
- gets commands through a new `live::Cmd` variant (one arm in `live::apply`) or reads an
  atomic in `Shared` at the top of `EngineLoop::step`;
- reports through a field of `engine::Snapshot` (it must stay `Copy`).

Everything in the engine is deterministic (time is the `now` passed in) and allocation-free.

### 4. Section-change timing: `Engine::change_point`

"When does a queued change take effect" has one answer: `Engine::change_point(Change,
now)` in `src/engine/sections.rs`. Sections, stops and style changes wait for the next bar
line; fills and breaks start at the next beat. "What plays when a section ends with
nothing queued" is `Engine::follow_on`. A feature that changes timing (Genos Section Change
Timing, OTS Link Timing, a chart player driving sections) changes these policies (a new
`Change` kind or a setting they read), not the call sites.

### 5. Input processor: `src/live/pipeline.rs`

The keyboard-part note path on the MIDI thread is a fixed sequence of stages: **note in →
transpose → processor → part routing, held-note bookkeeping, output**. The processor slot
(`live::Processor`, an enum) is where Keyboard Harmony (`src/harmony.rs`) and the
Arpeggiator (`src/arp/`) go, as variants: they are mutually exclusive, as on the Genos.
A processor sees every note-on (after transpose) and note-off; it passes the note on, or
swallows it, and sends any extra notes itself. Time-domain work does not happen on the
input thread: Harmony's Echo/Tremolo/Trill (`harmony::EchoGen`) run on engine
nanoseconds and the arp on style ticks, both pulled by the engine thread through an
engine hook and `hook_deadline`. The module docs have the details.

The chord section (recognition, Sync Stop) reads the keys as pressed, before the
pipeline's transpose and processor.

### 6. Terminal UI: `src/ui.rs`

A key for the command: `key_action` for a control the Launchkey also has (it maps to a
`launchkey::Action`, so the key and the pad send the same command), else `key_cmd`. The
`keys_send_commands` test covers them; the help lines at the foot of the screen (`help` in
`ui.rs`) list the main ones.

### 7. Launchkey: `src/launchkey.rs`

A pad or button: an `Action` (`launchkey::pad_action` / `cc_control`) that the control
side runs as its `AppCmd` (`impl From<Action> for AppCmd` in `src/api.rs`); its LED in
`session/leds.rs` (`Leds::update`) and `session/surface.rs` (the app's mirror).

### 8. Desktop app

| What | Where |
|---|---|
| Types | `app/src/lib/api/types.ts` (the `AppCmd` union and `AppState`, as docs/app-api.md) |
| Browser mock | `app/src/lib/api/mock.ts` (and `mock-*.ts`) |
| Tauri mock | `app/src-tauri/src/mock.rs` (matches on `AppCmd` groups) |
| Recorded engine shape | `app/src/lib/api/engine-shape.json`, re-recorded from `docs/fixtures` (`yahaha state-json`) |
| Tooltips | `app/src/help/tooltips.ts` (a section per panel); `coverage.test.ts`; `app/docs/controls.md` is generated (`npm run docs:controls`) |
| Panels | `app/src/panels/<panel>/` |
| Shortcuts | `app/src/lib/shortcuts.ts`, `keys.ts` |
| Screenshots | `app/scripts/screenshots.sh` |

### 9. Docs

`docs/app-api.md` (commands and state, with an example), `docs/fixtures/*.json`
(regenerate with `yahaha state-json`, see app-api.md), and a feature doc of its own
(`docs/<feature>.md`) for the Genos behaviour it implements and what is a guess.

## Hotspots and ownership

These files are shared by every feature. Add to them in the marked places only (one line,
one arm, one field), keep feature code in the feature's own module, and claim them on the
coordination board before editing anything else in them:

| Hotspot | What features add | Owned by |
|---|---|---|
| `src/api.rs` | `mod`/`pub use`, one `app_cmd!` line, one `AppState` field | the API: changes to the macro, `AppState` order, `Event` |
| `src/session.rs` | one arm in `apply`, one field in `build_state`, one call in `pump`, a `Control` field | the session frame: threads, locking, publishing |
| `src/engine.rs` | a `Snapshot` field | the engine core: `Engine` fields, types |
| `src/engine/hooks.rs` | one call per hook used, one `Features` field | the hook order and timing |
| `src/engine/sections.rs` | a `Change` kind / a policy branch | section-change timing |
| `src/live.rs` | a `Cmd` variant and its arm in `apply`; a `Shared` atomic | the threads, rings and wiring |
| `src/live/pipeline.rs` | a `Processor` variant and its arm | the stage order |
| `src/ui.rs` | a key in `key_action` / `key_cmd` | the terminal UI |
| `app/src/lib/api/types.ts`, `mock.ts`, `app/src-tauri/src/mock.rs` | the command and state shapes | the app API mirror |
| `app/src/help/tooltips.ts` | a section or keys in the panel's section | per panel |

Rules:

- **Claim before editing** anything in a hotspot beyond the one-line additions above; post
  a CHANGED line when done and release the claim.
- **Keep the wire format.** `tests/api_wire.rs` and `docs/fixtures` must keep passing; a
  change to the JSON is a change to docs/app-api.md and both mocks in the same PR.
- **Keep the order.** `apply`, `pump`, `build_state` and the engine hooks run in a fixed
  order; add to the end of a group unless there's a reason (and say it).
- **Real-time rules** (engine, input, audio threads): no allocation, freeing, locks,
  blocking or I/O; no wall clock in the engine. The no-alloc tests must pass.
- **Behaviour stays pinned.** The golden digests (`src/golden.rs`, `tests/golden`),
  `yahaha screen` and `yahaha state-json` output only change on purpose.

## Adding a feature

1. **Board.** Post a CLAIM for the hotspots you'll touch beyond one-line additions.
2. **Library.** The feature's logic in its own module (`src/<feature>.rs` or
   `src/<feature>/`), pure and tested on its own; real-time safe if a real-time thread
   runs it.
3. **API.** `src/api/<feature>.rs`: command enum and state struct; one `app_cmd!` line and
   one `AppState` field in `src/api.rs`. Round-trip cases in `tests/api_wire.rs`.
4. **Session.** `src/session/<feature>.rs`: `<feature>_cmd`, `<feature>_state`, and a pump
   step if needed; one line each in `apply`, `build_state`, `pump`.
5. **Engine** (if the band's playing is involved): a `Features` field, calls in the hooks
   you need, `hook_deadline` if you need exact ticks, a `Cmd` variant for commands, a
   `Snapshot` field for state. Timing of section changes: `change_point` / `follow_on`.
6. **Input** (if it changes what the keys play): a `Processor` variant in
   `src/live/pipeline.rs`.
7. **Front ends.** A TUI key in `src/ui.rs`; a Launchkey `Action` if it has a pad; in the
   app: `types.ts`, both mocks, a panel, tooltips, `coverage.test.ts`, regenerated
   `app/docs/controls.md`.
8. **Docs.** docs/app-api.md (commands, state, example), regenerated `docs/fixtures`,
   `app/src/lib/api/engine-shape.json`, and `docs/<feature>.md`.
9. **Verify.** `cargo build --release` (default, `--no-default-features`,
   `--features plugins`), `cargo test --release` (goldens unchanged unless on purpose,
   no-alloc tests), `cargo test` in `app/src-tauri`, `npm run verify` in `app/`, no new
   clippy warnings, and `yahaha bench <style>` if a real-time path changed.
10. **Board.** A CHANGED line (what moved, new shapes, new tooltip keys), then release
    the claims.
