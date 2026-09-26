# One Touch Setting and OTS Link (timing, what is recalled)

Topic id: `ots` · Pass: 2026-09-25 · Manual: OM p.47, OM p.60, OM p.67, RM p.11, RM p.163, DL p.82-92 · yahaha: `docs/fills-and-rules.md#ots-link-timing-and-ots--sync-start-28`, `src/session/ots.rs`, `src/parts.rs` (`apply_ots`), `src/sff.rs` (`parse_ots`), `README.md`

Paraphrased notes only. No transcript text, manual text or frames are committed.

**Video coverage this pass: 4 of 5** (V1–V4 read in full; V5 rate-limited, transcript pending). V1 is
shown on a PSR-S670, not a Genos; the OTS workflow is the same family, but treat its details as
Yamaha-arranger evidence, not Genos proof.

## Sources

| # | Video (channel) | URL | Timestamps used |
|---|---|---|---|
| V1 | What is OTS, how to set up One Touch Settings & OTS Link (BB Walker TV; PSR-S670) | https://www.youtube.com/watch?v=22Q645eVHzE | 02:24–02:51, 04:27–05:07, 10:52–11:33, 13:41–14:59 |
| V2 | Customize Style with your own OTS (Casper tutorSynth) | https://www.youtube.com/watch?v=RJtZxd9tJYs | 00:17, 00:49, 01:05–02:54, 03:12 |
| V3 | Genos & SX900: Personalising one touch settings (Leigh Wilbraham) | https://www.youtube.com/watch?v=dfG8Z3leUio | 00:43–01:51, 02:22–03:02, 03:28–03:41 |
| V4 | Yamaha Genos The First Steps (Genos Genie) | https://www.youtube.com/watch?v=rce-xYjdZlA | 01:51–02:30, 04:06–04:32 |
| V5 | OTS Genos (KEY STORE TV) | https://www.youtube.com/watch?v=9uF4gi1_yzw | transcript pending (429) |
| V6 | How to use registrations, full tutorial (Leigh Wilbraham) | https://www.youtube.com/watch?v=ocYzJhzD4Is | still 02:52 (Memory window) |

Stills (private folder): `ocYzJhzD4Is_0252_0`. It shows the Registration Memory window that MEMORY
opens: its prompt says to press either a Registration Memory button or a One Touch Setting button,
above the group checkboxes. The same MEMORY press is the entry to memorizing an OTS. Stills still
wanted: V2 around 00:31 (the four-step procedure) and V3 around 01:10 (the "save the style"
message), both pending.

## Genos behaviour (subtleties a player notices)

- Each style has four OTS. Pressing one sets the keyboard voices (Right 1–3, Left) that suit the
  style and turns ACMP and SYNC START on, so the next left-hand chord starts the band. [V4
  01:51–02:30], OM p.47. *manual-confirmed*
- An OTS holds more than voices. The DL OTS column marks, for the keyboard parts: voice and part
  on/off, the part's mixer values (filter, EQ, insertion depth, chorus/reverb depth, pan, volume),
  insertion effect on/off and type, the Voice Edit set (octave, tuning, portamento, mono/poly,
  touch sensitivity, pitch bend range, modulation/aftertouch depths, organ flute settings),
  Keyboard Harmony/Arpeggio on/off, type and settings, the Multi Pad bank and its volume offset, and
  ACMP/Sync Start on. Not in OTS: tempo, transpose, Upper Octave, split points, fingering, style part
  mixer, Live Control. DL p.82–92. *manual-confirmed*. V2 shows a presenter storing a different Multi
  Pad bank on OTS 1 and OTS 2 [V2 01:05–02:54], which agrees.
- OTS Link: Main A–D recall OTS 1–4. Presenters describe it as the voice following the variation
  without touching the OTS buttons, and stress the fixed mapping A→1 … D→4. [V1 04:27–05:07, V4
  04:06–04:32], OM p.67. *manual-confirmed*
- With OTS Link off, the last OTS pressed simply stays; nothing follows the Mains. [V1 13:41]
  *video-only* (implied by the manual).
- OTS Link Timing: Immediate (at the press) or At Main Section Change (at the next measure, when the
  Main changes). RM p.11. *manual-confirmed*. No video in this pass shows the factory default.
- A player's own OTS: set the voices (and Multi Pad) as wanted, press MEMORY, then an OTS button.
  The instrument then asks to save the style: the OTS lives in the style file, so the player saves
  it as a User style (usually renamed, e.g. with their initial). Without that save the edit is lost
  on a style change or power off. OTS buttons not memorized keep the style's original OTS. [V1
  10:52–11:33, V2 00:17–00:49, V3 00:43–01:51, 02:22–03:02], OM p.60. *manual-confirmed*. The
  Registration checkboxes don't affect what an OTS stores (OM p.60).
- Players care about this: three of the four OTS videos are about personalising OTS (swap one sound,
  put your own Multi Pad on it), because OTS + OTS Link then drive a whole song from the Mains. [V1,
  V2, V3]
- Parameter Lock: OTS recall does not change locked groups (RM p.163); none of the DL lock groups
  (split point, fingering type, master EQ, reverb type/returns, VH) is an OTS item, so in practice
  locks never meet an OTS. DL p.84, p.88. *manual-confirmed*

## Manual check

- The videos agree with the manuals everywhere they overlap. V1 (PSR-S670) shows the same OTS Link
  mapping and memorize/save flow as the Genos manual.
- The factory default of OTS Link Timing is not in the manuals and not shown in any video this pass
  (yahaha's default is the owner's choice; issue #111 already asks for a hardware check).

## yahaha today

- OTS 1–4 recall Right 1–3 and Left: voice (bank/program, through the program map and the part's
  sound library, #103), on/off, CC7 volume and octave; the selected part goes to Right 1; Sync Start
  is armed while stopped. `src/parts.rs` `apply_ots`, `src/session/ots.rs` `recall_ots`, README
  "One Touch Settings".
- **README "Known gaps" is stale:** it still lists "OTS voice changes" as not done. That line dates
  from the first commit (75dd7fe), before OTS existed (3f3dcb5); OTS voices have been recalled since.
- The parser reads only bank select, program change, CC7 and the part on/off and octave SysEx from
  the OTSc tracks (`src/sff.rs` `parse_ots`). A scan of 60 corpus styles (counts only, nothing
  copied) shows every OTS track also carries pan (CC10), reverb and chorus sends (CC91/93), filter
  and envelope (CC71–74), portamento (CC5/65), RPN/NRPN (e.g. bend range), XG Multi Part SysEx
  (`43 1n 4C 08 ...`) and further Yamaha `43 73 01 50/51 ...` SysEx beyond on/off and octave. All of
  that is dropped: a recalled OTS leaves the part's pan and effect sends as they were (or as the
  sound library patch the new voice maps to sets them), not as the OTS sets them, and Keyboard
  Harmony/Arpeggio and the Multi Pad bank never change with an OTS.
- OTS Link: Main A–D → OTS 1–4, Immediate or At Main Section Change (default At Main Section Change,
  owner decision), with documented edge cases for fills, stops, style changes and missing Mains.
  `docs/fills-and-rules.md` §OTS Link timing. Matches RM p.11.
- OTS +/−, OTS 1–4 and OTS Link are assignable pedal functions (`src/controllers.rs`).
- Parameter Lock: an OTS recall never needs to ask, as documented (`docs/registration.md`
  §Parameter Lock). Matches the DL.
- No user OTS: `OtsCmd` has recall, link and timing only; there is no Memorize-to-OTS and nothing is
  written back to a style (issue #104 noted "OTS Memory" as not existing; #179 is about OTS and
  directly picked plugins).

## Gaps

| Gap | Priority | Suggested next step |
|---|---|---|
| ~~OTS recall drops the parts' pan, sends, filter/EG, portamento, bend range and XG part parameters~~ Done (#198, #238). A voice change puts filter/EG/vibrato/portamento/XG back to neutral (Voice Set); bend range stays (DL: not Voice Set). The built-in synth only sounds the bend range (#246); plugins never get the XG SysEx (#247) | – | #246, #247 |
| OTS doesn't switch Keyboard Harmony/Arpeggio or its type/volume (DL: OTS items) | P1 | Decode the unparsed `43 73 01 50/51` OTS SysEx against a Genos (Style Information / Harmony display after an OTS press); if they carry Harmony/Arp, apply them through the harmony_arp commands |
| OTS doesn't recall a Multi Pad bank or its volume offset | P2 | After the SysEx is decoded: map to yahaha's pad banks by name when one exists, else leave pads alone and say so |
| No user OTS memorize (MEMORY → OTS button) | P2 | Owner decision on storage: a yahaha sidecar keyed by style file, or writing a User copy of the `.sty` with a new OTSc chunk (Genos-like). Include plugin/patch voices (#104, #179) |
| README Known gaps says "OTS voice changes" are not done | P2 | Doc fix: replace with the real gap (OTS effects/harmony/pads not recalled) |
| OTS Link Timing factory default unknown | – | Hardware check (already in #111); yahaha's default stays the owner's choice |
