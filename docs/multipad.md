# Multi Pads

Genos Multi Pads (docs/genos-features.md §7; OM p.59, p.74–75; RM p.12, p.64–68), wired
into the engine and the app. The bank parser and the player core are in `src/multipad/`
(`file.rs`, `player.rs`); this page is about how they play live.

## Where it lives

| Layer | Code |
|---|---|
| Bank file parser, player core | `src/multipad/file.rs`, `player.rs` (pure, no threads) |
| Bank list (`.pad` files in the style folders) | `src/multipad/library.rs` |
| Synthetic banks (tests, dev mode, `yahaha pad --demo`) | `src/multipad/synthetic.rs` |
| Engine: the player, its clock, the hooks | `src/engine/multipad.rs` (`hooks::Features::pads`) |
| Engine thread wiring | `live::Cmd::MultiPad`, `live::PadBank` ring in, old players out |
| Commands, state | `src/api/multipad.rs`, `src/session/multipad.rs` |
| Built-in synth voices (ch 5–8) | `synth::translate` |
| App panel | `app/src/panels/multipad/` |

A bank is parsed and its `MultiPadPlayer` built on the control side (`loadMultiPad`), sent
to the engine thread through a ring as a `Box`, swapped in there (the old bank's notes end),
and the old player comes back through another ring to be freed on the control side. The
engine thread never allocates or frees for pads (`tests/multipad_no_alloc.rs`).

## Behaviour

- **Output.** Pads 1–4 play on MIDI channels 5–8, the Genos Song convention (keyboard parts
  1–4, Multi Pads 5–8, Style 9–16). Pads are recorded on channels 1–4 in the file; the player
  re-channels every event, the voice setup (bank select, program change, controllers)
  included, so each pad plays its own voice. The built-in synth plays channels 5–8 on its band
  synthesizer: a Yamaha drum-kit bank (MSB 126/127) selects the SoundFont's drum bank, any
  other voice its GM bank (the same fallback as the Style parts). A pad with no voice setup
  plays the channel's current voice (GM Grand Piano at start).
- **Volume.** A pad's level is its own CC7 and velocities, sent unchanged (the mixer
  principle). There is no Multi Pad fader yet.
- **Timing.** Stopped, a pad starts at once. While the band plays, a press starts at the top
  of the next measure (at once exactly on a bar line). Pressing a playing pad restarts it
  from the top (at the next measure while the band plays). Up to four pads at once.
- **Clock.** Pads run on a clock of their own at a fixed 1920 ppq (the Tyros pad resolution),
  moving at the band's tempo, stopped or not. Why not the style's ticks: the engine's tick
  domain is the style's ppq, which a style change can change under a playing pad (the #78
  review). A tempo change re-anchors the pad clock where it is, so a pad keeps its place and
  changes speed. A next-measure start is the style's next bar line mapped onto the pad clock.
  A repeating pad loops on its own length, so a one-bar 4/4 pad stays on the bar at a steady
  tempo.
- **Chord Match** follows the chord the style follows (the chord section after Keyboard
  transpose). A chord played before or after pressing the pad both work; notes already
  sounding keep their pitch, notes after a chord change follow it. With no chord yet, or
  Chord Cancel, pads play as written. Transposition is the style engine's NTR/NTT machinery
  with the pad's CASM rule, or a CM7 Melody/Root Trans rule for pads without one (the
  phrase-authoring rule, RM p.65).
- **Synchro Start** (SELECT + pad): arms a pad (lamp flashing red). A chord played in the
  chord section, or the band starting, starts every armed pad; while the band plays, at the
  next measure. With pads in standby, pressing any one of them starts them all (OM p.75);
  pressing a pad that is not in standby starts only that pad. Arming again, or STOP,
  disarms.
- **STOP** stops every pad and cancels standby; **STOP + pad** stops one pad.
- **Multi Pad Synchro Stop** (Style Setting, RM p.12): *Style Stop* stops repeating pads
  when the band stops (any stop: START/STOP, an Ending finishing, Sync Stop); *Style Ending*
  stops them when an Ending section starts. One-shot pads always play out. A press still
  waiting for the band's bar line when the band stops starts at once (stopped, pads start
  at once) instead of later at the old bar line.
- **Controllers.** A pad that bent, modulated or held the sustain pedal on its channel
  re-centres the bend and releases the wheel and pedal there when it stops (STOP, STOP +
  pad, a bank swap, Synchro Stop), restarts, or ends as a one-shot, as the style parts do
  when they stop.
- **Master transpose** moves the pads' notes (after Chord Match), except drum-kit pads
  (bank MSB 126/127 before the first note), as it moves the style (RM p.41). A note ends on
  the key it sounded on, so a transpose change while a pad plays leaves no note hanging.
- **Panic** stops every pad and resets the bend, modulation and sustain pedal on ch 5-8.
- **Bank list.** Every `.pad` file under the style folders (`library.roots`), found by the
  same walk as the styles and refreshed by `rescanLibrary` (on a thread of its own).
  `loadMultiPadPath` loads any file (Registration can use it); a file outside the library
  joins the list only once it has loaded.

## Decisions (where the manuals are silent)

- **Decision: pads keep a fixed-PPQ clock (1920) of their own**, because the style's tick
  resolution changes with the style and pads play while the band is stopped; rescaling on
  every style change would round a playing pad's position each time.
- **Decision: Chord Match follows the style's chord in every case**, because yahaha has no
  ACMP switch: the chord section is always read, so the Genos "ACMP off: the LEFT section's
  chord" source is the same chord here. When an ACMP switch arrives, `Engine::pads_on_chord`
  and `process_pads` are where the source changes.
- **Decision: Synchro Start fires on the chord section's chord**, the Genos ACMP-on
  trigger (a key press would be the ACMP-off one). A chord that recognises the same chord
  again sends no new chord to the engine, so it does not fire standby.
- **Decision: Multi Pad Synchro Stop defaults to Style Stop on, Style Ending off.** The
  manual gives no defaults; the Owner's Manual says START/STOP "also stops" the pads (Style
  Stop on), and repeating pads playing on through an Ending until the band stops is the
  behaviour before firmware 1.20 added the setting.
- **Decision: a pad's drum-kit test is its bank MSB (126/127) before its first note**, fixed
  when the bank loads, because pads carry their own voice setup; the style engine's `kit`
  table is per style part and does not cover ch 5-8.
- **Decision: pads that don't parse fail the load** and keep the loaded bank; the message
  says why.

## Launchkey (proposal, not wired)

The Launchkey has no spare buttons for four more pads and STOP/SELECT on the pages in use.
Proposal: a pad page of its own, **page 5 "Multi Pads"** (page 4 is Registration):

```
  top row     Pad 1   Pad 2   Pad 3   Pad 4   | STOP    Repeat*  Chord*  Bank ▲
  bottom row  Arm 1   Arm 2   Arm 3   Arm 4   | Stop 1  Stop 2   Stop 3  Stop 4
```

The pads light as the Genos lamps (blue = data, red = playing, flashing red = armed, dim
white = waiting for the bar); STOP lights while anything plays. `Repeat*`/`Chord*` toggle the
last pressed pad's flags; `Bank ▲` steps through `multiPad.banks`. The actions would go
straight to the engine from the MIDI thread (like the section pads), so a press is not
delayed by the control thread. Holding Shift on page 1 could also turn the bottom row into
Pad 1–4 for one-handed use while playing sections.

## Keys

Terminal UI: `Z X C V` (Shift + z x c v) press pads 1–4, `B` (Shift + b) is STOP. The app's
Multi Pad panel has every control (see its tooltips).

## Testing without real banks

No `.pad` file from an instrument is in the corpus. `src/multipad/synthetic.rs` writes
banks in the documented Tyros layout from original phrases; `yahaha pad --demo [out.pad]`
writes the demo bank (Shaker Loop, Rise Arp, Bass Riff, Brass Hit) to try it live, and
`yahaha pad <file>` prints what the parser makes of a real one.
