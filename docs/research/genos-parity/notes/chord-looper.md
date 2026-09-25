# Chord Looper

Topic id: `chord-looper` · Pass: 2026-09-25 · Manual: OM p.68-69, RM p.14-19, RM p.141, DL p.82 · yahaha: `docs/chord-looper.md`, `src/looper.rs`, `src/engine/looper.rs`, `src/session/looper.rs`, `src/registration/mod.rs`, `docs/app-api.md#chord-looper`

Paraphrased notes only. No transcript text, manual text or frames are committed.

**Video coverage this pass: 3 of 4** (V1–V3 read in full; V4 rate-limited, transcript pending).

## Sources

| # | Video (channel) | URL | Timestamps used |
|---|---|---|---|
| V1 | Yamaha Genos 2 Chord Looper Masterclass (Scan Keyboards & Pianos) | https://www.youtube.com/watch?v=cDbItGUbbwM | 01:59, 02:14, 02:40, 05:01–05:28, 06:36 (still), 07:05–07:30, 08:34–09:00 (still 08:58), 10:28–11:11, 12:01–13:38, 14:05–14:44, 14:57–16:01, 16:51 |
| V2 | Using the Chord Looper (Yamaha Music Australia; shown on a CVP-809) | https://www.youtube.com/watch?v=o_norMmfNvA | 01:09–01:50, 02:03, 03:26–03:54, 03:54–05:50 |
| V3 | Genos 2 How To Use Chord Looper (MUSIC 2000) | https://www.youtube.com/watch?v=0fP2s9hdeZI | 00:59, 01:27, 01:53–02:07, 03:10, 03:23–03:37, 04:04–04:42 |
| V4 | Chord looper for rehearsal (Keyboardseminare) | https://www.youtube.com/watch?v=DrLqSdD1SWo | transcript pending |

Stills (private folder): `cDbItGUbbwM_0248_0`, `_0636_0/1`, `_0858_0/1`. The 08:58 still shows the
Chord Looper window during a loop: bank name at the top, the eight memory slots each with a short
list of its chords (the selected one highlighted), Clear/Import/Export tabs, the current chord shown
large, and On/Off lit (green on screen) next to Rec/Stop. The earlier stills are too small to read.

## Genos behaviour (subtleties a player notices)

- REC during playback arms and recording starts at the next bar; ON/OFF ends recording and the loop
  starts at the next bar, which presenters call out as the nice part: no need to time the press.
  [V1 01:59, 02:14], OM p.68, RM p.14-15. *manual-confirmed*
- REC while stopped turns SYNC START on; the first chord starts style and recording together. V2 and
  V3 stop the recording with the style's STOP button. [V3 00:59, V2 01:22], RM p.16.
  *manual-confirmed*
- REC forces ACMP on; during the loop ACMP flashes, chord input is off and the split disappears: the
  whole keyboard is one Right voice (V1 demonstrates a two-handed electric piano solo over a 12-bar
  loop). [V1 08:34–09:59], OM p.68, RM p.15. *manual-confirmed*
- Memories: record, then Memory + slot 1–8; slots can be used as song sections (verse, chorus, a
  held one-chord "ending" slot) and switched while looping, taking effect next bar. [V1 02:40–06:14],
  RM p.17, p.19. *manual-confirmed*
- Pressing ON/OFF before START arms the selected memory so the loop runs from the first bar (V1 uses
  it to vamp while waiting for a singer). [V1 05:01–05:42], RM p.18. *manual-confirmed*
- Looper as a safety net: play chords yourself and switch the loop on only for the section you
  can't remember, then off to take over again; switching off is immediate. [V1 07:05–07:56],
  RM p.19. *manual-confirmed*
- The loop survives a style change (same changes, same bar lengths, new style), and sections/fills
  still work while looping; players use it to audition styles and balance levels over a fixed
  progression. [V1 12:01–13:38, 16:01–16:39]. *video-only* (the manual neither says nor forbids it)
- **Keyboard transpose while looping moves the loop to the new key** (used for a key change on the
  last chorus). [V1 14:05–14:29]. *video-only*
- Bank save (.clb) names the whole set of 8; Clear per slot; Import/Export per slot (.cld) to share
  with other owners. [V1 10:28–11:11, 14:57–16:01], RM p.17-19. *manual-confirmed*
- **Registration stores the Chord Looper**: the bank, the selected memory and the ON/OFF state. V2
  builds a song where Registration 2–4 each select a memory with ON/OFF flashing, and Registration 5
  (the Ending) turns it off; recalling a registration then turns the loop on or off by itself.
  [V2 03:54–05:50, V1 16:51, V3 03:23]. DL p.82 confirms the Registration column and the Freeze group
  "Chord Looper". *manual-confirmed* (store), *video-only* (exactly which fields).
- Preset Chord Looper banks ship with the Genos2 (common progressions in several keys). [V3 03:10].
  *video-only* (not in the Genos2 RM text we have)
- The Yamaha Chord Tracker app can send a selected section's chords into the Chord Looper over a
  wired or wireless connection. [V3 04:04–04:42]. *video-only*
- V3 says the loop runs until you switch it off "or stop the style"; the RM says it resumes on the
  next start if ON/OFF stays on (RM p.18). *conflict (minor)*: the manual wins; yahaha follows it.
- Assignable functions Chord Looper On/Off and Rec/Stop exist. RM p.141. *manual-confirmed*

## Manual check

- The recording/looping timing, SYNC START on REC while stopped, ACMP forced/flashing, 8 memories,
  .clb/.cld, next-bar memory change and immediate stop are all in OM p.68-69 / RM p.14-19.
- Style change and transpose while looping, preset banks and Chord Tracker are *video-only*.
- Registration and Freeze are in DL p.82; the fields stored are shown only in V2.

## yahaha today

- Record/loop/stop timing, REC from stopped with Sync Start, loop armed across a stop, memory change
  at the bar, eight memories with CLD_ names, ignoring keyboard chords and Sync Stop while looping,
  whole keyboard for performance while looping: built and documented (`docs/chord-looper.md`,
  `src/engine/looper.rs`, tests `record_then_loop`, `record_from_stopped`,
  `memory_change_at_the_bar_line`). Matches the manual.
- Keyboard transpose while looping: loop chords enter as played chords and are shifted by the
  Keyboard transpose like any played chord (`src/engine/chords.rs` `shift_chord`, `set_transpose`
  re-shifts the held chord), so it should match; no test covers it.
- Style change while looping: the looper state lives in the engine's features and nothing in the
  style-change path resets it, so it should carry over; no test covers it.
- **No .clb/.cld files**: memories last for the session only (`docs/chord-looper.md` Decisions).
- **No Registration section**: `Group::ChordLooper` exists as a Freeze group, but the registrable is
  not wired ("add theirs when they are wired in", `src/session/registration/sections.rs`,
  `docs/registration.md`).
- **No Launchkey pads** for REC/STOP and ON/OFF (terminal `r` and `^`, the app drawer only), and no
  assignable functions for them (`src/controllers.rs` has none).
- No preset banks; no Chord Tracker import (out of scope).
- The app shows the looper state instead of a flashing ACMP lamp (no ACMP switch). Its drawer lists
  each memory's chords, like the Genos window (`app/src/panels/looper/Looper.svelte`).

## Gaps

| Gap | Priority | Suggested next step |
|---|---|---|
| No hands-on control: REC/STOP and ON/OFF are not on the Launchkey or assignable to pedals/buttons. A live player can't use the looper without the computer keyboard or mouse. | P1 | Small PR: `Function::ChordLooperRec`, `Function::ChordLooperOnOff` in `controllers.rs` (Trigger); then pads when the pad page rework lands |
| Registration does not store the looper (bank, memory, ON/OFF armed state), so the V2 workflow (each verse/chorus registration arms its memory, the Ending registration turns it off) is impossible. | P1 | PR: a `chordLooper` registrable: memory index + on/off (armed) + the 8 sequences (or a bank reference once files exist); Freeze group Chord Looper |
| Memories are lost when the session ends (no bank save/load, no per-slot export/import). | P1 | PR: yahaha's own JSON bank file (8 sequences, names) saved/loaded from the app; per-slot export/import; .clb/.cld stay unsupported (undocumented) |
| Transpose while looping and style change while looping are untested. | P2 | Add two engine tests (loop + `set_transpose`; loop + style load) |
| No preset progressions. | P2 | Optional: a few built-in banks (I–V–vi–IV, 12-bar blues, ii–V–I) as JSON |
