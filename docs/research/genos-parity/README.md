# Genos parity research

A repeatable way to check that yahaha still plays like a Genos, and to find the
subtleties we have not matched yet. Each pass looks at how the Genos behaves in
tutorial and demo videos, checks it against the Genos2 manuals, compares it with
what yahaha does today, and updates [parity-matrix.md](parity-matrix.md).

| File | What it is |
|---|---|
| [topics.toml](topics.toml) | The concept list: search queries, transcript keywords, manual pages, yahaha docs/code per topic |
| [scripts/research.py](scripts/research.py) | Helpers: YouTube search, subtitles, keyword timestamps, stills, notes skeletons |
| [prompt.md](prompt.md) | The agent prompt for a pass (coordinator and per-cluster workers) |
| [parity-matrix.md](parity-matrix.md) | The living comparison table, with a dated run log |
| [notes/](notes/) | One paraphrased note per topic |

## The public/private rule

The repo is public. **Nothing raw is committed**: no transcripts, subtitles,
video frames, manual text or style data. Raw material lives only in the private
folder, `$YAHAHA_RESEARCH_DIR` (default: `yahaha-research/` next to the main
checkout, i.e. `../yahaha-research` from the repo root). The scripts refuse a
private folder inside the repo.

What is committed: our own paraphrased notes (at most a short attributed
phrase), source references (title, channel, URL, timestamp), the matrix, the
topic list, the scripts and the prompt.

Private folder layout:

```
yahaha-research/
  search/<topic>.tsv    search candidates (id, duration, channel, title, query)
  subs/                 raw .srt subtitles from yt-dlp
  text/<id>.txt         de-duplicated transcript, one "[MM:SS] line" per cue
  hits/<topic>.txt      densest keyword windows per video (candidate timestamps)
  frames/<id>_<MMSS>_<k>.jpg   stills (the short clips are deleted after extraction)
  sources.tsv           every video used: topic, id, channel, title, url, duration
  runs/<date>.md        the pass's scratch log (optional)
```

## Running a pass

Needs `python3` ≥ 3.11, `yt-dlp`, `ffmpeg`, `gh`. Everything below runs from
the repo root.

```bash
export YAHAHA_RESEARCH_DIR=../yahaha-research
R=docs/research/genos-parity/scripts/research.py

python3 $R disk                       # stop if under 8 GB free
python3 $R topics                     # * marks the playing-feel topics
python3 $R search                     # all topics (or: search fingering ots)
#   read search/<topic>.tsv, pick 2-5 videos per topic: Yamaha official,
#   Genos-specific demonstrators, English (or with English subtitles)
python3 $R subs fingering W2phsGwMgoA GstkJE7Kl6Y
python3 $R hits fingering             # candidate timestamps per video
#   read text/<id>.txt around the hits; decide which moments show UI or state
python3 $R frames W2phsGwMgoA 02:10 03:45 --offsets 0 1.5
#   view the stills (an agent can read images) to confirm what the screen shows
python3 $R skeleton fingering         # notes/fingering.md, if it doesn't exist
```

Then, per topic: verify each claim against the manuals
(`docs/manuals/*.txt`, printed page = form-feed page), compare with the yahaha
doc/code listed in `topics.toml`, fill the note, and add or update rows in
`parity-matrix.md`. Where a video and the manual disagree, say so in the note and
the matrix (manual wins for the spec; the video is evidence of real behaviour
and goes to the owner as a question).

The easy way is to hand [prompt.md](prompt.md) to an agent: it runs the whole
pass, fans topics out to worker agents, and opens the PR.

Budgets: subtitles are tiny; stills are fetched as a few seconds of ≤480p video
each and the clip is deleted at once. Keep a pass under ~500 MB of downloads,
never download whole videos, and stop if the disk drops under 8 GB free.

### YouTube rate limits

YouTube rate-limits the caption endpoint (HTTP 429) after a burst of downloads;
the first pass hit it after about ten videos and it came and went for hours.
What helps:
- `subs` asks for one English track per video (asking for every `en.*` track
  multiplies requests), pauses between videos (`--pause`), and skips videos
  already fetched, so re-running a list is cheap.
- A yt-dlp with browser impersonation in a private venv:
  `python3 -m venv "$YAHAHA_RESEARCH_DIR/.venv" && "$YAHAHA_RESEARCH_DIR/.venv/bin/pip" install "yt-dlp[default,curl-cffi]"`,
  then `export YT_DLP="$YAHAHA_RESEARCH_DIR/.venv/bin/yt-dlp"`.
- `YT_TRIES=1` fails fast instead of backing off, for a loop that re-runs the
  whole list every 10 minutes in the background while the workers read what
  has arrived.
- Start fetching early (before planning the notes), and fetch the playing-feel
  topics first.

## Cadence

- **Before each wave** (when the coordinator plans the next milestone): a
  focused pass on the topics the wave touches, so the issues start from the
  latest gaps.
- **Monthly**, or after a Genos firmware update or a new Yamaha arranger
  release: a breadth pass over all topics, looking for new videos (search
  results change) and re-checking the P0/P1 rows.
- **After a big merge to `main`**: re-check the rows that merge claims to close,
  update "yahaha today", and move closed gaps to Done.

A pass takes one agent session. Record it in the matrix's Run log (date, topics,
videos, what changed).

## How results feed issues

1. The pass ends with a PR that updates the matrix and notes (docs only), and a
   top-10 gap list (P0/P1 first) with a small-PR plan each, in the PR body.
2. The coordinator reviews the list with the owner. Only then are issues filed,
   one per agreed gap, linking the matrix row and note. The issue number goes in
   the matrix's "Next step" column.
3. The implementation PR closes the issue and updates the matrix row's "yahaha
   today" and Gap columns (or the next pass does).

Priorities:
- **P0**: playability-breaking; a Genos player would stop or be thrown mid-song.
- **P1**: noticeable; a Genos player notices it within a song or two.
- **P2**: nice to have; power users or rare cases.
