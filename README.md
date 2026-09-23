# yahaha

A software arranger keyboard. It loads Yamaha Genos/PSR/Tyros style files (`.sty .prs .sst ...`), follows the chords you play on a MIDI keyboard, and plays the band out of a virtual MIDI port called **yahaha**. You pick the sounds in Ableton (or any synth).

## Run

```bash
cargo build --release
./target/release/yahaha play corpus/          # a style file or a folder of them
```

On launch:
- It connects to your Launchkey's keys.
- It puts the Launchkey into DAW mode so the pads become arranger buttons, and restores standalone mode on exit.
- It waits for your first chord (**Sync Start**).

Play a chord left of **F#2** (Yamaha numbering, C3 = middle C) and the band starts.

Options:
- `--split C3` moves the split point. You can also use `[` and `]` while playing.
- `--input "Name"` picks MIDI sources by name.
- `--all-inputs` merges every connected keyboard.
- `--no-pads` leaves the Launchkey pads alone.

## Ableton setup (once)

1. **Preferences → Link, Tempo & MIDI.**
   - Set the Launchkey *Control Surface* to **None** while jamming, because yahaha owns the pads.
   - Turn **Track** input **on** for `yahaha`.
   - Turn **Track** input **off** for the Launchkey. Your playing reaches Ableton through yahaha, so leaving it on doubles every note.
2. Create one MIDI track per channel, each with *MIDI From* `yahaha`:

   | ch | part | ch | part |
   |---|---|---|---|
   | 1 | your right hand | 11 | Bass |
   | 2 | your left hand (chord zone) | 12 | Chord 1 |
   | 9 | Rhythm 1 (sub drums) | 13 | Chord 2 |
   | 10 | Rhythm 2 (main drums) | 14 | Pad |
   | | | 15 | Phrase 1 |
   | | | 16 | Phrase 2 |

   The screen shows which Yamaha voice each part was written for (e.g. "≈ Finger Bass"), so you know what sound to load. Drum parts use the GM drum map.
3. Arm the tracks, or set Monitor to *In*.

## Controls

| Launchkey pad | top row | bottom row |
|---|---|---|
| 1–3 | Intro I–III | Main A–C |
| 4 | Sync Start | Main D |
| 5–7 | Ending I–III | Break · Tap tempo · Sync Stop |
| 8 | Auto Fill | Start/Stop |

- **Play** button: start/stop.
- **Stop** button: stop.
- Right-side arrows: tempo +/−.
- Pressing the current Main again plays its fill. With Auto Fill on, switching Main plays a fill into the new one.

Terminal keys:
- `space` start/stop
- `1-4` Main A–D
- `q w e` Intro I–III
- `i o p` Ending I–III
- `g` break
- `t` tap tempo
- `- =` tempo down/up
- `y` Sync Start · `u` Auto Fill · `j` Sync Stop
- `z…,` mute/unmute parts
- `←/→` next/previous style
- `!` panic (all notes off)
- `esc` quit

Chords are recognized in "Fingered On Bass" style, plus some shortcuts:
- one key = major
- two keys = power chord, major/minor third, or 7th
- the lowest note becomes a slash bass
- three adjacent keys = chord cancel (drums only)

## Architecture

```
CoreMIDI receive thread ──chord (AtomicU32) + commands (SPSC ring) + semaphore──▶ engine thread (Mach real-time)
UI thread (terminal, LEDs, style loading) ──styles/commands (SPSC)──────────────▶ engine thread
engine thread ──snapshots, old styles (SPSC)──▶ UI thread
```

- **Input thread** (CoreMIDI's own receive thread, running our callback):
  - forwards your notes straight to the output (right hand ch 1, left hand ch 2)
  - recognizes chords with a precomputed table of 4096×12 entries
  - publishes the result without locking
- **Engine thread**:
  - sleeps on a Mach semaphore until the next pattern event or an input signal, whichever comes first
  - spins the last 150 µs to hit the deadline exactly
  - never locks, allocates, or does I/O
- New styles are prepared on the UI thread, swapped in by pointer, and freed back on the UI thread.
- Every note is transposed at the moment it plays, and there is no lookahead. A chord change therefore reaches the very next note, and notes already sounding are re-pitched according to the style's retrigger rule. A note that started less than 40 ms before the chord arrived is corrected outright.

`yahaha bench <style>` measures the real path through CoreMIDI (M-series Mac, 2026-09):

| path | p50 | p99 |
|---|---|---|
| keyboard → passthrough out | 67 µs | 190 µs |
| chord → first band note (sync start) | 143 µs | 235 µs |
| chord published → engine applied | <15 µs | <28 µs |
| engine timing error vs. schedule | <1 µs | <1 µs |

## Other commands

- `yahaha dump <style>`: sections, channels, and CASM rules.
- `yahaha sim <style> "C Am7 F G7"`: offline render, one chord per bar, printed per part.
- `yahaha bench <style> [spin_us]`: latency benchmark using virtual ports.
- `yahaha drive`: fake keyboard for testing against a running `yahaha play --input TestKbd`.

Tests: run `cargo test --release`. It covers the spec's transposition examples, chord recognition, and a full performance of every style in `corpus/`, checking for stuck notes.

## Known gaps

- The NTT transposition tables are reconstructed from documentation and have not yet been checked against a real Genos (see PLAN.md §4).
- Not done yet: ritardando on a second Ending press, Ableton Link, audio styles, OTS voice changes, and the Ctb2 bytes that are still undocumented.
