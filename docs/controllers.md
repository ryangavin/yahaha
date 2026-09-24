# Controllers: pedals, wheels, assignable functions

Issue #34. Code: `src/controllers.rs` (the model and the assignable-function table),
`src/api/controllers.rs`, `src/session/controllers.rs`, the input thread in `src/live.rs`
(`Input::key_msg_from`), the engine loop's sync (`EngineLoop::step`). The app's page is
Settings → Pedals (`app/src/panels/settings/PedalsPage.svelte`).

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
| Voice | Sustain, Sostenuto, Soft (switch); Modulation, Pitch Bend (continuous); Kbd Harmony/Arpeggio On/Off, Arpeggio Hold (switch, RM p.141) |
| Style | Start/Stop, Sync Start, Sync Stop, Intro 1-3, Main A-D, Fill Down, Fill Self, Fill Break, Fill Up, Ending 1-3, Auto Fill, Stop Acmp, Fingered/Fingered On Bass |
| OTS | OTS Link, OTS 1-4, OTS +, OTS − |
| Registration | Registration Bank +, Registration Bank − (the REGIST BANK [+]/[−] buttons: they load the next or previous bank file, see docs/registration.md). Registration Sequence +/− is not pedal-assignable on the Genos (a pedal drives it through Pedal Control on the Registration Sequence display), so it is not in the table. |
| Overall | Tempo +, Tempo −, Tap Tempo, Transpose +, Transpose − (Master transpose, as the TRANSPOSE buttons), Right 1-3 and Left On/Off |

Two rows are yahaha's, not the Genos's:
- **Stop Acmp On/Off.** The Genos list has Acmp On/Off (the [ACMP] button), which yahaha
  doesn't have; Stop Acmp is the nearest yahaha control.
- **Right 1/2/3 and Left On/Off, one row each.** The Genos has a single Part On/Off that
  switches the parts chosen for it together; one row per part does the same one part at a
  time without a second setting.

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

The two threads flush their own output buffers at different times, so only one may send
the parts' controllers at a time (`Controllers::claim` before the sync, `release` after
the flush). Neither waits: a thread that finds the other holding it leaves a note, and the
holder syncs and flushes once more before it lets go. Without this, a change one thread
sent could reach the synth after a newer one the other flushed first, leaving a pedal or
bend stuck until that controller moved again.

## Pedals

- **Hold A / Hold B follow the pedal's position.** Picking Hold B with the pedal up turns
  the switch on at once (RM p.139: Hold B "turns the function off and keeps it inactive
  while holding down"); turning Hold A into Hold B while the pedal is held turns it off,
  and it comes on at the release. Toggle flips on each press.
- **More than one keyboard.** Each keyboard source keeps its own pedal edges. A pedal is
  down while any keyboard holds it (two sustain pedals on CC 64: Sustain stays on until
  both are up); a press on any keyboard fires a trigger or flips a Toggle.
- **CCs a pedal can't use**: 0 and 32 (bank select) and 7 (volume) belong to the parts and
  never reach a pedal; 1 is the modulation wheel; 6 and 38 (data entry), 98-101 ((N)RPN)
  and 120-127 (channel mode, including 121 Reset All Controllers) are not pedals. Learn
  skips them and `setPedal` refuses them with a message.

## Safety

- **Panic**: the engine sends pedal off (CC 64/66/67 = 0), modulation 0 and bend centre to
  every keyboard part, then All Notes Off on all 16 channels. The pedal counts as up until
  it is pressed again, whatever it physically is: the next press is a press (the input
  thread forgets the pedal's last edge), and the Pedals page lamp goes out.
- **A keyboard unplugged** (or deselected in Settings → MIDI) with keys held, **or with a
  pedal or wheel it moved** (`Controllers::touched`): the same reset, then All Notes Off on
  the keyboard parts. Before, a pedal left down with no keys held stayed down.
- **Reset All Controllers (CC 121)** from a keyboard: the pedal switches and wheels go
  back to neutral on the parts, then the CC goes on to all four parts (it also resets
  expression, pressure and the rest on the synth).
- **A pedal given another function or CC** (Settings, or Learn) while its switch is on
  (held, or latched by Toggle) or mid-sweep: what the old setup drove is released first
  (the switch goes off unless another pedal on the same switch is held; modulation to 0,
  bend to centre). The pedal then counts as up until it is pressed again. After a Panic or
  reset a Hold B switch stays off until its pedal is next pressed and released.
- **Style and section changes, Start and Stop** send nothing on channels 1-4 (tested):
  a held pedal and a bend carry across them.
- A part switched **off** under a held pedal or a bend is released and centred; switched
  **on**, it gets them.

## Decisions

- **Sustain reaches Left by default.** Decision: the sustain pedal holds Right 1-3 and
  Left (each can be taken out in Settings → Pedals), because the Genos RM describes
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
- **Kbd Harmony/Arpeggio On/Off and Arpeggio Hold** (RM p.141) are switches the control
  side keeps (`harmonyArp.on`, `harmonyArp.arp.pedalHold`: the pedal function, apart from
  the Hold setting `harmonyArp.arp.hold`; docs/arpeggio.md), so the pedal asks for them: a Toggle
  pedal switches them on each press (`Fire::control`); a Hold A or Hold B pedal sets them on
  or off as it goes down and up (`Fire::set`, `Action::AssignSet`, `api::function_set`).
  Like the pedal switches, a new setup puts a Hold pedal's switch where the pedal is (Hold
  B picked with the pedal up turns it on), and a Hold pedal given another function lets go
  of the switch it was keeping on (`controllers::control_switch_sets`). Decision: unlike
  Sustain, two Hold pedals on the same one of these don't hold it for each other; the last
  edge wins, because the panel button and the TUI key also switch it and the pedals don't
  own it. A reset (Panic, a keyboard unplugged) lets the pedals go without an edge, so it
  tells the control side (`Controllers::take_reset_releases`, `Control::pump_pedal_releases`),
  which turns off what a Hold pedal was keeping on (Hold A down, Hold B up:
  `controllers::reset_release`). As with the pedal switches, a Hold B switch then stays off
  until its pedal is next pressed and released: a reset never turns anything on. A Toggle
  pedal's switch is left as it is, as the panel button's would be.
- **Transpose +/− is Master transpose.** RM p.144 makes it "the same as the TRANSPOSE
  [+]/[−] buttons", and those transpose the overall pitch (OM p.61), which is yahaha's Master
  transpose (the Launchkey's KBD TR pads stay Keyboard transpose).
- **Not done**: Left Hold, Glide, Portamento, Mono/Poly, Pedal Wah, Organ Rotary, the
  Assignable buttons A-F (the Launchkey has none free), Joystick Hold, a Volume pedal.
