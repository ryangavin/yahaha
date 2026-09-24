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
- **Keyboard parts:** like the Genos, you play four parts: **Right 1**, **Right 2** and **Right 3** right of the split, and **Left** left of it. Each part has its own voice, volume, octave shift and on/off. The Right parts that are on sound together, which is how you layer (Piano + Strings = Right 1 + Right 2 on). At start only Right 1 (Grand Piano) is on; Right 2 is Strings, Right 3 Brass, Left Strings. Turn parts on/off with the buttons under faders 1–4 (fader Panel page), the bottom-left pads on pad page 3, or `5` `6` `7` `8` (`l` also toggles Left). Pick the part whose voice you want to change with `F1`–`F4`, the EDIT pads on pad page 3, or Shift + the button under its fader, then step its voice with `9`/`0` or the VOICE −/+ pads.
- **Where your parts go:** each part has its own channel, on the `yahaha` port and in the built-in synth alike: Right 1 = ch 1, Left = ch 2 (the channels your right and left hand always had), Right 2 = ch 3, Right 3 = ch 4. A part that is off sends nothing. With Left off, the Right parts play over the whole keyboard, as on the Genos, except that in Lower chord detection (outside the Full Keyboard fingerings) the keys left of the split only drive the chords. Each part's octave shift is applied to the notes it sends. Pedals, wheels and pressure go to all four parts (polyphonic aftertouch to the notes its key sounds); the keyboard's own volume (CC 7), bank select and program changes are ignored, because each part's voice and volume are its own.
- **Mixer:** the faders have two pages, like the Genos Mixer's Panel and Style tabs. The button under the master fader (or `F9`) switches between them; it lights blue on Panel and green on Style, and the screen outlines the active page in yellow.
  - **Panel:** faders 1–4 are the volumes of Right 1, Right 2, Right 3 and Left; their buttons turn the parts on/off (lit while on). Faders 5–8 do nothing on this page.
  - **Style:** faders 1–8 are the band's eight part volumes; their buttons mute and unmute the parts (lit while they play).
  - Every level is its part's CC 7, sent unchanged to the `yahaha` port and the built-in synth; there is no other per-part gain. Under Manual Bass, Left plays the Style's Bass voice at Left's own level (Panel fader 4), at the pitch you play (Left's octave shift is for its own voice), and Left can't be switched off until Manual Bass is. The master fader is always the synth's output level (100 = unity); a safety soft clipper above -1 dBFS keeps the output from hard clipping.
  - Loading a style sets the Style faders to the style's own levels (100 where it sets none). A volume change inside a style's pattern moves its part's fader too, until you move that fader yourself. Every section change puts back what the last section's patterns changed in the style's part setup (SInt: voices, pan, effect sends, bend ranges), so parts you have not touched go back to the style's levels before the new section's own changes; the levels you set are kept. Only what differs is sent: a section change that changes nothing sends nothing, never a program change for the voice a part already has, and the XG part parameters and drum setup only after a program change that resets them. A Fill or Break that comes in mid-bar also takes the voice and controller values its skipped first beats leave. Start/Stop does the same, and also sends the style's XG effects, with insertion and variation effects moved to the parts' destination channels. The SInt's GM/XG System On resets are never sent, and its SysEx goes to the `yahaha` port only. Changing style resets all of them.
  - The Launchkey faders, master included, use soft takeover: whenever a level moves without the fader (a style load, a pattern's volume change, a restart resetting an untouched part, an OTS recall, or switching the fader page), the hardware fader does nothing until it comes within 2 of that level or crosses it (`↕` on screen until then).
- **One Touch Settings:** each style carries four suggested panel setups. Each one sets Right 1–3 and Left: voice, on/off, volume and octave shift; the Panel faders then pick the new volumes up. Recall one with `shift+1`–`4` or the OTS pads on pad page 3. **OTS Link** (pad page 3, Shift + Pad Bank ▼, or `F10`) makes Main A–D recall settings 1–4 automatically, and picks the right one when you change style.
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
   | 1 | Right 1 (your right hand) | 11 | Bass |
   | 2 | Left (your left hand, when the Left part is on) | 12 | Chord 1 |
   | 3 | Right 2 (right-hand layer) | 13 | Chord 2 |
   | 4 | Right 3 (right-hand layer) | 14 | Pad |
   | 9 | Rhythm 1 (sub drums) | 15 | Phrase 1 |
   | 10 | Rhythm 2 (main drums) | 16 | Phrase 2 |

   The screen shows which Yamaha voice each part was written for (e.g. "≈ Finger Bass"), so you know what sound to load. Drum parts use the GM drum map.

   Channels 11–16 get a pitch bend range of at least 12 semitones (RPN 0), because a chord change can bend a held note to its new pitch. A part whose patterns bend on their own gets more, up to 24, so the pattern's bend fits on top; so does a style that sets more itself. If an instrument ignores RPN, set its pitch bend range by hand: 12, or 24 to be safe.
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
| top | OTS 1 | OTS 2 | OTS 3 | OTS 4 | OTS Link | — | Voice − | Voice + |
| bottom | Right 1 on/off | Right 2 on/off | Right 3 on/off | Left on/off | Edit Right 1 | Edit Right 2 | Edit Right 3 | Edit Left |

- The lit OTS pad is the last one recalled. OTS pads the style doesn't have are dark.
- The on/off pads are lit while the part is on. The lit Edit pad is the part whose voice Voice −/+ (`9`/`0`) changes.
- The accompaniment parts are muted with the buttons under the faders on the Style fader page, or `z`…`,`.

### Buttons

| button | does |
|---|---|
| **Play** | start/stop |
| **Stop** | stop |
| **< Track** / **Track >** | previous/next style, playing or stopped (folder, then name: the browser's order) |
| **Pad Bank ▲ / ▼** (left of the pads) | previous/next pad page |
| **Shift + Pad Bank ▲ / ▼** | Left part on/off / OTS Link on/off |
| **> (Scene Launch)** / **Function** (right of the pads) | tempo + / − |
| buttons under faders 1–8 | Panel page: Right 1–3, Left on/off (Shift: edit that part's voice) · Style page: mute/unmute the style parts |
| button under the master fader | fader page Panel / Style |

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
- `z…,` mute/unmute the style parts
- `shift+1`–`4` OTS 1–4 · `F10` OTS Link
- `5 6 7 8` Right 1, Right 2, Right 3, Left on/off · `l` Left on/off
- `F1`–`F4` pick the part to edit (Right 1–3, Left) · `9 0` previous/next voice for it
- `F9` fader page Panel / Style
- `←/→` previous/next style, in the style browser's order (folder, then name)
- `enter` open the style browser (see below)
- `tab` / `shift+tab` next/previous pad page
- `a` next audio output pair · `k` mute the synth
- `r` chart mode on/off · `( )` previous/next chart song (see "iReal Pro charts" below)
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

### iReal Pro charts

`yahaha play <styles> --ireal <playlist.html | irealb://…>` imports an iReal Pro playlist (an exported `.html` file, or a link), chooses its first song and turns **chart mode** on. In chart mode the band takes its chords from the chart instead of your left hand:

- Press `space` (or play a chord with Sync Start on) to start. An Intro plays first, then the chart, then an Ending.
- Chart sections A–D play Main A–D. With Auto Fill on, a fill leads into each new section.
- Play a chord to reharmonize: it holds until the next bar line, then the chart takes over again.
- Keyboard transpose (`; '`) moves the chart too.
- `r` turns chart mode on/off; `( )` pick the previous/next song of the playlist.

The desktop app has the same player, with a song browser, the chart in the lead-sheet band and the choruses, loop, Intro/Ending and style settings. [docs/ireal.md](docs/ireal.md) has the details.

Chords are recognized in "Fingered On Bass" style, plus some shortcuts:
- one key = major
- two keys = power chord, major/minor third, or 7th
- the lowest note becomes a slash bass
- three adjacent keys = chord cancel (drums only)

## Architecture

```
CoreMIDI receive thread ──chord (AtomicU32) + commands (SPSC ring) + semaphore──▶ engine thread (Mach real-time)
CoreMIDI receive thread ──Launchkey actions (SPSC) + semaphore──▶ session control
session control (commands, LEDs, OTS Link, style loading) ──styles/commands (SPSC)──▶ engine thread
engine thread ──snapshots, old styles (SPSC) + semaphore──▶ session control
clients (terminal UI, desktop app) ──AppCmd──▶ Session ──AppState──▶ clients
```

The engine, the runtime and the app API are the `yahaha` library. A `Session` owns MIDI, the engine thread, the synth and the Launchkey. Clients send `AppCmd`s and read `AppState` snapshots, and never touch the engine. The terminal UI is the `yahaha` binary's client, and the desktop app will be another. The contract is in [docs/app-api.md](docs/app-api.md).

- **Input thread** (CoreMIDI's own receive thread, running our callback):
  - forwards your notes straight to the output, on each keyboard part that is on (Right 1 ch 1, Left ch 2, Right 2 ch 3, Right 3 ch 4)
  - recognizes chords with a precomputed table of 4096×12 entries
  - publishes the result without locking
- **Engine thread**:
  - sleeps on a Mach semaphore until the next pattern event or an input signal, whichever comes first
  - spins the last 150 µs to hit the deadline exactly
  - never locks, allocates, or does I/O
- New styles are prepared on the session's control side, swapped in by pointer, and freed back there.
- Every note is transposed at the moment it plays, and there is no lookahead. A chord change therefore reaches the very next note, and notes already sounding are re-pitched according to the style's retrigger rule: Pitch Shift bends them with the part's pitch bend (one bend per part, so a note that needs a different shift is retriggered), Retrigger plays them again at the new pitch. A note that started less than 40 ms before the chord arrived is corrected outright, and one that ends (or is struck again) less than 40 ms after it is left to end rather than attacked again.

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
- `yahaha capture-kit <out-dir> [--clock-ppm N] [style]...`: writes the Genos-owner reference capture kit, with a chord-script MIDI file per style and instructions (`docs/capture-kit/`). `--clock-ppm` runs the files that much faster, for an instrument whose clock drifts past a style's tolerance.
- `yahaha capture-import <recording.mid> <style>`: compares a hardware recording of that kit with what yahaha plays, bar by bar and part by part. `--golden tests/reference` turns a verified recording into a reference digest (`tests/reference/README.md`).
- `yahaha oracle corpus/ [--pairs | --scores | --diff tests/oracle/scores.txt]`: scores our chord conversion against the authors' own major/minor source channels (docs/oracle.md). Counts only.
- `yahaha bench <style> [spin_us]`: latency benchmark using virtual ports.
- `yahaha drive`: fake keyboard for testing against a running `yahaha play --input TestKbd`.
- `yahaha state-json <style or folder> ["C Am F"] [--library]`: an offline session's `AppState` (or its library) as JSON, after playing the chords one bar each. This is mock data for the app (docs/app-api.md).

Tests: run `cargo test --release`. It covers the spec's transposition examples, chord recognition, and a full performance of every style in `corpus/`, checking for stuck notes. The oracle scores in `tests/oracle/scores.txt` are pinned too: a change to note conversion fails `oracle::tests::corpus_scores` with the score delta until you regenerate them with `UPDATE_GOLDEN=1`.

## Known gaps

- The NTT transposition tables are reconstructed from documentation and have not yet been checked against a real Genos (see PLAN.md §4).
- Not done yet: ritardando on a second Ending press, Ableton Link, audio styles, OTS voice changes, and the Ctb2 bytes that are still undocumented.

## License

MIT. Yamaha, Genos, PSR, and Tyros are trademarks of Yamaha Corporation. yahaha is an independent project, not affiliated with Yamaha; it reads the publicly documented style file format for interoperability.
