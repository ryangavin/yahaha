# App redesign, direction B ("Maschine colour"): handoff

Status 2026-09-26 (round 2): **the owner picked direction B** and is refining it. Nothing is built yet; everything here is wireframes. Round 2 redrew Home, added mixer layers, a Home at four window sizes, and an artwork options sheet. Read this, open the canvas, then go through "Waiting on the owner".

- **Canvas (private to the owner):** https://claude.ai/artifact/U3wyJmzqbr2WshAKhtG9UW
  - Row 1: the three original looks, A (Genos), B (Maschine) and C (Studio), all on the same layout.
  - Row "Direction B: every screen": seven 1440×900 screens, plus sheet 8 (style artwork options).
  - Row "Home at other window sizes": 1024×768, 1280×800 and 1920×1080.
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

**Round 2 answers (2026-09-26)**
- The display stays **about half the window** on every tab, but the layout must adapt to **all** window sizes: smaller laptops, narrow windows and big monitors.
- **Clicking a strip jumps to the Channel tab.**
- The 12 strips are **readable** at about 83 px.
- Home: the **section tiles were too big**. Replace them with the **Launchkey pad representation**, since the pads cover every section concept.
- Home: the Main pattern previews must be **real**, drawn from the style's own pattern.
- Home: show the **registration and OTS names**.
- Home needs **quick effect controls**: band send levels.
- **Sends for everything via a fader layer**, as in Ableton: the faders swap between volume and sends. The **hardware should match**, with Launchkey faders that cycle through send layers too.
- The artwork should be **smaller** and **look different**. The owner asked to see a few options.

**Look**
- **A colour per part plus lit active states**: Maschine groups plus Genos lamps.
- **"In between" feel:** mostly flat, with lit, glowing highlights. Direction B is near-black, vivid part colours and orange for active.

## Direction B, screen by screen (row 2 of the canvas)

1. **Home** (round 2).
   - **Left:** the art is smaller (pattern print for now), with the category, the style name, **Regist 3 · bank** and **OTS 2 · name**, and Browse and Edit.
   - **Top line:** Playing, Next, bar progress and the chord.
   - **Sections:** the **Launchkey Sections pad page drawn big**. It uses the same 2×8 order, colours and lights as `src/launchkey.rs` `section_looks`:
     - top row: Intro 1–3, Sync Start, Ending 1–3, Auto Fill;
     - bottom row: Main A–D, Break, Tap, Sync Stop, Start.
     - The Main pads show their real first bar: kick, snare, hats and bass.
     - A queued fill makes its Main flash, drawn with a dashed outline and a FILL tag.
   - **Toggle row:** OTS Link, ACMP, Left Hold, Accent, fingering and split. Auto Fill and Sync are on the pads.
   - **Right:** the **band effect sends** (Reverb, Chorus, Delay, with the type) and a link to Effects.
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

**Mixer layers (round 2).** The REV/DLY mini knobs are gone. Buttons at the mixer's left edge (**VOL · PAN · REV · CHO · DLY**) switch what all 12 faders control, as in Ableton's sends view:
- In a send layer, each fader fills in its part colour and shows the send value.
- PAN turns the faders into bipolar knobs.
- The Effects board shows the REV layer.
- Engine: `setPartSend` already covers reverb, chorus and variation (delay) per part. The style parts' sends and the hardware layers are new work.

**Launchkey mirror (round 2).** It shows the real Sections pad page (hardware colours, dim, bright and flash), the knob page, and the 9 faders with their page and layer.

**Home at other window sizes (round 2).** The display stays at about half the window. As the window shrinks, things give way in this order:
1. the Launchkey mirror (narrower at 1280, hidden at 1024);
2. the art column, which folds into a thumbnail in the status line at 1024;
3. the band-effects column, hidden at 1024 because sends stay on the mixer layers;
4. at 1024, the header, tab and registration labels shorten.

At 1920 everything grows, the keyboard shows 88 keys, and the mirror pads get taller.

**Sheet 8: style artwork options.** Four generated looks, each at Home size and at browser-list size:
1. **Pattern print:** a cell per 16th, a row per part;
2. **Genre poster;**
3. **Gradient + glyph;**
4. **Contour lines.**

The Browser board still uses the old rings and bars until the owner picks one.

## Waiting on the owner (round 2 decisions made without them)

1. Artwork look: pattern print is a placeholder default. Which of the four?
2. Hardware layers: **Shift + the master fader button** steps VOL→PAN→REV→CHO→DLY, and the master button alone still switches Panel/Style. This needs an issue and engine work: soft takeover per layer, and LED colour per layer.
3. The 1024 layout hides the mirror, the art column and the band-effects column. Is that the right order to give way?
4. 1920 only grows; nothing new is added. Should the extra room show more (the next chords, or the mirror at full size)?
5. Home's pads keep Tap, Start and the Sync buttons, because the hardware page has them, even though the header has them too.

## Open questions and not yet drawn

- **Not drawn yet:**
  - the Harmony/Arp tab;
  - the Settings tab;
  - the browser's Voices, Pads and Songs tabs;
  - what the Launchkey mirror shows on each page;
  - light mode (the app has one today);
  - the Style Editor entry (separate wireframes: https://claude.ai/artifact/SegsX9Qdg6xS6zzc6mac5H).
- **Style artwork:** see sheet 8. The browser still uses the old placeholder.
- **Other tabs at other sizes:** only Home is drawn at 1024, 1280 and 1920.

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
- `gen.py` fills the placeholders for each screen:
  - `%%TAB%%`;
  - `%%SEL%%`: the selected strip index, or -1;
  - `%%LAYER%%`: the mixer layer, VOL, PAN, REV, CHO or DLY;
  - `%%DISPLAY%%`: the display markup;
  - `%%JS%%`: extra `renderVals` data, which must define `extra`;
  - `%%OVERLAY%%`;
  - the size keys from `SIZES` (`W`, `H`, `DISPH`, `MIRW`, `MIRDISP`, `KEYH`, `NW`, `COMPACT`, `WIDE`).
- `home(artW, fxW, wide)` builds Home for any size. A screen's `SIZE` picks a size.
- `art_options.py` writes the standalone artwork sheet, `BArtOptions.dc.html`.
- `project/` is generated and git-ignored. Edit `canvas.json` from a fresh `read` of the live canvas, never from a local copy.

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
