# Genos2 playability feature catalogue (yahaha spec)

Extracted from the Genos2 manuals in `docs/manuals/`:

- **OM**: Genos2 Owner's Manual (`Genos2_owners_manual.txt`)
- **RM**: Genos2 Reference Manual V1.10 (`Genos2_reference_manual.txt`)
- **DL**: Genos Data List v2.00 (`Genos_data_list.txt`). This is the Data List for the original **Genos** (2019), not Genos2. Its chord table, parameter chart and MIDI formats are the best spec we have, but a few details may differ on Genos2.

Page numbers are the printed page numbers. In these PDFs they equal the PDF page index, which is also the form-feed page index in the `.txt` files.

Everything below is paraphrased. Where the manuals give no value, default or rule, the entry says **(not specified)** so we know we have to choose it ourselves.

Tags: `[chord-following]` `[transport]` `[sections]` `[voices]` `[registration]` `[multipad]` `[mixer]` `[settings]` `[ui]`

---

## 0. Hardware facts that shape behaviour

| Fact | Value | Ref |
|---|---|---|
| Keys | 76. The top key is G6: F#6 is called the "second right-most key", so the range is E0–G6 (the range is derived, not stated) | OM p.97, p.132 |
| Keyboard parts | Right 1, Right 2, Right 3, Left | OM p.16, p.132 |
| Default split point | F#2. F#2 and below is the left-hand section; the right-hand section starts at G2 | OM p.16, p.48 |
| Style sections | Intro ×3, Main ×4, Fill ×4, Break ×1, Ending ×3 | OM p.132 |
| OTS per style | 4 | OM p.132 |
| Registration buttons | 10 per bank | OM p.133 |
| Multi Pads | banks of 4 pads | OM p.16, p.133 |
| Tempo range | 5–500 BPM, plus Tap Tempo | OM p.46, p.133 |
| Transpose | −12…0…+12 semitones | OM p.61, p.133 |
| Master tune | 414.8–440.0–466.8 Hz in steps of about 0.2 Hz | OM p.133, RM p.42 |
| Fingering types | Single Finger, Fingered, Fingered On Bass, Multi Finger, AI Fingered, Full Keyboard, AI Full Keyboard | OM p.132 |
| Style formats | SFF and SFF GE (Guitar Edition, with improved guitar transposition). SFF files load and play, but the Genos saves them as SFF GE | OM p.10, p.45 |
| Style file size limit | Styles of about 120 KB or more cannot be selected | OM p.130 |
| Polyphony (Style stall note) | 256 AWM notes shared by keyboard, Style, Song and pads. Past that, the least important notes (soft or decaying) are cut first | OM p.130 |
| Chord Looper | 8 memories per bank (.clb bank file, .cld single-memory file) | RM p.17 |
| Playlist | up to 2,500 records per playlist file | OM p.17, p.133 |
| Touch response curves | Normal, Easy1, Easy2, Soft1, Soft2, Hard1, Hard2 | OM p.132, RM p.40 |
| Song channel convention | ch1–4 keyboard (R1, Left, R2, R3), ch5–8 Multi Pad, ch9–16 Style | RM p.76, OM p.94 |

---

## 1. Style playback and transport

### ACMP (Auto Accompaniment) on/off `[transport]` `[chord-following]`
- **Spec:** With ACMP off, the Style plays only its rhythm channels. With ACMP on, the chord section (left of Split Point (Style)) is chord-detected and all accompaniment channels follow the chord. The detected chord is shown in the Style area of the Home display.
- **Related behaviour:**
  - Chord Looper Rec forces ACMP on.
  - During Chord Looper playback, the ACMP lamp flashes and keyboard chord input is disabled.
  - OTS recall turns ACMP on.
- **Ref:** OM p.44, p.47, p.66, p.68; RM p.14–15

### START/STOP `[transport]`
- **Spec:** Starts the Style. If ACMP is off it plays the rhythm only; if ACMP is on, the accompaniment follows chords. Pressing again stops. Stopping the Style also stops Multi Pads that are playing.
- **Edge case:** If the Style's rhythm channels contain no data, START/STOP does nothing audible. The user has to turn ACMP on and play a chord.
- **Ref:** OM p.66, p.74, p.130

### SYNC START `[transport]`
- **Spec:** Puts the Style in standby.
  - ACMP off: the first key pressed anywhere starts playback.
  - ACMP on: the first chord in the chord section starts playback.
  - Pressing SYNC START while the Style is playing stops it and re-arms standby.
- **Ref:** OM p.66

### SYNC STOP `[transport]` `[chord-following]`
- **Spec:** Playback runs only while keys are held in the chord section. Releasing all chord-section keys stops the Style, and playing again restarts it. Requires ACMP on.
- **Restriction:** Cannot be turned on when the fingering type is Full Keyboard or AI Full Keyboard.
- **See also:** Synchro Stop Window (§7).
- **Ref:** OM p.66; RM p.12

### INTRO I–III `[sections]`
- **Spec:** Select an intro before starting. When the intro finishes, playback moves to the Main section automatically.
- **Format note:** The MIDI Section Control message defines INTRO 4 and ENDING 4, so SFF can hold four of each. Genos exposes only three.
- **Ref:** OM p.66; DL p.111

### MAIN VARIATION A–D `[sections]`
- **Spec:** Selects one of four looping main patterns.
  - Pressing the Main that is already selected (red lamp) plays that Main's own fill-in, then returns to the same Main.
  - While a fill plays, the Main button flashes red.
- **Ref:** OM p.67

### AUTO FILL IN `[sections]`
- **Spec:** When on, pressing any Main A–D during playback plays a fill-in first, then enters the chosen Main (next or same).
- **Side effect:** Forces "Next Bar" section-change timing even when "Immediate" is set.
- **Storage:** Auto Fill In is a **System** setting. It is not stored in Registration (DL parameter chart).
- **Ref:** OM p.67; RM p.12; DL p.82

### BREAK `[sections]`
- **Spec:** Plays a one-measure break pattern, then returns to the Main section automatically.
- **Ref:** OM p.68

### ENDING/rit. I–III `[sections]` `[transport]`
- **Spec:** Pressed during playback, plays the ending and then stops the Style. Pressing the same Ending button again while it plays adds a ritardando (the tempo gradually slows).
- **Timing exception:** Changes into Ending I always follow the "conventional rules" whatever the Intro/Ending section-change setting. The rules themselves are not described.
- **Ref:** OM p.66; RM p.12

### Section lamp states `[ui]` `[sections]`

| Lamp | Meaning |
|---|---|
| Red | Section selected |
| Red, flashing | Queued next. Also shown on Main buttons during a fill |
| Blue | Section has data but is not selected |
| Off | Section is empty and cannot be selected |

- **Ref:** OM p.68

### Fill Up / Fill Down / Fill Self / Fill Break (assignable) `[sections]`
- **Spec:** These can be assigned to pedals, Assignable buttons or external MIDI:
  - **Fill Down:** plays a fill, then moves to the Main on the left.
  - **Fill Up:** plays a fill, then moves to the Main on the right.
  - **Fill Self:** plays a fill.
  - **Fill Break:** plays the Break.
- **Ref:** RM p.142, p.157

### Half Bar Fill In (assignable) `[sections]`
- **Spec:** While on, a section change made on beat 1 starts the next section from the middle of the bar, with an automatic fill. Does not work with Audio Styles.
- **Control type:** Toggle / Hold A / Hold B.
- **Ref:** RM p.142

### Fade In/Out `[transport]`
- **Spec:** Arm it while stopped, then START: the Style (or MIDI Song) fades in. Press it during playback: the Style fades out.
- **Parameters:**
  - Fade In Time: 0–20.0 s
  - Fade Out Time: 0–20.0 s
  - Fade Out Hold Time (silence held at zero after the fade): 0–5.0 s
- **Default assignment:** ASSIGNABLE [2].
- **Storage:** Times are stored in Registration.
- **Ref:** OM p.67; RM p.142; DL p.88

### Style Section Reset (TAP TEMPO during playback) `[transport]` `[sections]`
- **Spec:** Tapping TAP TEMPO while a Style plays rewinds to the top of the current section, for stutter effects. A setting switches the button to change tempo during playback instead.
- **Settings:** Menu → Metronome → Tap Tempo → Style Section Reset: on/off. Default is on, implied by the OM description.
- **yahaha:** defaults to on, as the Genos (#128, owner: keep the Genos default); with it on a tap while playing only resets (the tempo stays). Off: TAP TEMPO while playing sets the tempo. Section Reset also has its own command and an assignable function (Style Section Reset).
- **Ref:** OM p.46, p.67; RM p.39

### Style Retrigger `[transport]` `[sections]`
- **Spec:** When on, each time a chord is played, the first part of the current Main section repeats at the retrigger length. This gives a stutter or loop-the-head effect.
- **Limits:**
  - Main sections only.
  - Unavailable during MIDI Song playback.
- **Controls (Live Control knob or slider):**
  - RtgRate: 1, 2, 4, 8, 16 or 32 (note-length divisions).
  - RtgOnOff.
  - RtgOff&Rt: fully left is off; turning right turns it on and shortens the length.
- **Storage:** Registration, Freeze group "Style".
- **Ref:** RM p.147, p.75; DL p.82

### Tempo `[transport]`
- **Spec:**
  - TEMPO −/+ opens a tempo pop-up and changes tempo within 5–500; holding a button repeats.
  - Pressing −/+ together restores the default tempo of the last selected Style or Song.
  - Rotating the Data dial on Home also opens the tempo pop-up.
  - The tempo display shows the current tempo, the Style's start tempo and the Song's start tempo.
  - Master Tempo on a knob, slider or foot controller changes the tempo within a range that depends on the Style or Song.
  - With an external MIDI clock, the display shows "EXT." and the panel cannot control tempo or transport.
- **Storage:** Registration (Freeze group "Tempo") and Style data. Not stored in OTS.
- **Ref:** OM p.46; RM p.13, p.144, p.148, p.151; DL p.82

### TAP TEMPO `[transport]`
- **Spec:**
  - Style and Song stopped: tap once per beat for one bar (four taps in 4/4). Style rhythm playback then starts at the tapped tempo.
  - MIDI Song playing: two taps set the tempo.
  - Style playing: resets the section (see Style Section Reset) or changes tempo, depending on that setting.
- **Settings:** Tap sound (a percussion instrument) and tap volume.
- **Ref:** OM p.46; RM p.39

### Style Dynamics Control `[transport]` `[settings]`
- **Spec:** A live control that changes the intensity of Style playback, not just its volume.
- **Settings:** Style Setting → "Dynamics Control" decides whether the Style can be controlled by the "Dynamics Control" Live Control or Assignable function at all.
- **Default controller:** Knob Assign Type 2, knob 3. OM p.69 says it sets Style volume "depending on playing strength", so a velocity-responsive reading is possible.
- **(Not specified):** the exact mapping from control value to Style intensity. Genos1 offered Off/Narrow/Medium/Wide.
- **yahaha (#180, `src/engine/dynamics.rs`):**
  - **Level.** A level from 0 to 127 scales the velocity of every Style note-on, on all eight parts. The factor is ×0.35 at 0, ×1 at 64 (as written) and ×1.6 at 127, with the result clamped to 1–127.
    - Scaling velocity rather than volume changes intensity, as the manual describes. CC7 is never touched.
    - With Dynamics Control off, the Style plays as written.
  - **Touch** (reading OM p.69 literally). Each strike in the chord section sets the level to its velocity minus 36, so a strike at velocity 100 plays the Style as written.
  - **Accent** (a stand-in for the PSR-SX Unison & Accent, §C.9). A chord-section strike at or above the threshold (default 110) makes the Main that is playing play its own fill from the next beat, as Fill Self does.
    - It is not a Main press, so OTS Link does not follow it.
    - It does nothing during an Intro, a fill, a break or an Ending, or while a change is queued.
  - **Pedal.** The assignable function "Dynamics Control" (RM p.142 marks it pedal-assignable) turns a foot controller's position into the level, 0–127. With Dynamics Control off the pedal does nothing. A pedal that is given another function leaves the level where it was.
  - **Storage.** System settings, not Registration (DL p.91: Dynamics Control is System only).
  - **Defaults.** Dynamics Control on, level 64, Touch off, Accent off.
  - **Controls.** The app's Settings › Style page (Dynamics Control, level, Touch, Accent, threshold). TUI and app keys: `H` toggles Accent, `&` toggles Touch. There is no Launchkey mapping: every pad page is full and yahaha does not read the encoders yet.
- **Ref:** OM p.11, p.69; RM p.11, p.142, p.147

### Ambience Depth `[mixer]`
- **Spec:** Wet/dry ratio of Ambient Drums/SFX kits used on the Style's Rhythm1/2 channels. Default: Knob Assign Type 2, knob 1. Only relevant if our synth models ambient kits; otherwise it can be a reverb-send proxy.
- **Ref:** OM p.52, p.69; RM p.147

### Style types `[ui]`
- **Session:** Mixes in original chord types, changes and riffs. May not match simple chords; a plain major triad can come out as a 7th. On-bass chords may give unexpected results.
- **Free Play:** Rubato; no strict tempo.
- **Pro:** Standard styles.
- **+Audio:** Audio Styles (out of scope). The audio part mutes above 160% of the default tempo.
- **Rhythm-only sections:** Some preset styles have sections that contain only rhythm parts.
- **Ref:** OM p.45; RM p.7

### Style channels `[mixer]`
- **Spec:** 8 channels: Rhythm1, Rhythm2, Bass, Chord1, Chord2, Pad, Phrase1, Phrase2, plus "Audio" for Audio Styles. On a MIDI Song they map to ch9–16.
- **Ref:** OM p.92; RM p.10, p.76

---

## 2. Chord Looper `[chord-following]` `[transport]`

- **Record during playback:**
  - Press REC/STOP. It flashes (standby) and recording starts at the next measure.
  - ACMP is forced on.
  - Play chords in time.
  - Press ON/OFF: recording stops, ON/OFF flashes orange, and loop playback starts at the next measure (lamp solid orange).
  - During the loop, ACMP flashes, keyboard chord input is ignored, and the whole keyboard is free for playing.
- **Stop the loop:**
  - Press ON/OFF. The loop stops immediately and the Style returns to normal chord following.
  - ON/OFF then lights blue: data is present but the loop is stopped.
  - Pressing again restarts the loop, at the next measure.
- **Record while stopped:**
  - Press REC/STOP. SYNC START and ACMP turn on automatically.
  - The first chord starts the Style and recording together, so recording begins exactly on beat 1.
  - START/STOP stops both the Style and recording.
  - REC/STOP stops only recording; the Style continues.
- **Memories:**
  - The current sequence can be stored to memories 1–8. The latest recording survives until power-off or until a memory that holds data is selected.
  - Bank save: .clb. Per-memory Export/Import: .cld. Clear. New bank.
  - Memories are auto-named "CLD_001" and so on; rename by export and re-import.
- **Recall:**
  - Select a bank and a memory number, then press ON/OFF before starting to loop from the beginning.
  - Otherwise press ON/OFF just before the target measure: the loop is armed and starts next measure.
  - Changing the memory number during a loop switches at the next measure.
- **Controls:** Panel CHORD LOOPER [REC/STOP] and [ON/OFF]; also assignable to pedals and buttons.
- **Storage:** Registration, Freeze group "Chord Looper".
- **(Not specified):** quantisation of recorded chord timing, maximum sequence length, and whether the loop length rounds to whole bars.
- **yahaha (#29):** see [chord-looper.md](chord-looper.md): chord times snap to 16ths, the loop is whole bars, 128 changes / 64 bars.
- **Ref:** OM p.68–69; RM p.14–19, p.141; DL p.82

---

## 3. Sections and style-change behaviour (Style Setting)

Menu → Style Setting. The manual gives **no factory defaults** for any of these settings.

| Setting | Options | Behaviour | Ref |
|---|---|---|---|
| **Section Change Timing – To Main [A]–[D]** | Immediate / Next Bar | Applies when changing into a Main section and when loading another Style during playback. **Immediate:** switches at the next beat; the new section continues from that beat number. **Next Bar:** switches at once if pressed within the first beat of the bar, otherwise at the next barline. Immediate falls back to Next Bar when Auto Fill In is on, and in Audio Style cases. The option is also recalled by Registration, but it only takes effect when a Style actually loads from that Registration. | RM p.12 |
| **Section Change Timing – Inside Intro/Ending** | Next Bar / End of Section | Applies when switching to another Intro or Ending while an Intro or Ending plays. **Next Bar:** as above. **End of Section:** waits until the current Intro or Ending finishes. Intro→Intro always uses Next Bar; changing to Ending I follows the conventional rules. | RM p.12 |
| **OTS Link Timing** | Immediate / At Main Section Change | With OTS LINK on: OTS n loads the moment Main n is pressed, or at the next measure when the section actually changes. System setting. | OM p.67; RM p.11 |
| **Stop ACMP** | Off / Style / Fixed | See Chord following spec §C.8. Stored in Registration (Freeze "Style"). | RM p.11 |
| **Synchro Stop Window** | Off / time value (values not listed) | With SYNC STOP on: if a chord is held longer than the window, Sync Stop cancels itself and the Style keeps playing after release. A quicker release stops the Style. Stored in Registration. | RM p.12 |
| **Multi Pad Synchro Stop (Style Stop)** | On/Off | Whether repeating pads stop when the Style stops. | RM p.12 |
| **Multi Pad Synchro Stop (Style Ending)** | On/Off | Whether repeating pads stop when an Ending section starts. | RM p.12 |
| **Dynamics Control** | on/off (see §1) | Enables the Style Dynamics live control. | RM p.11 |
| **Display Tempo** | On/Off | Shows each Style's tempo in the Style Selection list. | RM p.11 |
| **Change Behavior – Section Set** | Off / section (A–D etc.) | Section auto-selected when a different Style is chosen **while stopped**. Off keeps the current section. If the target Main is missing, the nearest one is used (for example D missing → C). | RM p.12 |
| **Change Behavior – Tempo** | Lock / Hold / Reset | **Lock:** always keep the old tempo. **Hold:** keep it while playing; take the new Style's tempo when stopped. **Reset:** always take the new Style's default tempo. Assignable toggles "Style Tempo Lock/Reset" and "Style Tempo Hold/Reset" exist. | RM p.12, p.144 |
| **Change Behavior – Part On/Off** | Lock / Hold / Reset | Same three modes applied to the Style channel on/off states. Hold and Reset turn all channels on for a newly loaded Style. | RM p.13 |

- **Storage (DL p.91):**
  - System only (not Registration): Section Set, Tempo, Part On/Off, OTS Link Timing, Display Tempo, Dynamics Control.
  - Stored in Registration, Freeze group "Style": Stop ACMP, Synchro Stop Window, MPad Synchro Stop.
- **Also relevant:**
  - The "Section" (current section selection) is stored in Registration (Freeze "Style").
  - Parameter Lock can protect "Style"-related groups from Registration and OTS recall (§9).

---

## 4. One Touch Setting (OTS) `[registration]`

### OTS 1–4 buttons
- **Spec:** Recalls one of the Style's four panel setups: voices, effects, Multi Pad bank and more. Recall also turns **ACMP and SYNC START on**, so the next left-hand chord starts the Style. An OTS button with no user data keeps the original Style's OTS.
- **Ref:** OM p.47, p.60

### OTS contents
The OTS column of the DL parameter chart marks these as stored in OTS:
- Voices and part on/off for Left and Right.
- Keyboard-part mixer values: filter, EQ, insertion and chorus/reverb depth, pan, volume.
- Insertion effect type and parameters.
- All Voice-Edit "voice set" parameters, including mono/poly, portamento, panel-sustain level, touch sensitivity, part octave, and vibrato/modulation/aftertouch depths.
- Voice Setting Tune: Tuning, Octave, Portamento Time.
- Pitch Bend Range for Left and Right.
- Keyboard Harmony/Arpeggio on/off, type, volume, speed, assign, chord-note-only and minimum velocity.
- Multi Pad bank (file) and Multi Pad volume offset.
- ACMP (on) and Sync Start (on).

**Not** in OTS: tempo, transpose, upper octave, Left Hold, joystick modulation, split points, fingering type.
- **Ref:** DL p.82–92

### OTS memorize (user)
- **Spec:** Press MEMORY, then an OTS button. The OTS is saved into the Style file (a User Style save). The Registration item checkboxes do not affect OTS.
- **Scope note:** Saving is in scope if we write back to .sty. The OTS data lives in the style's OTS chunk; the manuals do not describe the binary format.
- **Ref:** OM p.60

### OTS LINK
- **Spec:** When on, selecting Main A/B/C/D auto-recalls OTS 1/2/3/4. Timing follows the OTS Link Timing setting.
- **Storage:** Assignable "OTS Link On/Off" exists. Imported Music Finder records expect OTS LINK on.
- **Ref:** OM p.67; RM p.119

### OTS +/− (assignable)
- **Spec:** Steps to the next or previous OTS.
- **Ref:** RM p.142

### Style Information
- **Spec:** A window lists the voices assigned to OTS 1–4. Touching a number recalls it.
- **Ref:** OM p.47 `[ui]`

---

## 5. Keyboard parts and voices `[voices]`

### Parts and layering
- **Spec:** Four parts, each switched by PART ON/OFF:
  - Only R1/R2/R3 on: a single voice across the whole keyboard.
  - Two or more Right parts on: a layer.
  - LEFT on with any Right part on: a split at Split Point (Left). The Left part plays F#2 and below; Right parts play above.
  - Right 3 can be split off above Split Point (Right 3).
- **Ref:** OM p.48–50

### Split Points
- **Spec:** Three independent points:
  - **Style:** chord-section upper edge.
  - **Left:** Left voice versus Right voices.
  - **Right 3:** R1/R2 versus R3.
- **Constraints:** Left ≥ Style, and Right3 ≥ Left.
- **"Style + Left" mode:** sets Style and Left to the same key, so the chord section and Left voice section share one area.
- **Setting:** Hold the on-screen label and press a key, or use the L/R controls or the Data dial.
- **Default:** F#2 for Style and Left (Right 3 default not given).
- **Storage:** Registration. Freeze group Style (Style/Left) or Voice (Right3). Parameter Lock group "Split Point".
- **Ref:** OM p.49–50; DL p.87

### LEFT HOLD
- **Spec:** With LEFT on, the Left voice keeps sounding after keys are released. Sustaining voices sustain; decaying voices decay slowly, as if the sustain pedal were down. It is cancelled by stopping the Style or Song, or by turning LEFT HOLD off.
- **Storage:** Registration (Freeze "Style"); not OTS.
- **Ref:** OM p.49; DL p.82

### UPPER OCTAVE −/+
- **Spec:** Shifts Right 1–3 by one octave per press. Pressing −/+ together resets to 0.
- **Range:** Not stated. Per-part Octave (Voice Setting) is ±2 octaves.
- **Storage:** Registration (Freeze "Voice").
- **Ref:** OM p.61; RM p.41; DL p.82

### Per-part Octave / Tuning / Portamento Time (Voice Setting → Tune)
- **Spec:**
  - Octave: ±2 octaves per part.
  - Tuning: per-part pitch.
  - Portamento Time: 0 means no portamento; only used when Portamento is on for the part.
- **Ref:** RM p.41

### Voice Set / Voice Set Filter
- **Spec:** Selecting a voice also loads its default "voice set" parameters: effects, EQ and Keyboard Harmony type, among others. Voice Set Filter can stop parts of this, per part (Left/Right). For example, uncheck Keyboard Harmony/Arpeggio to keep the current harmony type when changing voice. By default, selecting a new Right 1 voice sets the Harmony/Arpeggio type memorised for that voice.
- **Ref:** OM p.56; RM p.41, p.48

### Mono/Poly and note priority
- **Spec:** A part can be monophonic.
- **Mono note priority:**
  - Mono voice on a Right part: the highest note if another enabled Right part is Poly; the latest note if all other enabled Right parts are Mono.
  - Mono voice on Left: always the latest note.
- **Portamento Type (Mono only):**
  - Normal: the next note waits for the previous one to stop.
  - Legato: keeps the previous sound and changes pitch only.
  - Crossfade: smooth crossfade.
- **Other portamento parameters:** Velocity-used-for-crossfade (Latest/First), Portamento Time Type (Fixed Rate / Fixed Time), Fast Playing Portamento (time threshold and alternative time), Min Portamento Time, and Velocity-to-Portamento-Time (sensitivity and reference velocity).
- **Edge case:** A Harmony-category effect treats a Mono, Legato or Crossfade part as off.
- **Ref:** RM p.50–51, p.46

### Panel SUSTAIN button
- **Spec:** Adds sustain to Right 1–3. The level comes from each voice's "Panel Sustain" parameter.
- **Ref:** OM p.71; RM p.49

### Touch Response
- **Spec:** Initial-touch curves: Normal, Easy1, Easy2, Soft1, Soft2, Hard1, Hard2. The curve is a system setting that applies to every part checked in the list. Unchecked parts use Fixed Velocity.
- **Aftertouch curves:** Soft, Medium or Hard, applied per checked part.
- **Voice Touch Sensitivity:** each voice also has Depth (64 = normal, 127 = double, 0 = none) and Offset (64 = normal), plus a Velocity Limit Low/High clamp.
- **Ref:** RM p.40, p.49

### Transpose
- **Spec:** TRANSPOSE −/+ in semitones, −12…+12. Pressing both resets. The display selects the target:
  - **Master:** everything except audio.
  - **Keyboard:** the keyboard pitch **including the chord root sent to the Style**.
  - **Song:** MIDI Song.
- **Exclusions:** Drum/SFX kits are never transposed.
- **"Transpose MIDI Input":** decides whether received MIDI notes are transposed.
- **Storage:** Registration (Freeze "Transpose"). "Transpose Assign" (which target the buttons act on) is System.
- **Ref:** OM p.61; RM p.42, p.151; DL p.82, p.92

### Master Tune `[settings]`
- **Spec:** Default 440.0 Hz, adjustable in 0.2 Hz steps. Does not affect drum or SFX kits. System setting.
- **Ref:** RM p.42

### Scale Tune `[settings]`
Optional for us.
- **Main Scale:**
  - Types: Equal, Pure Major, Pure Minor, Pythagorean, Mean-Tone, Werckmeister, Kirnberger, Arabic1, Arabic2.
  - Settings: Base Note, per-key cents, Bypass, part checkboxes.
  - Stored in Registration.
- **Sub Scale:** A temporary override on checked parts. It is reset when you return to Main or turn the power off.
- **"Scale Tune Quick Setting" pedal:** Hold it, press keys, release. Those keys are set to −50 cents. Pressing and releasing with no keys clears it.
- **Ref:** RM p.43–45, p.144

### Joystick (pitch bend / modulation)
- **Spec:**
  - X axis: pitch bend on all parts. Range 0–12 semitones per part, shared by all pitch-bend controllers.
  - Y axis: modulation (vibrato) on Right 1–3 by default.
  - Self-centring.
  - Pitch bend or modulation may not reach the Left part during Style playback, depending on the Style.
- **Assign Types:** Three joystick types, cycled with ASSIGNABLE [1] by default. They reset at power-off but can be saved to Registration.
- **JOYSTICK HOLD:** freezes the current joystick values. By default it holds only Y (modulation). Which items are held is configurable.
- **Ref:** OM p.64, p.70; RM p.140, p.145, p.147

### Super Articulation / ART 1–3 buttons
- **Spec:** The ART buttons trigger articulation effects on S.Art/S.Art2 voices. A lit blue button means that effect is available.
- **Button styles:**
  - One-shot noises.
  - Hold-to-change (for example guitar harmonics).
  - S.Art2 armed effects: flash red, then fire on the next key on/off.
- **Multiple parts:** With several S.Art parts, one ART press affects all of them.
- **Scope:** Depends on sample content. We can at most map ART buttons to keyswitch notes if our soundfont supports them.
- **Ref:** OM p.71–73; RM p.35

### Organ Flutes
- **Spec:** Live footage levers plus rotary slow/fast, vibrato, response and attack mode (First/Each). Only usable with an organ-modelling synth.
- **Assignables:** Organ Rotary Slow/Fast (pedal/button). Special slider assign types for the footage levers apply only when an Organ Flutes voice is selected; they have no "catch" behaviour.
- **Ref:** OM p.53, p.63; RM p.56–57, p.141

### Ensemble Voices
Optional.
- **Mode:** The keyboard parts become Ensemble Parts 1–4. There is no Left voice, but ACMP still works with the left hand. The chord detection area is forced to Lower.
- **Note assignment:** Rule-based assignment of held notes to four instruments:
  - Unison1/2
  - 4/3/2-Part Divide 1/2 (closed and open voicings)
  - 4/3/2-Part Incremental 1/2
  - Each with or without key-off retrigger.
  - Filters: F*D, A*D, F*A, A*A, THRU. Assign: HI, LO, EA (earliest), LA (latest).
- **Humanize:** timing, pitch range, attack pitch.
- **Ref:** OM p.54–55; RM p.58–63

---

## 6. Keyboard Harmony / Arpeggio `[voices]`

**Scope recommendation:**
- **Keyboard Harmony, Echo, Tremolo and Trill: in scope.** They are chord-driven, algorithmic and playable.
- **Arpeggio: in scope only as an engine.** Yamaha's arpeggio pattern data is internal and copyrighted. We would need our own patterns, so treat the preset Arpeggio list as out of scope.

### HARMONY/ARPEGGIO button
- **Spec:** Turns the selected Harmony, Echo or Arpeggio type on or off for the Right-hand parts.
- **Ref:** OM p.56

### Harmony category and chord source
The harmony chord source and chord-section layout depend on ACMP and LEFT:

| ACMP | LEFT | Chord source |
|---|---|---|
| on | off | Chord section left of Split Point (Style), shared with the Style |
| off | on | LEFT part section left of Split Point (Left) |
| on | on | The chord section (left of Split Point (Style)). The LEFT voice sits between Style and Left split points |

- **Exception:** Types "1+5" and "Octave" ignore the chord.
- **Ref:** OM p.56–57

### Harmony types (DL p.74)
- **Harmony:** Standard Duet 1, Standard Duet 2, Standard Trio, Full Chord, Rock Duet, Country Duet 1, Country Duet 2, Country Trio, Block, 4-Way Close 1–4, 4-Way Open 1–3, 1+5, Octave, Strum, Multi Assign.
- **Echo:** Echo, Tremolo, Trill.
- **Multi Assign:** Spreads the notes of a right-hand chord across R1/R2/R3 in the order pressed. Independent of ACMP and LEFT.
- **Echo category:** Echo, Tremolo and Trill repeat in time with the tempo, independent of ACMP and LEFT. Trill needs two held notes; with more than two held, it uses the last two.
- **(Not specified):** the voicing algorithm of each harmony type.
- **Ref:** OM p.57

### Detail settings
Items marked * also apply to Arpeggio. Multi Assign has none of these settings.

| Setting | Behaviour |
|---|---|
| **Volume*** | Level of the generated notes. No effect on voices with touch sensitivity 0 |
| **Speed** | Echo category only |
| **Assign*** | **Auto:** the Right parts that are on, prioritised R1, R2, R3. **Multi:** the played note on R1 and harmony notes spread over the other Right parts. **Right1/2/3:** a fixed part |
| **Chord Note Only** | Harmony category only. Harmonise only melody notes that belong to the current chord |
| **Minimum Velocity** | Harmony, echo and similar notes sound only when the key velocity is above the threshold (accent harmony) |

- **Storage:** OTS, Registration, Voice Set.
- **Ref:** RM p.46–47; DL p.87

### Arpeggio
- **Spec:** Held notes trigger a pattern that depends on the notes played.
- **Categories:** Up&Down, SynthSeq, ChordSeq, Trance, Electro, Filter&Gate, Guitar, Keyboard, and others.
- **Settings:**
  - Arpeggio Quantize: syncs to Style or Song timing (System).
  - Arpeggio Hold: keeps playing after release until the button is pressed again (System). Also available as a pedal.
  - Live Controls: ArpVel, ArpGateT and ArpUnitM (percent of the pattern default), HrmArpVol.
- **Ref:** OM p.57–58; RM p.41, p.141, p.147; DL p.74

---

## 7. Multi Pads `[multipad]`

- **Selection:** MULTI PAD [SELECT] opens the bank list. A bank holds 4 pads.
- **Playback:**
  - Pads 1–4 play their phrase at the current tempo; up to 4 at once.
  - Pressing a playing pad restarts it from the top.
  - Pads are either one-shot or loop (Repeat).
  - While a Style or MIDI Song plays, a pad press starts at the **top of the next measure**. When everything is stopped it starts immediately.
  - Repeat pads started during playback loop in sync with the beat.
- **Stopping:**
  - [STOP] stops all pads.
  - Holding STOP and pressing a pad stops only that pad.
  - Style START/STOP (while playing) also stops pads.
  - Song PLAY/STOP stops the Song, the Style and the pads.
- **Lamps:**

  | Lamp | Meaning |
  |---|---|
  | Blue | Pad has data |
  | Red | Playing |
  | Red, flashing | Synchro Start standby |
  | Off | Empty |

- **Chord Match:**
  - When ACMP or LEFT is on, pads with Chord Match on transpose their phrase to the current chord. The chord comes from the ACMP chord section, or from the LEFT section when ACMP is off.
  - The chord can be played before or after pressing the pad.
  - Some pads have Chord Match off.
  - Phrases are authored on a CM7 basis using C, E, G, A, B (avoiding F and D), like Style source patterns. That strongly implies NTR/NTT-style conversion from a CM7 source; the exact rule is not given.
- **Synchro Start:**
  - Hold SELECT and press pad(s) to arm them (flashing red).
  - A key press (ACMP off), a chord in the chord section (ACMP on), or a Style start triggers all armed pads.
  - If armed during playback, the trigger starts them at the next measure.
  - Repeating the arming gesture, or pressing STOP, disarms.
- **Synchro Stop options:** Style Stop, Style Ending, and MIDI Song stop (§3 and Song Setting).
- **Per-pad parameters:** Repeat on/off and Chord Match on/off are stored in the pad bank (Multi Pad column in DL).
- **Audio Link pads (WAV 44.1 kHz/16-bit):**
  - No repeat and no Chord Match.
  - A per-pad Audio Level.
  - "Simultaneous Play" On/Off: Off means a new pad stops the previous one.
  - Audio and MIDI pads cannot be mixed in one bank.
- **Storage:** The Multi Pad bank file is stored in OTS and Registration (Freeze "Multi Pad"). Mixer offsets for Multi Pads live in the Panel mixer.
- **(Not specified):** the .pad file internal format. The Data List has nothing on it.
- **Ref:** OM p.59, p.74–75; RM p.64–68, p.12, p.78; DL p.82, p.91

---

## 8. Mixer and channel control `[mixer]`

- **Tabs:**
  - Panel: Style (whole), Multi Pad (whole), Left, R1–3, Song A/B, Mic, Aux, and so on.
  - Style: 8 channels.
  - M.Pad: 4 Audio Link pads.
  - Song: 16 channels.
  - Master: Compressor and EQ.
  - Ref: OM p.90; RM p.129
- **Per-part parameters:**
  - Filter: Cutoff, Resonance.
  - EQ: High and Low gain per part. The Master EQ is 8-band.
  - Effect: insertion type and depth; Variation as System or Insertion.
  - Chorus and Reverb depth.
  - Pan and Volume.
  - Touch-and-hold resets to the default.
  - Ref: RM p.129–135
- **Style channel on/off and solo:** Menu → Channel On/Off or Mixer. Touch-and-hold a channel to solo it (purple); touch again to cancel. Channel on/off is stored in the Style file and in Registration.
- **Changing a style channel's voice:** Tap the instrument icon. Changing a drum kit resets drum-detail settings; reselecting the Style restores them. The Audio part voice cannot change.
- **Saving:** Style-mixer edits are saved into the Style file (via Style Creator Save). Panel mixer settings go to Registration.
- **Refs for the above:** OM p.91–93, p.131; RM p.10
- **Insertion effect slots:**
  - 1–19: keyboard parts and Song channels.
  - 20: Mic.
  - 21–28: Style parts.
  - Ref: RM p.133
- **Live Control volume functions:**
  - Volume (absolute).
  - VolRatio (0–100–200 %).
  - KbdVol (all keyboard parts).
  - Balance (A vs B groups).
  - RatioBal.
  - The Slider "Balance" assign type is fixed to part-volume balance.
  - Ref: RM p.145–146
- **Style Track Mute A/B knob functions:**
  - **A:** fully left leaves only Rhythm2 on. Turning up adds Rhythm1, Bass, Chord1, Chord2, Pad, Phrase1, Phrase2.
  - **B:** fully left leaves only Chord1 on. Turning up adds Chord2, Pad, Bass, Phrase1, Phrase2, Rhythm1, Rhythm2.
  - Ref: RM p.148

---

## 9. Registration Memory, Freeze, Sequence, Playlist `[registration]`

### Registration Memory 1–10
- **Memorize:** Press MEMORY and tick the item groups to store, then press a number button. This overwrites any previous data on that button.
- **Lamps:** red = stored and selected; blue = stored, not selected; off = empty.
- **Recall:** Press a lit button.
- **Bank file:** All ten buttons are saved as one Registration Bank file.
- **Bank select:** REGIST BANK −/+ steps through banks; pressing both opens the bank list.
- **Clear at power-on:** Powering on while holding F#6 clears all ten.
- **Factory Reset → Registration:** Deselects the bank (all lamps off) but keeps the files.
- **Items stored:** See the DL "Regist" column. Broadly: voices, parts, mixer, Style, section, tempo, transpose, split, fingering, Harmony/Arpeggio, Multi Pad, Chord Looper, Live Control and Assignable setups, pedal functions, scale tune, Song, and Vocal Harmony.
- **Not stored:** Auto Fill In, Style change-behaviour settings, OTS Link Timing, touch curve, Master Tune, arpeggio quantize/hold.
- **Ref:** OM p.96–99; RM p.165; DL p.82–93

### Regist Bank Info / Edit `[ui]`
- **Spec:** Info shows the voices and Style per button; touching an entry loads it. Edit renames or deletes individual memories.
- **Ref:** OM p.99

### Registration Freeze (FREEZE button)
- **Spec:** Checked groups are not changed by Registration recall.
- **Groups (from the DL "Freeze Group" column):** Style, Voice, Keyboard Harmony/Arpeggio, Multi Pad, Tempo, Transpose, Scale Tune, Chord Looper, Live Control, Foot Pedals, Assignable Buttons, Line Out, MIDI Song, Audio Song, Text, Vocal Harmony/Mic Setting.
- **Group membership notes:**
  - The **Left** voice and Left-part settings belong to the "Style" freeze group.
  - The **Right** voices belong to "Voice".
- **Assignable/MIDI:** "Registration Freeze On/Off" can be assigned.
- **Storage:** The checkbox setting is System.
- **Ref:** RM p.113; DL p.82–91

### Registration Sequence
- **Spec:** A programmed order of the ten memories. It is stepped with pedals ("Regist +" / "Regist −"), Assignable "Registration Sequence +/−", or DEC/INC on the Home screen.
- **End action:**
  - Stop: further advances do nothing.
  - Top: loop to the start.
  - Next: go to the first memory of the next bank file in the same folder.
- **Editing:** Replace, Insert, Delete, Clear.
- **Storage:** Saved in the bank file (one sequence per bank). The Home screen shows the sequence when it is on.
- **Pedal priority:** Voice Guide > Punch In/Out > Registration Sequence > Assignable function.
- **Ref:** RM p.114–115; OM p.31

### Regist Bank search and tags `[ui]`
Optional.
- **Spec:** Search by name, with filters for tag, Song, Style name and Style tempo range.
- **Ref:** RM p.116–117

### Playlist
- **Records:** Each record links to a Registration Bank file. Its optional Action can load a specific Registration number and/or switch to a display view.
- **Adding records:** search a bank, select a bank, copy from another playlist, or Append a whole playlist.
- **List management:** Up/Down reorder, Delete, and sort (normal, A→Z, Z→A). Reorder and delete are disabled while sorted. Save keeps the displayed order.
- **Files:** Playlist files hold up to 2,500 records.
- **Music Finder import:** .mfd import converts records into Registration banks on memory [1] with keyword/genre tags. Use it with OTS LINK on.
- **Ref:** OM p.96, p.100–103; RM p.118–119

### Parameter Lock `[settings]`
- **Spec:** Locked groups can only change by direct panel operation, never through Registration, OTS, Playlist or Song data.
- **Groups seen in the DL chart:** Split Point, Fingering Type, Master EQ, Reverb Type, Reverb/Chorus/Variation return levels, Vocal Harmony/Mic Setting. The RM also mentions effects.
- **Ref:** RM p.163; DL p.82–92

---

## 10. Controllers `[settings]`

### Foot pedals
- **Hardware:** Three jacks. Defaults: 1 = Sustain, 2 = ART.1, 3 = Volume (FC7 expression pedal).
- **Polarity:** Can be reversed. If a pedal is held at power-on, its polarity inverts.
- **Ref:** OM p.114, p.131; RM p.139

### Assignable buttons
- **Spec:** A–F and 1–3.
- **Defaults:** [1] = Joystick Assign; [2] = Fade In/Out.
- **Pop-up:** An optional pop-up window shows the assigned function's state.
- **Ref:** OM p.64, p.67; RM p.138–139

### Pedal Control Type and Range
- **Control Type:** Toggle, Hold A (on while held), Hold B (off while held).
- **Range (continuous functions):** Full, Upper (centre–max) or Lower (centre–min).
- **Ref:** RM p.139

### Assignable functions relevant to live play
| Category | Functions |
|---|---|
| Voice | Articulation 1–3, Volume\* (foot controller), Sustain (release dampens), Panel Sustain On/Off, Sostenuto (not organ or some S.Art), Soft, Glide (pitch dip with on/off speed and bend range), Mono/Poly, Portamento (per-part on/off), Portamento Time\*, Vel.Sens for Portamento Time\*, Pitch Bend\*, Modulation(+/−)\*, Modulation Alt (toggle), Initial Touch On/Off, Left Hold, Pedal Wah\*, Organ Rotary Slow/Fast, Kbd Harmony/Arp On/Off, Arpeggio Hold |
| Registration | Memory, Regist 1–10, Regist Sequence +/−, Bank +/−, Freeze On/Off, Sequence On/Off |
| Live Control | Knob Assign, Slider Assign, Joystick Assign, Joystick Hold, Reset Value |
| Chord Looper | On/Off, Rec/Stop |
| Style | Dynamics Control\*, Start/Stop, Sync Start, Sync Stop, Intro 1–3, Main A–D, Fill Down/Self/Break/Up, Ending 1–3, ACMP, OTS Link, Auto Fill In, Half Bar Fill In, Fade In/Out, **Fingered ⇄ Fingered On Bass**, **Bass Hold**, OTS 1–4, OTS +/− |
| Multi Pad | Pad 1–4, Select (sync start), Stop |
| Overall | Part On/Off (several parts at once), Insertion Effect On/Off, Metronome On/Off, Tempo +/−, Reset/Tap Tempo, Master Tempo\*, Style Tempo Lock/Reset, Style Tempo Hold/Reset, Transpose +/−, Upper Octave +/−, Scale Tune Quick Setting, Scale Tune Bypass, Percussion (pedal plays a chosen drum sound) |

\* marks functions that need a continuous foot controller. Ref: RM p.139–144

### LIVE CONTROL knobs and sliders
- **Hardware:** 6 knobs with 3 Knob Assign Types. 9 sliders with "Balance" plus 2 Slider Assign Types.
- **Switching:** KNOB ASSIGN and SLIDER ASSIGN cycle the types. The selected types reset at power-off.
- **Slider "catch":** A slider does nothing until its position crosses the current value. A value set elsewhere outside the slider's range cannot be reached with the slider.
- **LEDs:** show the current values.
- **Functions:**
  - **Mixer:** Volume, VolRatio, KbdVol, Balance, RatioBal, Pan, Reverb, Chorus, Rev&Cho, InsEffect, EQ High/Low, Cutoff, Resonance, Cut&Reso, Filter.
  - **Voice:** Attack, Decay, Release, Atk&Dec, Atk&Rel, Mod±, Tuning, Octave, PitchBend, PBRange, PortaTime, VelPTSens, FMDetune, FMSpread.
  - **Harmony/Arpeggio:** HrmArpVol, ArpVel, ArpGateT, ArpUnitM.
  - **Style:** AmbiDepth, DynCtrl, RtgRate, RtgOnOff, RtgOff&Rt, StyMuteA, StyMuteB.
  - **Overall:** Tempo (Master Tempo).
- **Reset Value:** restores all assigned values to their defaults.
- **Storage:** Registration (Freeze "Live Control").
- **Ref:** OM p.62–63, p.69; RM p.145–148

### External MIDI control
- **External Controller:** Maps CC#7/1/2/3/4 to continuous functions per part. Note numbers (or the equivalent CC 0–63 = off, 64–127 = on) trigger on/off functions: all transport and section buttons, Fingered/On Bass toggle, Bass Hold, OTS, Regist, Transpose, Multi Pad, part on/off, and so on.
- **On Bass Note channels:** notes on these channels set the Style bass note. Works regardless of ACMP or split.
- **Chord Detect channels:** notes on these channels are chord-detected using the current fingering type. Works regardless of ACMP or split; data from several channels is merged.
- **Chord SysEx:** can be transmitted and received.
- **Templates:** MIDI Pedal1 plays the chord root from a MIDI pedalboard; MIDI Pedal2 plays the Style bass part; MIDI Accordion 1–4. Useful models for yahaha's MIDI input.
- **Ref:** RM p.150–158

---

## 11. Metronome, UI and other performer settings

| Feature | Spec | Ref | Tag |
|---|---|---|---|
| Metronome | On/Off, Volume, Bell on beat 1, Time Signature. On/off also assignable | RM p.39; OM p.41 | `[transport]` |
| Home display | Voice area (L/R1–3 names and on/off), Style area (Style name, section position, **current chord name** when ACMP is on, time signature), Multi Pad bank, bar/beat/tempo, upper octave and transpose, Registration bank and number, Registration Sequence | OM p.30–31 | `[ui]` |
| Chord Tutor | Pick a root and type; it shows the Fingered-mode notes, which may omit some notes | RM p.7 | `[ui]` `[chord-following]` |
| Panel Reset | Resets voice, Style and MIDI panel state without power-cycling | OM p.36 | `[settings]` |
| Panel Lock | Locks the panel with a 4-digit PIN | OM p.41 | `[ui]` |
| Pop-up display time | Time before TEMPO, TRANSPOSE and OCTAVE pop-ups close, or Hold | RM p.163 | `[ui]` |
| Dial operation | Select (load on dial) or Move Cursor Only (ENTER loads) | RM p.163 | `[ui]` |
| Direct Access | DIRECT ACCESS plus a control jumps to that control's settings page | OM p.36, p.126 | `[ui]` |
| Home Shortcuts | 6 user-assignable shortcut icons | OM p.104 | `[ui]` |

---

## C. Chord following spec (consolidated)

### C.1 Chord Detection Area and split points
- **Chord Detection Area = Lower (normal):** The chord section is the keys at or below Split Point (Style). Chords there drive the Style when ACMP is on.
  - With ACMP off and LEFT on, the Left section (≤ Split Point (Left)) still yields a chord for Keyboard Harmony and Multi Pad Chord Match.
  - Fingered is described as working "when ACMP is on or the Left part is on".
  - Ref: OM p.44, p.49, p.56–57; RM p.9, p.65
- **Chord Detection Area = Upper:**
  - The chord section becomes the right-hand side, above Split Point (Left).
  - The left hand plays a bass line on the LEFT voice.
  - The fingering type is forced to **Fingered\***: the same as Fingered but with **no 1+5, no 1+8 and no Chord Cancel**.
  - **Manual Bass** becomes available and defaults to On. It mutes the Style's Bass channel and moves that channel's voice onto the Left part.
  - Selecting an Ensemble Voice forces the area back to Lower.
  - Ref: OM p.51; RM p.9
- **Split constraints:** Left ≥ Style ≥ lowest key; Right3 ≥ Left. The default Style and Left split is F#2. Ref: OM p.49–50
- **Full Keyboard / AI Full Keyboard:** Chords are detected across the whole keyboard, so split points do not limit detection. Ref: RM p.9
- **Keyboard transpose:** Keyboard (and Master) transpose shifts the chord root sent to the Style. Ref: OM p.61; RM p.42
- **MIDI input:** Notes on "Chord Detect" channels are chord-detected with the current fingering type. Notes on "On Bass Note" channels set the bass. Both ignore ACMP and split. Ref: RM p.154

### C.2 Fingering types (RM p.9, OM p.46)

| Type | Behaviour |
|---|---|
| **Single Finger** | Major = root only. Minor = root + any **black** key to its left. 7th = root + any **white** key to its left. m7 = root + a white **and** a black key to its left. Only M, m, 7 and m7 are available |
| **Multi Finger** | Detects Single Finger or Fingered shapes automatically, without switching modes. The disambiguation rule is not specified |
| **Fingered** | Play the full chord (the Data List table) in the chord section. **Bass = chord root** always |
| **Fingered On Bass** | Same shapes as Fingered, but the **lowest note played** in the chord section becomes the bass note (slash chords). A pedal or button can toggle Fingered ⇄ Fingered On Bass |
| **Full Keyboard** | Detection over the whole range, similar to Fingered, even when notes are split between hands (left bass + right chord, or left chord + right melody) |
| **AI Fingered** | Like Fingered, but fewer than 3 notes can indicate a chord, **inferred from the previously played chord and so on**. The inference rule is not specified |
| **AI Full Keyboard** | Like Full Keyboard with the same fewer-than-3-note inference. **9th, 11th and 13th chords cannot be played** |
| **Fingered\*** | Only in Upper detection mode. Fingered without 1+5, 1+8 or Cancel |

### C.3 Chord Cancel, 1+5, 1+8
- **Chord Cancel:**
  - Fingering: root + ♭2 + 2 (for example C–D♭–D).
  - Effect: sets a no-chord state.
  - Available only in **Fingered, Fingered On Bass and AI Fingered**; not in Fingered\*.
  - The MIDI chord type is 34 ("cc"), and the display reads "Cancel".
  - (Not specified): what each Style channel does under Cancel. **Our rule (#4):** Cancel is the same state as before any chord: rhythm channels and channels with the CASM autostart bit keep playing as recorded; every other part is released at once and rests until the next chord. Cancel does not trigger Sync Start. In the corpus only rhythm channels carry the autostart bit, so in practice this is "rhythm only". An Ending pressed under Cancel therefore plays rhythm only. A chord that ends the no-chord state up to 40 ms after the beat still brings in the downbeat notes the resting parts skipped (the same late-chord allowance as any other chord change).
  - Ref: OM p.46; DL p.45, p.111
- **1+5:**
  - Root plus fifth (for example C+G). A power chord with no third.
  - MIDI chord type 31.
  - Style tables handle it through the "C1+5" playable-note set.
  - **Our rule (#4):** CASM chord-mute bit 31 decides which channels play (corpus styles use it to route 1+5 to the major-family source channel). Parts that follow the chord play only what major and minor share: root, 5th and the 2nd/4th. The 3rd moves to the 5th, the 6th to the 5th, the 7th to the octave, and chromatic passing notes snap to the scale degree below. Chord-table parts keep root and 5th only.
  - Ref: DL p.45; RM p.29
- **1+8:**
  - Root plus its octave (for example C+C). A unison or octave chord.
  - MIDI chord type 30.
  - **Our rule (#4):** CASM chord-mute bit 30 decides which channels play. Parts that follow the chord play only the root (in octaves), including chromatic notes and every Guitar string. NTT Bypass parts still play as written.
  - Ref: DL p.45; RM p.29
- **Keyboard Harmony note:** Harmony types "1+5" and "Octave" are harmony types, not chord types. They ignore the detected chord. Ref: OM p.56

### C.4 Recognition edge cases stated or implied in the manuals
1. **Omissible notes:** Notes in parentheses in the chord table may be left out and the chord is still recognised. For example, C E B is recognised as CM7 without G. Ref: DL p.45
2. **Chord Tutor:** It shows Fingered shapes only and omits some notes for some chords. Ref: RM p.7
3. **Session Styles:** They may re-harmonise simple chords, for example a major triad played as a 7th. On-bass chords can give unexpected results. Ref: OM p.45
4. **Sync Stop:** Not available with Full or AI Full Keyboard. Ref: OM p.66
5. **Bass Hold:** Does not work with AI Full Keyboard. Ref: RM p.142
6. **Stop Accompaniment:** Chords are still recognised and displayed while the Style is stopped (§C.8).
7. **Chord Looper playback:** Keyboard chord input is ignored while the loop plays (ACMP lamp flashes). Ref: RM p.15
8. **Style Retrigger:** Re-triggers the Main section head on each chord change. Not available during Song playback (§C.7).
9. **Vocal Harmony troubleshooting hint:** It suggests setting Stop ACMP to something other than Off when chords are "not detected". This implies that with Stop ACMP = Off, a stopped Style may not publish its chord to consumers. Ref: OM p.131
10. **Pitch bend and modulation:** These may not reach the Left part during Style playback, depending on the Style. Ref: OM p.70
- **Not specified anywhere:**
  - Inversions: the Fingered table is given in root position, and there is no inversion rule except that Fingered On Bass uses the lowest note as bass.
  - Priority between ambiguous sets such as C6 = Am7, Cdim7 (symmetrical), Caug (symmetrical), and Cm6 vs Am7♭5.
  - Handling of octave-doubled notes.
  - What happens with 2-note inputs in plain Fingered (other than 1+5 and 1+8).
  - How long a partial chord change is debounced.
  - We must define these rules ourselves.

### C.4a Recognition rules we chose (Fingered / Fingered On Bass, `src/theory.rs`)
These fill the gaps above. Each one is a decision the owner may overrule after playtesting.
1. **Shapes:** A chord is any pitch-class set that contains every required note of a §D row and nothing outside that row. Notes in parentheses may be left out, one or all of them. Octave doublings and voicing order do not matter.
2. **Fewer than three notes:** Only 1+5 and 1+8 are chords. A single key and any other two-note set (C E, C E♭, C B♭) are not recognised, and the previous chord stays. "Fewer than three notes" is what sets AI Fingered apart (RM p.9).
   Consequence: a single key or a two-note set other than 1+5/1+8 does not start the style under Sync Start, because Sync Start fires on the first recognised chord (OM p.46).
3. **1+8:** Two or more keys that all share one pitch class. 1+5 accepts the fifth either way up: G C is C1+5, with bass G in On Bass.
4. **Ambiguous sets** (C6 = Am7, Cm6 = Am7♭5, C6(9) = Am7(11), Csus4 = Fsus2, C7♭5 = F♯7♭5, dim7, aug) are decided in this order:
   1. The reading whose root is the lowest note wins. C E G A is C6 and A C E G is Am7. dim7 and aug take the lowest note as root.
   2. Fewest omitted notes. For D E G A C, C6(9)/D is complete but Am7(11) would be missing its 9th.
   3. The lowest note's role in the chord: root, then 5th, then 3rd, then ♭5/♯5/4th, then 6th/7th, then tensions. E G A C is Am7/E (E is the 5th of Am7 but the 3rd of C6). G A C E is C6/G. E♭ G A C is Cm6/E♭. G C F is Csus4/G.
   4. Data List table order.
   Readings to playtest: E G A C gives Am7/E (some players would expect C6/E), and C D G B♭ gives B♭6(9)/C.
5. **Inversions:** Fingered On Bass reports the lowest note as bass whenever it is not the root. Plain Fingered reads the same chord with the bass dropped.
6. **Bass outside the chord (On Bass only):** If the whole set is not a chord, but the notes above the lowest one form a chord of three or more notes, the result is that chord over the bass: F♯ C E G is C/F♯. The upper chord must be a complete three- or four-note chord (no omitted notes), so a tension-laden set never becomes an unrelated root over the bass: C E G B F and C E G B♭ C♯ E♭ are not chords (not G13/C or E♭7♭9/C), and the previous chord stays. Plain Fingered does not recognise such a set. A complete table reading always comes first, so D C E G is Cadd9/D and not C/D.
7. **Chords without a MIDI code:** These are shown as themselves, but the style follows them as a CASM type (chord mute bit, NTT tables). The rule is the smallest CASM type that holds every played note, or, if none does, the largest CASM type made only of played notes. The mapped type never drops a played note when a superset exists, but it can add one: M7♭5 → M7(♯11) adds the natural 5th a semitone above the ♭5, and (♭5) → 7♭5 adds a ♭7. Only mM7♭5, which has no superset, drops a played note (its M7).
   - M7♭5 → **M7(♯11)** (type 3). The ♭5 is the ♯11.
   - (♭5) → **7♭5** (type 21).
   - mM7♭5 → **dim** (type 17). No CASM type contains all four notes.

### C.5 Note conversion: how Style channels follow the chord
This is our spec from the Style Creator "SFF Edit" pages. RM p.28–31. All these values are per channel and per section, stored in the Style file's CASM/SFF data.

**Conversion pipeline:** Source Pattern (recorded in Source Root/Chord) → NTR and NTT convert it for the played chord → High Key and Note Limit fold the octave → RTR controls what happens to sounding notes on a chord change → sounded notes.

**Source Root / Source Chord (Play Root/Chord)**
- The key and chord the pattern was recorded in. Default is **CM7** (root C, type M7).
- Playing exactly that chord reproduces the pattern unchanged.
- **Recording guidance for Main and Fill sections:** melody notes use C, E, G, A, B (root, 3, 5, 6/13, maj7). Avoid F (4th) and D (9th, because it clashes with ♭9/♯9 chords). Chord and Pad channels use chord tones only (C E G B).
- **Intros and Endings** are assumed not to change chord during playback. They may contain their own progressions: intros lead to the tonic, endings resolve to the key.
- **Pass-through case:** When a channel is set to NTR = Root Fixed, NTT = Bypass and NTT Bass = Off, the fields become "Play Root/Play Chord": the channel just plays as written.
- **Guitar exception:** Source Root/Chord is ignored when NTR = Guitar.
- **Playable-note tables:** RM p.29 has a table of chord tones (C) and recommended notes (R) per source chord type. Its source-type list is: Maj, 6, M7, M7♯11, add9, M7(9), 6(9), aug, m, m6, m7, m7♭5, m add9, m7(9), m7(11), mM7, mM7(9), dim, dim7, 7, 7sus4, 7♭5, 7(9), 7♯11, 7(13), 7(♭9), 7(♭13), 7(♯9), M7aug, 7aug, 1+8, 1+5, sus4, sus2. The chart graphics did not survive text conversion.

**NTR (Note Transposition Rule): how the root change is applied**

| NTR | Behaviour |
|---|---|
| **Root Trans** | Transposes the whole pattern by the root interval and keeps the intervals. Example: C3 E3 G3 over F becomes F3 A3 C4. For melodic parts |
| **Root Fixed** | Keeps each note as close as possible to its original pitch. Example: C3 E3 G3 over F becomes C3 F3 A3. For chordal parts |
| **Guitar** | Guitar-specific. Maps to realistic guitar-fingered voicings |

**NTT Type (Note Transposition Table): how the chord-type change is applied**

With NTR = Root Trans or Root Fixed:

| NTT | Behaviour |
|---|---|
| **Bypass** | With Root Fixed: no conversion at all. With Root Trans: interval-preserving transposition only |
| **Melody** | For melodic lines (Bass, Phrase1/2) |
| **Chord** | For harmonic parts (Chord1/2) |
| **Melodic Minor** | On major→minor: lowers the major 3rd above the source root a semitone. On minor→major: raises the minor 3rd. Nothing else changes. For Intro/Ending-type sections that respond only to major/minor |
| **Melodic Minor 5th** | Melodic Minor, plus the perfect 5th is moved for aug and dim chords |
| **Harmonic Minor** | Moves the 3rd and 6th (lowered on major→minor, raised on minor→major) |
| **Harmonic Minor 5th** | Harmonic Minor, plus 5th handling for aug/dim |
| **Natural Minor** | Moves the 3rd, 6th and 7th |
| **Natural Minor 5th** | Natural Minor, plus 5th handling for aug/dim |
| **Dorian** | Moves the 3rd and 7th |
| **Dorian 5th** | Dorian, plus 5th handling for aug/dim |

- **Our rule (#11):** The four minor tables are a major ↔ minor switch, not a scale. A chord counts as minor when its 3rd is minor (m, m6, m7, m7♭5, dim, dim7, mM7 and the minor tension chords) and as major otherwise. Going from a major-3rd source to a minor-3rd chord, the table lowers exactly the degrees it names: the major 3rd, plus the major 6th (Harmonic, Natural) and the major 7th (Natural, Dorian). Going the other way it raises the minor 3rd, 6th and 7th it names. Every other note, chromatic ones included, keeps its interval above the root: over C7 a major 7th stays B, and a C7 source keeps its B♭ over Cm. The "5th" variants are the same switch plus the 5th: the source's perfect 5th moves to the chord's ♯5 (aug, 7aug, M7aug) or ♭5 (dim, dim7, m7♭5, 7♭5, and (♭5), which plays as 7♭5). M7♭5 plays as M7(♯11), which has a perfect 5th, so it does not count. The base tables keep the perfect 5th. A source recorded over aug or dim has its ♯5 / ♭5 mapped back to the 5th by the "5th" tables; no corpus rule does this. Chords with no 3rd (sus4, 7sus4, 1+8, 1+5, 1+2+5) go through the Melody scale model instead, and a source with no 3rd counts as major. Corpus (208 styles): authors use the base tables mostly in Intros and Endings, and the 5th tables mostly in Mains and Fills. Recording a separate source per chord family (a C7 source for 7th chords, a CM7 source muted off them) is a Melodic Minor 5th habit only, and a partial one (28 of 68 CM7-source rules). Base-table channels often play a major-7th source over 7th chords (103 rules), and there that B now stays B against the chord's B♭, where the scale model lowered it. This is the literal manual reading and is flagged for playtest.

With NTR = Guitar:

| NTT | Behaviour |
|---|---|
| **All Purpose** | Works for both strumming and arpeggios |
| **Stroke** | Strumming. Some notes deliberately sound muted |
| **Arpeggio** | Four-note arpeggio voicings |

**Guitar NTR voicing model (#12, our model).** The manuals give only each table's purpose, so this is our model (`theory::guitar`).
- **Source: string codes, not pitches.**
  - A key's pitch class picks a string of a guitar voicing of the played chord: B (and B♭) = string 1, A (A♭) = 2, G = 3, F (F♯) = 4, E (E♭) = 5, D = 6.
  - C is the bass, the voicing's lowest string, and C♯ is the fifth above the bass (the octave over 1+8).
  - Keys from C7 (96) up are MegaVoice noise keys (strum, fret and body noises). They pass through untouched on every chord.
  - **Corpus evidence (all 208 styles, every file type):**
    - 382 CASM records in 140 styles have a Guitar zone: 26 All Purpose, 236 Stroke and 117 Arpeggio as the middle zone, plus 3 with a Guitar outer zone.
    - Of the strums of three or more strings (notes at most 30 ticks apart, below the noise keys), 13,372 of 14,153 are stacked seconds. That is 10,633 of 11,092 in T5Style, 2,739 of 2,787 in SX900, and 0 of 274 in MOX_v2.
    - In the playable range, Stroke uses F, G, A and B about equally often (10–12k each) and C, D and E much less. That is one note per string per strum, not a fingering.
    - About a fifth of the Guitar notes (20,887 of 97,156) are noise keys at 96 or above.
- **Source Root/Chord are ignored** (RM p.29). 42 Guitar rules declare another chord (32 Cm11, 3 Cm7, 3 AM7, 2 F♯M7, 1 C6/9, 1 C). Their codes are the same as the CM7 rules', and they play exactly like them.
- **Position:** the key's octave. Keys up to B2 (59) play in the open position (frets 0–4), C3–B3 (60–71) at frets 5–9, and from C4 (72) at frets 10–14. The corpus writes the bass and string codes of a strum inside one octave, so one strum plays one voicing.
- **Voicing:**
  - The bass (the root, or the slash bass with Bass On) goes on the lowest string that reaches it within the position.
  - Each string above takes the lowest fret of a chord tone not yet sounding, the important ones first (3rd, 5th and 7th; for five-note chords the 3rd, 7th and tension), then the lowest chord tone.
  - So the open position gives x32000 for CM7, x32010 for C, 320003 for G, xx0232 for D, x02210 for Am, 320001 for G7, and C D B♭ C E (x3233x-like) for C9. 7♯9 keeps its major 3rd next to the ♯9.
- **Per note:** each note is converted on its own, so a source key under a chord always gives the same result. That matters because most strums spread their notes over several ticks: 14,919 of 19,742 groups of two or more notes span more than one tick.
- **Range:** nothing sounds below the open low E (MIDI 40). Note Limit folds what sounds. It never decides which strings sound.
- **Bass On (per SFF2 zone, #13):** with Bass On, the voicing is built over the slash bass, which takes the lowest string. C/E is 032010 and C/G is 332010. With Bass Off, the slash is ignored. RM p.30: "only the bottom note as Bass inside the Guitar voicings" follows slash chords. 158 of the 382 records set Bass On in at least one zone.
- **All Purpose:** every string sounds. A string below the bass plays the 5th or the root when the hand reaches one (the alternate bass of 332010), else doubles the string above it.
- **Stroke:** strings below the bass, and strings with no chord tone within reach (1+8), are muted: "some notes may sound as if they are muted" (RM p.30). Over the corpus, Stroke leaves out 719,196 of 22,506,408 note × chord plays.
- **Arpeggio:** "four-note arpeggio sounds" (RM p.30). The bass stays on string 6 or 5, and strings 1–4 take the fingering within reach that rises from the bass and, with it, sounds the most chord tones. Every tone of a four-note chord sounds for 332 of the 336 four-note chords in the two main positions; All Purpose manages 248. A triad doubles its root.
- **Pitch-written sources (MOX_v2).** The 43 MOX_v2 Guitar rules are written as real fingerings, x32000 C E G B E for example. They are read as codes too: C as the bass, E as string 5, G as string 3, B as string 1, and the high E as string 5 one position up.
  - The strum becomes a subset of the target chord's guitar voicing. Every note is a chord tone, the root is always there, and there is never a cluster.
  - Over CM7, x32000 plays C E G (the B is lost). Over G it plays G B G G D.
  - Over 286 of the 348 chords with a 3rd, it keeps the 3rd. When the bass sits on string 5, the E written on that string doubles the bass.
  - The goldens' Guitar parts (BluesOrganTrio and JackDoesItAgain, ch12) are MOX_v2 pitch-written sources and show this.
- **One strike per pitch:** strings that land on the same key (1+8, 1+5, or two codes for one string) are struck once.
  - A Guitar string on a key another string of the part struck within a 32nd note (the same strum) joins it as a muted voice, like #63's twin voices for notes that meet at one instant. The key sounds until both have ended, and a later chord can part them again.
  - On one MIDI channel the unison would cut the first string short and strike the pitch again. A later strum strikes the pitch again, as a guitarist would.
  - A chord that lands just after the strum (the 40 ms late-chord allowance) corrects it outright. Strings the previous chord left out (muted by Stroke) come in, because they have no voice for `revoice` to re-pitch.
- **Retrigger Rule:** Guitar zones follow their own RTR through #63's revoice.
  - The Yamaha-authored guitar parts are written for Pitch Shift: 374 of the 375 Guitar zones in T5Style and SX900 (one is Retrigger to Root). All 43 MOX_v2 Guitar zones are Retrigger.
  - Under Pitch Shift, the part's bend takes the shift most of its ringing strings need. Strings that need another shift are retriggered, as for any part.
  - Noise keys are not pitches. A chord change never moves them, and they have no say in the bend. A noise key struck on a bent part goes out as written, since compensating it would pick a different noise. It sounds with the bend.
- **Unverified:** the string codes and the octave-as-position reading come from the corpus, not from a manual. How the Genos voices each chord, and exactly which strings each table mutes or doubles, are our choices. They are flagged for a hardware capture (#9).

**NTT Bass (On/Off)**
- When On, the channel follows slash chords: for Dm7/G, the bass transposes to G instead of D.
- With NTR = Guitar and NTT Bass On, only the lowest (bass) note of the guitar voicing follows the slash bass.
- This is what makes Fingered On Bass audible. Only channels with NTT Bass On move to the played bass note.
- SFF2 stores Bass On per zone (low / mid / high, split at Mid Low and Mid High), so a piano can have its left-hand zone follow the slash bass while its chord zone keeps the root.

**Rhythm channels** must be NTR = Root Fixed, NTT = Bypass, NTT Bass = Off. They never follow chords.

**High Key** (only when NTR = Root Trans)
- The upper limit for root transposition.
- If the new root is at or below High Key, the pattern is transposed **up** to it. If it is above High Key, the pattern is transposed **down**.
- Example with High Key = F: roots C–F go up; F♯–B go down an octave.

**Note Limit Low / High**
- The allowed pitch range after conversion. Any converted note outside it is octave-shifted back inside.
- Example: Low C3, High D4.
- Keeps bass from going too high and piccolo from going too low.
- A range narrower than an octave can't hold every pitch class. The manual doesn't cover this. Our choice: a note with no octave inside the range goes to the octave nearest the range, the lower one on a tie (#13). No corpus style uses a range this narrow.

**RTR (Retrigger Rule): notes already sounding when the chord changes**

| RTR | Behaviour |
|---|---|
| **Stop** | The note is cut |
| **Pitch Shift** | The pitch bends, with no new attack, to the corresponding note of the new chord |
| **Pitch Shift to Root** | Bends without a new attack to the new root, in the same octave |
| **Retrigger** | Restarts with a new attack at the new chord's corresponding note |
| **Retrigger to Root** | Restarts at the new root, in the same octave |

- **Note Generator** (SFF RTR value 5) is not in the Genos editor, and no corpus style uses it. yahaha plays it as Retrigger.
- **How yahaha does Pitch Shift over MIDI:** with the part's pitch bend, so there is no new attack. Each part following chords (ch 11–16) gets a bend range of at least 12 semitones: RPN 0, sent with the part setup, so a section change sets it again wherever a pattern changed it. A part whose patterns bend on their own gets 12 more than its widest pattern bend, up to 24 (the most a Genos part takes, DL p.98), and never shifts by more than the narrowest range it can have (a pattern may set a narrower one) leaves over the pattern's bend, so the two together always fit. The pattern's own bends, and the channel setup's, are rescaled from the style's range, including those a section change sends again and those a Fill entered mid-bar catches up on. Pitch bend is per channel, so all the notes on a part bend together, by the shift that suits most of its continuing notes. A note that needs a different shift is retriggered at its new pitch, and so is a held note the bend would detune. Notes started while a part is bent are sent that much lower, so they sound true. The bend returns to centre at the part's next note once it has fallen silent, so release tails keep their pitch.
- **Notes ending on the change:** a chord played up to 40 ms before a note's pattern note-off, before the pattern strikes its new pitch again, or before the section ends, does not attack that note again: where it would be retriggered it plays out as it is (or stops, if the part's bend moves). A note brought in by the chord (a part coming back from Chord Cancel) is not started if less of it is left than it has missed.
- **Two voices on one key:** when a chord folds two voices onto one key at the same moment (1+8, 1+5), the key sounds once and the second voice is kept muted beside it, so the next chord parts them again.
- **One engine wake, one chord change (#47):** a chord and a command that arrive together (a Keyboard transpose, Chord Cancel, Stop, a part switch, a Break, a Sync Start chord followed by another input) change state only; the band follows the result once, when the engine plays what is due. Before, the chord re-pitched notes and the command moved or cut them at the same instant: zero-length notes.
- **Chord settle (#65, not a Genos setting):** the recognizer publishes a chord on every key, so a rolled chord (F, then F7 3 ms later) is two chord changes. While the style plays, and for Stop Accompaniment and Chord Match Multi Pads with the style stopped too, a chord change reaches the accompaniment once the chord has held still for the **chord-settle window**: each change in a roll restarts the wait, but never past three windows after the first. Meanwhile the chord parts' new notes wait, and so do a Chord Match pad's (the rhythm parts, pads without Chord Match and everything already sounding play on); at the settle they start in the settled chord if the pattern still holds them, and sounding style notes are re-voiced once (a pad note, once struck, keeps its pitch, which is why pads wait too). The "late chord" rule still counts from when the chord arrived, so a chord up to 40 ms after the beat keeps the downbeat. The Chord Looper's playback is not held back: a recorded chord change is exact and never rolled, so it settles at once, before the notes of its own tick.
  - **Decision: default 10 ms, range 0–30 ms (`setChordSettle`, `--chord-settle`), because** the notes of a chord played "together" on a keyboard land up to a few tens of ms apart (the #63 review's rolls were 3 ms), and 10 ms covers a clean block chord while staying at the edge of what a player hears as late: it is about the size of a typical audio buffer plus MIDI transport, and a quarter of the 40 ms the late-chord rule already accepts. The cost is only there: a chord part's note due at the moment of a change starts up to one window later (three through a long roll), and a held-back style note that has little left once it may start is dropped rather than blipped: the rule for any note a chord brings in late (it starts only if no more of it is lost than is still to come) drops one shorter than about twice its wait, so under 20 ms at 10 ms, and up to 60 ms at the 3x cap through a long roll. A chord struck at least a window ahead of the beat costs nothing, and a chord a little late plays the downbeat as before. Across the corpus with F→F7 rolls 3 ms apart every half bar: 5,431 notes struck on the passing chord and cut within 1 ms of the settle with no window, none with 10 ms (`sim::settle_tests`).
  - **Decision: a global setting, not a Registration item (#99), because** the Genos has no such parameter to register, and its nearest kin, the timing settings of how the keyboard is read (Arpeggio Quantize, Arpeggio Hold), are System settings with no Regist column in the Data List parameter chart (DL, Split&Fingering / Keyboard Harmony/Arpeggio). Fingering Type, Chord Detection Area and Manual Bass, which are registrable there, are in the `chord` section; the window is how the player plays, not what a registration recalls.

**Storage:** Source Root/Chord, NTR, NTT Type, NTT Bass, High Key, Note Limit Low/High and RTR are all **Style Data** (DL p.90). The Style Creator Basic parameters (pattern length, tempo, time signature, per-section time signature) are also Style data.

**SFF1 encoding (Ctab + Cntt), our rule (#14):** SFF1 stores one NTT byte per channel in `Ctab` with its own numbering (Bypass, Melody, Chord, Bass, Melodic Minor, Harmonic Minor). Old "Bass" is Melody with NTT Bass On. Codes 06H–0AH aren't defined for Ctab, so we read them as the Cntt/Ctb2 tables with the same numbers (Harmonic Minor 5th … Dorian 5th). Nothing documents a Bass On bit in a Ctab byte, so every other value, 80H–FFH included, plays as Melody without Bass On. An optional `Cntt` record after the Ctabs refines a channel's table with the ones Ctab can't hold, using Ctb2 numbering, and it overrides the Ctab table. Bass On is the Ctab "Bass" code **or** the Cntt bit 7. This departs from the literal reading of the Wierzba/Bedesem Cntt table, where bit 7 is "Bass on/off" and the Cntt overrides the NTT, so a 01H Cntt would switch Bass On off. We don't follow that reading because in every corpus Cntt style, the Bass channel's Cntt is plain Melody (01H) with bit 7 clear, while its Ctab says Bass. Read literally, every one of those Bass parts would stop following slash chords. The evidence is narrow: all 7 Cntt styles come from one library (`MOX_v2/*.T552.sty`), so the oracle may still overrule this. Every other Cntt either repeats its Ctab table or promotes Harmonic Minor to Harmonic Minor 5th. `Cntt` never overrides a `Ctb2` (SFF2), which already stores NTT and Bass On per zone. No corpus file has both.

### C.6 Bass-related features
- **Fingered On Bass:** Uses the lowest chord-section note as the slash bass. Only channels with NTT Bass = On follow it. Ref: RM p.9, p.31
- **Fingered ⇄ Fingered On Bass toggle:** Assignable (pedal or button) and MIDI External Controller. Ref: RM p.142, p.157
- **Bass Hold:** While on, the Style's bass note stays fixed when the chord changes during playback (a pedal-point bass). Control type Toggle, Hold A or Hold B. Does not work with AI Full Keyboard. Ref: RM p.142, p.157
- **Manual Bass:** Upper detection mode only. Mutes the Style Bass channel and gives its voice to the Left part so the left hand plays the bass. Ref: OM p.51
- **MIDI On Bass Note channels:** An external source, such as a pedalboard, can supply the bass. Ref: RM p.154
- **MIDI Pedal templates:** Pedal1 supplies the chord root; Pedal2 plays the bass part. Ref: RM p.150

### C.7 Style Retrigger
- On each chord played, the head of the current **Main** section repeats for the retrigger length. Lengths: 1, 2, 4, 8, 16 or 32 (note values).
- On/off and rate are controlled by Live Control (RtgOnOff, RtgRate, RtgOff&Rt). They are stored in Registration.
- Not usable during MIDI Song playback.
- Ref: RM p.147, p.75

### C.8 Stop Accompaniment (Stop ACMP)
- **Condition:** ACMP on, SYNC START off, Style stopped.
- **Behaviour:** Chords in the chord section are still recognised, and the root and type show in the Home Style area.
- **Setting, whether the chord sounds:**
  - **Off:** silent.
  - **Style:** plays through the current Style's **Pad and Bass** channel voices. MegaVoices there may sound odd.
  - **Fixed:** plays through fixed Pad and Bass voices, whatever the Style.
- **Recording note:** When recording a Song, detected chords are recorded either way. With Style, the sounding notes are recorded too.
- Ref: RM p.11; OM p.131

### C.9 Unison & Accent
- **Not present in the Genos2 manuals.** It is a PSR-SX-series feature. The only related Genos2 items are:
  - Ensemble Voice "Unison" key-assign types (RM p.60).
  - FM voice 2/4 Unison mode (RM p.55).
  - Style Creator "Dynamics / Accent Type" (an editor feature, RM p.27).
- If yahaha wants Unison & Accent, the spec must come from another source.
- **Corpus (#180).** 0 of 208 styles carry any Unison & Accent data. The only chunks present are MThd, MTrk, CASM, OTSc and FNRc, and the only markers are the standard section markers. yahaha therefore cannot play Yamaha's accent figures. Its Accent plays the Main's own fill instead (see Style Dynamics Control).

### C.10 Chord identity numbering (MIDI Chord SysEx and Song chord meta)
- **Chord SysEx:** `F0 43 7E 02 cr ct bn bt F7`. The same data appears in the Song meta event `FF 7F 07 43 7B 01 cr ct bn bt`.
- **Root byte `cr`:** `0fffnnnn`.
  - `nnnn`: 1 = C, 2 = D, 3 = E, 4 = F, 5 = G, 6 = A, 7 = B.
  - `fff`: 0 = ♭♭♭, 1 = ♭♭, 2 = ♭, 3 = natural, 4 = ♯, 5 = ♯♯, 6 = ♯♯♯.
  - Example: 0x31 = C, 0x21 = C♭, 0x41 = C♯.
- **On-bass bytes:** `bn` and `bt` use the same encodings. 127 means no on-bass.
- **Type byte `ct`:** 0–34, listed in the table below.
- **Not in the 0–34 list:** M7♭5, (♭5) and mM7♭5 appear in the Genos Fingered table but have no code. They were probably added later, encoding unknown.
- **Type 2 message:** `F0 43 7E 03 note1…note10 F7` sends raw chord notes instead.
- **Section Control SysEx:** `F0 43 7E 00 ss dd F7`.
  - `ss`: Intro1–4 = 00–03, Main A–D = 08–0B, Fill AA–DD = 10–13, Break = 18, Ending1–4 = 20–23.
  - `dd`: 00 = off, 7F = on.
- **Tempo SysEx:** `F0 43 7E 01 t4 t3 t2 t1 F7`.
- Ref: DL p.111, p.115

---

## D. Chord types recognised in Fingered mode (DL p.45, complete)

In the Voicing column, the numbers are intervals above the root. Notes in parentheses may be omitted. "Example on C" is our own worked spelling. "MIDI ct" is the chord-type code from DL p.111; a dash means no code in that list.

| # | Chord name [abbr.] | Normal voicing | Display (root C) | Example on C | MIDI ct |
|---|---|---|---|---|---|
| 1 | 1+8 | 1+8 | C1+8 | C + C (octave) | 30 |
| 2 | 1+5 | 1+5 | C1+5 | C G | 31 |
| 3 | Major [M] | 1+3+5 | C | C E G | 0 |
| 4 | Sixth [6] | 1+(3)+5+6 | C6 | C (E) G A | 1 |
| 5 | Major seventh [M7] | 1+3+(5)+7 | CM7 | C E (G) B | 2 |
| 6 | Major seventh flatted fifth [M7b5] | 1+3+b5+7 | CM7(b5) | C E G♭ B | – |
| 7 | Major seventh add sharp eleventh [M7(#11)] | 1+(2)+3+#4+5+7 | CM7(#11) | C (D) E F♯ G B | 3 |
| 8 | Add ninth [(9)] | 1+2+3+5 | Cadd9 | C D E G | 4 |
| 9 | Major seventh ninth [M7_9] | 1+2+3+(5)+7 | CM7(9) | C D E (G) B | 5 |
| 10 | Sixth ninth [6_9] | 1+2+3+(5)+6 | C6(9) | C D E (G) A | 6 |
| 11 | Flatted fifth [(b5)] | 1+3+b5 | Cb5 | C E G♭ | – |
| 12 | Augmented [aug] | 1+3+#5 | Caug | C E G♯ | 7 |
| 13 | Seventh augmented [7aug] | 1+3+#5+b7 | C7aug | C E G♯ B♭ | 29 |
| 14 | Major seventh augmented [M7aug] | 1+(3)+#5+7 | CM7aug | C (E) G♯ B | 28 |
| 15 | Minor [m] | 1+b3+5 | Cm | C E♭ G | 8 |
| 16 | Minor sixth [m6] | 1+b3+5+6 | Cm6 | C E♭ G A | 9 |
| 17 | Minor seventh [m7] | 1+b3+(5)+b7 | Cm7 | C E♭ (G) B♭ | 10 |
| 18 | Minor seventh flatted fifth [m7b5] | 1+b3+b5+b7 | Cm7(b5) | C E♭ G♭ B♭ | 11 |
| 19 | Minor add ninth [m(9)] | 1+2+b3+5 | Cm add9 | C D E♭ G | 12 |
| 20 | Minor seventh ninth [m7(9)] | 1+2+b3+(5)+b7 | Cm7(9) | C D E♭ (G) B♭ | 13 |
| 21 | Minor seventh eleventh [m7(11)] | 1+(2)+b3+4+5+(b7) | Cm7(11) | C (D) E♭ F G (B♭) | 14 |
| 22 | Minor major seventh flatted fifth [mM7b5] | 1+b3+b5+7 | CmM7(b5) | C E♭ G♭ B | – |
| 23 | Minor major seventh [mM7] | 1+b3+(5)+7 | CmM7 | C E♭ (G) B | 15 |
| 24 | Minor major seventh ninth [mM7(9)] | 1+2+b3+(5)+7 | CmM7(9) | C D E♭ (G) B | 16 |
| 25 | Diminished [dim] | 1+b3+b5 | Cdim | C E♭ G♭ | 17 |
| 26 | Diminished seventh [dim7] | 1+b3+b5+6 | Cdim7 | C E♭ G♭ A | 18 |
| 27 | Seventh [7] | 1+3+(5)+b7 | C7 | C E (G) B♭ | 19 |
| 28 | Seventh suspended fourth [7sus4] | 1+4+5+b7 | C7sus4 | C F G B♭ | 20 |
| 29 | Seventh ninth [7(9)] | 1+2+3+(5)+b7 | C7(9) | C D E (G) B♭ | 22 |
| 30 | Seventh add sharp eleventh [7(#11)] | 1+(2)+3+#4+5+b7 | C7(#11) | C (D) E F♯ G B♭ | 23 |
| 31 | Seventh add thirteenth [7(13)] | 1+3+(5)+6+b7 | C7(13) | C E (G) A B♭ | 24 |
| 32 | Seventh flatted fifth [7b5] | 1+3+b5+b7 | C7(b5) | C E G♭ B♭ | 21 |
| 33 | Seventh flatted ninth [7(b9)] | 1+b2+3+(5)+b7 | C7(b9) | C D♭ E (G) B♭ | 25 |
| 34 | Seventh add flatted thirteenth [7(b13)] | 1+3+5+b6+b7 | C7(b13) | C E G A♭ B♭ | 26 |
| 35 | Seventh sharp ninth [7(#9)] | 1+#2+3+(5)+b7 | C7(#9) | C D♯ E (G) B♭ | 27 |
| 36 | Suspended fourth [sus4] | 1+4+5 | Csus4 | C F G | 32 |
| 37 | One plus two plus five [sus2] | 1+2+5 | Csus2 | C D G | 33 |
| 38 | cancel | 1+b2+2 | Cancel | C D♭ D | 34 ("cc") |

Footnote from the source: "Notes in parentheses can be omitted."

yahaha gives the three dash rows internal ids 35 (M7♭5), 36 ((♭5)) and 37 (mM7♭5). They are displayed as themselves and followed as their CASM type (§C.4a rule 7).

---

## E. File-format information found in the manuals

- **Style files:** SFF / SFF GE.
  - Sections: Intro, Main A–D, Fill-ins, Break, Endings.
  - Each section has 8 source-pattern channels: Rhythm1, Rhythm2, Bass, Chord1, Chord2, Pad, Phrase1, Phrase2, plus Audio for Audio Styles.
  - Per-channel SFF data: Source Root/Chord, NTR, NTT Type, NTT Bass, High Key, Note Limit Low/High, RTR.
  - Global and per-section data: tempo, basic time signature and per-section time signatures, pattern length.
  - The Style also stores its channel voices, channel mixer (filter, EQ, variation depth, chorus/reverb, pan, volume), channel on/off, reverb/chorus/variation types and return levels, and the four OTS setups.
  - The binary layout is **not** documented in these manuals.
  - Ref: RM p.20–31; OM p.60, p.91; DL p.82–90
- **OTS:** Four panel setups are stored inside the Style file. Their contents are given by the DL "OTS" column (§4).
- **Multi Pad bank:** 4 pads per bank. Per pad: phrase (one MIDI channel, Right-1 voice), name, Repeat, Chord Match. Audio Link pads store a WAV link and an Audio Level. MIDI and audio pads cannot share a bank. The binary format is not documented. Ref: RM p.64–68; DL p.91
- **Registration Bank:** 10 memories. It also stores the Registration Sequence and its end action, and search tags. Ref: RM p.114–117
- **Chord Looper:** .clb (bank of 8) and .cld (one memory). Ref: RM p.17
- **Playlist:** records that link to Registration Bank paths plus an Action (load Regist number, view). Music Finder .mfd can be imported. Ref: OM p.101; RM p.118
- **MIDI Song channel convention:** Right1 = ch1, Left = ch2, Right2 = ch3, Right3 = ch4; pads on ch5–8; Style on ch9–16. Ref: RM p.76

---

## F. Explicitly OUT of scope (found in the manuals)

- **Style Creator:** realtime, step and loop recording; Assembly; Channel Edit (groove, swing, dynamics/accent, quantize, velocity, bar copy/clear, remove event); SFF Edit (as an editor); Drum Setup. RM p.20–33
- **Audio Styles (+Audio):** time-stretched audio drum parts. RM p.7, p.13, p.20
- **Voice Edit:** common, controller, sound, effect and EQ, FM parameters. Also Organ Flutes voice edit and save, and Ensemble Voice edit. RM p.48–63
- **Multi Pad Creator:** MIDI pad recording, step edit, Audio Link pad creation, Multi Pad Edit (rename, copy, paste, delete). RM p.64–68
- **Song player:** Dual Player, Song List, Cross Fader, repeat modes, A-B repeat, position markers, Time Stretch, Pitch Shift, Vocal Cancel, Score, Lyrics, Text Viewer, Guide modes (Follow Lights, Any Key, Your Tempo, Karao-Key), Song Setting (Part Ch, Quick Start, fast-forward, Phrase Mark Repeat), and Auto Accompaniment with MIDI Song playback. OM p.76–89; RM p.69–78
- **MIDI Song recording:** Quick, Multi, Punch In/Out, Setup, channel edit, step edit. RM p.79–98
- **Audio recording:** Quick and Multi, .aud import/export, audio edit. OM p.94–95; RM p.99–112
- **Microphone:** Mic Setting, Vocal Harmony, Synth Vocoder, Talk, VH on/off pedals, VH Live Control functions. OM p.80, p.89, p.110; RM p.120–128
- **Expansion Packs:** install, Instrument Info file, Yamaha Expansion Manager, encryption. RM p.167–168
- **Networking and connectivity:** Wireless LAN, Bluetooth audio, time and clock settings, USB audio interface, HDMI and display out, Line Out routing, Audio Loopback. OM p.116–120; RM p.159–162, p.170
- **Utility and system:** speaker settings, touch calibration, brightness, storage format, Factory Reset, Backup/Restore, Setup files, Auto Power Off, Voice Guide, language, owner name. RM p.162–166
- **Demo, Favorites tab and file management UI:** OM p.36–40; RM p.8
- **Effects and mastering editors:** Master Compressor and Master EQ editing and saving, insertion, variation and system effect parameter editing, and User Effect storage. Selecting types can stay optional. RM p.131–136
- **Super Articulation / AEM articulation engine, MegaVoices, S.Art2 auto articulation:** tied to Yamaha sample content. RM p.37–38, p.41
- **Preset content:** Yamaha preset Styles, Multi Pad banks, Voices, Keyboard Harmony/Arpeggio *arpeggio pattern data*, Ensemble presets, and Vocal Harmony types. These are copyrighted.
- **Panel Lock PIN, Chord Tutor and Registration search/tags:** UI niceties; optional.
- **Unison & Accent:** not a Genos2 feature (§C.9).

---

## G. Open questions the manuals leave to us

1. Factory defaults of every Style Setting option (section timing, OTS link timing, Stop ACMP, Synchro Stop Window values, Change Behavior modes).
2. Chord-recognition priority for ambiguous pitch sets, how inversions are handled in Fingered, AI Fingered inference, and Multi Finger disambiguation.
3. What each channel plays under Chord Cancel. (Decided in #4; see §C.3.)
4. Quantisation and length rules for Chord Looper. (Decided in #29; see chord-looper.md.)
5. The exact algorithm for each NTT table ("Melody" and "Chord" are described only by purpose), and the Guitar NTR voicings. The SFF binary specifics must come from reverse-engineered SFF documentation, not these manuals.
6. Multi Pad Chord Match conversion rules and the .pad binary format.
7. The Style Dynamics Control response curve.
8. The Upper Octave range.
