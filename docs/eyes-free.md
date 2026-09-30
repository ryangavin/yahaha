# Eyes-free: the v1 release target

Draft, 2026-09-29. Not yet committed; the owner holds it until the mixer/rack lane lands.
This note is the target v1 is aimed at. It says what the release is for, what must be
true when it ships, and the design direction for the one slice that decides whether it
ships: playing without looking at the screen.

## What v1 is

v1 is a signed desktop build (and the iPad build) handed to the owner and a few friends.
It is not a public release: no notarized DMG, no onboarding for strangers, no shipped
content. The owner sets up styles and SoundFonts by hand for each person.

Feature freeze: everything on `develop` today, plus the mixer/rack lane in flight. The
substance is there; the surface is not. v1 is a presentation and reach pass, not a
feature milestone.

**Quality bar:** sounds right to the owner across the corpus. No Genos ground-truth
capture is needed for v1 (the capture kit stays for later).

**Done when:** a friend with a Mac (or iPad), a Launchkey MK4, a folder of styles and a
GM font installs the build and jams for a set without the owner beside them; and the
owner plays a set on it without looking at the screen, using the moves below.

## The problem

Two things happen constantly in a jam: changing sections and changing sounds. Sections
won the fight for the pads (page 1). Sounds lost, and live two pages down (OTS on page 3,
Quick Racks on page 4), so deviating from what the style gives you means paging while
playing. Rebalancing the band, which has the whole Style fader page to itself, is rare
mid-song.

The owner plays this app mostly without looking at the screen, and has no muscle memory
for the layout yet (the rack and layer concepts are new). So the goal is not to match a
layout already in the owner's head. It is to pick a layout worth building muscle memory
on, and live with it.

## The outcome

Eyes-free, without leaving the section pads, the owner can:

1. **Land on one exact sound** for a part. They know the sound by its number; they dial
   it, they don't scroll to it.
2. **Recall a whole rack** in one press: their own saved setups, beyond the style's four
   OTS.
3. **Tweak the rack under their hands and capture it**, so it is there next time,
   without opening the Library.

Each move is one gesture that can be done without looking and repeated identically every
time. The Launchkey display confirms what happened (it already names every control's
action and value). The screen is for setup and looking around the jam, not for the jam.

## The surface budget

Launchkey MK4: 16 pads in two rows, 8 knobs, 9 faders with a button under each, Pad Bank
▲/▼, encoder page ▲/▼, Shift, Play/Stop, Track </>, Scene Launch (>), Function.

What must stay where it is: page 1 (sections and transport), Play/Stop, Track </> (style
step), tempo on Scene/Function. Everything else is negotiable. Under-used now:

- The **Style fader page** (8 faders + 8 buttons): the band's balance and mutes. Rarely
  touched mid-song; it can move behind a modifier or a page and give up its buttons.
- **Panel fader buttons 6 and 7** (plugin reload, Left Hold) and **faders 7–8**, which do
  nothing on the Panel page.
- **Knob pages 3–6** (pan, reverb, chorus, delay) are set-and-forget in a jam.

## Direction: hold-modifiers, and pages the owner can arrange

Two mechanisms, both in the spirit of what the Launchkey already does with Shift:

**Hold-modifiers.** A held button turns the pads (or knobs) into another surface for as
long as it is held, then they snap back. A hold plus a pad is one gesture, drills into
muscle memory, and costs no page slot. Candidates, to be tried rather than decided here:

| Hold | Pads become | Knobs become |
|---|---|---|
| a "Sound" button (e.g. Panel fader button 6) | Quick Racks 1–8 of the current bank (top row), OTS 1–4 + bank −/+ + Store (bottom) | – |
| a part's fader button, held (swap mode; a tap still toggles the part) | – | knob 1 steps that part's sound by number, live, in the Library's order; knobs 2–8 its mix |
| Store (already on page 4) | tap a pad to store the live rack there | – |

(The first draft had the dial on knobs 1–4 under Sound; see "Decisions" for why it moved
to the part's own button.)

Direct dial by number needs the Library to give every sound a stable number the owner can
learn (favourites first, then by category), and the display to show `R1: 23 Rhodes Soft`
as the knob turns. A tap of the same knob (if the MK4 reports encoder touch) or a short
hold could audition without committing; if not, the change is live and that is fine.

**Capture.** "Hold Sound + tap the lit Quick Rack pad" overwrites that rack with the live
rack. "Hold Sound + tap an empty pad" saves the live rack there as a new rack, named from
its sounds. Both confirm on the display. No dialog, no Library.

**Pages the owner can arrange.** Pad pages 2–5 (Chord/Setup, OTS/Parts, Quick Racks,
Multi Pads) become a list the owner can reorder and trim in Settings, and page 1 stays
fixed. That is enough to put Quick Racks on Pad Bank ▼ once, and leaves the layout the
owner's to change as the muscle memory forms. Custom page contents (assigning any
function to any pad) is a later step, only if reordering isn't enough.

Parity rule holds: every one of these has an app control and a tooltip, and the app's
Launchkey mirror shows the held layer while it is held.

## How the work runs

This is a design-and-playtest loop, not a spec to build in one pass, because the owner
has no muscle memory yet and the right layout is the one that survives a week of playing.

| # | Step | Done when |
|---|---|---|
| 0 | The mixer/rack lane lands; `develop` pulls the crate split | `develop` green, owner playtests one set on it |
| 1 | Sound numbers in the Library, shown on the display and in the app | Owner can name a sound's number from memory after a session |
| 2 | Hold-Sound layer, first cut: Quick Racks and OTS on the pads under a held button | Owner recalls any saved rack from page 1 without looking |
| 3 | Swap mode: hold a part's fader button, knob 1 dials its sound | Owner lands on a named sound for one part in one gesture |
| 4 | Capture: hold + tap stores/overwrites a Quick Rack | Owner tweaks and stores a rack mid-jam, no screen |
| 5 | Racks and Setup pages; pad page order in Settings | Owner puts the pages in an order they like and it survives restart |
| 6 | A week of sets on it; one round of changes from what annoyed | Owner keeps the layout, or changes it and keeps that |
| 7 | Visual polish pass on the screens the jam uses (Stage, Mixer, Library), from a list the owner writes after step 6 | The list is empty |
| 8 | Release: `develop` → `main` (merge commit), tag `v0.1.0`, signed desktop and iPad builds handed out | Two friends jam a set each without the owner beside them |

Steps 1–5 are one lane at a time on the Launchkey map (`launchkey`, `controllers`, the
API contract), after the mixer/rack lane, since they share the rack commands. Step 7 is
web-only lanes and can run beside 6.

## Decisions (owner, 2026-09-29)

Step 0 is done: the mixer/strips lanes landed on `develop` (#467, #469, #470). The owner
still owes one playtest set on it; it doesn't block the contract PR.

- **"Sound" is Panel fader button 6** (plugin reload moves to the app only). It's a fader
  button, so the hold works while the Panel fader page is up; the contract makes the hold
  page-independent (the input thread reads the button, not the fader page).
- **Swap mode replaces the dial-under-hold-Sound.** Hold the fader button under a part
  (Panel page, buttons 1–4; a tap still toggles the part): knob 1 steps that part's sound
  by number, knobs 2–8 are its mix, the display shows `R1: 23 Rhodes Soft`, and every
  step sounds live. Release commits. Dialling back is the cancel; no revert gesture. A
  hold that turned no knob is a tap (the part toggles on release). Knob touch (channel 15,
  which the MK4 does send) stays unused.
- **Sound numbers:** favourites 1–n, then the Library's category order. Adding a favourite
  renumbers what follows; the display and the Library show the number.
- **OTS and Quick Racks share one pad page, "Racks":** top row Quick Racks 1–8 of the bank
  on view, bottom row OTS 1–4, bank −/+, Store, spare. Hold Sound shows this page on the
  pads from anywhere. Capture: hold Sound + tap the lit rack pad overwrites it, + tap an
  empty pad stores a new rack named from its sounds. Page 3's part on/off pads go (the
  Panel fader buttons have them).
- **A "Setup" pad page, last, holds the set-and-forget switches, each saved in
  settings:** fingering type 1–7, Upper/Lower, OTS Link, Stop ACMP mode. The Chord page
  keeps the mid-song ones: Manual Bass, Stop ACMP, Split ±, Transpose ±/reset, Retrigger.
  Pages become: 1 Sections (fixed), Racks, Chord, Multi Pads, Setup; pages 2–5 reorderable
  in Settings.
- **Page 1 stays exactly as it is.**

## Swarm

One contract PR, then four lanes with disjoint files, then a week of sets. Lane PRs
target `develop`, get one reviewer each on open, and merge in any order.

**Contract PR** (one lane lead, alone, first; the lanes branch from `develop` after it
lands). Green on its own, with stubs:

- The API: `Layer` (None / Sound / Swap(part)) in `AppState`'s Launchkey surface; commands
  `SwapSound { part, step }`, `StoreRack { slot }`, `SetPadPageOrder`; `number` on every
  Library sound; the Setup switches as saved settings. `docs/app-api.md`, `EVERY_CMD`,
  `tests/fixtures/state.json`, both mocks, `types.ts`, tooltips, actions, `controls.md`.
- The seams the lanes fill: `launchkey::Page` becomes `Sections, Racks, Chord, MultiPads,
  Setup`, with the per-page pad tables (`pad_action`, `pad_leds`, `looks`) split into
  `crates/yahaha-engine/src/launchkey/pages/{sections,racks,chord,multipads,setup}.rs`
  (a mechanical move; Racks and Setup start as the old OtsParts/QuickRacks and Chord
  content); `pad_action(page, layer, note)` takes the layer; `Shared.layer`; the page
  order comes from settings (page 1 fixed) and the Pad Bank buttons and the app mirror
  walk that list; the input thread's fader-button press/release timing and the Sound
  button hold set the layer and call `live::swap` / `live::sound_hold` stubs;
  `sound_library::{number_of, at_number}` stubbed as library index order.
- This note, committed as `docs/eyes-free.md`.

**Lanes** (each: own worktree, one PR, one reviewer; Rust first builds staggered):

| Lane | Done when | Owns |
|---|---|---|
| A Sound numbers | Numbers are favourites-first then by category, shown in the Library and the Sounds panel, stable across a session; `at_number` round-trips. | `src/session/sound_library*.rs`, `src/session/library.rs`, `app/src/panels/library/*`, `app/src/panels/sounds/*` |
| B Swap mode | Hold part button + knob 1 steps the sound live with the display line; knobs 2–8 edit its mix; release commits; a plain tap still toggles; no-alloc test on the input thread passes. | `src/live/swap.rs`, `src/session/part_sound.rs`, `src/session/knobs.rs`, `src/session/display.rs`, `tests/it/*swap*` |
| C Racks page, Sound hold, capture | Racks page as decided; hold Sound shows it from any page (the state's `surface.layer` carries it; the mirror is the Stage-layout swarm's); store/overwrite by hold + tap, confirmed on the display; OTS Link keeps working from Setup. | `launchkey/pages/racks.rs`, `src/live/sound_hold.rs`, `src/session/{quick_racks,style_racks,ots,pads,leds}.rs`, `src/racks/*` |
| D Setup page and page order | Setup page with the four switches, each saved and restored on restart; Chord page slimmed; page order editable in Settings › Launchkey and honoured by Pad Bank ▲/▼ and Tab after restart. | `launchkey/pages/{chord,setup}.rs`, `src/session/settings.rs`, `src/session/surface.rs`, `app/src/panels/settings/*` |

Hotspot rule: `src/live.rs` and `crates/yahaha-engine/src/launchkey.rs` belong to the
contract; a lane that needs a change there asks for a small contract follow-up PR.

The Stage-layout swarm runs in parallel and owns `app/src/App.svelte` and every file
under `app/src/panels/{launchkey,mixer,knobracks}`; its lanes render `surface.layer` and
`StoreRack { slot }` from our contract. Its web-only contract lands before ours; each
contract appends its own block to `tooltips.ts`, `actions.ts` and `controls.md`. The
orchestrator keeps an integration worktree (`develop` + all four branches) and runs the
core suite and `--test it` there as lanes push; fixes go to the owning lane.

After the four merge: step 6 (a week of sets, one change round), step 7 (web-only polish
lanes from the owner's list), step 8 (release).
