# Style Retrigger, Tap Tempo, Style Section Reset

Topic id: `retrigger-reset` · Pass: 2026-09-25 · Manual: OM p.46, OM p.67, RM p.39, RM p.147 · yahaha: `docs/section-timing.md`, `src/engine/retrigger.rs`

Paraphrased notes only. No transcript text, manual text or frames are committed.

## Sources

| # | Video (channel) | URL | Timestamps used |
|---|---|---|---|
| V1 | PSR-SX Series: Style Section Reset (YamahaMusikID, official, 51 s) | https://www.youtube.com/watch?v=N-7CUty9tj4 | description; stills 00:10, 00:25, 00:40 |
| V2 | Apply reset tap tempo, e.g. "Song Sung Blue" (Keyboard-Akademie, German) | https://www.youtube.com/watch?v=zeFMS1b3VBg | description only; transcript pending (HTTP 429) |
| V3 | Style Reset practical example "Greek Wine" (Keyboard-Akademie) | https://www.youtube.com/watch?v=jwCHl1vCIMg | transcript pending |
| V4 | Genos 2: Live Control knobs and Style Retrigger (Keyboardseminare) | https://www.youtube.com/watch?v=j_SiBlE3FBY | description only; transcript pending |
| V5 | Genos Style Retrigger testing (YT AndI) | https://www.youtube.com/watch?v=52bGKWLmETU | transcript pending |
| V6 | Genos 2 Style Section Reset (Casper tutorSynth) | https://www.youtube.com/watch?v=n9uc-dJ5J_w | transcript is music only |
| V7 | Fill-in buttons etc. (Robert Gregson, entry-level Yamaha) | https://www.youtube.com/watch?v=apR_iPYGhiE | 12:21-12:47 (tap tempo) |

**Video evidence is still pending for Retrigger** (V4, V5) and for the practical Section Reset
walkthroughs (V2, V3). The descriptions and the official clip's stills are used here.

## Genos behaviour (subtleties a player notices)

- **TAP TEMPO while stopped starts the style** (OM p.46). The player taps once per beat for a bar
  (four taps in 4/4), and the rhythm part then starts at the tapped tempo. With no chord held
  that means rhythm only, as a START with no chord. *manual-confirmed*. It works as a drummer's
  count-in. Robert Gregson taps four on a small Yamaha and the rhythm comes in after the
  taps [V7 12:21-12:47]. *video-consistent* (entry-level model).
- **TAP TEMPO while a MIDI Song plays**: two taps set the tempo (OM p.46). There are no Songs in
  yahaha.
- **TAP TEMPO while the style plays = Style Section Reset**: the section rewinds to its top, for
  stutter effects (OM p.46, p.67). The setting Menu › Metronome › Tap Tempo › Style Section Reset
  turns it into a tempo change instead (RM p.39). *manual-confirmed*.
  - Yamaha's official clip shows the same function on the PSR-SX900. It is labelled RESET/TAP
    TEMPO on the panel, the section goes back to measure 1 / beat 1 (the Home display's
    bar/beat counter reads 001/001), and it can also be triggered from a foot switch
    [V1 stills 00:10, 00:25]. The clip pitches it for following a singer and for songs that
    change metre.
  - Keyboard-Akademie put Reset on a foot switch. Their use: songs that drop half a bar,
    where the player taps the reset on the song's new downbeat so bass and percussion line up
    again. The function is available from Genos firmware 2.x [V2 description]. This is
    evidence that the bar grid restarts at the tap.
- **Tap Tempo settings** (RM p.39): the Volume and Sound (a percussion instrument) of the
  click each tap makes, and Style Section Reset. All three are stored in Registration (DL p.88).
  *manual-confirmed*.
- **Assignable**: "Reset/Tap Tempo" is the same as the button (RM p.144).
- **Style Retrigger** (RM p.147): while on, a chord played repeats the first part of the current
  *Main* for the set length (1, 2, 4, 8, 16, 32). There are Live Control functions for the rate,
  for on/off, and one knob for both (fully left is off; turning right turns it on and shortens
  the length). Retrigger on/off and rate are stored in Registration (DL p.82).
  *manual-confirmed*; video pending.

## Manual check

- Tap-to-start while stopped: *manual-confirmed* (OM p.46), consistent with V7.
- Section Reset to the section top, setting to change tempo instead: *manual-confirmed*
  (OM p.46/67, RM p.39), consistent with V1 (PSR-SX).
- Tap sound and volume: *manual-confirmed* (RM p.39).
- Retrigger: *manual-confirmed* (RM p.147); no video read yet.

## yahaha today

- **Tap while stopped only sets the tempo.** `Button::TapTempo` → `Engine::tap`
  (`src/engine/transport.rs`) averages the last four taps from the second tap on, and never
  starts the band. README and `docs/app-api.md` (`tapTempo`) describe it this way. **Gap
  against OM p.46.**
- Tap while playing, with `styleSettings.sectionReset` on (default on, #128): the section resets
  and the tempo stays. With it off, the tap sets the tempo. The bar grid restarts at the tap; a
  queued change moves to the new grid (`reset_section`, `docs/section-timing.md#style-section-reset-25`).
  Section Reset is also its own command and assignable function. **Matches** (V2's use case
  works).
- **No tap sound.** Taps are silent, and there are no Tap Volume or Sound settings. On the
  Genos every tap clicks (RM p.39), which is how the player hears the count-in.
- Retrigger: head loops at the chord's instant, Mains only, rate 1-32, default 8, stored in
  Registration (`src/engine/retrigger.rs`, `docs/registration.md`). The Launchkey steps the
  rate with Shift + > / Function. yahaha has **no Live Control knob assignment at all** (only a
  `liveControl` Registration group name), so RtgRate, RtgOnOff and RtgOff&Rt (a knob sweep) are
  missing along with the rest. Otherwise it matches the manual. The loop-until-change reading
  is a documented guess pending V4/V5.
- Doc nit: `docs/app-api.md` `setSectionReset` still says off is "yahaha's default". Since
  #128 the default is on.
- Open bug #174 (Section Reset ignores a style change waiting for an Ending) is in this area.

## Gaps

| Gap | Priority | Suggested next step |
|---|---|---|
| TAP TEMPO while stopped doesn't start the style after a bar of taps (OM p.46) | P1 | In `Engine::tap`, when stopped: after `beats per bar` taps at a steady interval, set the tempo and `start` one beat after the last tap (rhythm only if no chord); test in `perform_tests` |
| Taps make no sound; no Tap Volume / Sound settings (RM p.39) | P2 | Click on the built-in synth per tap (reuse `src/click.rs`), plus two settings stored with the Style group |
| No Live Control knob functions for Retrigger (RtgRate, RtgOnOff, RtgOff&Rt): part of the missing Live Control knob assignment | P2 | When Live Control assignment lands (dynamics topic), include the three Retrigger functions; RtgOff&Rt maps the knob's left end to off |
| `app-api.md` says Section Reset off is yahaha's default | P2 (doc) | Fix the sentence |
| Retrigger audible behaviour unverified by video | – | Re-read V4/V5 when captions arrive |
