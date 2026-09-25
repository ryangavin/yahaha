# Registration Memory, Freeze, Registration Sequence, Playlist

Topic id: `registration` · Pass: 2026-09-25 · Manual: OM p.96-103, RM p.113-119, RM p.138-144, RM p.163, DL p.82-93 · yahaha: `docs/registration.md`, `src/registration`, `src/session/registration/sections.rs`, `src/controllers.rs`, `docs/controllers.md`

Paraphrased notes only. No transcript text, manual text or frames are committed.

**Video coverage this pass: 4 of 5** (V1–V4 read in full; V5 rate-limited, transcript pending). V6 is
the live-control cluster's pedal video, used here for the Foot Pedals group.

## Sources

| # | Video (channel) | URL | Timestamps used |
|---|---|---|---|
| V1 | Registration Freeze 03, explained with examples (Casper tutorSynth) | https://www.youtube.com/watch?v=VcJ8oYgQa8g | 00:22–00:47, 01:01, 01:18–02:52, 04:08, 08:40–09:31 |
| V2 | Genos: how to use registrations, full tutorial (Leigh Wilbraham) | https://www.youtube.com/watch?v=ocYzJhzD4Is | 01:46–03:21 (still 02:52), 03:54, 05:31–05:55, 09:24–10:27, 17:24–18:15 |
| V3 | Genos 2 Registration & Playlist masterclass (Scan Keyboards & Pianos) | https://www.youtube.com/watch?v=oM2FkjRt6FA | 00:55–02:22, 03:05–04:38 (still 04:05), 07:39, 08:56–09:20, 12:37–13:02, 18:51–19:05, 19:57–20:22, 22:34, 23:50–25:04, 26:48–29:22, 33:02, 34:37–35:03 |
| V4 | How to change your registrations with a foot switch (Genos Tipsters) | https://www.youtube.com/watch?v=mrhO6veyjMU | 00:40–01:57, 02:21–03:53 |
| V5 | Registration Sequence (Keyboard-Akademie) | https://www.youtube.com/watch?v=bqcP6bH1lMw | transcript pending (429) |
| V6 | How to keep your pedals working correctly (ePianos.co.uk) | https://www.youtube.com/watch?v=m2WFeE7Eeyk | 00:14–01:47, 01:59–03:41 |

Stills (private folder): `ocYzJhzD4Is_0252_0`, `oM2FkjRt6FA_0405_0` (+ a zoom). Both show the
Registration Memory window: the group checkboxes (Voice, Style, Tempo, Multi Pad, Line Out, Chord
Looper, Transpose, Scale Tune, Live Control, Foot Pedals, Assignable Buttons, Keyboard
Harmony/Arpeggio, Vocal Harmony/Mic Setting, the Song/Text groups; the Genos 2 adds MIDI Setting),
and a prompt that the button pressed may be a Registration Memory or a One Touch Setting button.
Transpose is unticked in both (in V2 by the presenter's choice). The V3 zoom shows the STYLE CONTROL
row while button 5 is memorized with the band stopped: an ENDING lamp is lit.

## Genos behaviour (subtleties a player notices)

- MEMORY opens the group checklist; a ticked group is stored; then a number button stores the
  panel. Lamps: red = selected, blue = stored, dark = empty. [V2 02:40–03:21], OM p.97.
  *manual-confirmed*
- Presenters describe a registration as a snapshot of the whole panel: voices with their reverb and
  chorus edits, the style, the Main, an armed Intro, Sync Start, OTS Link, Multi Pad, joystick/Live
  Control setup, songs. [V2 01:46, 03:54; V3 03:05–04:38, 18:51–19:05, 22:34, 33:02, 34:37].
  DL p.82–93 confirms each of these as Regist items (keyboard-part pan/reverb/chorus/filter/EQ,
  Section, Sync Start, OTS Link, Live Control, Foot Pedals, Assignable Buttons, Chord Looper).
  *manual-confirmed*
- **Conflict:** V3's presenter says it remembers "any buttons I press" and mentions Auto Fill being
  on, but the DL marks Auto Fill In as not a Registration item (DL p.82). Manual wins; the video
  never shows Auto Fill being recalled.
- A registration can hold an **Ending**: V3 memorizes button 5 with an Ending lit and then plays
  intro → buttons 2–4 → button 5, which ends the song without touching the style buttons. [V3
  03:59–04:38, still 04:05]. The DL's "Section" item is a Regist item (DL p.82) but the manuals don't
  say an Ending counts. *video-only*
- Many players leave Transpose out of their registrations so a song's key stays theirs. [V2
  02:53–03:07; V3 still] *video-only* (the group exists: DL p.92, group Transpose)
- Selecting a bank recalls nothing; only a number button does. Players get caught by this. [V2
  09:24–10:27], OM p.98. *manual-confirmed*
- The ten buttons survive power-off; holding the top F#6 while powering on clears them (and the last
  bank). [V3 00:55–02:22], OM p.97. *manual-confirmed*
- Bank Edit renames or deletes single memories, several at once; banks list alphabetically; Bank
  Info shows each button's voices and style, and touching one loads it. [V3 08:56–09:20, 12:37,
  13:54–14:18, 23:50–24:26], OM p.99. *manual-confirmed*
- Freeze keeps the ticked groups when recalling: freeze Voice to keep your right hand while the
  rhythm and tempo change, or freeze Style and Tempo to keep the band and change only sounds. [V1
  00:22–00:47, 01:18–04:08], RM p.113. *manual-confirmed*. V1 notes the Voice group also covers the
  right-hand split and voice characteristics [V1 01:01]; DL puts Split Point (Right3) in Voice and
  the Left split in Style (DL p.88).
- Foot pedal and Assignable button setups are Registration items (groups Foot Pedals, Assignable
  Buttons). A registration made on someone else's instrument can silently change your pedals; the
  common fix is to tick only Foot Pedals in Freeze and turn Freeze on. [V6 00:14–03:41], DL p.88–89.
  *manual-confirmed*
- Registration Sequence: a programmed order of buttons stepped by a pedal (Regist +/− set under Pedal
  Control on the Sequence display) or an Assignable button; end action Stop/Top/Next; saved in the
  bank. [V3 24:39–25:04; V4 00:40–03:53], RM p.114–115. *manual-confirmed*. V4 (firmware 1.40)
  says a new sequence defaults to 1–10 in order, so a foot switch steps through the buttons with no
  programming; the home screen shows the sequence and the current step. *video-only* (not in the
  Genos2 RM)
- Pedal priority when one pedal has several jobs: Voice Guide → Punch In/Out → Registration Sequence
  → the Assignable function. RM p.114, p.138. *manual-confirmed*
- Assignable functions for registration: Memory, Regist 1–10, Bank +/−, Freeze On/Off, Sequence
  On/Off (pedal and buttons); Sequence +/− on the Assignable buttons only (pedals use Pedal Control).
  RM p.141. *manual-confirmed*
- Playlist: records link to bank files (and optionally load one memory and switch the view); a
  playlist is only an order, so editing a registration never needs the playlist redone; Up/Down,
  Delete, Append, multi-select add, search. [V3 25:30–38:25], OM p.100–103. *manual-confirmed*
- Parameter Lock keeps locked groups from Registration, OTS and Playlist recalls. RM p.163.
  *manual-confirmed*

## Manual check

- Agreement everywhere except the V3 Auto Fill remark (DL wins) and two video-only points: an Ending
  held in a registration, and the 1–10 default sequence of firmware 1.40.
- DL p.88: Arpeggio Quantize and Arpeggio Hold are **not** Registration items (Backup only), while
  Arpeggio Velocity, Gate Time and Unit Multiply are.

## yahaha today

- Ten buttons per bank, lamps, Memorize groups, Freeze, bank files, Bank −/+, bank recalls nothing
  until a button, playlist with bank/style records, Append, sort, 2,500 records; Parameter Lock for
  Split Point and Fingering Type. `docs/registration.md`. Matches the Genos flow.
- Stored sections: style, Multi Pad bank, tempo, chord settings, styleControl (Main, armed Intro,
  Sync Start/Stop, Stop ACMP, OTS Link), style mixer (player-set levels, mutes), keyboard parts
  (voice/patch/plugin, on, CC7, octave), transpose, harmonyArp, styleSettings.
  `src/session/registration/sections.rs`. No Ending, no per-part pan/reverb/chorus/filter/EQ, no Live
  Control, no Foot Pedals, no Chord Looper section.
- `harmonyArp` stores the arpeggio's Quantize and Hold setting (docs/registration.md), which the DL
  keeps out of Registration. The group table in docs/registration.md still calls the Keyboard
  Harmony/Arpeggio group "reserved" although the section exists.
- Freeze groups: Style, Voice, Tempo, Transpose, Multi Pad, Assignable, Keyboard Harmony/Arpeggio;
  Live Control and Chord Looper reserved; no Foot Pedals group (a `footPedals` group from a newer
  build is kept, not recalled). Pedals are global settings, so in effect they are always frozen.
- Registration Sequence: steps, end action, Regist +/− from the Launchkey pads, keys and app; an
  empty sequence does nothing (`src/registration/sequence.rs` `step`). **No pedal can step it:** the
  assignable table has Registration Bank +/− only (`src/controllers.rs` FUNCTIONS; docs/controllers.md
  says Sequence +/− is left out because the Genos drives it through Pedal Control, but yahaha has no
  Pedal Control either). Regist 1–10, Memory, Freeze On/Off and Sequence On/Off aren't assignable.
- Start-up: an empty, unsaved New Bank; the Genos keeps the last bank and its ten buttons
  (documented follow-up in docs/registration.md).
- Memorize ticks every group by default; the videos show Transpose unticked (default unknown).

## Gaps

| Gap | Priority | Suggested next step |
|---|---|---|
| No foot pedal can step the Registration Sequence (and none can recall Regist 1–10, Memory, Freeze, Sequence On/Off) | P1 | Add Pedal Control Regist +/− (a Setup setting per pedal, priority above the pedal's function, as RM p.114) or the Assignable "Registration Sequence +/−" rows; add Regist 1–10, Memory, Freeze On/Off, Sequence On/Off as assignable functions; tests in `src/controllers.rs` |
| Empty sequence does nothing; Genos fw 1.40 steps 1–10 by default (video-only) | P2 | Owner/hardware check; if confirmed, treat an empty sequence as the stored buttons 1–10 in order |
| Keyboard-part pan, reverb and chorus depth (and filter/EQ) aren't stored or recalled | P1 | Comes with the mixer's per-part pan/sends (see mixer.md); then a `parts` section field, group Voice (Right) / Style (Left) |
| An Ending can't be held in a registration (video-only) | P2 | Hardware check of what a registration memorized with an Ending lit does on recall (playing and stopped); if it triggers the Ending, store it in `styleControl` |
| Foot Pedals (and Assignable button) setups aren't Registration items; no Foot Pedals Freeze group | P2 | Add a `footPedals` section and group (pedal function, CC, control type, parts), default ticked, as the DL; Freeze then gives today's behaviour |
| Last bank not restored at start-up | P2 | Remember the last bank path in `setup.json` and load it (buttons lit, nothing recalled) |
| Arpeggio Quantize and Hold are stored, though the DL keeps them out of Registration | P2 | Drop them from the `harmonyArp` section (keep reading old banks), or record a decision |
| Live Control and Chord Looper sections not stored | P2 | When those features land (Chord Looper exists: see chord-looper.md) |
| docs/registration.md group table says Keyboard Harmony/Arpeggio is reserved | P2 | Doc fix |
