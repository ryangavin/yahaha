# yahaha

A software arranger keyboard. It loads Yamaha Genos/PSR/Tyros style files (`.sty .prs .sst ...`) and follows the chords you play on a MIDI keyboard. The band plays through a built-in SoundFont synth, and also out of a virtual MIDI port called **yahaha** so you can use your own sounds in Ableton.

## Setup

yahaha doesn't ship any styles or sounds. You add two things yourself (both folders are git-ignored):
- **Styles:** put `.sty/.prs/.sst` files in `corpus/`. Free ones are available from Yamaha, PSR Tutorial, and Sand, Software and Sound. Encrypted Expansion Packs (`.cpi`/`.ppi`) are not supported.
- **SoundFont (optional):** put a General MIDI `.sf2` in `soundfonts/`, for example [GeneralUser GS](https://github.com/mrbumpy409/GeneralUser-GS).

macOS only; it uses CoreMIDI and CoreAudio directly.

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

The built-in synth uses the first `.sf2` file in `soundfonts/` (GeneralUser GS, downloaded separately; it's not in git). It plays on your default audio output with a 64-frame buffer (about 1.3 ms at 48 kHz).
- Your right hand plays a piano by default. Change its voice with `9` and `0`.
- `l` makes your left-hand chord notes sound too.
- `k` mutes the synth, for example when you're using Ableton sounds instead.
- The synth plays on outputs 11/12 when the audio device is a TASCAM Model 16, and on 1/2 otherwise. `a` steps through the output pairs while playing, and `--audio-out 11` sets the pair at launch.

Options:
- `--sf2 file` uses a different SoundFont.
- `--audio-out N` sends the synth to outputs N/N+1.
- `--no-synth` turns the synth off, leaving MIDI out only.
- `--palette-leds` uses the Launchkey's built-in palette colours instead of RGB SysEx.
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

The screen shows a live map of the pads, in the same colours as the hardware:
- **dim**: the section is available
- **bright**: playing, or the feature is on
- **flashing**: queued; takes over at the next bar (fills at the next beat)
- **pulsing**: armed and waiting for you
- **dark**: this style doesn't have it

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

## License

MIT. Yamaha, Genos, PSR, and Tyros are trademarks of Yamaha Corporation. yahaha is an independent project, not affiliated with Yamaha; it reads the publicly documented style file format for interoperability.
