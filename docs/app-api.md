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
| `main` | `index` 0–3 | Main A–D. Pressing the Main that is playing plays its fill. With Auto Fill on, a change plays the fill first. Pressed during a fill, a press that would play a fill plays it right after that fill, from its top: tapping every bar loops fills (#229). |
| `break` | | Break (Fill In BA). |
| `fill` | `delta` −1, 0, 1 | Fill Down, Fill Self, Fill Up (the Genos assignable functions): the same as `fillDown`, `fillSelf` and `fillUp`. |
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
| `tapTempo` | | TAP TEMPO. Taps set the tempo, from the second tap (the last four averaged), stopped or playing. Stopped, a bar's worth of steady taps (four in 4/4) also starts the style one beat after the last tap, rhythm only until a chord (OM p.46); `stop` calls that count-in off. While the style plays with `styleSettings.sectionReset` on (the default, as on the Genos), a tap is a Style Section Reset instead and the tempo stays. |
| `tempoUp`, `tempoDown` | | One tempo step (1 BPM). The Launchkey's TEMPO buttons also repeat while held, on the engine; a client that wants that repeats the command itself (the app: 400 ms, then 10, 20 and 40 steps a second). |
| `resetTempo` | | TEMPO − and + together: back to the tempo the style came with (`style.tempo`; OM p.46). A ritardando playing slows on from it. |
| `toggleFade` | | FADE IN/OUT. Stopped: arms (or disarms) a fade in for the next start. Playing: fades out over `styleSettings.fadeOutMs`, then the band stops and the Style stays silent for `fadeHoldMs`. Only the Style fades: each Style part's CC7 (channels 9–16) goes out, on the port and to the built-in synth, as its fader value scaled by the fade; the faders don't move, and your playing and the Multi Pads never fade (docs/section-timing.md). `transport.fade` shows it. A fade out already running carries on; START/STOP mid-fade ends it at full volume. |
| `sectionReset` | | Style Section Reset: the section playing starts again from its top, now. A change queued for the next bar line waits for the new bar grid's. Stopped: nothing. |
| `toggleRetrigger` | | Style Retrigger on/off (`transport.retrigger`). While on, each chord played in a Main restarts the Main at the chord and loops its first `4 / styleSettings.retriggerRate` beats (a whole note .. a 32nd) until a section change, a style change or Retrigger goes off; off, the Main plays on from there. The same chord struck again (after letting go) counts as a chord played. Only Mains retrigger. |
| `toggleAcmp`, `setAcmp` | `on` | [ACMP] on/off (OM p.44, p.47; `transport.acmp`, default on). Off: START plays the rhythm only, chords played change nothing (the chord parts end their notes), Sync Start starts on any key, Sync Stop and Stop Accompaniment have nothing to act on, and the whole keyboard plays the Right parts (with Left on, Left below the split). Turned on, the chord parts come in with the next chord. An OTS recall and Chord Looper REC turn it on. Launchkey: Shift + encoder page ▼; key `%`; assignable function `acmp`. |
| `toggleUnison`, `setUnison`, `setUnisonHeld`, `setUnisonType` | `on`; `unisonType` (`root`, `melody`) | Unison (a PSR-SX feature, not a Genos2 one: docs/genos-features.md §C.9). While engaged (latched with `toggleUnison`/`setUnison`, or held with `setUnisonHeld`), each right-hand key also sounds on the Style's pitched parts in the player's rhythm, and note-offs follow the player's: the Bass plays the chord root (on-bass note if any) in C2–B2, or with `melody` the played key folded there; Chord 1, Chord 2 and Pad play the chord tones just below the played key; Phrase 1 and 2 double the key; the drums play on. Those parts' pattern notes rest while it is engaged and come back at their next notes. Works stopped or playing. Not stored. Assignable function `unison` (Hold A: on while held; Toggle: latches). No Launchkey mapping (the pad pages are full). |
| `setTempo` | `bpm` | Sets the tempo. The range is 5–500 BPM (Genos, OM p.46); values outside are clamped. |
| `toggleStylePart` | `part` 0–7 | Mutes or unmutes a Style part. |
| `setStylePartVolume` | `part` 0–7, `volume` 0–127 | The part's CC7. The Launchkey fader has to reach the new value before it takes over again. |
| `setStylePartSend` | `part` 0–7, `send` `reverb` \| `chorus` \| `variation`, `value` 0–127 | The part's own send (#268): it goes out at once as the part's CC91/93/94, and every CC91/93/94 the style sends on that part goes out at this value instead, through section changes, style changes and restarts, on the MIDI port and in the synth. The block's band send scale (`setBandSend`) doesn't apply to it. |
| `resetStylePartSends` | `part` 0–7 or null | Hand the part's sends (null: every part's) back to the style: the style's last value (or the default, reverb 40, chorus and variation 0, where it set none) goes out, and its own CCs pass as written again. |
| `setStyleVolume` | `volume` 0–127 | The Style volume (the Genos Balance page's Style slider), 100 = as written: every Style part's CC7 goes out multiplied by `volume`/100 (at most 127), as a Fade In/Out scales it; the part levels (`styleParts[].volume`) do not move. It scales the CC7 that's sent, like the Fade, rather than adding gain (AGENTS.md, "Engine rules"). Launchkey Panel fader 5, with soft takeover. Registered with the Style mixer. |
| `setMultiPadVolume` | `volume` 0–127 | The Multi Pad volume (the Genos Balance page's M.Pad slider), 100 = as written: the pads' CC7 on channels 5–8 go out multiplied by `volume`/100 (at most 127); a pad channel whose phrase sets no CC7 counts as 100. It scales CC7 the same way as `setStyleVolume`. Launchkey Panel fader 6, with soft takeover. Registered with the Multi Pad bank. |
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
| `setSwing` | `amount` 0–100 | Live Swing: 0 plays the Style as written; 100 moves straight off-beats (8ths or 16ths, `swingGrid`) to the triplet position. A tick remap of the Style's events as they play (drums and all accompaniment parts; the player's keys are never moved). Parts already swung are not swung again: a triplet off-beat stays put, and positions between scale in proportion. Each style load sets it back to 0; registrations store it (group Style). |
| `stepSwing` | `delta` | Swing moved by `delta` %, clamped to 0–100. |
| `setSwingGrid` | `grid` | 8 (off-beat 8ths, the default) or 16 (off-beat 16ths). |
| `setSectionTempo` | `on` | Play the tempo changes a style writes inside its sections, mostly the ritardandos at the end of Endings and Intros (#243). Default on. They play relative to the tempo playing: an Ending written to slow from the style's 120 to 90 slows from 100 to 75 at 100 BPM. The panel tempo comes back when the section ends or the band stops. TEMPO −/+, TAP and `setTempo` during one move the tempo it is read against; pressing the Ending again (its ritardando) takes over from the tempo reached. |

### Chord detection, split, transpose

| Command | Fields | Does |
|---|---|---|
| `setFingering` | `fingering` | One of `singleFinger`, `multiFinger`, `fingered`, `fingeredOnBass`, `aiFingered`, `fullKeyboard`, `aiFullKeyboard`. |
| `nextFingering` | | Next type in display order, wrapping. The display order is single, fingered, onBass, multi, ai, full, aiFull. |
| `setUpper` / `toggleUpper` | `on` | Chord Detection Area Upper/Lower. Selecting Upper turns Manual Bass on. |
| `setManualBass` / `toggleManualBass` | `on` | The Manual Bass setting. Ignored in Lower. |
| `setSplit` | `note` (MIDI) | Split point, clamped to 24–96. |
| `moveSplit` | `delta` | Moves the split by `delta` keys. |
| `setTranspose` | `keyboard`, `master` | Semitones, each clamped to −12..12. Keyboard moves the keyboard parts at once, but the chord the style follows only from the next chord input (a chord held, or an Intro playing, stays in the old key, as on the Genos; #264). Master moves every note started from now on. |
| `stepTranspose` | `keyboard`, `master` | Adds to the current transpose. |
| `resetTranspose` | | Both back to 0. |
| `setChordSettle` | `ms` | The chord-settle window, clamped to 0–30 ms (default 10). While the style plays (and, with it stopped, for Stop Accompaniment and Chord Match Multi Pads), a chord change reaches the accompaniment once the chord has held still this long (at most three windows after the first change), so a rolled chord is followed once. 0: at once. Not a Genos setting; see docs/genos-features.md (Chord settle). |
| `setLeftHold`, `toggleLeftHold` | `on` | LEFT HOLD (OM p.49): while on, the Left part's notes ring on after its keys are let go (its channel is held as if by a sustain pedal). Each key that sounds on Left lets go of what was held first, so a chord rings until the next one; stopping the style lets go too (the setting stays on). The sustain pedal on Left wins. |

### Keyboard parts

| Command | Fields | Does |
|---|---|---|
| `setPartOn` / `togglePart` | `part` 0–3, `on` | Turns a part on or off. Left is refused while Manual Bass is in effect, with a `Failed` error and a message. |
| `selectPart` | `part` | The part that `stepVoice` and the Launchkey voice pads edit. |
| `setPartVoice` | `part`, `program` 0–127 | GM program. Selecting a voice replaces the part's voice: a plugin picked for the part (`setPartPlugin`) ends and is no longer saved, and its library patch goes (with a plugin that patch plays). |
| `stepVoice` | `delta` | Previous or next voice for the selected part. Like `setPartVoice`, it ends a plugin picked for the part. |
| `setPartVolume` | `part`, `volume` 0–127 | The part's CC7. The Launchkey fader has to reach it before it takes over. |
| `setPartOctave` | `part`, `octave` −2..2 | Octave shift. |
| `setPartPan` | `part`, `pan` 0–127 | The part's pan (CC10; 64 = centre), sent on its channel to the MIDI port and the synth. |
| `setPartSend` | `part`, `send`: `reverb` \| `chorus` \| `variation`, `value` 0–127 | The part's reverb (CC91), chorus (CC93) or variation (CC94: the tempo delay) send depth to the effect bus (#204). |
| `setPartEq` | `part`, `eq`: `{ lowGain, lowFreq, highGain, highFreq }` | The part's channel-strip EQ (#247): a low shelf and a high shelf, as the Genos Mixer's Part EQ. Gains in dB, −12..12; `lowFreq` 32–2000 Hz, `highFreq` 500–16000 Hz; out-of-range values are clamped. yahaha plays it on the part's audio, whatever plays it (SoundFont or plugin), before its level, pan, meters and sends. It is a tone control, not a level: a band at 0 dB is out of the signal, so a flat EQ leaves the part bit-identical. Saved with the rack (a rack without it plays flat). An OTS recall sets it from the OTS's XG part EQ (see `eq` under Keyboard parts). Example: `{"type":"setPartEq","part":0,"eq":{"lowGain":3,"lowFreq":80,"highGain":-2,"highFreq":10000}}`. |
| `setKeyboardInsertEffect` | `part`, `effect`: `distortion` \| `compressor` \| `autoWah` \| `tremolo` \| `rotary` \| `phaser` | *Superseded by `setStripInsertKind` (strip = `part`, slot 0), which still sends this; it keeps working.* The effect in the part's insert slot (Genos Mixer > Effect: Insertion Effect Type). yahaha plays it on the part's audio, whatever plays it (SoundFont or plugin), after its EQ and before its level, pan, meters and sends; the effect sees the part at full volume. It does not turn the slot on. Saved with the rack. `phaser` is accepted but plays dry until its DSP lands (the mixer rework). Example: `{"type":"setKeyboardInsertEffect","part":0,"effect":"rotary"}`. |
| `setKeyboardInsertOn` | `part`, `on` | *Superseded by `setStripInsertOn` (strip = `part`, slot 0); still works.* The part's insert slot on or off. Off, the part plays bit-identical to no insert at all. `setInsertsOn` (the style's inserts) doesn't touch it. |
| `setKeyboardInsertAmount` | `part`, `amount` 0–127 | *Superseded by `setStripInsertSetting` (strip = `part`, slot 0, setting 0); still works.* The insert's amount: the distortion's drive, the compressor's squeeze, the wah's sensitivity, the tremolo's and rotary's depth, the phaser's depth. Out-of-range values are clamped. |
| `setPartSolo` | `part` 0–3 or null | Solos a keyboard part: only it sounds from the keys, even if it is switched off (Left soloed plays the left hand; another part soloed plays the whole keyboard when Left is not sounding). `null` ends it. The switches are not changed (`mixer.partSolo`). |

### Mixer, Launchkey pages, synth

| Command | Fields | Does |
|---|---|---|
| `setFaderPage` / `toggleFaderPage` | `page`: `panel` \| `style` | What the Launchkey faders control. |
| `setFaderLayer` | `layer`: `volume` \| `pan` \| `reverb` \| `chorus` \| `delay` | The fader layer (the mixer's VOL · PAN · REV · CHO · DLY): what the faders move across the parts. Volume: each part's CC7 (as always). A send layer: Panel faders 1–4 move Right 1–3 and Left's pan / CC91 / CC93 / CC94 (as `setPartPan` / `setPartSend`, with soft takeover); Style faders 1–8 move the Style parts' reverb / chorus / delay sends (as `setStylePartSend`, with soft takeover; the Style parts have no pan, so PAN leaves them alone). Faders 5–6 on the Panel page stay the Style and Multi Pad levels, and the master fader stays the master. On the Launchkey, **Shift + the master fader's button** steps the layer; the button alone still switches the page. |
| `stepFaderLayer` | `delta` | The next/previous fader layer, wrapping (VOL → PAN → REV → CHO → DLY → VOL). |
| `setPadPage` | `page`: `sections` \| `chordSetup` \| `otsParts` \| `quickRacks` \| `multiPads` | The Launchkey pad page (`registration`, page 4's old name, is still read as `quickRacks`). |
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
| `setMidiInputs` | `all`, `names` | Which MIDI sources play the keyboard: every one (`all`), or the ones whose name contains one of `names`. `all` false with no names is the default: a Launchkey's keys when there is one, else every source. yahaha's own port and DAW ports are never keyboards; the Launchkey DAW port is always the pads. Sources connect and disconnect at once. Keys held on a source that is dropped are released: their notes stop at once (All Notes Off on the keyboard parts' channels, which also stops notes other sources hold) and the chord section lets go. |
| `setPaletteLeds` | `on` | Launchkey LEDs in Novation palette colours (and hardware flashing) instead of RGB. Every pad is sent again. |
| `setAudioBuffer` | `frames` 64, 128, 256, 512 or 1024 | The synth's audio buffer (`io.synth.bufferFrames`; within what the device allows, and a message says so when it differs). The output reopens with a moment of silence; the voices, the plugins and held notes carry over, and messages sent meanwhile wait for the new stream (nothing sticks). Plugins are loaded for larger blocks already, so none reloads. A live session remembers it (`~/Library/Application Support/yahaha/audio.json`; `--buffer N` at launch wins). Fails when the synth is off or for another size. |
| `rescanLibrary` | | Walks the style folders (`library.roots`) again on a thread of its own (`library.scanning`). A file still there keeps its id and index; new files are added and indexed; a file gone leaves the list (its id stays valid). |

### One Touch Settings and styles

| Command | Fields | Does |
|---|---|---|
| `recallOts` | `index` 0–3 | Recalls OTS 1–4 into the keyboard parts: voice, on/off, volume, octave, and the pan and reverb/chorus sends the OTS sets (CC10/91/93; one it does not set is left as it is). A part the OTS gives a voice ends a plugin picked for it, as `setPartVoice` does. Ignored if the style has no such OTS. When `setOtsRack` gave this OTS one of the user's racks (for the loaded style), that rack loads instead, as `loadRack` does: with unsaved changes it fails with `unsavedChanges` and `liveRack.prompt` asks, and the OTS counts as recalled (`ots.applied`) once the prompt's switch is made. From the Launchkey (pad page 3), a pedal or OTS Link, which have no dialog, unsaved changes are kept as a "Recovered: <name>" rack and the switch goes ahead. A rack that is gone falls back to the style's own OTS. Either way it turns Sync Start on. |
| `setOtsRack` | `index` 0–3, `id` | For the loaded style, OTS `index` loads the user's rack `id` instead of the style's own (docs/racks.md "Styles and OTS"; `ots.racks`). Kept by the style's file name in `<data>/style-racks.json`; the style file isn't touched. Loading a style never loads a rack by itself: only OTS Link, which is off by default. Fails for an OTS the style lacks, a rack that doesn't exist, or while `ots.racksReadOnly`. Deleting a rack (`deleteRack`) gives every OTS that loaded it back to its style. |
| `clearOtsRack` | `index` 0–3 | For the loaded style, OTS `index` is the style's own again ("Style's own"). |
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

### Registration Memory and the Playlist: gone

[Quick Racks](#quick-racks) replace Registration Memory (docs/racks.md, "Migration"). The
Registration commands (`pressRegist`, `pressSnapshot`, `stepSnapshotBank`,
`toggleRegistMemory`, `toggleFreeze`, `stepRegistBank`, `stepRegistSequence`, `stepRegist`
and the rest) and the Playlist's (`newPlaylist` … `stepPlaylist`) are refused as unknown
commands, and `registration` and `playlist` are no longer in the state. Registration bank
and playlist files stay on disk, unread; they aren't imported.

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
| `newLooperBank` | | Empties all eight memories: a new, unsaved bank ("New Bank"). The current sequence stays. |
| `saveLooperBank` | `name` (null: its own file), `overwrite`? | Saves the eight memories as a bank file (`<name>.looper.json` in the data folder's `ChordLooper` folder), which becomes the bank's file. Refused when another bank has that file, unless `overwrite: true`; refused without a name for a bank that has no file yet, and without a data folder. |
| `loadLooperBank` | `path` | Loads a bank file (`looper.banks`): its memories replace the eight, and it becomes the bank in use (also at the next start). Refused while recording. |

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
| `setPartPluginPreset` | `part` 0–3, `id`, `preset` | Plays the part on one of plugin `id`'s AU presets: `preset` is its key (`f:<number>` for a factory preset, set with `kAudioUnitProperty_PresentPreset`; `u:<path>` for an `.aupreset` the scan listed, restored as the plugin's ClassInfo state). The part gets an instance of its own, so one plugin can play a different preset on every part. Loads as `setPartPlugin` does; `keyboardParts[i].plugin.preset` / `presetKey` name it. The live rack keeps it (`liveRack`, with the state read once it plays); a saved rack stores the plugin's state as for any plugin, with its sound's id and name. Once it plays, the part plays the preset itself (docs/racks.md "Saving"): `keyboardParts[i].sound` and the part's saved voice name it by its catalog id (`{ "id": "au:<component id>#<key>", "name" }`). Picking a preset adds no library record; `saveSound` makes one. `assignSound` with a preset id sends it. |
| `clearPartPlugin` | `part` 0–3 | Back to the part's SoundFont voice (a 5 ms fade). |
| `savePartPluginState` | `part` 0–3 | Stores the plugin's current preset (what its editor changed) with the part, so it is kept across restarts. Send it when the editor window closes. The state is read on a thread of its own and lands a moment later; a failed read shows in `message`. |
| `rescanPlugins` | | Scans the installed instruments again, ignoring the cache (`plugins.scanning` meanwhile). After it (and after the start-up scan), plugins found for the first time are `new`, plugins seen before that are gone are in `plugins.missing`, keyboard parts whose plugin is gone go silent, and parts whose plugin is back play it again with the state they kept (docs/racks.md, "Plugins coming and going"). |
| `markPluginSeen` | `id` | The player opened plugin `id` (Library › Instruments): it is no longer `new` (kept in `<data>/known-plugins.json`). Playing it on any part does the same. Fails for a plugin that isn't installed. |
| `reloadPartPlugin` | `part` 0–3 or null | Loads the part's plugin again with its saved preset after it stopped working (`muted`) or failed to load (`failed`); null is the part selected for editing. Fails when the part has no plugin, or it is playing or still loading. The TUI's `s` and the Launchkey's Panel fader button 6 send it; that button is red while the selected part's plugin needs it. |
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
| `createPatch` | `patch`: PatchFields | Adds a patch at the end of the list; its new id is `soundLibrary.lastAdded`. PatchFields: `name`, `category`, `tags`, `favourite`, `source` (see [`soundLibrary`](#soundlibrary); a plugin source here takes `state`?, the plugin's state as base64, in place of `hasState`). A sound is the raw instrument: it has no mix (docs/racks.md); an older client's `defaults` is ignored. |
| `updatePatch` | `id`, `patch` | Replaces a patch's fields (rename, recategorise, tags, favourite, source); the id stays. A plugin source with no `state`, for the patch's own plugin and `origin`, keeps the state the library holds (so the state's source, sent back without `hasState`, never wipes it). Another plugin or another `origin` (for example another factory preset) starts with no state; there is no command to clear a stored state otherwise. |
| `deletePatch` | `id` | Deletes it. Map rules that name it go; a keyboard part playing it goes back to its GM voice. |
| `duplicatePatch` | `id` | A copy ("… copy") right after it, with a new id. |
| `movePatch` | `id`, `to` | Moves it to position `to` (0-based) in the list. |
| `setPatchFavourite` | `id`, `favourite` | Marks or unmarks a favourite. |
| `saveSound` | `part` 0–3 | Save: what a keyboard part plays now over the Sound it plays (`keyboardParts[i].sound`): a plugin sound's state (read afresh, landing a moment later, as `saveSoundAs`); never the part's mix (volume, octave, pan, sends), which stays the part's. The part keeps its plugin instance, and `soundEdited` clears. Only the user's own sounds are overwritten: a factory preset's or an `.aupreset` file's sound, a sound of another plugin, or a part that plays no named sound is saved as a new one instead (`saveSoundAs`), named after the preset or voice it plays (not the plugin), which the part then plays: a second `saveSound` updates it, so one save makes one record. |
| `saveSoundAs` | `part` 0–3, `name` or null | Save as…: what a keyboard part plays as a new Sound, as `savePartAsPatch` describes. A part playing a plugin then plays the new sound (`keyboardParts[i].sound` names it, not edited); a part whose own patch was a plugin sound gets the new patch, on the same instance. A part playing a SoundFont sound (a GM voice, or the patch the map gives it) takes the new sound as its own patch (`keyboardParts[i].patch`), except Left playing Manual Bass. |
| `savePartAsPatch` | `part` 0–3, `name` or null | The old "Save as patch", kept for older clients: `saveSoundAs`. Saves what a keyboard part plays as a new patch: its plugin (component id and its state as its editor left it; a playing plugin's state is read afresh and lands in the patch a moment later), else the patch it plays (its own, or the one the program map sends its GM voice to), else its GM voice on the synth's SoundFont. The part's mix is not saved: a sound has none. |
| `addPresetAsPatch` | `file`, `bank`, `program`, `name` or null | Adds a SoundFont preset (`browseSoundFont`) as a patch, named after the preset and categorised from its bank and program. |
| `auditionPatch` | `id` | Plays the patch on its own for about 3 s (an arpeggio and a chord; a drum kit plays a beat), on channel 16 of the built-in synth, which the band is not using while stopped, at a fixed level (CC7 100) for every sound. A plugin patch first loads its plugin there (#91's rack), then plays; the plugin goes when the audition ends. Refused while the band plays (like `auditionStyle`); `soundLibrary.auditioning` names it. |
| `auditionPreset` | `file`, `bank`, `program` | The same for a SoundFont preset, before adding it. A SoundFont the synth hasn't loaded loads first. |
| `stopPatchAudition` | | Ends the audition now. |
| `setFamilyRule` | `family` 0–15, `patch` or null, `style` | A GM family (programs 8·family … 8·family+7) plays `patch`; null clears the rule. `style`: the current style's own map instead of the global one (may be left out: false). In the three rule commands `patch` may also be a [sound catalog](#sound-catalog) id: a saved sound's patch, or a preset or plugin, which becomes a library patch the first time (a plugin with its default preset). A rule given another patch loses its level (`familyVolumes`, an override's `volume`, `drumsVolume`); the same patch keeps it. |
| `setProgramOverride` | `program` 0–127, `patch` or null, `style` | One GM program plays `patch`, whatever its family's rule. |
| `setDrumRule` | `patch` or null, `style` | The drum parts (Rhythm 1 and 2, and any part on a Yamaha drum kit bank, MSB 126/127) play `patch`. |
| `clearStyleMap` | | Forgets the current style's own map. |
| `setPartPatch` | `part` 0–3, `id` or null | A keyboard part plays a library patch; the part keeps its own volume, octave, pan and sends (a sound has no mix). Null: back to its GM voice (through the map). `setPartVoice`, `stepVoice` and an OTS recall that gives the part a voice also end it. A plugin patch loads its plugin with the patch's state, as `setPartPlugin` does (the part's `plugin` shows it loading, then playing); leaving the patch takes that plugin away. `setPartPlugin` (and `clearPartPlugin` while a plugin patch plays) ends the part's patch; a SoundFont patch picked over a `setPartPlugin` plugin ends that plugin. |
| `setPortSendsMapped` | `on` | The `yahaha` MIDI port gets the mapped bank and program for the band's program changes the map sends to a SoundFont patch, instead of the style's own (default off: the port mirrors the style). |
| `browseSoundFont` | `file` or null | Lists a SoundFont's presets in `soundLibrary.browse` (a file in `io.soundFonts`); null closes the list. |
| `importSoundLibrary` | `path`, `replace`, `maps` | Reads a library file (a full library, or a bare list of patches). Its patches are added (ids that clash get new ones); `maps`: its program maps' rules are added too; `replace`: it replaces the library instead. `replace` and `maps` may be left out (false). |
| `exportSoundLibrary` | `path` or null | Writes the library as a bundle to `path` (null: `sound-library-export.json` in the data folder): `{ "kind": "yahaha-sound-bundle", "fonts": [...], "library": {...} }`, the library (patches with every plugin sound's state, the global and per-style maps) and the SoundFont file names it plays. Fonts are never copied. `importSoundLibrary` reads a bundle or a plain library file; it resolves fonts by file name in the SoundFont folder and reports (as an error message) any that are missing, keeping their sounds. |
| `exportSoundPreset` | `id`, `overwrite` | Writes plugin sound `id` as `<name>.aupreset` in its plugin's user preset folder (`~/Library/Audio/Presets/<Manufacturer>/<Plugin>/`, which Logic reads). Refused for a SoundFont sound, a factory sound not yet played (no state), or when a preset of that name exists and `overwrite` is not set (the app asks Replace/Cancel). `overwrite` may be left out (false). |

### Parameter Lock
Genos Menu › Utility › Parameter Lock (RM p.163): a locked group changes only from the
panel. Rack and One Touch Setting recalls leave it as it is (a rack's split point is the
only lock-group item a recall sets today). The groups are the Data List's lock groups that
yahaha has: `splitPoint` (the split point) and `fingeringType` (the fingering type and the
Chord Detection Area: Upper, Manual Bass).

| Command | Fields | What it does |
|---|---|---|
| `setParamLock` | `item` (`splitPoint` \| `fingeringType`), `on` | Locks or unlocks a group. A setup setting, not part of a rack: it is kept in `param-locks.json` in the data folder (read from the old Registration folder's `setup.json` until that file exists). |

### Style Dynamics
Genos2 Style Dynamics Control (OM p.11, p.69; RM p.11, p.142, p.147), with Touch and Accent (#180; docs/genos-features.md, Style Dynamics Control).
- **Level.** A value from 0 to 127 that scales the velocity of every Style note. At 64 the Style plays as written. The level changes the band's intensity; the parts' CC7 volumes are never touched.
- **Touch.** Each key struck in the chord section sets the level from that key's velocity.
- **Accent.** A key struck at or above the threshold (a chord-section key; with Source `both`, a right-hand key too) accents. Mode `hits` (the default) plays a one-shot hit from the Style's drum kit on its drum channel, with the style stopped or playing; mode `fill` starts the playing Main's fill from the next beat (stopped, it plays hits). yahaha's Accent stands in for the PSR-SX Unison & Accent feature: no style carries Yamaha's accent data.
- **Storage.** All of these are System settings. Racks do not store them.

| Command | Fields | What it does |
|---|---|---|
| `setDynamicsControl` | `on` | Style Setting › Dynamics Control. Off: the Style plays as written, whatever the level. |
| `setDynamics` | `level` 0–127 | The Dynamics level (127, the default: as written; each style load sets 127). |
| `stepDynamics` | `delta` | Moves the level by `delta`, clamped to 0–127. |
| `setDynamicsTouch`, `toggleDynamicsTouch` | `on` | Touch: each chord-section strike sets the level to its velocity × 1.27, so a strike at 100 or harder plays as written. |
| `setAccent`, `toggleAccent` | `on` | Accent: a chord-section strike at or above the threshold, while a Main plays, starts that Main's own fill at the next beat, as Fill Self does. It is not a Main press, so OTS Link does not follow it. It does nothing during an Intro, fill, break or Ending, or while a change is queued. |
| `setAccentThreshold` | `velocity` 1–127 | The Accent threshold (default 110). |
| `setAccentMode` | `mode`: `hits` or `fill` | Accent Mode. `hits` (default): each accent plays two GM drum notes on the Style's kit (channel 10), picked by the strike's velocity: below the midpoint of threshold..120 kick + closed hat, above it kick + snare, from 120 kick + crash; each note's velocity scales with the strike's, retriggers rather than stacks, and ends 150 ms later. Nothing while Rhythm 2 is muted. `fill`: while a Main plays, the Main's own fill as above (stopped, hits). |
| `setAccentSource` | `source`: `left` or `both` | Accent Source. `left` (default): chord-section strikes accent. `both`: right-hand strikes accent too (Touch still hears the chord section only). |

### Effects
The shared effect bus (#204): every part, SoundFont or plugin, feeds three System Effect blocks
through its sends (CC91 Reverb, CC93 Chorus, CC94 Variation; `setPartSend` for the keyboard
parts, the style's own for the Style parts). Each block's return comes back into the mix
before the master fader.

A block's **band send** (#236) scales every Style part's send to it: the style's CC as
written times the band send, in the built-in synth only (the MIDI port carries the style's
CCs unchanged). By default the band's reverb plays as the style wrote it (100) and the
band's chorus and delay are off (0); the keyboard parts' and Multi Pads' sends are never
scaled. A change glides in over about 30 ms.

| Command | Fields | What it does |
|---|---|---|
| `setEffectType` | `block` `reverb` \| `chorus` \| `variation`, `effect` | The block's type. Reverb: `hall` (default), `room`, `stage`, `plate`. Chorus: `chorus` (default), `celeste`, `flanger`. Variation, a stereo delay at the style tempo: `eighth`, `dottedEighth` (default), `quarter`, `pingPong` (1/8, alternating sides). Another block's type is refused. |
| `setEffectReturn` | `block`, `level` 0–127 | The block's return level: 64 = 0 dB (default), 127 = +6 dB, 0 = off (Genos). |
| `setEffectParam` | `block`, `param`, `value` | One of the block's parameters (#236), in the parameter's own unit, clamped to its range (see the table below). A parameter of another block is refused. A change glides on the audio thread, so it never clicks. `setEffectType` puts the block's parameters back to the new type's own values. |
| `setInsertsOn` | `on` | The style's insertion effects (#269, `effects.inserts`) on or off, all together (default on). Off, every Style part plays dry. |
| `setPartInsertOn` | `part`, `on` | *Superseded by `setStripInsertOn` (strip = `part` + 4, slot 0), which still sends this; it keeps working.* One Style part's (0–7) insertion effect on or off, until the next style. |
| `setPartInsertAmount` | `part`, `amount` | *Superseded by `setStripInsertSetting` (strip = `part` + 4, slot 0, setting 0); still works.* One Style part's insertion effect amount, 0–127 (drive, squeeze, wah sensitivity, tremolo/rotary depth), until the next style. A part with no insert is refused. |
| `setRotaryFast` | `on` | Every rotary insert at its fast speed or its slow one; it glides between them (about 1 s up, 2 s down). |
| `setFollowStyle` | `block`, `on` | Whether the block follows the style's own effect type (#237). On (the default), each style load gives the block the style's type (and the delay's time, feedback and tone, the reverb's time, pre-delay and tone, and the block's return level, as the style sets them; #269), or the block's default type if the style sets none that yahaha has. `setEffectType` turns it off, so the player's choice stays through style changes. Turning it on takes the loaded style's type at once. |
| `setBandSend` | `block`, `level` 0–127 | The block's band send, in percent: 100 = the Style parts' sends as written, 0 = none of the band, above 100 up to 127 raises them (each part's send at most the whole signal). Defaults: reverb 100, chorus 0, variation 0. |
| `setPadSend` | `block`, `level` 0–127 | The block's Multi Pad send (#267), in percent: the same scale as `setBandSend`, on the four Multi Pads' sends (channels 5–8). Defaults: reverb 100, chorus 0, variation 0. In the built-in synth only (the MIDI port carries the pads' CCs as written). |
| `setMasterCompressorOn` | `on` | The Master Compressor on or off (default off). Example: `{"type":"setMasterCompressorOn","on":true}`. |
| `setMasterCompressorPreset` | `preset` `natural` \| `rich` \| `punchy` \| `electronic` \| `loud` | The Master Compressor's type; its Compression, Texture and Output come with it (see below). Example: `{"type":"setMasterCompressorPreset","preset":"punchy"}`. |
| `setMasterCompressorParam` | `param` `compression` \| `texture` \| `output`, `value` | One Master Compressor parameter, clamped: `compression` 0–100 %, `texture` 0–100 %, `output` −12..12 dB. Example: `{"type":"setMasterCompressorParam","param":"output","value":-3}`. |
| `setMasterEqOn` | `on` | The Master EQ on or off (default off). Example: `{"type":"setMasterEqOn","on":true}`. |
| `setMasterEqPreset` | `preset` `flat` \| `mellow` \| `bright` \| `loudness` \| `powerful` | The Master EQ's type: every band comes with it. Example: `{"type":"setMasterEqPreset","preset":"loudness"}`. |
| `setMasterEqBand` | `band` 0–7, `gain`, `freq`, `q`, `shelf` | One Master EQ band, clamped to its ranges (below). A band other than 0–7 is refused. Example: `{"type":"setMasterEqBand","band":7,"gain":3,"freq":10000,"q":7,"shelf":true}`. |

**Master Compressor and Master EQ** (Genos RM p.130–131, p.136; OM p.106). They run on the
whole mix, after the effect returns and before the output's safety clipper, compressor
first; the metronome click doesn't go through them (as on the Genos). They are tone on the
master, shown with their settings, and may boost (an EQ band, the compressor's Output). Off
(the default), they aren't run: the output is bit-identical to the mix without them. They
are a setup setting, saved in `<data>/master-effects.json` (never in a rack) and read at
start; a missing or unreadable file, or a missing field, reads as its default (both off).

- **Compressor.** `compression` 0–100 %: threshold −3 − 0.27 × `compression` dBFS, ratio
  1 + 0.07 × `compression` (0 % compresses nothing, 100 % is −30 dBFS at 8:1), a 6 dB soft
  knee, stereo-linked. `texture` 0–100 %: higher is lighter, attack 30 → 1 ms and release
  500 → 60 ms. `output` −12..12 dB after it. A change glides; switched off, it glides back to
  unity and then stops. Types (compression / texture / output): Natural 30 / 50 / +1, Rich
  45 / 30 / +2, Punchy 70 / 80 / +4, Electronic 60 / 65 / +3, Loud 85 / 45 / +6.
- **EQ.** Eight bands, each `{ gain, freq, q, shelf }`: `gain` −12..12 dB; `freq` Hz, band 0
  32–2000, bands 1–6 100–10000, band 7 500–16000; `q` in tenths, 1–120 (0.1–12.0; higher is
  narrower); `shelf` (bands 0 and 7 only) makes the band a low (high) shelf, with a fixed
  slope rather than a Q. A band at 0 dB is out of the signal. Every type puts the bands at
  80, 250, 500, 630, 800, 1000, 4000 and 8000 Hz, Q 0.7, bands 0 and 7 as shelves, with gains:
  Flat all 0; Mellow −2 at 4 kHz and −4 at 8 kHz; Bright +2, +4 there; Loudness +4 at 80 Hz,
  +1 at 250 Hz, +2 and +4 at the top; Powerful +4, +2, +1, +1, +1, +1, +2, +3.

The effect parameters (`param`, its unit and range, and each type's own value):

| Block | `param` | Unit | Range | Hall / Room / Stage / Plate |
|---|---|---|---|---|
| Reverb | `reverbTime` | 0.1 s (decay to −60 dB) | 3–100 (0.3–10 s) | 24 / 9 / 17 / 18 |
| Reverb | `preDelay` | ms | 0–200 | 22 / 4 / 12 / 1 |
| Reverb | `reverbTone` | 100 Hz (the tail's high cut; lower is darker) | 10–200 (1–20 kHz) | 45 / 60 / 65 / 90 |

| Block | `param` | Unit | Range | Chorus / Celeste / Flanger |
|---|---|---|---|---|
| Chorus | `chorusRate` | 0.01 Hz (the LFO speed; the second tap keeps its type's ratio) | 5–500 (0.05–5 Hz) | 55 / 29 / 21 |
| Chorus | `chorusDepth` | 0.1 ms (how far the taps swing) | 0–50 (0–5 ms) | 22 / 9 / 18 |

| Block | `param` | Unit | Range | 1/8 / 1/8. / 1/4 / Ping-Pong |
|---|---|---|---|---|
| Variation | `delaySync` | switch: 1 = the time is `delayNote` at the style tempo, 0 = `delayTime` | 0–1 | 1 |
| Variation | `delayNote` | note value: 0 `1/16`, 1 `1/8T`, 2 `1/8`, 3 `1/4T`, 4 `1/8.`, 5 `1/4`, 6 `1/4.`, 7 `1/2` | 0–7 | 2 / 4 / 5 / 2 |
| Variation | `delayTime` | ms (with tempo sync off) | 10–2000 | 375 |
| Variation | `delayFeedback` | % of each repeat that comes back | 0–90 | 38 |
| Variation | `delayTone` | 100 Hz (the repeats' high cut) | 10–200 (1–20 kHz) | 50 |
| Variation | `pingPong` | switch: 1 = the repeats alternate left and right | 0–1 | 0 / 0 / 0 / 1 |

**A style's own effects (#237).** A style's SInt carries XG System Effect SysEx
(`F0 43 1n 4C 02 01 aa …`), which the Genos treats as Style data: the Reverb type (address
00), Chorus type (20) and Variation type (40, counted only when `5A` VARIATION CONNECTION
is 1, System). Each maps onto the nearest type here:

| Block | XG type (MSB) | Plays as |
|---|---|---|
| Reverb | 1 Hall, 17–19 Tunnel/Canyon/Basement | `hall` |
| Reverb | 2 Room, 16 White Room | `room` |
| Reverb | 3 Stage | `stage` |
| Reverb | 4 Plate | `plate` |
| Chorus | 65, 66 Chorus (66/0 Celeste 1) | `chorus` (`celeste`) |
| Chorus | 67 Flanger | `flanger` |
| Chorus | 87 Ensemble Detune | `celeste` |
| Variation | 21 Tempo Delay, Tempo Echo | `dottedEighth` with the style's delay time (Data List Table#5, nearest note), feedback and high damp |
| Variation | 22 Tempo Cross | `pingPong`, the same |
| Variation | 5 Delay LCR, 6 Delay LR | `dottedEighth` |

A style's **insertion effects** (#269: an XG Insertion block `03 nn`, or a Variation
connected as Insertion, assigned to a Style part as its first setup routes it; the first
per part) play in the built-in synth on that part's own signal, before its sends. By the
type's family (Genos Data List Effect Type List):

| XG types (MSB/LSB) | Plays as |
|---|---|
| 73 Distortion, 74 Overdrive, 75 Stereo Amp Sim, 83/36 Uni Comp Clipper Dist, 95/32–35 Multi FX distortions, 96 Small Stereo Dist, 97 British Combo, 98 V Distortion, 99/32+ US Combo, 100 Jazz Combo, 101 US High Gain, 102 British Lead, 103 Tweed Guy, 105/32+ Y-Amp | `distortion`, drive by the type: clean (Clean, Jazz Combo, Y-Amp Live Clean), crunch, or lead (Lead, High Gain, Metal, Distortion) |
| 83 Uni Comp, 105/0–31 Multi Band Comp, 124/4 VCM Compressor | `compressor` |
| 78 Auto Wah, 79 Tempo Auto Wah, 124/5 VCM Auto Wah, 125 VCM Pedal Wah | `autoWah` (the part's envelope moves it) |
| 70 Tremolo, 120 Tempo Tremolo | `tremolo` (a 1/8 note of the style tempo) |
| 69 Rotary Speaker, 99/16–31 Dual Rotary Speaker | `rotary` (slow) |

Anything else (THRU, EQ, chorus, reverbs, delays) leaves the part dry. The type's own
parameters are not read. The MIDI port gets the style's SysEx as written.

With a matching type, the style's reverb parameters come too (#269): parameter 1 Reverb
Time (Data List Table#1) as `reverbTime` (at most 10 s), 3 Initial Delay (Table#2) as
`preDelay`, and the high cut (Table#3; a Real Reverb's 4 High Damp Frequency, another
reverb's 5 LPF Cutoff; Thru = 20 kHz) as `reverbTone`. A block's return level (Reverb
`0C`, Chorus `2C`, Variation `56`) comes where the style sets one; where it sets none (or
the block has no match), a following block's return goes back to 64 (0 dB), so one style's
return never carries into the next. A block that doesn't follow keeps the player's. The chorus's parameters are not read (no corpus style sets them on
a chorus type yahaha has).

Any other type (a phaser or a tempo delay in the chorus block, a distortion or a reverb as the
variation) has no match: the block plays its default type. The MIDI port gets the SysEx as
the style wrote it. The band sends (#236) still apply on top: by default the band doesn't
reach the chorus or the delay, whatever their type.

The Variation types are starting points: each sets the note value and the ping-pong switch,
which can then be changed like any parameter (as a Genos type loads its own settings). A
delay longer than 2 s repeats at 2 s.

### Channel strips and send effects

The mixer rework: every part has a **channel strip** that runs EQ → compressor → insert 1 →
insert 2 → sends → pan and level, and the strips feed up to six **send effects**.

- **`strip`** 0–11: 0–3 the keyboard parts (Right 1, Right 2, Right 3, Left), 4–11 the Style
  parts 0–7 (Rhythm 1 … Phrase 2). Its state is `strip` on every keyboard part and Style
  part (StripState, under `keyboardParts`).
- **`send`** 0–5: sends 1–3 (0–2) are always there and are the effect bus's Reverb, Chorus
  and Variation blocks, which the style's reverb, chorus and delay sends feed; sends 4–6
  (3–5) are the player's, added with `addSend`, and belong to the rack. Their state is
  `effects.sends` (SendState).
- **`slot`** 0–1: insert 1 and insert 2. On a keyboard strip, slot 0 is the part's insert
  (`insert`); on a Style strip, it is the style's insertion effect for that part
  (`effects.inserts`), whose kind comes from the style while the style sets one.

Where an older command already does what a strip command does, the session and the dev mock
send that older command, and keep the strip command's own settings too; the older commands
keep working unchanged, and the two always show the same. What no older command covers (the
strip compressor, insert 2, a Style strip's EQ, insert settings after the first, sends 4–6,
a phaser) is kept, saved in the state and shown, but doesn't play yet: it arrives with the
mixer rework's DSP. A refused command changes nothing and returns `Failed` with the reason.

| Command | Fields | What it does |
|---|---|---|
| `setStripEq` | `strip`, `eq`: `{ lowGain, lowFreq, highGain, highFreq }` | The strip's EQ, clamped as `setPartEq`. On a keyboard strip it is `setPartEq` (`part` = `strip`). Example: `{"type":"setStripEq","strip":4,"eq":{"lowGain":2,"lowFreq":100,"highGain":0,"highFreq":8000}}`. |
| `setStripCompressorOn` | `strip`, `on` | The strip's compressor on or off (default off). Example: `{"type":"setStripCompressorOn","strip":0,"on":true}`. |
| `setStripCompressorPreset` | `strip`, `preset` `natural` \| `rich` \| `punchy` \| `electronic` \| `loud` | The strip compressor's type; its parameters come with it (see below). On/off is unchanged. Example: `{"type":"setStripCompressorPreset","strip":5,"preset":"punchy"}`. |
| `setStripCompressorParam` | `strip`, `param` `threshold` \| `ratio` \| `attack` \| `release` \| `makeup`, `value` | One strip compressor parameter, clamped: `threshold` −48..0 dB, `ratio` 10–200 in tenths (40 = 4.0:1), `attack` 1–100 ms, `release` 10–1000 ms, `makeup` 0–24 dB. Example: `{"type":"setStripCompressorParam","strip":0,"param":"threshold","value":-24}`. |
| `setStripInsertKind` | `strip`, `slot` 0–1, `kind` | Insert `slot` plays `kind` (see the insert kinds below; `none` empties it), its settings at the kind's defaults. On/off is unchanged. An unknown kind is refused. On a keyboard strip's slot 0 it is `setKeyboardInsertEffect` (`none`: `setKeyboardInsertOn` off). Example: `{"type":"setStripInsertKind","strip":5,"slot":1,"kind":"phaser"}`. |
| `setStripInsertOn` | `strip`, `slot`, `on` | Insert `slot` on or off. Slot 0 is `setKeyboardInsertOn` on a keyboard strip, `setPartInsertOn` (`part` = `strip` − 4) on a Style strip. Example: `{"type":"setStripInsertOn","strip":5,"slot":1,"on":true}`. |
| `setStripInsertSetting` | `strip`, `slot`, `setting` 0–3, `value` | One of the insert's settings (by `settings` in its InsertSlotState), clamped to its range; a setting its kind doesn't have (or an empty slot) is refused. Slot 0's setting 0 is the older amount: `setKeyboardInsertAmount` on a keyboard strip, `setPartInsertAmount` on a Style strip. Example: `{"type":"setStripInsertSetting","strip":5,"slot":1,"setting":1,"value":120}`. |
| `setStripSend` | `strip`, `send` 0–5, `level` 0–127 | The strip's level to send `send`. A send that isn't there is refused. Sends 0–2 are `setPartSend` (`reverb`, `chorus`, `variation`) on a keyboard strip and `setStylePartSend` on a Style strip. Example: `{"type":"setStripSend","strip":8,"send":3,"level":50}`. |
| `addSend` | `kind` | Adds a send effect (the next of sends 4–6) playing `kind` (see the send kinds below) at its defaults, returning at 64 (0 dB); every strip's level to it starts at 0. Refused when all six are there, or for an unknown kind. Example: `{"type":"addSend","kind":"plate"}`. |
| `removeSend` | `send` 3–5 | Removes an added send effect; the ones after it move down one, with every strip's level to them. Sends 0–2 can't be removed. Example: `{"type":"removeSend","send":3}`. |
| `setSendKind` | `send`, `kind` | The send effect's kind; its parameters go back to that kind's defaults, and its return is unchanged. Sends 0–2 take only their own block's types (send 0 `hall` \| `room` \| `stage` \| `plate`, send 1 `chorus` \| `celeste` \| `flanger`, send 2 `eighth` \| `dottedEighth` \| `quarter` \| `pingPong`) and are `setEffectType`; sends 3–5 take any kind. An unknown kind is refused. Example: `{"type":"setSendKind","send":3,"kind":"room"}`. |
| `setSendParam` | `send`, `param` (an index into its `params`), `value` | One of the send effect's parameters, clamped; one its kind doesn't have is refused. On sends 0–2 it is `setEffectParam` (the block's parameter at that index). Example: `{"type":"setSendParam","send":2,"param":3,"value":60}`. |
| `setSendReturn` | `send`, `level` 0–127 | The send effect's return level (64 = 0 dB). On sends 0–2 it is `setEffectReturn`. Example: `{"type":"setSendReturn","send":3,"level":80}`. |
| `setRackSendOverride` | `send` 0–2, `on` | Whether the live rack overrides send `send`'s kind: on, the rack keeps the kind the send has now and brings it back when it loads, over the style's (`setByRack` in its SendState). Sends 3–5 are always the rack's and are refused. Example: `{"type":"setRackSendOverride","send":1,"on":true}`. |

**Strip compressor types** (threshold / ratio / attack / release / make-up). Editing a
parameter keeps the type and sets `edited`.

| Type | Threshold | Ratio | Attack | Release | Make-up |
|---|---|---|---|---|---|
| Natural (default) | −18 dB | 2.5:1 | 10 ms | 200 ms | +3 dB |
| Rich | −20 dB | 2.0:1 | 30 ms | 400 ms | +3 dB |
| Punchy | −24 dB | 6.0:1 | 5 ms | 120 ms | +6 dB |
| Electronic | −22 dB | 4.0:1 | 3 ms | 100 ms | +5 dB |
| Loud | −30 dB | 8.0:1 | 2 ms | 150 ms | +9 dB |

**Insert kinds** (`kind` in `setStripInsertKind`), each with 2–4 settings, in order: name,
range and default. The first setting is what the older single `amount` was (drive, squeeze,
sensitivity, depth), so an older insert keeps sounding the same. A kind a newer build wrote
(one this build doesn't know) reads back as its own name, with no settings, and plays dry.

| Kind | Name | Settings |
|---|---|---|
| none | None | (an empty slot) |
| distortion | Distortion | Drive 0–127 (64), Tone 0–127 (64), Output 0–127 (100) |
| compressor | Compressor | Squeeze 0–127 (64), Attack 1–80 ms (10), Release 10–1000 ms (200), Output 0–127 (100) |
| autoWah | Auto Wah | Sensitivity 0–127 (64), Resonance 0–127 (64), Frequency 0–127 (32) |
| tremolo | Tremolo | Depth 0–127 (64), Note 0–7 (2 = 1/8; the note values of `delayNote`), Shape 0–127 (0) |
| rotary | Rotary | Depth 0–127 (64), Drive 0–127 (0), Balance 0–127 (64) |
| phaser | Phaser | Depth 0–127 (64), Rate 5–500 in 0.01 Hz (50 = 0.50 Hz), Feedback 0–90 % (40). Plays dry until its DSP lands. |

**Send kinds** (`kind` in `addSend` and `setSendKind`). The reverb, chorus and delay kinds
are the bus types of the same names (Effects, above), with that block's parameters in its
order and each type's own values as defaults: reverb kinds `reverbTime`, `preDelay`,
`reverbTone` ("Time", "Pre-delay", "Tone"); chorus kinds `chorusRate`, `chorusDepth` ("Rate",
"Depth"); delay kinds `delaySync`, `delayNote`, `delayTime`, `delayFeedback`, `delayTone`,
`pingPong` ("Tempo sync", "Note", "Time", "Feedback", "Tone", "Ping-pong"). A kind a newer
build wrote reads back as its own name, with no parameters, and is silent.

| Kind | Name | Parameters |
|---|---|---|
| hall, room, stage, plate | Hall, Room, Stage, Plate | The reverb's three (defaults in the reverb table above). |
| chorus, celeste, flanger | Chorus, Celeste, Flanger | The chorus's two. |
| eighth, dottedEighth, quarter, pingPong | Delay 1/8, Delay 1/8., Delay 1/4, Ping-Pong | The delay's six. |
| phaser | Phaser | Depth 0–127 (64), Rate 5–500 in 0.01 Hz (50), Feedback 0–90 % (40). Silent until its DSP lands. |

### Knob Assign pages
The Launchkey's 8 encoders as the Genos LIVE CONTROL knobs (#197; OM p.62–63, RM p.145–148;
README › Knobs). A page gives each knob a function; the knobs are relative, so a turn moves
the value from where it is now, whoever set it last. A turn runs the command of the knob's
function (`setDynamics`, `stepRetriggerRate`, `toggleRetrigger`, `styleTrackMute`,
`setTempo`, `setSwing`, `setPartVolume`, `setHarmonyVolume`, `setMetronomeVolume`, `setPartPan`,
`setPartSend`, `setEffectReturn`, `setEffectParam`, `toggleHarmonyArp`, `setSplit`), so it
behaves exactly as that command does.

The `rack` page's knobs do what the live rack's controller map says (`liveRack.controls`,
set with `setRackControl`). With the default map it is the page it replaced (`parts`): Right
1–3 and Left volume, Harmony volume, Metronome volume, none, Tempo.

| Command | Fields | What it does |
|---|---|---|
| `setKnobPage` | `page` `style` \| `rack` \| `pan` \| `reverb` \| `chorus` \| `delay` | The Knob Assign page. The old name `parts` is read as `rack`. One page per effect: knobs 1–4 are Right 1, Right 2, Right 3 and Left's send to it (`setPartSend`, CC91/93/94), knob 8 its return (`setEffectReturn`), knobs 5–7 its parameters (`setEffectParam`, #236): `reverb` Time, Pre-delay, Tone; `chorus` Rate, Depth, (none); `delay` Time, Feedback, Tone. A parameter turn pins its block to the player's own (`followStyle` off, #237). The old names `effects` and `fx` are read as `reverb` and `delay`. |
| `stepKnobPage` | `delta` | Steps the page, stopping at the first and last (the encoder page buttons ▲/▼). |
| `resetKnob` | `knob` 0–7 | Puts a knob's function back to its default (the app's double-click): Dynamics to 127, part and Harmony volumes 100, Metronome 90, pan centre, sends dry (0), returns 64, Swing 0, Retrigger off at 1/8, Track Mute all on, Tempo the style's (`resetTempo`), an effect parameter its current type's own, Harmony/Arpeggio off, the split point F#2. No Assign does nothing. |
| `turnKnob` | `knob` 0–7, `delta` | Turns a knob `delta` steps (positive: clockwise). Levels move 2 a step, tempo 1 BPM; Retrigger Rate and On/Off switch every 3 steps (right: shorter, on); Track Mute A/B move their position 4 a step. An effect parameter moves its own step (reverb time 0.1 s, pre-delay 2 ms, tones 200 Hz, feedback 2%, chorus rate 0.02 Hz and depth 0.1 ms); the Delay Time knob steps the note value every 3 steps with tempo sync on, or 10 ms a step with it off. Harmony/Arpeggio switches every 3 steps (right: on); the split point moves a semitone a step (24–96). A knob with No Assign does nothing. |

### Sound catalog
One list of every sound for the Sound Browser (#117): every preset of every `.sf2` in the
SoundFont folder, every instrument plugin, and every saved sound (the sound library's
patches). Entry ids: `sf:<file>:<bank>:<program>`, `au:<component id>`, `saved:<patch id>`.
Favourites, Recents and plugin categories are saved in `sound-settings.json` in the data
folder.

| Command | Fields | What it does |
|---|---|---|
| `setSoundFavourite` | `id`, `on` | Marks or unmarks a favourite. A saved sound's favourite is its patch's `favourite`. |
| `assignSound` | `part` 0–3, `id` | The keyboard part plays the sound. A preset of the synth's main font (`io.soundFontFile`, bank 0) becomes the part's voice (`setPartVoice`). A preset of another font becomes a saved sound (the library's patch for it, added once) and plays as `setPartPatch`. A plugin plays as `setPartPlugin` (its default preset), and a saved sound as `setPartPatch`. The sound goes to the top of the Recents (20 kept). |
| `replacePartSound` | `part` 0–3, `id` | Library › Replace… (for a part whose plugin is missing, or any part): the part plays sound `id` as `assignSound` does, and keeps its mix — level, pan, sends, octave, voice settings, bend range and on/off stay as they were, whatever the sound's defaults (docs/racks.md: swapping a sound never touches the mix). Nothing is saved until the rack is. |
| `setSoundCategory` | `id`, `category` | A plugin's or plugin preset's category (until set: a plugin's is guessed from its name and maker, a preset's from its name and folder, else its plugin's), or a saved sound's (its patch's). A preset's category is its GM family: refused. |
| `listPluginPresets` | `id` (`au:<component id>`) | The browser expanded a plugin: list its AU presets. Its `.aupreset` files (in `~/Library/Audio/Presets/<Manufacturer>/<Plugin>/` and `/Library/Audio/Presets/...`) are listed at every scan; its factory presets (`kAudioUnitProperty_FactoryPresets`) need an instance, so a plugin no load has read yet is loaded once in the background (`sounds.listingPresets` has it meanwhile) and they are cached with the scan. The catalog moves when they are in: presets are entries `au:<component id>#f:<number>` (factory) and `au:<component id>#u:<path>` (file), with `parent` the plugin's id; the plugin's `plugin.presets` counts them (null until the factory presets were read). Placeholder factory presets (no name, a blank one, `<disabled>` and the like, repeats of a name) are never listed. A listing always ends: if it fails, times out (30 s) or gets no list, the plugin's `plugin.presetsError` says why, and it is not tried again until the next scan. A rescan (`rescanPlugins`) lists everything again. |
| `addToMySounds` | `id` | The Instruments tab's "Add to my sounds": adds catalog entry `id` (a font preset `sf:…`, a plugin `au:<component id>`, or a plugin preset `au:<component id>#<key>`) to the sound library, once: the same patch a part or a map rule gets for it (a factory preset's state is captured when it first plays). Nothing plays it. A `saved:` id is in already, so nothing changes. Fails for an id not in the catalog. |
| `savePartAsPluginPreset` | `part` 0–3, `name`, `category`, `overwrite`? | Saves what the part's plugin plays now (its editor's changes: a Kontakt instrument loaded there, say) as `<name>.aupreset` in `~/Library/Audio/Presets/<Manufacturer>/<Plugin>/`, the standard file Logic and MainStage read. A preset of that name that exists already is refused unless `overwrite` is true (the app asks "Replace '<name>'?" first, as Logic does). It lists under the plugin, filed under `category` (kept in `sound-settings.json`; the file is not changed), and the part then plays it. The state is read and written off the control thread. Fails when the part's plugin is not playing. |

The list itself is fetched, not in the state: see [`sounds`](#sounds).

The program map's rule commands (`setFamilyRule`, `setProgramOverride`, `setDrumRule`) also
take a catalog id as their `patch`, so the map's pickers pick from the same list.

### Racks
The user's racks (docs/racks.md, "Saving"): `<data>/Racks/<name>.rack.json`, named by a
stable `id` (a rename keeps it). The list is [`racks`](#racks); the rack playing now is
[`liveRack`](#liverack). Loading or saving a rack leaves the live rack that rack
(`name`, `id`), unmodified, and autosaves it. Every command fails with no data folder,
except `newRack`.

| Command | Fields | What it does |
|---|---|---|
| `newRack` | `discard`? | A new rack: every keyboard part on its default GM voice (Right 1 Grand Piano, Right 2 Strings, Right 3 Brass, Left Strings) with level 100, pan centre and dry sends, only Right 1 on, octave 0, no voice settings, bend range 2; the split F#2, no keyboard transpose, Harmony/Arpeggio off (its settings kept) and the default controller map. `liveRack` becomes `New rack` with no id. With unsaved changes (`liveRack.modified`) and no `discard: true`, nothing changes: it returns `{"kind":"unsavedChanges"}` and `liveRack.prompt` asks. |
| `loadRack` | `id`, `discard`? | Loads the user's rack `id`: its parts' sounds and mix, split, Harmony/Arpeggio, keyboard transpose and controller map (as `applyRack`). A sound that can't play is reported in `message` and the rest still loads. The same guard as `newRack`. Fails for an unknown id or a rack that can't be read. |
| `saveRack` | `soundNames`? | Save rack: writes the live rack over its own rack (the one `liveRack.id` names); a live rack with none (`New rack`, `Restored`, or its rack deleted) is saved as a new rack under its name (`Name 2`… if taken). Edited sounds are saved in the same step: a part whose plugin was edited (`soundEdited`) and plays the user's own sound saves over that sound (as `saveSound`); one playing a factory preset, an `.aupreset` file or a sound that isn't the user's becomes a new sound of the user's (as `saveSoundAs`), which needs a name in `soundNames` (`{"1": "Soft Pad"}`, by part). Without one, nothing is saved: it returns `{"kind":"needsSoundNames"}` and `liveRack.prompt` lists the parts. Each sound is one record, and the rack names it with no edit of its own. A plugin picked with no sound (its default) keeps its settings in the rack, with no sound made. Factory sounds are never overwritten. |
| `saveRackAs` | `name`, `soundNames`? | Save as…: the live rack as a new rack called `name`, with its edited sounds as `saveRack`. Fails for an empty name or one another rack has. |
| `revertRack` | | Discards the live rack's changes: loads its own rack again, with no question. Fails when it has none. |
| `renameRack` | `id`, `name` | Renames rack `id` (its file follows; the id stays, so Quick Racks keep it). Renaming the loaded rack renames the live rack too, leaving `modified` as it was. Fails for an empty name or one another rack has. |
| `duplicateRack` | `id` | Copies rack `id` as `<name> copy` (`<name> copy 2`… if taken), with a new id. |
| `deleteRack` | `id` | Deletes rack `id`'s file. Refused for the loaded rack (`liveRack.id`): load another first. Quick Rack buttons holding it are emptied, and any style's OTS that loaded it (`setOtsRack`) is the style's own again. |
| `dismissRackPrompt` | | Keep editing: clears `liveRack.prompt`; nothing else changes. |
| `setRackControl` | `control` (`fader` \| `knob`), `index` (0-based: faders 1–4, knobs 1–8), `target` | Sets what that Launchkey fader or knob does in the live rack's controller map (`liveRack.controls`; a target as listed there). The live rack becomes modified; Save rack keeps the map. The Rack knob page and the Panel faders follow it at once. Fails for a target this build doesn't know, a part outside 0–3, the tempo on a fader, or no such controller. |
| `moveRackFader` | `fader` 0–3, `volume` 0–127 | Panel fader `fader` moved to `volume`, where the controller map gives it something other than its own part's level: runs its target's command (`setPartVolume`, `setPartPan`, `setPartSend`, `setHarmonyVolume`, `setMetronomeVolume` with the value; `setHarmonyArpOn` on from 64; `setSplit` across 24–96). Nothing for none. The Launchkey sends it (Volume layer), and it is the `set` of such a fader in `surface.faders`. |

The Launchkey and pedals, which have no dialog, switch racks with
`Session::load_rack_from_hardware`: unsaved changes are kept as a rack of the user's,
`Recovered: <name>` (numbered if taken, edited plugin states kept as the parts' edits, no
sound saved), and the switch goes ahead. If that rack can't be written, nothing changes.

### Quick Racks
The one-press rack buttons (docs/racks.md): banks A–H of eight, each one of the user's
racks (by `id`, so a rename keeps it) or empty, kept in `<data>/quick-racks.json` (format
`yahaha.quick-racks`, version 1, written atomically; a file from a newer yahaha, or one that
can't be read, is never saved over and leaves Quick Racks read-only). The bar, Launchkey
pad page 4 and the pedals play them; the state is [`quickRacks`](#quickracks). `slot` is
a button of the bank on view, 0–7.

| Command | Fields | What it does |
|---|---|---|
| `pressQuickRack` | `slot`, `discard`? | Not armed: loads the button's rack as `loadRack` does, with the same guard (`{"kind":"unsavedChanges"}` and `liveRack.prompt`; `discard: true` switches anyway). Armed (`toggleQuickRackStore`): stores the live rack on the button and disarms. A live rack with unsaved changes, or never saved, isn't stored yet: `quickRacks.storeWaiting` holds the button until `saveRack` / `saveRackAs` succeeds, which stores the saved rack there. Fails for an empty button, or one whose rack is gone. Slots 8 and 9 run on into the next bank's 1 and 2 (the `regist9`/`regist10` pedal functions). |
| `stepQuickRackBank` | `delta` | Bank −/+: views the previous/next bank. It stops at A and at H. |
| `toggleQuickRackStore` | | Store: arms or disarms it for the next press. Disarming lets a waiting button go. |
| `clearQuickRack` | `bank` 0–7, `slot` 0–7 | Empties a button. |
| `stepQuickRack` | `delta`, `discard`? | Previous/next rack in the bank on view: the stored button before/after the lit one (from none, + the first and − the last; it stops at either end), loaded as `pressQuickRack` loads. Fails when the bank has no racks. |

`deleteRack` empties every button naming that rack; `dismissRackPrompt`, or loading
another rack, lets a waiting button go.

From the Launchkey (pad page 4, Shift + Track ◀ ▶), the pedals (`regist1`–`regist10`,
`registNext`/`registPrev`, `registMemory`, `snapshotBankNext`/`snapshotBankPrev`) and the
terminal keys, which have no dialog, a press with unsaved changes switches anyway and keeps
them as a `Recovered: <name>` rack (as `Session::load_rack_from_hardware`), and Store needs
a saved, unmodified rack (otherwise it says so and disarms). `Session::hardware(action)`
runs a Launchkey action that way.

### Result: `CmdError`

`send` returns `Ok(())` or one of these errors:
- `{"kind":"busy"}`: the engine's queue was full for a moment and nothing changed. Try
  again.
- `{"kind":"failed","message":"…"}`: refused or failed. The same text is in
  `state.message`.
- `{"kind":"unsavedChanges"}`: `loadRack` or `newRack` would lose the live rack's unsaved
  changes. Nothing changed; `liveRack.prompt` holds the switch.
- `{"kind":"needsSoundNames"}`: `saveRack` or `saveRackAs` would make new sounds that
  need names. Nothing was saved; `liveRack.prompt` lists the parts.

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
  pressed straight after a page change must already read the new page. A Panel fader 1–4
  that the live rack's controller map gives another target than its own part's level
  becomes `moveRackFader` (Volume layer; none does nothing); the map reaches the MIDI
  thread as a fixed table, updated when it changes.
- The encoders and their page buttons ▲/▼ become `turnKnob` and `stepKnobPage` (see
  Knob Assign pages). On entering DAW mode the session turns the encoders' relative
  output on (feature control 45h); in the Transport encoder mode they are relative anyway.
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
| `landing` | string? | The Main a fill (or the Break) queued or playing lands on, e.g. `"Main A"`; null when none is (#282). The first press picks the fill; every later Main press before the fill ends only changes this. Pressing the fill's own Main while it plays queues it once more (`queued` names it), so mashing keeps the fill going. The Launchkey and the app pulse this Main's pad when it is not the fill's own. |
| `acmp` | bool | [ACMP] is on (the default; `toggleAcmp`). Off: no chord section. |
| `unison`, `unisonLatched` | bool | Unison is engaged (latched or held by a pedal), and its latched switch (`toggleUnison`). |
| `unisonType` | `root` \| `melody` | What the Bass plays in Unison (default `root`). |
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
| `leftHold` | bool | Left Hold (`setLeftHold`). |

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
| `voiceName` | string | The name of what actually sounds, as `sound` resolves it: a plugin preset (or a factory or file preset's sound) reads "<plugin> · <preset>" ("Sampler Deluxe · Warm Keys"), a sound of the user's by its name, a bare plugin by the plugin's name; otherwise (no plugin, or one that failed and plays the SoundFont) the SoundFont patch or font preset's name, or the GM voice's when nothing covers it. For Left under Manual Bass, that is the Style's Bass voice. |
| `playsBass` | bool | Left is playing the bass (Manual Bass). |
| `octave` | −2..2 | The octave setting. It is not applied while `playsBass` is true. |
| `pan` | 0–127 | Pan (CC10): 0 left, 64 centre, 127 right. 64 until something sets it (`setPartPan`, a library patch, an OTS). |
| `reverb`, `chorus` | 0–127 | *Superseded by `strip.sends[0]` and `[1]` (always the same values); still sent.* Reverb and chorus send depth (CC91, CC93). Until something sets them (`setPartSend`, a library patch, an OTS), Genos-like defaults sent at start: reverb 50 and chorus 10 on Right 1–3, reverb 40 and chorus 10 on Left. They go out again after a Panic, a Reset All Controllers from the keyboard, or a new synth. |
| `variation` | 0–127 | *Superseded by `strip.sends[2]`; still sent.* Variation send depth (CC94): the effect bus's tempo delay. 0 until something sets it. |
| `eq` | PartEq | *Superseded by `strip.eq` (always the same); still sent.* Its channel-strip EQ (`setPartEq`): `lowGain`, `highGain` (dB, −12..12) and `lowFreq`, `highFreq` (Hz). Flat (0 dB, 80 Hz, 0 dB, 10000 Hz) until something sets it. An OTS recall sets it from the OTS's XG part EQ (bass/treble gain and frequency, XG multi part 72H, 73H, 76H, 77H: 40H = 0 dB, 1 dB a step, frequencies from the XG EQ frequency table), the bands it leaves out flat; a part the OTS gives a voice but no EQ goes flat; any other part keeps its EQ. A voice change keeps it. |
| `insert` | PartInsert | *Superseded by `strip.inserts[0]` (its kind, `on`, and its first setting as the amount); still sent.* Its insert slot (`setKeyboardInsertEffect`, `setKeyboardInsertOn`, `setKeyboardInsertAmount`): `effect` (`distortion` \| `compressor` \| `autoWah` \| `tremolo` \| `rotary` \| `phaser`; a phaser plays dry until its DSP lands), `on`, `amount` 0–127. Off (a distortion, amount 64) until something sets it. An OTS recall sets it from the OTS's XG Insertion Effect type for the part (block n is part n: Right 1, Right 2, Right 3, Left), mapped as the Style parts' are (`effects.inserts`): on with the effect that plays it and its amount, or off when nothing here plays that type (THRU, an EQ, a delay...); a part the OTS gives a voice but no insertion type turns it off; any other part keeps its slot. A voice change and a plugin swap keep it. |
| `fader` | 0–127? | Where its Launchkey fader (Panel page, faders 1–4) physically is, as last reported. Null until that fader moves. |
| `plugin` | PartPlugin? | The instrument plugin the part plays instead of its SoundFont voice. The key is absent when there is none. `id`, `name`, `manufacturer`, `status` (`loading` \| `playing` \| `failed` \| `muted`: still on the SoundFont, or the previous plugin, while loading; on the SoundFont after a failed load, keeping the choice so it is saved and can be retried; silent after the plugin crashed or produced bad audio), `stage` (while loading: `queued`, `instantiating`, `initializing`, `restoringState`), `error`, `outOfProcess` (runs in its own process), `inProcessFallback` (the system refused to host it in its own process, so it loaded in yahaha's process instead: a crash in it takes yahaha down; the app shows a warning badge), `cpu` (share of real time, updated once a second), `overruns` (renders slower than half the buffer, since it loaded), `recentOverruns` (those in the last 10 seconds, updated once a second: the live readout the mixer badge shows; a larger `setAudioBuffer` gives the plugin more time), `editor` (its window can be opened), `missing` (the plugin isn't installed: the last scan did not find it. The status is `failed`, the part is silent rather than on its SoundFont voice, and its mix, sound and saved state are kept; once the plugin is back and the plugins are scanned again, it loads as it was. A plugin that is installed but fails to load is not missing). Its volume is still `volume` (CC7), and its pan is CC10; the host applies both to the plugin's output. |
| `patch` | string? | Its own sound library patch (`setPartPatch`). Null: its GM voice plays, through the program map; `voiceName` then names the patch the map sends it to, if any. |
| `sound` | SoundTag? | What's playing (docs/sound-browser.md): `{ id, name }` of the Sound the part plays, as a Sounds catalog id, named as the library names it now (a rename or delete shows at once). A plugin that is loading, playing or muted: the preset it was given (`au:<component>#<key>`, unless its sound is another library sound, one the user saved), else its library sound (`saved:<id>`), else the bare plugin (`au:<component>`). No plugin, or one that failed (it plays the SoundFont): its own SoundFont patch (`saved:<id>`), else what the GM map resolves its voice to, the auto-fill included (`saved:<id>` or `sf:<file>:<bank>:<program>`). Absent for a GM voice nothing covers (or one the map gives a plugin sound no plugin plays). The Sounds dialog marks this row ▶; its footer reads "<name> plays <instrument> · <sound.name>". |
| `soundEdited` | bool? | Its plugin's state no longer matches `sound`: edited in the plugin's editor, or a recalled or restored state that isn't the sound's. Absent when false. Checked by fingerprint off the audio thread: about every half second while the part's plugin window is open (so an edit there shows within about a second, and undoing it clears it), otherwise on the autosave's schedule (every 30 s) and at every explicit state read. `saveSound` or `saveSoundAs` clears it. The demo session's plugin window turns a knob when opened, and back when opened again. |
| `strip` | StripState | Its channel strip, strip 0–3 (below). |

#### StripState

A part's channel strip (Channel strips and send effects, under AppCmd), on every keyboard
part and Style part: `{ eq, comp, inserts, sends }`. A state without it reads as a flat,
empty strip.

| Field | Type | Meaning |
|---|---|---|
| `eq` | PartEq | `setStripEq`. On a keyboard part it is always its `eq`. |
| `comp` | PartCompState | `{ on, preset, threshold, ratio, attack, release, makeup, edited }`: `setStripCompressorOn`, `setStripCompressorPreset` and `setStripCompressorParam`. `threshold` dB −48..0, `ratio` tenths 10–200, `attack` ms 1–100, `release` ms 10–1000, `makeup` dB 0–24; `preset` the type the parameters started from and `edited` whether they now differ from it. Off at Natural by default. Kept and shown; it doesn't play yet. |
| `inserts` | InsertSlotState[2] | Insert 1 and insert 2, each `{ kind, name, on, settings }`: `kind` the insert kind (`none` for an empty slot), `name` as shown ("Auto Wah", "None"), `on` (`setStripInsertOn`), and `settings`, its kind's 2–4 settings in order (none for an empty slot), each a SettingState. Insert 1 is the part's older insert: a keyboard part's `insert`, a Style part's entry in `effects.inserts` while the style has one for it. Insert 2 doesn't play yet. |
| `sends` | number[6] | Its level to sends 1–6, 0–127 (0 for a send that isn't there). `sends[0..3]` are the part's `reverb`, `chorus` and `variation`. |

SettingState, one setting of an insert or parameter of a send effect: `{ name, value, min,
max, default, display }`: `value` in the setting's own unit, clamped to `min`–`max`;
`default` where its kind starts it; `display` the value as it reads ("64", "12 ms",
"0.50 Hz", "40%", "1/8", "On", "2.4 s").

### `mixer`
| Field | Type | Meaning |
|---|---|---|
| `faderPage` | `panel` \| `style` | What the Launchkey faders control. Panel: faders 1–4 are the keyboard parts, fader 5 the Style volume, fader 6 the Multi Pad volume. Style: faders 1–8 are the Style parts. |
| `faderLayer` | `volume` \| `pan` \| `reverb` \| `chorus` \| `delay` | What the faders move across the parts (`setFaderLayer`). |
| `styleSendWaiting` | number | Style parts (bit = part 0–7) whose fader, in a send layer, has moved but not yet reached the send. |
| `sendWaiting` | number | Keyboard parts (bit = part 0–3) whose fader, in a send layer, has moved but not yet reached the value. |
| `styleParts` | StylePart[8] | See the table below. |
| `master` | 0–127? | The synth master level (100 = unity). Null without the synth. |
| `masterWaiting` | bool | The master fader has not yet reached `master`. It turns on as soon as `setMasterVolume` moves the level away from the fader. |
| `styleVolume` | 0–127 | The Style volume (`setStyleVolume`; Panel fader 5): 100 = the Style parts' CC7 as written. |
| `styleVolumeWaiting` | bool | Panel fader 5 has not yet reached `styleVolume`. |
| `multiPadVolume` | 0–127 | The Multi Pad volume (`setMultiPadVolume`; Panel fader 6): 100 = the pads' CC7 as written. |
| `multiPadVolumeWaiting` | bool | Panel fader 6 has not yet reached `multiPadVolume`. |
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
| `reverb`, `chorus`, `variation` | 0–127 | *Superseded by `strip.sends[0..3]` (always the same values); still sent.* Its sends as they play (CC91/93/94, #268): its own where `sendsSet` lists it, else the style's (the default where the style sets none). |
| `sendsSet` | PartSend[] | The sends the player set (`setStylePartSend`); the others follow the style. |
| `strip` | StripState | Its channel strip, strip 4–11 (see StripState under `keyboardParts`). Insert 1 is the style's insert for the part (`effects.inserts`). |
| `voice` | Voice? | The voice the style was written for: `bankMsb`, `bankLsb`, `program` (0-based), `kit` (a drum or SFX kit), and `label` (what the synth plays, for example `≈ Finger Bass  [Yamaha 104/18/88]`). |

### `pads`
The Launchkey's 16 pads. Together with [`surface`](#surface) (every other control,
Shift, the faders and the beat clock), they are a 1:1 mirror of the Launchkey MK4: every
control's meaning, and every LED as the hardware shows it.

| Field | Type | Meaning |
|---|---|---|
| `page` | `sections` \| `chordSetup` \| `otsParts` \| `quickRacks` \| `multiPads` | The current Launchkey pad page. Page 4, Quick Racks: top row Quick Racks 1–8 of the bank on view (red: the loaded rack, blue: a rack, dark: empty; all flashing while Store is armed); bottom row Bank −, Bank +, two dark pads, Store, a dark pad, Rack −, Rack + (`stepQuickRack`). |
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
| `racks` | OtsRack[0–4] | Per OTS of the loaded style (as `settings`), what its button loads: `rack` (the user's rack id `setOtsRack` chose, or null for the style's own), `name` (that rack's name; empty for the style's own) and `missing` (the rack chosen is gone: the style's own loads). |
| `racksReadOnly` | bool | `style-racks.json` can't be changed: it is from a newer yahaha or can't be read (it is never saved over), or there is no data folder. |

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
- **Fader buttons on the Panel page:** the fader layer's colour (`mixer.faderLayer`: VOL
  blue, PAN yellow, REV cyan, CHO pink, DLY white), bright when the part sounds and dim
  when it is off. Fader buttons 5–8 keep their own colours.
- **Fader buttons on the Style page:** green in every layer, bright when the part plays and
  dim when it is muted or muted by Manual Bass.
- **Master button:** the page's colour (on the Panel page, the layer's), bright.

#### SurfaceFader
| Field | Type | Meaning |
|---|---|---|
| `label` | string | What it controls on this page, for example `RIGHT 1`, `BASS` or `MASTER`. Empty when unused: faders 5–8 on the Panel page, a Panel fader 1–4 the controller map sets to none, or the master fader without the synth. A Panel fader 1–4 the map gives another target shows that target's short knob name in capitals (`PANR2`, `HARMARP`, `SPLIT`). |
| `value` | 0–127? | The level it controls (for another target, where it is in its range; in a send layer, the pan or send). Null when unused. |
| `waiting` | bool | The value is waiting for the hardware fader (soft takeover). |
| `position` | 0–127? | Where the hardware fader physically is, as last reported. It is the same physical fader on both pages. Null until it moves. |
| `set` | AppCmd? | What moving it sends: this command with its value filled in, 0 here. `volume` for `setPartVolume`, `setStylePartVolume`, `setStyleVolume`, `setMultiPadVolume`, `setMasterVolume`, or `moveRackFader` for a Panel fader the controller map gives another target. In a send layer (`mixer.faderLayer` not `volume`), faders follow the layer as the hardware faders do (as a Genos slider shows and sets its Slider Assign Type's parameter): Panel faders 1–4 send `setPartPan` (`pan`) or `setPartSend` (`value`), and the Style faders `setStylePartSend` (`value`); in PAN the Style faders are unused. Panel faders 5–6 and the master stay levels. Null when unused. |

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
| `soundFontFile` | string? | The synth's main font: the most GM-complete in the folder (the most GM programs on bank 0, then a drum kit, then the first file name), which plays a channel no route covers. Not a setting: the GM map (`soundLibrary.gmMap`) decides what every program plays, and when the folder changes the main font follows it (it loads on a thread of its own and swaps in between two audio buffers). Null without the synth. |
| `soundFontLoading` | bool | A rack of SoundFonts is loading: a new main font, or fonts the map needs. |
| `synth` | SynthState? | `soundFont`, `device`, `sampleRate` (Hz), `bufferFrames`, `channels`, `outputPair` (1-based, for example [1, 2]), `muted`, `dropouts` (audio dropouts since the synth started: CoreAudio reported an overload, or the audio callback took longer than its buffer lasts; counted on the audio side with atomics, never logged there. The app suggests a larger buffer when 3 come within 30 seconds). Null when the synth is off. |
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
| `sectionReset` | bool | TAP TEMPO while playing resets the section. Default on (the Genos default). |
| `retriggerRate` | 1, 2, 4, 8, 16, 32 | Style Retrigger length. Default 8 (an eighth note). |
| `swing` | 0–100 | Live Swing (`setSwing`). Default 0; each style load sets 0. |
| `swingGrid` | 8, 16 | The swing grid. Default 8. |
| `sectionTempo` | bool | The tempo changes written inside sections play (`setSectionTempo`). Default on. |

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
| `bankName` | string | The bank's name: "New Bank" until it is saved or loaded. |
| `bankPath` | string? | Its file; null while unsaved. Either way every change to the memories is written at once (to the file, or to `ChordLooper/autosave.json`), and the next session starts with this bank. |
| `banks` | BankFile[] | The bank files in the `ChordLooper` folder: `name`, `path`. |

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
| `list` | PluginEntry[] | The installed instrument Audio Units, by manufacturer then name, from the cached scan: `id` (what `setPartPlugin` takes), `name`, `manufacturer`, `version`, `format` (`AUv2` \| `AUv3`), `lastError` (why the last load failed, or null), `inProcess` (the player chose to run it in yahaha's process: `setPluginInProcess`), `canRunInProcess` (every AUv2, and an AUv3 that allows it), `new` (a scan found it for the first time and it hasn't been opened or played since: `markPluginSeen`; the first scan ever marks nothing new), `racks` (how many of the user's racks, `<data>/Racks/*.rack.json`, have a part that plays it), `sounds` (how many sound library sounds play it). |
| `missing` | MissingPlugin[] | Plugins that were installed before and aren't now, and plugins the user's racks or sounds use that aren't installed, by name: `id`, `name` and `manufacturer` (as last installed; the id and `""` if it never was here), `racks`, `sounds`. Empty until the first scan is in. A plugin seen before stays known (`<data>/known-plugins.json`), so reinstalling it doesn't make it `new`. |
| `needsAttention` | RackAttention[] | The user's racks that need attention, by file name: a part's sound (a plugin, or a library sound) is on a missing plugin. `id`, `name`, `parts` (those parts, 0–3). The racks folder is read after each scan and within 2 s of a change. Nothing is rewritten. |
| `instances` | number | Plugin instances loaded now (#407): one for every part, keyboard or Style, that plays a plugin (each part has its own instance), plus one still playing out while its part's next plugin loads. The Mixer and Library › Instruments show it. |

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
| `patches` | PatchInfo[] | In the user's order: `id`, `name`, `category`, `tags`, `favourite`, `source`, `available` (false: it plays the SoundFont fallback) and `note` (why, e.g. "needs plugin hosting (#91)"). `source` is `{ "kind": "soundFont", "file", "bank", "program" }` (bank 128 = drum kits) or `{ "kind": "plugin", "componentId", "hasState", "origin"? }` (the Audio Unit's id, as #91 writes it, and whether the library holds its state). The state itself (base64, MBs for a sampler) stays in the library and is never in the state. A plugin source is the one kind of plugin sound (docs/sound-browser.md): `origin` (absent = made in yahaha) is `{ "kind": "factory", "number" }` or `{ "kind": "file", "path" }` (an `.aupreset`). A factory preset has no state (`hasState` false) until it first plays, when it is captured. A rule command naming a plugin preset id (`au:<id>#f:<n>` or `#u:<path>`) uses that preset's one library sound (or a plugin sound with exactly the preset's settings, such as the one saved with an `.aupreset` by Save as…), adding it once. A patch has no mix settings (docs/racks.md). |
| `categories` | object[] | The Genos voice categories in display order: `id` (`piano`, `ePiano`, `organ`, `guitar`, `bass`, `strings`, `brass`, `saxWoodwind`, `synthLead`, `pad`, `choir`, `drumsPerc`, `sfx`) and `label`. |
| `families` | string[16] | The GM family names; family `i` is programs 8i … 8i+7. |
| `map` | ProgramMap | The global map: `families` (16 patch ids or null), `overrides` (`{ program, patch, volume? }`, by program) and `drums` (a patch id or null). Each rule may have a level: `familyVolumes` (16 levels or null, absent when none has one), an override's `volume`, and `drumsVolume` (absent: none). A rule's level (CC7, 0–127) is what a Style part that resolves by the rule takes when the style sets no level of its own (the mixer shows it); a library of format 2 or older had it on the sound, and it moved onto every rule naming the sound. |
| `styleMap` | ProgramMap | The current style's own map (empty: none). Its rules win over the global map's; what it leaves unset falls through. |
| `styleKey` | string | What the style's own map is stored under: its file name. |
| `usage` | ProgramUse[] | Every program the current style sends its parts (its setup and every section), by channel: `channel` (9–16), `part`, `msb`, `lsb`, `program`, `gmProgram` (what the map looks up), `voice` (the voice without the library), `drums`, `patch` (null: the fallback), `rule` (`drums` \| `override` \| `family` \| `fallback`), `fromStyle`, `plays` (the patch's name, or the voice). |
| `portSendsMapped` | bool | `setPortSendsMapped`. |
| `auditioning` | string? | The patch id being auditioned, or `preset`. |
| `browse` | object? | The SoundFont being browsed: `file`, `presets` (`{ bank, program, name }`), `error`. |
| `file` | string? | Where the library is saved; null when it isn't (an offline session, `state-json`). |
| `extraSoundFonts` | string[] | The SoundFonts the synth has loaded for library patches besides its own. |
| `lastAdded` | string? | The id of the patch last created, duplicated or saved. |
| `gmMap` | GmMapRow[] | The GM map for the style playing (docs/sound-browser.md): 129 rows, the drums first, then programs 0–127. Each row: `program` (null on the drums row), `family` (0–15, null on the drums row), `overrideRule` and `familyRule` (patch ids, the style's own where it has one; on the drums row `familyRule` is the drum rule) and `resolved`: `sound` (a Sound id: `saved:<patch>` for a rule, `sf:<file>:<bank>:<program>` for auto; null when nothing covers it), `layer` (`drums` \| `override` \| `family` \| `auto` \| `none`: the layer that decided it), `fromStyle` (the rule is the style's own) and `font` (`{ file, bank, program }`, the font preset that plays when a font does; null for a plugin sound). Layers, most specific first: drums, override, family (the style's rule before the global one), then auto: the best-matching preset in the scanned fonts, the most GM-complete font first. There is no default sound set. |

### `sounds`
The sound catalog's summary (#117; the list is `sounds()`, see [Sound catalog](#sound-catalog)).

| Field | Type | Meaning |
|---|---|---|
| `revision` | number | Moves whenever the catalog changes (fonts, plugins, saved sounds, favourites, Recents, categories). |
| `count` | number | Entries in the catalog. |
| `scanning` | bool | Plugins are being scanned: more may come. |

The catalog itself: `session.sound_catalog()` (Tauri `sounds()`)
returns `{ revision, entries, recents, fonts }`. Fetch it again when `sounds.revision` moves
(`soundsChanged`). `recents` lists ids, most recent first. `fonts` has one summary per
SoundFont in the folder, for the Instruments tab: `file`, `presets` (melodic presets),
`kits` (bank 128), `gmPrograms` (GM programs on bank 0, of 128) and `gmKit` (it has a
kit): how GM-complete it is, as auto-fill ranks fonts (`patches::gm::gm_completeness`).
Each entry has:

| Field | Type | Meaning |
|---|---|---|
| `id` | string | `sf:<file>:<bank>:<program>`, `au:<component id>` or `saved:<patch id>`. |
| `name` | string | The preset's, the plugin's or the patch's name. |
| `category` | string | One of `soundLibrary.categories`. A preset's comes from its GM family (bank 128: `drumsPerc`), and a plugin's from its name and maker. |
| `source` | `soundFont` \| `plugin` \| `saved` | Where it comes from. |
| `detail` | string | The SoundFont file, the plugin's maker, or what a saved sound plays (its file or component id). |
| `favourite`, `recent` | bool | In the Favourites, in the Recents. |
| `plugin` | object? | Plugins only: `format` (`AUv2` \| `AUv3`), `lastError` (the last load's error, or null), `presets` (how many presets it has, once its factory presets were read; null while unknown, even with `.aupreset` files listed) and `presetsError` (left out unless listing its factory presets failed: why; it is not tried again until the next plugin scan). |

Entries are in this order: presets by file, then bank and program; plugins by maker, then
name; saved sounds in the library's order.

### `paramLocks`
Parameter Lock: `{ splitPoint, fingeringType }`, each a bool (true: locked). All false by
default.

### `dynamics`
Style Dynamics: `{ control, level, touch, accent, accentThreshold, accentMode, accentSource }`.
- `control`: Style Setting › Dynamics Control. Default true.
- `level`: the level in effect, 0–127. Touch moves it. Default 127 (as written); each style load sets it back to 127.
- `touch`, `accent`: default false.
- `accentThreshold`: a velocity from 1 to 127. Default 110.
- `accentMode`: `hits` (default) or `fill`.
- `accentSource`: `left` (default) or `both`.

### `knobs`
The Knob Assign page: `{ page, pageName, pageNumber, pageCount, knobs }`.
- `page`: `style` (the default), `rack` (the live rack's controller map), `pan`, `reverb`, `chorus` or `delay`. `pageNumber` is 1-based.
- `knobs`: always eight, knob 1 first: `{ function, name, short, value, level }`.
  - `function`: `none`, `dynamics`, `retriggerRate`, `retriggerOnOff`, `trackMuteA`,
    `trackMuteB`, `tempo`, `swing`, `partVolume`, `harmonyVolume`, `metronomeVolume`, `partPan`,
    `partReverb`, `partChorus`, `partDelay`, `fxReturn` (an effect block's return level; the `pan` page's
    knobs 5–7 are Reverb, Chorus and Delay Return), `fxParam` (an effect parameter, #236; the
    `name` says which, "Reverb Time"), `delayTime` (the delay's note value, or its ms with
    tempo sync off), `harmonyArp` (the HARMONY/ARPEGGIO switch) or `splitPoint` (its value
    a note name, "F#2").
  - `name` is the full name ("Dynamics Control"); `short` is up to 8 characters ("DynCtrl",
    "---" for No Assign), as the Genos Live Control view and the Launchkey display show it.
  - `value`: the value as text ("64", "1/8", "On", "3 of 8", "All", "120 BPM", a pan "L20" /
    "C" / "R20"); empty for No
    Assign.
  - `level`: where the knob is, 0–127, as the Genos LED ring shows it; null for tempo and No
    Assign. Track Mute A/B keep their own position (they only set the Style parts' switches),
    starting fully right.

### `home`
Read-only: what the Home screen shows, derived from the rest of the state (no commands).

| Field | Type | Meaning |
|---|---|---|
| `mains` | HomeMain[4] | Main A–D: `name`, `present`, `bars` (pattern length), `stepsPerBar` (sixteenths: 16 in 4/4), `density` (note-ons per step over the whole pattern, `bars × stepsPerBar` entries), `lanes` (`kick`, `snare`, `hats`, `bass`: the first bar, the loudest velocity per step, 0 = none), `fill` (`name` "Fill In AA", `present`, `bars`, `active`: queued or playing), `current` (the Main the style is on). Worked out once when the style loads. |
| `progress` | object | `running`, `bar`, `beat` (1-based), `bars` (the section's length; null when stopped), `beatsPerBar`, `fraction` (0–1 through the section, at beat resolution). |
| `ots` | object? | The OTS applied last: `index` (0–3), `name`. |
| `bandSends` | HomeSend[3] | Reverb, Chorus, Delay: `block`, `name`, `effectName`, `level` (the band send, as `setBandSend`). |

### `effects`
`{ blocks }`: the effect bus's Reverb, Chorus and Variation blocks, in that order (#204).
Each is `{ block, name, effect, effectName, types, returnLevel, bandSend, padSend }`: `effect` is the type
(`setEffectType`), `effectName` its name ("Hall", "Delay 1/8."), `types` the block's own
types as `{ effect, name }`, `returnLevel` 0–127 (64 = 0 dB), `bandSend` 0–127 % (#236,
`setBandSend`: 100 = the Style parts' sends as written; reverb 100, chorus 0, variation 0
at start), `padSend` 0–127 % (#267, `setPadSend`: the same for the Multi Pads' sends;
reverb 100, chorus 0, variation 0 at start), and `params` (#236), the block's parameters in order, each
`{ param, name, value, min, max, default, display }`: `value` in the parameter's own unit
(`setEffectParam`), `default` the type's own value, `display` the value as it reads
("2.4 s", "22 ms", "4.5 kHz"); `styleEffect` (#237), the loaded style's own type for the
block as `{ name, effect }` (`name` the XG type, "Real Medium Hall"; `effect` the type it
plays as, null if nothing is near it), or null when the style sets none; and `followStyle`
(`setFollowStyle`).
`inserts` (#269; *superseded by insert 1 of each Style part's `strip`, which shows the same
kind, on/off and amount; still sent*): the loaded style's insertion effects, one per Style part at most, each
`{ part, partName, name, effect, on, amount }`: `part` 0–7, `name` the XG type ("British Combo
Classic"), `effect` what plays it here (`distortion`, `compressor`, `autoWah`, `tremolo`,
`rotary`) or null (the part plays dry), `on` (`setPartInsertOn`), `amount` 0–127
(`setPartInsertAmount`); `insertsOn` (`setInsertsOn`); `rotaryFast` (`setRotaryFast`).
`master`: the Master Compressor and Master EQ (see Effects), `{ compressor, eq }`.
`compressor` is `{ on, preset, compression, texture, output, edited }`; `eq` is
`{ on, preset, bands, edited }`, `bands` the eight `{ gain, freq, q, shelf }`, low to high.
`preset` is the type the settings started from, and `edited` whether they now differ from
it. Absent in an older state: both off.
`sends` (the mixer rework): the send effects, sends 1–6 in order (3 to 6 of them), each a
SendState `{ send, kind, name, params, returnLevel, fromStyle, setByRack }`: `send` 0–5;
`kind` the send kind (`setSendKind`) and `name` as shown ("Hall", "Delay 1/8.");
`params` its kind's parameters in order, each a SettingState (see StripState;
`setSendParam`); `returnLevel` 0–127, 64 = 0 dB (`setSendReturn`); `fromStyle`, fed by the
style's sends (sends 1–3); `setByRack`, the rack sets it (an added send, 4–6, or send 1–3
with `setRackSendOverride` on). Sends 1–3 are `blocks` shown the new way: the same kind,
parameters and return. Absent in an older state: none.

### `liveRack`
The live rack (docs/racks.md): what's under the player's hands now, unsaved changes and
plugin states included. `{ name, id, modified, controls, prompt }`.
- `name`: the saved rack's it came from; `Restored` on the first start after racks came in
  (made from the old `plugin-parts.json` and the parts); `New rack` when it came from none.
- `id`: the saved rack it came from, or null.
- `modified`: something it holds changed since it was loaded or saved: a keyboard part's
  sound, level, pan, sends, octave, voice settings, bend range or switch, the split, the
  keyboard transpose, Harmony/Arp, the controller map, or a plugin edit (`soundEdited`).
  It shows at once (a Launchkey fader within the control thread's next 10 ms). Loading or
  saving a rack clears it (the rack commands, docs/racks.md).
- `controls`: its controller map, `{ version, faders, knobs }`: `version` is 1 (written
  with every map; the app ignores it), then four and eight targets for
  Launchkey faders 1–4 and knobs 1–8 on the Rack knob page. A target is `{ "kind": "none" }`,
  `partLevel`, `partPan`, `partReverb` or `partChorus` with `part` 0–3, `harmonyArp`,
  `splitPoint`, `harmonyVolume`, `metronomeVolume` or `tempo` (knobs only); a target a
  newer build wrote is passed through as it is. A new rack has the four parts' levels on
  faders 1–4 and knobs 1–4, then Harmony volume, Metronome volume, none and Tempo on
  knobs 5–8 (the Parts knob page before racks). A map saved before the map could be edited
  (no `version`), with none on knobs 5–8, reads as that; a map with a `version` is kept
  as saved. The Rack panel edits it (`setRackControl`).
- `prompt`: a rack command waiting for the player's answer, or null. It is set when a
  command is refused for it, and cleared by `dismissRackPrompt` or once a rack is loaded
  or saved.
  - `{ "kind": "unsavedChanges", "then": { "kind": "load", "id", "name" } }` (or
    `"then": { "kind": "new" }`): `loadRack` / `newRack` with unsaved changes. The app
    offers Save first (`saveRack` or `saveRackAs` while this prompt is up: the engine holds
    the switch and makes it once the save is done, through a `soundNames` prompt if one
    comes; a save that fails drops it, so the app never sends the switch itself), Discard and switch (the switch with
    `discard: true`) and Keep editing (`dismissRackPrompt`).
  - `{ "kind": "soundNames", "parts": [{ "part", "suggested" }], "saveAs" }`: `saveRack`
    (`saveAs` null) or `saveRackAs` (`saveAs` the rack's name) found edited presets that
    become new sounds. `suggested` is the preset's name. Send the save again with
    `soundNames`.

A live session autosaves the live rack to `~/Library/Application Support/yahaha/live-rack.json`
(atomically, off the control and audio threads: a second after the last change, or at most
ten seconds after the first unsaved one, and on stop with the plugins' states read afresh)
and applies it at the next start, so the parts sound and mix as before quitting, saved or
not. A file that can't be read is moved aside (`live-rack.json.bak`) and the session starts
on its defaults; one a newer yahaha wrote is left alone and not saved over. An offline
session keeps no live rack unless `Options::live_rack` names a file.

### `racks`
The user's racks (`<data>/Racks/*.rack.json`), by file name, for Library › Racks. Each is
`{ id, name, parts, on, needsAttention }`: `parts` names the sound each keyboard part plays
(Right 1, Right 2, Right 3, Left: the library sound's name, the SoundFont preset's or GM
voice's, or the plugin's), `on` says which parts are on, and `needsAttention` is true when a
part's sound is on a missing plugin (`plugins.needsAttention` names the parts). The folder
is read at start, after each rack command, after each plugin scan, and within 2 s of a
change on disk. A rack that can't be read is not listed.

### `quickRacks`
[Quick Racks](#quick-racks), as the bar and pad page 4 show them.

| Field | Type | Meaning |
|---|---|---|
| `bank` | 0–7 | The bank on view (0 = A). |
| `buttons` | QuickRackButton[] | Its eight buttons: `rack` (the rack's id; null when empty), `name` (the rack's; empty when the button is empty or its rack is gone), `missing` (it names a rack that isn't in `racks`), `loaded` (its rack is `liveRack.id`: lit). |
| `store` | bool | Store is armed: the next press stores the live rack. |
| `storeWaiting` | 0–7? | A button of the bank on view waiting for the live rack to be saved before it is stored there. |
| `readOnly` | bool | Quick Racks can't be changed: no data folder, or the file is from a newer yahaha (or can't be read). |

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
every audio buffer, and republishing the state for them would flood the clients. The app
shell reads it at about 30 Hz and sends each frame as the Tauri event `meters` (payload:
this object); the `meters` command returns the latest frame.

| Field | Type | Meaning |
|---|---|---|
| `atMs` | ms | The session clock at the read. |
| `channels` | `{ channel, peak, rms, cpu, cpuPeak }[]` | All 16 channels: 1–4 the keyboard parts, 5–8 the Multi Pads, 9–16 the Style parts. `peak`: the highest since the last read; `rms`: the loudest audio buffer's RMS since the last read. Linear (1.0 = full scale), after the master level, before the soft clipper. `cpu` (#340): the track's render time over the last second (its SoundFont voices, part filter and insertion effect, or the plugin that plays it) as a share of the audio buffers' time (1.0 = the whole buffer); `cpuPeak`: its slowest single buffer in that second, the same way. Empty without the synth. |
| `master` | [l, r] | The peaks after the soft clipper. |
| `masterRms` | [l, r] | The RMS after the soft clipper, the loudest buffer's since the last read. |
| `clips` | number | Audio buffers in which the soft clipper worked (above −1 dBFS), since start. |
| `cpu` | `{ total, peak, bufferUs }` | #340: every track together. `total`: their render time over the last second as a share of the buffers' time; `peak`: the slowest single buffer's; `bufferUs`: the audio buffer's length in µs (0 before the first buffer). The effect bus, the click and the output stage are not in it. |

Each read takes the levels (they restart from 0), so there is one reader (the app shell's
meter thread); the client does the decay and peak hold. Measuring costs no allocation or
lock on the audio thread: the rack sums the squares of each part's stem as it mixes it
(`src/synth/rack.rs`) and the callback folds the result into atomics.

The CPU figures are not reset by a read: the audio callback times each track's render
(the rack's lanes, filter and insert for the channel, and its plugin's render) and adds
it, with the buffer's length, to running totals in atomics (`synth::CpuCounters`); the
worst buffer goes into an atomic maximum. A read at least a second after the last reading
takes the difference of the totals (`synth::CpuWindow`) off the audio thread, so a reading
changes once a second and every client polling the meters sees the same one. The app's
Mixer reads the meters twice a second while it is open. Both mocks make up plausible
figures (a playing plugin's `cpu`, the Style parts while the band plays).

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

The `library`, `surface.trackPrev`/`trackNext`, the master fader and `io` show what a
live session reports with a library folder, a Launchkey and the synth. Right 1's strip has a
phaser in insert 2 and sends to a phaser the player added as send 4 (`effects.sends`).

Unabridged fixtures from `yahaha state-json` are in `tests/fixtures/`: `state.json` (SlowWalker,
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
    "landing": "Main B",
    "acmp": true,
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
    "unison": false,
    "unisonLatched": false,
    "unisonType": "root",
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
    "settleMs": 10,
    "leftHold": false
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
      "pan": 64,
      "reverb": 40,
      "chorus": 0,
      "variation": 0,
      "eq": { "lowGain": 3, "lowFreq": 80, "highGain": -2, "highFreq": 10000 },
      "insert": { "effect": "rotary", "on": true, "amount": 90 },
      "strip": {
        "eq": { "lowGain": 3, "lowFreq": 80, "highGain": -2, "highFreq": 10000 },
        "comp": { "on": false, "preset": "natural", "threshold": -18, "ratio": 25, "attack": 10, "release": 200, "makeup": 3, "edited": false },
        "inserts": [
          {
            "kind": "rotary", "name": "Rotary", "on": true,
            "settings": [
              { "name": "Depth", "value": 90, "min": 0, "max": 127, "default": 64, "display": "90" },
              { "name": "Drive", "value": 0, "min": 0, "max": 127, "default": 0, "display": "0" },
              { "name": "Balance", "value": 64, "min": 0, "max": 127, "default": 64, "display": "64" }
            ]
          },
          {
            "kind": "phaser", "name": "Phaser", "on": true,
            "settings": [
              { "name": "Depth", "value": 64, "min": 0, "max": 127, "default": 64, "display": "64" },
              { "name": "Rate", "value": 50, "min": 5, "max": 500, "default": 50, "display": "0.50 Hz" },
              { "name": "Feedback", "value": 40, "min": 0, "max": 90, "default": 40, "display": "40%" }
            ]
          }
        ],
        "sends": [40, 0, 0, 30, 0, 0]
      },
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
        "recentOverruns": 0,
        "editor": true,
        "missing": false
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
      "pan": 64,
      "reverb": 40,
      "chorus": 0,
      "variation": 0,
      "eq": { "lowGain": 0, "lowFreq": 80, "highGain": 0, "highFreq": 10000 },
      "insert": { "effect": "distortion", "on": false, "amount": 64 },
      "strip": {
        "eq": { "lowGain": 0, "lowFreq": 80, "highGain": 0, "highFreq": 10000 },
        "comp": { "on": false, "preset": "natural", "threshold": -18, "ratio": 25, "attack": 10, "release": 200, "makeup": 3, "edited": false },
        "inserts": [{ "kind": "none", "name": "None", "on": false, "settings": [] }, { "kind": "none", "name": "None", "on": false, "settings": [] }],
        "sends": [40, 0, 0, 0, 0, 0]
      },
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
      "pan": 64,
      "reverb": 40,
      "chorus": 0,
      "variation": 0,
      "eq": { "lowGain": 0, "lowFreq": 80, "highGain": 0, "highFreq": 10000 },
      "insert": { "effect": "distortion", "on": false, "amount": 64 },
      "strip": {
        "eq": { "lowGain": 0, "lowFreq": 80, "highGain": 0, "highFreq": 10000 },
        "comp": { "on": false, "preset": "natural", "threshold": -18, "ratio": 25, "attack": 10, "release": 200, "makeup": 3, "edited": false },
        "inserts": [{ "kind": "none", "name": "None", "on": false, "settings": [] }, { "kind": "none", "name": "None", "on": false, "settings": [] }],
        "sends": [40, 0, 0, 0, 0, 0]
      },
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
      "pan": 64,
      "reverb": 40,
      "chorus": 0,
      "variation": 0,
      "eq": { "lowGain": 0, "lowFreq": 80, "highGain": 0, "highFreq": 10000 },
      "insert": { "effect": "distortion", "on": false, "amount": 64 },
      "strip": {
        "eq": { "lowGain": 0, "lowFreq": 80, "highGain": 0, "highFreq": 10000 },
        "comp": { "on": false, "preset": "natural", "threshold": -18, "ratio": 25, "attack": 10, "release": 200, "makeup": 3, "edited": false },
        "inserts": [{ "kind": "none", "name": "None", "on": false, "settings": [] }, { "kind": "none", "name": "None", "on": false, "settings": [] }],
        "sends": [40, 0, 0, 0, 0, 0]
      },
      "patch": null
    }
  ],
  "mixer": {
    "faderPage": "panel",
    "faderLayer": "volume",
    "sendWaiting": 0,
    "styleSendWaiting": 0,
    "styleParts": [
      {
        "name": "Rhythm 1",
        "channel": 9,
        "on": true,
        "mutedByManualBass": false, "reverb": 40, "chorus": 0, "variation": 0, "sendsSet": [],
        "strip": {
          "eq": { "lowGain": 0, "lowFreq": 80, "highGain": 0, "highFreq": 10000 },
          "comp": { "on": false, "preset": "natural", "threshold": -18, "ratio": 25, "attack": 10, "release": 200, "makeup": 3, "edited": false },
          "inserts": [{ "kind": "none", "name": "None", "on": false, "settings": [] }, { "kind": "none", "name": "None", "on": false, "settings": [] }],
          "sends": [40, 0, 0, 0, 0, 0]
        },
        "volume": 65,
        "waiting": false,
        "fader": null,
        "voice": { "bankMsb": 127, "bankLsb": 0, "program": 57, "kit": true, "label": "drum kit 127/0/58" }
      },
      {
        "name": "Rhythm 2",
        "channel": 10,
        "on": true,
        "mutedByManualBass": false, "reverb": 40, "chorus": 0, "variation": 0, "sendsSet": [],
        "strip": {
          "eq": { "lowGain": 0, "lowFreq": 80, "highGain": 0, "highFreq": 10000 },
          "comp": { "on": false, "preset": "natural", "threshold": -18, "ratio": 25, "attack": 10, "release": 200, "makeup": 3, "edited": false },
          "inserts": [{ "kind": "none", "name": "None", "on": false, "settings": [] }, { "kind": "none", "name": "None", "on": false, "settings": [] }],
          "sends": [40, 0, 0, 0, 0, 0]
        },
        "volume": 70,
        "waiting": false,
        "fader": null,
        "voice": { "bankMsb": 127, "bankLsb": 0, "program": 56, "kit": true, "label": "drum kit 127/0/57" }
      },
      {
        "name": "Bass",
        "channel": 11,
        "on": true,
        "mutedByManualBass": false, "reverb": 40, "chorus": 0, "variation": 0, "sendsSet": [],
        "strip": {
          "eq": { "lowGain": 0, "lowFreq": 80, "highGain": 0, "highFreq": 10000 },
          "comp": { "on": false, "preset": "natural", "threshold": -18, "ratio": 25, "attack": 10, "release": 200, "makeup": 3, "edited": false },
          "inserts": [{ "kind": "none", "name": "None", "on": false, "settings": [] }, { "kind": "none", "name": "None", "on": false, "settings": [] }],
          "sends": [40, 0, 0, 0, 0, 0]
        },
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
    "styleVolume": 100,
    "styleVolumeWaiting": false,
    "multiPadVolume": 100,
    "multiPadVolumeWaiting": false,
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
    "linkTiming": "mainChange",
    "racks": [{ "rack": "3f2a9c1e", "name": "Ballad Pad", "missing": false }],
    "racksReadOnly": false
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
      "muted": false,
      "dropouts": 0
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
    "sectionReset": true,
    "retriggerRate": 8,
    "swing": 0,
    "swingGrid": 8,
    "sectionTempo": true
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
    ],
    "bankName": "New Bank",
    "bankPath": null,
    "banks": []
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
      { "id": "aumu dls  appl", "name": "DLSMusicDevice", "manufacturer": "Apple", "version": "1.0.0", "format": "AUv2", "lastError": null, "inProcess": false, "canRunInProcess": true, "new": false, "racks": 1, "sounds": 2 }
    ],
    "missing": [
      { "id": "aumu Smp7 Fake", "name": "Sampler Deluxe", "manufacturer": "Fake Instruments", "racks": 1, "sounds": 1 }
    ],
    "needsAttention": [
      { "id": "r5f3a2c1d-0", "name": "Ballad", "parts": [1] }
    ],
    "instances": 1
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
        "available": true,
        "note": null
      },
      {
        "id": "keys",
        "name": "Keys",
        "category": "ePiano",
        "tags": [],
        "favourite": false,
        "source": { "kind": "plugin", "componentId": "aumu dls  appl", "hasState": false },
        "available": false,
        "note": "needs plugin hosting (#91)"
      }
    ],
    "categories": [{ "id": "piano", "label": "Piano" }, { "id": "bass", "label": "Bass" }],
    "families": ["Piano", "Chromatic Perc.", "Organ", "Guitar", "Bass", "Strings", "Ensemble", "Brass", "Reed", "Pipe", "Synth Lead", "Synth Pad", "Synth FX", "Ethnic", "Percussive", "Sound FX"],
    "map": {
      "families": [null, null, null, null, "my-bass", null, null, null, null, null, null, null, null, null, null, null],
      "familyVolumes": [null, null, null, null, 100, null, null, null, null, null, null, null, null, null, null, null],
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
    "lastAdded": "my-bass",
    "gmMap": [
      { "program": null, "family": null, "overrideRule": null, "familyRule": "studio-kit", "resolved": { "sound": "saved:studio-kit", "layer": "drums", "fromStyle": false, "font": { "file": "GeneralUser-GS.sf2", "bank": 128, "program": 0 } } },
      { "program": 16, "family": 2, "overrideRule": null, "familyRule": null, "resolved": { "sound": "sf:GeneralUser-GS.sf2:0:16", "layer": "auto", "fromStyle": false, "font": { "file": "GeneralUser-GS.sf2", "bank": 0, "program": 16 } } }
    ]
  },
  "paramLocks": { "splitPoint": false, "fingeringType": true },
  "sounds": { "revision": 3, "count": 1219, "scanning": false },
  "dynamics": { "control": true, "level": 72, "touch": true, "accent": true, "accentThreshold": 110, "accentMode": "hits", "accentSource": "left" },
  "knobs": {
    "page": "style",
    "pageName": "Style",
    "pageNumber": 1,
    "pageCount": 4,
    "knobs": [
      { "function": "dynamics", "name": "Dynamics Control", "short": "DynCtrl", "value": "72", "level": 72 },
      { "function": "retriggerRate", "name": "Retrigger Rate", "short": "RtgRate", "value": "1/8", "level": 76 },
      { "function": "retriggerOnOff", "name": "Retrigger On/Off", "short": "RtgOnOff", "value": "Off", "level": 0 },
      { "function": "trackMuteA", "name": "Style Track Mute A", "short": "StyMuteA", "value": "All", "level": 127 },
      { "function": "trackMuteB", "name": "Style Track Mute B", "short": "StyMuteB", "value": "All", "level": 127 },
      { "function": "swing", "name": "Swing", "short": "Swing", "value": "0%", "level": 0 },
      { "function": "none", "name": "No Assign", "short": "---", "value": "", "level": null },
      { "function": "tempo", "name": "Tempo", "short": "Tempo", "value": "92 BPM", "level": null }
    ]
  },
  "effects": {
    "blocks": [
      {
        "block": "reverb", "name": "Reverb", "effect": "hall", "effectName": "Hall", "returnLevel": 64, "bandSend": 100, "padSend": 100,
        "params": [
          { "param": "reverbTime", "name": "Time", "value": 24, "min": 3, "max": 100, "default": 24, "display": "2.4 s" },
          { "param": "preDelay", "name": "Pre-delay", "value": 22, "min": 0, "max": 200, "default": 22, "display": "22 ms" },
          { "param": "reverbTone", "name": "Tone", "value": 45, "min": 10, "max": 200, "default": 45, "display": "4.5 kHz" }
        ],
        "styleEffect": { "name": "Real Medium Hall", "effect": "hall" }, "followStyle": true,
        "types": [{ "effect": "hall", "name": "Hall" }, { "effect": "room", "name": "Room" }, { "effect": "stage", "name": "Stage" }, { "effect": "plate", "name": "Plate" }]
      },
      {
        "block": "chorus", "name": "Chorus", "effect": "chorus", "effectName": "Chorus", "returnLevel": 64, "bandSend": 0, "padSend": 0,
        "params": [
          { "param": "chorusRate", "name": "Rate", "value": 55, "min": 5, "max": 500, "default": 55, "display": "0.55 Hz" },
          { "param": "chorusDepth", "name": "Depth", "value": 22, "min": 0, "max": 50, "default": 22, "display": "2.2 ms" }
        ],
        "styleEffect": null, "followStyle": true,
        "types": [{ "effect": "chorus", "name": "Chorus" }, { "effect": "celeste", "name": "Celeste" }, { "effect": "flanger", "name": "Flanger" }]
      },
      {
        "block": "variation", "name": "Variation", "effect": "dottedEighth", "effectName": "Delay 1/8.", "returnLevel": 64, "bandSend": 0, "padSend": 0,
        "params": [
          { "param": "delaySync", "name": "Tempo sync", "value": 1, "min": 0, "max": 1, "default": 1, "display": "On" },
          { "param": "delayNote", "name": "Note", "value": 4, "min": 0, "max": 7, "default": 4, "display": "1/8." },
          { "param": "delayTime", "name": "Time", "value": 375, "min": 10, "max": 2000, "default": 375, "display": "375 ms" },
          { "param": "delayFeedback", "name": "Feedback", "value": 38, "min": 0, "max": 90, "default": 38, "display": "38%" },
          { "param": "delayTone", "name": "Tone", "value": 50, "min": 10, "max": 200, "default": 50, "display": "5.0 kHz" },
          { "param": "pingPong", "name": "Ping-pong", "value": 0, "min": 0, "max": 1, "default": 0, "display": "Off" }
        ],
        "styleEffect": null, "followStyle": true,
        "types": [{ "effect": "eighth", "name": "Delay 1/8" }, { "effect": "dottedEighth", "name": "Delay 1/8." }, { "effect": "quarter", "name": "Delay 1/4" }, { "effect": "pingPong", "name": "Ping-Pong" }]
      }
    ],
    "inserts": [], "insertsOn": true, "rotaryFast": false,
    "master": {
      "compressor": { "on": true, "preset": "natural", "compression": 30, "texture": 50, "output": 1, "edited": false },
      "eq": {
        "on": false, "preset": "flat",
        "bands": [
          { "gain": 0, "freq": 80, "q": 7, "shelf": true }, { "gain": 0, "freq": 250, "q": 7, "shelf": false },
          { "gain": 0, "freq": 500, "q": 7, "shelf": false }, { "gain": 0, "freq": 630, "q": 7, "shelf": false },
          { "gain": 0, "freq": 800, "q": 7, "shelf": false }, { "gain": 0, "freq": 1000, "q": 7, "shelf": false },
          { "gain": 0, "freq": 4000, "q": 7, "shelf": false }, { "gain": 0, "freq": 8000, "q": 7, "shelf": true }
        ],
        "edited": false
      }
    },
    "sends": [
      {
        "send": 0, "kind": "hall", "name": "Hall",
        "params": [
          { "name": "Time", "value": 24, "min": 3, "max": 100, "default": 24, "display": "2.4 s" },
          { "name": "Pre-delay", "value": 22, "min": 0, "max": 200, "default": 22, "display": "22 ms" },
          { "name": "Tone", "value": 45, "min": 10, "max": 200, "default": 45, "display": "4.5 kHz" }
        ],
        "returnLevel": 64, "fromStyle": true, "setByRack": false
      },
      {
        "send": 1, "kind": "chorus", "name": "Chorus",
        "params": [
          { "name": "Rate", "value": 55, "min": 5, "max": 500, "default": 55, "display": "0.55 Hz" },
          { "name": "Depth", "value": 22, "min": 0, "max": 50, "default": 22, "display": "2.2 ms" }
        ],
        "returnLevel": 64, "fromStyle": true, "setByRack": false
      },
      {
        "send": 2, "kind": "dottedEighth", "name": "Delay 1/8.",
        "params": [
          { "name": "Tempo sync", "value": 1, "min": 0, "max": 1, "default": 1, "display": "On" },
          { "name": "Note", "value": 4, "min": 0, "max": 7, "default": 4, "display": "1/8." },
          { "name": "Time", "value": 375, "min": 10, "max": 2000, "default": 375, "display": "375 ms" },
          { "name": "Feedback", "value": 38, "min": 0, "max": 90, "default": 38, "display": "38%" },
          { "name": "Tone", "value": 50, "min": 10, "max": 200, "default": 50, "display": "5.0 kHz" },
          { "name": "Ping-pong", "value": 0, "min": 0, "max": 1, "default": 0, "display": "Off" }
        ],
        "returnLevel": 64, "fromStyle": true, "setByRack": false
      },
      {
        "send": 3, "kind": "phaser", "name": "Phaser",
        "params": [
          { "name": "Depth", "value": 64, "min": 0, "max": 127, "default": 64, "display": "64" },
          { "name": "Rate", "value": 50, "min": 5, "max": 500, "default": 50, "display": "0.50 Hz" },
          { "name": "Feedback", "value": 40, "min": 0, "max": 90, "default": 40, "display": "40%" }
        ],
        "returnLevel": 64, "fromStyle": false, "setByRack": true
      }
    ]
  },
  "home": { "mains": [], "progress": { "running": false, "bar": 1, "beat": 1, "bars": null, "beatsPerBar": 4, "fraction": 0.0 }, "ots": null, "bandSends": [] },
  "liveRack": {
    "name": "Restored", "id": null, "modified": true,
    "controls": {
      "version": 1,
      "faders": [{ "kind": "partLevel", "part": 0 }, { "kind": "partLevel", "part": 1 }, { "kind": "partLevel", "part": 2 }, { "kind": "partLevel", "part": 3 }],
      "knobs": [{ "kind": "partLevel", "part": 0 }, { "kind": "partLevel", "part": 1 }, { "kind": "partLevel", "part": 2 }, { "kind": "partLevel", "part": 3 }, { "kind": "harmonyVolume" }, { "kind": "metronomeVolume" }, { "kind": "none" }, { "kind": "tempo" }]
    },
    "prompt": null
  },
  "racks": [
    { "id": "r5f3a2c1d-0", "name": "Ballad", "parts": ["Grand Piano", "Sampler Deluxe", "Brass Section", "Strings"], "on": [true, true, false, true], "needsAttention": true }
  ],
  "quickRacks": {
    "bank": 0,
    "buttons": [
      { "rack": "r5f3a2c1d-0", "name": "Ballad", "missing": false, "loaded": false },
      { "rack": null, "name": "", "missing": false, "loaded": false },
      { "rack": "r1b2c3d4e-1", "name": "", "missing": true, "loaded": false },
      { "rack": null, "name": "", "missing": false, "loaded": false },
      { "rack": null, "name": "", "missing": false, "loaded": false },
      { "rack": null, "name": "", "missing": false, "loaded": false },
      { "rack": null, "name": "", "missing": false, "loaded": false },
      { "rack": null, "name": "", "missing": false, "loaded": false }
    ],
    "store": false,
    "storeWaiting": null,
    "readOnly": false
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
    `tests/it/engine_no_alloc.rs` checks both allocate and free nothing on the engine thread.
  - The input thread keeps each key's state (held, side, parts) and each source's held
    keys in atomics for the key strip; the control side reads them.
- The audio thread measures each part's and the master's peak into atomics (`meters`).
  A new rack of SoundFonts (a new main font, or fonts the GM map needs) loads on a thread
  of its own into a new pair of
  synthesizers (the band's and the keyboard parts'), which the control side hands to the
  audio thread through a ring; the old pair comes back through another ring and is freed
  on the control side.
- A rescan (`rescanLibrary`) walks the folders on a thread of its own.
- The synth's audio stream lives on a thread of its own, which keeps `Session` `Send`.
