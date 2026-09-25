# What makes Genos styles sound alive (power-user tips, first impressions, SX vs Genos)

Topic id: `style-feel` · Pass: 2026-09-25 · Manual: OM p.11, p.52 (Revo/Ambient Drums), RM p.142/147 · yahaha: `README.md`, issue #127, issue #180, `docs/plugin-hosting.md`, `docs/sound-library.md`

Paraphrased notes only. No transcript text, manual text or frames are committed.

## Sources

| # | Video (channel) | URL | Timestamps used |
|---|---|---|---|
| V1 | Yamaha Genos 2 – Demo & Review (Woody Piano Shack) | https://www.youtube.com/watch?v=uAUPavLttMc | 02:11–05:59, 06:12–08:13, 11:21–12:00, 14:22–16:53, 17:16–18:20, 23:37–24:26, 25:58–27:29 |
| V2 | Genos2 Demo (Sweetwater Soundcheck) | https://www.youtube.com/watch?v=Q8BLjKaFGy4 | 01:29–02:00, 04:48–05:24, 05:54–06:45 |
| V3 | Amazing Yamaha SX920 feature Genos 2 doesn't have (Scan Keyboards) | https://www.youtube.com/watch?v=BvLF5aBXxRk | 00:42–01:56, 03:40–04:34, 05:13–06:06 |

Video coverage: all three transcripts read in full. No stills: the relevant points are audible or spoken, not on screen.

## Genos behaviour (subtleties a player notices)

**What owners and reviewers praise**
- **The acoustic sounds inside the Styles.** Guitars, saxes, strings, woodwinds and brass are called the best the reviewer has heard in any keyboard. The Style and OTS voice choices are what he actually plays, not the menus [V1 02:11–02:39]. The weak spots he names are Rhodes, Wurlitzer and tonewheel organ solo sounds; in the band mix they are fine [V1 04:37–05:44].
- **OTS as the "instant band".** The presenter picks a Style, moves through A–D, Intros and Endings, and lets the four One Touch Settings supply the right-hand sounds. There is no programming [V2 01:29–02:00].
- **Generic Styles over song-specific ones.** Of 1,200 Styles (800 built in plus free packs) he expects to use a couple of hundred. He prefers generic grooves to Styles copied from one hit song [V1 06:12–08:13].
- **Registration + Registration Sequence** is the praised live workflow: one memory per song part (intro, verse, chorus), stepped with a foot pedal [V1 14:22–16:53].
- **Chord recognition breadth.** Extended chords and slash bass played with two fingers in AI Fingered; Chord Looper to free both hands [V2 04:48–05:24].
- **The Style plays out as MIDI.** Every accompaniment part is sent over USB MIDI, so it can be recorded and edited in a DAW [V1 11:21–12:00].

**Dynamics and "alive"**
- **Style Dynamics Control**, as V2 presents it: each variation's energy used to be fixed by the programmer, and now a simple control gives "a huge range" inside one Main. Adding Ambient Drums on top is what makes it "rock" [V2 05:54–06:45]. *manual-confirmed* in purpose (OM p.11; RM p.142, p.147).
- **Counterpoint, V1.** The reviewer rarely touches Dynamics or Ambient Drums while playing; the Style as programmed is fine. He finds Dynamics subtle, as if it only spans about mf to f, and wishes it went from whisper-quiet to very loud [V1 17:16–18:20]. *video-only*. The two presenters disagree on range, so this goes to the owner.
- **Drum realism** comes from the samples, not only the timing: the Revo Drums vary the sound on repeated hits of one key, and the Ambient Drums add a sampled room [OM p.52]. *manual-confirmed*.

**Things reviewers dislike (where yahaha can beat the Genos)**
- **No seamless sound transitions.** Change variation with OTS Link, or change registration, while holding a note, and the held sound either cuts off or switches abruptly. Players learn to lift their hands first [V1 25:58–27:29]. *video-only*: the manuals don't mention held notes across voice changes.
- **Slider "catch" mode feels awkward**: you have to sweep the fader to pick up the value. He would like a jump option [V1 23:37–24:26]. *manual-confirmed* that catch exists (genos-features §10, LIVE CONTROL).

**SX vs Genos**
- **Smart Chord (PSR-SX920 only).** You set the key, major/minor and a genre (standard, pop, jazz, dance, simple). A single finger then plays the diatonic chord for that degree, with genre-appropriate extensions: a 9th on the minor, a maj7(9) on the major, a m11 in a jazz ballad [V3 00:42–06:06]. *Not in the Genos2 manuals* (checked: no "Smart Chord"). It is a competitive feature of the cheaper model.
- **Unison & Accent** is SX-only too; see `unison-accent.md`.

## Manual check

- Dynamics' purpose and Ambient/Revo Drums: confirmed (OM p.11, p.52; RM p.142, p.147).
- Its range: the manuals are silent. V1 and V2 disagree.
- Seamless sound transitions: the manuals are silent. V1 is the evidence.
- Smart Chord: absent from the Genos2 manuals, which is consistent with V3's "Genos 2 doesn't have it".

## yahaha today

- **Timing and velocity are exact.** Every Style note-on lands within a microsecond of its tick, and velocities pass through unchanged (#127 audit).
  - **Timing:** the one timing blur is buffer-start scheduling, 0–1.33 ms at 64 frames.
  - **Loudness:** the quiet parts came from a stale CC11 after a style change, since fixed (#122/#131/#165), and from GM-program fallbacks for Genos-bank voices (#103).
- **So "stiff" (#127) is most likely timbral, not rhythmic.** This pass's evidence points at what the Genos adds on top of the notes:
  - drum samples that vary per hit, plus a room sound (Revo/Ambient Drums; SF2 kits are one sample per velocity zone);
  - velocity→filter response, which rustysynth ignores (SF2 modulators, #127);
  - a live Dynamics control. The engine has one on `develop` (#180), but it isn't wired to a control yet.
- **OTS, Registration, Registration Sequence, Chord Looper and AI-style fingerings** exist (README, docs/registration.md, docs/chord-looper.md). **The Style out as MIDI**: the `yahaha` port mirrors the Style (docs/sound-library.md).
- **Seamless sound transitions:** not specified in yahaha docs.
  - **SoundFont parts:** a program change probably lets held notes ring on the old preset (rustysynth applies the program to new notes). Unverified.
  - **Plugin swaps:** the old instance gets Sustain off and All Notes Off and fades out over 5 ms (docs/plugin-hosting.md, Swaps), so a held note is cut, as on the Genos.
- **Soft takeover:** the Launchkey faders use catch within 2 (README Mixer). That is the same design V1 finds awkward.
- **Smart Chord:** none (`src/fingering.rs` has no such mode).

## Gaps

| Gap | Priority | Suggested next step |
|---|---|---|
| "Grooves feel stiff" (#127): timing is exact, so the likely cause is flat drum timbre (no per-hit variation, no velocity→filter) and no hands-on dynamics | P1 | (1) Give rustysynth the SF2 default velocity→filter modulator. (2) Wire Dynamics to a knob (#180 parts 2–3). (3) Recommend a plugin drum kit with round-robin and room mics through the drum rule (docs/sound-library.md). Then replay the #127 playtest. |
| Dynamics range: V2 hears a wide range, V1 only about mf→f | P2 | Owner A/B of yahaha's ×0.35…×1.6 against a Genos2 recording. Keep it wide: V1's complaint is that the Genos range is too narrow. |
| Seamless sound transitions: held notes cut on voice changes (a Genos weakness) | P2 | Test what a held Right 1 note does across an OTS Link change on SoundFont and on a plugin part. For plugins, keep the outgoing instance sounding until its held keys are released, instead of the 5 ms fade. A chance to beat the Genos. |
| Soft-takeover catch feels awkward to some players | P2 | Optional "jump" mode for the Launchkey faders (a setting). |
| Smart Chord (SX920): one-finger diatonic chords with genre extensions | P2 | A new fingering type: key + mode + flavour table mapping a scale degree to a chord (our own tables). Owner question: is it wanted? |
