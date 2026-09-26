# Multi Pads, Chord Match, Audio Link pads, pad sync start

Topic id: `multipads` · Pass: 2026-09-25 · Manual: OM p.59, OM p.74-75, RM p.64-68 · yahaha: `docs/multipad.md`, `src/multipad`, `src/engine/multipad.rs`

Paraphrased notes only. No transcript text, manual text or frames are committed.

**Video coverage: partial.** V1 (the Scan masterclass, 35 min) was read in full, and two
stills were taken from it. V2-V5 were still rate-limited (HTTP 429). Two videos fetched for
the `ots` and `registration` topics (V6, V7) mention pads and are used for one point each.
The next pass should read V2-V5 (`research.py hits multipads`), especially V3 for the
Synchro Stop defaults.

## Sources

| # | Video (channel) | URL | Timestamps used |
|---|---|---|---|
| V1 | Yamaha Genos 2 - Multi Pad Masterclass (Scan Keyboards & Pianos) | https://www.youtube.com/watch?v=AKKU_yHrg-0 | 01:07-02:23, 04:25, 05:43-06:29, 08:38, 11:49-12:25, 13:51, 17:13-17:52, 21:10-21:38, 24:37, 26:37-27:28, 32:16; stills 06:12, 24:40 |
| V2 | YAMAHA GENOS MULTIPAD TUTORIAL (Leigh Wilbraham) | https://www.youtube.com/watch?v=FFj1Wb2g7dw | transcript pending |
| V3 | Multi Pad Sync Start/Stop (Alois Müller) | https://www.youtube.com/watch?v=UeCqDAnNFS4 | transcript pending |
| V4 | Audiolink Multipad on the SX900 and Genos (Keyboardhugo) | https://www.youtube.com/watch?v=Fub6M3gznkk | transcript pending |
| V5 | Stopping a multi pad (Keyboardseminare) | https://www.youtube.com/watch?v=4Ea0ep31SeE | transcript pending |
| V6 | Customize Style with your own OTS (Casper tutorSynth) | https://www.youtube.com/watch?v=RJtZxd9tJYs | 01:05-02:54 |
| V7 | Yamaha Genos 2 - Registration & Playlist Masterclass (Scan Keyboards & Pianos) | https://www.youtube.com/watch?v=oM2FkjRt6FA | 33:02 |

Stills taken: V1 06:12 shows the Home screen with the Multi Pad area (the bank name,
here a steel-guitar strum bank) beside the Style name, while the presenter moves the panel
sliders; V1 24:40 shows the MIDI Multi Pad Recording page: the bank name, four rows (Rec
button, pad name, a Repeat On/Off and a Chord Match On/Off switch per pad), Rec and Stop,
and a metronome switch. Still pending: the pad lamps in the flashing-red standby state and
the Style Setting page with the two Synchro Stop switches (V3).

## Genos behaviour (subtleties a player notices)

- **Banks of four.** A bank is four pads; SELECT opens the bank list; up to four pads play
  at once, each at the current tempo (OM p.59, p.74).
- **Start timing.** With the Style (or a MIDI Song) stopped, a pad starts at once; while the
  Style plays, a press waits for the top of the next measure (OM p.74). V1 presents this as
  a relief for players: press whenever you like, a late press just comes in on the next bar
  [V1 12:01-12:25].
- **Tempo lock.** Pads play at the Style tempo, so a bank can be auditioned before the
  Style starts and needs no tempo matching; a tempo change while a pad loops changes its
  speed at once [V1 04:25, 21:10-21:38].
- **Pads without the Style.** With the Style stopped, a Chord Match pad still follows the
  left-hand chord, so a bank can act as a small accompaniment on its own, or over a
  free-play (tempo-less) style to add a pulse [V1 08:38, 19:40-22:03].
- **Re-press restarts.** Pressing a pad that is playing restarts it from the top (OM p.74).
- **One-shot or loop.** Each pad is either one-shot (stops at its end) or Repeat (loops
  until stopped). A Repeat pad started while the Style plays loops in step with the beat
  (OM p.74; RM p.65).
- **Stopping.** [STOP] stops every pad; holding [STOP] and pressing a pad stops just that
  pad (OM p.74; shown at [V1 13:51]). [STOP] also clears every Synchro Start standby
  (OM p.75).
- **Stop lets the tail ring.** Stopping a pad does not cut the sound dead: the last notes'
  release and reverb ring on, so a stop anywhere sounds like a natural ending [V1 17:13-17:52].
- **Pads stop with the Style.** Stopping the Style with START/STOP also stops the pads
  (OM p.74). Since firmware 1.20 this is two Style Setting switches: Multi Pad Synchro Stop
  on Style Stop (repeating pads stop when the Style stops) and on Style Ending (repeating
  pads stop when an Ending starts) (RM p.12). A Song Setting switch does the same for MIDI
  Song stop (RM p.78). The manuals give no defaults.
- **Lamps.** Blue: pad has data; red: playing; flashing red: Synchro Start standby; off:
  empty (OM p.74).
- **Chord Match.** With ACMP on (or LEFT on with ACMP off), a Chord Match pad transposes its
  phrase to the chord in the chord section (or the LEFT section). The chord may come before
  or after the pad press. Some pads ignore the chord (OM p.74; RM p.65). Phrases are written
  on a CM7 basis with C, E, G, A, B, the same rule as style source patterns (RM p.65).
  V1 suggests recording a hard passage (a run, a key-change link) into a pad with Chord
  Match off so it plays exactly as written over any chord [V1 26:37-27:28].
- **Synchro Start.** SELECT + pad arms that pad (flashing red). With ACMP off any key starts
  it, with ACMP on a chord in the chord section; starting the Style also starts it. Armed
  during playback, the trigger starts it at the next measure. With several armed, pressing
  any one of them starts them all. Arming again, or STOP, disarms (OM p.75).
- **Registration and OTS.** The Multi Pad bank file is stored in OTS and in Registration;
  the Synchro Start standby and the Multi Pad part offsets (volume, pan, reverb, chorus, EQ
  gain) are stored in Registration, all under the Freeze group "Multi Pad" (DL parameter
  chart, Multi Pad rows). V1 stresses that choosing an OTS always loads a bank that suits
  the style (preset styles pick, for example, a drum-hit bank or a guitar bank)
  [V1 01:07-02:23, 10:19-10:46]. V6 shows that each OTS of a style holds one pad bank, and swaps
  the bank on OTS 1 and OTS 2 of a style and saves it with the style [V6 01:05-02:54]. V7
  notes that a registration also remembers Audio Link pads [V7 33:02].
- **Mixer.** The Panel mixer tab has a whole-Multi-Pad level beside Style, Left, Right 1-3
  and so on; an M.Pad tab balances the four Audio Link pads (OM p.90). V1 uses the Multi Pad
  level against the Style level three times in one session, as the normal way to sit a pad
  in the mix (for example pads up, Style down) [V1 05:43-06:29, 11:49, 20:06]. V1 also mutes
  the Style's own guitar parts (Channel On/Off) so a guitar pad replaces them
  [V1 12:37-13:16].
- **Recording.** Pads record to Song channels 5-8 (RM p.76, p.79).
- **Audio Link pads.** A pad can be a WAV link (44.1 kHz/16-bit) instead of a MIDI phrase:
  no Repeat, no Chord Match, per-pad Audio Level, a slightly longer load, and a
  "Simultaneous Play" switch (Off: a new audio pad stops the one playing, the pre-1.20
  behaviour). MIDI and audio pads cannot share a bank (RM p.66-68). V1 builds one from an a
  cappella vocal and sets its Audio Level at import [V1 27:40-28:31, 32:16].

## Manual check

- *Manual-confirmed and video-confirmed* (V1): next-bar start, STOP / STOP + pad, Chord
  Match, the per-pad Repeat and Chord Match switches, the Panel mixer Multi Pad level, OTS
  loading a bank (also V6), Audio Link level.
- *Video-only* (V1), no manual text either way: the tail ringing on after a stop; a tempo
  change taking effect in a looping pad at once; Chord Match pads following the chord with
  the Style stopped (the manual says ACMP or LEFT on, not that the Style must run, so this
  is consistent). No conflicts found.
- Synchro Start and Synchro Stop: *manual-confirmed*, video pending (V3).
- The manuals do not say: the Synchro Stop defaults; whether a one-shot pad also stops on
  Style stop (the switch text says "repeat playback"); what "in sync with the beat" means
  for a phrase whose length is not a whole bar; how a style file stores its OTS pad bank
  (by file name? by preset number?). These are the questions for the videos.

## yahaha today

yahaha matches the playing behaviour closely (`docs/multipad.md`, "Behaviour"):

- Four pads on channels 5-8, at the band tempo stopped or not (a tempo change re-anchors the
  pad clock, so a looping pad changes speed where it is), start at once when stopped / next bar line while playing,
  re-press restarts, up to four at once (`src/engine/multipad.rs`, `src/multipad/player.rs`).
- Repeat loops on the pad's own length, which the parser rounds up to a whole beat
  (`src/multipad/file.rs`), so a looping pad stays on the beat: matches "in sync with the
  beat".
- Stopping sends note-offs, not All Sound Off (`notes_off` in `src/multipad/player.rs`), so
  release tails and reverb ring on as on the Genos (V1 17:13).
- STOP, STOP + pad, Synchro Start (arm, fire on chord or band start, any armed pad fires
  all, STOP disarms), the lamp states (plus a "queued" state), Chord Match through the style
  engine's NTR/NTT with a CM7 fallback rule: all present.
- Multi Pad Synchro Stop: both switches, defaults Style Stop on / Style Ending off (a yahaha
  decision), one-shot pads always play out.
- Registration stores the bank file (`src/session/registration/sections.rs`), not the
  standby or the part offsets.
- Gaps: no Multi Pad level in the mixer (a pad's own CC7 only); OTS recall does not load a
  pad bank (`src/session/ots.rs` sets the keyboard parts only); no ACMP switch, so the
  ACMP-off triggers (any key starts armed pads; the LEFT chord drives Chord Match) are
  not reachable; no Audio Link pads; **the Launchkey pad page is a proposal only**
  (`docs/multipad.md`, "Launchkey"): pads are played from the terminal keys or the app.

## Gaps

| Gap | Priority | Suggested next step |
|---|---|---|
| No Launchkey control of the pads (only terminal keys and the app) | P1 | Wire the proposed page 5 "Multi Pads" (pads 1-4, STOP, arm row, STOP + pad row), actions straight to the engine from the MIDI thread |
| No whole-Multi-Pad level in the Panel mixer (and no pad offsets in Registration); V1 treats it as the everyday way to balance pads | P1 | Add a Multi Pad master level applied to ch 5-8 CC7 (scaled like the Style fade), store it in the `multiPad` registration item |
| OTS recall does not load the Multi Pad bank the OTS names (V1, V6: every OTS brings a matching bank). Kept at P2 because yahaha ships no Yamaha pad banks, so the named preset banks won't exist; it matters for user styles and banks | P2 | Find where the style's OTS data names the bank (the OTS tracks of a user-saved style from the owner), then load it from the pad library on OTS recall |
| Synchro Start standby not stored in Registration | P2 | Store armed pads with the bank; re-arm after the bank loads |
| Synchro Stop defaults are a guess | P2 | Confirm from V3 or on a Genos (owner question) |
| Audio Link pads (WAV) | – | Out of scope while yahaha plays MIDI only; revisit if the built-in synth gains a sample player |
