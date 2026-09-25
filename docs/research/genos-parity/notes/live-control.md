# Live Control knobs/sliders, Assignable buttons, joystick, pedals

Topic id: `live-control` · Pass: 2026-09-25 · Manual: OM p.62-65, OM p.69-70, RM p.138-148, DL p.88-89 · yahaha: `docs/controllers.md`, `src/controllers.rs`, `src/launchkey.rs`, `README.md#controls`, `docs/solo-metronome.md`, `docs/app-api.md`

Paraphrased notes only. No transcript text, manual text or frames are committed.

**Video coverage this pass: 4 of 4 chosen, plus V5 from the mixer topic.** All read in full.

## Sources

| # | Video (channel) | URL | Timestamps used |
|---|---|---|---|
| V1 | Genos 2: how to use Live Controls & Assignable buttons (MUSIC 2000) | https://www.youtube.com/watch?v=QeVghgD5jN8 | 00:28–01:44 (still 01:02), 02:21–02:46 (still 02:30), 02:59–04:14, 04:27–05:26 |
| V2 | Live Control sliders, explanation and examples (Casper tutorSynth) | https://www.youtube.com/watch?v=5RM9Oo5yiw0 | 00:24–00:51, 01:53, 02:49–03:02, 04:40–05:26, 05:43–05:56 |
| V3 | Assignable button tips for Genos & SX (ePianos.co.uk) | https://www.youtube.com/watch?v=sexv71Miva0 | 00:15–01:45, 02:10–02:35, 04:03–05:19, 05:45–06:24 |
| V4 | How to keep your pedals working correctly (ePianos.co.uk) | https://www.youtube.com/watch?v=m2WFeE7Eeyk | 00:14–01:47, 01:59–03:41 |
| V5 | Genos Style Mute feature (A&C Hamilton) | https://www.youtube.com/watch?v=Gvf57O5E7ig | 00:07–00:57 |

Stills (private folder): `QeVghgD5jN8_0102_0/1`, `QeVghgD5jN8_0230_0/1` (+ zooms). 01:02: the
sub-display above the controls shows the slider Balance page (Style, M.Pad, Left, Right1–3, Song
A/B, Mic with values) and the slider LED meters lit to match. 02:30: Knob Assign Type 2 is showing;
the six labels are too small to read, but knob LED rings show different values (knob 1 at zero).

## Genos behaviour (subtleties a player notices)

- Six knobs with three Knob Assign Types; nine sliders with the fixed Balance page and Slider Assign
  Types 1–2, stepped with KNOB ASSIGN / SLIDER ASSIGN; with an Organ Flutes voice on a part, extra
  slider pages for that part's footages appear. The sub-display names each control and its value.
  [V1 00:28–01:44; V2 00:24–00:51], OM p.62. *manual-confirmed*
- Knobs move the value from where it is (LED rings show it); sliders "catch": nothing happens until
  the slider meets the current value, and a value set elsewhere outside the slider's range can't be
  reached. The organ footage pages don't catch. OM p.63. *manual-confirmed*
- Defaults players use: Balance sliders (Style, Multi Pad, keyboard parts, Song); Slider Type 1 = the
  8 Style channel volumes; Knob Type 2 knob 1 = Ambience Depth and knob 3 = Style Dynamics; Knob Type
  3 knobs 4–5 = Style Track Mute A/B. [V1 00:54–02:46; V2 02:49–04:40; V5 00:07–00:57], OM p.69,
  RM p.148. Type 2 knobs 1/3 *manual-confirmed*; the others *video-only*.
- Any knob, slider or joystick axis can get another function (Mixer: volume/ratio/balance, pan,
  reverb, chorus, insertion depth, EQ, cutoff/resonance/filter; Voice: envelope, modulation, tuning,
  octave, bend range, portamento; Harmony/Arp volume and arpeggio velocity/gate/speed; Style:
  Ambience, Dynamics, Retrigger rate/on-off, Track Mute A/B; Tempo), a name (up to 9 characters) and
  the parts it affects. DIRECT ACCESS then touching a control opens its edit page. [V1 02:59–04:14],
  RM p.145–148. *manual-confirmed*
- The chosen assign types reset at power-off, but the whole Live Control setup is a Registration item
  (group Live Control), so players set it per song. [V2 05:43; V1 01:31], OM p.62, DL p.88.
  *manual-confirmed*. Reset Value returns all Live Control values to default (RM p.145); V2 says the
  factory setup itself comes back via Utility [V2 05:56].
- A player complaint: no per-pad Multi Pad volumes on the sliders (Genos 1 era). [V2 04:57–05:26]
  *video-only*
- Assignable buttons (A–F and 1–3 on the Genos 2): a function or a shortcut to a display (Style
  Information, Mixer, Utility, Tap Tempo, Metronome, Voice Guide...); an optional pop-up shows the
  function's state; the Home screen's bottom shortcuts are also assignable. [V3 00:15–05:19; V1
  04:27–05:52], RM p.138–144. *manual-confirmed*
- Pedals: three jacks, each with a function from the same list, a Control Type (Toggle/Hold A/Hold B)
  and a Range; the pedal setup is a Registration item (group Foot Pedals), and freezing that group is
  how players stop other people's registrations from rewiring their pedals. [V4 00:14–03:41], RM
  p.139, DL p.88. *manual-confirmed*
- Joystick: X bends all four parts, Y modulates Right 1–3; JOYSTICK HOLD keeps the Y value; three
  Joystick Assign Types (Assignable [1] by default). OM p.64, p.70. *manual-confirmed*

## Manual check

- Everything the videos show is in OM p.62–70 and RM p.138–148. The default contents of Balance
  (order), Slider Type 1 and Knob Type 3 are video-only; the RM lists functions, not the factory
  pages.

## yahaha today

- Faders: the Launchkey's 8 faders + master with two pages, Panel (Right 1–3, Left volumes; faders
  5–8 unused) and Style (the 8 Style parts). Soft takeover = the Genos's slider catch (pick-up within
  2 or on crossing). README "Mixer", `src/engine/mixer.rs` `Takeover`. The Style page matches Slider
  Type 1; the Panel page is a partial Balance page (no Style, Multi Pad or Song level; different
  order).
- Knobs: **unmapped**. No Knob Assign pages at all (docs/solo-metronome.md: "the knobs are unmapped in
  yahaha so far"; `src/launchkey.rs` has no encoder handling). Style Track Mute A/B exists only as the
  `styleTrackMute` command; Retrigger length is Shift + > / Function; there is no Ambience Depth or
  Style Dynamics (#180), no knob tempo, no knob pan/reverb/cutoff.
- Assignable buttons: none (docs/controllers.md "Not done": the Launchkey has none free); the pad pages
  act as fixed assignments.
- Pedals: three pedals, CC learn, Toggle/Hold A/Hold B, Range, per-part reach, and the functions in
  `controllers::FUNCTIONS` (Voice, Style, OTS, Registration Bank ±, Overall). Defaults are Sustain,
  Sostenuto, Soft (a documented decision; the Genos defaults are Sustain, ART.1, Volume). Pedal setup
  is a global setting, not a Registration item.
- Missing assignable functions for features yahaha has: Registration Memory, Regist 1–10, Regist
  Sequence ±, Freeze On/Off, Sequence On/Off; Multi Pad 1–4 / Select / Stop; Chord Looper On/Off and
  Rec/Stop; Metronome On/Off; Half Bar Fill In; Style Tempo Lock/Reset and Hold/Reset (these exist as
  commands, `toggleStyleTempoLock`/`Hold`, but not as pedal rows). RM p.141–144.
- Wheels: pitch bend on all four parts, modulation on Right 1–3, per-part bend range. Matches the
  joystick defaults; no Joystick Hold (the Launchkey's mod wheel doesn't spring back, so little is
  lost).

## Gaps

| Gap | Priority | Suggested next step |
|---|---|---|
| Launchkey knobs do nothing; no Knob Assign pages | P1 | Map the 8 encoders (relative, like Genos knobs) to Knob Assign pages with Genos-like defaults: Style Track Mute A/B, Retrigger rate and on/off, Master Tempo, and Dynamics / part Reverb / Cutoff as those land; a KNOB ASSIGN button (a Shift combo); the page and values in state for the app's sub-display |
| Panel fader page lacks the Balance items (Style, Multi Pad) | P1 | See mixer.md (Style offset on faders 5–8) |
| No pedal path for Registration (Sequence ±, Regist 1–10, Memory, Freeze, Sequence On/Off) | P1 | See registration.md |
| Missing assignable rows for Multi Pad 1–4/Select/Stop, Chord Looper On/Off and Rec/Stop, Metronome, Half Bar Fill In, Style Tempo Lock/Hold | P2 | Add rows to `controllers::FUNCTIONS` (+ app JSON fixture); they map onto existing commands |
| Live Control setup and Foot Pedals setup aren't Registration items | P2 | After the knob pages exist: `liveControl` and `footPedals` sections (groups already reserved / to add) |
| No user-assignable functions for knobs, rename, parts per function | P2 | After the default pages: an assign editor in the app (Settings), mirroring RM p.145 |
| No Style Dynamics or Ambience Depth to put on a knob | – | Tracked in #180 (dynamics topic) |
