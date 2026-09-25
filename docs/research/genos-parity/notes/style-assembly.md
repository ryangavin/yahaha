# Style Creator, Assembly, Groove and Dynamics (read for what performers value)

Topic id: `style-assembly` · Pass: 2026-09-25 · Manual: RM p.20–33 · yahaha: `docs/genos-features.md#f-explicitly-out-of-scope-found-in-the-manuals`, `docs/sound-library.md`, `docs/solo-metronome.md`

Paraphrased notes only. No transcript text, manual text or frames are committed.

Style Creator is out of scope as an editor (genos-features §F). This note asks one thing: which of its
effects do performers actually want while they play, and could yahaha offer them as playing-side
features without an editor?

## Sources

| # | Video (channel) | URL | Timestamps used |
|---|---|---|---|
| V1 | Style Creator: Assembly (LeeABakerMusic) | https://www.youtube.com/watch?v=Vbj5zYZObyw | 00:26–04:27 (what Assembly is, a swap demo), 05:01–08:19 (the pattern-length trap), 09:27–12:19 (Bar Copy + pattern length) |
| V2 | Style Creator: groove, dynamics and velocity (LeeABakerMusic) | https://www.youtube.com/watch?v=RpgYNnfVF34 | 01:11–01:37, 05:20–07:27 (Groove: Beat Converter, Swing, Fine), 08:32 (why he uses Groove), 09:02–12:53 (Dynamics, Boost, a comparison with Live Control Dynamics), 13:55–14:21 (preset-style edit limits) |

Video coverage: both transcripts read in full. No stills: the step-edit clock values he reads out are enough.

## Genos behaviour (subtleties a player notices)

- **How a Style is built.** A Style is a set of sections, each with eight channels, and each channel holds its own "source pattern". Style Creator can record one channel at a time, or take a channel's pattern from another Style [RM p.20]. *manual-confirmed*.
- **Assembly.** For every channel of the current section you pick which Style, section and channel it comes from [RM p.26]. That is how players make "the drums of style X under the bass and chords of style Y". An Audio part cannot be moved to another Style [RM p.20, p.26]. *manual-confirmed*.
- **Groove.** Applies to all channels of a section [RM p.27]:
  - Beat Converter re-times 8ths as triplets, or the reverse.
  - Swing A–E delays the off-beats.
  - "Fine" templates push some beats early or drag them late, in three strengths.

  *manual-confirmed*.
- **Dynamics.** Per channel or for all channels [RM p.27]:
  - An Accent Type chooses which notes are emphasised, with a Strength.
  - Expand/Compress widens or narrows the velocity range around its centre.
  - Boost/Cut raises or lowers every velocity.
  - A separate Velocity function scales one channel's velocities by a percentage.

  *manual-confirmed*. All of these are offline edits that are saved into the Style. None of them is a live control; the live counterpart is Style Dynamics Control (see `dynamics.md`).
- **Drum Setup.** Per-instrument edits of a Style's drum kit: level, pan, pitch, reverb and chorus sends, **Ambience Depth** for Ambient kits, and note-off receive [RM p.28–33]. *manual-confirmed*.
- **Live alternatives the Genos already has, without the Creator:**
  - Style Mixer voice changes for any Style channel, saved with the Style (genos-features §8).
  - Part on/off.
  - Style Track Mute A/B knobs (RM p.148).
  - Style Dynamics Control (RM p.142, p.147).

- **What performers do with Assembly** [V1 00:26–04:27]:
  - Borrow one channel (for example Rhythm 2 from another Style's Main D) under the current Style's other parts, to make "something unique".
  - Copy one Main's drums to the other Mains so the feel stays the same across variations.

  Empty source channels simply stay silent [V1 03:38]. *video-only* for the use; the mechanism is *manual-confirmed* (RM p.26).
- **Pattern length matters.** A copy takes only as many bars as the target section's current length. Lengthen first, copy, then trim with Bar Copy [V1 05:01–12:19]. *video-only*; the manual doesn't warn about it.
- **Groove in numbers** (at 1920 ticks per beat) [V2 05:20–07:27]:
  - Beat Converter 12 moves the off-beat 8th from tick 960 to 1280 (triplet).
  - Swing C moves it from 960 to 1200 (a moderate swing).
  - Fine pushes or drags chosen beats.

  Groove affects all channels [V2 01:11–01:37]. *manual-confirmed* in kind (RM p.27); the tick values are *video-only*.
- **Why he uses Groove.** Re-feeling a Style he likes gives him, in effect, a whole new set of Styles [V2 08:32]. That is the performer value: the same arrangement with a different feel.
- **Dynamics accent types** are poorly documented, even for power users. One type, for example, lifts every other note to full velocity. Boost/Cut is relative, so a percentage of a low velocity stays low. He compares Expand/Compress to the live Style Dynamics Control and hears something similar [V2 09:02–12:53]. *conflict*: the manual has no accent-type table, so the video is the only evidence of what they do.
- **Editing limits on preset Styles.** On Yamaha's preset Styles only the rhythm channels can be edited (Bar Copy); the other channels are locked [V2 14:09]. *video-only*. The presenter guesses copyright protection.

## Manual check

- Every Creator function above is in RM p.20–33. Groove and Dynamics are offline edits applied with Execute/Undo [RM p.26–27], not real-time.
- Assembly swaps, pattern length and the lock on preset non-rhythm channels are *video-only*. Groove tick values are *video-only* (they come from step edit).

## yahaha today

- **No Style Creator.** It is out of scope by design (genos-features §F).
- **Part voices:** the sound library's program map picks the patch each Style part plays (docs/sound-library.md). The owner's "favourite voices" are therefore already a playing-side stand-in for a Style Mixer voice change, and they cover every Style at once.
- **Part on/off:** Style-page fader buttons mute parts. The Style Track Mute A/B knob function is in the API (`styleTrackMute`, docs/app-api.md), but no Launchkey knob drives it yet.
- **Velocity and feel:** Style velocities pass through unchanged, and timing is tick-exact (#127 audit). Style Dynamics (#180, `develop`) scales velocity live. There is no groove or swing adjustment and no humanise.
- **Assembly-like swaps:** none. A part always plays its own Style's pattern.

## Gaps

| Gap | Priority | Suggested next step |
|---|---|---|
| No per-part pattern swap ("drums from style X"), the thing performers use Assembly for | P2 | Playing-side "Part Source": for Rhythm 1/2 only, borrow the rhythm channels of another loaded Style, section for section, without saving a file. Needs a design; tempo and length must match. Owner question first. |
| No live swing/groove amount. Performers re-feel a Style to get "new" Styles | P2 | A live "Swing" amount that shifts off-beat 8ths (or 16ths) toward the triplet position, for example Swing C ≈ tick 960→1200 at 1920 ppq [V2]. Make it a knob or setting per style, not a Creator edit; the engine timing is exact, so this is a small tick remap in playback. Useful for #127 only if the owner wants a different feel from the Style's. |
| No per-part velocity trim (Creator Velocity / Boost-Cut) | P2 | Fold into Dynamics as a per-part depth (#180 follow-up), not a separate editor. |
| Style Creator editor | – | Out of scope (genos-features §F). No change. |
