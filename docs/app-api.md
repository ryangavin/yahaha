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
use yahaha::{api::TransportCmd, Options, Session};

let session = Session::start(Options { paths: vec!["corpus".into()], sf2: Some(sf2), ..Options::default() })?;
session.send(TransportCmd::Main { index: 1 })?; // any AppCmd or group; Result<(), CmdError>
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
| `#[tauri::command] fn state() -> AppState` | `session.state_now()` (the state, with its clock read now: see [`surface.clock`](#surfaceclock)) |
| `#[tauri::command] fn library() -> LibraryList` | `session.library_list()` |
| `#[tauri::command] fn meters() -> Meters` | `session.meters()` (output levels; see [Meters](#meters)) |
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
| `fill` | `delta` −1, 0, 1 | Fill Down, Fill Self, Fill Up (the Genos assignable functions): the fill, then the Main to the left, the same Main, or the Main to the right, whatever Auto Fill says. Past Main A or D, the fill of the Main at the end. Stopped: selects that Main. |
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

### Settings

| Command | Fields | Does |
|---|---|---|
| `setSoundFont` | `file` | Reloads the synth from another `.sf2` in its folder (`io.soundFonts`, by file name; the folder is the one the `--sf2` file is in). The SoundFont loads on a thread of its own (`io.soundFontLoading`) and swaps in between two audio buffers: the voices and controllers every channel has carry over, notes sounding fade out over one buffer. Fails when the synth is off or the file isn't there. |
| `setMidiInputs` | `all`, `names` | Which MIDI sources play the keyboard: every one (`all`), or the ones whose name contains one of `names`. `all` false with no names is the default: a Launchkey's keys when there is one, else every source. yahaha's own port and DAW ports are never keyboards; the Launchkey DAW port is always the pads. Sources connect and disconnect at once. Keys held on a source that is dropped are released: their notes stop at once (All Notes Off on the keyboard parts' channels, which also stops notes other sources hold) and the chord section lets go. |
| `setPaletteLeds` | `on` | Launchkey LEDs in Novation palette colours (and hardware flashing) instead of RGB. Every pad is sent again. |
| `rescanLibrary` | | Walks the style folders (`library.roots`) again on a thread of its own (`library.scanning`). A file still there keeps its id and index; new files are added and indexed; a file gone leaves the list (its id stays valid). |

### One Touch Settings and styles

| Command | Fields | Does |
|---|---|---|
| `recallOts` | `index` 0–3 | Recalls OTS 1–4 into the keyboard parts. Ignored if the style has no such OTS. |
| `setOtsLink` / `toggleOtsLink` | `on` | OTS Link: Main A–D recall OTS 1–4, and so does a style change. |
| `loadStyle` | `id` | A library entry (`LibraryEntry.id`). Stopped, it loads at once. Playing, it takes over at the next bar line, as on a Genos: the band carries on in the same section (the same Main, or the nearest the new style has) at the same bar position, at the same tempo. Until then `preview.queued` names it and `style` is still the old one. A later style change before the bar line replaces it; stopping first loads it then. |
| `queueStyle` | `id` | The same as `loadStyle` (the browser's "next bar" button). |
| `loadStylePath` | `path` | Any style file. It is added to the library if it isn't there already. |
| `stepStyle` | `delta` | Previous or next style in library order, from the style waiting for the bar line if there is one. Files that don't load are skipped. |
| `auditionStyle` | `id` | Previews a style while the band is stopped: its Main A, at its own tempo, with its own voices and levels, over C Am F G7 (a chord a bar) for 4 bars, then it stops by itself (`preview.audition`). The loaded style, OTS, keyboard parts, mixer and transport are untouched; the loaded style's setup is sent again when it ends. Refused (`failed`) while the band plays. A new one replaces the one playing; it ends early on `stopAudition`, a style change, START/STOP, `panic` or a chord that starts the band (Sync Start). |
| `stopAudition` | | Ends the preview now. |

### Multi Pads

Pads are 0–3 (pads 1–4). See docs/multipad.md for the Genos behaviour and what is a guess.

| Command | Fields | Does |
|---|---|---|
| `loadMultiPad` | `id` | Loads a bank from `multiPad.banks` (the `.pad` files in the style folders). The file is parsed on the control side; pads playing stop when the new bank takes over (`multiPad.loading` until then, a moment later live). A file that doesn't parse fails and keeps the bank loaded. |
| `loadMultiPadPath` | `path` | Any `.pad` file; it is added to `multiPad.banks` if it isn't there already (once it has loaded). A `rescanLibrary` keeps such a bank listed, with its id, while its file is there. |
| `clearMultiPad` | | No bank: the pads go dark. |
| `triggerMultiPad` | `pad` | Presses a pad: it plays from the top (a playing pad restarts). Stopped, it starts at once; while the band plays, at the next bar line (`lamp` `queued` until then). Pads in Synchro Start standby start with it. |
| `stopMultiPad` | `pad` | STOP + pad: that pad stops now. |
| `stopAllMultiPads` | | STOP: every pad stops, and Synchro Start standby is cancelled. |
| `armMultiPad` | `pad` | SELECT + pad: toggles the pad's Synchro Start standby (`lamp` `armed`). Armed pads start on the next chord played in the chord section, or when the band starts; while the band plays, at the next bar line. |
| `setMultiPadRepeat` | `pad`, `on` | Overrides the pad's Repeat flag (from the bank file) until the next bank loads. |
| `setMultiPadChordMatch` | `pad`, `on` | Overrides the pad's Chord Match flag until the next bank loads. |
| `setMultiPadSynchroStop` | `styleStop`, `ending` | Multi Pad Synchro Stop: repeating pads stop when the band stops (`styleStop`, default on) and when an Ending starts (`ending`, default off). One-shot pads always play out. |

### Controllers

Pedals, the wheels and the assignable functions (docs/controllers.md).

| Command | Fields | Does |
|---|---|---|
| `setPedal` | `pedal` 0–2, `cc`, `function`, `controlType`, `reverse`, `range` | Sets up a pedal: the control change it listens for on the keyboards (`cc` 0–127, or null for none), its assignable function (an `id` from `app/src/lib/api/assignable-functions.json`, for example `sustain`, `startStop`, `fillUp`, `ots1`), its Control Type for Sustain, Sostenuto and Soft (`holdA`: on while held, `holdB`: off while held, `toggle`), reversed polarity, and the Range of a Pitch Bend pedal (`upper`, `lower`, `full`). `controlType`, `reverse` and `range` may be left out (`holdA`, false, `upper`). |
| `learnPedal` | `pedal` 0–2 or null | The pedal takes the CC of the next control change a keyboard presses (a value of 64 or more; not bank select, volume, the modulation wheel, data entry or channel mode messages). Null stops learning. |
| `setPartControllers` | `part` 0–3, `sustain`, `pitchBend`, `modulation` | Which controllers reach a keyboard part: the pedal switches (sustain, sostenuto, soft), the pitch bend, the modulation. |
| `setBendRange` | `part` 0–3, `semitones` 0–12 | The part's Pitch Bend Range (RPN 0 on its channel). |
| `triggerFunction` | `function` | Runs an assignable function as a pedal press would (Sustain, Sostenuto and Soft toggle). Fails for a function yahaha doesn't have yet (`available` false) and for Modulation and Pitch Bend, which need a foot controller. |

### Result: `CmdError`

`send` returns `Ok(())` or one of these errors:
- `{"kind":"busy"}`: the engine's queue was full for a moment and nothing changed. Try
  again.
- `{"kind":"failed","message":"…"}`: refused or failed. The same text is in
  `state.message`.

`Ok` means the control side has applied the command, and `state()` straight after
`send` already shows it. For engine commands (sections, tempo, mute, Style volume), a
live session shows the result in the state a few
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
| `sectionBars` | number? | How many bars the section playing lasts (a Main's pattern length; it loops). Null when stopped. |
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
| `fader` | 0–127? | Where its Launchkey fader (Panel page, faders 1–4) physically is, as last reported. Null until that fader moves. |

### `mixer`
| Field | Type | Meaning |
|---|---|---|
| `faderPage` | `panel` \| `style` | What the Launchkey faders control. Panel: faders 1–4 are the keyboard parts. Style: faders 1–8 are the Style parts. |
| `styleParts` | StylePart[8] | See the table below. |
| `master` | 0–127? | The synth master level (100 = unity). Null without the synth. |
| `masterWaiting` | bool | The master fader has not yet reached `master`. It turns on as soon as `setMasterVolume` moves the level away from the fader. |

StylePart:

| Field | Type | Meaning |
|---|---|---|
| `name`, `channel` | | `Rhythm 1` on channel 9 through `Phrase 2` on channel 16. |
| `on` | bool | Not muted, and not muted by Manual Bass. |
| `mutedByManualBass` | bool | The Bass part while Manual Bass is in effect. |
| `volume` | 0–127 | CC7. |
| `waiting` | bool | The fader is waiting to pick up the value. |
| `fader` | 0–127? | Where its Launchkey fader (Style page, faders 1–8) physically is. Null until it moves. |
| `voice` | Voice? | The voice the style was written for: `bankMsb`, `bankLsb`, `program` (0-based), `kit` (a drum or SFX kit), and `label` (what the synth plays, for example `≈ Finger Bass  [Yamaha 104/18/88]`). |

### `pads`
The Launchkey's 16 pads. Together with [`surface`](#surface) (every other control,
Shift, the faders and the beat clock), they are a 1:1 mirror of the Launchkey MK4: every
control's meaning, and every LED as the hardware shows it.

| Field | Type | Meaning |
|---|---|---|
| `page` | `sections` \| `chordSetup` \| `otsParts` | The current Launchkey pad page. |
| `pageName`, `pageNumber` (1-based), `pageCount` | | For example `Chord/Setup`, 2, 3. |
| `pads` | Pad[16] | This page: the top row (notes 96–103), then the bottom row (112–119). |
| `connected` | bool | A Launchkey DAW port is connected. It is set once, at start: see the limitation below. |
| `paletteLeds` | bool | The session runs the LEDs in Novation palette mode (`setPaletteLeds`, `--palette-leds`). The pads then carry `palette`. |

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
| `palette` | PaletteLed? | Palette-LED mode only: what the pad was sent. The fields are `mode` (`solid`, `flash` or `pulse`), `colour` (palette index) with its `rgb` and `level`, and for `flash` also `flashColour`, `flashRgb` and `flashLevel` (the second colour). Null in RGB mode. |

In RGB mode (the default), the pads show `rgb`, `level` and `anim`, animated on the LED
clock. Draw a pad as the hardware lights it, where `beats` is the LED clock
(`led` in [`surface.clock`](#surfaceclock)):

```
k = off: 0 · dim: 0.18 · bright+solid: 1
    bright+flash: 1 while frac(beats) < 0.5, else 0.18
    bright+pulse: 0.25 + 0.75·tri(frac(beats/2)), tri(p) = p<0.5 ? 2p : 2−2p
colour = rgb · k   (0–127 per channel; scale by 2 for CSS)
```

In Rust, `session.beats()` returns the same clock.

**Palette mode has a limitation.** The Launchkey flashes and pulses palette colours by
itself, on its own timing. yahaha sends it no MIDI clock, so in palette mode:
- The hardware's flash and pulse are not in step with the tempo or with the LED clock.
- The colours are the palette's, so they are close to `rgb` but not the same.

`palette` describes what was sent. Animate it on the LED clock: it won't match the
hardware's phase, and nothing can.

**The Launchkey is connected once, at start** (#74). A Launchkey plugged in later is not
seen, and a replugged one stays out of DAW mode until the session restarts. `connected`
describes the start. Keyboards are different: the session lists the MIDI sources every
2 s and connects what `setMidiInputs` chose, so a keyboard plugged in later plays
(`io.sources`, `io.inputs`).

### `ots`
| Field | Type | Meaning |
|---|---|---|
| `settings` | OtsSetting[0–4] | `name` (`OTS 1` to `OTS 4`; styles don't name them) and `parts`: Right 1, Right 2, Right 3, Left as the setting sets them (`on`, `program` or null for a drum kit, `voiceName`, `volume`, `octave`). |
| `applied` | 0–4 | The last OTS recalled, 1-based. 0 means none since the style loaded. |
| `link` | bool | OTS Link. |

### `library`
| Field | Type | Meaning |
|---|---|---|
| `revision` | number | The revision `library()` returns. It changes when the library changes: fetch `library()` when it does. All four fields describe that same list. |
| `count` | number | Entries. |
| `position` | number | The loaded style's position in library order, 0-based. |
| `pending` | number | Entries still being indexed. |
| `roots` | string[] | The style folders (and files) the library scans. |
| `scanning` | bool | A rescan (`rescanLibrary`) is walking the folders. |

`library()` returns `LibraryList { revision, entries, voices }`. `entries` are in display
order (folder, then name). `voices` is the list `setPartVoice` picks from, the same for
every revision: `{ program, bankMsb, bankLsb, name }`, the 128 GM voices on bank 0 (the
names are `gm_name`'s). Each `LibraryEntry` has these fields:
- `id`
- `name`
- `folder`
- `path`
- `status`: `pending`, `ok` or `error`
- `error`
- `tempo`
- `timeSignature`
- `sections`: a short list, for example `Main ABCD · Intro ABC · Ending ABC · Fill ABCD · Break`
- `format`: `SFF1` or `SFF2` from the file's header; null while pending or unreadable

Filter on the client. The terminal UI matches a case-insensitive substring of the name,
the file name or the folder.

### `surface`
The Launchkey beyond the pads.

| Field | Type | Meaning |
|---|---|---|
| `shift` | bool | The Shift button is held. Show the controls' Shift layer (`shiftLabel`, `shiftAction`) while it is. The pads have no Shift layer: the firmware keeps Shift + pad for itself. |
| `controls` | SurfaceControl[17] | Every button, in this order: `padBankUp`, `padBankDown`, `trackPrev`, `trackNext`, `play`, `stop`, `scene` (right of the top pad row), `function` (right of the bottom row), `faderButton1`…`faderButton8` (under the faders), `masterButton` (under the master fader). |
| `faders` | SurfaceFader[9] | Faders 1–8 on the active fader page, then the master fader. |
| `trackPrev`, `trackNext` | Neighbour? | Where Track ◀ / ▶ (and `stepStyle`) go: `{ id, name, path }` of the previous and next style in library order, skipping files known not to load. Null when there is nowhere to go. |
| `clock` | ClockState | The beat clocks. See [below](#surfaceclock). |

#### SurfaceControl
| Field | Type | Meaning |
|---|---|---|
| `id` | string | See above. |
| `cc` | number | Its CC on the DAW port, channel 1. |
| `label` | string | What it does now, for example `PAGE ▼`, `RIGHT 2`, `PAD` (mutes the Style's Pad part), or `PANEL` (the master fader button: the faders are on the Panel page, and pressing switches). Empty when it does nothing. |
| `action` | AppCmd? | What pressing it sends. `send(action)` does exactly what the hardware button does. Null when it does nothing, for example Pad Bank ▲ on the first page, Track with one style, or fader buttons 5–8 on the Panel page. |
| `shiftLabel`, `shiftAction` | string, AppCmd? | What it does with Shift held. Most buttons do the same as without Shift, and there these equal `label`/`action`. The ones that differ: Pad Bank ▲ = `LEFT` (Left on/off), Pad Bank ▼ = `OTS LINK`, Panel fader buttons 1–4 = `EDIT R1`… (select the part). |
| `rgb`, `level`, `anim` | | Its light, as a pad's. `anim` is always `solid`: buttons don't flash. |
| `colour` | number? | The palette index yahaha sends it. Buttons have no RGB mode, so `rgb` is a close match to that colour. Null for Play, Stop, Scene and Function: yahaha doesn't drive those LEDs, they show the Launchkey's own default, and they are reported `off`. |

Which button LEDs are lit, and in what colour:
- **Pad Bank ▲/▼:** lit in the page's colour (white, cyan, pink) where there is a page to
  go to.
- **Track ◀/▶:** white when the library has another style.
- **Fader buttons on the Panel page:** blue, bright when the part sounds and dim when it
  is off. Fader buttons 5–8 are dark.
- **Fader buttons on the Style page:** green, bright when the part plays and dim when it
  is muted or muted by Manual Bass.
- **Master button:** the page's colour, bright.

#### SurfaceFader
| Field | Type | Meaning |
|---|---|---|
| `label` | string | What it controls on this page, for example `RIGHT 1`, `BASS` or `MASTER`. Empty when unused: faders 5–8 on the Panel page, or the master fader without the synth. |
| `value` | 0–127? | The level it controls. Null when unused. |
| `waiting` | bool | The level is waiting for the hardware fader (soft takeover). |
| `position` | 0–127? | Where the hardware fader physically is, as last reported. It is the same physical fader on both pages. Null until it moves. |
| `set` | AppCmd? | What moving it sends: this command with `volume` filled in (`setPartVolume`, `setStylePartVolume` or `setMasterVolume`; `volume` is 0 here). Null when unused. |

#### `surface.clock`
Everything here is about time: the playing position, and the clock the pads flash on.
Times are the session's monotonic clock in **ms**, as fractional numbers (ns would exceed
JavaScript's exact integers). Beats are quarter notes. The state is republished when
something changes, not as time passes, so the clock is anchors. Each anchor is a value
at a time, and it moves on at `tempo` until the next state:

| Field | Type | Meaning |
|---|---|---|
| `atMs` | ms | The session clock when this state was read. With `state()`, that is when it last changed. With `state_now()`, it is now. |
| `running` | bool | The style is playing. |
| `tempo` | BPM | Quarter notes per minute. |
| `beatsPerBar` | number | Quarter notes per bar: 4 in 4/4, 3 in 3/4 and in 6/8. |
| `bar`, `beat` | 1-based | The position at `atMs`, in the section. 1, 1 when stopped. |
| `phase` | 0..1 | How far into the beat at `atMs`. 0 when stopped. |
| `sectionAnchorMs`, `sectionAnchorBeats` | ms, beats | At `sectionAnchorMs`, the section had played `sectionAnchorBeats`. The anchor moves when the tempo, the section or its loop changes. |
| `ledAnchorMs`, `ledAnchorBeats` | ms, beats | The free-running clock the pads flash and pulse on: it read `ledAnchorBeats` at `ledAnchorMs`. It re-anchors when the tempo changes, carrying on from where it was. |

To animate without calling Rust, record `receivedMs = performance.now()` when a state
arrives, then on every frame:

```
t     = atMs + (performance.now() − receivedMs)                  // session ms, now
pos   = running ? max(0, sectionAnchorBeats + (t − sectionAnchorMs) · tempo / 60000) : 0
bar   = floor(pos / beatsPerBar) + 1
beat  = floor(pos mod beatsPerBar) + 1
phase = pos − floor(pos)
led   = ledAnchorBeats + (t − ledAnchorMs) · tempo / 60000        // `beats` in the pad formula
```

- **Read the time on fetch.** Use `session.state_now()` for the `state` command, not
  `state()`. It stamps `atMs` at fetch time, so `t` is right however old the state is.
  The error is then only the IPC latency.
- **When the section loops or changes,** the engine re-anchors and a new state follows.
  Between the two, `pos` can briefly run past the section's end.
- **The position clamps at 0.** A `t` a hair before the anchor (a client clock behind the
  session's, or a state read just as a section starts) would give a negative `pos`; it
  reads as 0, the section's start. `ClockState::position` does the same, and `bar`/`beat`
  are never below 1.
- **In Rust,** `ClockState::at(t)`, `position(t)` and `led_beats(t)` compute the same
  thing in ms. `ns_to_ms(session.now_ns())` is "now": the virtual clock offline.

### `io`
| Field | Type | Meaning |
|---|---|---|
| `outputPort` | string | The virtual MIDI output, `yahaha`. Empty offline. |
| `inputs` | string[] | The MIDI sources connected now. The Launchkey DAW port is listed with ` (pads)`. |
| `sources` | MidiSource[] | Every MIDI source but yahaha's own: `name` (as `setMidiInputs` matches it), `listening` (yahaha listens to it, as a keyboard or as the pads), `pads` (the Launchkey DAW port). Empty offline. |
| `allInputs` | bool | Every source is a keyboard (`setMidiInputs { all: true }`, `--all-inputs`). |
| `soundFonts` | string[] | The `.sf2` files in the synth's folder, for `setSoundFont`. |
| `soundFontFile` | string? | The file the synth plays. Null without the synth. |
| `soundFontLoading` | bool | A `setSoundFont` is loading. |
| `synth` | SynthState? | `soundFont`, `device`, `sampleRate` (Hz), `bufferFrames`, `channels`, `outputPair` (1-based, for example [1, 2]), `muted`. Null when the synth is off. |
| `engine` | EngineStats | `realtime` (the engine thread got real-time scheduling), and 99th percentiles in µs: `wakeP99Us` (wake versus deadline), `chordP99Us` (chord to engine), `midiInP99Us` (MIDI in to callback). |
| `lastControl` | number | The last Launchkey DAW-port message, packed 0x00SSDDVV. |
| `unmapped` | string | The last Launchkey control nothing is mapped to, for example `unmapped CC 51 = 127`. |
| `offline` | bool | An offline session. |

### `keyboard`
What the app's keyboard strip draws.

| Field | Type | Meaning |
|---|---|---|
| `held` | HeldNote[] | The keys held, from any keyboard source, low to high: `note` (as played, before Keyboard transpose and the parts' octaves), `zone` (`left` \| `right`: the side of the split it went to when pressed), `parts` (the keyboard parts sounding it, 0–3 = Right 1, Right 2, Right 3, Left; empty for a key that only gives the chord). |
| `leftSplit` | MIDI note | Split Point (Left): keys at or below it play the Left part. yahaha has one split, so it equals `chord.split`. |
| `chordTones` | number[] | Pitch classes (0–11, C = 0) of the chord as fingered (`chord.fingered`), root first. Empty for none. |
| `chordBass` | number? | Its bass: the root, or the slash / on-bass note. |
| `detection` | [lo, hi] | The keys chord detection reads, as MIDI notes (inclusive): `[0, split]` in Lower, `[split + 1, 127]` in Upper (Fingered*), `[0, 127]` in the Full Keyboard types. Clip it to the keys you draw. |

### `controllers`
Pedals, wheels and the assignable functions (docs/controllers.md).

| Field | Type | Meaning |
|---|---|---|
| `pedals` | PedalState[] | Always 3: `cc` (the control change it listens for, or null), `function` (its assignable function's id), `controlType` (`holdA` \| `holdB` \| `toggle`), `reverse`, `range` (`upper` \| `lower` \| `full`), `down` (held now). |
| `learning` | number? | The pedal waiting for its CC (`learnPedal`), or null. |
| `parts` | PartControllers[] | Right 1, Right 2, Right 3, Left: `sustain` (the pedal switches reach it), `pitchBend`, `modulation`, `bendRange` (semitones, 0–12). |
| `sustain`, `sostenuto`, `soft` | bool | The pedal switches in effect now. |

The table of assignable functions is static: `app/src/lib/api/assignable-functions.json`
(`id`, `name`, `category`, `kind`: `switch` \| `trigger` \| `continuous`, `available`).
A Rust test keeps it equal to `controllers::FUNCTIONS`.

### `preview`
The style browser's preview and queue.

| Field | Type | Meaning |
|---|---|---|
| `audition` | object? | The preview playing (`auditionStyle`): `id` (the library id), `bar` (1-based) of `bars` (4), `chord` (the chord playing: `C`, `Am`, `F`, `G7`). Null when none. |
| `queued` | number? | The library id of a style waiting for the next bar line (`loadStyle`, `queueStyle` or `stepStyle` while playing). Null when none. |

### `multiPad`
Multi Pads (docs/multipad.md).

| Field | Type | Meaning |
|---|---|---|
| `bank` | object? | The bank loaded: `id` (in `banks`), `name` (the file name without `.pad`), `path`. Null when none. |
| `loading` | bool | A bank is on its way to the engine (`loadMultiPad`). |
| `pads` | MultiPadPad[] | Always 4: `index` (0–3), `name` (from the file; empty for an empty pad), `lamp` (`empty` \| `ready` \| `armed` \| `queued` \| `playing`: off, blue, red flashing, waiting for the bar line, red), `repeat`, `chordMatch`, `channel` (the MIDI channel it plays on, 5–8). |
| `synchroStop` | object | `styleStop`, `ending` (`setMultiPadSynchroStop`). |
| `banks` | MultiPadBankEntry[] | The `.pad` files in the style folders, folder then name: `id`, `name`, `folder` (relative to its root, `/`-separated), `path`. A `rescanLibrary` refreshes it; a file still there keeps its id. Banks loaded by path from outside the style folders follow, while their file is there; the bank loaded is always listed. |

### `message`
`{ seq, text, error }` or null. It holds the last notice or error, for example a style
that fails to load. `seq` increases with every new message, so the same text arriving
twice counts as two messages. A successful style change clears it, and so does
`clearMessage`.

## Meters

`session.meters()` (the Tauri `meters` command) returns the output levels, measured on
the audio thread as the synthesizer mixes each part (its voices after the part's volume,
expression and pan; the reverb and chorus are shared by the parts, so a part's level does
not include them). It is not part of `AppState`: levels change with
every audio buffer, and republishing the state for them would flood the clients. Poll it
at display rate.

| Field | Type | Meaning |
|---|---|---|
| `atMs` | ms | The session clock at the read. |
| `channels` | `{ channel, peak }[]` | Channels 1–4 (the keyboard parts) and 9–16 (the Style parts): the peak since the last call, linear (1.0 = full scale), after the master level, before the soft clipper. Empty without the synth. |
| `master` | [l, r] | The peaks after the soft clipper. |
| `clips` | number | Audio buffers in which the soft clipper worked (above −1 dBFS), since start. |

Each call takes the peaks (they restart from 0), so use one reader, and do the decay and
peak hold in the client.

## Events

JSON form `{"type": …}`:
- `stateChanged { version }`: `state()` has a new version.
- `libraryChanged { revision }`: `library()` changed. This happens while indexing (at
  most every 250 ms), when a file fails to load, when a path is added, and after a
  rescan.
- `stopped`: the session stopped.

Events carry no state. Always read the latest.

## Example `AppState`

This is a real offline session on SlowWalker, from `state_now()`: playing Main A with
Fill In BB queued, with OTS 1 recalled. Some lists are shortened here:
- `lamps` and `pads` have 16 entries.
- `styleParts` has 8.
- `ots.settings` lists every OTS in the style.
- `surface.controls` has 17 and `surface.faders` has 9.

The `library`, `surface.trackPrev`/`trackNext`, the master fader and `io` show what a
live session reports with a library folder, a Launchkey and the synth.

Unabridged fixtures from `yahaha state-json` are in `docs/fixtures/`: `state.json` (SlowWalker,
after `"C Am F G7"`) and `library.json` (`corpus/MOX_v2`).

```json
{
  "version": 6,
  "style": {
    "id": 12,
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
    "sectionBars": 4,
    "tempo": 75.0,
    "lamps": [
      {
        "note": 96,
        "label": "INTRO 1",
        "key": "q",
        "rgb": [127, 95, 0],
        "level": "dim",
        "anim": "solid",
        "action": { "type": "intro", "index": 0 },
        "palette": null
      },
      {
        "note": 97,
        "label": "INTRO 2",
        "key": "w",
        "rgb": [127, 95, 0],
        "level": "dim",
        "anim": "solid",
        "action": { "type": "intro", "index": 1 },
        "palette": null
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
      "fader": null,
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
      "fader": null,
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
      "fader": null,
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
      "fader": null,
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
        "fader": null,
        "voice": { "bankMsb": 127, "bankLsb": 0, "program": 57, "kit": true, "label": "drum kit 127/0/58" }
      },
      {
        "name": "Rhythm 2",
        "channel": 10,
        "on": true,
        "mutedByManualBass": false,
        "volume": 70,
        "waiting": false,
        "fader": null,
        "voice": { "bankMsb": 127, "bankLsb": 0, "program": 56, "kit": true, "label": "drum kit 127/0/57" }
      },
      {
        "name": "Bass",
        "channel": 11,
        "on": true,
        "mutedByManualBass": false,
        "volume": 74,
        "waiting": false,
        "fader": null,
        "voice": {
          "bankMsb": 104,
          "bankLsb": 18,
          "program": 87,
          "kit": false,
          "label": "≈ Finger Bass  [Yamaha 104/18/88]"
        }
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
        "action": { "type": "main", "index": 0 },
        "palette": null
      },
      {
        "note": 113,
        "label": "MAIN B",
        "key": "2",
        "rgb": [0, 127, 16],
        "level": "bright",
        "anim": "flash",
        "action": { "type": "main", "index": 1 },
        "palette": null
      }
    ],
    "connected": true,
    "paletteLeds": false
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
  "library": { "revision": 3, "count": 35, "position": 23, "pending": 0, "roots": ["/Users/me/Styles/MOX_v2"], "scanning": false },
  "surface": {
    "shift": false,
    "controls": [
      {
        "id": "padBankUp",
        "cc": 106,
        "label": "",
        "action": null,
        "shiftLabel": "LEFT",
        "shiftAction": { "type": "togglePart", "part": 3 },
        "rgb": [0, 0, 0],
        "level": "off",
        "anim": "solid",
        "colour": 0
      },
      {
        "id": "padBankDown",
        "cc": 107,
        "label": "PAGE ▼",
        "action": { "type": "setPadPage", "page": "chordSetup" },
        "shiftLabel": "OTS LINK",
        "shiftAction": { "type": "toggleOtsLink" },
        "rgb": [127, 127, 127],
        "level": "bright",
        "anim": "solid",
        "colour": 3
      },
      {
        "id": "play",
        "cc": 115,
        "label": "PLAY",
        "action": { "type": "startStop" },
        "shiftLabel": "PLAY",
        "shiftAction": { "type": "startStop" },
        "rgb": [0, 0, 0],
        "level": "off",
        "anim": "solid",
        "colour": null
      },
      {
        "id": "faderButton1",
        "cc": 37,
        "label": "RIGHT 1",
        "action": { "type": "togglePart", "part": 0 },
        "shiftLabel": "EDIT R1",
        "shiftAction": { "type": "selectPart", "part": 0 },
        "rgb": [0, 0, 127],
        "level": "bright",
        "anim": "solid",
        "colour": 45
      },
      {
        "id": "masterButton",
        "cc": 45,
        "label": "PANEL",
        "action": { "type": "toggleFaderPage" },
        "shiftLabel": "PANEL",
        "shiftAction": { "type": "toggleFaderPage" },
        "rgb": [0, 0, 127],
        "level": "bright",
        "anim": "solid",
        "colour": 45
      }
    ],
    "faders": [
      {
        "label": "RIGHT 1",
        "value": 100,
        "waiting": false,
        "position": null,
        "set": { "type": "setPartVolume", "part": 0, "volume": 0 }
      },
      { "label": "", "value": null, "waiting": false, "position": null, "set": null },
      {
        "label": "MASTER",
        "value": 100,
        "waiting": false,
        "position": 100,
        "set": { "type": "setMasterVolume", "volume": 0 }
      }
    ],
    "trackPrev": { "id": 2, "name": "Poppyhanger style", "path": "/Users/me/Styles/MOX_v2/Poppyhanger.T552.sty" },
    "trackNext": { "id": 21, "name": "SmoothItOver.S837.STY", "path": "/Users/me/Styles/MOX_v2/SmoothItOver.S930.STY" },
    "clock": {
      "atMs": 2300.0,
      "running": true,
      "tempo": 75.0,
      "beatsPerBar": 4.0,
      "bar": 1,
      "beat": 3,
      "phase": 0.875,
      "sectionAnchorMs": 0.0,
      "sectionAnchorBeats": 0.0,
      "ledAnchorMs": 0.0,
      "ledAnchorBeats": 0.0
    }
  },
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
    "offline": false,
    "sources": [
      { "name": "Launchkey MK4 61 MIDI Out", "listening": true, "pads": false },
      { "name": "Launchkey MK4 61 DAW Out", "listening": true, "pads": true },
      { "name": "IAC Driver Bus 1", "listening": false, "pads": false }
    ],
    "allInputs": false,
    "soundFonts": ["GeneralUser-GS.sf2", "MuseScore_General.sf2"],
    "soundFontFile": "GeneralUser-GS.sf2",
    "soundFontLoading": false
  },
  "preview": { "audition": null, "queued": null },
  "keyboard": {
    "held": [
      { "note": 43, "zone": "left", "parts": [] },
      { "note": 47, "zone": "left", "parts": [] },
      { "note": 50, "zone": "left", "parts": [] },
      { "note": 53, "zone": "left", "parts": [] }
    ],
    "leftSplit": 54,
    "chordTones": [7, 11, 2, 5],
    "chordBass": 7,
    "detection": [0, 54]
  },
  "multiPad": {
    "bank": { "id": 0, "name": "Demo", "path": "/Users/me/Styles/Pads/Demo.pad" },
    "loading": false,
    "pads": [
      { "index": 0, "name": "Shaker Loop", "lamp": "playing", "repeat": true, "chordMatch": false, "channel": 5 },
      { "index": 1, "name": "Rise Arp", "lamp": "ready", "repeat": false, "chordMatch": true, "channel": 6 },
      { "index": 2, "name": "Bass Riff", "lamp": "queued", "repeat": true, "chordMatch": true, "channel": 7 },
      { "index": 3, "name": "Brass Hit", "lamp": "armed", "repeat": false, "chordMatch": true, "channel": 8 }
    ],
    "synchroStop": { "styleStop": true, "ending": false },
    "banks": [{ "id": 0, "name": "Demo", "folder": "Pads", "path": "/Users/me/Styles/Pads/Demo.pad" }]
  },
  "controllers": {
    "pedals": [
      { "cc": 64, "function": "sustain", "controlType": "holdA", "reverse": false, "range": "upper", "down": true },
      { "cc": 66, "function": "fillUp", "controlType": "holdA", "reverse": false, "range": "upper", "down": false },
      { "cc": null, "function": "none", "controlType": "holdA", "reverse": false, "range": "upper", "down": false }
    ],
    "learning": null,
    "parts": [
      { "sustain": true, "pitchBend": true, "modulation": true, "bendRange": 2 },
      { "sustain": true, "pitchBend": true, "modulation": true, "bendRange": 2 },
      { "sustain": true, "pitchBend": true, "modulation": true, "bendRange": 2 },
      { "sustain": false, "pitchBend": true, "modulation": false, "bendRange": 2 }
    ],
    "sustain": true,
    "sostenuto": false,
    "soft": false
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
  - A style preview is an `Engine` of its own, built on the control side and handed to
    the engine thread through a ring; it plays there beside the (stopped) band and goes
    back through another ring to be freed. A style change while playing waits inside the
    engine for the bar line; the style it replaces goes back the same way.
    `tests/engine_no_alloc.rs` checks both allocate and free nothing on the engine thread.
  - The input thread keeps each key's state (held, side, parts) and each source's held
    keys in atomics for the key strip; the control side reads them.
- The audio thread measures each part's and the master's peak into atomics (`meters`).
  A new SoundFont (`setSoundFont`) loads on a thread of its own into a new pair of
  synthesizers (the band's and the keyboard parts'), which the control side hands to the
  audio thread through a ring; the old pair comes back through another ring and is freed
  on the control side.
- A rescan (`rescanLibrary`) walks the folders on a thread of its own.
- The synth's audio stream lives on a thread of its own, which keeps `Session` `Send`.
