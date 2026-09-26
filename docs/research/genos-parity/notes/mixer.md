# Mixer, part volume behaviour, style part mute, balance

Topic id: `mixer` · Pass: 2026-09-25 · Manual: OM p.90-93, RM p.12-13, RM p.129-137, RM p.148, DL p.82-84 · yahaha: `README.md#controls`, `src/engine/mixer.rs`, `src/session/mixer.rs`, `src/api/mixer.rs`, `docs/architecture.md` (the mixer rule), `docs/fills-and-rules.md#change-behavior-26`, `docs/registration.md`

Paraphrased notes only. No transcript text, manual text or frames are committed.

**Video coverage this pass: 2 of 2 chosen, plus 2 from live-control.** V1 is about the MIDI Song
mixer (Setup and save), so it counts only as evidence for how the Genos mixer keeps (or doesn't
keep) edits; V2 is short. V3 and V4 are the live-control videos, used for the Balance and Style
slider pages.

## Sources

| # | Video (channel) | URL | Timestamps used |
|---|---|---|---|
| V1 | PSR-SX920 & Genos 2 tips part 3: save mixer changes, revoice tracks (ePianos.co.uk) | https://www.youtube.com/watch?v=yV8OPw0esiY | 01:05–02:37, 04:40–05:36, 09:08–09:33 |
| V2 | Genos Style Mute feature (A&C Hamilton) | https://www.youtube.com/watch?v=Gvf57O5E7ig | 00:07–00:57 |
| V3 | Genos 2: how to use Live Controls & Assignable buttons (MUSIC 2000) | https://www.youtube.com/watch?v=QeVghgD5jN8 | 00:54–01:31 (still 01:02) |
| V4 | Live Control sliders, explanation and examples (Casper tutorSynth) | https://www.youtube.com/watch?v=5RM9Oo5yiw0 | 02:49–03:02, 04:40–05:26, 05:43 |

Stills (private folder): `QeVghgD5jN8_0102_0/1` (+ a zoom). The slider sub-display on the Balance
page reads, sliders 1–9: Style, M.Pad, Left, Right1, Right2, Right3, SongA, SongB, Mic (values 100,
90, 70, 100, 100, 100, 100, 100, 100 at that moment). Wanted but not taken: a Mixer › Style tab still
with a channel's voice icon (V1 is the Song tab only).

## Genos behaviour (subtleties a player notices)

- Mixer tabs: Panel (the whole Style, the whole Multi Pad, Left, Right 1–3, Song A/B, Mic, audio
  inputs), Style (8 channels), M.Pad, Song, Master; per part: Filter (cutoff, resonance), EQ
  (high/low), Effect (insertion depth), Chorus/Reverb depth, Pan/Volume. Touch-and-hold a value resets
  it. OM p.90–91, RM p.129–135. *manual-confirmed*
- The Genos's default slider page is "Balance", fixed: Style, Multi Pad, Left, Right 1–3, Song A,
  Song B, Mic. Presenters ride it to balance the band against the right hand while playing.
  [V3 00:54–01:18 and still 01:02; V4 02:49–03:02], RM p.145. *manual-confirmed* (fixed page);
  the order is from the still.
- Slider Assign Type 1 by default holds the 8 Style channel volumes: a per-song band mix, stored in a
  registration. [V3 01:18–01:31; V4 04:40]. *video-only* for the default contents; RM p.145–146 lists
  Volume as a slider function.
- Style channel on/off from the Mixer or Channel On/Off; touch-and-hold a channel to solo it
  (purple), touch again to cancel. OM p.92. *manual-confirmed*; V1 shows the same solo gesture on the
  Song tab [V1 04:40–05:23].
- Style Track Mute A/B on Knob Assign Type 3, knobs 4 and 5: A starts from the drums and adds the
  band, B adds the drums last; players "twiddle" them live and store the knob setup in a
  registration. [V2 00:07–00:57], RM p.148. *manual-confirmed* (functions and orders)
- Changing a Style channel's voice: touch its instrument icon in Mixer › Style. OM p.93.
  *manual-confirmed*. V1 shows the same revoicing for Song channels [V1 04:40–09:08].
- What persists: Panel mixer settings go to Registration; Style mixer settings (voice, volume, pan,
  sends, filter, EQ) are Style data, kept only by saving a User style, and are also Registration
  items (group Style). Changing style loads the new style's mix. OM p.91, DL p.83. *manual-confirmed*.
  V1 shows the user-facing consequence for Songs: edits that aren't saved the right way come back
  reset [V1 01:05–02:37].
- Change Behavior › Part On/Off (Lock / Hold / Reset) decides whether channel mutes survive a style
  change. RM p.13. *manual-confirmed*
- Registration stores the Style volume **offset**, the Multi Pad volume offset, and each keyboard
  part's pan, volume, reverb, chorus, filter and EQ. DL p.82–83. *manual-confirmed*

## Manual check

- No conflicts. The Balance order and the default Slider Type 1 contents come from the still and
  the videos (the RM says only that Balance is fixed).

## yahaha today

- Two fader pages: Panel (faders 1–4 = Right 1, Right 2, Right 3, Left volume; faders 5–8 unused;
  button 5 Harmony/Arp) and Style (faders 1–8 = the 8 Style parts, buttons mute). Soft takeover on
  every fader, master = synth output. README "Mixer", `src/engine/mixer.rs`.
- Levels are CC7 only (the mixer rule, docs/architecture.md); a style load resets the Style faders,
  levels the player set hold against pattern CC7 across sections. Change Behavior Part On/Off
  Lock/Hold/Reset (#26). Style solo and part solo exist (`SetStyleSolo`, `SetPartSolo`). Style Track
  Mute A/B exists as the `styleTrackMute` command (not on a knob; see live-control.md).
- Registration stores the player-set Style part levels and mutes and the keyboard parts' CC7
  (docs/registration.md). Matches the Genos in effect (the Genos stores an offset).
- Not there: a whole-Style volume (the Genos's Style offset) and a whole-Multi-Pad volume on the
  Panel page; per-part pan, reverb/chorus depth, filter, EQ or insertion depth for keyboard or Style
  parts (`MixerCmd` has volume, mute, solo, master only); per-channel Style revoicing. The nearest is
  the sound library's per-style program override (`SetProgramOverride` with the current style), which
  replaces a program for every part that sends it and is kept in the library, not in a registration.

## Gaps

| Gap | Priority | Suggested next step |
|---|---|---|
| No whole-Style (and whole-Multi-Pad) volume on the Panel page, so band-vs-hands balance needs 8 faders | P1 | Owner decision against the mixer rule: a Style offset applied to the outgoing CC7 like the Fade exception (`src/engine/fade.rs`), on a Panel fader (faders 5–8 are free), stored in Registration as the DL's offset |
| No per-part pan and reverb/chorus depth (keyboard parts first, then Style parts) | P1 | `SetPartPan` / `SetPartSend` (CC10/91/93) with state; app mixer knobs; Registration `parts` fields; OTS recall uses the same path (ots.md) |
| No filter (cutoff/resonance), EQ or insertion depth per part | P2 | After pan/sends: CC74/71 and the XG part EQ parameters per part |
| No per-channel Style revoicing from the mixer (and not stored in Registration) | P2 | A Style-part voice override for the current style (channel, not program), kept until style change, stored in Registration's `styleMixer`; decide how it relates to the library's per-style program override |
| Touch-and-hold / double-click reset of a mixer value to its default | P2 | App mixer: reset gesture to the style's (or voice's) own level |
| Style mixer edits can't be saved into a User style | P2 | Later, with a style writer (also needed for user OTS, ots.md) |
