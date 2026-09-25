# Unison & Accent (PSR-SX) and what the Genos offers instead

Topic id: `unison-accent` · Pass: 2026-09-25 · Manual: not in the Genos2 manuals (see genos-features.md §C.9); related Genos2 items RM p.27, p.55, p.60 · yahaha: `docs/genos-features.md#c9-unison--accent`, issue #180, `src/engine/dynamics.rs` (on `develop`)

Paraphrased notes only. No transcript text, manual text or frames are committed.

## Sources

| # | Video (channel) | URL | Timestamps used |
|---|---|---|---|
| V1 | PSR-SX600: Play with Unison (All Parts) (Yamaha Global) | https://www.youtube.com/watch?v=yWYuZeG3I3E | music only; stills 00:10, 00:30, 00:50 |
| V2 | PSR-SX600 features with Karthick Devaraj (Yamaha Music India) | https://www.youtube.com/watch?v=jLVzt_SqHFY | 03:58–05:37 |
| V3 | PSR-SX600: Play with Accent, Unison & Accent function (Yamaha Global) | https://www.youtube.com/watch?v=WuvATAS6Ing | transcript pending (HTTP 429; probably music only, 57 s); stills 00:15, 00:35 |
| V4 | PSR-SX600 demo with Unison (Auto Split) and Accent, Viennese Waltz style (Eyyub Eyyubov) | https://www.youtube.com/watch?v=tu_CNwgcmYM | no English captions; stills 00:05, 00:20 |
| V5 | 5 cool things about PSR-SX (Yamaha Music India) | https://www.youtube.com/watch?v=c31P2pQuF3Y | read in full; it doesn't cover Unison & Accent (Bluetooth, sub out, expansion packs, joystick, Registration Sequence by pedal, Drum Setup) |

Video coverage: thin. V1 has no speech. V2 is a promo with one short explanation. V3 and V4 are known only from their titles and stills. There is no PSR-SX owner's manual in `docs/manuals`, so none of this can be checked against a manual.

Stills (private folder), `yWYuZeG3I3E_0010_0.jpg`, `_0030_0.jpg`, `_0050_0.jpg`: all three show a PSR-SX600 played with **both hands on the right half of the keyboard in parallel**. A caption reads "UNISON", and an inset shows the player's **foot holding a footswitch** down. The point: Unison is played as a moment. You hold a footswitch, and the band plays your line with you.

More stills:
- `WuvATAS6Ing_0015_0.jpg` and `_0035_0.jpg`, captioned "ACCENT": two-handed playing in the middle and right of the keyboard, with **no footswitch inset**. Accent comes from touch alone.
- `tu_CNwgcmYM_0020_0.jpg`: the same Yamaha footage with both captions, "UNISON" and "ACCENT", lit and the footswitch held. The two features combine.
- V4's title mentions a "Unison (Auto Split)" mode, which suggests Unison can also work split: left hand chords, right hand the unison line. *title-only*.

## Genos behaviour (subtleties a player notices)

- **The Genos2 has no Unison & Accent.** The Genos2 OM/RM don't mention it. The nearest Genos2 items are the Ensemble Voice "Unison" note-assignment types (RM p.60), the FM voice unison/detune (RM p.55), and Style Creator's offline Dynamics "Accent Type" (RM p.27). *manual-confirmed (absence)*.
- **Unison (PSR-SX).**
  - While it is on, the melody you play is doubled by several parts at once: "multiple voices on multiple layers" from one key, beyond a normal layer or split [V2 04:33–05:10].
  - V1's title says the unison can involve **all parts**, and the stills show it held from a footswitch for a passage. *video-only*.
  - From V1's audio, the Style's own parts stop their patterns and play the performer's line: a band tutti or "unison break". The text sources are too thin to pin down which parts join and how they voice it. *video-only, unconfirmed*.
- **Accent (PSR-SX).** Playing velocity controls accents: hard notes trigger rhythm "shots" (hits) that the Style adds to match the line you play [V2 03:58–04:11, 05:24]. *video-only*. #180's corpus probe found that this needs accent data built into styles made for the feature, and **0 of yahaha's 208 corpus styles** carry it.
- **Together.** The presenter's favourite combination: Unison for the line, and Accent so the band punctuates it following his touch [V2 05:24–05:37]. What owners like here is the band reacting to the player, the same thing the owner asked for in #180.

## Manual check

- The absence from the Genos2 manuals is confirmed (grep: no Unison/Accent feature outside the items listed above).
- Everything about the SX behaviour is *video-only*, and the videos are thin. An SX720/920 Reference Manual would be the spec source.

## yahaha today

- **On `develop` (PR #184):** "Accent" is a yahaha stand-in. A chord-section strike at or above a threshold, while a Main plays, starts that Main's own Fill In from the next beat. It is not a Main press, so OTS Link doesn't follow it.
  - Touch lets chord-section strikes set the Dynamics level.
  - Both are off by default, and neither is reachable until #180 parts 2–3 land.
- **Unison:** none.
- **Nearest building blocks:**
  - Keyboard Harmony's part assignment (docs/harmony.md);
  - the Style part voices, which yahaha knows per section (README Mixer, SInt);
  - the pedal function table (`src/controllers.rs`).

## Is it a competitive gap?

- **Not a Genos parity gap.** The Genos2 has neither feature, and yahaha's aim is Genos 1:1.
- **A competitive gap against the cheaper PSR-SX line**, and one the owner wants in spirit (#180 asks for a band that reacts to how hard you play).
- **Accent:** Yamaha's version needs style data yahaha's styles don't have. The fill-on-hard-hit stand-in is the realistic answer, already built.
- **Unison needs no special data.** It can be built from what yahaha has: while held, silence the chosen Style parts' patterns and send the right-hand line to those parts' channels (their current voices), optionally octave-folded per part. That makes it a moderate, self-contained feature.

## Gaps

| Gap | Priority | Suggested next step |
|---|---|---|
| Accent stand-in exists in the engine but isn't reachable | P2 | Ships with #180 parts 2–3. Label it a yahaha/SX-inspired extension in docs/genos-features.md §C.9. |
| No Unison ("band plays my line while I hold a pedal") | P2 | A design issue after the owner confirms interest. Pedal function "Unison" (Hold A). Parts: all, or melodic only (Bass, Chord 1/2, Pad, Phrase 1/2). While held, those parts' pattern notes are muted and the Right-hand notes go to their channels at their voices, each folded into the part's range. Rhythm keeps playing. |
| Real SX accent data can't be played (no corpus style has it) | – | Nothing to do until such styles appear. #180 follow-up already notes it. |
| SX behaviour unverified | – | Next pass: read V3, or an SX920 Reference Manual if the owner can supply one. |
