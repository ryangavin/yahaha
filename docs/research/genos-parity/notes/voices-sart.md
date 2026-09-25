# Voice features: Super Articulation, ART buttons, organ flutes, Ensemble

Topic id: `voices-sart` · Pass: 2026-09-25 · Manual: OM p.52–55, OM p.71–73, RM p.35–38, RM p.56–63, RM p.139, RM p.141 · yahaha: `docs/sound-library.md`, `docs/plugin-hosting.md`, `docs/controllers.md`, `src/controllers.rs`

Paraphrased notes only. No transcript text, manual text or frames are committed.

The sounds themselves (S.Art/S.Art2 samples, AEM, Organ Flutes modelling) are tied to Yamaha's sample content, and genos-features §F puts them out of scope. This note asks what a software player could still offer: the *controls* around those voices, played through the user's own plugins.

## Sources

| # | Video (channel) | URL | Timestamps used |
|---|---|---|---|
| V1 | What is Super Articulation+ and Super Articulation 2 (Yamaha Global) | https://www.youtube.com/watch?v=J-UZ2Zwk4uQ | 00:38–02:07 (S.Art+), 02:49–04:09 (S.Art2 head/body/tail) |
| V2 | Yamaha Genos Right Hand Voices (Genos Genie) | https://www.youtube.com/watch?v=32wLa0Coy0Q | 00:41–02:38 (Right 1–3 on/off and layering), 02:51–04:29 (voice select, EXIT) |
| V3 | Yamaha Genos: The Hammond Sound (Genos Genie) | https://www.youtube.com/watch?v=4y19RN9FC-g | music only; no usable transcript |
| V4 | Ensemble Voices (Casper tutorSynth) | https://www.youtube.com/watch?v=3N2fUOzJQks | no English captions; not used |

Video coverage: V1 and V2 read in full. V3 is a performance with no speech. V4 has no English track. The Organ Flutes and Ensemble claims below rest on the manuals only.

## Genos behaviour (subtleties a player notices)

- **Layering.**
  - Right 1–3 each have a PART ON/OFF button and their own voice. Any combination sounds together (organ + vibes, guitar + vibes), and Left is a fourth part [V2 00:41–02:38].
  - VOICE SELECT opens the picker for a part. EXIT always climbs back to Home [V2 02:51–03:34].
  - *manual-confirmed* (OM p.48–49; genos-features §5).
- **S.Art (automatic).** How you play picks the articulation: a legato step on a sax sounds as one breath, and a firm legato on a guitar slides [OM p.71]. *manual-confirmed*.
- **S.Art2 / AEM (automatic).**
  - Each note is built from a sampled head, body and tail, chosen and joined in real time.
  - Legato, staccato and big jumps each get their own joins. Some note-off effects happen by themselves on long notes [V1 02:49–04:09; OM p.71].
  - Each S.Art2 voice brings its own vibrato, on the joystick [OM p.71].
  - *manual-confirmed*.
- **ART 1–3 buttons.** A button lights blue when the current voice offers an effect [OM p.72–73]. Three behaviours:
  1. **One-shot:** a noise such as sax breath/key noise or guitar fret/body noise. Red while it sounds.
  2. **Hold-to-change:** harmonics while held on NylonGuitar SW. Red while held.
  3. **Armed (S.Art2):** a bend, glissando or fall that fires on the next key-on or key-off. The button flashes red while armed and pressing it again cancels. Holding it lets the effect repeat.

  One ART press affects every part with an S.Art voice. With an armed effect on both Right and Left, playing one part leaves the other armed [OM p.73]. *manual-confirmed*.
- **ART on pedals.** Articulation 1–3 are assignable to pedals and buttons [RM p.139]. Pedal 2 defaults to ART.1 (genos-features §10). *manual-confirmed*.
- **Super Articulation+ (PSR-SX920/720 only).**
  - Four articulations live in one voice (for strings: bowed, tremolo, spiccato, pizzicato).
  - They switch by joystick up (bowed → tremolo) or Assignable buttons and footswitches (spiccato, pizzicato) [V1 00:38–02:07].
  - *Not in the Genos2 manuals*: a newer SX technology. It is essentially keyswitching mapped to panel controls.
- **Organ Flutes.** Footage levers, rotary speaker slow/fast (only with a Rotary effect), vibrato on/off/depth/speed, response, and attack mode First/Each [OM p.53].
  - With an Organ Flutes voice, special slider assign types turn the sliders into drawbars for each part [OM p.62 note].
  - The rotary speed can go on a pedal or button, with Toggle/Hold behaviour [RM p.141].
  - *manual-confirmed*; V3 audio only.
- **Ensemble Voices.** Up to four instruments within one voice, with the chord you play spread across them. The note-assignment rules are Unison, 2/3/4-part Divide and Incremental. A humanise setting varies pitch and timing between the players [OM p.54; RM p.58–63].
  - The Left part is unavailable, but the Style still follows the left hand [OM p.54].
  - *manual-confirmed*. This is **MIDI-level logic, not sample content**: the rules could run on any four sounds.

## Manual check

- V1's S.Art2 description agrees with OM p.71–73.
- S.Art+ is SX-only, and absent from the Genos2 manuals. That is consistent: the video announces it for PSR-SX.
- V2's layering agrees with OM p.48–49.

## yahaha today

- **Layering:** four parts (Right 1–3, Left) with voice, volume, octave and on/off, as on the Genos (README, Keyboard parts). *Matches.*
- **Sounds:** SoundFont presets and AU plugins per keyboard part (docs/plugin-hosting.md Phase 2), plus the ~20-patch sound library (docs/sound-library.md).
  - A plugin gets velocity, modulation, pitch bend, sustain and the other controllers. Its CC7/CC11 are applied by the rack as gain.
  - A Kontakt or other sample-library plugin with keyswitches or CC-switched articulations therefore *plays*. But yahaha has no control that sends a keyswitch or a CC.
- **ART buttons:** none. docs/controllers.md chose GM pedal defaults (Sustain/Sostenuto/Soft) instead of ART.1/Volume, "because yahaha has no Super Articulation".
- **Organ:** no drawbar or rotary control. "Organ Rotary" is listed as not done (docs/controllers.md).
- **Ensemble Voices:** none. genos-features §5 marks them optional.

## Gaps

| Gap | Priority | Suggested next step |
|---|---|---|
| No ART 1–3. Sample-library plugins with articulations can't be switched from the keyboard | P2 | Add assignable functions **Articulation 1–3** (pedal and Launchkey button), whose action a patch defines: a keyswitch note (momentary or latched), a CC value, or a program change on that part's channel. The mappings are stored per sound-library patch. This covers ART types 1–2 and S.Art+-style switching. The armed type 3 needs the plugin's own logic. |
| No rotary slow/fast or drawbars for organ plugins | P2 | "Organ Rotary Slow/Fast" as an assignable switch (RM p.141) that sends a per-patch CC (many organ plugins use CC1 or a dedicated CC). Drawbars: an optional slider page sending per-patch CCs. |
| Ensemble Voices (rule-based four-part note assignment) | P2 | It needs no samples. A later feature could reuse the harmony engine's part assignment (docs/harmony.md, Assign = Multi) with the Unison/Divide rules from RM p.58–63. Low priority; ask the owner. |
| S.Art/S.Art2 automatic articulation, AEM, Organ Flutes modelling | – | Out of scope (sample content, genos-features §F). Covered by choosing good plugins. |
