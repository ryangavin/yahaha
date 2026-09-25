# Section Change Timing, Fill-in timing, To Main, Half Bar Fill

Topic id: `section-timing` · Pass: 2026-09-25 · Manual: RM p.12, RM p.142 · yahaha: `docs/section-timing.md`, `src/engine/timing.rs`, `src/engine/sections.rs`

Paraphrased notes only. No transcript text, manual text or frames are committed.

## Sources

| # | Video (channel) | URL | Timestamps used |
|---|---|---|---|
| V1 | Genos - Half Bar Fill In (Alois Müller, German) | https://www.youtube.com/watch?v=MwoQNwfJBVo | stills 00:30, 01:10; transcript pending (HTTP 429) |
| V2 | Yamaha Genos - Half Bar Fill In (Andy's Backing Track; demo, probably no speech) | https://www.youtube.com/watch?v=pO6RPs-eRZs | not read: transcript pending; stills still pending (wanted 00:20, 00:50 to see which beat the fill enters on) |
| V3 | Genos 2 Style Section Reset (Casper tutorSynth) | https://www.youtube.com/watch?v=n9uc-dJ5J_w | transcript is music only (nothing usable) |
| V4 | What is Auto Fill in / Fade / Mixer (BB Walker TV, PSR-S670) | https://www.youtube.com/watch?v=Ww5j2YN0-iE | 03:46, 04:27 |
| V5 | Two-bar fill / long break (Casper tutorSynth) | https://www.youtube.com/watch?v=4opmPtm8oF0 | 00:47-01:13 (fills and breaks are always one bar) |

**Video evidence pending** for the timing edges (fill entry beat, late presses, Half Bar Fill's
audible result). This note rests mostly on the manual and yahaha's code.

## Genos behaviour (subtleties a player notices)

- **To Main A-D** (RM p.12). This covers a section change into a Main and a style loaded while
  playing.
  - **Immediate:** the change comes at the next beat, and the new section plays on from that
    same beat of its bar.
  - **Next Bar:** the change is at once if pressed within the first beat of the bar, otherwise
    at the next bar line.
  - Auto Fill In on forces Next Bar, and so do the Audio Style cases.
  - The setting is recalled with a Registration, but it only takes effect once that
    Registration's style is loaded. *manual-confirmed*.
- **Inside Intro/Ending** (RM p.12): Next Bar (as above) or End of Section. Intro to Intro is
  always Next Bar. Changing into Ending I follows the "conventional rules", which the manual
  never describes. *manual-confirmed*.
- **Fills**: the manuals give no timing rule. On the PSR, BB Walker notes that the fill or
  break sounds different depending on where in the bar it is pressed. That fits a fill that
  comes in at the next beat and plays the rest of the bar [V4 04:27]. *video-only*
  (PSR-S670). A fill or break is always exactly one bar in the style data [V5 00:47], so a
  fill entered mid-bar can only be the rest of that bar.
- **Half Bar Fill In** (RM p.142): an assignable function with a control type (Toggle / Hold).
  While it is on, a section change on the first beat starts the next section from the middle,
  with an automatic fill. It does not work with Audio Styles. *manual-confirmed*. The V1 still
  at 01:10 shows the presenter on the Assignable settings page, pressing an Assignable button
  (the screen text can't be read at 480p). The audible half-bar entry is not yet confirmed
  from video.

## Manual check

- To Main / Inside Intro-Ending / Registration note / Ending I exception: *manual-confirmed* (RM p.12).
- Fill entry at next beat: *video-only* (PSR), manual silent.
- Half Bar Fill "from the middle": *manual-confirmed* wording, exact musical reading unconfirmed.

## yahaha today

- `Engine::change_point` (`src/engine/sections.rs`) is the one timing policy. `MainTiming::{Immediate,
  NextBar}` and `IntroEndingTiming::{NextBar, EndOfSection}` follow RM p.12 exactly. That
  includes the first-beat rule (`bar_or_now`), the Auto Fill fallback, Intro to Intro, and
  Ending I at the next bar line. The Registration recall stores To Main and applies it with
  the Registration's style (`docs/registration.md`). **Matches.**
- Fills and Break: `Change::Fill` → `next_beat`, which uses `ceil`. A press exactly on a beat
  starts the fill there; a press a few ms after the beat waits for the next beat. A fill
  pressed in beat 4 starts on the next downbeat and plays a whole bar. Then the Main comes at
  the bar after. Unlike Next Bar for Mains, there is no "within the first part of the beat,
  start now" allowance. So a fill hit just late on beat 1 loses a whole beat. Whether the
  Genos catches late presses is unknown (owner question).
- Half Bar Fill In: a Main or fill function pressed during beat 1 of a bar while a Main plays
  queues the fill at the half bar (`Change::HalfBar`, `Prepared::half_bar`: beat 3 in 4/4). The
  Main follows at the next bar line. It plays with Auto Fill off too. This is a documented
  interpretation (`docs/fills-and-rules.md#half-bar-fill-in-24`).
- Half Bar Fill In is reachable from the terminal (`N`), the API (`setHalfBarFill`,
  `toggleHalfBarFill`) and the Launchkey. It is **not in the assignable function list**
  (`src/controllers.rs` `FUNCTIONS`), although RM p.142 has it as a pedal/Assignable function
  with a control type, and the fills doc says the Hold types are a controller setting (#34).

## Gaps

| Gap | Priority | Suggested next step |
|---|---|---|
| A fill pressed just after a beat waits a whole beat; no late-press allowance like Next Bar's first beat | P1 (if the Genos has one) | Owner question: on the Genos, does Fill hit ~50 ms late on beat 1 start at once? If yes, add a small grace window in `next_beat` for `Change::Fill` |
| Half Bar Fill In missing from assignable functions (pedal Toggle / Hold A / Hold B) | P2 | Add `Function::HalfBarFill` as a `ControlSwitch`-style engine switch using `SetHalfBarFill` |
| Half Bar Fill musical reading unconfirmed | P2 | Re-read V1 when captions arrive; take stills of V2 at 00:20 and 00:50 |
