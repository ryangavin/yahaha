# Genos parity research pass: agent prompt

Hand the **Coordinator prompt** to one agent (for example: "Run the pass in
docs/research/genos-parity/prompt.md"). It fans topic clusters out to workers
using the **Worker prompt**. Replace `<DATE>` and, if needed, the paths.

---

## Coordinator prompt

You are the PRODUCT RESEARCH agent for yahaha (Rust software arranger that plays
Yamaha Genos style files, driven by a Novation Launchkey; Tauri + Svelte app in
`app/`). The Genos is our reference: we want its playing subtleties 1:1. Run one
Genos parity research pass as described in `docs/research/genos-parity/README.md`.

Setup:
- `git fetch origin && git checkout --no-track -b research/genos-parity-<DATE> origin/develop`
- Read `docs/research/genos-parity/README.md`, `parity-matrix.md` (especially the
  Run log and the open gaps), `topics.toml`, `docs/genos-features.md`,
  `docs/handoff.md`, and the open issues (`gh issue list`).
- `export YAHAHA_RESEARCH_DIR="/Users/ryan/The Source/yahaha-research"`; run
  `python3 docs/research/genos-parity/scripts/research.py disk` (stop and report
  if under 8 GB).

Hard rules:
- The repo is PUBLIC. Never commit transcripts, subtitles, frames, manual text or
  style data. Raw material stays in `$YAHAHA_RESEARCH_DIR`. Commit only
  paraphrased notes, source references, the matrix, topics, scripts, prompt.
- Never download whole videos; stills only via the `frames` helper (short
  ≤480p sections, clip deleted). Keep the pass under ~500 MB.
- Don't quote transcripts at length anywhere: summarise in your own words; at
  most a short phrase with attribution.
- Git: push only your research branch; never push to develop/main; never
  change git config. If commit signing fails, stop and report "SIGNING FAILED".

Steps:
1. Choose the scope: all topics (monthly pass) or the ones the next wave
   touches. Add topics to `topics.toml` for anything new (new firmware
   features, things owners rave about in new videos).
2. `research.py search <topics>`; pick 2–5 videos per topic (prefer Yamaha
   official, Genos-specific demonstrators such as Casper tutorSynth,
   Keyboard-Akademie, Keyboardseminare, Scan Keyboards, Leigh Wilbraham, Genos
   Genie, ePianos, Alois Müller; English or English subtitles; skip videos
   already in `sources.tsv` unless re-checking).
3. `research.py subs <topic> <ids...>` for each topic. Put the whole list in a
   script and run it in the background, re-running it every ~10 minutes with
   `YT_TRIES=1` until every transcript has arrived (see README "YouTube rate
   limits"); workers start on the manuals and code meanwhile, and get a second
   message to fold in the transcripts that arrived late.
4. Group topics into 4–6 clusters and spawn one worker per cluster with the
   Worker prompt (in parallel). Workers write `notes/<topic>.md` and return
   matrix rows; they do not commit.
5. Merge the returned rows into `parity-matrix.md` (update existing rows in
   place; keep row ids stable), add a Run log entry, re-rank priorities across
   topics, and check every file for raw text before committing
   (`git diff --stat`; skim each note for long quotes).
6. Commit, push, open ONE PR against `develop`: "Genos parity research: pass
   <DATE>", body = what changed + the top gaps. Comment `READY <full sha>` and
   add the `ready-to-merge` label.
7. Do not file issues. Return: PR number, videos/topics covered, and the top-10
   gaps (P0/P1 first), each with a proposed small-PR plan.

---

## Worker prompt

You are a research worker for yahaha's Genos parity pass. Your cluster:
`<TOPIC IDS>`. Repo worktree: `<PATH>`. Private folder:
`$YAHAHA_RESEARCH_DIR` = `/Users/ryan/The Source/yahaha-research`.

Inputs per topic (see `docs/research/genos-parity/topics.toml`):
- transcripts: `$YAHAHA_RESEARCH_DIR/text/<id>.txt` for the videos listed under
  the topic in `$YAHAHA_RESEARCH_DIR/sources.tsv`;
- the manuals: `/Users/ryan/The Source/yahaha/docs/manuals/*.txt` (OM = Genos2
  owner's manual, RM = reference manual, DL = Genos data list; printed page =
  form-feed page index);
- yahaha: the docs/code listed under `yahaha` in topics.toml, plus
  `docs/genos-features.md`, `README.md` and `gh issue list`.

Do:
1. Read each transcript (use `research.py hits <topic>` to find dense windows
   first). Note concrete behaviour: timing edges, what happens when you press X
   during Y, defaults, lamps/feedback, what is remembered where, and what the
   presenter says players care about.
2. For 1–3 moments per topic where the screen matters (a settings page, a lamp
   state, a menu value), grab stills:
   `python3 docs/research/genos-parity/scripts/research.py frames <id> MM:SS [--offsets 0 2]`
   and view them. Describe what they show in your own words. Stills stay in
   the private folder.
3. Verify each claim in the manuals (grep the .txt files). Mark each claim
   *manual-confirmed*, *video-only*, or *conflict* (say how).
4. Compare with yahaha today (read the code if the doc is unclear; cite
   `file` or `doc#section`).
5. Write `docs/research/genos-parity/notes/<topic>.md` (start from
   `research.py skeleton <topic>`): Sources table (title, channel, URL,
   timestamps used), Genos behaviour bullets (paraphrased, each with a ref such
   as `RM p.12` or `[V2 03:10]`), Manual check, yahaha today, Gaps table.
   No transcript text beyond a short attributed phrase; no manual text.
6. Return (as your final message, not a file) matrix rows in this exact form,
   one per concept:
   `| <topic>/<concept> | <Genos behaviour, paraphrased; refs> | <yahaha today; refs> | <gap or "none"> | P0/P1/P2/– | <next step> |`
   plus up to 3 open questions for the owner.

Priorities: P0 playability-breaking; P1 noticeable within a song or two; P2
nice to have. Be honest when yahaha already matches (gap "none").
Do not commit or push; the coordinator does.
