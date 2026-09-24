# App API: `Session`, `AppCmd`, `AppState`

This is the contract between the yahaha engine and its clients: the terminal UI
(`yahaha play`) and the desktop app (Tauri, #17). Both drive the arranger the same way:

- **In:** [`AppCmd`](#appcmd): one enum for every user action, whether it comes from
  the screen, a computer-keyboard shortcut or the Launchkey.
- **Out:** [`AppState`](#appstate): one plain snapshot of everything a front panel
  shows. Its `version` changes whenever anything in it changes.
- **Notifications:** [`Event`](#events): a cheap "something changed" signal. The client
  then reads the state.

The source of truth is `src/api.rs` (types) and `src/session.rs` (runtime). All types
derive `serde::Serialize` and `Deserialize`.

## Using it from Rust

```toml
# app/src-tauri/Cargo.toml
[dependencies]
yahaha = { path = "../..", default-features = false }   # the library alone, no terminal UI
```

```rust
use yahaha::{AppCmd, Options, Session};

let session = Session::start(Options { paths: vec!["corpus".into()], sf2: Some(sf2), ..Options::default() })?;
session.send(AppCmd::Main { index: 1 })?;       // Result<(), CmdError>
let state = session.state();                    // Arc<AppState>, cheap to call
let events = session.subscribe();               // std::sync::mpsc::Receiver<Event>
let styles = session.library_list();            // LibraryList, when the library changes
session.stop();                                 // also runs on drop
```

`Session` is `Send + Sync`, so it can go straight into Tauri's managed state.

A Tauri shell needs about four pieces:

| Tauri side | Session call |
|---|---|
| `#[tauri::command] fn send(cmd: AppCmd) -> Result<(), CmdError>` | `session.send(cmd)` |
| `#[tauri::command] fn state() -> AppState` | `(*session.state()).clone()` |
| `#[tauri::command] fn library() -> LibraryList` | `session.library_list()` |
| a thread that emits events to the webview | `for e in session.subscribe() { app.emit("yahaha", e) }` |

The frontend listens for `yahaha` events. On `stateChanged` it fetches `state()`. On
`libraryChanged` it fetches `library()`. Events can arrive at up to about 100 per second
while playing. Throttle to the frame rate if you like: several changes can be merged
into one fetch, because the state is always complete.

### Dev mode: an offline session

`Session::offline(opts)` runs the same engine with no CoreMIDI, audio or threads, on a
virtual clock:

```rust
let s = Session::offline(Options { paths: vec!["corpus/MOX_v2/SlowWalker.T552.sty".into()], ..Options::default() })?;
s.midi_in(Port::Keys, &[0x90, 36, 100, 0x90, 40, 100, 0x90, 43, 100]); // a C chord: Sync Start
s.advance(500_000_000);                        // run the clock 500 ms
s.midi_in(Port::Pads, &[0x90, 113, 100]);      // Launchkey pad 113 (page 1: Main B)
let st = s.state();
```

The engine runs at every deadline on the way through `advance`. `send` and `midi_in`
apply at once, so `state()` straight after them already shows the result. In the app's
dev mode, call `advance` from a timer (for example 16 ms per frame) to watch it play.
`take_output()` returns the MIDI it played. `finish_indexing()` blocks until the library
index is complete.

To get real states as mock data, use `yahaha state-json <style or folder> ["C Am F G7"]`. It
plays the chords one bar each and prints the `AppState`. `yahaha state-json <folder> --library`
prints the `LibraryList`.

## AppCmd

JSON form: `{"type": "<camelCaseVariant>", ...fields}`, for example `{"type":"main","index":1}`,
`{"type":"startStop"}`, `{"type":"setFingering","fingering":"fingered"}`.

All indices are 0-based.
- **Keyboard parts:** 0 = Right 1, 1 = Right 2, 2 = Right 3, 3 = Left.
- **Style parts:** 0–7 = Rhythm 1, Rhythm 2, Bass, Chord 1, Chord 2, Pad, Phrase 1,
  Phrase 2 (MIDI channels 9–16).

**Toggles and setters.** The transport toggles are toggles only. The engine owns that
state, and pressing the button is the action. For settings, a GUI checkbox can use a
`set…` command. The terminal UI and the Launchkey use the toggles.

### Sections and transport

| Command | Fields | Does |
|---|---|---|
| `intro` | `index` 0–2 | Intro 1–3. Stopped: plays at the start. Playing: queued for the next bar. |
| `main` | `index` 0–3 | Main A–D. Pressing the Main that is playing plays its fill. With Auto Fill on, a change plays the fill first. |
| `break` | | Break (Fill In BA). |
| `ending` | `index` 0–2 | Ending 1–3. |
| `startStop` | | START/STOP. |
| `stop` | | Stops if playing, otherwise does nothing. This is the Launchkey Stop button. |
| `toggleSyncStart` | | SYNC START on/off. |
| `toggleSyncStop` | | SYNC STOP on/off. The engine ignores it while `transport.syncStopAvailable` is false. |
| `toggleAutoFill` | | AUTO FILL IN on/off. |
| `toggleStopAcmp` | | STOP ACMP on/off. |
| `tapTempo` | | TAP TEMPO. |
| `tempoUp`, `tempoDown` | | One tempo step. |
| `toggleStylePart` | `part` 0–7 | Mutes or unmutes a Style part. |
| `setStylePartVolume` | `part` 0–7, `volume` 0–127 | The part's CC7. The Launchkey fader has to reach the new value before it takes over again. |

### Chord detection, split, transpose

| Command | Fields | Does |
|---|---|---|
| `setFingering` | `fingering` | One of `singleFinger`, `multiFinger`, `fingered`, `fingeredOnBass`, `aiFingered`, `fullKeyboard`, `aiFullKeyboard`. |
| `nextFingering` | | Next type in display order, wrapping. The display order is single, fingered, onBass, multi, ai, full, aiFull. |
| `setUpper` / `toggleUpper` | `on` | Chord Detection Area Upper/Lower. Selecting Upper turns Manual Bass on. |
| `setManualBass` / `toggleManualBass` | `on` | The Manual Bass setting. Ignored in Lower. |
| `setSplit` | `note` (MIDI) | Split point, clamped to 24–96. |
| `moveSplit` | `delta` | Moves the split by `delta` keys. |
| `setTranspose` | `keyboard`, `master` | Semitones, each clamped to −12..12. |
| `stepTranspose` | `keyboard`, `master` | Adds to the current transpose. |
| `resetTranspose` | | Both back to 0. |

### Keyboard parts

| Command | Fields | Does |
|---|---|---|
| `setPartOn` / `togglePart` | `part` 0–3, `on` | Turns a part on or off. Left is refused while Manual Bass is in effect, with a `Failed` error and a message. |
| `selectPart` | `part` | The part that `stepVoice` and the Launchkey voice pads edit. |
| `setPartVoice` | `part`, `program` 0–127 | GM program. |
| `stepVoice` | `delta` | Previous or next voice for the selected part. |
| `setPartVolume` | `part`, `volume` 0–127 | The part's CC7. The Launchkey fader has to reach it before it takes over. |
| `setPartOctave` | `part`, `octave` −2..2 | Octave shift. |

### Mixer, Launchkey pages, synth

| Command | Fields | Does |
|---|---|---|
| `setFaderPage` / `toggleFaderPage` | `page`: `panel` \| `style` | What the Launchkey faders control. |
| `setPadPage` | `page`: `sections` \| `chordSetup` \| `otsParts` | The Launchkey pad page. |
| `cyclePadPage` | `delta` | Steps the pad page, wrapping. |
| `setMasterVolume` | `volume` 0–127 | Synth master (100 = unity). Fails when the synth is off. |
| `setSynthMuted` / `toggleSynthMute` | `on` | Mutes the synth audio. |
| `setAudioOutput` | `first` | Stereo pair by its left channel, 0-based (0 = outputs 1/2). |
| `nextAudioOutput` | | 1/2 → 3/4 → … → 1/2. |
| `panic` | | All notes off, and the style stops. |
| `clearMessage` | | Clears `state.message`. |

### One Touch Settings and styles

| Command | Fields | Does |
|---|---|---|
| `recallOts` | `index` 0–3 | Recalls OTS 1–4 into the keyboard parts. Ignored if the style has no such OTS. |
| `setOtsLink` / `toggleOtsLink` | `on` | OTS Link: Main A–D recall OTS 1–4, and so does a style change. |
| `loadStyle` | `id` | A library entry (`LibraryEntry.id`). Playing or stopped, the band carries on. |
| `loadStylePath` | `path` | Any style file. It is added to the library if it isn't there already. |
| `stepStyle` | `delta` | Previous or next style in library order. Files that don't load are skipped. |

### Result: `CmdError`

`send` returns `Ok(())` or one of these errors:
- `{"kind":"busy"}`: the engine's queue was full for a moment and nothing changed. Try
  again.
- `{"kind":"failed","message":"…"}`: refused or failed. The same text is in
  `state.message`.

`Ok` means the control side has applied the command. For engine commands (sections,
tempo, mute, Style volume), a live session shows the result in the state a few
milliseconds later, once the engine thread has run the command and published a
snapshot. An offline session shows it at once.

### The Launchkey

The session owns the Launchkey, so it works the same whichever client is running.
- Pads and buttons that are not engine buttons become the `AppCmd` that the matching
  keyboard shortcut sends, and run through the same code as `send`.
- Section pads, Start/Stop and tempo go straight from the MIDI thread to the engine, as
  before.
- Some controls stay on the MIDI thread for real-time reasons: the faders (soft takeover
  against session-internal atomics), Pad Bank ▲/▼ and the fader-page button. A pad
  pressed straight after a page change must already read the new page.
- The session drives all the LEDs.

`state.pads` mirrors the hardware.

## AppState

Units: MIDI values are 0–127. Tempo is in BPM. Times are in µs. Channels are 1-based.
Indices are 0-based unless a field says otherwise.

### `version`
`u64`. Grows by one each time the state changes. The same number means the same state.

### `style`: the loaded style
| Field | Type | Meaning |
|---|---|---|
| `id` | number | Its library entry id. |
| `path` | string | The style file. |
| `name` | string | The SFF name, or the file name when the style has none. |
| `format` | string | `SFF1` or `SFF2`. |
| `tempo` | number | The style's own tempo in BPM. The current tempo is `transport.tempo`. |
| `timeSignature` | [n, d] | For example `[4, 4]`. |
| `sections` | string[] | The sections the style has: `Intro A`–`D`, `Main A`–`D`, `Fill In AA`–`DD`, `Fill In BA` (Break), `Ending A`–`D`. |

### `transport`
| Field | Type | Meaning |
|---|---|---|
| `running` | bool | The style is playing. |
| `syncStart` | bool | Sync Start is armed: the next chord starts the style. |
| `syncStop` | bool | Sync Stop is on. |
| `syncStopAvailable` | bool | False in the Full Keyboard fingering types in Lower. |
| `autoFill`, `stopAcmp` | bool | Auto Fill In and Stop Accompaniment. |
| `section` | string? | The section playing, for example `Main A` or `Fill In AA`. Null when stopped. |
| `queued` | string? | The section queued next: at the next bar, or for a fill, at the next beat. |
| `pendingIntro` | 0–2? | The Intro armed to play at the start. |
| `main` | 0–3 | The Main (A–D) that is playing or queued to follow. Changes as soon as a Main is pressed. |
| `bar`, `beat` | 1-based | Position within the section playing. Both are 1 when stopped. |
| `beatsPerBar` | number | The numerator of the time signature. |
| `tempo` | number | Current tempo in BPM. |
| `lamps` | Pad[16] | Page 1 of the pads, whatever page the hardware is on. These are the section, Sync, Auto Fill, Tap and Start/Stop lamps exactly as the pads light them. See [Pad](#pad). |

### `chord`
| Field | Type | Meaning |
|---|---|---|
| `name` | string? | The chord the style follows, after Keyboard transpose, for example `Am7/G`. |
| `fingered` | string? | The chord as played, before Keyboard transpose. |
| `fingering` | enum | See `setFingering`. |
| `fingeringName` | string | For example `Fingered On Bass`. In Upper, the type actually used is Fingered*. |
| `upper` | bool | Chord Detection Area = Upper. |
| `manualBass` | bool | The Manual Bass setting. |
| `manualBassActive` | bool | Upper with the setting on. The left hand plays the Style's Bass voice, and the Style's Bass part is muted. |
| `split` | MIDI note | Keys at or below it are the left hand. |
| `splitName` | string | Yamaha octave numbering (C3 = 60), for example `F#2` or `Ab2`. |
| `transposeKeyboard`, `transposeMaster` | −12..12 | Semitones. |

### `keyboardParts`: always four, Right 1, Right 2, Right 3, Left
| Field | Type | Meaning |
|---|---|---|
| `name` | string | `Right 1` … `Left`. |
| `channel` | 1–16 | Right 1 = 1, Left = 2, Right 2 = 3, Right 3 = 4. The same on the MIDI port and in the synth. |
| `on` | bool | The part's switch. |
| `sounding` | bool | The part sounds: it is on, or it is Left playing the bass under Manual Bass. Light the part's lamp from this. |
| `selected` | bool | The part the voice commands edit. |
| `volume` | 0–127 | CC7. |
| `waiting` | bool | The Launchkey fader has moved but not yet reached `volume`. The terminal UI shows ↕. |
| `program` | 0–127 | The part's GM voice. |
| `voiceName` | string | What its channel plays. For Left under Manual Bass, that is the Style's Bass voice. |
| `playsBass` | bool | Left is playing the bass (Manual Bass). |
| `octave` | −2..2 | The octave setting. It is not applied while `playsBass` is true. |

### `mixer`
| Field | Type | Meaning |
|---|---|---|
| `faderPage` | `panel` \| `style` | What the Launchkey faders control. Panel: faders 1–4 are the keyboard parts. Style: faders 1–8 are the Style parts. |
| `styleParts` | StylePart[8] | See the table below. |
| `master` | 0–127? | The synth master level (100 = unity). Null without the synth. |
| `masterWaiting` | bool | The master fader has not yet reached `master`. |

StylePart:

| Field | Type | Meaning |
|---|---|---|
| `name`, `channel` | | `Rhythm 1` on channel 9 through `Phrase 2` on channel 16. |
| `on` | bool | Not muted, and not muted by Manual Bass. |
| `mutedByManualBass` | bool | The Bass part while Manual Bass is in effect. |
| `volume` | 0–127 | CC7. |
| `waiting` | bool | The fader is waiting to pick up the value. |
| `voice` | Voice? | The voice the style was written for: `bankMsb`, `bankLsb`, `program` (0-based), `kit` (a drum or SFX kit), and `label` (what the synth plays, for example `≈ Finger Bass  [Yamaha 104/18/88]`). |

### `pads`
| Field | Type | Meaning |
|---|---|---|
| `page` | `sections` \| `chordSetup` \| `otsParts` | The current Launchkey pad page. |
| `pageName`, `pageNumber` (1-based), `pageCount` | | For example `Chord/Setup`, 2, 3. |
| `pads` | Pad[16] | This page: the top row (notes 96–103), then the bottom row (112–119). |
| `connected` | bool | A Launchkey DAW port is connected. |

#### Pad
| Field | Type | Meaning |
|---|---|---|
| `note` | number | The pad's note on the Launchkey DAW port. |
| `label` | string | For example `MAIN A`, `FINGERED`, `OTS 1`. Empty for an unused pad. |
| `key` | string | The terminal UI's shortcut, for example `1`, `spc` or `F10`. The app can ignore it. |
| `rgb` | [r, g, b] 0–127 | Full-brightness colour. |
| `level` | `off` \| `dim` \| `bright` | `off`: not available (dark). `dim`: available, or a setting that is off. `bright`: playing or on. |
| `anim` | `solid` \| `flash` \| `pulse` | Flash: queued, waiting for the bar or beat. Pulse: armed, waiting for you. |
| `action` | AppCmd? | What pressing the pad sends. `send(pad.action)` does exactly what the hardware pad does. |

Draw a pad as the hardware lights it, where `beats` is a clock running at the current
tempo:

```
k = off: 0 · dim: 0.18 · bright+solid: 1
    bright+flash: 1 while frac(beats) < 0.5, else 0.18
    bright+pulse: 0.25 + 0.75·tri(frac(beats/2)), tri(p) = p<0.5 ? 2p : 2−2p
colour = rgb · k   (0–127 per channel; scale by 2 for CSS)
```

In Rust, `session.beats()` returns the beat clock the hardware LEDs use, so a Rust
client can flash in step with the hardware.

### `ots`
| Field | Type | Meaning |
|---|---|---|
| `settings` | OtsSetting[0–4] | `name` (`OTS 1` to `OTS 4`; styles don't name them) and `parts`: Right 1, Right 2, Right 3, Left as the setting sets them (`on`, `program` or null for a drum kit, `voiceName`, `volume`, `octave`). |
| `applied` | 0–4 | The last OTS recalled, 1-based. 0 means none since the style loaded. |
| `link` | bool | OTS Link. |

### `library`
| Field | Type | Meaning |
|---|---|---|
| `revision` | number | Changes when the library changes. Fetch `library()` when it does. |
| `count` | number | Entries. |
| `position` | number | The loaded style's position in library order, 0-based. |
| `pending` | number | Entries still being indexed. |

`library()` returns `LibraryList { revision, entries }` in display order (folder, then
name). Each `LibraryEntry` has these fields:
- `id`
- `name`
- `folder`
- `path`
- `status`: `pending`, `ok` or `error`
- `error`
- `tempo`
- `timeSignature`
- `sections`: a short list, for example `Main ABCD · Intro ABC · Ending ABC · Fill ABCD · Break`

Filter on the client. The terminal UI matches a case-insensitive substring of the name,
the file name or the folder.

### `io`
| Field | Type | Meaning |
|---|---|---|
| `outputPort` | string | The virtual MIDI output, `yahaha`. Empty offline. |
| `inputs` | string[] | The connected MIDI sources. The Launchkey DAW port is listed with ` (pads)`. |
| `synth` | SynthState? | `soundFont`, `device`, `sampleRate` (Hz), `bufferFrames`, `channels`, `outputPair` (1-based, for example [1, 2]), `muted`. Null when the synth is off. |
| `engine` | EngineStats | `realtime` (the engine thread got real-time scheduling), and 99th percentiles in µs: `wakeP99Us` (wake versus deadline), `chordP99Us` (chord to engine), `midiInP99Us` (MIDI in to callback). |
| `lastControl` | number | The last Launchkey DAW-port message, packed 0x00SSDDVV. |
| `unmapped` | string | The last Launchkey control nothing is mapped to, for example `unmapped CC 51 = 127`. |
| `offline` | bool | An offline session. |

### `message`
`{ seq, text, error }` or null. It holds the last notice or error, for example a style
that fails to load. `seq` increases with every new message, so the same text arriving
twice counts as two messages. A successful style change clears it, and so does
`clearMessage`.

## Events

JSON form `{"type": …}`:
- `stateChanged { version }`: `state()` has a new version.
- `libraryChanged { revision }`: `library()` changed. This happens while indexing (at
  most every 250 ms), when a file fails to load, and when a path is added.
- `stopped`: the session stopped.

Events carry no state. Always read the latest.

## Example `AppState`

This is a real offline session on SlowWalker, playing Main A with Fill In BB queued, with
OTS 1 recalled. Some lists are shortened here:
- `lamps` and `pads` have 16 entries.
- `styleParts` has 8.
- `ots.settings` lists every OTS in the style.

The `io` section shows what a live session with a Launchkey and the synth reports.

```json
{
  "version": 6,
  "style": {
    "id": 0,
    "path": "/Users/me/Styles/MOX_v2/SlowWalker.T552.sty",
    "name": "Regular style: SlowWalker",
    "format": "SFF1",
    "tempo": 75.0,
    "timeSignature": [4, 4],
    "sections": ["Intro A", "Intro B", "Intro C", "Main A", "Main B", "Main C", "Main D", "Fill In AA", "Fill In BB", "Fill In CC", "Fill In DD", "Fill In BA", "Ending A", "Ending B", "Ending C"]
  },
  "transport": {
    "running": true,
    "syncStart": false,
    "syncStop": false,
    "syncStopAvailable": true,
    "autoFill": true,
    "stopAcmp": false,
    "section": "Main A",
    "queued": "Fill In BB",
    "pendingIntro": null,
    "main": 1,
    "bar": 1,
    "beat": 3,
    "beatsPerBar": 4,
    "tempo": 75.0,
    "lamps": [
      {
        "note": 96,
        "label": "INTRO 1",
        "key": "q",
        "rgb": [127, 95, 0],
        "level": "dim",
        "anim": "solid",
        "action": { "type": "intro", "index": 0 }
      },
      {
        "note": 97,
        "label": "INTRO 2",
        "key": "w",
        "rgb": [127, 95, 0],
        "level": "dim",
        "anim": "solid",
        "action": { "type": "intro", "index": 1 }
      }
    ]
  },
  "chord": {
    "name": "Am",
    "fingered": "Am",
    "fingering": "fingeredOnBass",
    "fingeringName": "Fingered On Bass",
    "upper": false,
    "manualBass": true,
    "manualBassActive": false,
    "split": 54,
    "splitName": "F#2",
    "transposeKeyboard": 0,
    "transposeMaster": 0
  },
  "keyboardParts": [
    {
      "name": "Right 1",
      "channel": 1,
      "on": true,
      "sounding": true,
      "selected": true,
      "volume": 100,
      "waiting": false,
      "program": 80,
      "voiceName": "Square Lead",
      "playsBass": false,
      "octave": -1
    },
    {
      "name": "Right 2",
      "channel": 3,
      "on": true,
      "sounding": true,
      "selected": false,
      "volume": 80,
      "waiting": false,
      "program": 94,
      "voiceName": "Halo Pad",
      "playsBass": false,
      "octave": 0
    },
    {
      "name": "Right 3",
      "channel": 4,
      "on": false,
      "sounding": false,
      "selected": false,
      "volume": 100,
      "waiting": false,
      "program": 94,
      "voiceName": "Halo Pad",
      "playsBass": false,
      "octave": 0
    },
    {
      "name": "Left",
      "channel": 2,
      "on": true,
      "sounding": true,
      "selected": false,
      "volume": 40,
      "waiting": false,
      "program": 52,
      "voiceName": "Choir Aahs",
      "playsBass": false,
      "octave": 1
    }
  ],
  "mixer": {
    "faderPage": "panel",
    "styleParts": [
      {
        "name": "Rhythm 1",
        "channel": 9,
        "on": true,
        "mutedByManualBass": false,
        "volume": 65,
        "waiting": false,
        "voice": { "bankMsb": 127, "bankLsb": 0, "program": 57, "kit": true, "label": "drum kit 127/0/58" }
      },
      {
        "name": "Rhythm 2",
        "channel": 10,
        "on": true,
        "mutedByManualBass": false,
        "volume": 70,
        "waiting": false,
        "voice": { "bankMsb": 127, "bankLsb": 0, "program": 56, "kit": true, "label": "drum kit 127/0/57" }
      },
      {
        "name": "Bass",
        "channel": 11,
        "on": true,
        "mutedByManualBass": false,
        "volume": 74,
        "waiting": false,
        "voice": { "bankMsb": 104, "bankLsb": 18, "program": 87, "kit": false, "label": "≈ Finger Bass  [Yamaha 104/18/88]" }
      }
    ],
    "master": 100,
    "masterWaiting": false
  },
  "pads": {
    "page": "sections",
    "pageName": "Sections",
    "pageNumber": 1,
    "pageCount": 3,
    "pads": [
      {
        "note": 112,
        "label": "MAIN A",
        "key": "1",
        "rgb": [0, 127, 16],
        "level": "bright",
        "anim": "solid",
        "action": { "type": "main", "index": 0 }
      },
      {
        "note": 113,
        "label": "MAIN B",
        "key": "2",
        "rgb": [0, 127, 16],
        "level": "bright",
        "anim": "flash",
        "action": { "type": "main", "index": 1 }
      }
    ],
    "connected": true
  },
  "ots": {
    "settings": [
      {
        "name": "OTS 1",
        "parts": [
          { "on": true, "program": 80, "voiceName": "Square Lead", "volume": 100, "octave": -1 },
          { "on": true, "program": 94, "voiceName": "Halo Pad", "volume": 80, "octave": 0 },
          { "on": false, "program": 94, "voiceName": "Halo Pad", "volume": 100, "octave": 0 },
          { "on": true, "program": 52, "voiceName": "Choir Aahs", "volume": 40, "octave": 1 }
        ]
      }
    ],
    "applied": 1,
    "link": false
  },
  "library": { "revision": 2, "count": 1, "position": 0, "pending": 0 },
  "io": {
    "outputPort": "yahaha",
    "inputs": ["Launchkey MK4 61 MIDI Out", "Launchkey MK4 61 DAW Out (pads)"],
    "synth": {
      "soundFont": "GeneralUser-GS",
      "device": "MacBook Pro Speakers",
      "sampleRate": 48000,
      "bufferFrames": 64,
      "channels": 2,
      "outputPair": [1, 2],
      "muted": false
    },
    "engine": { "realtime": true, "wakeP99Us": 1, "chordP99Us": 12, "midiInP99Us": 90 },
    "lastControl": 0,
    "unmapped": "",
    "offline": false
  },
  "message": null
}
```

## Threads and real-time rules

These are for maintainers.
- `send` runs on the caller's thread, under the session's control lock.
- A control thread wakes on Launchkey actions and new engine snapshots, or at least
  every 10 ms, which keeps the RGB pad animation smooth. It does the following:
  - runs Launchkey actions
  - applies OTS Link
  - drives the LEDs
  - applies index results
  - republishes `AppState` if anything changed
- The CoreMIDI and engine threads never lock or allocate. They talk to the control side
  only through SPSC rings, atomics and non-blocking semaphore signals.
- The synth's audio stream lives on a thread of its own, which keeps `Session` `Send`.
