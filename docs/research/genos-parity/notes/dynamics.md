# Style Dynamics Control, Ambience, style volume by touch

Topic id: `dynamics` · Pass: 2026-09-25 · Manual: OM p.11, OM p.69, RM p.11, RM p.142, RM p.147, DL p.91 (Parameter Chart) · yahaha: `docs/genos-features.md#style-dynamics-control-transport-settings`, `src/engine/dynamics.rs` (on `develop`), issue #180

Paraphrased notes only. No transcript text, manual text or frames are committed.

## Sources

| # | Video (channel) | URL | Timestamps used |
|---|---|---|---|
| V1 | What is Style Dynamics Control (Yamaha Global) | https://www.youtube.com/watch?v=7aUvXAcsNXs | 00:22–01:17 (transcript); still 00:40 |
| V2 | Genos 2: What is Style Dynamics Control? (MUSIC 2000) | https://www.youtube.com/watch?v=NLsARnEdTXM | 00:24–01:14 (a repost of V1 with auto captions; adds nothing new) |
| V3 | Richard Bower demonstrates Style Dynamic Control (Rimmers Music) | https://www.youtube.com/watch?v=NesMrYjrP9U | 00:03–00:42 |
| V4 | Genos 2: Dynamics Control mit Schweller FC-7 steuern (Keyboardseminare, German) | https://www.youtube.com/watch?v=BNPkyB5y_-Y | transcript pending (HTTP 429); stills 00:45, 01:30, 02:30 |
| V5 | Yamaha Genos 2 Demo 3: Style Dynamic Control (Pihlajamaan Musiikki Pori) | https://www.youtube.com/watch?v=5Wq1-_GZF60 | transcript pending (HTTP 429) |
| V6 | Genos 2 Demo & Review (Woody Piano Shack), see style-feel.md | https://www.youtube.com/watch?v=uAUPavLttMc | 17:16–18:20 |
| V7 | Genos2 Demo (Sweetwater Soundcheck), see style-feel.md | https://www.youtube.com/watch?v=Q8BLjKaFGy4 | 05:54–06:45 |

Video coverage: V1 and V3 read in full (both short: a promo and a 1.5-minute demo). V1 and V2 are the same Yamaha script. Most of their length is music, so the audible curve (how soft is "soft") is not documented in text. The pedal-driven use (V4) is known from its title and stills only. V6 and V7 are longer reviews, read in full for style-feel.md; their Dynamics passages are used here.

Still used (private folder): `7aUvXAcsNXs_0040_0.jpg`. It shows a vertical fader labelled SOFT at the bottom and HARD at the top, with a pp–p–mp–mf–f–ff scale beside it, pushed to ff. Below it is a row of Intro, A, B, C, D and End blocks with C highlighted. The picture's point: the same section (Main C) plays anywhere from pp to ff.

V4 stills:
- `BNPkyB5y_-Y_0130_0.jpg`: the presenter on the Genos2 Menu page (Style Setting, Assignable and the other setup icons are visible), apparently on his way to the pedal assignment.
- `BNPkyB5y_-Y_0230_0.jpg`: the Home screen, with his finger on the Live Control knob/slider block (LED rings lit).

The title says outright that he drives Dynamics Control from an FC-7 expression pedal.

## Genos behaviour (subtleties a player notices)

- **What it is.** A continuous control over how hard the band plays, for any Style, user Styles included. Yamaha's pitch: until now, each section's energy was fixed [V1 00:22–00:47]. *manual-confirmed* (OM p.11).
- **What it changes.** Turning up makes the Style parts play harder, turning down softer. It changes the parts' **velocity**, not just their volume, and that includes drums and percussion [V1 00:47–01:17]. V3 names drums, bass and the acoustic instruments as the parts whose velocity drops [V3 00:30]. *manual-confirmed* (RM p.142 and p.147 say it changes the intensity of playback, not only its level). Neither source says whether it thins parts or changes voicings. Both describe velocity only, so read it as per-note velocity scaling across all parts.
- **Where it applies.** Every section, Intros and Endings included [V1 01:17]. *video-only*: the manuals don't say which sections it covers.
- **How it relates to Main A–D.** A–D stay the verse/chorus energy steps. Dynamics shades the band softer or harder *inside* one variation [V3 00:03–00:18; the V1 still, where Main C runs from pp to ff]. *video-only*, and consistent with RM p.142.
- **The controller.**
  - The default is Live Control **Knob Assign Type 2, knob 3** [OM p.69]. It is also a Live Control function ("DynCtrl", RM p.147).
  - It can go on an assignable **foot pedal** but not on the Assignable buttons [RM p.142]. It carries the "*" mark: it needs a continuous foot controller (an FC7-type expression pedal), and a footswitch won't do [RM p.139]. V4 shows it done in practice with an FC-7 (title and stills only).
  - V3 turns a knob [V3 00:18–00:42].
- **The on/off gate.** Style Setting has a "Dynamics Control" switch that decides whether the control may act on the Style at all [RM p.11]. The Parameter Chart marks it as a System (Backup) setting only: not Registration, not OTS, not Style data [DL p.91]. *manual-confirmed*.
- **Conflict: touch.** OM p.69 describes the knob-3 parameter as setting Style volume according to how hard you play. RM p.142 and p.147 and both videos describe a hand-set intensity control; no video shows the band following your touch. The OM wording is probably carried over from the older Style Setting "Dynamics Control" on Genos1/Tyros, a touch-to-style-volume range. genos-features.md records it as Off/Narrow/Medium/Wide, from memory; it is not in the Genos2 manuals. *conflict (manual vs manual)*: the RM and the videos win for the spec. yahaha's "Touch" mode (#180) is the owner's extension, not Genos2 behaviour.
- **The curve and range.** Not specified anywhere (genos-features §G.7). Reviewers disagree:
  - Sweetwater's presenter calls it a huge range within one Main [V7 05:54–06:22].
  - Woody finds it subtle, about mf to f, and wishes it reached ppp–fff [V6 17:41–18:20]. He also says he rarely touches it mid-song [V6 17:16].

  *video-only*.
- **Ambience Depth.** Sets the wet/dry balance of the **Ambient Drums/SFX** kits when the Style's Rhythm 1/2 use them. The default is Live Control Knob Assign Type 2, knob 1 [OM p.69, p.52; RM p.147]. Style Creator's Drum Setup has a per-instrument Ambience Depth [RM p.33]. The ambience is a sampled room sound, which Yamaha says DSP effects can't easily copy [OM p.52]. It does nothing on a Style whose rhythm parts use ordinary kits [OM p.69 note]. *manual-confirmed*. In V7, bringing the Ambient Drums in on top of Dynamics is the moment the demo lifts [V7 06:22–06:45]; V6 rarely uses them [V6 17:16].

## Manual check

- The videos and RM p.142/p.147 agree: an intensity (velocity) control, not a volume fader.
- OM p.69's "playing strength" wording conflicts with the RM and the videos (see above).
- Sections covered (all, Intros/Endings included) and parts covered (drums included) are *video-only* (V1), stated by Yamaha itself.
- Storage: System only (DL Parameter Chart p.91). Matches #180's decision.

## yahaha today

- **This branch (`main`):** nothing. There is no dynamics code.
- **`develop` (PR #184, merged 2026-09-25):** `src/engine/dynamics.rs`.
  - A level 0–127 (64 plays the Style as written) scales the velocity of **every Style note-on on all eight parts**, from ×0.35 at 0 to ×1.6 at 127, clamped to 1–127 so no note drops out.
  - It is applied in `note_on` (`src/engine/playback.rs`), so it covers Intros, Fills and Endings. That matches V1.
  - CC7 is untouched (the mixer rule). A `control` flag (default on) is the Style Setting gate.
  - Two yahaha extensions: **Touch** (a chord-section strike sets the level to velocity − 36) and **Accent** (a hard chord-section strike plays the Main's own fill, as Fill Self).
- **Not reachable yet.** The API (#180 part 2) is on branch `accent/api-session`, unmerged. The UI, TUI key and Launchkey mapping (part 3) aren't started. The Launchkey knobs are unmapped in yahaha so far (docs/solo-metronome.md). The pedal table (`src/controllers.rs` `Function`) has no Dynamics Control row, and docs/controllers.md lists "a Volume pedal" as not done.
- **Timbre.**
  - On AU plugins the scaled velocity reaches the instrument, so its velocity layers and filters respond (docs/plugin-hosting.md, mixer rule).
  - On the built-in SoundFont synth, rustysynth ignores SF2 modulators such as velocity→filter (#127). There Dynamics mostly changes level, plus sample-layer switches where a preset has velocity zones. The Genos "harder, not louder" character only partly comes through.
- **Ambience Depth:** none. yahaha has no Ambient Drums kits, and no knob function aims at the Rhythm parts' reverb send.

## Gaps

| Gap | Priority | Suggested next step |
|---|---|---|
| The engine plays Dynamics, but a player can't reach it: no API, UI, knob or pedal | P1 | Finish #180 parts 2–3. Put the level on a Launchkey knob (the first knob mapping) and in the app next to the section buttons. |
| No "Dynamics Control" continuous pedal function (RM p.142, foot controller only) | P1 | Add a `continuous` row to `controllers::FUNCTIONS` that sets the level from the pedal position (Range Full/Upper/Lower as for Pitch Bend). It works with an expression pedal on CC 11 or CC 4. |
| On SoundFonts, Dynamics sounds like a volume change (no velocity→filter) | P2 | Tie to #127: give rustysynth voices the SF2 default velocity→filter-cutoff modulator. Then A/B a Main at levels 20/64/110. |
| The curve (×0.35…×1.6, the same for every part) is a guess; reviewers call the Genos range anything from subtle (V6) to huge (V7) | P2 | Owner A/B against a Genos2 recording (V4/V5 audio). Consider a gentler floor for drums, or per-part depth (#180 follow-up). |
| Touch (the style follows the left hand) is not Genos2 behaviour; OM p.69 is ambiguous | – | Keep Touch off by default and label it a yahaha extension in docs/genos-features.md. |
| Ambience Depth: no Ambient Drums kits | P2 | Optional proxy: a knob function that sets CC91 (reverb send) on Rhythm 1/2 only, documented as a proxy. Or skip until a plugin drum kit with room mics is routed. |
