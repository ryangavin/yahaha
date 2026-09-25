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
    out, last = [], ""
    t = 0
    for block in re.split(r"\n\s*\n", srt.read_text(errors="replace")):
        lines = [l.strip() for l in block.strip().splitlines()]
        for i, l in enumerate(lines):
            m = TS.match(l)
            if m:
                h, mi, s, _ = map(int, m.groups())
                t = h * 3600 + mi * 60 + s
                text = " ".join(x for x in lines[i + 1:] if x)
                text = re.sub(r"<[^>]+>", "", text).strip()
                for piece in text.split("  "):
                    piece = piece.strip()
                    if piece and piece != last and not last.endswith(piece):
                        # auto-subs repeat the previous line as the first line of the next cue
                        if piece.startswith(last) and last:
                            piece = piece[len(last):].strip()
                        if piece:
                            out.append((t, piece))
                            last = text
                break
    return out


def mmss(s: int) -> str:
    return f"{s // 60:02d}:{s % 60:02d}"


def cmd_subs(args):
    p = private_dir(args)
    check_disk(args)
    sources = p / "sources.tsv"
    new = not sources.exists()
    with open(sources, "a", newline="") as f:
        w = csv.writer(f, delimiter="\t")
        if new:
            w.writerow(["topic", "id", "channel", "title", "url", "duration_s"])
        for v in args.videos:
            vid = vid_of(v)
            url = f"https://www.youtube.com/watch?v={vid}"
            meta = run(["yt-dlp", "--skip-download", "--print",
                        "%(channel)s\t%(title)s\t%(duration)s", url]).stdout.strip().split("\t")
            r = run(["yt-dlp", "--skip-download", "--write-auto-subs", "--write-subs",
                     "--sub-langs", "en.*", "--convert-subs", "srt",
                     "-o", str(p / "subs" / "%(id)s.%(ext)s"), url])
            srts = sorted(glob.glob(str(p / "subs" / f"{vid}*.srt")),
                          key=lambda s: ("orig" in s, "-" in Path(s).stem.split(".")[-1]))
            if not srts:
                print(f"{vid}: no English subtitles ({r.stderr.strip()[-200:]})")
                continue
            # prefer a manual 'en' track, else the auto one
            lines = srt_to_text(Path(srts[0]))
            txt = p / "text" / f"{vid}.txt"
            with open(txt, "w") as tf:
                tf.write(f"# {meta[1] if len(meta) > 1 else vid} | {meta[0]} | {url}\n")
                for t, line in lines:
                    tf.write(f"[{mmss(t)}] {line}\n")
            w.writerow([args.topic, vid, *(meta + ["", "", ""])[:2], url, (meta + ["", "", ""])[2]])
            print(f"{vid}: {len(lines)} lines -> {txt}")


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
    s = sp.add_parser("hits"); s.add_argument("topic"); s.add_argument("videos", nargs="*"); s.add_argument("--top", type=int, default=8)
    s = sp.add_parser("frames"); s.add_argument("video"); s.add_argument("times", nargs="+")
    s.add_argument("--span", type=int, default=3, help="seconds after the timestamp to download")
    s.add_argument("--height", type=int, default=480)
    s.add_argument("--offsets", type=float, nargs="+", default=[0.0], help="stills at these seconds after the timestamp")
    s = sp.add_parser("skeleton"); s.add_argument("topic"); s.add_argument("--force", action="store_true")
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
     "skeleton": cmd_skeleton}[args.cmd](args)


if __name__ == "__main__":
    main()
