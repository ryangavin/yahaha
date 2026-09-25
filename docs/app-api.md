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
| `#[tauri::command] fn sounds() -> SoundCatalog` | `session.sound_catalog()` (the sound catalog; see [`sounds`](#sounds)) |
| `#[tauri::command] fn meters() -> Meters` | `session.meters()` (output levels; see [Meters](#meters)) |
| a thread that emits events to the webview | `for e in session.subscribe() { app.emit("yahaha", e) }` |

The frontend listens for `yahaha` events. On `stateChanged` it fetches `state()`. On
`libraryChanged` it fetches `library()`, and on `soundsChanged` it fetches `sounds()`. Events can arrive at up to about 100 per second
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
| `intro` | `index` 0–2 | Intro 1–3. Stopped: plays at the start. Playing: queued for its change point (see Section Change Timing below). |
| `main` | `index` 0–3 | Main A–D. Pressing the Main that is playing plays its fill. With Auto Fill on, a change plays the fill first. |
| `break` | | Break (Fill In BA). |
| `fill` | `delta` −1, 0, 1 | Fill Down, Fill Self, Fill Up (the Genos assignable functions): the fill, then the Main to the left, the same Main, or the Main to the right, whatever Auto Fill says. Past Main A or D, the fill of the Main at the end. Stopped: selects that Main. |
| `ending` | `index` 0–2 | Ending 1–3. Pressing the Ending that is playing again adds a ritardando (`transport.ritardando`): the tempo slows to 65% by the ending's end, and comes back when the band stops. |
| `startStop` | | START/STOP. |
| `stop` | | Stops if playing, otherwise does nothing. This is the Launchkey Stop button. |
| `toggleSyncStart` | | SYNC START on/off. |
| `toggleSyncStop` | | SYNC STOP on/off. The engine ignores it while `transport.syncStopAvailable` is false. |
| `toggleAutoFill` | | AUTO FILL IN on/off. |
| `toggleStopAcmp` | | STOP ACMP on/off: Off, or back to the mode last on (`style` at first). |
| `setStopAcmp` | `mode`: `off` \| `style` \| `fixed` | Stop Accompaniment (Style Setting > Stop ACMP): with the band stopped and Sync Start off, the chord you play sounds on nothing (`off`), on the style's Bass and Pad voices (`style`), or on fixed ones, Finger Bass and Warm Pad (`fixed`, GM 34 and 90 on the Bass and Pad channels; the style's voices go back when the band starts or the mode changes). The chord is recognised in every mode. |
| `fillUp`, `fillDown` | | Fill Up / Fill Down (Genos assignable functions): a fill, then the next Main to the right / left that the style has. At Main D (A) it plays that Main's own fill. Stopped: selects that Main. |
| `fillSelf` | | Fill Self: the Main's own fill, as pressing the Main playing. |
| `fillBreak` | | Fill Break: the Break (the same as `break`). |
| `setHalfBarFill` / `toggleHalfBarFill` | `on` | Half Bar Fill In: a Main change or fill asked for on the first beat of a bar plays a fill from the middle of that bar (beat 3 in 4/4), then the Main at the next bar line, even with Auto Fill off. |
| `tapTempo` | | TAP TEMPO. Taps set the tempo, from the second tap (the last four averaged), stopped or playing. While the style plays with `styleSettings.sectionReset` on (off by default), a tap is a Style Section Reset instead. |
| `tempoUp`, `tempoDown` | | One tempo step. |
| `toggleFade` | | FADE IN/OUT. Stopped: arms (or disarms) a fade in for the next start. Playing: fades out over `styleSettings.fadeOutMs`, then the band stops and the Style stays silent for `fadeHoldMs`. Only the Style fades: each Style part's CC7 (channels 9–16) goes out, on the port and to the built-in synth, as its fader value scaled by the fade; the faders don't move, and your playing and the Multi Pads never fade (docs/section-timing.md). `transport.fade` shows it. A fade out already running carries on; START/STOP mid-fade ends it at full volume. |
| `sectionReset` | | Style Section Reset: the section playing starts again from its top, now. A change queued for the next bar line waits for the new bar grid's. Stopped: nothing. |
| `toggleRetrigger` | | Style Retrigger on/off (`transport.retrigger`). While on, each chord played in a Main restarts the Main at the chord and loops its first `4 / styleSettings.retriggerRate` beats (a whole note .. a 32nd) until a section change, a style change or Retrigger goes off; off, the Main plays on from there. The same chord struck again (after letting go) counts as a chord played. Only Mains retrigger. |
| `setTempo` | `bpm` | Sets the tempo. The range is 5–500 BPM (Genos, OM p.46); values outside are clamped. |
| `toggleStylePart` | `part` 0–7 | Mutes or unmutes a Style part. |
| `setStylePartVolume` | `part` 0–7, `volume` 0–127 | The part's CC7. The Launchkey fader has to reach the new value before it takes over again. |
| `setStyleSolo` | `part` 0–7 or null | Solos a Style part: only it plays, even if it is switched off; the other parts' notes stop. `null` ends the solo. The on/off switches are not changed (`mixer.styleSolo`). |
| `styleTrackMute` | `order` `a` \| `b`, `value` 0–127 | Style Track Mute, a Genos Live Control knob (RM p.148). `value` is the knob: fully left (0) leaves one part on, and turning up adds parts until all eight are on at 127. Order A: Rhythm 2, Rhythm 1, Bass, Chord 1, Chord 2, Pad, Phrase 1, Phrase 2. Order B: Chord 1, Chord 2, Pad, Bass, Phrase 1, Phrase 2, Rhythm 1, Rhythm 2. It sets the parts' on/off switches. |

### Style settings

Genos Menu › Style Setting (Section Change Timing, Synchro Stop Window), Tap Tempo ›
Style Section Reset, the Fade In/Out times and the Style Retrigger length. The state is
`styleSettings`.

| Command | Fields | Does |
|---|---|---|
| `setMainTiming` | `timing`: `immediate` \| `nextBar` | Section Change Timing, To Main A–D; also a style change while playing. **nextBar** (default): at once when pressed within the first beat of a bar (the new section starts from that point of its bar), otherwise at the next bar line. **immediate**: at the next beat; the new section carries on from that beat of its bar. A Main change with Auto Fill In on is always nextBar. |
| `setIntroEndingTiming` | `timing`: `nextBar` \| `endOfSection` | Section Change Timing, Inside Intro/Ending: changing to another Intro or Ending while one plays. **nextBar** (default): as above. **endOfSection**: when the Intro or Ending playing has finished. Intro to Intro is always nextBar. Into Ending I, and from a Main into an Intro or Ending, the change waits for the next bar line. |
| `setSyncStopWindow` | `ms` 0–5000 | Synchro Stop Window. 0 = Off. With Sync Stop on, a chord held longer than this turns Sync Stop off, so letting go no longer stops the band; a quicker release stops it. |
| `setFadeInTime`, `setFadeOutTime` | `ms` 0–20000 | Fade In and Fade Out times. |
| `setFadeHoldTime` | `ms` 0–5000 | How long the volume stays at 0 after a fade out. |
| `setSectionReset` | `on` | TAP TEMPO while the style plays: Section Reset (on, the Genos default) or set the tempo (off, yahaha's default). |
| `setRetriggerRate` | `rate` | Style Retrigger length: 1, 2, 4, 8, 16 or 32 (a whole note .. a 32nd). Other values snap down to one of these. |
| `stepRetriggerRate` | `delta` | Steps along 1, 2, 4, 8, 16, 32; positive is shorter. Stops at the ends. |

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
| `setChordSettle` | `ms` | The chord-settle window, clamped to 0–30 ms (default 10). While the style plays (and, with it stopped, for Stop Accompaniment and Chord Match Multi Pads), a chord change reaches the accompaniment once the chord has held still this long (at most three windows after the first change), so a rolled chord is followed once. 0: at once. Not a Genos setting; see docs/genos-features.md (Chord settle). |

### Keyboard parts

| Command | Fields | Does |
|---|---|---|
| `setPartOn` / `togglePart` | `part` 0–3, `on` | Turns a part on or off. Left is refused while Manual Bass is in effect, with a `Failed` error and a message. |
| `selectPart` | `part` | The part that `stepVoice` and the Launchkey voice pads edit. |
| `setPartVoice` | `part`, `program` 0–127 | GM program. |
| `stepVoice` | `delta` | Previous or next voice for the selected part. |
| `setPartVolume` | `part`, `volume` 0–127 | The part's CC7. The Launchkey fader has to reach it before it takes over. |
| `setPartOctave` | `part`, `octave` −2..2 | Octave shift. |
| `setPartSolo` | `part` 0–3 or null | Solos a keyboard part: only it sounds from the keys, even if it is switched off (Left soloed plays the left hand; another part soloed plays the whole keyboard when Left is not sounding). `null` ends it. The switches are not changed (`mixer.partSolo`). |

### Mixer, Launchkey pages, synth

| Command | Fields | Does |
|---|---|---|
| `setFaderPage` / `toggleFaderPage` | `page`: `panel` \| `style` | What the Launchkey faders control. |
| `setPadPage` | `page`: `sections` \| `chordSetup` \| `otsParts` \| `registration` | The Launchkey pad page. |
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
| `setDefaultSoundSet` | `file` (string or null) | The default sound set (#117): the SoundFont that plays whatever the program map leaves unmapped (Style parts' and keyboard parts' GM voices). A `.sf2` in the SoundFont folder (`io.soundFonts`, by file name), or null for Auto: the most GM-complete font there (`io.autoSoundSet`: the most GM programs on bank 0, then a drum kit on bank 128, then the first file name). The choice is saved in `sound-settings.json` in the data folder. When the synth plays another font, the new one loads on a thread of its own (`io.soundFontLoading`) and swaps in between two audio buffers: the voices and controllers every channel has carry over, notes sounding fade out over one buffer. Fails when the file isn't in the folder. Without the synth it is only saved. |
| `setSoundFont` | `file` | `setDefaultSoundSet` with that file, kept for older clients. Fails when the synth is off. |
| `setMidiInputs` | `all`, `names` | Which MIDI sources play the keyboard: every one (`all`), or the ones whose name contains one of `names`. `all` false with no names is the default: a Launchkey's keys when there is one, else every source. yahaha's own port and DAW ports are never keyboards; the Launchkey DAW port is always the pads. Sources connect and disconnect at once. Keys held on a source that is dropped are released: their notes stop at once (All Notes Off on the keyboard parts' channels, which also stops notes other sources hold) and the chord section lets go. |
| `setPaletteLeds` | `on` | Launchkey LEDs in Novation palette colours (and hardware flashing) instead of RGB. Every pad is sent again. |
| `setAudioBuffer` | `frames` 64, 128 or 256 | The synth's audio buffer (`io.synth.bufferFrames`; within what the device allows, and a message says so when it differs). The output reopens with a moment of silence; the voices, the plugins and held notes carry over, and messages sent meanwhile wait for the new stream (nothing sticks). Plugins are loaded for larger blocks already, so none reloads. A live session remembers it (`~/Library/Application Support/yahaha/audio.json`; `--buffer N` at launch wins). Fails when the synth is off or for another size. |
| `rescanLibrary` | | Walks the style folders (`library.roots`) again on a thread of its own (`library.scanning`). A file still there keeps its id and index; new files are added and indexed; a file gone leaves the list (its id stays valid). |

### One Touch Settings and styles

| Command | Fields | Does |
|---|---|---|
| `recallOts` | `index` 0–3 | Recalls OTS 1–4 into the keyboard parts. Ignored if the style has no such OTS. |
| `setOtsLink` / `toggleOtsLink` | `on` | OTS Link: Main A–D recall OTS 1–4, and so does a style change. |
| `setOtsLinkTiming` | `timing`: `immediate` \| `mainChange` | OTS Link Timing: during playback, recall the Main's OTS as it is pressed (`immediate`), or when that Main starts playing (`mainChange`, the default: at its change point, or after its fill; never while the old section still plays). Stopped, both recall at once. A style change recalls the new style's OTS when that style takes over (the bar line or beat Section Change Timing gives, or the end of an Ending), under both. |
| `loadStyle` | `id` | A library entry (`LibraryEntry.id`). Stopped, it loads at once. Playing, it takes over at the next bar line, as on a Genos: the band carries on in the same section (the same Main, or the nearest the new style has) at the same bar position, at the same tempo. Until then `preview.queued` names it and `style` is still the old one. A later style change before the bar line replaces it; stopping first loads it then. |
| `queueStyle` | `id` | The same as `loadStyle` (the browser's "next bar" button). |
| `loadStylePath` | `path` | Any style file. It is added to the library if it isn't there already. |
| `stepStyle` | `delta` | Previous or next style in library order, from the style waiting for the bar line if there is one. Files that don't load are skipped. |
| `auditionStyle` | `id` | Previews a style while the band is stopped: its Main A, at its own tempo, with its own voices and levels, over C Am F G7 (a chord a bar) for 4 bars, then it stops by itself (`preview.audition`). The loaded style, OTS, keyboard parts, mixer and transport are untouched; the loaded style's setup is sent again when it ends. Refused (`failed`) while the band plays. A new one replaces the one playing; it ends early on `stopAudition`, a style change, START/STOP, `panic` or a chord that starts the band (Sync Start). |
| `stopAudition` | | Ends the preview now. |

### Style change behaviour

Style Setting > Change Behavior (RM p.12–13): what choosing another style does.

| Command | Fields | Does |
|---|---|---|
| `setTempoChange` | `rule`: `lock` \| `hold` \| `reset` | Tempo. `lock`: keep the tempo. `hold`: keep it while the band plays, take the new style's when stopped (the default). `reset`: always take the new style's. |
| `setPartsChange` | `rule` | Style part on/off, the same three rules (`hold` and `reset` turn every part on). Default `hold`. |
| `setSectionSet` | `section` 0–3 or null | Section Set: the Main (A–D) a style chosen while stopped starts on (the nearest Main it has), or null (Off, the default) to keep the Main selected. |
| `toggleStyleTempoLock` | | The assignable "Style Tempo Lock/Reset": Tempo `reset` becomes `lock`, anything else becomes `reset`. |
| `toggleStyleTempoHold` | | The assignable "Style Tempo Hold/Reset": Tempo `reset` becomes `hold`, anything else becomes `reset`. |

### iReal Pro chart player

The band takes its chords, and its Main sections, from an iReal Pro chart instead of the
left hand ([ireal.md](ireal.md), "Chart player"). Playlists live in the session's memory
(nothing is saved).

| Command | Fields | Does |
|---|---|---|
| `importCharts` | `text` | Imports playlists from an `irealb://` / `irealbook://` link, or the text of an exported `.html` playlist, into `chart.playlists`, e.g. `{"type":"importCharts","text":"irealb://..."}`. With no song chosen yet, it chooses the first one imported. Fails when there is no link in the text. |
| `importChartFile` | `path` | The same, reading a file. |
| `selectChart` | `playlist`, `song` | Chooses the chart the band plays (`chart.selected`, `chart.song`). It suggests a library style from the chart's style label (`chart.suggestedStyle`) and, with `chart.autoStyle`, loads it as `loadStyle` would. With the band stopped, the tempo becomes the chart's when it has one; a playing band starts the new song from its first bar at the next bar line. The loop is cleared. |
| `stepChart` | `delta` | The previous / next song of the playlist. |
| `removeChartPlaylist` | `playlist` | Forgets a playlist. If the chosen song was in it, there is no chart any more and chart mode turns off. |
| `setChartMode` / `toggleChartMode` | `on` | Chart mode (`chart.on`). While the band plays, the chart gives the chords at their places in the bar (beat / beats of the chart bar) and the Mains at its section marks. Turning it on with no chart fails. Only one of the chart and the Chord Looper gives the chords: turning chart mode on stops a loop that plays or is armed. |
| `setChartChoruses` | `choruses` 1–99 | Times through the form. The chart is expanded again; a playing band keeps its bar. A loop past the new last bar is cleared. |
| `setChartLoop` | `range` | Loops bars `[start, end)` of `chart.song.bars` instead of ending (e.g. `{"type":"setChartLoop","range":[8,16]}`); `null` for no loop. Fails for bars the chart doesn't have. |
| `setChartIntro` | `index` | The Intro 0–2 (A–C) before the chart, or `null` for none. An Intro pressed before the start plays instead. |
| `setChartEnding` | `index` | The Ending 0–2 after the last bar, or `null`: the band stops at the end of the last bar. |
| `setChartAutoStyle` | `on` | Load the suggested style whenever a song is chosen. |

### Registration Memory

Buttons are 0-based (`index` 0–9 = the panel's [1]–[10]). Groups are `style`, `voice`,
`harmonyArp`, `multiPad`, `tempo`, `transpose`, `chordLooper`, `liveControl`, `assignable`
(the Genos Freeze groups; docs/registration.md lists what each covers).

| Command | Fields | Does |
|---|---|---|
| `pressRegist` | `index` | A REGISTRATION MEMORY button (the Launchkey pads send this): recalls it, or memorizes into it while MEMORY is armed. |
| `recallRegist` | `index` | Recalls a button: the groups it memorized, less the frozen ones while Freeze is on. The style comes first; when it changes, the rest follows once the new style plays (at once when stopped, at the next bar line when playing; `registration.pending` meanwhile). Refused if the button is empty. |
| `memorizeRegist` | `index` | Stores the panel (the `memorizeGroups`) in a button, replacing what it held. A saved bank is written to its file at once. |
| `toggleRegistMemory` | | The MEMORY button: the next `pressRegist` memorizes. |
| `setMemorizeGroup` | `group`, `on` | Ticks a group in the Memory window. |
| `clearRegist` | `index` | Empties a button. |
| `renameRegist` | `index`, `name` | Renames a button. |
| `stepRegistBank` | `delta` | REGIST BANK −/+: the previous/next bank file in the folder (stops at the ends). Loading a bank recalls nothing. |
| `selectRegistBank` | `path` | Loads a bank file. |
| `newRegistBank` | | A new, empty, unsaved bank. |
| `saveRegistBank` | `name` (null: its own file), `overwrite`? | Saves the bank; with a name, as a file of that name in the folder. Refused when another bank already has that file, unless `overwrite: true`. Fails without a data folder. |
| `setFreeze` / `toggleFreeze` | `on` | Registration Freeze. |
| `setFreezeGroup` | `group`, `on` | Ticks a group on the Freeze display: it stays unchanged on recall while Freeze is on. |
| `setRegistSequence` | `steps` (buttons 0–9), `end`: `stop` \| `top` \| `next` | Programs the bank's Registration Sequence. |
| `setRegistSequenceOn` / `toggleRegistSequence` | `on` | Registration Sequence on/off. A panel setting, not part of the bank (as on the Genos): it stays when the bank changes, and is kept in the Registration folder's `setup.json`. |
| `stepRegistSequence` | `delta` | Regist +/−: recalls the next/previous step. Past the end: `stop` stays, `top` wraps, `next` loads the next bank and recalls its first step. Refused while the sequence is off. |

### Playlist

Record `index` is a record's position in the playlist file (`PlaylistRow.index`), whatever
the display order. A record is `{ "name", "kind": "bank", "path", "regist"? }` (a bank
file, and the button to recall after loading it) or `{ "name", "kind": "style", "path" }`.

| Command | Fields | Does |
|---|---|---|
| `newPlaylist` | | A new, empty, unsaved playlist. |
| `loadPlaylist` | `path` | Opens a playlist file. |
| `savePlaylist` | `name` (null: its own file), `overwrite`? | Saves in the displayed order and sets the sort back to `normal`; with a name, as a file of that name in the folder (refused when another playlist has it, unless `overwrite: true`). |
| `addPlaylistRecord` | `record` | Adds a record at the end (at most 2,500). An empty name takes the file's. |
| `addCurrentBank` | | Adds the bank in use (it must be saved), recalling the lit button. |
| `addCurrentStyle` | | Adds the loaded style. |
| `appendPlaylist` | `path` | Adds every record of another playlist file. |
| `setPlaylistRecord` | `index`, `record` | Replaces a record (Record Edit). |
| `movePlaylistRecord` | `index`, `delta` | Up (−1) / Down (+1). Refused while sorted. |
| `deletePlaylistRecord` | `index` | Refused while sorted. |
| `setPlaylistSort` | `sort`: `normal` \| `aToZ` \| `zToA` | Display order. |
| `loadPlaylistRecord` | `index` | Loads its bank and recalls its button, or loads its style. |
| `stepPlaylist` | `delta` | Loads the previous/next record in display order (Shift + Track ◀/▶). Does nothing on an empty playlist. |

### Chord Looper

Genos CHORD LOOPER (RM p.14–19): record a chord progression while the style plays, then
loop it; the looper feeds its chords to the style as if they were played. Recording, loop
playback and a memory change start at the next bar line; stopping the loop is immediate.
Details and decisions: [chord-looper.md](chord-looper.md).

| Command | Fields | Does |
|---|---|---|
| `looperRec` | | REC/STOP. Playing: recording starts at the next bar line, with the chord held then as its first. Stopped: Sync Start turns on and the first chord starts the style and the recording together. Recording: stops recording (the style plays on). Armed: cancels. While looping: the loop stops and recording arms. |
| `looperOnOff` | | ON/OFF. Recording: recording stops (the bars recorded, counting the one playing) and the loop starts at the next bar line. With a sequence: the loop starts at the next bar line (stopped: when the style starts). Armed: cancels. Looping: the loop stops at once and the style keeps the loop's chord until a chord is played. With chart mode on, an ON/OFF that would arm a loop turns chart mode off first. |
| `selectLooperMemory` | `index` 0–7 | Selects a memory. One that holds a sequence replaces the current one; while looping, at the next bar line (`looper.pendingMemory` until then). Refused while recording. |
| `storeLooperMemory` | `index` 0–7 | Stores the current sequence in the memory (named `CLD_001` and on). Refused with nothing recorded. |
| `clearLooperMemory` | `index` 0–7 | Empties the memory. |
| `newLooperBank` | | Empties all eight memories. The current sequence stays. |

### Metronome

| Command | Fields | Does |
|---|---|---|
| `setMetronome` / `toggleMetronome` | `on` | Metronome on/off. It clicks on every beat, with the style while it plays and free-running at the tempo while stopped. The click sounds on the built-in synth only, never on the MIDI port. |
| `setMetronomeVolume` | `volume` 0–127 | The click's own volume (the synth master applies on top). |
| `setMetronomeBell` | `on` | A bell on the first beat of each bar. |

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

### Instrument plugins

Audio Unit instruments for the keyboard parts (docs/plugin-hosting.md). They need a build
with the `plugins` feature (the desktop app has it) and the built-in synth
(`plugins.available`).

| Command | Fields | Does |
|---|---|---|
| `setPartPlugin` | `part` 0–3, `id`, `state`? | Plays the part on an instrument plugin: `id` from `plugins.list` (for example `"aumu dls  appl"`), `state` a saved preset (base64) or null for the plugin's default. It loads in the background (`keyboardParts[i].plugin.status` `loading`, with the `stage`). The part keeps its SoundFont voice until the plugin is ready, then switches without a click. If the load fails, a plugin that was playing keeps the part; otherwise the part plays its SoundFont voice (`failed`, with the `error`), and picking the plugin again with a null `state` retries it with the state it kept (a restore that timed out, or a plugin reinstalled since, comes back as saved; go back to the SoundFont voice first to start it fresh). A state over 64 MB is refused. Fails at once for an unknown id or with no synth. |
| `clearPartPlugin` | `part` 0–3 | Back to the part's SoundFont voice (a 5 ms fade). |
| `savePartPluginState` | `part` 0–3 | Stores the plugin's current preset (what its editor changed) with the part, so it is kept across restarts. Send it when the editor window closes. The state is read on a thread of its own and lands a moment later; a failed read shows in `message`. |
| `rescanPlugins` | | Scans the installed instruments again, ignoring the cache (`plugins.scanning` meanwhile). |
| `setPluginInProcess` | `id`, `inProcess` | Runs plugin `id` in yahaha's process (`true`) or in its own (`false`, the default for third-party plugins). In process saves the IPC cost per render for the lightest plugins, but a crash in the plugin takes yahaha down. Kept in the scan cache (across rescans and plugin updates) and shown as `plugins.list[i].inProcess`. It applies from the plugin's next load; a part playing it now keeps running where it is (the message line says so). Fails for an unknown id, or for an AUv3 that only runs out of process (`canRunInProcess` false). |

The plugin's editor window is not a command: it opens on the app's main thread. The
Tauri shell has the commands `open_plugin_editor(part)` and `close_plugin_editor(part)` for it
(`Session::plugin_editor` gives the shell the handle).

### Keyboard Harmony / Arpeggio

One HARMONY/ARPEGGIO switch and one type, as on the Genos: a Keyboard Harmony type or an
arpeggio pattern, never both. The type lists are in `LibraryList` (`harmonyTypes`,
`arpPatterns`). What each does: docs/harmony.md, docs/arpeggio.md.

| Command | Fields | Does |
|---|---|---|
| `toggleHarmonyArp` / `setHarmonyArpOn` | `on` | The HARMONY/ARPEGGIO switch. Turning it off (or changing the type) stops the arpeggio and the Echo repeats at once; keys held keep their harmony notes until they go up. |
| `setHarmonyType` | `index` | A Keyboard Harmony type, by its index in `harmonyTypes` (Data List order). Selects the Harmony list. |
| `setArpPattern` | `index` | An arpeggio pattern, by its index in `arpPatterns`. Selects the arpeggio list. |
| `stepHarmonyArpType` | `delta` | Steps through the Harmony types and then the arpeggios, as one list, wrapping. |
| `setHarmonyVolume` | `volume` 0–127 | Volume: the level of the added notes (127 = the key's velocity) and of the arpeggio. |
| `setHarmonySpeed` | `speed` | Echo, Tremolo and Trill: `1/4`, `1/6`, `1/8`, `1/12`, `1/16` or `1/32`. |
| `setHarmonyAssign` | `assign` | `auto`, `multi`, `right1`, `right2` or `right3`: the Right parts the effect (and the arpeggio) sounds on. `multi` is for the Harmony and Echo categories only (RM p.46); an arpeggio plays it as `auto`. |
| `setChordNoteOnly` | `on` | Harmony category: harmonise only melody notes of the current chord. |
| `setTouchLimit` | `velocity` 1–127 | The effect sounds only for keys played at least this hard (Minimum Velocity). |
| `setArpQuantize` | `quantize` | `off`, `eighth` or `sixteenth`: the grid the arpeggio starts on. |
| `setArpHold` / `toggleArpHold` | `on` | The Arpeggio Hold setting (RM p.41): the pattern plays on after the keys are released, until the switch goes off or Hold is turned off. |
| `setArpPedalHold` / `toggleArpPedalHold` | `on` | The Arpeggio Hold pedal function (RM p.141), apart from the setting: the pattern plays on after the keys are released while it is on, and stops when it goes off. A pedal on Arpeggio Hold sends these (Hold A / Hold B set it, Toggle and the function's Try switch it); it never changes the setting. PANIC and an unplugged keyboard turn it off where a Hold pedal was keeping it on. |
| `setArpVelocity` | `mode`, `velocity` | `original` (the pattern's accents), `thru` (each key's velocity) or `fixed` (every note at `velocity`, 1–127). |
| `setArpKeepKeyOn` | `on` | Keep Key On: the pattern clock runs on through a full release, so the next chord picks up in phase. |

### Sound library
The sound library (#103, docs/sound-library.md): a short user list of patches, and the
program map that sends every Style part (and every keyboard part's GM voice) to one of
them. It is saved to `sound-library.json` in the data folder (`soundLibrary.file`) after
every change. A patch id that doesn't exist fails the command.

| Command | Fields | What it does |
|---|---|---|
| `createPatch` | `patch`: PatchFields | Adds a patch at the end of the list; its new id is `soundLibrary.lastAdded`. PatchFields: `name`, `category`, `tags`, `favourite`, `source`, `defaults` (see [`soundLibrary`](#soundlibrary)). |
| `updatePatch` | `id`, `patch` | Replaces a patch's fields (rename, recategorise, tags, favourite, source, defaults); the id stays. |
| `deletePatch` | `id` | Deletes it. Map rules that name it go; a keyboard part playing it goes back to its GM voice. |
| `duplicatePatch` | `id` | A copy ("… copy") right after it, with a new id. |
| `movePatch` | `id`, `to` | Moves it to position `to` (0-based) in the list. |
| `setPatchFavourite` | `id`, `favourite` | Marks or unmarks a favourite. |
| `savePartAsPatch` | `part` 0–3, `name` or null | Saves a keyboard part's sound as a new patch: its own patch, else its GM voice on the synth's SoundFont, with its volume and octave as defaults. |
| `addPresetAsPatch` | `file`, `bank`, `program`, `name` or null | Adds a SoundFont preset (`browseSoundFont`) as a patch, named after the preset and categorised from its bank and program. |
| `auditionPatch` | `id` | Plays the patch on its own for about 3 s (an arpeggio and a chord; a drum kit plays a beat), on channel 16 of the built-in synth, which the band is not using while stopped. A plugin patch first loads its plugin there (#91's rack), then plays; the plugin goes when the audition ends. Refused while the band plays (like `auditionStyle`); `soundLibrary.auditioning` names it. |
| `auditionPreset` | `file`, `bank`, `program` | The same for a SoundFont preset, before adding it. A SoundFont the synth hasn't loaded loads first. |
| `stopPatchAudition` | | Ends the audition now. |
| `setFamilyRule` | `family` 0–15, `patch` or null, `style` | A GM family (programs 8·family … 8·family+7) plays `patch`; null clears the rule. `style`: the current style's own map instead of the global one (may be left out: false). In the three rule commands `patch` may also be a [sound catalog](#sound-catalog) id: a saved sound's patch, or a preset or plugin, which becomes a library patch the first time (a plugin with its default preset). |
| `setProgramOverride` | `program` 0–127, `patch` or null, `style` | One GM program plays `patch`, whatever its family's rule. |
| `setDrumRule` | `patch` or null, `style` | The drum parts (Rhythm 1 and 2, and any part on a Yamaha drum kit bank, MSB 126/127) play `patch`. |
| `clearStyleMap` | | Forgets the current style's own map. |
| `setPartPatch` | `part` 0–3, `id` or null | A keyboard part plays a library patch; its defaults (volume, octave, pan, reverb and chorus sends) go to the part as CCs. Null: back to its GM voice (through the map). `setPartVoice`, `stepVoice` and an OTS recall that gives the part a voice also end it. A plugin patch loads its plugin with the patch's state, as `setPartPlugin` does (the part's `plugin` shows it loading, then playing); leaving the patch takes that plugin away. `setPartPlugin` (and `clearPartPlugin` while a plugin patch plays) ends the part's patch; a SoundFont patch picked over a `setPartPlugin` plugin ends that plugin. |
| `setPortSendsMapped` | `on` | The `yahaha` MIDI port gets the mapped bank and program for the band's program changes the map sends to a SoundFont patch, instead of the style's own (default off: the port mirrors the style). |
| `browseSoundFont` | `file` or null | Lists a SoundFont's presets in `soundLibrary.browse` (a file in `io.soundFonts`); null closes the list. |
| `importSoundLibrary` | `path`, `replace`, `maps` | Reads a library file (a full library, or a bare list of patches). Its patches are added (ids that clash get new ones); `maps`: its program maps' rules are added too; `replace`: it replaces the library instead. `replace` and `maps` may be left out (false). |
| `exportSoundLibrary` | `path` or null | Writes the library to `path` (null: `sound-library-export.json` in the data folder). |

### Parameter Lock
Genos Menu › Utility › Parameter Lock (RM p.163): a locked group changes only from the
panel. Registration Memory, One Touch Setting and Playlist recalls leave it as it is. The
groups are the Data List's lock groups that yahaha has: `splitPoint` (the split point) and
`fingeringType` (the fingering type and the Chord Detection Area: Upper, Manual Bass).

| Command | Fields | What it does |
|---|---|---|
| `setParamLock` | `item` (`splitPoint` \| `fingeringType`), `on` | Locks or unlocks a group. A setup setting, not part of a bank: it is kept in the Registration folder's `setup.json`. |

### Sound catalog
One list of every sound for the Sound Browser (#117): every preset of every `.sf2` in the
SoundFont folder, every instrument plugin, and every saved sound (the sound library's
patches). Entry ids: `sf:<file>:<bank>:<program>`, `au:<component id>`, `saved:<patch id>`.
Favourites, Recents and plugin categories are saved in `sound-settings.json` in the data
folder.

| Command | Fields | What it does |
|---|---|---|
| `setSoundFavourite` | `id`, `on` | Marks or unmarks a favourite. A saved sound's favourite is its patch's `favourite`. |
| `auditionSound` | `id` | Plays the sound on its own for about 3 s, as `auditionPatch` does (a plugin plays its default preset). Refused while the band plays. `sounds.auditioning` names it. |
| `stopSoundAudition` | | Stops the audition. |
| `assignSound` | `part` 0–3, `id` | The keyboard part plays the sound. A preset of the default sound set (bank 0) becomes the part's voice (`setPartVoice`). A preset of another font becomes a saved sound (the library's patch for it, added once) and plays as `setPartPatch`. A plugin plays as `setPartPlugin` (its default preset), and a saved sound as `setPartPatch`. The sound goes to the top of the Recents (20 kept). |
| `setSoundCategory` | `id`, `category` | A plugin's category (the guess from its name and maker until set), or a saved sound's (its patch's). A preset's category is its GM family: refused. |

The list itself is fetched, not in the state: see [`sounds`](#sounds).

The program map's rule commands (`setFamilyRule`, `setProgramOverride`, `setDrumRule`) also
take a catalog id as their `patch`, so the map's pickers pick from the same list.

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
| `autoFill`, `stopAcmp` | bool | Auto Fill In, and Stop Accompaniment sounding (`stopAcmpMode` is not `off`). |
| `section` | string? | The section playing, for example `Main A` or `Fill In AA`. Null when stopped. |
| `queued` | string? | The section queued next: at the next bar, or for a fill, at the next beat. |
| `pendingIntro` | 0–2? | The Intro armed to play at the start. |
| `main` | 0–3 | The Main (A–D) that is playing or queued to follow. Changes as soon as a Main is pressed. |
| `bar`, `beat` | 1-based | Position within the section playing. Both are 1 when stopped. |
| `beatsPerBar` | number | The numerator of the time signature. |
| `sectionBars` | number? | How many bars the section playing lasts (a Main's pattern length; it loops). Null when stopped. |
| `tempo` | number | Current tempo in BPM. |
| `lamps` | Pad[16] | Page 1 of the pads, whatever page the hardware is on. These are the section, Sync, Auto Fill, Tap and Start/Stop lamps exactly as the pads light them. See [Pad](#pad). |
| `halfBarFill` | bool | Half Bar Fill In. |
| `stopAcmpMode` | `off` \| `style` \| `fixed` | Stop Accompaniment (`setStopAcmp`). |
| `fade` | `off` \| `armed` \| `fadingIn` \| `fadingOut` \| `holding` | Fade In/Out (`toggleFade`). `armed`: stopped, START fades in. `holding`: faded out and stopped, silent for the hold time. |
| `retrigger` | bool | Style Retrigger is on (`toggleRetrigger`). |
| `ritardando` | bool | The Ending is slowing down (pressed again while it plays). |

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
| `settleMs` | 0–30 | The chord-settle window in ms (`setChordSettle`). |

### `keyboardParts`: always four, Right 1, Right 2, Right 3, Left
| Field | Type | Meaning |
|---|---|---|
| `name` | string | `Right 1` … `Left`. |
| `channel` | 1–16 | Right 1 = 1, Left = 2, Right 2 = 3, Right 3 = 4. The same on the MIDI port and in the synth. |
| `on` | bool | The part's switch. |
| `sounding` | bool | The part sounds: it is on, or it is Left playing the bass under Manual Bass; while a keyboard part is soloed, only that part. Light the part's lamp from this. |
| `selected` | bool | The part the voice commands edit. |
| `volume` | 0–127 | CC7. |
| `waiting` | bool | The Launchkey fader has moved but not yet reached `volume`. The terminal UI shows ↕. |
| `program` | 0–127 | The part's GM voice. |
| `voiceName` | string | What its channel plays: its own patch's name, the patch its GM voice maps to, or the GM voice. For Left under Manual Bass, that is the Style's Bass voice. |
| `playsBass` | bool | Left is playing the bass (Manual Bass). |
| `octave` | −2..2 | The octave setting. It is not applied while `playsBass` is true. |
| `fader` | 0–127? | Where its Launchkey fader (Panel page, faders 1–4) physically is, as last reported. Null until that fader moves. |
| `plugin` | PartPlugin? | The instrument plugin the part plays instead of its SoundFont voice. The key is absent when there is none. `id`, `name`, `manufacturer`, `status` (`loading` \| `playing` \| `failed` \| `muted`: still on the SoundFont, or the previous plugin, while loading; on the SoundFont after a failed load, keeping the choice so it is saved and can be retried; silent after the plugin crashed or produced bad audio), `stage` (while loading: `queued`, `instantiating`, `initializing`, `restoringState`), `error`, `outOfProcess` (runs in its own process), `inProcessFallback` (the system refused to host it in its own process, so it loaded in yahaha's process instead: a crash in it takes yahaha down; the app shows a warning badge), `cpu` (share of real time, updated once a second), `overruns` (renders slower than half the buffer), `editor` (its window can be opened). Its volume is still `volume` (CC7), and its pan is CC10; the host applies both to the plugin's output. |
| `patch` | string? | Its own sound library patch (`setPartPatch`). Null: its GM voice plays, through the program map; `voiceName` then names the patch the map sends it to, if any. |

### `mixer`
| Field | Type | Meaning |
|---|---|---|
| `faderPage` | `panel` \| `style` | What the Launchkey faders control. Panel: faders 1–4 are the keyboard parts. Style: faders 1–8 are the Style parts. |
| `styleParts` | StylePart[8] | See the table below. |
| `master` | 0–127? | The synth master level (100 = unity). Null without the synth. |
| `masterWaiting` | bool | The master fader has not yet reached `master`. It turns on as soon as `setMasterVolume` moves the level away from the fader. |
| `styleSolo` | 0–7? | The Style part soloed (`setStyleSolo`): only it plays. Null when none. |
| `partSolo` | 0–3? | The keyboard part soloed (`setPartSolo`). Null when none. |

StylePart:

| Field | Type | Meaning |
|---|---|---|
| `name`, `channel` | | `Rhythm 1` on channel 9 through `Phrase 2` on channel 16. |
| `on` | bool | Not muted, and not muted by Manual Bass. (A solo does not change it: see `mixer.styleSolo`.) |
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
| `linkTiming` | `immediate` \| `mainChange` | OTS Link Timing (default `mainChange`). |

### `library`
| Field | Type | Meaning |
|---|---|---|
| `revision` | number | The revision `library()` returns. It changes when the library changes: fetch `library()` when it does. All four fields describe that same list. |
| `count` | number | Entries. |
| `position` | number | The loaded style's position in library order, 0-based. |
| `pending` | number | Entries still being indexed. |
| `roots` | string[] | The style folders (and files) the library scans. |
| `scanning` | bool | A rescan (`rescanLibrary`) is walking the folders. |

`library()` returns `LibraryList { revision, entries, voices, harmonyTypes, arpPatterns }`.
`entries` are in display order (folder, then name). `voices` is the list `setPartVoice`
picks from, the same for every revision: `{ program, bankMsb, bankLsb, name }`, the 128 GM
voices on bank 0 (the names are `gm_name`'s). `harmonyTypes` (the 23 Keyboard Harmony
types in Data List order, for `setHarmonyType`) and `arpPatterns` (yahaha's arpeggio
patterns, for `setArpPattern`) are `{ name, category }` and never change. Each
`LibraryEntry` has these fields:
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
| `soundFonts` | string[] | The `.sf2` files in the SoundFont folder (`--soundfonts DIR`, the app's `YAHAHA_SOUNDFONTS`; `soundfonts/` by default). |
| `soundFontFile` | string? | The file the synth plays as its default sound set. Null without the synth. |
| `soundFontLoading` | bool | A `setDefaultSoundSet` is loading. |
| `defaultSoundSet` | string? | The default sound set chosen (`setDefaultSoundSet`). Null: Auto. |
| `autoSoundSet` | string? | The font Auto picks from the folder. Null when there are no fonts. |
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

### `chart`
The iReal Pro chart player.

| Field | Type | Meaning |
|---|---|---|
| `on` | bool | Chart mode. |
| `playlists` | ChartPlaylist[] | The imported playlists: `name`, `songs` (`title`, `composer` as iReal stores it, `style` (iReal's label, e.g. `Bossa Nova`), `key` (`Eb`, `A-` for minor), `tempo` (BPM, or null)). |
| `selected` | [playlist, song]? | The song chosen. |
| `song` | ChartSong? | The chart chosen: the `playlists` song fields, plus `bars` (the form played `choruses` times through: `section` (`A`, `B`, `V`, `i`, or null), `sectionStart`, `main` (the Main it plays, 0–3), `time` ([4, 4]), `chorus` (1-based), `chords` (`{ beat, name }`, beat 0-based; a bar with none holds the chord before)) and `sections` (runs of bars: `label`, `chorus`, `start`, `bars`). |
| `choruses` | number | Times through the form (1–99). |
| `intro`, `ending` | number? | Intro / Ending 0–2 around the chart, or null. |
| `loop` | [start, end]? | The bars looped, or null. |
| `autoStyle` | bool | Choosing a song loads its suggested style. |
| `suggestedStyle` | number? | The library style the chart's style label suggests (`LibraryEntry.id`). |
| `bar` | number? | The bar of `song.bars` playing. Null when stopped, in the Intro or the Ending, or with chart mode off. |
| `overridden` | bool | A chord you played has taken over until the next bar line (through the next bar too, when played in the last half beat before its line). |

### `styleSettings`
The settings the `Style settings` commands set.

| Field | Type | Meaning |
|---|---|---|
| `mainTiming` | `immediate` \| `nextBar` | Section Change Timing, To Main. Default `nextBar`. |
| `introEndingTiming` | `nextBar` \| `endOfSection` | Section Change Timing, Inside Intro/Ending. Default `nextBar`. |
| `syncStopWindowMs` | 0–5000 | Synchro Stop Window; 0 = Off (the default). |
| `fadeInMs`, `fadeOutMs` | 0–20000 | Default 5000 each. |
| `fadeHoldMs` | 0–5000 | Default 2000. |
| `sectionReset` | bool | TAP TEMPO while playing resets the section. Default off (the Genos's is on). |
| `retriggerRate` | 1, 2, 4, 8, 16, 32 | Style Retrigger length. Default 8 (an eighth note). |

### `registration`

| Field | Type | Meaning |
|---|---|---|
| `bank` | object | The bank in use: `name`, `path` (null until saved), `dirty` (changed since loaded or saved), `position` (its place in `banks`). |
| `banks` | {name, path}[] | The bank files in the folder, in order. |
| `folder` | string? | Where banks are saved (`<data dir>/Registration`); null when saving is off. |
| `buttons` | RegistButton[10] | `index`, `stored`, `name`, `groups` (what it memorized), `style` (name)?, `tempo`?, `voices` ({name, on} for Right 1, Right 2, Right 3, Left; empty when it stores no parts). |
| `selected` | 0–9? | The button last recalled or memorized (the red lamp). |
| `memory` | bool | MEMORY is armed. |
| `memorizeGroups`, `freezeGroups` | Group[] | The ticked groups. |
| `freeze` | bool | Registration Freeze is on. |
| `sequence` | object | `on`, `steps` (buttons), `end` (`stop` \| `top` \| `next`), `position` (the step last recalled)? |
| `pending` | bool | A recall waits for its style to take over (the bar line). |

### `playlist`

| Field | Type | Meaning |
|---|---|---|
| `name`, `path`?, `dirty` | | The playlist in use. |
| `sort` | `normal` \| `aToZ` \| `zToA` | Display order. |
| `records` | PlaylistRow[] | In display order: `index` (file position), `record`, `missing` (its file isn't there). |
| `current` | number? | The record last loaded (file position). |
| `playlists` | {name, path}[] | The playlist files in the folder. |
| `folder` | string? | Where playlists are saved (`<data dir>/Playlists`). |

### `looper`
The Chord Looper.

| Field | Type | Meaning |
|---|---|---|
| `mode` | `off` \| `recArmed` \| `recording` \| `loopArmed` \| `looping` | `recArmed`: REC/STOP flashing, recording starts at the next bar line (stopped: with the first chord). `recording`: REC/STOP lit. `loopArmed`: ON/OFF flashing, the loop starts at the next bar line (stopped: when the style starts). `looping`: ON/OFF lit, the keyboard's chords are ignored (the ACMP lamp flashes on a Genos). `off` with `hasData`: ON/OFF lit blue. |
| `hasData` | bool | There is a sequence to loop. |
| `bar` | number? | Recording: the bar being recorded; looping: the loop's bar playing (1-based). |
| `bars` | number | Recording: bars so far; otherwise the sequence's length. |
| `chords` | LoopChord[] | The current sequence (empty while recording): `bar` (1-based), `beat` (1-based quarter notes; 2.5 is the "and" of 2), `chord` (as fingered, e.g. `Cm7`). |
| `memory` | 0–7? | The memory selected. A new recording is in no memory until stored. |
| `pendingMemory` | 0–7? | A memory selected while looping, taking over at the next bar line. |
| `memories` | LooperMemory[8] | `name` (`CLD_001`…, null when empty), `bars`, `chords` (LoopChord[]). |

### `metronome`
| Field | Type | Meaning |
|---|---|---|
| `on` | bool | The metronome is on. |
| `volume` | 0–127 | The click's volume. |
| `bell` | bool | A bell on the first beat of each bar. |
| `audible` | bool | The built-in synth is running: the only place the click sounds. |

### `plugins`
The instrument plugin host.

| Field | Type | Meaning |
|---|---|---|
| `available` | bool | Plugins can be used: the build hosts them and the built-in synth runs. |
| `scanning` | bool | A scan is running. |
| `list` | PluginEntry[] | The installed instrument Audio Units, by manufacturer then name, from the cached scan: `id` (what `setPartPlugin` takes), `name`, `manufacturer`, `version`, `format` (`AUv2` \| `AUv3`), `lastError` (why the last load failed, or null), `inProcess` (the player chose to run it in yahaha's process: `setPluginInProcess`), `canRunInProcess` (every AUv2, and an AUv3 that allows it). |

### `multiPad`
Multi Pads (docs/multipad.md).

| Field | Type | Meaning |
|---|---|---|
| `bank` | object? | The bank loaded: `id` (in `banks`), `name` (the file name without `.pad`), `path`. Null when none. |
| `loading` | bool | A bank is on its way to the engine (`loadMultiPad`). |
| `pads` | MultiPadPad[] | Always 4: `index` (0–3), `name` (from the file; empty for an empty pad), `lamp` (`empty` \| `ready` \| `armed` \| `queued` \| `playing`: off, blue, red flashing, waiting for the bar line, red), `repeat`, `chordMatch`, `channel` (the MIDI channel it plays on, 5–8). |
| `synchroStop` | object | `styleStop`, `ending` (`setMultiPadSynchroStop`). |
| `banks` | MultiPadBankEntry[] | The `.pad` files in the style folders, folder then name: `id`, `name`, `folder` (relative to its root, `/`-separated), `path`. A `rescanLibrary` refreshes it; a file still there keeps its id. Banks loaded by path from outside the style folders follow, while their file is there; the bank loaded is always listed. |

### `harmonyArp`
Keyboard Harmony / Arpeggio (the commands above).

| Field | Type | Meaning |
|---|---|---|
| `on` | bool | The HARMONY/ARPEGGIO switch. |
| `mode` | `harmony` \| `arpeggio` | Which list the selected type is in. |
| `harmonyType` | number | The Harmony type (index into `harmonyTypes`), kept while an arpeggio is selected. |
| `arpPattern` | number | The arpeggio pattern (index into `arpPatterns`). |
| `typeName`, `category` | string | The selected type's name and category (`Harmony`, `Echo`, or the pattern's, such as `Up & Down`). |
| `volume` | 0–127 | Volume of the added notes and the arpeggio. |
| `speed` | string | Echo-category speed, `1/4` … `1/32`. |
| `assign` | string | `auto`, `multi`, `right1`, `right2`, `right3`. |
| `chordNoteOnly` | bool | Harmony category: only chord tones are harmonised. |
| `touchLimit` | 1–127 | Minimum Velocity. |
| `arp` | object | `quantize` (`off` \| `eighth` \| `sixteenth`), `hold` (the setting), `pedalHold` (the Arpeggio Hold pedal function is on; the arpeggio holds while either is), `velocity` (`original` \| `thru` \| `fixed`), `fixedVelocity`, `keepKeyOn`. |

### `soundLibrary`
The sound library (docs/sound-library.md).

| Field | Type | Meaning |
|---|---|---|
| `patches` | PatchInfo[] | In the user's order: `id`, `name`, `category`, `tags`, `favourite`, `source`, `defaults`, `available` (false: it plays the SoundFont fallback) and `note` (why, e.g. "needs plugin hosting (#91)"). `source` is `{ "kind": "soundFont", "file", "bank", "program" }` (bank 128 = drum kits) or `{ "kind": "plugin", "componentId", "state" }` (the Audio Unit's id, as #91 writes it, and its state, base64). `defaults`: `volume`, `pan`, `reverb`, `chorus` (0–127 or null) and `octave` (−2..2). |
| `categories` | object[] | The Genos voice categories in display order: `id` (`piano`, `ePiano`, `organ`, `guitar`, `bass`, `strings`, `brass`, `saxWoodwind`, `synthLead`, `pad`, `choir`, `drumsPerc`, `sfx`) and `label`. |
| `families` | string[16] | The GM family names; family `i` is programs 8i … 8i+7. |
| `map` | ProgramMap | The global map: `families` (16 patch ids or null), `overrides` (`{ program, patch }`, by program) and `drums` (a patch id or null). |
| `styleMap` | ProgramMap | The current style's own map (empty: none). Its rules win over the global map's; what it leaves unset falls through. |
| `styleKey` | string | What the style's own map is stored under: its file name. |
| `usage` | ProgramUse[] | Every program the current style sends its parts (its setup and every section), by channel: `channel` (9–16), `part`, `msb`, `lsb`, `program`, `gmProgram` (what the map looks up), `voice` (the voice without the library), `drums`, `patch` (null: the fallback), `rule` (`drums` \| `override` \| `family` \| `fallback`), `fromStyle`, `plays` (the patch's name, or the voice). |
| `portSendsMapped` | bool | `setPortSendsMapped`. |
| `auditioning` | string? | The patch id being auditioned, or `preset`. |
| `browse` | object? | The SoundFont being browsed: `file`, `presets` (`{ bank, program, name }`), `error`. |
| `file` | string? | Where the library is saved; null when it isn't (an offline session, `state-json`). |
| `extraSoundFonts` | string[] | The SoundFonts the synth has loaded for library patches besides its own. |
| `lastAdded` | string? | The id of the patch last created, duplicated or saved. |

### `sounds`
The sound catalog's summary (#117; the list is `sounds()`, see [Sound catalog](#sound-catalog)).

| Field | Type | Meaning |
|---|---|---|
| `revision` | number | Moves whenever the catalog changes (fonts, plugins, saved sounds, favourites, Recents, categories). |
| `count` | number | Entries in the catalog. |
| `scanning` | bool | Plugins are being scanned: more may come. |
| `auditioning` | string? | The id being auditioned (`sf:`, `au:` or `saved:`), or null. |

The catalog itself: `session.sound_catalog()` (Tauri `sounds()`)
returns `{ revision, entries, recents }`. Fetch it again when `sounds.revision` moves
(`soundsChanged`). `recents` lists ids, most recent first. Each entry has:

| Field | Type | Meaning |
|---|---|---|
| `id` | string | `sf:<file>:<bank>:<program>`, `au:<component id>` or `saved:<patch id>`. |
| `name` | string | The preset's, the plugin's or the patch's name. |
| `category` | string | One of `soundLibrary.categories`. A preset's comes from its GM family (bank 128: `drumsPerc`), and a plugin's from its name and maker. |
| `source` | `soundFont` \| `plugin` \| `saved` | Where it comes from. |
| `detail` | string | The SoundFont file, the plugin's maker, or what a saved sound plays (its file or component id). |
| `favourite`, `recent` | bool | In the Favourites, in the Recents. |
| `plugin` | object? | Plugins only: `format` (`AUv2` \| `AUv3`) and `lastError` (the last load's error, or null). |

Entries are in this order: presets by file, then bank and program; plugins by maker, then
name; saved sounds in the library's order.

### `paramLocks`
Parameter Lock: `{ splitPoint, fingeringType }`, each a bool (true: locked). All false by
default.

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
- `soundsChanged { revision }`: the sound catalog changed; fetch `sounds()`.
- `stopped`: the session stopped.

Events carry no state. Always read the latest.

## Example `AppState`

This is a real offline session on SlowWalker, from `state_now()`: playing Main A with
Fill In BB queued, with OTS 1 recalled. Some lists are shortened here:
- `lamps` and `pads` have 16 entries.
- `styleParts` has 8.
- `ots.settings` lists every OTS in the style.
- `surface.controls` has 17 and `surface.faders` has 9.
- `registration.buttons` has 10.

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
    ],
    "halfBarFill": false,
    "stopAcmpMode": "off",
    "fade": "off",
    "retrigger": false,
    "ritardando": false
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
    "transposeMaster": 0,
    "settleMs": 10
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
      "octave": -1,
      "plugin": {
        "id": "aumu dls  appl",
        "name": "DLSMusicDevice",
        "manufacturer": "Apple",
        "status": "playing",
        "stage": null,
        "error": null,
        "outOfProcess": false,
        "inProcessFallback": false,
        "cpu": 0.015625,
        "overruns": 0,
        "editor": true
      },
      "patch": null
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
      "octave": 0,
      "patch": null
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
      "octave": 0,
      "patch": null
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
      "octave": 1,
      "patch": null
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
    "masterWaiting": false,
    "styleSolo": null,
    "partSolo": null
  },
  "pads": {
    "page": "sections",
    "pageName": "Sections",
    "pageNumber": 1,
    "pageCount": 4,
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
    "link": false,
    "linkTiming": "mainChange"
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
    "soundFontLoading": false,
    "defaultSoundSet": null,
    "autoSoundSet": null
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
  "styleChange": { "tempo": "hold", "parts": "hold", "sectionSet": null },
  "chart": {
    "on": false,
    "playlists": [],
    "selected": null,
    "song": null,
    "choruses": 1,
    "intro": 0,
    "ending": 0,
    "loop": null,
    "autoStyle": true,
    "suggestedStyle": null,
    "bar": null,
    "overridden": false
  },
  "styleSettings": {
    "mainTiming": "nextBar",
    "introEndingTiming": "nextBar",
    "syncStopWindowMs": 0,
    "fadeInMs": 5000,
    "fadeOutMs": 5000,
    "fadeHoldMs": 2000,
    "sectionReset": false,
    "retriggerRate": 8
  },
  "registration": {
    "bank": { "name": "Friday Gig", "path": "/Users/me/Documents/yahaha/Registration/Friday Gig.regist.json", "dirty": false, "position": 0 },
    "banks": [
      { "name": "Friday Gig", "path": "/Users/me/Documents/yahaha/Registration/Friday Gig.regist.json" },
      { "name": "Jazz Set", "path": "/Users/me/Documents/yahaha/Registration/Jazz Set.regist.json" }
    ],
    "folder": "/Users/me/Documents/yahaha/Registration",
    "buttons": [
      {
        "index": 0,
        "stored": true,
        "name": "SlowWalker",
        "groups": ["style", "voice", "harmonyArp", "multiPad", "tempo", "transpose", "chordLooper", "liveControl", "assignable"],
        "style": "SlowWalker",
        "tempo": 91.0,
        "voices": [
          { "name": "Grand Piano", "on": true },
          { "name": "Strings", "on": false },
          { "name": "Brass Section", "on": false },
          { "name": "Strings", "on": false }
        ]
      },
      { "index": 1, "stored": false, "name": "", "groups": [], "style": null, "tempo": null, "voices": [] }
    ],
    "selected": 0,
    "memory": false,
    "memorizeGroups": ["style", "voice", "harmonyArp", "multiPad", "tempo", "transpose", "chordLooper", "liveControl", "assignable"],
    "freeze": false,
    "freezeGroups": ["tempo"],
    "sequence": { "on": true, "steps": [0, 2, 1], "end": "next", "position": 0 },
    "pending": false
  },
  "playlist": {
    "name": "Friday",
    "path": "/Users/me/Documents/yahaha/Playlists/Friday.playlist.json",
    "dirty": false,
    "sort": "normal",
    "records": [
      {
        "index": 0,
        "record": { "name": "Opener", "kind": "bank", "path": "/Users/me/Documents/yahaha/Registration/Friday Gig.regist.json", "regist": 0 },
        "missing": false
      },
      {
        "index": 1,
        "record": { "name": "SlowWalker", "kind": "style", "path": "/Users/me/Styles/MOX_v2/SlowWalker.T552.sty" },
        "missing": false
      }
    ],
    "current": 0,
    "playlists": [{ "name": "Friday", "path": "/Users/me/Documents/yahaha/Playlists/Friday.playlist.json" }],
    "folder": "/Users/me/Documents/yahaha/Playlists"
  },
  "looper": {
    "mode": "looping",
    "hasData": true,
    "bar": 2,
    "bars": 4,
    "chords": [
      { "bar": 1, "beat": 1.0, "chord": "C" },
      { "bar": 2, "beat": 1.0, "chord": "Am" },
      { "bar": 3, "beat": 1.0, "chord": "F" },
      { "bar": 4, "beat": 1.0, "chord": "G7" }
    ],
    "memory": 0,
    "pendingMemory": null,
    "memories": [
      {
        "name": "CLD_001",
        "bars": 4,
        "chords": [
          { "bar": 1, "beat": 1.0, "chord": "C" },
          { "bar": 2, "beat": 1.0, "chord": "Am" },
          { "bar": 3, "beat": 1.0, "chord": "F" },
          { "bar": 4, "beat": 1.0, "chord": "G7" }
        ]
      },
      { "name": null, "bars": 0, "chords": [] },
      { "name": null, "bars": 0, "chords": [] },
      { "name": null, "bars": 0, "chords": [] },
      { "name": null, "bars": 0, "chords": [] },
      { "name": null, "bars": 0, "chords": [] },
      { "name": null, "bars": 0, "chords": [] },
      { "name": null, "bars": 0, "chords": [] }
    ]
  },
  "metronome": {
    "on": false,
    "volume": 90,
    "bell": true,
    "audible": true
  },
  "plugins": {
    "available": true,
    "scanning": false,
    "list": [
      { "id": "aumu dls  appl", "name": "DLSMusicDevice", "manufacturer": "Apple", "version": "1.0.0", "format": "AUv2", "lastError": null, "inProcess": false, "canRunInProcess": true }
    ]
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
  "harmonyArp": {
    "on": true,
    "mode": "harmony",
    "harmonyType": 2,
    "arpPattern": 0,
    "typeName": "Standard Trio",
    "category": "Harmony",
    "volume": 100,
    "speed": "1/8",
    "assign": "auto",
    "chordNoteOnly": false,
    "touchLimit": 1,
    "arp": { "quantize": "off", "hold": false, "pedalHold": false, "velocity": "original", "fixedVelocity": 100, "keepKeyOn": false }
  },
  "soundLibrary": {
    "patches": [
      {
        "id": "my-bass",
        "name": "My Bass",
        "category": "bass",
        "tags": ["warm"],
        "favourite": true,
        "source": { "kind": "soundFont", "file": "GeneralUser-GS.sf2", "bank": 0, "program": 33 },
        "defaults": { "volume": 100, "pan": null, "reverb": 20, "chorus": null, "octave": 0 },
        "available": true,
        "note": null
      },
      {
        "id": "keys",
        "name": "Keys",
        "category": "ePiano",
        "tags": [],
        "favourite": false,
        "source": { "kind": "plugin", "componentId": "aumu dls  appl", "state": "" },
        "defaults": { "volume": null, "pan": null, "reverb": null, "chorus": null, "octave": 0 },
        "available": false,
        "note": "needs plugin hosting (#91)"
      }
    ],
    "categories": [{ "id": "piano", "label": "Piano" }, { "id": "bass", "label": "Bass" }],
    "families": ["Piano", "Chromatic Perc.", "Organ", "Guitar", "Bass", "Strings", "Ensemble", "Brass", "Reed", "Pipe", "Synth Lead", "Synth Pad", "Synth FX", "Ethnic", "Percussive", "Sound FX"],
    "map": {
      "families": [null, null, null, null, "my-bass", null, null, null, null, null, null, null, null, null, null, null],
      "overrides": [{ "program": 4, "patch": "keys" }],
      "drums": null
    },
    "styleMap": { "families": [null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null], "overrides": [], "drums": null },
    "styleKey": "SlowWalker.T552.sty",
    "usage": [
      { "channel": 11, "part": "Bass", "msb": 0, "lsb": 0, "program": 33, "gmProgram": 33, "voice": "Finger Bass (GM 34)", "drums": false, "patch": "my-bass", "rule": "family", "fromStyle": false, "plays": "My Bass" }
    ],
    "portSendsMapped": false,
    "auditioning": null,
    "browse": null,
    "file": "/Users/me/Documents/yahaha/sound-library.json",
    "extraSoundFonts": [],
    "lastAdded": "my-bass"
  },
  "paramLocks": { "splitPoint": false, "fingeringType": true },
  "sounds": { "revision": 3, "count": 1219, "scanning": false, "auditioning": null },
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
