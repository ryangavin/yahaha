# Song, lead sheet, Score/Lyrics and playing along

Topic id: `song-score` · Pass: 2026-09-25 · Manual: OM p.76-89, RM p.69-78 · yahaha: `docs/ireal.md`, `src/ireal`

Paraphrased notes only. No transcript text, manual text or frames are committed.

**Video coverage: both chosen videos read** (V1, V2). No stills: V1 is a computer
application and V2's screens are the Multi Pad recording page already captured for the
`multipads` note.

The Song player itself is out of scope for yahaha (`docs/genos-features.md` §F). This note
only asks what a Genos player gets from Songs *while playing with a Style*, and whether
yahaha's iReal chart player covers that need.

## Sources

| # | Video (channel) | URL | Timestamps used |
|---|---|---|---|
| V1 | How To Use MIDI Song To Style (Yamaha Global) | https://www.youtube.com/watch?v=yYfPbMFM5wo | 00:00-00:40, 02:14-02:41, 03:37-05:13, 05:44-06:35, 07:21 |
| V2 | Playback of the solo part and synchronization with the Style that we play with the left hand (Casper tutorSynth) | https://www.youtube.com/watch?v=OiWiZm7zYmQ | 00:34-02:58 |

Stills: none needed (see above).

## Genos behaviour (subtleties a player notices)

- **Song plus Style.** When a MIDI Song and a Style play together, the Style replaces the
  Song's channels 9-16 and the player supplies the chords with the left hand. Recipe:
  Song Synchro Start on, ACMP and SYNC START on, then START/STOP or a chord starts both
  (RM p.75).
- **Solo backing is done with a Multi Pad, not a Song (V2).** To get help with a solo the
  player can't play, V2 records the solo into a Multi Pad: start from a factory bank, turn
  Chord Match off and Repeat on, record on Right 1 with the drums running (recording starts
  at the first key), trim, save to User; then trigger it with the Style while the left
  hand plays the chords [V2 00:34-02:58]. The video title says "synchronised with the
  Style", and that is the pad's next-bar start and tempo lock.
- **The Song leads.** In that mode the Song's tempo is used; Style Retrigger is not
  available; stopping the Song also stops the Style unless the Song Setting "Style Synchro
  Stop" is off (RM p.75, p.78). A Song Setting switch also stops repeating Multi Pads with
  the Song (RM p.78). Song PLAY/STOP stops Song, Style and pads together (OM p.74).
- **Chords in a Song.** A Song can carry chord events (the Yamaha chord meta event, the
  same data as the Chord SysEx; `docs/genos-features.md` C.10). The Score display can show
  them as chord symbols, and Vocal Harmony's Chordal mode can follow them (RM p.72; OM p.80).
  Chord and section-change events entered in Song step recording make no Style sound
  until they are expanded into note data (RM p.95-96), so the Genos does not drive its live
  Style from a Song's chord events.
- **Reading while playing.** The Score shows notation with a bouncing ball, optional chord
  symbols, lyrics and note names; the Lyrics page highlights the current words; the Text
  viewer shows any .txt file, which players use for chord charts and performance notes
  (OM p.87-88; RM p.70-74).
- **Guide modes** (Follow Lights, Any Key, Your Tempo, Karao-Key) make the Song wait for
  the player (RM p.77). Practice features, not accompaniment features.
- **Song Position markers.** SP1-SP4 and Loop let a player jump to a marker at the end of
  the current bar, or loop between markers, "arranging on the fly" (OM p.85-86).
- **MIDI Song to Style** is a separate Yamaha computer program that converts a MIDI Song
  into a Style file (OM p.15, list of documents). It is a content tool, not an instrument
  feature; its output is an ordinary style file. V1: it analyses any SMF and proposes
  sections automatically (Easy mode); in Edit mode you pick which bars become which
  section, assign MIDI tracks to Style parts, mark melody tracks to leave out, and set the
  source key per section so the chord conversion works; it can audition sections on a
  connected instrument, with the chord coming from a palette or, with ACMP off on the
  instrument, from the chords you play; Genos2 voice list by default
  [V1 00:00-00:40, 02:14-02:41, 03:37-05:13, 05:44-06:35, 07:21].

## Manual check

- Song, Score, Lyrics, Text and Song + Style bullets: *manual-confirmed* (OM p.74, p.76-89;
  RM p.70-78, p.95-96); no video in this pass shows them.
- V2's pad recipe is consistent with RM p.64-65 (Right 1 recorded, Repeat and Chord Match
  per pad, recording with the Style's rhythm); *manual-confirmed* apart from the workflow.
- MIDI Song to Style details: *video-only* (V1); the OM only names the application.

## yahaha today

- No MIDI Song player, Score, Lyrics or Text viewer (out of scope, `docs/genos-features.md` §F).
- **The iReal chart player covers the "chords from a song drive the Style" case, and
  goes further than the Genos:** the chart gives the chords at their beat and the Main
  sections, with Intro/Ending, choruses, a loop, auto fills, and a left-hand override until
  the next bar line (`docs/ireal.md`, "Chart player"). The lead-sheet band in the app
  shows the chords, much as the Score's chord line or a Text chart does on the Genos.
- The chart player's loop (`setChartLoop`, whole song or a section) is the nearest thing to
  Song Position markers + Loop.
- The V2 use (a solo phrase in time with the band, fixed pitch) is already playable in
  yahaha **if the phrase is a `.pad` file**: a pad with Chord Match off and Repeat on starts
  on the next bar at the band tempo (`docs/multipad.md`). What yahaha lacks is a way to
  make one: no pad recorder (Multi Pad Creator is out of scope, §F) and no import of a
  MIDI file as a pad; only `yahaha pad --demo` writes synthetic banks.
- A MIDI file's chord events cannot be imported as a chart; Chord SysEx is only read by
  the capture kit (`docs/capture-kit/README.md`).

## Gaps

| Gap | Priority | Suggested next step |
|---|---|---|
| No MIDI Song (melody/solo track) playing with the Style | – | Out of scope (§F); V2 shows Genos players solve this with a Multi Pad instead (next row) |
| No way to make a user pad (the V2 solo-backing recipe) | P2 | Import one channel of a MIDI file as a pad into a user bank (reuse the `synthetic.rs` bank writer), with Repeat and Chord Match switches; cheaper than a pad recorder |
| A MIDI file's chord events (Yamaha chord meta / XF chords) can't become a chart | P2 | Add an SMF chord-track importer that builds the same `PlanBar` list the iReal importer does, so Song files with chords play in chart mode |
| No lyrics / text notes beside the chart | P2 | Optional: a free-text notes field per chart song in the app (the Genos Text viewer use) |
| MIDI Song to Style output | – | Nothing to do: it writes ordinary style files, which yahaha loads like any other; if the owner has converted styles, one is a good spot-check of odd source keys per section (V1 05:44) |
