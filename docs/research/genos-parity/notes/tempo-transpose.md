# Tempo, Transpose, Upper Octave, tempo per style

Topic id: `tempo-transpose` · Pass: 2026-09-25 · Manual: OM p.46, OM p.61, RM p.12-13, RM p.42 · yahaha: `docs/fills-and-rules.md#change-behavior-26`, `src/api/transport.rs`

Paraphrased notes only. No transcript text, manual text or frames are committed.

## Sources

| # | Video (channel) | URL | Timestamps used |
|---|---|---|---|
| V1 | Style Tempo Lock/Reset, Genos v1.4 (Casper tutorSynth, 45 s) | https://www.youtube.com/watch?v=5HVLrLvMQYo | description; stills 00:12, 00:30 |
| V2 | Style Tempo Lock feature in Yamaha (Arranger Tutorials) | https://www.youtube.com/watch?v=bhDrkJm1ZTg | 00:17-00:49, 01:19-01:50, 02:26-03:20 |
| V3 | Tempo delay at the touch of a button (Keyboard-Akademie, German) | https://www.youtube.com/watch?v=vIJSAy8kVXY | description only (it is about an effect-delay tempo, not a ritardando button); transcript pending |
| V4 | Parameter Lock (Casper tutorSynth) | https://www.youtube.com/watch?v=kcy42Hnp-DE | 00:21-00:34, 00:49-02:28, 03:55 |
| V5 | Genos testing style tempo variation (YT AndI) | https://www.youtube.com/watch?v=H6bUl7EaE_g | transcript pending |
| V6 | Genos 2: questions you may be embarrassed to ask (Scan Keyboards & Pianos) | https://www.youtube.com/watch?v=nrS_ZQObREk | 21:23-22:25, 22:50-23:02 |

**Video evidence is still pending** for the tempo buttons (step size, hold-to-repeat) and for
V3 and V5.

## Genos behaviour (subtleties a player notices)

- **Tempo range and buttons** (OM p.46): TEMPO −/+ open a tempo pop-up. The range is 5-500
  BPM, and holding a button keeps stepping it. Pressing −/+ together goes back to the tempo
  that the most recently chosen Style or Song came with. The manual gives no step size per press.
  *manual-confirmed*.
- **Change Behavior › Tempo** (RM p.12). *manual-confirmed*.
  - **Lock**: always keep the tempo.
  - **Hold**: keep it while playing; take the new style's tempo when stopped.
  - **Reset**: always take the new style's tempo.
  - The assignable functions "Style Tempo Lock/Reset" and "Style Tempo Hold/Reset" flip
    between Reset and Lock/Hold (RM p.144). Casper says they came with Genos firmware 1.4
    (V1 description).
  - The V1 still at 00:30 shows the feedback: pressing the Assignable button opens a small
    pop-up with the function name and the new value (Reset). The player sees which state
    they are in.
  - V2 walks through both toggles. With Lock on, a **lock icon** appears next to the tempo,
    and choosing styles with other default tempos leaves it unchanged [V2 01:19-01:50]. With
    Hold, the icon shows **only while the style plays**: it appears at start and goes at
    stop, and a style chosen while stopped takes its own tempo [V2 02:26-03:20].
    *manual-consistent* (RM p.12).
  - V2 says the **factory default is Reset**: the tempo follows every style change
    [V2 00:17-00:49]. *video-only*. The model isn't named, and the manual gives no default.
    Reset, if the Genos uses it too, means a style picked while playing jumps to its own
    tempo.
- **Tempo is stored** in Registration (freeze group Tempo) and in the Style data. It is not
  lockable by Parameter Lock (DL p.82 chart). *manual-confirmed*.
- **Transpose** (OM p.61) runs −12..+12 per target. *manual-confirmed*.
  - **Master** moves everything except audio and mic.
  - **Keyboard** moves the keyboard, including the chord root sent to the Style.
  - **Song** moves the MIDI Song.
  - Pressing −/+ together resets the value to 0 (also shown on a Genos 2 in [V6 22:50-23:02]).
  - Drum and SFX kits are never transposed.
  - **Transposing during an Intro changes nothing until the Intro has finished and the next
    chord is played.** Scan uses this for a second verse up a key: he restarts the Intro, then
    transposes while it plays [V6 21:23-22:25]. *video-only*. It suggests that on the Genos a
    Keyboard transpose reaches the style only with the next chord played, not on the chord
    already held.
- **Upper Octave −/+** (OM p.61) shifts Right 1-3 by an octave per press. Pressing both resets
  it. It is stored in Registration (freeze Voice; DL p.82). *manual-confirmed*.
- **Parameter Lock** (RM p.163): locked groups change only from the panel, never from
  Registration, OTS, Playlist or Song data. From the DL chart's Parameter Lock column, Split
  Point and Fingering Type can be locked, but Tempo, Transpose and Upper Octave cannot.
  *manual-confirmed*.
  - Casper counts eight lockable groups and says a locked value can still be changed in its
    own menu without unlocking. It just isn't overwritten by Registration, OTS, Song or
    Music Finder [V4 00:21-00:34].
  - His main example is the style's master effect. Every style load normally changes it,
    and it also colours the right-hand voices. With the lock on, it stays as last set
    [V4 00:49-01:45]. Split Point and Fingering are handled the same way [V4 02:02-02:28].
  - He calls it a layer above Registration, OTS and Song, a companion to Registration Freeze
    with a different reach [V4 03:55]. *manual-confirmed* (RM p.163).

## Manual check

- Tempo range, hold-to-repeat, both-buttons default recall: *manual-confirmed* (OM p.46).
- Change Behavior and the assignable toggles: *manual-confirmed* (RM p.12, p.144); V1 still consistent.
- Transpose targets / reset, Upper Octave: *manual-confirmed* (OM p.61, DL p.82).
- Lock icon, Hold icon only while playing: *video-only* (V2), consistent with RM p.12. Factory
  default Reset: *video-only* (V2).
- Transpose deferred during an Intro until the next chord: *video-only* (V6, Genos 2);
  the manual is silent. **Conflict** with yahaha's immediate re-voicing.
- Parameter Lock scope: *manual-confirmed* and V4 agree.
- Step per TEMPO press: *not in the manual*. It is commonly 1 BPM on Yamaha panels, but that
  is unconfirmed (owner question).

## yahaha today

- Range 5-500 (`MIN_BPM`/`MAX_BPM`, `src/engine.rs`). **Matches.**
- `TempoUp`/`TempoDown` step **2 BPM** per press (`src/engine/transport.rs`). There is **no
  hold-to-repeat** (the Launchkey Scene/Function buttons send one step), and **no
  "−/+ together = the style's default tempo"**. No command or key restores the style's tempo;
  the only way is `setTempo` to the number shown in `library` / `style.tempo`.
- Change Behavior Tempo Lock/Hold/Reset and Part On/Off: implemented, default Hold
  (`src/engine/change_rules.rs`, `docs/fills-and-rules.md#change-behavior-26`). **Matches.**
  - The toggles exist as API commands (`toggleStyleTempoLock`, `toggleStyleTempoHold`). They
    are **not in the assignable function list** (`src/controllers.rs` `FUNCTIONS`), so a pedal
    can't flip them as on the Genos.
  - **No lock indicator by the tempo.** The rule is shown only in Settings ›
    `ChangeBehavior.svelte`. The Genos shows a lock icon (Lock always, Hold while playing, V2).
  - **The default is Hold; V2 says Yamaha's default is Reset** (model not named). The manual
    gives none. yahaha's choice is documented in `docs/fills-and-rules.md`.
- Transpose: Keyboard and Master, −12..12, drums untouched, keyboard transpose moves the chord
  root (`src/engine.rs` `Transpose`, `src/sim.rs` transpose tests). Transpose +/− are
  assignable. There is one reset that zeroes both (`resetTranspose`, key `/`). **Matches**,
  except that the Genos reset acts on the target the buttons are set to.
- **A Keyboard transpose with a chord held moves the band at once.** The test
  `keyboard_transpose_change_moves_held_chord` (`src/sim.rs`) checks that it is the same as
  playing the moved chord. In V6 the Genos keeps the Intro in the old key until the next
  chord. This is a conflict, but video-only.
- **No Upper Octave** control: per-part octave exists (`setPartOctave`, −2..2), but there is no
  Right 1-3 group shift, no both-buttons reset, and no Upper Octave +/− assignable function.
- Parameter Lock: Split Point and Fingering Type (`src/api/param_lock.rs`). That covers every
  lockable group yahaha has. **Matches.**
- Tempo in Registration (group `tempo`) and a stopped style change following Change
  Behavior. **Matches.**

## Gaps

| Gap | Priority | Suggested next step |
|---|---|---|
| No "TEMPO −/+ together = style default tempo" (OM p.46) | P1 | Add `resetTempo` (engine: the loaded style's tempo); Launchkey: Scene + Function together; terminal key; assignable is not needed (the Genos has none) |
| TEMPO −/+ step is 2 BPM; the Genos step is unconfirmed (likely 1); no hold-to-repeat | P2 | Owner question on the step; add auto-repeat while the Launchkey button is held |
| Style Tempo Lock/Reset and Hold/Reset not assignable to pedals (RM p.144) | P2 | Add two `Function`s mapped to the existing session commands, with an on-screen toast of the new value (V1 still) |
| No Upper Octave −/+ (Right 1-3, both = reset; Registration freeze Voice) | P2 | Add `upperOctave` (−2..2?, OM gives no range) on top of part octaves; assignable Upper Octave +/− |
| Keyboard transpose with a chord held re-voices the band at once; on the Genos, V6 shows it waiting for the next chord (seen during an Intro) | P1 (if confirmed) | Owner question; if confirmed, apply a Keyboard transpose to the style's chord only on the next chord input (keep the held chord's root) and flip `keyboard_transpose_change_moves_held_chord` |
| No tempo-lock icon by the tempo (Lock always, Hold while playing) | P2 | Show a lock badge on the tempo readout from `styleChange.tempo` and `running` |
| Change Behavior Tempo default Hold vs Yamaha's Reset (V2, model not named) | P2 | Owner question; keep Hold unless the Genos default is confirmed |
