# Synchro Start / Synchro Stop / Stop ACMP / ACMP on-off

Topic id: `sync-start-stop` · Pass: 2026-09-25 · Manual: OM p.44, OM p.47, OM p.66, OM p.130-131, RM p.11-12, RM p.65, RM p.142, DL p.82, DL p.91 · yahaha: `docs/section-timing.md#synchro-stop-window-23`, `docs/fills-and-rules.md#stop-accompaniment-27`, `src/engine/sync_stop.rs`, `src/engine/transport.rs`, `src/engine/chords.rs`, `src/engine/multipad.rs`

Paraphrased notes only. No transcript text, manual text or frames are committed.

**Video coverage this pass: none.** The four chosen videos were rate-limited (HTTP 429) on the
caption endpoint and had not arrived when this note was written. The note rests on the manuals and
yahaha's code; video-dependent points are marked "pending video".

## Sources

| # | Video (channel) | URL | Timestamps used |
|---|---|---|---|
| V1 | What is Stop ACMP (Keyboardseminare) | https://www.youtube.com/watch?v=8te-bzxM5eQ | transcript pending |
| V2 | Outsmart Sync Stop (Alois Müller) | https://www.youtube.com/watch?v=qFdDcr6Tyy0 | transcript pending |
| V3 | Playing arrangers without an active style (Casper tutorSynth) | https://www.youtube.com/watch?v=WUV5RlGRDdk | transcript pending |
| V4 | Genos 2 questions you may be embarrassed to ask (Scan) | https://www.youtube.com/watch?v=nrS_ZQObREk | transcript pending |

No stills taken for this topic (wanted, still pending: the Style Setting page with Stop ACMP and
Synchro Stop Window in V1, and the ACMP/SYNC lamps in V2).

## Genos behaviour (subtleties a player notices)

- **ACMP off**: START/STOP plays the rhythm channels only; the whole keyboard plays the Right voices
  (or Left + Right with LEFT on). With ACMP on, the chord section drives the accompaniment.
  OM p.44, OM p.66, OM p.48. A style with an empty rhythm part makes no sound at START with ACMP off
  (troubleshooting, OM p.130).
- ACMP is a panel button, stored in Registration (Freeze Style) and forced on by OTS recall and by
  Chord Looper REC. OM p.47, OM p.68, DL p.82. It can be turned off and on while the style plays
  (a common "drop to drums" move; the manual does not describe mid-song toggling: pending video).
- **SYNC START**: standby. ACMP off: any key anywhere starts the style. ACMP on: the first chord in
  the chord section starts it. Pressing SYNC START during playback stops the style and re-arms.
  OM p.66. OTS turns ACMP and SYNC START on. OM p.47, DL p.82.
- **SYNC STOP**: needs ACMP on; releasing every chord-section key stops the style, playing again
  restarts it. Not available with Full Keyboard or AI Full Keyboard. OM p.66.
- **Synchro Stop Window** (Style Setting): holding a chord longer than the window cancels Sync Stop;
  a quicker release still stops. RM p.12. Stored in Registration, Freeze group Style. DL p.91.
- **Stop ACMP** (Style Setting): with ACMP on, SYNC START off and the style stopped, chords are still
  recognised and shown; the setting picks whether they sound (Off / Style's Pad+Bass voices / Fixed
  voices). MegaVoice warning for Style. Song recording keeps the chord either way. RM p.11. The
  Vocal Harmony troubleshooting table hints that chord detection for consumers depends on it
  (OM p.131). Stored in Registration, Freeze Style. DL p.91.
- With ACMP off and LEFT on, the Left section still gives a chord to Keyboard Harmony and to Multi
  Pad Chord Match. OM p.57, RM p.65.
- Multi Pad Synchro Stop (Style Stop / Style Ending) are separate Style Setting switches. RM p.12.
- Whether turning SYNC STOP on also arms SYNC START (as on several PSR models) is not in the Genos2
  manuals: *pending video* (V2).

## Manual check

- All bullets are *manual-confirmed* except mid-song ACMP toggling and SYNC STOP arming SYNC START
  (*pending video*).
- The Stop ACMP factory default is not printed anywhere (RM p.11, DL p.91 give none).

## yahaha today

- **No ACMP switch.** The chord section is always read and the accompaniment always follows it
  (`src/engine/multipad.rs` module doc and line 295, `docs/chord-looper.md`, controllers.rs note:
  the Genos "Acmp On/Off" assignable row is replaced by yahaha's "Stop Acmp On/Off").
  What that means in practice:
  - Rhythm only: START before any chord plays only rhythm channels and CASM-autostart channels
    (genos-features §C.3 "Our rule (#4)"), and Chord Cancel returns to that state in Fingered, On
    Bass and AI Fingered. So a drums-only groove is reachable, but only until the first chord-section
    chord.
  - The left of the split never plays the Right voices in Lower detection with the non-Full types:
    a two-handed piano part over drums needs Full Keyboard / AI Full Keyboard fingering (which then
    reads chords from the whole keyboard and brings the band in) or Upper detection.
  - Sync Start fires only on the first recognised chord (`Engine::apply_chord_from`,
    `starts_on_chord`); a single key or a right-hand note never starts the style.
  - Keyboard Harmony and Chord Match always use the chord section's chord, which is the Genos
    "ACMP off, LEFT on" case too (multipad.rs doc), so that part matches.
- SYNC START: toggles while stopped; pressed while playing it stops and re-arms (`transport.rs`
  `Button::SyncStart`). OTS recall turns it on (`src/session/ots.rs`). Matches.
- SYNC STOP: toggle refused in Full/AI Full (`allow_sync_stop`); releasing every chord-section key
  stops the band and re-arms Sync Start; ignored while the Chord Looper plays (`chord_released`).
  Turning Sync Stop on does not itself arm Sync Start.
- Synchro Stop Window: Off or 0.1–5.0 s, default Off, timed from the first chord after the last
  release, engine wakes on the deadline (`sync_stop.rs`). Matches the manual.
- Stop ACMP: Off / Style / Fixed (Fixed = GM Finger Bass + Warm Pad on the style's Bass and Pad
  channels), default Off, only when Sync Start is off and the band is stopped
  (`src/engine/chords.rs` `apply_chord_from`, `change_rules.rs`). Matches.

## Gaps

| Gap | Priority | Suggested next step |
|---|---|---|
| No ACMP on/off. A Genos player loses: (a) drums-only playing with the whole keyboard on the Right voices (practice, ballad intros, piano over a groove), (b) the mid-song "band out, drums on" drop by pressing ACMP, (c) Sync Start on any key. Workarounds exist (Chord Cancel, Full Keyboard) but are not the same gesture. | P1 | PR: `Button::Acmp` + `transport.acmp` (default on; Registration Freeze Style; OTS and Looper REC force on). Off: accompaniment channels released as under Cancel, chord detection only feeds Harmony/Chord Match from the Left section when LEFT is on, key routing treats the left of the split as Right (or Left if on), Sync Start fires on any key, Sync Stop and Stop ACMP inactive. Launchkey pad + `Acmp On/Off` assignable row |
| Sync Start "any key" start (only meaningful with ACMP off). | P2 | Part of the ACMP PR |
| Sync Stop does not arm Sync Start when turned on while stopped (Genos behaviour unconfirmed). | P2 | Confirm in V2; if the Genos does, one-line change in `Button::SyncStop` |
| Stop ACMP default Off vs the Genos factory default (unknown). | – | Owner question only if a video shows a different default |
