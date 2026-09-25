# Auto Fill In, Fill Up/Down/Self, Break behaviour

Topic id: `auto-fill` · Pass: 2026-09-25 · Manual: OM p.67, RM p.142, DL p.82 · yahaha: `docs/fills-and-rules.md`, `src/engine/fills.rs`

Paraphrased notes only. No transcript text, manual text or frames are committed.

## Sources

| # | Video (channel) | URL | Timestamps used |
|---|---|---|---|
| V1 | What is Auto Fill in / Fade / Mixer (BB Walker TV, PSR-S670) | https://www.youtube.com/watch?v=Ww5j2YN0-iE | 00:53, 01:51-03:46, 07:21, 09:04-09:46 |
| V2 | Fill-in buttons, music arranger etc. (Robert Gregson, entry-level Yamaha) | https://www.youtube.com/watch?v=apR_iPYGhiE | 01:53, 02:44, 17:09-18:57 |
| V3 | Auto Fill in and OTS Link buttons (jorgebopart) | https://www.youtube.com/watch?v=g8tzT_Qvccc | transcript pending (HTTP 429) |

Both videos read are PSR (arranger-family) demos. **No Genos-specific video evidence this pass**.
The Genos claims below rest on the manuals.

## Genos behaviour (subtleties a player notices)

- **Auto Fill In on**: any Main press plays a fill first, into the next Main or the same one
  (OM p.67). On the PSR the Main lamp blinks while the fill plays, and the Main takes over
  after it [V1 03:46]. *manual-confirmed* (OM p.68 lamp note).
- **Auto Fill off**: a Main change just switches pattern, with no transition [V1 01:51].
  *manual-consistent*.
- **Auto Fill forces Next Bar timing** even when To Main is Immediate (RM p.12).
  *manual-confirmed*.
- **Storage**: Auto Fill In is a System setting, not a Registration one (DL p.82 chart).
  *manual-confirmed*.
- **Default**: on the PSR-S670 Auto Fill is off from the factory [V1 00:53]. *video-only*. The
  Genos manuals give no factory state.
- **Fill Up / Down / Self / Break** are assignable functions (RM p.142). "Up" means the Main on
  the immediate right, and "Down" the immediate left. The manual doesn't say what happens at
  the ends of the row. BB Walker assigns them to a pedal on a PSR. His demo shows Fill Up
  going from D back round to A: it wraps [V1 09:18-09:46]. *video-only* (PSR-S670); the
  manual is silent.
- **Holding BREAK** keeps the break going until released, on the PSR [V1 07:21]. *video-only*.
- **Fills as a timing rescue**: beginners are taught to press a fill after losing the beat,
  because the band realigns them at the next bar [V2 02:44]. This is a player habit, not a
  function. It needs the fill to start promptly and end on the bar line.
- On entry-level Yamaha "Music Arranger" models the fill button also steps the variation
  [V2 17:09-18:57]. That does not apply to the Genos; noted only to rule it out.

## Manual check

- Auto Fill behaviour, Next Bar fallback, System storage, Fill function meanings: *manual-confirmed*.
- Fill Up wrap D→A and hold-BREAK: *video-only* (PSR); the manual is silent. **Owner question.**
- Factory default of Auto Fill: *video-only* (PSR: off); Genos unknown.

## yahaha today

- `press_main` (`src/engine/fills.rs`): with Auto Fill, the fill of the *target* Main plays,
  then that Main (Fill In BB into Main B). The Main lamp flashes during the fill
  (`section_leds`, `src/launchkey.rs`). **Matches.**
- Auto Fill → Next Bar: `Change::Main` in `change_point`. **Matches.**
- Not stored in Registration (`docs/registration.md`). **Matches.**
- Default **on** (`src/engine.rs`, `auto_fill: true`). The PSR default is off; the Genos default is unknown.
- Fill Up/Down: they skip missing Mains and **do not wrap**. At the end of the row they play
  that Main's own fill (`neighbour_main`, a documented decision). This differs from the PSR's
  wrap in V1.
- Fill Self is the selected Main's fill; Fill Break is the Break. All four are assignable
  (`src/controllers.rs`). While stopped, Fill Up/Down select the Main.
- Holding a Break pad: press only.

## Gaps

| Gap | Priority | Suggested next step |
|---|---|---|
| Fill Up on D / Fill Down on A: yahaha stays; a PSR wraps round (video-only) | P2 | Owner question; if the Genos wraps, change `neighbour_main` to wrap (one-line change + test) |
| Auto Fill default on vs PSR off | P2 | Owner question (keep yahaha's choice unless the Genos is known to be off) |
| Hold BREAK to keep the break going | P2 | Same as `sections` note |
| V3 (jorgebopart) not read | – | Re-read when captions arrive |
