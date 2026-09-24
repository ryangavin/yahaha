# Fills, Stop Accompaniment, Change Behavior, OTS Link timing

Genos behaviour yahaha implements for issues #24, #26, #27 and #28, and the choices made
where the manuals leave a gap. The spec is [genos-features.md](genos-features.md) §2–§4 and
§C.8; the commands and state are in [app-api.md](app-api.md).

## Fill Up / Down / Self / Break (#24)

The Genos has these as assignable functions (RM p.142), for pedals, Assignable buttons or
external MIDI. yahaha has them as commands (`fillUp`, `fillDown`, `fillSelf`, `fillBreak`),
keys (`A` `S` `G` in the terminal and the app: Shift + the left-hand pair for Down / Up,
Shift + the Break key for Self) and buttons in Settings › Style. Half Bar Fill In is `N`.
(The first draft's `<` `>` `.` `H` belong to the Playlist, the metronome and Arp Hold on
sibling branches.)

| Function | Genos (RM p.142) | yahaha |
|---|---|---|
| Fill Down | a fill, then the Main on the immediate left | the fill of that Main, then that Main |
| Fill Up | a fill, then the Main on the immediate right | the same, to the right |
| Fill Self | a fill | the selected Main's own fill, as pressing that Main again |
| Fill Break | a break | `break` |

- A fill function is a Main press with the fill forced, Auto Fill or not
  (`Engine::press_main`, `src/engine/fills.rs`); the fill starts at the next beat, as any fill.
- **Decision:** the fill played on the way to another Main is that Main's fill, because
  that is what a Main press with Auto Fill already plays in yahaha (Fill In BB into Main B).
- **Decision:** Fill Up / Down skip Mains the style lacks (A → C when B is missing), because
  "the Main on the right" of a missing button is the next one that plays; at the end of the
  row (Fill Up on D, Fill Down on A) they play that Main's own fill and stay, because the
  performer asked for a fill and there is nowhere to go. No wrap.
- **Decision:** stopped, Fill Up / Down select the Main the band starts on (a Main press
  while stopped does the same); Fill Self and Fill Break do nothing.
- **Decision:** Fill Self plays the fill of the *selected* Main, which is the one playing
  unless another Main is already queued; then it is the queued Main's fill, and the band
  still goes on to that Main. RM p.142 says only "plays a fill-in"; this keeps Fill Self
  a Main press with the fill forced, so it never cancels the Main the player chose.
- During an Intro, fill or break, a fill function selects the Main that follows, as a Main
  press does. During an Ending it queues the Main at the next bar.

## Half Bar Fill In (#24)

RM p.142: "While this function is on, changing sections of a Style at the first beat of the
current section starts the next section from the middle with an automatic fill-in."

- **Decision:** "the first beat" is the first beat of the bar playing, and "from the middle"
  is a fill that starts at the middle of that bar: a Main pressed (or a fill function used)
  during beat 1 of a bar while a Main plays queues that Main's fill at the half bar, and the
  new Main starts at the next bar line. This is the reading that makes the function useful
  (a half-bar pickup into the next section) and matches how the fills already align to the
  bar (the fill plays its own second half). Pressed after beat 1, nothing changes.
- The fill is automatic: it plays with Auto Fill off.
- The middle is half the bar's notated beats, rounded down (`Prepared::half_bar`, from the
  time signature): beat 3 in 4/4, beat 2 in 3/4, the 4th eighth (the true middle) in 6/8
  and 12/8.
- Break and Endings are not affected. Timing goes through `Engine::change_point`
  (`Change::HalfBar`), the one section-timing policy.
- Control type Toggle / Hold A / Hold B is a controller setting (#34): the engine has
  `setHalfBarFill` for Hold types and `toggleHalfBarFill` for Toggle.

## Stop Accompaniment (#27)

RM p.11 and genos-features §C.8: with ACMP on, Sync Start off and the Style stopped, a chord
in the chord section is recognised and shown, and the Stop ACMP setting decides whether it
sounds: Off, Style (the style's Pad and Bass voices) or Fixed (fixed Pad and Bass voices,
whatever the style).

- `setStopAcmp { mode }` sets it; `toggleStopAcmp` (the `h` key, the Launchkey Stop ACMP pad)
  switches between Off and the mode last on, Style at first. `transport.stopAcmp` stays the
  "sounds" flag; `transport.stopAcmpMode` is the mode.
- Default Off (yahaha's behaviour so far; the manual gives no factory default).
- **Decision:** Fixed sounds on GM Finger Bass (program 34) and Warm Pad (program 90), bank
  0, on the style's Bass and Pad channels (11 and 14), because every MIDI channel already has
  a part (1–4 keyboard, 5–8 Multi Pads, 9–16 style) and the Genos voices are not documented.
  Their volume is the Bass and Pad parts' CC7, so the mixer stays the only level. The fixed
  program changes go out before the first note; the style's own voices go back (only what
  differs, `reapply_init`) when the mode changes back to Style or Off, and with the whole
  setup when the band starts or a style loads.

## Change Behavior (#26)

RM p.12–13, Style Setting › Change Behavior: what choosing another style does.

| Setting | Lock | Hold | Reset |
|---|---|---|---|
| Tempo | keep the tempo | keep it while playing; the new style's when stopped | the new style's |
| Part On/Off | keep the style part mutes | keep them while playing; all on when stopped | all on |

Section Set: Off (keep the Main) or a Main A–D a style chosen **while stopped** starts on,
the nearest Main the style has when it lacks that one (RM p.12: D missing → C).

- Commands: `setTempoChange`, `setPartsChange`, `setSectionSet`, and the assignable
  "Style Tempo Lock/Reset" and "Style Tempo Hold/Reset" (`toggleStyleTempoLock`,
  `toggleStyleTempoHold`, RM p.144). State: `styleChange`.
- A style change while playing takes over at the next bar line (as before); "playing" in
  the rules is the band's state at that bar line. A change queued while playing that lands
  because the band stopped first follows the stopped rules.
- **Decision:** defaults Tempo Hold, Part On/Off Hold, Section Set Off. The manual gives no
  factory defaults (genos-features §G.1). Tempo Hold and Section Set Off are what yahaha did
  so far. Part On/Off Hold changes one thing: a style chosen while stopped now starts with
  every part on (before, mutes carried over); the manual says Hold and Reset "turn all
  channels on for a newly loaded Style", and a fresh style with parts silently muted is the
  more surprising result.
- Change Behavior is a System setting on the Genos (DL p.91), not stored in Registration;
  yahaha keeps it for the session.

## OTS Link timing and OTS → Sync Start (#28)

- OTS Link Timing (RM p.11): **Immediate**: the OTS is recalled as the Main button is
  pressed. **At Main Section Change**: it is recalled "at the next measure", when the Main
  actually changes.
- `setOtsLinkTiming { timing: "immediate" | "mainChange" }`; state `ots.linkTiming`. Default
  **At Main Section Change**.
- **Decision (owner preference):** the default is At Main Section Change, not Immediate
  ("Real Time"). The manual gives no default; if the Genos's factory setting is Immediate,
  this deviates from it on purpose. The owner found that Immediate changes the right-hand
  sound under them the moment they press the next Main while soloing, before the band
  gets there. At Main Section Change recalls the pressed Main's OTS exactly when that Main
  starts (its change point from `Engine::change_point`, which follows Section Change
  Timing; after its fill with Auto Fill), never while the old section still plays.
  Immediate stays available in Settings › Style.
- **Style changes:** a style chosen while the band plays takes over at its change point
  (the next bar line, or the next beat with Section Change Timing Immediate; a style chosen
  during an Ending waits for the Ending to finish, #94). With OTS Link on, the new style's
  OTS reaches the keyboard parts when it takes over, never on selection, under both
  timings (the new style's OTS belongs to the new style, which isn't playing yet).
- **Decision:** At Main Section Change recalls when the Main starts playing, so with Auto
  Fill (or a fill function) it is after the fill, at the bar line where the Main begins,
  because that is the section change the name refers to and the moment the band's sound
  changes. An Intro, fill or break in between changes nothing. Stopped, there is no section
  change to wait for: both timings recall at the press.
- **Decision:** stopping the band is not a Main change. With At Main Section Change, a Main
  pressed but not yet playing when the band stops keeps waiting: its OTS is recalled when
  the band starts on it, or at once if a Main is pressed while stopped (the same Main
  again, or Fill Self, included: a press is a press). Recalling it at the stop would arm
  Sync Start (below), and the next chord would restart the band the player just stopped.
- **Decision:** switching OTS Link Timing is a settings change, not a Main change. A Main
  waiting after a stop keeps waiting under either timing (Immediate recalls it as the band
  starts), so the switch recalls nothing and does not arm Sync Start.
- **Decision:** for a Main the style lacks (it plays the nearest one), both timings recall
  the pressed button's OTS, because OTS 1–4 belong to the Main A–D buttons. (Genos styles
  have all four Mains, so the manual never meets this.)
- **OTS recall turns Sync Start on** (OM p.47; the DL OTS column stores "ACMP on, Sync Start
  on"). yahaha has no ACMP off, so a recall arms Sync Start while the band is stopped; the
  next chord starts it. **Decision:** this applies to every recall (the OTS buttons, OTS
  Link, a style change with Link on), because they are all an OTS recall on the Genos.
  While the band plays nothing changes: Sync Start pressed while playing would stop it.

## Where it lives

| What | Where |
|---|---|
| Fill functions, Half Bar Fill | `src/engine/fills.rs`, `Change::HalfBar` in `src/engine/sections.rs` |
| Stop ACMP modes, Change Behavior | `src/engine/change_rules.rs` (engine), `src/session/style_change.rs`, `src/api/style_change.rs` |
| OTS Link timing, OTS → Sync Start | `src/session/ots.rs` (`pump_ots_link`, `recall_ots`), `live::Cmd::SyncStartOn` |
| Tests | `src/engine/rules_tests.rs`, `src/session/style_change.rs`, `tests/engine_no_alloc.rs` |
| App | Settings › Style (`StylePage.svelte`, `FillButtons.svelte`, `ChangeBehavior.svelte`) |
