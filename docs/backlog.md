# Backlog

Issues live on GitHub, grouped by milestone. This file is the snapshot they were created from.

Scope: **playability**. Load Genos style files and play them the way the hardware does. There are no editors in scope. Spec references `§` point to [genos-features.md](genos-features.md).

Each `## M` heading is a milestone and each `###` is an issue. `labels:` sets the issue's labels.

---

## M1 · Chord recognition 1:1
The left hand has to be read exactly the way a Genos reads it before anything downstream can be trusted.

### Test harness: golden snapshots for recognizer and transposer (#1)
labels: harness, chord-following
- Build a table-driven test that covers every chord type in the Data List, in all 12 roots, in every inversion, and with the optional notes omitted.
- Add a snapshot mode to `yahaha sim`: a corpus style plus a chord script produces a per-part note list, saved under `tests/golden/`. Record the output only for our own scripts, and never commit style data.
- Show a diff on every change, so any change to chord behaviour is a deliberate, reviewed one.

### Recognizer: match the Data List chord table exactly (#2)
labels: chord-following
- Cover all 37 entries, including the ones that have no MIDI code: M7b5, (b5) and mM7b5. Decide which CASM type each of those maps to.
- Handle the voicings where notes may be left out, e.g. `1+3+(5)+7`.
- Produce 1+8. Right now an octave collapses to major.
- Resolve ambiguous inversions (C6 vs Am7, dim7, aug, m6 vs m7b5) and record each rule in the spec.
- Write one test per row.

### Fingering modes: Single, Multi, Fingered, On Bass, AI Fingered, Full, AI Full (#3)
labels: chord-following, settings
- Make the fingering type a setting (§ Chord following spec). Implement:
  - Single Finger: white/black key below the root.
  - Multi Finger auto-detection.
  - Fingered: bass is the root.
  - Fingered On Bass: this is today's behaviour.
  - AI Fingered: infer from under 3 notes using the previous chord.
  - Full Keyboard and AI Full Keyboard: detect across the whole keyboard, with the documented limits (no Sync Stop, no 9th/11th/13th under AI Full).

### Chord Cancel and the 1+5 / 1+8 behaviour (#4)
labels: chord-following
- Cancel (root + b2 + 2) only exists in Fingered, On Bass and AI Fingered.
- Decide and document what each part plays under Cancel, 1+5 and 1+8, using CASM chord mute plus the corpus-consistency check (M2).

### Chord detection area: Upper, and Manual Bass (#5)
labels: chord-following
- Upper mode: the chord comes from the right hand, fingering becomes Fingered* (no 1+5, 1+8 or Cancel), Manual Bass is on, the style's Bass channel is muted, and the Left part plays bass.
- Follow-up: Manual Bass takes the Bass voice from the Style's init setup only. A Bass program change inside a section (per-section voice) is not followed yet.

### Keyboard transpose feeds the chord root (#6)
labels: chord-following
- Transpose shifts both the played notes and the chord root sent to the style. Cover Keyboard, Song and Master transpose (song transpose only as far as it affects playing).

### Bug: an out-of-range Ctab src_type can panic on the RT thread (#7)
labels: bug, chord-following
- `importance` and `mask_of` index `TONES[ty]` without a bounds check. Clamp or validate at parse time, and add a fuzz/corpus test.

---

## M2 · Chord-to-note conversion 1:1
Make NTR, NTT and RTR match the hardware, checked against an oracle built from the corpus itself.

### Corpus self-consistency oracle (#8)
labels: harness, chord-following
- Many styles have separate source channels for major and minor chords, routed by chord mute.
- Play the major source through our NTT on a minor chord and compare the result with the minor source the author wrote. Score it per NTT table and per style. Track the score in CI and make every NTT change report its delta.

### Genos-owner reference capture kit (#9)
labels: harness, community
- Write a test .mid chord script with instructions for a Genos/PSR owner: load style X, play the script into ACMP, and record the MIDI out.
- Write a diff tool that turns their recording into golden data. This produces the forum post draft; posting it is up to Ryan.

### RTR: real Pitch Shift and Note Generator (#10)
labels: chord-following
- Pitch Shift should bend the sounding note (not note-off/note-on). Implement Pitch Shift to Root the same way, and add Note Generator. Currently all of these retrigger.

### NTT 5th-variation tables (aug/dim handling) (#11)
labels: chord-following
- Melodic minor 5th, Harmonic minor 5th, Natural minor 5th and Dorian 5th currently behave like their base tables. Implement the flattened/sharpened-5th handling.

### Guitar NTR: All-Purpose, Stroke and Arpeggio tables (#12)
labels: chord-following
- `guitar()` ignores the NTT value. Implement per-table voicing behaviour, and replace the string/position heuristic with a documented model.

### Note limits narrower than an octave; per-zone Bass On (#13)
labels: chord-following
- `fold_into` ignores ranges under 11 semitones. Work out and implement the hardware behaviour.
- Track Bass On per SFF2 zone instead of ORing it across zones.

### SFF1 NTT codes above 5, and Cntt precedence (#14)
labels: chord-following, sff
- Decode every SFF1 NTT value instead of falling back to Melody.
- Cntt vs Ctab/Ctb2 precedence: settled from corpus evidence (see "SFF1 encoding" in docs/genos-features.md). The oracle may still overrule the Cntt Bass On rule.

### Parse SInt properly and re-apply it on section change (#15)
labels: sff, engine
- Parse SInt into structured per-channel init (bank, program, volume, pan, sends, XG effects).
- Re-apply it on every section change, the way newer instruments do.

---

## M3 · Engine API + Tauri shell
### Extract the `engine` library with a command/event API (#16)
labels: architecture
- Move every mutation behind `Cmd`, including SynthControl voice/OTS/left writes that currently bypass the engine.
- Publish state as `Snapshot` and events. The TUI becomes the first client. No real-time behaviour changes; bench latency must not regress.

### Tauri app skeleton (#17)
labels: ui
- The webview only displays state: it subscribes to snapshots (~60 Hz) and sends commands. MIDI, audio and timing stay in Rust.
- Panel: transport, sections with lamp states, chord display, tempo, parts, mixer.

### Launchkey and keyboard control parity in the app (#18)
labels: ui
- Everything the TUI and Launchkey can do works while the app is running. Pad LEDs stay in sync.

---

## M4 · Style browser
### Style library index (#19)
labels: browser
- Scan folders and cache each style's name, category, tempo, time signature, available sections, OTS names, source file and SFF version. Rescan incrementally.

### Browser UI: categories, search, favorites, recents (#20)
labels: browser, ui
- Genos-style category pages plus instant search. Use keyboard/pad navigation for live use.

### Audition on hover (#21)
labels: browser
- Preview a style with a short default progression without disturbing the current performance, or as a clean handoff when it's stopped.

---

## M5 · Sections & transport parity
### Section Change Timing settings (Main: Immediate/Next Bar; Intro/Ending: Next Bar/End of Section) (#22)
labels: sections, settings

### Ending ritardando, fade in/out, Synchro Stop window (#23)
labels: transport

### Fill Up / Down / Self / Break and Half Bar Fill (assignable) (#24)
labels: sections

### Style Section Reset (Tap during playback) and Style Retrigger (#25)
labels: sections, transport

### Style-change rules: Lock/Hold/Reset for tempo, part on/off and section (#26)
labels: settings

### Stop Accompaniment modes: Off / Style / Fixed (#27)
labels: chord-following, settings

### OTS Link timing (at section change vs immediate); OTS turns ACMP and Sync Start on (#28)
labels: registration

### Chord Looper (8 memories, bar-quantized record/loop) (#29)
labels: chord-following, transport

### Part solo and Style Track Mute; tempo range 5–500; metronome (#30)
labels: mixer, transport

---

## M6 · Parts, voices & live play
### Parts model: Right 1–3 + Left, three split points, Left Hold, per-part octave (#31)
labels: voices
- Done: the Genos part model (Right 1–3 + Left, per-part voice, volume, octave, on/off) replaced the voice slots; OTS targets these parts.
- Still open: three split points (Style, Left, Right 3). Left Hold: #202.

### Keyboard Harmony (our own implementation of the documented types) (#32)
labels: voices
- Includes chord source rules for each ACMP/LEFT combination (§6).

### Arpeggio engine (our own patterns) (#33)
labels: voices

### Controllers: pedals, joystick, sustain, assignable functions (#34)
labels: voices, settings

### Plugin hosting (AU/VST3) for right-hand parts (#35)
labels: voices, audio
- This is the realistic route to Genos-class sound without Yamaha samples.

---

## M7 · Registration, Multi Pads, Playlist
### Registration Memory 1–10 + banks, Freeze, Registration Sequence (#36)
labels: registration

### Multi Pads: parse .pad, playback, Repeat, Chord Match, Synchro Start (#37)
labels: multipad

### Playlist (#38)
labels: registration

---

## Out of scope (for now)
Editors (Style, Voice, Multi Pad), Song player/recorder, Audio Styles, Mic/Vocal Harmony, Expansion Pack decryption, Yamaha content, Unison & Accent (PSR-SX only).
