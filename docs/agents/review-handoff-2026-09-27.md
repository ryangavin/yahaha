# Orchestrator handoff: Rust idiom and maintainability review (2026-09-27)

You are the ORCHESTRATOR for a code-quality wave. This page gives you the findings, the tracks to spawn, and, for every PR, the OUTCOME you validate against. The full review with every file:line reference is `docs/agents/review-2026-09-27.md`; read it once, then work from this page.

## 0. Read first
- `docs/agents/wave-brief.md` and `docs/agents/wave2/wave2-brief.md`: the hard rules (small PRs, gates, READY handoff, both mocks, no copyrighted data). They all apply here.
- `docs/architecture.md`: the thread model and the add-a-feature checklist. Refactors must not move work across threads.
- `docs/agents/review-2026-09-27.md`: the findings this page is built from. Round 1 covers idiom, single responsibility and error handling; Round 2 covers duplication.

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
Each track is a chain of small PRs. Spawn one agent per track; tracks A, C and D are independent of everything and can start at once. Order inside a track matters where marked.

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

### Track D: test scaffolding (two PRs; the Round 2 section refines D1)
**D1. `testkit`.** One `#[cfg(test)] pub(crate) mod testkit` (`src/testkit.rs`) holding the corpus path helpers, `offline()`/`session()`, `Rec`/`Nop` sinks, the stepping helpers and `bar_ns`, plus `tests/common/mod.rs` for the nine `*_no_alloc.rs` counting allocators.
Outcome: `SlowWalker.T552` appears in one file; "corpus missing" appears once, as a macro or helper that marks the test skipped visibly (the agent proposes `#[ignore]`-by-feature or a printed SKIP count; the owner's rule "the corpus actually runs" in the gates stays true on the owner's machine); `fn bar_ns` defined once; test count unchanged (`cargo test -- --list | wc -l` before and after in the PR body).

**D2. Extract inline test giants.** `src/sim.rs` (seven inline modules, 2602 lines), `src/live.rs` (1588), `src/session/fx.rs`, `src/session/style_change.rs` (its tests are OTS-Link tests and move under `ots`), `src/session/controllers.rs`, `src/synth.rs`, `src/theory.rs`, `src/capture.rs`, `src/sff.rs`.
Outcome: no production file carries an inline test module over 100 lines; `#[cfg(test)] #[path = "..._tests.rs"]` used as the existing convention; `git diff --stat` shows moves (`-M` detects them); test count unchanged.

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

## 4. Round 2: duplication
See §8 of `docs/agents/review-2026-09-27.md`. (Filled in below once the duplication pass completes.)

## 5. Rules for every agent in this wave
- A refactor PR body has three sections: **What moved**, **Proof of no change** (the tests and goldens that cover it, and the metric from §2 it moves), **Follow-ups** (anything noticed, not fixed).
- No PR mixes a bug fix with a refactor. Track A PRs change one function each.
- Do not "improve" behaviour while passing through: a clamp that looks wrong is a Finding on the board, not a change.
- When a shared helper is introduced, the PR that introduces it also converts every caller listed for it; a helper with one caller is not a helper.
- Wire format (`tests/api_wire.rs`) and `docs/app-api.md` do not change in this wave. If a type move would change JSON, stop and post NEED.
- Update both mocks only when a shared type moves; otherwise leave `app/` alone.

## 6. Definition of done for the wave
Every row in §2 at its target, `cargo fmt --check` and clippy in the documented gates, Track A merged with its six regression tests, and a final report listing each PR with its state (merged / READY / blocked) and the §2 table re-measured.
