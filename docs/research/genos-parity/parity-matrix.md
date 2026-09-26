# Genos parity matrix

The living comparison between the Genos (our reference) and yahaha. One row per
concept a player notices. Updated by each research pass ([README.md](README.md));
the notes behind each row are in [notes/](notes/).

- **Genos behaviour** is paraphrased. Refs: `OM`/`RM`/`DL` + printed page (Genos2
  Owner's Manual, Reference Manual, Genos Data List), `[<note> Vn MM:SS]` for video
  Vn in that note's Sources table (short names: looper = chord-looper, pads =
  multipads, tempo = tempo-transpose, songs = song-score, harmony = harmony-arp,
  voices = voices-sart, assembly = style-assembly, unison = unison-accent). *video-only* = not found in the
  manuals; *conflict* = video and manual disagree (see the note).
- **Priority**: P0 playability-breaking · P1 noticeable within a song or two · P2
  nice to have · – no gap.
- **Next step**: a proposed small PR, or an issue number once filed.

## Top gaps (pass 2026-09-25)

Ranked across topics; P1 first. Each is sized for one small PR (see the row).

| # | Row | Why a Genos player notices | Pri |
|---|---|---|---|
| 1 | [chord/ai-fingered-bass](#chord-following) | AI Fingered (the type presenters recommend) loses slash and walking-bass chords: C then B+C stays C, not C/B | P1 |
| 2 | [tempo/tap-start](#transport-and-tempo) | Counting the band in with TAP TEMPO does nothing: the style never starts | P1 |
| 3 | [pads/launchkey + pads/level](#multi-pads-harmony-songs) | Multi Pads can't be played from the Launchkey, and there is no one level to sit them against the style | P1 |
| 4 | [control/knob-pages](#live-control-mixer-and-balance) | The Launchkey's 8 encoders do nothing; on the Genos the knobs carry Dynamics Control (now on develop, but only in Settings and on a pedal), Track Mute, Retrigger, filters | P1 |
| 5 | [mixer/pan-sends + ots/recall-contents](#live-control-mixer-and-balance) | OTS and Registration drop each part's pan, reverb and chorus; no per-part pan/send control | P1 |
| 6 | [mixer/style-balance](#live-control-mixer-and-balance) | No single fader for band-vs-hands balance (Genos Balance page: Style, M.Pad, parts) | P1 |
| 7 | [registration/pedal-sequence](#ots-and-registration) | Foot-switch registration changes (Regist ±) are how players move through a song hands-free | P1 |
| 8 | [looper/controls-registration-files](#chord-following) | The Chord Looper can't be driven from pads/pedals, isn't in Registration, and its 8 memories vanish on quit | P1 |
| 9 | [split/left-hold](#chord-following) | Left voice can't sustain after the left hand lets go | P1 |
| 10 | [feel/stiff-grooves](#style-feel-and-dynamics) | Owner's #127 "stiff": timing is exact, so it is timbre (no velocity→filter on SoundFonts, one sample per hit) | P1 |

Close behind: tempo reset/auto-repeat (P1, tiny), Keyboard transpose with a chord held (P1 if a Genos
confirms the video), fill late-press allowance (P1 if a Genos has one), and the ACMP
on/off switch (P2 on this pass's evidence: the manuals describe it, but no video used it).

## Matrix

Rows marked *pending video* rest on the manuals and code only: YouTube rate-limited
the caption downloads during this pass (see the Run log).

### Transport and tempo

| Concept | Genos behaviour | yahaha today | Gap | Pri | Next step |
|---|---|---|---|---|---|
| tempo/tap-start | Stopped: tapping one bar's worth of beats (4 in 4/4) starts the style's rhythm at the tapped tempo. OM p.46; consistent with [auto-fill V2 12:21] | `Engine::tap` sets the tempo from the 2nd tap and never starts (`src/engine/transport.rs`) | Tap-to-start missing | P1 | When stopped, after a bar of steady taps start one beat after the last tap (rhythm only until a chord); perform test. [note](notes/retrigger-reset.md) |
| tempo/reset-repeat | TEMPO − and + together restore the style's tempo; holding a button repeats. OM p.46 | 1 BPM a press; held, repeats and speeds up; both together = `resetTempo` (#263, `src/engine/tempo_repeat.rs`) | none (step and repeat rate not in the manual: yahaha's choice) | – | – |
| tempo/tap-sound | Each tap clicks; Tap Sound and Volume settings, stored in Registration/OTS. RM p.39, DL p.88 | Silent taps | Missing | P2 | Click on the built-in synth (`src/click.rs`) + two settings |
| tempo/section-reset | TAP while playing rewinds the section to its top; a setting turns it into a tempo change. OM p.46, p.67; RM p.39; Yamaha clip stills | Default on (#128), own command and assignable (`docs/section-timing.md`) | none; `docs/app-api.md` `setSectionReset` still calls off "yahaha's default" | P2 | Fix the doc sentence |
| tempo/retrigger | Chord restarts the Main's head at rate 1–32; Mains only; three Live Control functions. RM p.147 | Matches the manual (`src/engine/retrigger.rs`); no knob functions | Knob functions (see control/knob-pages); audible behaviour *pending video* | P2 | Add RtgRate/RtgOnOff/RtgOff&Rt with the knob pages |
| tempo/change-behavior | Tempo Lock/Hold/Reset; the assignable toggles pop up the new value; a lock icon beside the tempo (always with Lock, while playing with Hold). RM p.12, p.144; [tempo V1 still 00:30; V2 01:19–03:20]. One video says the factory default is Reset (*video-only*) | Implemented (#26), default Hold; toggles are API-only; no lock indicator | Not assignable; no lock badge; default Hold vs Reset | P2 | Two `controllers::FUNCTIONS` rows + a toast + a lock badge; owner question on the default |
| tempo/transpose | Master/Keyboard ±12, both buttons reset, drums untouched. OM p.61 | Matches | none | – | – |
| tempo/held-chord-transpose | Changing Keyboard transpose while a chord is held (or an Intro plays) changes nothing until the next chord is played; used to take the next verse up a key [tempo V6 21:23–22:25, *video-only*; manual silent] | A transpose with a chord held moves the band at once (`src/sim.rs` test `keyboard_transpose_change_moves_held_chord`) | *conflict* with the video | P1 if confirmed | Owner/hardware check; if confirmed, apply a transpose to the style chord only from the next chord input and flip the test. [note](notes/tempo-transpose.md) |
| tempo/upper-octave | UPPER OCTAVE −/+ shifts Right 1–3, both = reset, in Registration. OM p.61, DL p.82 | Per-part octave only | Missing | P2 | Upper octave + assignable + registrable |
| tempo/param-lock | Groups such as Split Point and Fingering Type can be locked; a locked value can still be edited, it just isn't overwritten by Registration/OTS. DL chart, RM p.163; [tempo V4 00:21–02:28] | Split Point and Fingering Type (`src/api/param_lock.rs`) | none | – | – |

### Sections and fills

| Concept | Genos behaviour | yahaha today | Gap | Pri | Next step |
|---|---|---|---|---|---|
| sections/change-timing | To Main Immediate / Next Bar (first-beat rule), Auto Fill forces Next Bar, Intro→Intro and Ending I rules. RM p.12 | `Engine::change_point` matches exactly (`docs/section-timing.md`) | none | – | – |
| sections/fill-late-press | Fills start at the next beat and play the rest of their bar [auto-fill V1 04:27 (PSR); sections V4 00:47]; manuals silent | `next_beat` rounds up: a fill hit a few ms after a beat waits a whole beat | Possibly no late-press grace | P1 if the Genos has one | Owner question; if yes, a small grace window for `Change::Fill` |
| sections/lamps | Red = selected, flashing = next, blue = has data, off = empty; the Main flashes during its fill. OM p.68; [sections stills] | `section_leds` matches, except the Main after an Intro or Break is solid, not flashing | Minor | P2 | Flash the queued Main while an Intro/Break plays |
| sections/ending-rit | Second press of the playing Ending slows it gradually, from the press to the Ending's end (an early press slows too much). OM p.66; [sections V7 28:38–29:03] | Linear from the press to 65% at the end (`src/engine/ritardando.rs`) | none; README "Known gaps" still lists it as not done | P2 | Fix README Known gaps |
| sections/ending-i-short | Intro I is roughly a count-in and Ending I is very short on Genos 2 styles [sections V7 26:27–27:04] | Plays the style data as written | none: supports closing #129 as by design | – | Comment on #129 |
| sections/intro-midsong | An Intro pressed mid-song replays as an interlude (e.g. before a verse up a key) [sections V7 18:40–21:10, *video-only*] | Queued for the next bar line, then the selected Main | none | – | – |
| sections/main-during-ending | A Main pressed during an Ending carries on into that Main (an Ending used as a fill) [sections: Alois Müller, video-only] | Queued for the next bar | none (see #187) | – | – |
| sections/in-section-tempo | Fill/Break are one bar; power users stretch one with tempo events inside the section [sections: Casper 06:12–07:48, video-only] | `scan_meta` ignores tempo events after the first section marker (`src/sff.rs`) | Such edited styles play the fill at the wrong speed | P2 | Owner question, then honour per-section tempo events relative to the player's tempo |
| sections/hold-break | Holding BREAK keeps the break going [auto-fill V1 07:21, PSR only] | Press only | Unknown on the Genos | P2 | Owner/hardware check |
| sections/half-bar | Half Bar Fill In, assignable with a control type. RM p.142 | `Change::HalfBar`, not in the pedal function list | Not assignable; audible reading *pending video* | P2 | `Function::HalfBarFill` (Toggle/Hold) |
| sections/auto-fill | Every Main press plays a fill first; with it off, A→D waits for the bar end and jumps abruptly; System setting, not in Registration. OM p.67, RM p.12, DL p.82; [auto-fill V4 23:15–24:21] | Matches; default on (PSR default off [auto-fill V1 00:53]) | Factory default unknown | P2 | Owner question |
| sections/fill-up-down-wrap | Fill Up/Down go to the Main on the immediate right/left. RM p.142; the PSR wraps D→A [auto-fill V1 09:18] | No wrap: plays that Main's own fill at the end of the row | Possible mismatch | P2 | Hardware check; wrap is a one-line change |

### Chord following

| Concept | Genos behaviour | yahaha today | Gap | Pri | Next step |
|---|---|---|---|---|---|
| transport/acmp-switch | ACMP off: START plays rhythm only, the whole keyboard plays the Right voices, Sync Start fires on any key; OTS and Looper REC turn ACMP on. OM p.44, p.47, p.66, p.68. No video this pass uses it: the no-style workflow keeps ACMP on with Stop ACMP Off [sync V3 00:33–03:10] | No ACMP switch; rhythm-only lasts until the first chord; the no-style workflow already works | Drums-only full-keyboard play, mid-song band drop, any-key Sync Start | P2 (owner question: P1 if you use it) | `Button::Acmp` (default on; registrable; OTS/REC force on), key routing, any-key Sync Start, pad + assignable. [note](notes/sync-start-stop.md) |
| chord/fingering-types | Seven types plus Fingered\* in Upper; presenters say the Genos ships on Multi Finger. RM p.9, OM p.46; [fingering V8 12:55; V2 01:31] | All built (`src/fingering.rs`); default Fingered On Bass; registrable + Parameter Lock | Default differs from the factory setting | P2 | Owner decision |
| chord/ai-fingered-bass | AI Fingered keeps a slash bass: after a chord, a held chord note plus a lower note gives chord/bass (C then B+C = C/B; Am then G+A = Am/G; a two-note E7/G#), and the display shows the slash chord. RM p.9 says root bass only for plain Fingered. [fingering V8 09:23–12:05, stills 09:45, 11:51] | Root bass forced (`src/fingering.rs` `ai(...).map(root_bass)`); B+C after C stays C | Slash and descending-bass lines lost in the type presenters recommend | P1 | AI Fingered uses the lowest note as bass; 1–2 notes whose upper notes fit the previous chord keep it over the lower note; tests from the three examples. [note](notes/fingering.md) |
| chord/ai-dyads | The 1–2 note rule is undocumented. RM p.9 | Our own interval table (`infer_dyad`) | Unverified | P2 | Compare with the videos; owner playtest |
| chord/multi-finger-dyads | Single Finger shapes use the nearest keys to the left; a root + minor-third dyad in Multi Finger changes nothing [fingering V8 03:12–03:51; V1 01:51–02:43, *video-only*] | Two-key Multi Finger reads any key below the root (C+Eb = Eb7) | Accidental chord changes for Multi Finger players | P2 | Limit 2-key Single/Multi shapes to adjacent keys |
| chord/cancel-1+5-1+8 | Cancel in Fingered, On Bass, AI Fingered only; none in Fingered\*. OM p.46, RM p.9, DL p.45 | Matches | none | – | – |
| chord/bass-hold | Bass Hold assignable (not in AI Full Keyboard). RM p.142, p.157 | Missing (the On Bass toggle exists) | Missing | P2 | `Function::BassHold` + engine bass freeze |
| chord/settle | No latency spec | 10 ms settle window (`src/engine/settle.rs`) | none known | – | Playtest only |
| split/three-points | Style, Left and Right 3 split points; a Right 3 split (solo voice on top, R1/R2 below) is a common trick; set by holding a label and pressing a key; in Registration. OM p.49–50, DL p.88; [split V3; V4 01:25, 03:04; V1 05:25, 09:13] | One split point; no Right 3 split; no key capture | Missing | P2 | `split_r3` + routing, then separate Style/Left, app "set from key" |
| split/left-hold | LEFT HOLD sustains the Left voice after release; cleared by stopping the style; in Registration. OM p.49, RM p.141, DL p.82. In use: organ/pad Left held across chord changes, per-registration on/off, a held Left as an intro with the style stopped [split V2 01:40, 10:35; V1 02:00–02:48; V5 01:55] | Not built (#31 remainder) | Missing | P1 | Left Hold flag + assignable + registrable + pad. [note](notes/split-detection.md) |
| split/upper-manual-bass | Upper forces Fingered\*; Manual Bass moves the Style bass voice to Left. OM p.51 | Built | none | – | – |
| sync/sync-start | Standby; pressed while playing it stops and re-arms (players use it as a stop-time break: next chord restarts on the beat); OTS turns it on. OM p.66, p.47; [sync V4 29:57–32:54] | Matches | none | – | – |
| sync/sync-stop | Needs ACMP; not in Full / AI Full Keyboard. OM p.66 | Matches; whether turning it on also arms Sync Start is *pending video* | Possible arming difference | P2 | Confirm with qFdDcr6Tyy0 |
| sync/sync-stop-window | Holding past the window cancels Sync Stop. RM p.12, DL p.91 | Off or 0.1–5 s | none | – | – |
| sync/stop-acmp | Off / Style (Pad+Bass) / Fixed. RM p.11, DL p.91 | Matches; default Off | Factory default unknown | – | – |
| looper/record-loop-timing | Record and loop start at the next bar; REC while stopped arms Sync Start; whole keyboard is free while looping; memory switch at the next bar. OM p.68–69, RM p.14–19, [looper V1 02:14, 08:34] | Built and tested (`src/engine/looper.rs`) | none | – | – |
| looper/transpose-style-change | Transpose moves a running loop; a style change keeps it; fills still work [looper V1 12:01–14:29, video-only] | Should match; untested | Tests | P2 | Two engine tests |
| looper/controls-registration-files | Panel REC/STOP and ON/OFF (also assignable); Registration stores bank, memory and ON/OFF; 8-slot banks save as files. RM p.14–19, p.141; DL p.82; [looper V1 10:28, V3] | Terminal keys and app drawer only; `Group::ChordLooper` exists but is not wired; memories last the session | Can't drive it hands-on, not registered, not saved | P1 | Three small PRs: assignables + pads; `chordLooper` registrable; JSON bank save/load. [note](notes/chord-looper.md) |

### OTS and Registration

| Concept | Genos behaviour | yahaha today | Gap | Pri | Next step |
|---|---|---|---|---|---|
| ots/recall-contents | An OTS recalls voice and on/off plus each part's mixer values (pan, reverb/chorus, filter, EQ), Voice Edit, and turns ACMP and Sync Start on. OM p.47, DL p.82–92; [ots V2 01:05] | Voice (via the program map), on/off, volume, octave, pan and sends (#198); filter, EG, vibrato, portamento, bend range and the XG part SysEx (#238: `OtsPart::tone/bend_range/xg`, `Parts::send_tone`; a voice change puts them back to neutral); arms Sync Start | The built-in synth sounds only the bend range of these (#246); plugins get the CCs but never the XG SysEx (#247); the Genos-own part SysEx (`43 73 01 5x`) beyond on/off and octave is not decoded | P2 | #246, #247. [note](notes/ots.md) |
| ots/harmony-pads | OTS switches Keyboard Harmony/Arp and the Multi Pad bank. DL p.82, p.88; [ots V2 01:05–02:54] | Not recalled | Missing | P2 | Decode the OTS Harmony SysEx (needs a Genos-saved style) |
| ots/link-timing | Main A–D → OTS 1–4, Immediate or At Main Section Change. OM p.67, RM p.11; [ots V1] | Matches; default At Main Section Change (owner choice) | none | – | #111 |
| ots/user-memorize | MEMORY + OTS stores your own OTS into a User style. OM p.60; [ots V1–V3] | Recall only | No user OTS | P2 | Owner decides storage (sidecar vs user `.sty` copy) |
| ots/readme | – | README "Known gaps" lists "OTS voice changes", written before OTS existed | Stale doc | P2 | Fix README Known gaps (with sections/ending-rit) |
| registration/core | Memorize groups, lamps, banks, Freeze, Bank Info, Playlist. OM p.96–103, RM p.113–119; [registration V1–V3] | Matches (`docs/registration.md`) | none | – | – |
| registration/pedal-sequence | Registration Sequence stepped by pedal (Regist ±) or Assignable button; Regist 1–10, Memory, Freeze, Sequence On/Off assignable. RM p.114, p.141; [registration V4 00:40–03:53] | Pads, keys and app only; the pedal list has Bank ± only (`src/controllers.rs`) | No foot-switch registration changes | P1 | Pedal Control Regist ± + the Registration assignable rows. [note](notes/registration.md) |
| registration/default-sequence | A new sequence defaults to 1–10 (firmware 1.40) [registration V4 01:06, video-only] | An empty sequence does nothing | Possible mismatch | P2 | Hardware check, then treat empty as 1–10 |
| registration/part-effects | Each part's pan, reverb, chorus, filter, EQ are stored. DL p.82–83; [registration V2 03:54] | `parts` stores voice, volume, octave | Missing | P1 | Follows mixer/pan-sends |
| registration/ending | A button memorized with an Ending lit ends the song [registration V3 03:59, video-only] | `styleControl` stores Main and Intro | Possibly missing | P2 | Hardware check |
| registration/foot-pedals-group | Foot Pedals and Assignable Buttons are Registration groups. DL p.88 | Pedals are global | Missing | P2 | `footPedals` section + group |
| registration/startup-bank | The bank survives power-off. OM p.97 | Starts empty | Missing | P2 | Remember the last bank in `setup.json` |
| registration/arp-items | Arpeggio Quantize/Hold are not Registration items. DL p.88 | Stored in `harmonyArp` | Small deviation | P2 | Record a decision; fix the doc |

### Live Control, mixer and balance

| Concept | Genos behaviour | yahaha today | Gap | Pri | Next step |
|---|---|---|---|---|---|
| control/knob-pages | 6 knobs × 3 Knob Assign types (Type 2: Ambience knob 1, Dynamics knob 3; Type 3: Style Track Mute A/B), value LEDs, stored in Registration. OM p.62–63, p.69; RM p.145–148; [live-control V1 02:21] | Launchkey encoders unmapped; `styleTrackMute` is a command only | No knob control | P1 | Map the 8 encoders to Knob Assign pages (Dynamics, Track Mute A/B, Retrigger, tempo; later reverb/cutoff). [note](notes/live-control.md) |
| control/assignables | Assignable buttons/pedals cover Multi Pad, Chord Looper, Metronome, Half Bar Fill, Tempo Lock/Hold, Registration. RM p.139–144 | Missing rows though the features exist; no Assignable buttons | Missing function rows | P2 | Add rows to `controllers::FUNCTIONS` |
| control/pedals | 3 pedals, Control Type, Range, stored in Registration. RM p.139 | 3 pedals with learn, type, range; global | Registration storage only | P2 | See registration/foot-pedals-group |
| control/fader-catch | Slider catch mode; a reviewer wants a jump option [style-feel V1 23:37] | Soft takeover within 2 | Same design | P2 | Optional "jump" mode |
| mixer/style-balance | Balance page: Style, M.Pad, Left, Right 1–3, Song, Mic; a whole-Style offset in Registration. RM p.145, DL p.83; [live-control V1 01:02 still] | Panel page is Right 1–3 and Left; faders 5–8 unused | No band-vs-hands fader | P1 | Owner decision on the mixer rule (a CC7 scale like Fade), then a Style offset fader, registrable. [note](notes/mixer.md) |
| mixer/pan-sends | Every part has pan, reverb, chorus, filter, EQ, insertion depth. RM p.129–135 | Volume, mute, solo, master only | No per-part pan/sends | P1 | `SetPartPan` / `SetPartSend` (CC10/91/93) + app knobs; OTS and Registration reuse them |
| mixer/revoice | Change a Style channel's voice; kept in Registration or a User style. OM p.91–93 | Sound-library per-program override only | No per-channel revoice | P2 | Per-style part voice override in `styleMixer` |
| mixer/mute-levels | Channel on/off, solo, Part On/Off change behaviour. OM p.92, RM p.13 | Matches (#26, #30) | none | – | – |

### Style feel and dynamics

| Concept | Genos behaviour | yahaha today | Gap | Pri | Next step |
|---|---|---|---|---|---|
| feel/dynamics-control | A live control (knob 3 of Type 2, or an expression pedal; not a button) that changes how hard every Style part plays, all sections; shades energy inside a Main while A–D stay the coarse steps; a System on/off gate. OM p.11, p.69; RM p.11, p.139, p.142, p.147; DL p.91; [dynamics V1 00:47–01:17, V3] | Engine (#184: velocity ×0.35–×1.6 on all Style note-ons, CC7 untouched), API (#189), pedal function (#190), Settings › Style controls (#192) on develop; no Launchkey knob | Not on a knob, so not a hands-on live control yet | P1 | Covered by control/knob-pages (knob 3 = Dynamics). [note](notes/dynamics.md) |
| feel/dynamics-touch | OM p.69 says style volume follows playing strength; RM and all videos show a hand-set control (*conflict*, manual vs manual) | Touch mode (#180), off by default | none; document Touch as a yahaha extension | – | Note it in genos-features.md §1 |
| feel/dynamics-curve | Curve unspecified; reviewers disagree on range [style-feel V2 05:54 vs V1 17:41] | ×0.35–×1.6, the same for every part | Unverified; flat timbre on SoundFonts | P2 | Owner A/B against a Genos2 recording |
| feel/stiff-grooves | Genos realism is timbral: per-hit drum sample variation, sampled room (Ambient Drums), velocity → tone, live Dynamics. OM p.52; [style-feel V2 05:54–06:45] | Timing tick-exact, velocities unchanged (#127); SF2 kits one sample per zone; rustysynth ignores velocity→filter modulators | "Stiff" is most likely timbre | P1 | Give rustysynth the SF2 default velocity→filter modulator; Dynamics on a knob; suggest a round-robin plugin kit. [note](notes/style-feel.md) |
| feel/ambience | Ambient Drums wet/dry on Rhythm 1/2 (knob 1 of Type 2). OM p.52, p.69; RM p.147 | None | No ambient kits | P2 | Knob proxy (CC91 on Rhythm 1/2) or a plugin kit with room mics |
| feel/voice-change-cutoff | Genos2 cuts held notes on OTS/variation voice changes; players lift their hands [style-feel V1 25:58–27:29, video-only] | Plugin swap sends All Notes Off; SF2 behaviour unverified | A chance to beat the Genos | P2 | Test held notes across OTS Link; keep the outgoing voice until its keys are released |
| feel/unison | Not in the Genos2 manuals; PSR-SX: hold a pedal and Style parts play your melody [unison V2 04:33] | None | Competitive gap vs SX, not a parity gap | P2 | Owner question |
| feel/accent | PSR-SX only; needs accent data none of the 208 corpus styles has (#180) | Stand-in: a hard strike plays Fill Self (#180) | Stand-in not reachable yet | P2 | Ships with #180 |
| feel/smart-chord | SX920 only: key + mode, one finger plays the diatonic chord [style-feel V3 00:42–06:06] | None | Competitive gap, not parity | P2 | Owner question |
| feel/articulation-buttons | ART 1–3 (one-shot / hold / armed), pedal 2 defaults to ART.1. OM p.71–73; RM p.139; [voices V1 00:38–02:07] | None; nothing sends keyswitches to plugins | Can't switch plugin articulations | P2 | Assignable "Articulation 1–3" with a per-patch keyswitch/CC action. [note](notes/voices-sart.md) |
| feel/organ | Footage levers, rotary slow/fast, sliders as drawbars. OM p.53, p.62; RM p.141 | None | No rotary/drawbar control for organ plugins | P2 | Assignable "Organ Rotary" sending a per-patch CC |
| feel/ensemble | Ensemble Voices: four players, Unison/Divide rules (MIDI logic). OM p.54; RM p.58–63 | None | Missing | P2 | Later; reuse the harmony engine |
| feel/part-swap | Assembly: take a channel from another style/section; performers borrow drums. RM p.20, p.26; [assembly V1] | No Creator | No playing-side swap | P2 | Owner question: "Part Source" for Rhythm 1/2 |
| feel/groove | Swing A–E, Beat Converter, Push/Heavy re-time a section. RM p.27; [assembly V2 05:20–08:32] | Tick-exact, no groove control | No live swing amount | P2 | Per-style swing as a playback tick remap. [note](notes/style-assembly.md) |

### Multi Pads, harmony, songs

| Concept | Genos behaviour | yahaha today | Gap | Pri | Next step |
|---|---|---|---|---|---|
| pads/playback | Start at once stopped / next bar playing (a late press comes in on the next bar); press again restarts; up to 4; Repeat in step; tempo changes apply at once; stop lets tails ring. OM p.74, RM p.65; [pads V1 04:25, 12:01, 17:13, 21:10] | Matches (`docs/multipad.md`, `src/multipad/player.rs`) | none | – | – |
| pads/stop-sync | STOP all, STOP + pad one; pad Synchro Stop on Style stop/Ending. RM p.12, p.78 | Matches; defaults are our choice | Defaults unconfirmed | P2 | Confirm with UeCqDAnNFS4 |
| pads/synchro-start | SELECT + pad arms; a chord or style start fires it; stored in Registration. OM p.75, DL | Implemented; armed state not registered | Missing registrable | P2 | Store armed pads with the bank |
| pads/chord-match | Pads follow the chord, even with the style stopped; Chord Match off plays a run as written. OM p.74, RM p.65; [pads V1 08:38, 26:37] | Matches | none | – | – |
| pads/launchkey | Dedicated panel buttons 1–4, STOP, SELECT. OM p.19 | Terminal keys and app only; page 5 is a proposal | Can't play pads from the controller | P1 | Wire the proposed pad page 5. [note](notes/multipads.md) |
| pads/level | One Multi Pad level beside the Style level; players balance pads against the style constantly (and mute a style part so a pad replaces it). OM p.90; [pads V1 05:43–06:29, 11:49, 12:37, 20:06; still 06:12] | Per-pad CC7 only | No overall pad level | P1 | Pad master level (a CC7 scale on ch 5–8), registrable; a fader with mixer/style-balance. [note](notes/multipads.md) |
| pads/ots-bank | Every OTS loads a pad bank that suits the style. OM p.60, DL; [pads V1 01:07–02:23; ots V2 01:05] | Not recalled | Missing (helps only user banks; yahaha ships none) | P2 | With ots/harmony-pads |
| harmony/chord-source | ACMP chord / LEFT chord / both. OM p.56–57 | Always the Style chord | none until ACMP off exists | – | Revisit with transport/acmp-switch |
| harmony/echo-timing | Echo/Tremolo/Trill at the tempo, Speed incl. 1/32. OM p.57; [harmony V1 03:38–04:20] | Repeats counted from the key press | Unclear if the Genos locks to the beat | P2 | Owner listening test. [note](notes/harmony-arp.md) |
| harmony/storage | Type and settings in Voice Set, OTS, Registration; synth voices arrive with their own arpeggio. DL chart; [harmony V3 01:31] | Registration yes; OTS and Voice Set no | Missing | P2 | With ots/harmony-pads; per-patch default later |
| harmony/mono-left | A mono voice on LEFT sounds one chord note; with Left Hold it becomes an automatic counter melody. RM p.49; [harmony V4 01:05–03:10] | No per-part Mono/Poly, no Left Hold | Trick impossible | P2 | With split/left-hold: a per-part Mono flag |
| harmony/arp-live | ArpVel / ArpGateT / ArpUnitM as Live Control. RM p.141, p.147 | Engine has them; not in app or Launchkey | Not reachable | P2 | Expose in the Harmony panel |
| songs/song-plus-style | MIDI Song + Style: Style replaces ch 9–16, the Song sets the tempo. RM p.75, p.78; players often use a recorded pad instead [songs V2] | No Song player (out of scope) | Covered by songs/solo-backing | – | Owner question |
| songs/solo-backing | Players record a solo or riff into a Multi Pad (Chord Match off, Repeat on) and trigger it with the style. RM p.64–65; [songs V2 00:34–02:58] | Plays such a `.pad`; can't make one | No user pad | P2 | Import one MIDI-file channel as a pad into a user bank |
| songs/midi-song-to-style | Yamaha's computer app converts MIDI files to styles. [songs V1] | Loads the output like any style | none | – | – |
| songs/chord-track | Song chord events show on the Score. RM p.72, p.95–96 | iReal chart player (beyond the Genos) | No MIDI chord-track import | P2 | SMF chord-track → chart importer. [note](notes/song-score.md) |

## Run log

### 2026-09-25: first pass (breadth)

- **Scope:** all 21 topics in `topics.toml`; 91 rows.
- **Sources:** 84 videos chosen (listed in the notes' Sources tables), 53
  transcripts fetched and read, 45 stills viewed. All of these stay in the
  private folder. Every row was checked against the Genos2 OM/RM and the Genos
  Data List, and against yahaha's docs and code on `develop` (`3eab26b`, then rebased on `3321f15`).
- **Rate limit:** YouTube returned HTTP 429 on captions after about ten
  downloads, and the limit came and went for the rest of the pass. The flow now
  fetches in a background loop while the workers start on the manuals and code,
  then sends them a second round with the late transcripts (README "YouTube
  rate limits"). About 30 videos are still pending, marked "transcript pending"
  in the notes. The ones that matter most: the Keyboard-Akademie AI Fingered and
  slash-chord videos (the exact AI bass rule), all five retrigger/Section Reset
  videos, the Stop ACMP and Sync Stop videos (does Sync Stop arm Sync Start?),
  and the Multi Pad Synchro Start/Stop video (defaults).
- **Findings:**
  - Confirmed P1s: AI Fingered slash bass, TAP to start, Left Hold, Registration
    changes from a pedal, no knob pages, OTS dropping pan and effects, band
    balance and Multi Pad levels.
  - The ACMP switch went down to P2: no video used it.
  - Dynamics Control landed on develop during the pass (#184, #189, #190, #192); only a knob is missing.
  - #127's "stiff" is most likely timbre.
  - #129 (Ending I short) is supported as by design.
  - README "Known gaps" is stale: it still lists ritardando and OTS voices,
    which are done.
- **Owner questions:**
  - Held-chord transpose timing.
  - Fill late-press grace.
  - Fill Up wrap.
  - Hold BREAK.
  - Factory defaults (fingering type, Auto Fill, Tempo change behaviour, TEMPO step).
  - Whether to allow a Style/Multi Pad balance as a CC7 scale (the mixer rule).
  - User OTS storage.
  - PSR-SX extras beyond parity (Unison, Smart Chord).
  - ACMP-off usage.
  - Plugin keyswitch articulations.
  - Echo timing.
- **Next pass:** fetch the pending transcripts first and re-check the
  *pending video* rows. Then do a hardware check of the owner questions, if a
  Genos owner can help (`yahaha capture-kit`).
