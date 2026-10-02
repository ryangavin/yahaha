# Push: the final design direction

The owner chose **Push** as the app's final look. This folder is the reference for every spec under `docs/specs/push/` and for the code built from them. The canvas it came from is the claude.ai artifact "yahaha · Push" (https://claude.ai/artifact/Vma4hDTe9ZQFm2R9YqLcRy, version 1790924408-13a3, exported 2026-10-02).

## What's here

- `<Screen>-Dark.dc.html`, `<Screen>-Light.dc.html`: each board's full source, 1440×900. The markup is the design: exact boxes, px sizes, colours (CSS custom properties at the top), type and copy. The `BOARD NOTES` comment at the top of each file is the designer's rationale: which commands each control sends, tooltips, Launchkey mappings and what changed from the board it was copied from.
- `png/<Screen>-Dark.png`, `png/<Screen>-Light.png`: a screenshot of each board, rendered from the HTML with headless Chrome at 1440×900.
- `canvas.json`: the canvas index: which page (user flow) each board sits on, in order.
- `notes/`: the design session's working notes. `DECISIONS.md` is the checklist of product decisions the boards must show; `PUSH-APP.md` the rules of the Push language; `KIT-DEBATE.md` and `FIX-DEBATE.md` the shared kit and the fix round. The boards also cite `GAPS-DESIGN.md`, `GAP-AUDIT` and `FIX-PUSH.md`, which weren't kept; the board notes repeat what they decided.

## Opening a board in a browser

The boards load `./support.js`, the canvas's component runtime. It isn't committed (third-party code in a public repo; `.gitignore` here). To render a board locally, fetch it once from the canvas with the Artifact tool (`read`, path `artifact-type/dc-runtime.js`) and save it here as `support.js`, then open the `.dc.html` file in Chrome. Without it, read the markup or use the PNG.

## Screens, by user flow

| Flow | Screens |
|---|---|
| Play | Stage, FirstRun, PadsPage2, Stage-Help, Stage-Metronome, PadsChord, PadsMultiPads, PadsSetup |
| Find a style | Browser |
| Change a sound | SoundPick, LibrarySounds, LibraryInstruments, Rack, Rack-MapEdit |
| Shape the mix | Channel, Effects, Channel-StylePart, Effects-Master, Effects-Reverb, Effects-Chorus |
| Save your setup | QuickRacks, LibraryRacks, Prompts, Prompts-More |
| Pads, loops, harmony | MultiPads, Looper, Harmony, Harmony-Arp |
| Set up | SettingsChord, SettingsStyle, SettingsKeyboard, SettingsPedals, SettingsSystem, SettingsLaunchkey, StyleMap, SoundPicker |

**Stage** holds the shared kit (app bar, now-playing block, count row, fader band, knobs, pads, transport, key strip, the three button faces). Every other board copies those parts from it, so its spec comes first and the others refer to it rather than repeat it.
