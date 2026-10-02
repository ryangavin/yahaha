# yahaha · Push: the whole app in the chosen skin, organised by what the player is doing

The owner chose the **Push** skin (`project/Stage-Dark.dc.html` and `project/Stage-Light.dc.html` in this folder: read both in full first; they ARE the design system: tokens, type, every component's drawing). The content and layout of every other screen comes from the Ink round (`src/*.dc.html`: read the one you are restyling in full). Your job: redraw each screen in the Push language, dark and light, keeping its content, zones and positions, and applying the content fixes in `ROUND3.md` (Step B items for your screens; `DECISIONS.md` lists the product truths; `FIX-PUSH.md` lists what the skin fixed and what must not break).

Rules of the Push language (from the Stage; copy its markup for shared parts verbatim: app bar, now-playing block, band, half band, key strip):
- Pure black (#000) ground in dark; no boxes; hairline group headers; one geometric sans (DM Sans) everywhere, JetBrains Mono only for tabular digits.
- Selection = solid block with dark text; tabs = white block active, grey text inactive; names in their part/family hue as coloured text; lamps = 2px light bar (lit + bloom, dim when off, none on one-shots).
- Values: light-weight numerals with the unit as a small suffix; labels small grey sentence case; plain words with codes small beneath.
- Knobs: bare 44px ring arcs with a tip dot; faders: Push meters (twin bars, part-hue bracket, white peak tick, takeover ghost), level value at the track top; pads: solid = playing, 1px outline + NEXT = queued, grey = absent; square.
- Hue families: parts saturated and unique (R1 blue, R2 pink, R3 orange, L teal); sections muted (Intro gold, Main green, Ending red, Break violet, Fill grey-blue); utility pads grey/white; accent violet; running green. No two hues within 20°.
- Light twin: identical markup, token swap only (grey paper ground, ink text, denser hues, no bloom). Write the dark file, then derive the light file from it by swapping the tokens the Stage pair swaps.
- Lists: hairline-separated rows, the chosen row a solid block; inputs: an underline with a caret; dialogs: a dark panel with a 1px light edge over the dimmed screen.
- Tall pages (Library, Settings, Channel, Rack) keep the half-height band and keys from the Stage pair (copy verbatim); display pages keep the full band.

File rules as BRIEF.md: 1440×900 fixed root, inline styles, `<sc-for>` for lists, real buttons, Google Fonts css2 only, no images, nothing overflows. Names: `<Screen>-Dark.dc.html` and `<Screen>-Light.dc.html` under `project/`. Write tool only, no verification, no publishing. Reply in under 80 words.

## Pages (how the canvas is organised) and lanes

| Page (what the player is doing) | Screens (from src/) | Lane |
|---|---|---|
| **Play** | Stage (done), FirstRun (src Stage-FirstRun), Pads page 2 + Style faders + Sound hold (src Stage-Racks) | L1 |
| **Find a style** | Browser | L1 |
| **Change a sound** | Quick sound list (src Stage-SoundPick), Library Sounds, Library Instruments, Rack | L2 |
| **Shape the mix** | Channel (src Stage-Channel), Effects | L3 |
| **Save your setup** | Quick Racks, Library Racks, Prompts | L3 |
| **Pads, loops, harmony** | Multi Pads, Looper, Harmony | L4 |
| **Set up** | Settings merged to six full-screen pages per ROUND3 (Chord & Split; Style; Keyboard = Transpose + Lock; Pedals; System = Audio + MIDI + Library; Launchkey), Style map (src Library-Map), Sound picker (src SoundPicker, over Library-Map) | L5 (Settings) and L4 (Style map, Sound picker) |

Output file stems: FirstRun, PadsPage2, Browser, SoundPick, LibrarySounds, LibraryInstruments, Rack, Channel, Effects, QuickRacks, LibraryRacks, Prompts, MultiPads, Looper, Harmony, SettingsChord, SettingsStyle, SettingsKeyboard, SettingsPedals, SettingsSystem, SettingsLaunchkey, StyleMap, SoundPicker. Each with -Dark and -Light.
