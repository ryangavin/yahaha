# Keyboard Harmony, Arpeggio, Vocal Harmony basics

Topic id: `harmony-arp` · Pass: 2026-09-25 · Manual: OM p.56-58, RM p.46-47, DL p.74 · yahaha: `docs/harmony.md`, `docs/arpeggio.md`, `src/harmony`, `src/arp`

Paraphrased notes only. No transcript text, manual text or frames are committed.

**Video coverage: partial.** V1, V3 and V4 were read; two stills from V1. V2 (the
Arpeggiator power tip) was still rate-limited (HTTP 429); the next pass should read it. No
English Keyboard Harmony types video was found (the one hit was Tyros-era).

## Sources

| # | Video (channel) | URL | Timestamps used |
|---|---|---|---|
| V1 | The Tremolo Effect, Genos Power Tip (Keyboard-Akademie; German, English auto-captions) | https://www.youtube.com/watch?v=vOyPy8QMOtw | 01:12-05:16 (Tremolo setup), 05:46-06:45 (speed vs tempo), stills 03:46, 04:40 |
| V2 | The Arpeggiator, Genos Power Tip (Keyboard-Akademie) | https://www.youtube.com/watch?v=0lmYJa6TDZs | transcript pending |
| V3 | PSR-SX920 & Genos 2 Recording Tips Part 2: Arpeggiators, Mixing (ePianos) | https://www.youtube.com/watch?v=8yy3ZiTXM64 | 01:18-01:44, 02:09, 02:48-03:19 |
| V4 | Make Your Genos Play a Counter Melody, Left Voice Trick (Keyboard World) | https://www.youtube.com/watch?v=pasgN3mweIg | 01:05-02:45, 02:58-03:10 |

Stills still pending: the arpeggio detail settings (V2), the Voice Setting S.Art2/Arpeggio page
with Quantize and Hold (V2 or V3).

## Genos behaviour (subtleties a player notices)

- **One button, one type.** HARMONY/ARPEGGIO turns the selected type on or off. The type
  comes from one list: the Harmony category, the Echo category (Echo, Tremolo, Trill) or an
  Arpeggio category (OM p.56-57). The still at 03:46 of V1 shows the Kbd Harmony/Arp
  page: an On switch, a category column (Harmony, Echo, Arpeggio / Up&Down, Synth Seq ...)
  and a type column (Echo, Tremolo, Trill) [V1 03:46].
- **Chord source.** The chord-following Harmony types use the Style chord section with ACMP
  on, the LEFT section with ACMP off and LEFT on; with both on, the chord section is left of
  the Style split and the LEFT voice sits between the two splits. 1+5 and Octave ignore the
  chord; Multi Assign and the Echo category ignore ACMP/LEFT (OM p.56-57).
- **Echo, Tremolo, Trill follow the tempo.** They repeat in time with the current tempo;
  Trill needs two held keys (the last two if more) (OM p.57). V1 sets Tremolo to 1/32 and
  says the repeat rate follows the tempo and always stays in rhythm; the preset's rate was
  turned down a little so the effect does not crowd the sound, and on another style the
  presenter switches between 1/8 and 1/32 by ear [V1 03:38-04:20, 06:12].
- **Minimum Velocity as an accent switch.** V1's main tip: set Minimum Velocity high
  (about 79) so normal playing sounds plain and only a hard strike adds the tremolo, for a
  kalimba / steel-drum lead [V1 02:07, 04:44-05:04]. The manual describes exactly this use
  (RM p.47).
- **Assign.** V1 sets Assign to Right 1 so the effect stays on one part while other Right
  parts play plainly [V1 04:20-04:32]. Auto / Multi / Right 1-3 as in RM p.46.
- **Detail settings.** Volume, Speed (Echo category only), Assign, Chord Note Only
  (Harmony only), Minimum Velocity; only Volume and Assign for Arpeggio; none for Multi
  Assign. A Mono/Legato part counts as off for the Harmony category (RM p.46-47).
- **Voice Set.** Selecting a Right 1 voice loads the Harmony/Arpeggio type memorised for
  that voice, unless the Voice Set Filter box is unticked (OM p.56 note; RM p.41). V3 relies
  on it: many synth voices come with an arpeggio already set, which the player can then
  change [V3 01:31-01:44].
- **Arpeggio in practice.** V3 holds a right-hand chord (adding a note so the pattern has
  more to walk through) and the arpeggio runs in time with the recording's drums; the
  presenter's own chord changes were loose, and he fixes the timing afterwards with the
  Song's track quantize [V3 01:18, 02:09, 02:48-03:19]. So Arpeggio Quantize alone did not
  hide late chord changes in his take (its setting is not shown).
- **Counter melody from a mono Left voice (V4).** Not a Harmony type, but the same "extra
  line from the chord" idea: with a monophonic voice on the LEFT part (a viola), the Left
  part sounds one note of the left-hand chord, the lowest when a chord is struck and then
  the newest key added, so inversions and added 7ths make it move like a counter melody;
  Left Hold keeps it sounding between chords [V4 01:05-02:45, 02:58-03:10]. The
  one-note behaviour comes from the voice's Mono setting (RM p.49), not from a harmony
  function.
- **Storage.** Type, on/off and the detail settings are in Voice Set, OTS and Registration
  (Freeze group Keyboard Harmony/Arpeggio). Arpeggio Quantize and Hold are System settings,
  not registrable; ArpVel, ArpGateT, ArpUnitM are registrable (DL parameter chart).
- **Arpeggio.** The pattern depends on the notes held; Quantize lines it up with Song or
  Style playback; Hold keeps it going after release until the button is pressed again;
  Arpeggio Hold is also a pedal function that holds only while it is on (OM p.58; RM p.41,
  p.141). ArpVel / ArpGateT / ArpUnitM are Live Control percentages of the type's defaults,
  HrmArpVol the level (RM p.147).
- **Vocal Harmony (out of scope):** its Chordal mode takes the chord from the chord
  section, the LEFT section, or a Song's chord data (OM p.80).

## Manual check

- Chord source, Echo-category tempo, Trill rule, detail settings, Voice Set, storage:
  *manual-confirmed* (OM p.56-58; RM p.41, p.46-47, p.141, p.147; DL p.74 and parameter chart).
- 1/32 as a Speed value, and the Minimum Velocity accent use: *video-confirmed* (V1), the
  latter also described in RM p.47. The manuals do not list the Speed values.
- V3's Voice Set point and V4's mono Left voice are consistent with RM p.41 and p.49
  (Mono/Poly per part); the lowest-note-first choice in V4 is *video-only*.
- Whether Echo repeats are counted from the key press or snapped to the beat grid: *video
  only, unclear*. V1 says it "runs in rhythm" with the tempo, which fits both. Question for
  the owner.

## yahaha today

- Every Harmony and Echo type, Multi Assign, the chord-source table, all detail settings
  and the Mono rule are implemented (`docs/harmony.md`); voicings are our own and marked
  (guess) where the manual is silent. Speed includes 1/32; Minimum Velocity is inclusive.
  Echo repeats count from the key press (guess), at the band tempo.
- Chord source: yahaha has no ACMP switch and one split point, so the harmony always
  follows the Style chord (the ACMP-on rows); the "LEFT between two splits" layout is not
  reachable (`docs/harmony.md`, "Wiring").
- Arpeggio engine with Quantize (Off / 1/8 / 1/16), Hold setting and a separate Hold pedal
  function, ArpVel/ArpGateT/ArpUnitM in the engine, Assign, Volume; 23 original patterns,
  no Yamaha pattern data by design (`docs/arpeggio.md`). The Live Control percentages are
  not on the Launchkey or in the app yet.
- Registration stores the `harmonyArp` item (docs/app-api.md). OTS recall sets the keyboard
  parts only (`src/session/ots.rs`), so a style's OTS does not bring its harmony type. No
  Voice Set: changing the Right 1 voice never changes the harmony type.
- No per-part Mono/Poly and no Left Hold (`docs/controllers.md` "Not done";
  `src/live/kbdfx.rs` passes `mono: [false; 3]`), so V4's counter-melody trick can't be
  done unless the patch itself is monophonic, and the Harmony rule "a Mono part counts as
  off" never triggers.
- The HARMONY/ARPEGGIO switch is on the Launchkey (button under fader 5) and a pedal.

## Gaps

| Gap | Priority | Suggested next step |
|---|---|---|
| Echo/Tremolo/Trill phase: from key press (yahaha) vs possibly beat-locked (Genos) | P2 | Owner hear-test: hold a note off the beat at 1/8 and listen against the style; change `EchoGen` to snap to the style grid if the Genos does |
| OTS recall does not set the Harmony/Arpeggio type and on/off | P2 | Check whether style OTS data carries the harmony settings; if so, apply them in `recall_ots` (with the `ots` topic) |
| No Voice Set: a voice change never recalls a harmony or arpeggio type (V3: synth voices arrive with their arpeggio) | P2 | Only meaningful once the sound library has per-patch defaults; add an optional "harmony type" to a patch plus a Voice Set Filter switch |
| Arpeggio Live Control percentages (ArpVel, ArpGateT, ArpUnitM) not reachable in the app | P2 | Expose them in the Harmony panel; later a Launchkey knob page |
| No per-part Mono/Poly and no Left Hold, so no mono-Left counter melody (V4) and the Harmony Mono rule is dead code | P2 | Belongs with the parts model (#31 remainder): a per-part Mono flag (lowest note of a new chord, then last-note priority) and Left Hold |
| Voicings of the Harmony types are guesses | P2 | Run the hear-test checklist in `docs/harmony.md` against a Genos (owner) |
| Preset arpeggio patterns | – | Out of scope (copyrighted data); own patterns only |
| Vocal Harmony | – | Out of scope (microphone features) |
