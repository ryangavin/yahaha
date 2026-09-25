#!/usr/bin/env python3
"""Genos parity research helpers.

Raw material (search results, subtitles, transcripts, stills) goes ONLY to the
private folder, never into the repo (the repo is public). The private folder is
$YAHAHA_RESEARCH_DIR, or --dir, default ../yahaha-research next to the repo.

Subcommands (run from anywhere):
  search  [topic ...] [--n 8]     YouTube search per topic query -> search/<topic>.tsv
  subs    <topic> <id|url> ...    fetch English subtitles -> subs/, text/<id>.txt, sources.tsv
  hits    <topic> [id ...]        keyword hits with timestamps -> hits/<topic>.txt
  frames  <id> <MM:SS> ...        grab one still per timestamp -> frames/ (clips deleted)
  skeleton <topic>                write notes/<topic>.md in the repo if missing
  retext                          rebuild text/*.txt from the saved .srt files (offline)
  disk                            show free space (the scripts stop under --min-free-gb)
  topics                          list topic ids

Needs: python3 >= 3.11 (tomllib), yt-dlp, ffmpeg.
"""

import argparse
import csv
import glob
import os
import re
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
PASS_DIR = HERE.parent  # docs/research/genos-parity
REPO = PASS_DIR.parents[2]
TOPICS = PASS_DIR / "topics.toml"


def main_checkout() -> Path:
    """The main checkout (not a worktree under .claude/worktrees)."""
    r = run(["git", "-C", str(REPO), "rev-parse", "--path-format=absolute", "--git-common-dir"])
    return Path(r.stdout.strip()).parent if r.returncode == 0 else REPO


def private_dir(args) -> Path:
    d = args.dir or os.environ.get("YAHAHA_RESEARCH_DIR") or str(main_checkout().parent / "yahaha-research")
    p = Path(d).expanduser().resolve()
    for root in {REPO, main_checkout()}:
        if root in p.parents or p == root:
            sys.exit(f"refusing: private dir {p} is inside the public repo")
    for sub in ("search", "subs", "text", "hits", "frames", "tmp"):
        (p / sub).mkdir(parents=True, exist_ok=True)
    return p


def load_topics() -> dict:
    with open(TOPICS, "rb") as f:
        data = tomllib.load(f)
    return {t["id"]: t for t in data["topic"]}


def free_gb() -> float:
    # The data volume on macOS; falls back to the home directory elsewhere.
    target = "/System/Volumes/Data" if os.path.exists("/System/Volumes/Data") else str(Path.home())
    return shutil.disk_usage(target).free / 1e9


def check_disk(args):
    gb = free_gb()
    if gb < args.min_free_gb:
        sys.exit(f"STOP: only {gb:.1f} GB free (< {args.min_free_gb} GB). Free space before continuing.")
    return gb


def run(cmd, **kw):
    return subprocess.run(cmd, text=True, capture_output=True, **kw)


def vid_of(s: str) -> str:
    m = re.search(r"(?:v=|youtu\.be/|shorts/)([A-Za-z0-9_-]{11})", s)
    return m.group(1) if m else s


# ---------------------------------------------------------------- search
def cmd_search(args):
    p = private_dir(args)
    topics = load_topics()
    ids = args.topics or list(topics)
    for tid in ids:
        t = topics[tid]
        rows, seen = [], set()
        for q in t.get("queries", []):
            r = run(["yt-dlp", f"ytsearch{args.n}:{q}", "--flat-playlist", "--print",
                     "%(id)s\t%(duration)s\t%(channel)s\t%(title)s"])
            for line in r.stdout.splitlines():
                parts = line.split("\t")
                if len(parts) != 4 or parts[0] in seen:
                    continue
                seen.add(parts[0])
                rows.append(parts + [q])
        out = p / "search" / f"{tid}.tsv"
        with open(out, "w", newline="") as f:
            w = csv.writer(f, delimiter="\t")
            w.writerow(["id", "duration_s", "channel", "title", "query"])
            w.writerows(rows)
        print(f"{tid}: {len(rows)} candidates -> {out}")


# ---------------------------------------------------------------- subs
TS = re.compile(r"(\d+):(\d+):(\d+)[,.](\d+)\s*-->")


def srt_to_text(srt: Path) -> list[tuple[int, str]]:
    """SRT -> [(seconds, line)] with the rolling duplicates of auto-subs removed."""
    # Auto-subs roll: each cue repeats the previous cue's line above the new
    # one, so a line is emitted only the first time it appears.
    out, recent = [], []
    for block in re.split(r"\n\s*\n", srt.read_text(errors="replace")):
        lines = [l.strip() for l in block.strip().splitlines()]
        for i, l in enumerate(lines):
            m = TS.match(l)
            if not m:
                continue
            h, mi, s, _ = map(int, m.groups())
            t = h * 3600 + mi * 60 + s
            for x in lines[i + 1:]:
                x = re.sub(r"<[^>]+>", "", x).strip()
                if x and x not in recent:
                    out.append((t, x))
                    recent = (recent + [x])[-3:]
            break
    return paragraphs(out)


def paragraphs(lines, span=12):
    """Join short caption lines into one line per ~span seconds (easier to read)."""
    out = []
    for t, x in lines:
        if out and t - out[-1][0] < span:
            out[-1] = (out[-1][0], out[-1][1] + " " + x)
        else:
            out.append((t, x))
    return out


def mmss(s: int) -> str:
    return f"{s // 60:02d}:{s % 60:02d}"


def pick_track(info: dict):
    """One English caption track: manual en*, else auto en-orig/en, else auto
    English translated from the video's language. Returns (lang, is_auto)."""
    subs = info.get("subtitles") or {}
    auto = info.get("automatic_captions") or {}
    for lang in ("en", "en-GB", "en-US", *sorted(k for k in subs if k.startswith("en"))):
        if lang in subs:
            return lang, False
    for lang in ("en-orig", "en"):
        if lang in auto:
            return lang, True
    src = (info.get("language") or "").split("-")[0]
    for lang in (f"en-{src}", *sorted(k for k in auto if k.startswith("en-"))):
        if lang in auto:
            return lang, True
    return None


def yt(cmd, tries=6):
    """yt-dlp with a polite pause between requests and backoff on HTTP 429.
    YouTube's caption endpoint rate-limits bursts (a pass of ~70 videos hit it);
    the limit clears after some minutes, so back off 1, 2, 3... minutes."""
    import time
    r = None
    for i in range(tries):
        r = run(["yt-dlp", "--sleep-requests", "1", *cmd])
        if "429" not in r.stderr:
            return r
        print(f"  429 rate limit, waiting {60 * (i + 1)} s", flush=True)
        time.sleep(60 * (i + 1))
    return r


def cmd_subs(args):
    """One English track per video (asking for every en.* track trips YouTube's rate limit)."""
    import json
    import time
    p = private_dir(args)
    check_disk(args)
    sources = p / "sources.tsv"
    known = set()
    if sources.exists():
        with open(sources) as f:
            known = {(r["topic"], r["id"]) for r in csv.DictReader(f, delimiter="\t")}
    new = not sources.exists()
    with open(sources, "a", newline="") as f:
        w = csv.writer(f, delimiter="\t")
        if new:
            w.writerow(["topic", "id", "channel", "title", "url", "duration_s"])
        for v in args.videos:
            vid = vid_of(v)
            url = f"https://www.youtube.com/watch?v={vid}"
            txt = p / "text" / f"{vid}.txt"
            if txt.exists() and not args.force:
                if (args.topic, vid) not in known:
                    head = txt.read_text().splitlines()[0].lstrip("# ").split(" | ")
                    w.writerow([args.topic, vid, head[1] if len(head) > 1 else "", head[0], url, ""])
                print(f"{vid}: already fetched -> {txt}")
                continue
            r = yt(["-J", "--skip-download", url])
            try:
                info = json.loads(r.stdout)
            except json.JSONDecodeError:
                print(f"{vid}: metadata failed: {r.stderr.strip()[-200:]}")
                continue
            track = pick_track(info)
            if not track:
                print(f"{vid}: no English captions (language {info.get('language')})")
                continue
            lang, is_auto = track
            for old in glob.glob(str(p / "subs" / f"{vid}.*")):
                os.remove(old)
            r = yt(["--skip-download", "--write-auto-subs" if is_auto else "--write-subs",
                    "--sub-langs", lang, "--convert-subs", "srt",
                    "-o", str(p / "subs" / "%(id)s.%(ext)s"), url])
            srts = glob.glob(str(p / "subs" / f"{vid}*.srt"))
            if not srts:
                print(f"{vid}: subtitle download failed ({r.stderr.strip()[-200:]})")
                continue
            lines = srt_to_text(Path(srts[0]))
            title, channel = info.get("title", vid), info.get("channel", "")
            with open(txt, "w") as tf:
                tf.write(f"# {title} | {channel} | {url} | track {lang}{' (auto)' if is_auto else ''}\n")
                for t, line in lines:
                    tf.write(f"[{mmss(t)}] {line}\n")
            if (args.topic, vid) not in known:
                w.writerow([args.topic, vid, channel, title, url, info.get("duration", "")])
            f.flush()
            print(f"{vid}: {len(lines)} lines ({lang}{', auto' if is_auto else ''}) -> {txt}")
            time.sleep(args.pause)


def cmd_retext(args):
    """Rebuild text/<id>.txt from the saved .srt (no network), e.g. after a parser fix."""
    p = private_dir(args)
    for txt in sorted((p / "text").glob("*.txt")):
        srts = glob.glob(str(p / "subs" / f"{txt.stem}*.srt"))
        if not srts:
            continue
        head = txt.read_text().splitlines()[0]
        lines = srt_to_text(Path(srts[0]))
        txt.write_text(head + "\n" + "".join(f"[{mmss(t)}] {l}\n" for t, l in lines))
        print(f"{txt.stem}: {len(lines)} lines")


# ---------------------------------------------------------------- hits
def cmd_hits(args):
    p = private_dir(args)
    t = load_topics()[args.topic]
    kws = [k.lower() for k in t.get("keywords", [])]
    vids = args.videos
    if not vids:
        with open(p / "sources.tsv") as f:
            vids = [r["id"] for r in csv.DictReader(f, delimiter="\t") if r["topic"] == args.topic]
    out = p / "hits" / f"{args.topic}.txt"
    with open(out, "w") as o:
        for vid in vids:
            txt = p / "text" / f"{vid_of(vid)}.txt"
            if not txt.exists():
                continue
            lines = txt.read_text().splitlines()
            o.write(lines[0] + "\n")
            # score each 30 s window by keyword hits; list the densest windows
            buckets: dict[int, list[str]] = {}
            for l in lines[1:]:
                m = re.match(r"\[(\d+):(\d+)\] (.*)", l)
                if not m:
                    continue
                s = int(m.group(1)) * 60 + int(m.group(2))
                found = [k for k in kws if k in m.group(3).lower()]
                if found:
                    buckets.setdefault(s // 30 * 30, []).extend(found)
            best = sorted(buckets.items(), key=lambda kv: -len(kv[1]))[: args.top]
            for s, found in sorted(best):
                o.write(f"  {mmss(s)}  {len(found):2d} hits  {', '.join(sorted(set(found)))}\n")
    print(out.read_text())


# ---------------------------------------------------------------- frames
def to_s(ts: str) -> int:
    parts = [int(x) for x in ts.split(":")]
    s = 0
    for x in parts:
        s = s * 60 + x
    return s


def cmd_frames(args):
    p = private_dir(args)
    vid = vid_of(args.video)
    url = f"https://www.youtube.com/watch?v={vid}"
    for ts in args.times:
        check_disk(args)
        s = to_s(ts)
        a, b = max(0, s - 1), s + args.span
        clip_base = p / "tmp" / f"{vid}_{s}"
        r = run(["yt-dlp", "-f", f"bv*[height<={args.height}]/b[height<={args.height}]/wv*",
                 "--download-sections", f"*{mmss(a)}-{mmss(b)}", "--force-keyframes-at-cuts",
                 "-o", str(clip_base) + ".%(ext)s", url])
        clips = glob.glob(str(clip_base) + ".*")
        if not clips:
            print(f"{vid} {ts}: download failed: {r.stderr.strip()[-300:]}")
            continue
        for k, off in enumerate(args.offsets):
            still = p / "frames" / f"{vid}_{mmss(s).replace(':', '')}_{k}.jpg"
            run(["ffmpeg", "-y", "-loglevel", "error", "-ss", str(1 + off), "-i", clips[0],
                 "-frames:v", "1", "-q:v", "3", str(still)])
            if still.exists():
                print(still)
        for c in clips:
            os.remove(c)  # keep only the stills


# ---------------------------------------------------------------- skeleton
SKELETON = """# {title}

Topic id: `{id}` · Pass: {date} · Manual: {manual} · yahaha: {yahaha}

Paraphrased notes only. No transcript text, manual text or frames are committed.

## Sources

| # | Video (channel) | URL | Timestamps used |
|---|---|---|---|

## Genos behaviour (subtleties a player notices)

- ...

## Manual check

- Agrees / disagrees with the videos: ...

## yahaha today

- ...

## Gaps

| Gap | Priority | Suggested next step |
|---|---|---|
"""


def cmd_skeleton(args):
    import datetime
    t = load_topics()[args.topic]
    out = PASS_DIR / "notes" / f"{args.topic}.md"
    if out.exists() and not args.force:
        print(f"exists: {out}")
        return
    out.parent.mkdir(exist_ok=True)
    out.write_text(SKELETON.format(
        title=t["title"], id=t["id"], date=datetime.date.today().isoformat(),
        manual=", ".join(t.get("manual", [])) or "-",
        yahaha=", ".join(f"`{y}`" for y in t.get("yahaha", [])) or "-"))
    print(out)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--dir", help="private research folder (default $YAHAHA_RESEARCH_DIR or ../yahaha-research)")
    ap.add_argument("--min-free-gb", type=float, default=8.0)
    sp = ap.add_subparsers(dest="cmd", required=True)
    s = sp.add_parser("search"); s.add_argument("topics", nargs="*"); s.add_argument("--n", type=int, default=8)
    s = sp.add_parser("subs"); s.add_argument("topic"); s.add_argument("videos", nargs="+")
    s.add_argument("--force", action="store_true", help="fetch again even if text/<id>.txt exists")
    s.add_argument("--pause", type=float, default=4.0, help="seconds between videos (YouTube rate-limits)")
    s = sp.add_parser("hits"); s.add_argument("topic"); s.add_argument("videos", nargs="*"); s.add_argument("--top", type=int, default=8)
    s = sp.add_parser("frames"); s.add_argument("video"); s.add_argument("times", nargs="+")
    s.add_argument("--span", type=int, default=3, help="seconds after the timestamp to download")
    s.add_argument("--height", type=int, default=480)
    s.add_argument("--offsets", type=float, nargs="+", default=[0.0], help="stills at these seconds after the timestamp")
    s = sp.add_parser("skeleton"); s.add_argument("topic"); s.add_argument("--force", action="store_true")
    sp.add_parser("retext")
    sp.add_parser("disk")
    sp.add_parser("topics")
    args = ap.parse_args()
    if args.cmd == "disk":
        print(f"{free_gb():.1f} GB free"); return
    if args.cmd == "topics":
        for tid, t in load_topics().items():
            print(f"{tid:18s} {'*' if t.get('feel') else ' '} {t['title']}")
        return
    {"search": cmd_search, "subs": cmd_subs, "hits": cmd_hits, "frames": cmd_frames,
     "skeleton": cmd_skeleton, "retext": cmd_retext}[args.cmd](args)


if __name__ == "__main__":
    main()
