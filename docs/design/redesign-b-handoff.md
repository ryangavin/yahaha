# App redesign, direction B ("Maschine colour"): handoff

Status 2026-09-26: **the owner picked direction B** and wants to keep refining it. Nothing is built yet; everything here is wireframes. Read this, open the canvas, then ask the owner what to refine.

- **Canvas (private to the owner):** https://claude.ai/artifact/U3wyJmzqbr2WshAKhtG9UW
  - Row 1: the three original looks, A (Genos), B (Maschine) and C (Studio), all on the same layout.
  - Row "Direction B: every screen": seven 1440×900 screens.
- **Sources to regenerate the B screens:** `docs/design/wireframes-b/` (`shell.html` and `gen.py`). See "Editing the wireframes".

## Why a redesign

The owner found the current app (Tauri + Svelte 5, `app/`) "overwhelming", "clinical" and unfamiliar. Features are tucked into drawers and sidebars they don't expect. The apps they know and like are Maschine, Ableton Live, Logic/GarageBand, the Genos panel and Kontakt.

## The owner's answers (binding unless they change them)

The owner was asked short atomic questions. Keep doing that: they asked for it.

**How they use it**
- Eyes are half on the keys and half on the screen.
- They **rarely use the mouse while performing**.
- It runs on a **laptop screen**, in **one window**.

**Layout, top to bottom**
1. **Header** with the transport: Start/Stop, Sync Start/Stop, Tempo −/value/+, Tap, beat lights, bar·beat. Also Browse and the Master level.
2. **Genos-like display**, about **half the window**. It has tabs. The owner chose Home, Multi Pads, Looper & Charts, Harmony/Arp and Settings; we added **Channel** and **Effects**.
   - **Home** shows the style name, tempo, bar/beat, the section playing and the one queued, the chord, and the current registration and OTS.
   - **Every style gets a generated Kontakt-style artwork**, derived from the style, as its visual identity.
3. **Mixer row, always visible.**
   - 12 strips with **all parts equal**: Right 1–3, Left, then the 8 style parts. Strips are merged with the parts view, so each strip shows its voice name.
   - The **Launchkey mirror** sits **beside the mixer**. It always follows the hardware's current pad and knob page, and it's the first thing to shrink when space is short.
4. **Registration and One Touch row** above the keys: bank, buttons 1–10, Memory, Freeze, OTS 1–4, Link.
5. **The on-screen keyboard stays**, showing held notes and the split point.

**Interaction**
- **Browser:** full screen when open, like the Genos.
- **Clicking a voice name** on a strip opens a **quick list**. "More…" in that list opens the browser.
- **Everything the Launchkey can do is also reachable by mouse.**

**Look**
- **A colour per part plus lit active states**: Maschine groups plus Genos lamps.
- **"In between" feel:** mostly flat, with lit, glowing highlights. Direction B is near-black, vivid part colours and orange for active.

## Direction B, screen by screen (row 2 of the canvas)

1. **Home.**
   - Sections are big tiles: Intro I–III stacked, Main A–D as large tiles, Break, and Ending I–III stacked.
   - Each Main tile shows a pattern preview, its length in bars, and a **Fill button under it**.
   - A bar-progress line runs across the top.
   - A toggle row holds Auto Fill, OTS Link, Sync Start, ACMP, Left Hold and Accent.
   - This replaced the first B's small section buttons, because the owner wanted "better use of the space showing the sections".
2. **Channel.** Click any strip, or use the Launchkey page buttons, and the display becomes that part's full channel strip:
   - Level: fader, meter and pan.
   - Effect sends: Reverb, Chorus, Delay and Dry.
   - Tone, from OTS and voice: Cutoff, Reso, Attack, Decay, Release and Vibrato.
   - Play: Octave, Mono/Poly, Bend range and Portamento.
   - ‹ › buttons step between parts.
   - The owner asked for this ("select a track and then show a detailed channel strip").
3. **Effects.** Reverb, Chorus and Delay cards. Each has "From style / Mine", type chips, parameter knobs and a Band send slider.
   - Reverb: Time, Pre-delay, Tone.
   - Chorus: Rate, Depth.
   - Delay: note values 1/16–1/2 with triplets and dotted, Tempo sync, Ping-pong, Feedback, Tone.
4. **Multi Pads.** The bank, the pad level (fader 6), four big pads with Playing/Armed state and SYNC/REPEAT/CHORD flags, and Stop all.
5. **Looper & Charts.**
   - The Chord Looper: memories 1–8, the chord sequence with a playhead, Rec/Stop, Loop, Clear, and bank Save.
   - The chart: a bar grid with the current bar.
6. **Voice quick list.** A popup over the Chord 1 strip listing similar voices, favourites, plugins and patches, and "More in the Browser…".
7. **Browser (full screen).**
   - Tabs: Styles / Voices / Pads / Songs.
   - A category list with colour chips and counts, and search that includes song titles (from the Music Finder data).
   - Style cards with generated art.
   - A preview panel: Preview, Load, Favourite and "Edit a copy…".

**Also changed from the first B:** every mixer strip has small **REV and DLY knobs**, so effects are one turn away ("I want the effects more easily accessible").

## Open questions and not yet drawn

- **Not drawn yet:**
  - the Harmony/Arp tab;
  - the Settings tab;
  - the browser's Voices, Pads and Songs tabs;
  - what the Launchkey mirror shows on each page;
  - light mode (the app has one today);
  - the Style Editor entry (separate wireframes: https://claude.ai/artifact/SegsX9Qdg6xS6zzc6mac5H).
- **Layout questions:**
  - Does the display stay at about half the window, or grow and shrink per tab?
  - Are 12 strips readable at about 83 px on a 1440-wide laptop? The owner hasn't said.
- **Channel tab behaviour:** does it replace Home while a strip is selected, or only when the owner clicks the tab?
- **Style artwork:** generated per style. Its look (rings and bars) is a placeholder; keep asking whether it feels right.

## What the screens map to in the engine

Everything drawn exists in the engine or API, or is in flight:
- effect params and band sends: #236;
- the style's own effect types, "From style": #237;
- OTS tone controls: #238 and #246;
- Registration of the parts' settings: #238, #252;
- knob pages: #197, #224;
- Left Hold: #202;
- the looper banks: #201.

The API is in `docs/app-api.md`. Both mocks (`app/src-tauri/src/mock.rs` and the TS mock) must follow any API change.

## Editing the wireframes

The canvas is a Claude Design canvas: each artboard is one self-contained `.dc.html` file, laid out by `project/canvas.json`. The B screens are generated:

```bash
cd docs/design/wireframes-b && python3 gen.py   # writes project/B*.dc.html next to it
```

**How the generator is built**
- `shell.html` is the shared frame: header, the display frame with its tabs, the mixer strips with REV/DLY minis, the Launchkey mirror, the Registration/OTS row and the keyboard.
- `gen.py` fills the placeholders for each screen: `%%TAB%%`, `%%SEL%%` (selected strip index, or -1), `%%DISPLAY%%` (display markup), `%%JS%%` (extra `renderVals` data) and `%%OVERLAY%%`.

**Palette:** bg `#0e0e10`, panels `#19191c`/`#141417`, edges `#2d2d32`, text `#f2f2f2`/`#8d8d95`, active orange `#ff7a2f`. Part colours are in `COL` in the shell's script. Fonts are Archivo and Archivo Narrow.

**`.dc.html` rules that bite:**
- keep `<script src="./support.js"></script>`;
- give the root a fixed 1440×900;
- `{{hole}}` is a lookup only: compute values in `renderVals()`;
- UI is markup only, never script-built;
- use `<sc-for>` with `hint-placeholder-count`.

**To publish a change:**
1. `read` the canvas's `project/canvas.json` first, because the owner can edit it live.
2. Publish only the changed `project/*.dc.html` files, with `root` set to the folder that holds `project/`. Send `canvas.json` only when adding or moving artboards.
3. Don't verify renders unless the owner asks.

## Suggested next steps

1. Ask the owner, in short atomic questions, what to refine on Home, Channel and Effects.
2. Draw the missing tabs: Harmony/Arp and Settings.
3. Once the screens settle, write the build plan. Swap the app's drawers for the display tabs in small PRs:
   - the shell layout;
   - the Home tiles;
   - mixer strips with minis;
   - the Channel tab;
   - the Effects tab;
   - the full-screen browser;
   - the voice quick list.

   Follow `docs/agents/ui-brief.md`: tooltips on every control, both mocks updated, `npm run verify`.
