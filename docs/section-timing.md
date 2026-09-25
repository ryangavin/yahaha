# Section timing, Fade, Ritardando, Synchro Stop Window, Section Reset, Retrigger

M5 issues #22, #23 and #25. How yahaha implements these Genos features, where the code is,
and which parts are guesses because the manuals leave them open. Spec:
[genos-features.md](genos-features.md) §1 and §3. Commands and state:
[app-api.md](app-api.md) (`styleSettings`, `toggleFade`, `sectionReset`, `toggleRetrigger`).

## Where the code is

| What | Where |
|---|---|
| The settings (`StyleSettings`), Next Bar / next beat points | `src/engine/timing.rs` |
| Section Change Timing policy | `Engine::change_point` in `src/engine/sections.rs` (`Change::Main`, `Change::IntroEnding`, `Change::Style`) |
| Fade In/Out | `src/engine/fade.rs`; the synth side in `src/synth.rs` (`master_volume_msg`), routed by `live::Out::push` |
| Ending ritardando | `src/engine/ritardando.rs` |
| Section Reset, Retrigger | `src/engine/retrigger.rs` |
| Synchro Stop Window | `src/engine/sync_stop.rs` |
| Engine state | `hooks::Features` (`settings`, `fade`, `rit`, `retrigger`, `sync_window`); `on_start`, `on_stop`, `after_section_change`, the new `on_wake` hook and `hook_wake_ns` deadline |
| Commands | `live::Cmd::StyleSettings`; `Button::{Fade, SectionReset, Retrigger}` |
| API | `src/api/style_settings.rs`, `TransportCmd::{ToggleFade, SectionReset, ToggleRetrigger}`, `TransportState::{fade, retrigger, ritardando}` |
| Session | `src/session/style_settings.rs` (keeps the settings, sends the whole set) |
| Tests | `src/engine/perform_tests.rs`, `tests/perform_no_alloc.rs`, `session_tests.rs` |

## Section Change Timing (#22)

- **To Main** (`mainTiming`), also a style change while a Main plays:
  - **Next Bar** (default): at once when pressed within the first beat of a bar (the new
    section starts at that point of its bar, like a fill entered mid-bar), otherwise at
    the next bar line.
  - **Immediate**: at the next beat; the new section plays from that beat of its bar.
    With Auto Fill In on, a Main change falls back to Next Bar (RM p.12). A style change
    stays Immediate (the fallback names section changes only).
- **Inside Intro/Ending** (`introEndingTiming`), switching Intro/Ending while one plays:
  - **Next Bar** (default): as above. **End of Section**: at the end of the one playing.
  - Intro to Intro is always Next Bar (RM p.12).
- The new "first beat" rule is the manual's Next Bar. yahaha used to wait for the bar
  line even when pressed just after it; it now switches at once within the first beat.

Decisions (the manuals leave these open):
- **Decision (confirmed by the owner, as on the Genos): a style change while an Ending
  plays waits for the Ending to end**, even when chosen in the Ending's first beat; the
  band stops there with the new style loaded. The first-beat rule and Immediate apply to
  a style change only while a Main plays; from an Intro, a Fill or the Break it waits for
  the next bar line, as before this setting existed (`Engine::change_point`,
  `Change::Style`). Checked against RM p.12 (#107): To Main covers "changing from a
  section to a Main section" and "loading another Style", both about the Main playing;
  during an Intro, Fill or Break no Main plays yet, so the style keeps the bar line (a
  Fill's bar line is where its Main starts). Pinned by
  `perform_tests::a_style_change_from_an_intro_or_a_fill_waits_for_the_bar_line`.
- **An Ending pressed but still waiting for its bar line counts as playing (#111).** A
  style chosen then waits for that Ending's end as well: the Ending plays in the old
  style, and the band stops with the new style loaded. A style chosen first and an
  Ending pressed later, both due at the same bar line, is left as it was: the style takes
  over at that bar line and the Ending plays in the new style.
- **Into Ending I, and from a Main or Fill into an Intro or Ending: the next bar line.**
  The manual says Ending I follows "the conventional rules" without saying what they are;
  yahaha's rule before this setting existed was the next bar line, so that is kept. The
  "Inside Intro/Ending" setting is only about changes while an Intro or Ending plays.
- **The stop an Ending the style lacks makes waits for the bar line**, as before.
- **Defaults: Next Bar for both.** The manual gives no factory defaults; Next Bar is the
  closer to yahaha's old behaviour.

## Ending ritardando (#23)

Press the Ending that is playing again: the tempo slows down to the end of the ending.

Decisions:
- **Linear to 65% of the tempo at the ending's last tick**, updated at every engine wake
  and at least every sixteenth note
  (`RIT_END`). The manual says only "gradually slows".
- **The tempo comes back when the band stops**, or when a Main takes over from the
  ending. The tempo buttons during a ritardando move the tempo it comes back to, and a
  TAP TEMPO (with Style Section Reset off) sets it: the band slows on from the tapped
  tempo and comes back to it.
- Pressing the Ending again while it is only queued does nothing new (it stays queued).

## Fade In/Out (#23)

Stopped: arm, then START (or a Sync Start chord) fades in. Playing: fade out, then the
band stops and the volume is held at 0 for the hold time. Times: 0–20.0 s in, 0–20.0 s
out, 0–5.0 s hold (RM p.142).

Decisions:
- **Only the Style fades** (RM p.142: the fade moves "the Style/Song volume"). Your
  playing (Right 1-3, Left) and the Multi Pads never fade, and the Fade Out Hold Time
  holds the Style at 0 after it stops.
- **The fade is the Style parts' CC7, nothing else.** While a fade runs, each Style part's
  CC7 goes out (to the yahaha port and the built-in synth alike) as its fader value scaled
  by the fade position; when it ends, the fader value goes out unchanged again. The
  faders themselves (the mixer, the app, soft takeover) never move, and there is no gain
  in the synth or anywhere else that the wire does not show: the mixer rule's one
  exception, written into docs/architecture.md. A fader moved or a pattern CC7 during a
  fade goes out scaled too.
- **The curve**: the CC7 scale moves linearly in time (CC7 is a squared gain on a GM
  receiver, so it sounds like a fader), a new level every 10 ms while it moves.
- **Defaults: 5.0 s in, 5.0 s out, 2.0 s hold.** The manuals list none.
- A fade out already running carries on if pressed again. START/STOP (or Panic) during a
  fade ends it at full volume, and Panic also ends the hold. START during the hold ends
  the hold (a fade in if armed).
- The fade times are session settings (Registration is not implemented yet).

## Synchro Stop Window (#23)

With Sync Stop on and a window set, a chord held longer than the window turns Sync Stop
off: letting go no longer stops the band (RM p.12).

Decisions:
- **Values: Off, or 0.1–5.0 s in 0.1 s steps; default Off.** The manual lists none.
- The hold is timed from the first chord after the chord section was last let go;
  changing chords without letting go keeps timing. The engine wakes at the window's end
  (`hook_wake_ns`), so Sync Stop goes off on time, not at the next event.

## Style Section Reset (#25)

On the Genos, TAP TEMPO while the style plays restarts the section from its top, at the
tap (OM p.46, p.67), unless Tap Tempo › Style Section Reset is turned off (RM p.39).
yahaha does the same: `styleSettings.sectionReset` defaults to **on**, and a tap while
playing resets the section and leaves the tempo alone (the OM's note: the setting makes
Tap "change the tempo instead"). Turned off, TAP TEMPO always sets the tempo, from the
second tap, averaging the last four. Section Reset is also a command of its own (Launchkey Shift + Play, key `|`) and an
assignable function, Style Section Reset (yahaha's own row: the Genos reaches it only
through TAP TEMPO).

Decisions:
- Decision (#128, owner 2026-09-25: "keep the genos default"): Section Reset defaults to
  on, as on the Genos. With it on, a tap while playing is a reset only: the tapped tempo
  does not apply (the manuals give reset and tempo change as alternatives).
- The bar grid restarts at the tap. A section change queued for a bar line moves to the
  new grid's next bar line; a queued fill to its next beat; a style waiting for the bar
  line to the new grid's next one.
- The section's setup is not sent again (it is the same section); its events from tick 0
  play again, and the section hooks run as for a Main repeating.

## Style Retrigger (#25)

While on, each chord played in a Main restarts the Main at the chord, and its first
`4 / rate` beats (rate 1, 2, 4, 8, 16, 32: a whole note .. a 32nd) loop (RM p.147).

Decisions:
- **The head loops until a section change or Retrigger goes off**, and a new chord
  restarts it. The manual says "a specific length of the first part of the current Style
  is repeated when the chord is played"; the Live Control knob that shortens it while
  it plays reads as a continuous stutter, not a one-shot.
- **A style change ends the loop**: the new style's Main plays on from where it comes in,
  and loops again only from a chord played in it (Retrigger stays on). The loop was of
  the old style's head; carrying it over would stutter a head the player never struck.
- **A chord struck again counts**: letting go of the chord section and striking the same
  chord restarts the head, as a different chord does ("when the chord is played"). The
  input thread publishes a re-struck chord with a new generation once every key the
  chord is read from was up; the Synchro Stop Window times it and Sync Stop restarts
  the band on it too. It is no chord change, though: the Retrigger Rules move no
  sounding note (a Pitch Shift to Root bass stays where it walked).
- **Decision (#107): in AI Full Keyboard, only three notes or more strike the same chord
  again.** A single note or a dyad that fits the chord is melody (the AI reads fewer
  than three notes "based on the previously played chord", RM p.9). So a two-note
  figure in the right hand does not restart the head, time the Synchro Stop Window or
  restart Sync Stop. A dyad that changes the chord still changes it
  (`fingering::restrikes`, checked in `Input::recompute`).
- **The restart is at the chord's instant**, not quantised: the stutter follows the
  player. The bar grid restarts with it (as Section Reset).
- Turning Retrigger off lets the Main play on from where the head is.
- Only Mains retrigger (RM p.147); a Fill, Break, Intro or Ending plays as usual.
- **Changing the length while the head loops** takes effect at once. Shortened past
  where the head is, the loops already missed are skipped, not played back to back: the
  head starts again at the last whole loop (on the grid from the chord) before now, or
  before a change queued inside them, which still comes at its time.
- **Default length: 8** (an eighth note). The Launchkey steps it with Shift + > / Shift +
  Function; the terminal with `{` `}`.

## Controls

| | Terminal | Launchkey | App |
|---|---|---|---|
| Fade In/Out | `F` | page 3 top pad 6; Shift + Stop | Settings › Style; the mirror; a pedal (assignable function Fade In/Out, RM p.142) |
| Section Reset | `\|` (and `t` while playing, with the Tap setting on) | Shift + Play (and Tap while playing, with the Tap setting on) | Settings › Style (Tap setting); the mirror; a pedal (assignable function Style Section Reset) |
| Retrigger on/off | `~` | page 2 bottom pad 8 | Settings › Style; the mirror |
| Retrigger length | `{` `}` | Shift + > / Shift + Function | Settings › Style |
| Ritardando | the Ending key again | the Ending pad again | the Ending pad again |
| Timing, window, fade times | | | Settings › Style |
