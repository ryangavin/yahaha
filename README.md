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
./target/release/yahaha play corpus/          # a style file or a folder of them (searched recursively)
```

On launch:
- It connects to your Launchkey's keys.
- It puts the Launchkey into DAW mode so the pads become arranger buttons, and restores standalone mode on exit.
- It waits for your first chord (**Sync Start**).

Play a chord left of **F#2** (Yamaha numbering, C3 = middle C) and the band starts.

The built-in synth uses the first `.sf2` file in `soundfonts/` (GeneralUser GS, downloaded separately; it's not in git). It plays on your default audio output with a 64-frame buffer (about 1.3 ms at 48 kHz).
- **Voices:** your right hand plays one of eight voice slots (Piano, E.Piano, Organ, Strings, Brass, Pad, Guitar, Synth Lead). Pick a slot with the Launchkey buttons under faders 1–8, or `F1`–`F8`. Change the voice in the last-picked slot with `9`/`0`.
- **Layering:** turn on layer mode with the button under the master fader, or `F9`. Tapping a slot then adds it to or removes it from the layer, so you can play Piano + Strings together.
- **Mixer:** faders 1–8 are the band's eight part volumes: each fader value is that part's CC 7, sent unchanged to the `yahaha` port and the built-in synth. Loading a style sets the faders to the style's own levels (100 where it sets none). A volume change inside a style's pattern moves its part's fader too, until you move that fader yourself. Every section change plays the style's part setup (SInt: voices, pan, effect sends, XG part parameters, and the drum setup, which a drum part's program change resets) again, as newer instruments do, so parts you have not touched go back to the style's levels before the new section's own changes; the levels you set are kept. A Fill or Break that comes in mid-bar also plays the voice changes from its skipped first beats. Start/Stop does the same, and also sends the style's XG effects, with insertion and variation effects moved to the parts' destination channels. The SInt's GM/XG System On resets are never sent, and its SysEx goes to the `yahaha` port only. Changing style resets all of them. The Launchkey faders, master included, use soft takeover: whenever software moves a level (a style load, a pattern's volume change, or a restart resetting an untouched part), the hardware fader does nothing until it comes within 2 of that level or crosses it (`↕` on screen until then). The master fader sets the synth's output level (100 = unity); a safety soft clipper above -1 dBFS keeps the output from hard clipping. Your own parts' levels (voice slot, OTS, Left) go out on the port as CC 7 too: the right hand's lowest layered slot on ch 1, and the Left volume on ch 2. Under Manual Bass, ch 2 gets the Bass part fader instead.
- **Left voice:** your left hand can play its own voice (default Strings) while it drives the chords. Toggle it with the LEFT pad on pad page 3, Shift + Pad Bank ▲, or `l`. Change the voice with `(` / `)`.
- **One Touch Settings:** each style carries four suggested panel setups. Each one covers Right 1–3 (loaded into voice slots 1–3, with their on/off as the layer) and the Left voice, including volumes and octave shifts. Recall one with `shift+1`–`4`. **OTS Link** (pad page 3, Shift + Pad Bank ▼, or `F10`) makes Main A–D recall settings 1–4 automatically, and picks the right one when you change style.
- **Stop Accompaniment** (`h`): with Sync Start off and the band stopped, a held chord sounds on the style's bass and pad voices.
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

### Pad pages

The 16 pads have three pages. The **Pad Bank ▲/▼** buttons left of the pads switch pages: ▼ goes to the next page, ▲ to the previous one, and they stop at the ends, so a few presses of ▲ always take you home to page 1. The arrows light in the current page's colour where there is a page to go to. `Tab` / `Shift+Tab` switch pages from the terminal too, and the on-screen pad map always shows the current page and its name.

**Page 1 · Sections** (per-section colours, the original layout):

| pad | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
|---|---|---|---|---|---|---|---|---|
| top | Intro I | Intro II | Intro III | Sync Start | Ending I | Ending II | Ending III | Auto Fill |
| bottom | Main A | Main B | Main C | Main D | Break | Tap tempo | Sync Stop | Start/Stop |

Pressing the current Main again plays its fill. With Auto Fill on, switching Main plays a fill into the new one.

**Page 2 · Chord/Setup** (all cyan):

| pad | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
|---|---|---|---|---|---|---|---|---|
| top | Single Finger | Fingered | Fingered On Bass | Multi Finger | AI Fingered | Full Keyboard | AI Full Keyboard | Upper (on) / Lower (off) |
| bottom | Manual Bass | Stop ACMP | Split − | Split + | Keyboard transpose − | Keyboard transpose + | Transpose reset | — |

- The lit fingering pad is the active type. Upper overrides it with Fingered* until you go back to Lower.
- Manual Bass is dark in Lower, where it isn't available.
- The transpose pads light while the transpose is down, up, or not zero.

**Page 3 · OTS/Parts** (all magenta):

| pad | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
|---|---|---|---|---|---|---|---|---|
| top | OTS 1 | OTS 2 | OTS 3 | OTS 4 | OTS Link | Left voice on/off | Left voice − | Left voice + |
| bottom | Rhythm 1 | Rhythm 2 | Bass | Chord 1 | Chord 2 | Pad | Phrase 1 | Phrase 2 |

- The lit OTS pad is the last one recalled. OTS pads the style doesn't have are dark, as are the OTS and Left pads when the synth is off.
- The part pads mute and unmute the eight accompaniment parts. They're lit while the part plays. Bass is shown off while Manual Bass mutes it.

### Buttons

| button | does |
|---|---|
| **Play** | start/stop |
| **Stop** | stop |
| **< Track** / **Track >** | previous/next style, playing or stopped (folder, then name: the browser's order) |
| **Pad Bank ▲ / ▼** (left of the pads) | previous/next pad page |
| **Shift + Pad Bank ▲ / ▼** | Left voice on/off / OTS Link on/off |
| **> (Scene Launch)** / **Function** (right of the pads) | tempo + / − |
| buttons under faders 1–8 / master | voice slots / layer mode |

The last Launchkey note or CC that nothing is mapped to shows at the bottom of the screen, e.g. `unmapped CC 103 = 127`. If a button does nothing, that shows the number it really sends.

### Terminal keys

- `space` start/stop
- `1-4` Main A–D
- `q w e` Intro I–III
- `i o p` Ending I–III
- `g` break
- `t` tap tempo
- `- =` tempo down/up
- `y` Sync Start · `u` Auto Fill · `j` Sync Stop
- `h` Stop ACMP
- `f` next fingering type · `d` Lower/Upper · `D` Manual Bass
- `[ ]` split point down/up
- `; '` Keyboard transpose −/+ · `: "` Master transpose −/+ · `/` reset both
- `z…,` mute/unmute parts
- `shift+1`–`4` OTS 1–4 · `F10` OTS Link
- `l` Left voice on/off · `( )` previous/next Left voice
- `F1`–`F8` voice slots · `F9` layer · `9 0` previous/next voice in the slot
- `←/→` previous/next style, in the style browser's order (folder, then name)
- `enter` open the style browser (see below)
- `tab` / `shift+tab` next/previous pad page
- `a` next audio output pair · `k` mute the synth
- `\` panic (all notes off)
- `esc` twice (within 1.5 s) quit, or `ctrl+c`; one `esc` closes the style browser

### Style browser

`yahaha play <folder>` finds every style under the folder and its subfolders (`.sty .prs .sst .bcs .pcs .pst .fps`, any case). Press `enter` to browse them:

- Each row shows the style's name (from the file, or the file name if it has none), its folder, tempo and time signature, plus the sections it has. The folder is the category.
- The list fills in right after launch while a background thread reads the files; rows show `…` until they're read. A file that can't be read shows as a red error row and is skipped by `←/→`.
- The style that's loaded is marked `▶`, and the cursor starts on it.
- **Type** to filter: a case-insensitive match on the name or folder. `backspace` edits the filter.
- `↑/↓`, `PgUp/PgDn` and `Home/End` move the cursor.
- `enter` loads the style and closes the browser. It works like `←/→`: while the band plays it keeps playing and follows your next chord in the new style.
- `esc` closes the browser without changing the style (with the browser closed, `esc` twice quits, so one extra `esc` never stops the band).
- While the browser is open, typed keys only go to the filter, never to the performance shortcuts. Your MIDI keyboard, the Launchkey pads and the Launchkey buttons keep working as usual, including **< Track / Track >**.

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
