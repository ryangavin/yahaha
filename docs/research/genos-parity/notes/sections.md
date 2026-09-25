# Style sections: Intro / Main / Fill / Break / Ending subtleties

Topic id: `sections` · Pass: 2026-09-25 · Manual: OM p.66-68, RM p.12, DL p.111 · yahaha: `docs/section-timing.md`, `docs/fills-and-rules.md`, `src/engine/sections.rs`, `src/engine/fills.rs`

Paraphrased notes only. No transcript text, manual text or frames are committed.

## Sources

| # | Video (channel) | URL | Timestamps used |
|---|---|---|---|
| V1 | Easy Intros and Endings, style creation part 6 (Arranger Tutorials) | https://www.youtube.com/watch?v=zrWqeJNiLK4 | 00:28, 03:34-04:34 |
| V2 | Yamaha Genos The First Steps (Genos Genie) | https://www.youtube.com/watch?v=rce-xYjdZlA | 02:30-03:54 |
| V3 | PSR/Genos: Ending as Fill In or Break (Alois Müller, German, 52 s) | https://www.youtube.com/watch?v=SjfcjJX-MbM | description; stills 00:12, 00:30 (no transcript: captions rate-limited) |
| V4 | Two-bar fill / long break (Casper tutorSynth) | https://www.youtube.com/watch?v=4opmPtm8oF0 | 00:47-01:13, 06:12-07:48, 09:28, 10:35 (transcript arrived late in the pass) |
| V5 | What is Auto Fill in / Fade / Mixer (BB Walker TV, PSR-S670) | https://www.youtube.com/watch?v=Ww5j2YN0-iE | 03:46, 04:27, 07:21 |
| V6 | Fill-in buttons etc., beginner (Robert Gregson, entry-level Yamaha) | https://www.youtube.com/watch?v=apR_iPYGhiE | 09:06-10:48 |

Video evidence is thin for this topic. V4 is the one Genos-specific fills/breaks tutorial read,
and it is about style editing. V5 and V6 are PSR models, so they count as Yamaha arranger
behaviour, not Genos proof.

## Genos behaviour (subtleties a player notices)

- **Intro, then Main.** An Intro is chosen before START and flows into the selected Main when it
  ends (OM p.66). The Intros and Endings carry their own chord movement: the pattern is written
  with a fixed-root rule (the NTR/NTT settings), so the player holds one chord and the Intro
  "plays the progression" itself. Factory styles also keep separate major and minor tracks for
  Intros and Endings and pick one from the chord played [V1 00:28, 03:34-04:34]. Beginners
  are told that I is short and II/III are longer [V6 09:06-10:48].
- **Main again = its own fill** (OM p.67). While a fill plays, the Main lamp flashes red
  (OM p.68; seen in [V5 03:46] on a PSR).
- **Break** is a one-measure pattern that then goes back to the Main (OM p.68).
  - Casper confirms the limit. Mains, Intros and Endings can have several bars (up to 32), but
    a Fill or Break is always exactly one bar and the Style Creator won't change that
    [V4 00:47-01:13].
  - His workaround for a two-bar break works outside the keyboard. He squeezes two bars of
    notes into the one-bar Fill and puts a slower tempo event at its start, so it sounds twice
    as long. A faster tempo gives a half-bar fill [V4 06:12-07:48, 10:35]. The tempo event
    has to sit after the section marker [V4 09:28].
  - So the Genos **plays tempo events inside a style section**. *video-only*; the manual is
    silent.
  - How the Break sounds depends on where in the bar it is pressed, because it comes in
    mid-bar [V5 04:27].
- **Holding BREAK** on a PSR keeps the break going (the melody parts drop out) until it is
  released, in BB Walker's demo [V5 07:21]. *video-only* (PSR-S670); the Genos manuals
  never mention holding a section button.
- **Ending with ritardando.** Ending I-III stop the style after the ending. Pressing the
  same Ending again while it plays slows it down gradually (OM p.66). *manual-confirmed*.
- **An Ending used as a fill or break.** Alois Müller presses an Ending, then a Main before
  the Ending finishes, so the band carries on in the Main. He says this works with some
  Endings but not all [V3 description, still 00:12]. The still at 00:30 shows an Ending
  button lit red among blue-lit section buttons. *video-only* (the manual doesn't say what
  a Main press during an Ending does).
- **Lamps** (OM p.68): red means selected, flashing red means next, blue means the section has
  data, off means it is empty. The Main lamp also flashes red during a fill. So during an Intro
  or a Break, the Main that comes next should flash. In the V3 still the lamps are red and blue,
  which fits.
- **After an Ending ends**, the manuals don't say which Main is selected. V2 does not show it
  either: the presenter picks the Main again himself before the next start (06:13).

## Manual check

- Intro → Main, Main-again fill, one-bar Break, Ending + rit.: *manual-confirmed* (OM p.66-68).
- Lamp meanings: *manual-confirmed* (OM p.68).
- Hold BREAK to sustain: *video-only* (PSR). Main during Ending: *video-only*.
- Fill/Break exactly one bar: *manual-consistent* (OM p.68 for the Break) and V4. In-section
  tempo events play: *video-only* (V4).
- Intro/Ending built-in progressions and major/minor tracks: style-format behaviour (CASM),
  consistent with DL/format docs; not a panel behaviour.

## yahaha today

- Intro → Main, Main-again fill, Break → Main: `press_main`, `follow_on` (`src/engine/fills.rs`,
  `src/engine/sections.rs`). Matches.
- Ending + second press = ritardando to 65% at the Ending's last tick, tempo back at stop
  (`src/engine/ritardando.rs`, `docs/section-timing.md#ending-ritardando-23`). Matches the
  manual; the curve is a documented guess.
- Ending I "feels immediate" (#129): the investigation found the Ending starts on the bar
  line; most Genos/T5 Ending I patterns are a single held chord hit, and CC11 fades are in the
  style's own data. yahaha plays the data as written. Nothing to change.
- Main pressed during an Ending: queued for the next bar line (`press_main`, `SectionId::Ending`
  arm), with the Ending's CC11 reset for the Main (#122). This matches V3's use. Open bug #187
  (a waiting style change is not re-timed when the Ending is cut short) is related.
- CASM chord-mute / NTT Bypass are parsed (`src/sff.rs`), so Intro/Ending major/minor tracks
  play as on the Genos.
- Lamps (`section_leds`, `src/launchkey.rs`): queued = flash, current = bright, has data = dim,
  empty = off; a Main flashes while its fill plays or is queued. **Difference:** during an Intro
  or a Break the Main that follows is lit *solid* (the `s.main == i && cur not Main` case), not
  flashing as "next". An Intro armed while stopped *pulses*. On the Genos it is simply
  "selected" (red), though a pulse is a reasonable Launchkey reading.
- Holding a section pad does nothing special (press only).
- **Tempo events inside sections are ignored**: `scan_meta` (`src/sff.rs`) reads the tempo
  only before the first section marker. A style edited with Casper's trick (V4) would play its
  squeezed Fill at double speed in yahaha.
- Which Main is selected after an Ending: the last selected one (`self.main` is kept). The
  manual is silent; no video evidence either way.

## Gaps

| Gap | Priority | Suggested next step |
|---|---|---|
| The Main that follows an Intro or Break lights solid, not flashing "next" (OM p.68) | P2 | In `section_leds`, flash the `s.main` pad while `cur` is an Intro or Break; add a lamp test |
| Hold BREAK to keep the break going (PSR behaviour, video-only) | P2 | Owner question; if the Genos does it, loop the Break while the pad is held |
| Tempo events inside a section (the two-bar fill trick) are ignored | P2 | Owner question first: does the Genos go back to the song tempo after such a Fill? Then keep per-section tempo events in `Prepared` and apply them relative to the player's tempo |
