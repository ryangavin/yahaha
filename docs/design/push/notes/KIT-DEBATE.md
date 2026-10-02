# Kit for the debate fix round (phase 1 output)

Copy these line ranges verbatim (inclusive). Markup is byte-identical between dark and light: every theme difference is a token, so the dark and light ranges hold the same text except the KIT `:root` line. All paths are under `project/`.

| Piece | Dark | Light |
|---|---|---|
| **KIT tokens + lamp CSS** (comment, kit `:root` line, `.lamp` rules). Paste right after the board's `a{...}` line inside `<style>`. | Stage-Dark 59-61 | Stage-Light 35-37 |
| **App bar, Stage variant** (wordmark, nav, fixed 196px right area: Launchkey, "Audio" health slot). Move the white `aria-current` block to your page. | Stage-Dark 66-88 | Stage-Light 42-64 |
| **App bar, page variant** (adds Rack readout + One Touch buttons 1-4; for every non-Stage page) | Channel-Dark 72-102 | Channel-Light 36-66 |
| **Section row with the count row** (Accomp lamp, count row, Metronome/Unison lamps, Panic, ?) | Stage-Dark 90-115 | Stage-Light 66-91 |
| - count row alone | Stage-Dark 93-108 | Stage-Light 69-84 |
| **One Touch group** (Stage display style line) | Stage-Dark 130-137 | Stage-Light 106-113 |
| **Full band** (faders, knobs, pads, transport) | Stage-Dark 215-374 | Stage-Light 191-350 |
| - full-band lamp row (headers + 4 part lamps + Functions) | Stage-Dark 264-281 | Stage-Light 240-257 |
| - full-band strips JS (TRAVEL, F, LAYER/LV, strips map) | Stage-Dark 407-459 | Stage-Light 379-431 |
| **Half band** (faders, Track, knobs, pads, transport; no Shift button) | Channel-Dark 348-483 | Channel-Light 312-447 |
| - half-band lamp row | Channel-Dark 399-411 | Channel-Light 363-375 |
| - half-band pads (16 static pads, about 71×28) | Channel-Dark 449-470 | Channel-Light 413-434 |
| - pads header "Sections · next Main C" | Channel-Dark 432 | Channel-Light 396 |
| - half-band strips JS (TRAVEL, TOP, F, LAYER/LV, strips map) | Channel-Dark 591-641 | Channel-Light 555-605 |
| **Compact block** (320 × 84: style, tempo, running, chord, playing section) | Channel-Dark 137-149 | Channel-Light 101-113 |

Ranges updated after the self-review round (SELF-REVIEW.md).

## Notes for lanes

- **KIT `:root` line**, dark: `--lamp:#9fe04a; --lamp-ink:#000; --solid-ink:#000; --past:#333; --mbg:#141414; --bg/--ba/--bw` glows; `--warn:#ff9a2e`. Light: `--lamp:#4f8a0e; --lamp-ink:#111; --solid-ink:#fff; --past:#c4c3bf; --mbg:#e2e1dd`; glows `none`; `--warn:#c85f00`. It redefines `--bg --ba --bw` with the values some boards already carry, so it's safe after any existing `:root`. Added later on all 48 boards: `--rec` (record red, #e5534b / #b8322c), `--bm` (Main-hue text glow), `--ba2` (display-size chord glow), `--bl` (L-zone line glow on the keys), `--keyline` (white-key separator, #000 / #d3d2ce); the glows are `none` in light. The `.lamp` line adds `.lamp.rec` (pressed = solid `--rec` face) and `.lamp[aria-disabled="true"]` (`--d` label, default cursor).
- **Lamp button**: `class="lamp"` plus `aria-pressed="true|false"`. Don't set `background`, `color` or `font-weight` inline (the class owns them). On = lime face, black label; off = the `--btn` face, grey label, no bar. A small code label goes in `<span class="sub">`. Example: Stage-Dark 85 (Accomp), Channel-Dark 195-196 (On/Solo).
- **Chosen** = white block (`background: var(--t); color: var(--g)`), e.g. One Touch "2" and the layer tab. **Waiting** = `border: 1px solid <hue>`, e.g. count row "Main C" and the queued pad.
- **Fader layer**: set `LAYER = 'Rev'` (with LV values) in the strips JS. Move the white tab block to that layer's tab and change the header word to `Faders · <span style="color: var(--t)">Reverb</span>`. Pan's centre-detent bracket isn't drawn in the kit; the Effects lane adds it.
- **Health trouble state** (SettingsSystem): swap the slot's text to `CPU 74%` / `2 dropouts` / `3 dropouts · buffer 256?` / `R3 failed` with `color: var(--ending)`. A failed strip uses mark `'failed'` in `F` (red ✕ svg, already in both bands).
- **Sound latched**: set the Sound lamp's `aria-pressed="true"`. The rest (header "Racks" block, pager "Racks · Sound", "Save over" caption) is PadsPage2's own markup.
- Line numbers are from the files as left by phase 1. Copy before you edit your own board.
