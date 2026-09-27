# Orchestrator handoff: Rust idiom and maintainability review (2026-09-27)

You are the ORCHESTRATOR for a code-quality wave. This page gives you the findings, the tracks to spawn, and, for every PR, the OUTCOME you validate against. The full review with every file:line reference is `docs/agents/review-2026-09-27.md`; read it once, then work from this page.

## 0. Read first
- `docs/agents/wave-brief.md` and `docs/agents/wave2/wave2-brief.md`: the hard rules (small PRs, gates, READY handoff, both mocks, no copyrighted data). They all apply here.
- `docs/architecture.md`: the thread model and the add-a-feature checklist. Refactors must not move work across threads.
- `docs/agents/review-2026-09-27.md`: the findings this page is built from. Round 1 covers idiom, single responsibility and error handling; Round 2 covers duplication. `docs/agents/testing-2026-09-27.md` covers test organisation, fixtures and CI.

## 1. What this wave is, and is not
- **It is a quality wave.** No PR in it changes behaviour the user can hear or see: the band plays the same notes, the JSON wire format stays byte-identical, LEDs light the same. The exceptions are the six latent bugs in Track A, each of which changes behaviour only on an input that panics or corrupts today.
- **Every refactor PR ships with a proof of no change.** The proof is the existing test suite plus the gates. Where a PR touches the engine or synth, it also names the golden digest and no-alloc test that covered it. A reviewer who cannot see the proof in the PR body sends it back.
- **Small PRs, one concern each**, under about 400 changed lines excluding tests and moved code. A pure move (a test module to its own file, a helper to a shared module) may be larger if the diff is mechanical and `git diff --stat` shows deletions matching additions.
- **The review could not run clippy.** It ran on Linux, where the crate does not build (`mach2`, `coremidi-sys`). Every track's first PR on macOS should record the clippy warning count on `develop` so later PRs can prove "no new warnings".

## 2. Baseline you validate against
Measured on `develop` at `724686d` (merge of #319). Re-measure with the commands given; report drift.

| Metric | Command | Baseline | Target at end of wave |
|---|---|---|---|
| rustfmt diff hunks | `cargo fmt --check 2>&1 \| grep -c '^Diff in'` | 4086 | 0, and `fmt --check` in the gate |
| Lines over 120 columns in `src` (excl. tests) | `awk 'length > 120' $(git ls-files 'src/*.rs' \| grep -v tests) \| wc -l` | 823 | 0 at whatever `max_width` the owner picks |
| Production `unwrap()` outside tests | see review §metrics | 52 | 52 or lower; no new ones in `plugin/host.rs` |
| `.lock().unwrap()` in `src/plugin` | `grep -rn 'lock().unwrap()' src/plugin \| wc -l` | 24 | 0 |
| `unsafe` blocks without `// SAFETY:` | `clippy::undocumented_unsafe_blocks` | 108 blocks, about 20 documented | 0 undocumented, lint on |
| `pub` items vs `pub(crate)` in the library | `grep -cE '^ *pub (fn\|struct\|enum\|const\|static\|type\|trait)'` / `grep -c 'pub(crate)'` | 1826 / 135 | `Parts` atomics, `Prepared` fields, `Shared` fields no longer `pub` |
| Definitions of `bar_ns` (fn or closure) | `grep -rnE 'fn bar_ns\|let bar_ns' src tests \| wc -l` | 11 (plus 55 inline `60e9` expressions) | 1 |
| Files containing the SlowWalker corpus path | `grep -rl 'SlowWalker.T552' src tests \| wc -l` | 46 | 1 (the testkit) |
| Occurrences of "corpus missing; skipping" / "no corpus" | `grep -rn 'corpus missing\|no corpus' src tests \| wc -l` | 107 | 1 macro or helper |
| Atomic-write (tmp + rename) implementations | `grep -rn 'fs::rename(&tmp' src \| wc -l` | 6 (+1 plain `fs::write` in `session/settings.rs`) | 1 |
| Hand-mirrored api/engine enum match arms | review §16 | 44 | 0, or `From` impls only |
| Largest function (non-test) | brace-depth script in review | 348 lines (`capture::import`) | none over 150 |
| Fields on `session::Control` | read `src/session.rs` | 71 | about 40 (loose fields grouped) |

## 3. Tracks
Each track is a chain of small PRs. Spawn one agent per track; tracks A, C and T are independent of everything and can start at once. Track H's H1 waits for T1 (it needs the synthetic style) and for the owner's decision. Order inside a track matters where marked.

For every PR the agent's report must state: what changed, the OUTCOME below restated as a checklist with each item ticked, and the exact commands run. You validate against the OUTCOME, not against the diff.

### Track A: latent bugs (start first, six one-file PRs)
Each PR carries a test that fails before and passes after.

**A1. SF2 chunk underflow.** `src/patches/sf2.rs:43,47`: `size - 4` on a `LIST` chunk with `size < 4`.
Outcome: a `LIST` header with size 0..3 returns `Err`, never panics; a test feeds such a file through `sf2::preset_names` (or the public entry) and asserts `is_err()`; the existing SF2 tests still pass.

**A2. Sound library save on serialisation failure.** `src/patches/store.rs:88,103`: `to_json()` returns `unwrap_or_default()` and `save()` writes it.
Outcome: `to_json` returns `Result<String>` (or `save` uses `?` on `serde_json::to_string_pretty`); a failing serialisation leaves the existing file untouched; a test with a poisoned value (or a mocked writer) proves no empty file is written.

**A3. `Engine::retire` overflow drops on the RT thread.** `src/engine/style_change.rs:29-34`.
Outcome: on overflow the evicted `Box<Prepared>` is returned to the caller (or the path is a `debug_assert!` with the comment corrected); `tests/engine_no_alloc.rs` unchanged and green; a unit test retires five styles and asserts none is dropped inside the engine (count via a `Drop` counter type or by taking all five back).

**A4. Audio tail past 8192 frames.** `src/synth.rs:1070,1200` plus the eight `8192` literals.
Outcome: one `const MAX_FRAMES` used for every scratch allocation; `process()` either zero-fills `out[frames * channels..]` or renders in `MAX_FRAMES` chunks; a test calls `process` with a 16384-frame buffer and asserts the tail is finite and zero when muted; `tests/synth_no_alloc.rs` unchanged and green.

**A5. Extra-font idle mask above 64 slots.** `src/synth.rs:327-335` vs `synth/routing.rs:73`.
Outcome: extras capped at a named constant of at most 63 shared by both sites, or the mask replaced by a per-slot `Vec<bool>` allocated in `new`; a test routes a channel to slot 65 (or to the cap) and asserts it renders after `IDLE_FRAMES`.

**A6. Plugin host locks and lock-held I/O.** `src/plugin/host.rs` and `src/plugin/editor.rs` (24 sites), `host.rs:143-151`, `editor.rs:180-212`.
Outcome: zero `lock().unwrap()` under `src/plugin` (one `lock()` helper in `plugin/mod.rs`, same poisoning policy as `plugin/sys.rs`); `check_deadline` calls `record` after releasing `state`; the view-controller slot holds a `Retained<NSViewController>` so a late completion block cannot leak; `cargo test --features plugins` green.

### Track B: part and channel validation (two PRs, B1 before B2)
**B1. One validation path for keyboard-part indices.** `src/session/parts.rs`, `session/mixer.rs`, `session/plugins.rs`, `session/sound_library.rs`, `api/controllers.rs`, `api/mixer.rs` (34 sites), `src/parts.rs` accessors, `src/knobs.rs` (`& 3`, `.min(2)`).
Outcome: an out-of-range part in any `*Cmd` returns `CmdError::Failed` (the way `multipad.rs:93-99` does) instead of aliasing or panicking; a table-driven test sends `part: 4` and `part: 255` to every part-taking command in `AppCmd` and asserts the error; `tests/api_wire.rs` unchanged; no `& 3` or `.min(2)` on a part index remains in `session/` or `knobs.rs`.

**B2. `Part` enum.** `#[repr(u8)] enum Part { Right1, Right2, Right3, Left }` with `ALL`, `channel()`, `index()`, `Index<Part>` on `[T; 4]`, `TryFrom<u8>`, serde as the existing integer.
Outcome: `Parts`, `Controllers::set_part`, `KnobFn::Part*` and the `*Cmd` structs take `Part`; the JSON in `tests/api_wire.rs` is byte-identical; `docs/app-api.md` unchanged; `grep -rn '& 3' src` shows no part-index masking.

### Track C: tooling (three PRs, C1 first)
**C1. Format the tree.** Add `rustfmt.toml` (the owner picks `max_width`; 120 or 140 keeps most of the current one-line style), run `cargo fmt` once, add `cargo fmt --check` to the gates in `docs/agents/wave-brief.md` and `docs/architecture.md`.
Outcome: `cargo fmt --check` is clean; the PR is format-only (`git diff -w --stat` shows near-zero non-whitespace change; the agent pastes it); every other open PR merges `develop` in afterwards, so schedule C1 at a quiet moment and announce it on the board.

**C2. `[lints]` table.** In `Cargo.toml`: `clippy::undocumented_unsafe_blocks = "warn"`, `clippy::cast_possible_truncation = "warn"`, `clippy::too_many_lines = "warn"` (threshold 150 in `clippy.toml`), plus whatever `clippy -D warnings` already flags on `develop` (see `docs/handoff.md:57`).
Outcome: `cargo clippy --all-targets` on macOS reports the counts per lint in the PR body; nothing is allowed at file scope to silence them except the existing `plugin/sys.rs` `non_upper_case_globals`; the counts become the baseline for C3 and Track F.

**C3. SAFETY comments.** `src/rt.rs` (16 blocks, 0 comments), `src/midi.rs` (19, 2), `src/plugin/sys.rs` (33, 3), `src/bench.rs` (2, 0), `src/perf.rs`.
Outcome: `undocumented_unsafe_blocks` reports zero; each comment states the invariant, not the operation; `Wakeup::new` checks `semaphore_create`'s return (debug assert at minimum) and the `Copy` semaphore handle is either documented as never destroyed or made non-`Copy`; `midi::init()` returns the CoreMIDI status instead of swallowing it.

### Track T: testing (replaces the old Track D; details in `testing-2026-09-27.md`)
Order: T1, then T2 and T3 in parallel, then T4.

**T1. Shippable fixtures.** `sff::synthetic::StyleBuilder` writing SFF1 and SFF2 (after the `smf::write` merge in H4), a set of generated styles under `tests/fixtures/styles/`, generated SoundFonts under `tests/fixtures/soundfonts/` from an extended `tiny_sound_font`, and full golden listings for the synthetic styles.
Outcome: each fixture file has a regeneration test that byte-compares it; `docs/fixtures/README.md` says what each exercises (every NTR, every NTT table, chord mute, fills mid-bar, in-pattern program and volume changes, bend, SInt with RPN and XG SysEx, 3/4 and 6/8, OTS, no CASM, SFF1); a golden listing per synthetic style is committed in full and diffed by the existing golden harness; breaking `theory::transpose` on a scratch branch turns Linux CI red.

**T2. Explicit skips.** The 107 early returns become either the synthetic fixture (tests that need *a* style) or `#[ignore = "corpus: <style>"]` (tests that need that Yamaha style); `testkit::corpus()` panics when a corpus file is missing.
Outcome: `grep -rn 'skipping' src tests` is empty; the vacuous-test count (script in the testing doc §1) is 0; `cargo test` prints the ignored count; the owner's gate runs `--include-ignored`; `.config/nextest.toml` (if nextest is adopted) names the `corpus`, `soundfont` and `hardware` tiers.

**T3. `testkit` and `tests/common`.** One in-crate `#[cfg(test)] pub(crate) mod testkit` and one `tests/common/mod.rs` with the surface listed in the review (D7): fixture loaders, `offline()`, one `Recorder` with `ons/offs/cc`, `Null`, `bar_ns`, the three stepping helpers, `until`, MIDI byte builders, `chord()`, `temp_dir()`, the per-thread counting allocator, `energy()`; plus `impl Default for engine::Snapshot`.
Outcome: `fn bar_ns`/`let bar_ns` defined once; `const MS` once; `impl Sink for` in test code at most 3 (Recorder, Null, the click recorder); the SlowWalker path in one file; five byte-identical allocator preambles gone; test count unchanged (`cargo test -- --list | wc -l` before and after in the PR body).

**T4. Placement and virtual time.** Apply the one placement rule (inline only under about 100 lines and needing private items; otherwise `<module>/tests.rs`): `sim.rs` splits into `sim/{mod,recorder,script}.rs` plus tests, and `live.rs`, `synth.rs`, `theory.rs`, `capture.rs`, `sff.rs`, `session/fx.rs`, `session/style_change.rs` (its tests move under `ots`), `session/controllers.rs` move theirs out. The six `wait_*` loops and the `Instant::now()` pumps in the plugin tests drive the offline clock instead.
Outcome: no production file carries an inline test module over 100 lines; `git diff -M --stat` shows moves; `grep -rn 'Instant::now' src/**/*_tests.rs tests` is limited to the one bounded helper; test count unchanged.

**CI (with T1 to T3).** The Linux job runs T0 and T1 with real assertions; the macOS job runs `--features plugins` restricted to the plugin and `cfg(target_os = "macos")` tests.
Outcome: macOS job under ten minutes; both jobs print the ignored count; `AGENTS.md`'s sentence about the corpus becomes "the corpus tests are ignored on CI; run `--include-ignored` locally".

### Track E: persistence and paths (three PRs, E1 first)
**E1. One atomic writer.** `crate::fsutil::write_atomic(path, bytes) -> Result<()>` using the pid-plus-sequence tmp naming from `plugin/scan.rs:186`; replaces `registration/mod.rs:490`, `patches/store.rs:101`, `plugin/presets.rs:238`, `session/sound_set.rs:99`, `session/plugins.rs:1055`, and `session/settings.rs:56` (today a plain `fs::write` that swallows errors).
Outcome: `grep -rn 'rename(' src` finds the helper only; `settings.rs` surfaces its error through the existing `Message` path; two concurrent writers to one path in a test never leave a partial file.

**E2. Persistence off the control lock.** A `Persist` worker thread with an `mpsc` of `(PathBuf, Vec<u8>)` jobs; the six per-edit savers (`param_lock.rs:19`, `registration.rs:311-329`, `looper_banks.rs:105`, `sound_library.rs:389-408`, `sound_set.rs`, `settings.rs`, `plugins.rs:837`) enqueue instead of writing.
Outcome: no `fs::write`/`write_atomic` call reachable from `Control::apply` or `Control::pump` (the agent lists the call graph in the PR body); a test drives 100 edits with a slow writer stub and asserts `Session::send` latency stays under the 10 ms control cadence; files on disk are identical to before after the queue drains (existing save/load round-trip tests).

**E3. Style parsing off the control lock, and one data-dir rule.** `library.rs:151` (`Style::load` + `Prepared::new` inside `load_entry`) moves to a loader thread the way `sf_load` works, using the existing `pending_style` handoff; the hard-coded `~/Library/Application Support/yahaha` paths in `session/settings.rs:41` and `session/plugins.rs:100` route through `Options::data_dir` (or a `Paths` struct built once in `assemble`).
Outcome: `switch_style` on a large style no longer blocks `Session::send` (timed test with the offline session and a synthetic 5 MB style); with `Options::data_dir` set to a temp dir, no test writes outside it (assert by listing the real support dir before and after in a test guarded to the CI machine).

### Track F: structure (five PRs, any order after C1)
Each is a pure refactor: behaviour identical, proven by the existing suite plus the goldens.

**F1. Group `Control`'s loose fields.** `src/session.rs:213-332`: `LibraryCtl`, `SoundFontCtl`, `OtsLink`, `LedClock`, `Sources` sub-structs, matching the shape the newer features already use.
Outcome: `Control` under about 40 fields; `assemble()` under 40 lines; the 26 tuple-typed fields listed in review §10 become named structs on the way; no `pub(super)` access to another feature's sub-struct internals (each `impl Control` module touches only its own).

**F2. `Shared` by owner, `Snapshot: Default`, `Input` wiring.** `src/live.rs:121-212`, `engine.rs:196`, `live.rs:652-672`.
Outcome: `Shared` fields grouped by writing thread with writer methods and the ordering chosen once (the doc-comment ownership rules become the module structure); `Snapshot` derives or implements `Default` and the two 45-field literals in `launchkey.rs:1207` and `ui.rs:836` use struct-update syntax; `Input::new` takes an `InputWiring` argument, and `pipeline.rs:251`'s silent fallback when `fx_tx` is `None` is either impossible or explicit.

**F3. Split `AudioCore::process` and `EngineLoop::step`.** `src/synth.rs:912-1222`, `src/live.rs:1369-1509`.
Outcome: each becomes a sequence of named private methods (`take_new_rack`, `route_channels`, `drain_midi`, `update_sends`, `render_racks`, `mix_out`; `take_handoffs`, `apply_commands`, `sync_parts_out`, `publish_snapshot`); `tests/synth_no_alloc.rs`, `tests/engine_no_alloc.rs`, `perform_no_alloc.rs` unchanged and green; `yahaha bench-audio` numbers in the PR body within noise of `develop`.

**F4. Pure planning out of `revoice_part`, `Prepared::new`, `capture::import`.** `engine/chords.rs:307`, `engine/prepared.rs:209`, `capture.rs:774`.
Outcome: `plan_revoice(&self, ...) -> RevoicePlan` is pure and gets direct unit tests for the voice-leading rules without a corpus; `Prepared::new` is `build_sections` + `dedupe_routes` + `scan_bend_ranges` + `tempo_map`; `import` returns `Vec<Problem>` (an enum) rendered by `Display`, and the tests in `capture.rs` assert on variants instead of sentences (`report.contains("leave the tempo alone")` and its siblings are gone); golden digests unchanged.

**F5. Narrow the public surface.** `Parts` atomics private with the missing accessors (`parts.rs:119-150`, 16 external writers); `Engine.style` and `Prepared` fields `pub(crate)` with read accessors; `midi::Client.client` private and `Endpoint`/`OutPort` newtypes; `api.rs` re-exports explicit (no glob) with the logic (`function_run`, `guess_category`, base64) moved to `controllers`, `patches` and a `util` module; `patches/mod.rs` globs made explicit.
Outcome: `cargo doc --no-deps` item count drops (report before and after); the desktop app (`app/src-tauri`) still compiles against the library with `default-features = false`; `tests/api_wire.rs` unchanged.

### Track G: domain types (four PRs, largest change, last)
**G1. `ChordType`.** `#[repr(u8)] enum ChordType` in `theory.rs` with `TryFrom<u8>`; `fingering.rs:270-276` consts, `fingering.rs:412-423`, `theory.rs:343-421,525-533`, `ireal/chords.rs:33-176`, `sff.rs:113` doc, `multipad/player.rs:99`.
Outcome: no numeric chord-type literal outside `ChordType`'s own definition (`grep -n '=> 19\|19..=27' src/theory.rs src/fingering.rs` empty); `Chord` stays `Copy` and its `pack`/`unpack` layout unchanged (round-trip test exists); recognizer golden unchanged; `tests/*_no_alloc.rs` green.

**G2. `Slot` and `Next`.** `Slot(u8)` with `main(i)`, `fill(i)`, `ending(i)`, `kind()`; `enum Next { Section(Slot), Stop }` replacing `usize::MAX`; `enum Source { Pattern(u8), StopAcmp, Unison }` replacing 254/255.
Outcome: no `4 +`, `8 +`, `13 +` slot arithmetic outside `Prepared::slot_of` and `Slot`; no `usize::MAX` in `src/engine`; all engine tests and goldens unchanged.

**G3. `Tick` and `Ns`.** `Tick(f64)` with one `EPS` and named comparisons; `Ns(u64)`.
Outcome: the roughly 60 `1e-6`/`1e-9` literals in `src/engine` reduce to the one constant; `engine/chart.rs:352`'s `1.0` tolerance is either justified in a comment or brought in line; section-timing tests and `docs/section-timing.md` cases unchanged.

**G4. `SynthMsg`.** One decoder for the ring's non-MIDI messages (`click.rs:12`, `patches/route.rs:214,219`, `synth/xg_part.rs:21`, `synth/drum_setup.rs:62`), consumed by a `match` in `process`.
Outcome: a `const _: () = assert!` proves the byte ranges are disjoint; the `if ... continue` chain at `synth.rs:996-1057` is gone; `tests/synth_no_alloc.rs` unchanged.

## 4. Track H: duplication (Round 2 of the review, §Round 2 in `review-2026-09-27.md`)
Ranked by drift already observed. H1 needs an owner decision; the rest are mechanical.

**H1 (decision needed). The desktop app's Rust mock is a second engine.** `app/src-tauri/src/mock*.rs` is 5,316 lines re-implementing transport, sections, OTS, registration, playlist, chart, looper, sound library and pads; 44 of the last 60 commits to `src/session` also touched it, and it has already drifted (registration mock stores 6 of 9 groups; `StepStyle` implemented twice with different bounds; a different suggest-style algorithm; the looper name counter; several messages). The library's `Session::offline` runs the real engine with no threads, no audio and a virtual clock; the only missing piece is a style file, which T1's synthetic styles supply.
Recommended: replace `Backend::Mock` with `Session::offline(Options { paths: [demo style], data_dir: Some(tmp), .. })`, a ~60-line demo player that sends chords and section presses on a timer, and delete `mock*.rs`. Alternative if the owner prefers a smaller step: move the pure `AppState`-level logic the mock copies (`surface`, pad looks, harmony/arp, knobs, home tail, `sections_text`, the colour and part tables) into `src/api` so both call it; this removes 600 to 700 lines but leaves the engine copy.
Outcome (recommended path): `app/src-tauri/src/mock*.rs` deleted; `Backend` collapses to `Session`; `YAHAHA_MOCK`, no-styles and start-failure all run the offline session on the synthetic style; `cargo test` in `app/src-tauri` green; `docs/agents/wave-brief.md` rule 6 says "update the TS mock"; the two production Svelte imports from `app/src/lib/api/mock.ts` (`GM`, `noteName`) move to non-mock modules. Outcome (alternative): the eight listed copies gone from the mock; `mock.rs` under 2,500 lines; no colour, label or part-name literal in `app/src-tauri` that also exists in `launchkey.rs` or `parts.rs`.

**H2. One boundary policy for wire enums.** Eleven enums and five structs are defined twice with hand-written conversion (`HarmonySpeed`, `HarmonyAssign`, `ArpQuantize`, `ArpVelocityMode`, `HarmonyArpMode`, `LooperMode`, `PadLamp`, `StopAcmpMode`, `ChangeRuleMode`, `InsertEffect`, `FxType`; `TransportCmd`/`Button` 68 arms; `MultiPadCmd`/`PadCmd`; `MultiPadSynchroStop`, `StyleSettingsState`, `DynamicsState`, `PedalState`, `StyleChangeState`). About 28 others already follow "define once in the owner, derive serde, re-export from `api`". `InsertKind::None` currently reports as `Distortion` on the wire.
Outcome: no `match` or `From` between two enums with the same variant set anywhere in `src/api` or `src/session` (list the 16 in the PR body, each deleted or justified); `tests/api_wire.rs` byte-identical except where a bug is fixed (the insert `None` case gets its own wire value and a doc update); the wrong "plays the Main's fill" sentence exists nowhere; `knobs.rs` uses `engine::MIN_BPM/MAX_BPM`; one `MAX_BEND_RANGE` renamed; validation clamps live in the owner's setter only (`Parts::set_octave` added, eight scattered `clamp(-2, 2)` gone).

**H3. One per-channel MIDI state tracker.** `engine/mirror.rs` `Mirror::track`, `synth.rs` `Shadow`, `plugin/rack.rs` `Controllers`, `sff.rs` `parse_ots` and the local arrays in `engine/setup.rs` disagree on tracked CCs, CC121 reset, mono and bend storage.
Outcome: `midi::state::ChannelState { observe, selected, replay }` used by all five; one RPN replay loop; a table test that feeds the same message stream to the tracker and asserts the replay for each former caller's filter; `tests/synth_no_alloc.rs` and `plugin_rack_no_alloc.rs` unchanged.

**H4. `smf` and `fs_util` modules.** SMF reading (three MThd parsers, four chunk walks, three tempo decoders; `capture.rs`'s lacks the length and zero-tempo guards) and two production SMF writers; six atomic-write copies plus one plain `fs::write` (this is E1, folded here), the JSON load/save trios and `is_hidden`.
Outcome: `smf::read::{header, chunks, parse_track, tempo_us, timesig}` and `smf::write::{vlq, chunk, header, track, meta}` with `sff.rs`, `multipad/file.rs`, `multipad/synthetic.rs`, `capture.rs` and the test writer as callers; `fs_util::{write_atomic, read_json, check_format, is_hidden}`; `grep -rn 'fs::rename(&tmp' src` finds one site; the corrupt-SMF tests in `sff.rs` and `multipad/file.rs` now also run against the shared reader.

**H5. `midi::msg` helpers.** 119 status-byte constructions and about 60 parses; the "controllers to neutral" triple in four places with two orders; `BEND_CENTRE` twice; the 14-bit split five times.
Outcome: `midi::msg::{cc, note_on, note_off, pc, bend, bend_split, bend_join, status, channel, neutral_controls, rpn_select, xg_part_param, BEND_CENTRE}`; no `0x[89ABCDE]0 | ` left in `src/engine`, `src/controllers.rs`, `src/multipad`, `src/plugin` (grep in the PR body); `neutral_controls` sends in one documented order and the four former sites cite it; goldens unchanged.

**H6. One Launchkey lamp model.** `section_leds`/`section_looks` and `regist_leds`/`regist_looks` are the same state machine twice; `multipad_leds` already derives from looks.
Outcome: `section_leds` and `regist_leds` deleted; `led_of(look, palette)` derives every page; a pinning test asserts `pad_leds(page) == looks(page).map(led_of)` for pages 1 and 4 across a set of snapshots; `page_1_leds_unchanged_by_panel` still passes.

**H7. Small shared helpers, one PR each or grouped by module.** `cli::Args` for the five flag loops (and `--data-dir` stops accepting a missing value); `engine::timing::TickClock` for `transport.rs:128` and `multipad.rs:174` plus the four `60e9 / (bpm * ppq)` sites; `Engine::load_stopped` and `apply_setup_msg` for `send_init`/`reapply_init` and the three-line tail in `style_change.rs`; `KnobFn` spec table; `rt::lock` replacing 18 spelled-out poison-tolerant locks; `rt::Handoff` for the five ring pairs; `step_clamped`/`step_wrapping`; delete `multipad/player.rs::default_rule`; move `fingering.rs:270-274` chord-type consts into `theory` and drop the two redeclarations; `theory::note_index` replacing the two tables in `lib.rs`; one "Model 16" rule; `perf/top.rs` `row()`.
Outcome per helper: every caller listed in the review converted in the same PR; the old definitions gone; no behaviour change (goldens, no-alloc tests, `api_wire.rs` unchanged).

## 5. Rules for every agent in this wave
- A refactor PR body has three sections: **What moved**, **Proof of no change** (the tests and goldens that cover it, and the metric from §2 it moves), **Follow-ups** (anything noticed, not fixed).
- No PR mixes a bug fix with a refactor. Track A PRs change one function each.
- Do not "improve" behaviour while passing through: a clamp that looks wrong is a Finding on the board, not a change.
- When a shared helper is introduced, the PR that introduces it also converts every caller listed for it; a helper with one caller is not a helper.
- Wire format (`tests/api_wire.rs`) and `docs/app-api.md` do not change in this wave. If a type move would change JSON, stop and post NEED.
- Update both mocks only when a shared type moves; otherwise leave `app/` alone.

## 6. Definition of done for the wave
Every row in §2 at its target, `cargo fmt --check` and clippy in the documented gates, Track A merged with its six regression tests, and a final report listing each PR with its state (merged / READY / blocked) and the §2 table re-measured.
