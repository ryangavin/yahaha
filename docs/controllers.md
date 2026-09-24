# Controllers: pedals, wheels, assignable functions

Issue #34. Code: `src/controllers.rs` (the model and the assignable-function table),
`src/api/controllers.rs`, `src/session/controllers.rs`, the input thread in `src/live.rs`
(`Input::key_msg_from`), the engine loop's sync (`EngineLoop::step`). The app's page is
Settings → Controllers (`app/src/panels/settings/ControllersPage.svelte`).

## What the Genos does (and yahaha follows)

| Genos | Ref | yahaha |
|---|---|---|
| Three foot-pedal jacks. Defaults: 1 Sustain, 2 ART.1, 3 Volume. | OM p.114; RM p.139 | Three pedals. Each listens for one CC from the keyboards. Defaults: 1 = CC 64 Sustain, 2 = CC 66 Sostenuto, 3 = CC 67 Soft (see Decisions). |
| Polarity can be reversed. | RM p.138 | `reverse` per pedal. |
| Control Type: Toggle, Hold A (on while held), Hold B (off while held), for the functions marked "Control Type". | RM p.139 | `controlType` for Sustain, Sostenuto and Soft. Triggers (Start/Stop, sections, ...) fire on the press. |
| Range for continuous functions: Full, Upper, Lower. | RM p.139 | `range` for a Pitch Bend pedal. |
| Sustain: "all notes played on the keyboard have a longer sustain. Releasing the pedal immediately stops (dampens) any sustained notes." Which parts a function affects is set at the bottom of the Assignable display. | RM p.139 | CC 64 on each keyboard part the pedal reaches (per part), when the part is on. |
| Pitch Bend Range, 0-12 semitones per keyboard part (Left, Right 1/2/3), shared by every pitch-bend controller. | RM p.140, OM p.70 | `setBendRange`, default 2, sent as RPN 0 on the part's channel. |
| Joystick X: pitch bend on all parts. Y: modulation on Right 1-3 by default. | OM p.70 | The Launchkey's wheels: pitch bend reaches all four parts, modulation Right 1-3, by default; per part. |
| The live-play assignable functions (Voice, Style, OTS, Registration, Overall). | RM p.139-144 | The table below, as far as yahaha has the feature. |

## Assignable functions

The table is `controllers::FUNCTIONS` (Rust) and `app/src/lib/api/assignable-functions.json`
(the app; a test keeps them equal). Each row: `id` (the wire name), `name`, `category`,
`kind` (`switch`: Control Type applies; `trigger`: fires on the press; `continuous`: follows
an expression pedal), `available`.

| Category | Functions |
|---|---|
| Voice | Sustain, Sostenuto, Soft (switch); Modulation, Pitch Bend (continuous) |
| Style | Start/Stop, Sync Start, Sync Stop, Intro 1-3, Main A-D, Fill Down, Fill Self, Fill Break, Fill Up, Ending 1-3, Auto Fill, Stop Acmp, Fingered/Fingered On Bass |
| OTS | OTS Link, OTS 1-4, OTS +, OTS − |
| Registration | Registration +, Registration − (not available yet: yahaha has no Registration Memory; a pedal can hold them and says so when pressed) |
| Overall | Tempo +, Tempo −, Tap Tempo, Transpose +, Transpose −, Right 1-3 and Left On/Off |

Engine buttons (sections, Start/Stop, tempo) go from the input thread straight to the
engine, as a Launchkey button does; the rest go to the control side as a Launchkey action
(`Action::Assign`) and run as the `AppCmd` they stand for. `triggerFunction` runs any of
them from software.

Adding a function: a variant at the end of `controllers::Function`, a row at the end of
`FUNCTIONS`, its `effect()` (or a `run_function` arm on the control side), then
`YAHAHA_WRITE_FIXTURES=1 cargo test the_app_table` to rewrite the app's JSON.

## What reaches each part

A part's channel always holds what applies to it now: the pedal switches, the modulation
and the bend if the part sounds and the controller reaches it, else released, 0 and centre.
`Controllers::sync` sends only the difference from what the channel was last sent. The
input thread syncs right after the pedal or wheel message (so a pedal and the notes in the
same packet keep their order); the engine thread syncs on every wake (a part switched on
or off, a setting changed).

## Safety

- **Panic**: the engine sends pedal off (CC 64/66/67 = 0), modulation 0 and bend centre to
  every keyboard part, then All Notes Off on all 16 channels. The pedal counts as up until
  it is pressed again, whatever it physically is.
- **A keyboard unplugged** (or deselected in Settings → MIDI) with keys held, **or with a
  pedal or wheel it moved** (`Controllers::touched`): the same reset, then All Notes Off on
  the keyboard parts. Before, a pedal left down with no keys held stayed down.
- **Reset All Controllers (CC 121)** from a keyboard: the pedal switches and wheels go
  back to neutral on the parts (the CC itself is not passed on, so what yahaha thinks each
  channel holds stays true).
- **Style and section changes, Start and Stop** send nothing on channels 1-4 (tested):
  a held pedal and a bend carry across them.
- A part switched **off** under a held pedal or a bend is released and centred; switched
  **on**, it gets them.

## Decisions

- **Sustain reaches Left by default.** Decision: the sustain pedal holds Right 1-3 and
  Left (each can be taken out in Settings → Controllers), because the Genos RM describes
  Sustain as affecting "all notes played on the keyboard" and makes the parts a per-function
  setting; the panel SUSTAIN button (Right 1-3 only) and LEFT HOLD are different features.
  Under Manual Bass, Left plays the Style's bass voice, and the pedal holds it too when
  Left is chosen.
- **A part must be on to be sustained.** Decision: the pedal reaches only the parts that
  sound (`Parts::sounding_mask`); a part switched off is released at once, one switched on
  joins the held pedal, because a switched-off part must not keep ringing and must never
  come back with a stuck pedal.
- **Default pedals are the GM pedals.** Decision: pedals listen for CC 64, 66 and 67 as
  Sustain, Sostenuto and Soft by default, instead of the Genos jack defaults (ART.1 and
  Volume), because yahaha has no Super Articulation and the part volume is only its CC 7
  (the mixer principle: no pedal Volume function). A CC 64/66/67 that no pedal listens for
  still works as its GM pedal through the parts model.
- **Modulation defaults to Right 1-3**, pitch bend to all four parts, as the Genos
  joystick (OM p.70).
- **Pitch Bend Range defaults to 2** semitones (GM and the Genos default).
- **Fill Up at Main D / Fill Down at Main A** play the fill of the Main at the end and stay
  there (the Genos has no Main to go to); stopped, Fill Up/Down select the next Main.
- **Registration +/−** are in the table but unavailable until Registration Memory exists.
- **Not done**: Left Hold, Glide, Portamento, Mono/Poly, Pedal Wah, Organ Rotary, the
  Assignable buttons A-F (the Launchkey has none free), Joystick Hold, a Volume pedal.
