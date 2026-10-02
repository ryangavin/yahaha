# Ink redraw: decisions checklist

This is what the Ink redraw must show. A reviewer ticks each box against the boards. Every item has one line for the decision, its source with a date, and a test the board can pass or fail.

**Sources**
- owner memory: `mvp-sprint-simplify.md`, `ui-redesign-decisions.md`, `software-parity-rule.md`, `genos-questions-use-manual.md`
- PRs merged into `develop` (#301 to #497, 2026-09-26 to 09-30)
- docs: `docs/eyes-free.md`, `docs/racks.md`, `docs/design/redesign-b-handoff.md`, the "Decision:" lines in `docs/*.md`, `docs/app-api.md`
- rules: `AGENTS.md`
- review: `ink/REVIEW-compat.md`

**Precedence:** where two decisions disagree, the newer one wins. Conflicts are marked **⚠ CONFLICT**, with the newer side named. The four owner decisions made after the review (2026-09-30) are marked **[NEW]**.

---

## Hardware mapping (Launchkey MK4, the band on every board)

- [ ] **H1 [NEW]** The fader block (9 faders and the 9 buttons under them) is always visible, on every page and overlay. It may go half height on tall pages, but only if unavoidable. (owner, post-review 2026-09-30) · **Test:** on each of the 24 boards, 9 faders with 9 buttons are visible. Any half-height board says why in its notes.
- [ ] **H2** Layout left to right: the faders and their buttons on the left; Shift, Track ◀ ▶ and the optional screen in the middle; 8 knobs directly over 16 pads (2×8) on the right; Pad Bank ▲▼ left of the pads; Scene and Function right of them; transport at the far right; the keys as a thin strip along the bottom. (mvp-sprint-simplify, layout rule and round 3, 2026-09-30) · **Test:** the order matches on the Stage board. The key strip is thin (it is not a 1/5-height keyboard).
- [ ] **H3** Pad page 1 is Sections and never moves. Top row: Intro I, II, III, Sync Start, Ending I, II, III, Auto Fill. Bottom row: Main A–D, Break, Tap, Sync Stop, Start/Stop. (eyes-free.md "Page 1 stays exactly as it is"; `pages/sections.rs`) · **Test:** 16 pad labels in this order.
- [ ] **H4** Pad pages 2–5 default to Racks, Chord, Multi Pads, Setup. The player can reorder them or leave any out in Settings, so the page count is 1–5. (eyes-free.md; #477, #487) · **Test:** the pager reads "n/N" from the page order. One board shows a trimmed order, for example 1/3.
- [ ] **H5** The Racks page: top row is Quick Racks 1–8 of the bank on view. Bottom row: OTS 1–4, Bank −, Bank +, Store, and one dark spare. (eyes-free.md; #477, #488; `pages/racks.rs`) · **Test:** Stage-Racks has exactly these pads, with the 16th dark.
  - **⚠ CONFLICT:** racks.md "Screens" (2026-09-28) put prev/next rack on the pads. The newer eyes-free decision (2026-09-29/30) moved them to Shift + Track. Draw the eyes-free layout.
- [ ] **H6** The Chord page: bottom row is Manual Bass, Stop ACMP, Split −, Split +, Kbd Tr −, Kbd Tr +, Tr Reset, Retrigger. The top row is dark. (#477; `pages/chord.rs`) · **Test:** a Chord-page board or pager thumbnail shows the top row dark.
- [ ] **H7** The Multi Pads page: top row is Pad 1–4, STOP, then 3 dark. Bottom row: Select 1–4, Stop 1–4. (`pages/multipads.rs`; multipad.md "dedicated Select and Stop pads") · **Test:** labels as listed.
- [ ] **H8** The Setup page: top row is the 7 fingerings plus UPPER. Bottom row: OTS LINK, ACMP STYLE, ACMP FIXED, then 5 dark. Each switch is saved in settings. (#477, #487; `pages/setup.rs`) · **Test:** labels as listed. Upper is a switch, not an 8th fingering.
- [ ] **H9** Pad lamps follow the hardware LED semantics. Queued flashes, armed pulses, and a section the style lacks is dark. Racks: red = loaded, blue = stored, dark = empty, all flashing while Store is armed. Multi Pads: blue = data, red = playing, amber = waiting for the bar, flashing red = armed. (`launchkey.rs:963-974`; #488) · **Test:** Stage-Racks shows loaded, stored and empty as three distinct states. One Stage board shows a missing section dark and a queued Main flashing.
  - The Ink family hues may replace the LED colours on screen only if the engine's LED palette changes too. **Owner call needed:** the screen and the pads must not disagree (REVIEW #10).
- [ ] **H10** Buttons: Play = Start/Stop, Stop, Scene = tempo +, Function = tempo −, Pad Bank ▲▼ = pad page, Track ◀ ▶ = previous/next style, encoder ▲▼ = knob page. (`launchkey.rs:22-32`) · **Test:** each button is captioned with its function.
- [ ] **H11** Shift layer: Play = Section Reset, Stop = Fade, Scene/Function = **Retrigger shorter/longer**, Pad Bank ▲ = Left on/off, Pad Bank ▼ = OTS Link, Track ◀ ▶ = previous/next Quick Rack, encoder ▼ = ACMP, encoder ▲ = Rotary slow/fast (▲ lit while fast), fader buttons 1–4 = select part, button 8 = **LOOP REC**, master button = **LAYER**, Style-page button 6 = mute Pad. (`surface.rs:57-110`; #469, #477) · **Test:** the Stage-Shift board shows all 13, including RTG SHORT/LONG, LOOP REC and LAYER.
- [ ] **H12** Fader button 6 is **Sound** on both fader pages: dim white, bright white while held. (#477) · **Test:** button 6 reads "Sound" on the Panel and Style pages alike.
- [ ] **H13** Knob pages 1–6: Style, Rack, Pan, Reverb, Chorus, Delay. Style = DynCtrl, RtgRate, RtgOnOff, StyMuteA, StyMuteB, Swing, ---, Tempo. Rack = the live rack's controller map; by default Right1, Right2, Right3, Left volumes, HarmVol, MetroVol, ---, Tempo. (`src/knobs.rs:69-107,366`; #392, #310) · **Test:** knob names match exactly. Page 2 is titled "Rack", not "Voice".
- [ ] **H14** Dynamics defaults to max (127). (software-parity-rule, 2026-09-26) · **Test:** DynCtrl reads 127 on a fresh stage.
- [ ] **H15** Nothing is hardware-only: every Launchkey function has an on-screen control. (AGENTS.md "Controls"; software-parity-rule) · **Test:** for each function in H10–H13 and M1–M4, the reviewer can point at a clickable control on some board.

## Mixer and levels

- [ ] **M1 [NEW]** The stage keeps the 9-fader hardware band, not the always-visible 12-strip mixer. (owner, post-review 2026-09-30) · **Test:** there is no 12-strip row on the Stage. The band is present.
  - **⚠ CONFLICT:** "Mixer row always visible, 12 strips, all parts equal" (ui-redesign-decisions 2026-09-25/26; handoff; built in #457, #481, #496 on 2026-09-29/30). The newer decision is [NEW] M1. The shipped `MixerRow` is superseded.
- [ ] **M2 [NEW]** On the Panel page, fader 5 is **Style** volume and fader 6 is **M.Pad** (Multi Pad) volume, each with a readout and a soft-takeover mark. Faders 7–8 are unused. Buttons 5–8 are Harm/Arp, Sound, L Hold and Looper. (owner 2026-09-30; `surface.rs:176-190`; app-api.md:123-124) · **Test:** fader 5 is labelled "Style" and moves a Style-volume readout, 0–127 with 100 = as written. Fader 6 is labelled "M.Pad". The labels of buttons 5–8 sit under the faders and are never used as fader names.
- [ ] **M3 [NEW]** Fader layers VOL, PAN, REV, CHO and DLY. The layer is named on the master button ("PANEL REV") and by the fader-button colours (VOL blue, PAN yellow, REV cyan, CHO pink, DLY white). The fader labels stay the part names. On the Style page, PAN leaves the faders unused. (owner 2026-09-30; #408, #453, #481) · **Test:** there is an on-screen layer selector, and one board shows the REV layer with the send values on the faders. Style-page PAN shows "—".
- [ ] **M4** The Style fader page: faders and buttons 1–8 are the 8 style parts (Rhythm 1 … Phrase 2) with mutes, button 6 is Sound, and the master is master. Flipping the page is the master button or an on-screen Panel/Style switch. (`launchkey.rs:34-40`) · **Test:** Stage-Racks shows 8 part colours and names, mutes, and button 6 = Sound.
- [ ] **M5** Mixer rule: every change in a part's loudness shows on a control, either the part's own fader or the group control that scales it (Style volume, M.Pad volume, Fade). Style volume and M.Pad volume scale the CC7 that is sent, not the audio. (AGENTS.md "Engine rules"; #413, #415) · **Test:** with the Panel page up, the reviewer can still reach each style part's level in one click (the Style page, or the Channel page).
- [ ] **M6** Panel faders 1–4 follow the live rack's **controller map**, so a fader may read HARMVOL or SPLIT, or be unused. (#392) · **Test:** fader labels are drawn from state. The board notes say this.
- [ ] **M7** Soft takeover: a level that the hardware fader hasn't reached yet shows a "waiting" mark, and the fader's physical position shows as a ghost. The mark follows the layer. (#303, #453) · **Test:** at least one fader shows the waiting mark and the ghost.
- [ ] **M8** Meters are peak and RMS per channel plus the master, polled about every 100 ms. The compressor has no gain-reduction meter. (#302; types.ts `Meters`) · **Test:** fader meters appear; no "GR" readout on the Channel board.
- [ ] **M9** Master Compressor and Master EQ (8 bands) on the master bus, each with on/off and a type. (#420) · **Test:** they are reachable from the master fader or the Effects page.
- [ ] **M10** Solo per part (keyboard and style), and Style Track Mute A/B. (#457; tooltips `mixer.solo`, `mixer.track_mute`) · **Test:** Solo is on the Channel page. Track Mute is on the Style knob page and has a screen control.
- [ ] **M11** Strip marks: ⚠ plugin missing and ● sound edited, on keyboard parts only. (#493) · **Test:** one fader label shows ⚠, another ●.

## Racks and Quick Racks

- [ ] **R1 [NEW]** Snapshots, Registration and Freeze are **gone**. Everything reconciles to **Quick Racks**: banks A–H, 8 buttons each, one saved rack per button. (owner 2026-09-30; racks.md; #383, #394; app-api.md:259) · **Test:** no "Snapshot", "SNAP", "Registration" or "Freeze" text on any board. The Stage header shows the live rack's name and the lit Quick Rack, for example "Rack: Sunday drive ● · A1".
  - **⚠ CONFLICT:** the handoff (2026-09-26) had Registration buttons 1–10 with Freeze. ui-redesign-decisions (2026-09-26) had "Snapshots ×8, Memory → Store". racks.md and #383 (2026-09-28/29) replaced Registrations with Quick Racks. The Ink mock (2026-09-30, first round) drew Snapshots again. The newest is [NEW] R1.
- [ ] **R2** A rack is the four keyboard parts (sound and mix), the split, Harmony/Arp, the transpose, the controller map and the send effects. It excludes the style parts, Multi Pads, the tempo and the style. (racks.md "Words", "The model") · **Test:** the Rack drawer shows only rack contents.
- [ ] **R3** The live rack autosaves. **Modified ●** shows as soon as anything differs, including a plugin edit, within about 1 s. (racks.md "Saving"; #377, #365) · **Test:** the Stage header and the Rack drawer show ● modified.
- [ ] **R4** Switching racks with unsaved changes asks **Save first / Discard and switch / Keep editing**. From the hardware or a pedal there is no dialog: the switch goes ahead and the old rack is kept as "Recovered: <name>". (racks.md; app-api.md:215,708) · **Test:** the Prompts board has those three buttons. Its notes say hardware never shows the dialog.
- [ ] **R5** An edited factory or file preset becomes a new sound on save, and a **sound-names prompt** asks for its name. (racks.md "Saving"; types.ts `RackPrompt.soundNames`) · **Test:** the Prompts board includes the sound-names prompt.
- [ ] **R6** Store: arm **Store**, then tap a button. The one-step capture is a long press (500 ms) or right-click on screen, and hold **Sound** + tap on the hardware. Overwriting asks **no confirm**. A name is asked **only** when the live rack was never saved (storeWaiting). An auto-name is "Rhodes Soft + Strings". (#492, #488, #495; eyes-free.md "No dialog") · **Test:** the "Store to A4? Name this rack" prompt is drawn only for a never-saved rack. No confirm on overwrite.
- [ ] **R7** Under the Sound hold, a pad holding **another** rack recalls it. Only the lit pad and empty pads capture. (#488) · **Test:** the notes on the Sound-hold board say this.
- [ ] **R8** Delete is refused for the loaded rack. It empties the Quick Rack buttons that held the rack, and gives back every OTS that loaded it. (app-api.md:702) · **Test:** the delete prompt names the emptied slots and the OTS side effect. Delete is disabled on the loaded rack.
- [ ] **R9** Style racks: each of the style's OTS 1–4 can load one of your racks instead ("Style's own" to revert), kept per style. OTS Link is off by default, and its timing default is "At Main Section Change". (racks.md "Styles and OTS"; #390; genos-questions memory) · **Test:** an OTS → rack selector exists (Library › Racks or the Rack drawer). The OTS Link timing reads Immediate / At Main change.
- [ ] **R10** The Quick Racks buttons are reachable on screen at all times: a bank pager, 1–8 with names, Store, ✕ to clear. The Quick Racks row stays one row tall. (#480; racks.md "Screens") · **Test:** Quick Racks are visible on the Stage when the pads are on page 1. **Owner call:** the redraw moved them onto pad page 2; if that is the only place, the checkbox fails.

## Eyes-free layers (v1 target)

- [ ] **E1** **Sound hold:** hold fader button 6 and the pads become the Racks page from any page; let go and they snap back. On screen, a click latches and a press over 350 ms is momentary. The Racks tab is marked dashed while held. (eyes-free.md; #494, #497) · **Test:** a board shows Sound held from page 1, with the pads as Racks and a dashed tab.
- [ ] **E2** **Swap mode:** hold a part's Panel button 1–4 and turn a knob. Knob 1 steps that part's sound by **number**. Knobs 2–8 are Level, Pan, Rev, Cho, Dly, Insert 1 amount and Send 4. It is live, commits on release, and a tap still toggles the part. On screen: a 350 ms long press on the part's **On** latches it; a click on the lit On lets go. (eyes-free.md; #489, #497) · **Test:** a board shows swap for R1, with the knob captions as listed and knob 1 reading "23 Rhodes Soft".
- [ ] **E3** **Sound numbers:** favourites are numbered 1–n, then the rest by category order. The dial clamps at 1 and at the last number; it does not wrap. The Launchkey display reads `R1: 23 Rhodes Soft`. (eyes-free.md; #482, #485, #489) · **Test:** numbers appear in Library › Sounds, the quick sound list and the swap knob.
- [ ] **E4** The Launchkey display confirms every control's action and value, so the screen is for setup, not for the jam. (eyes-free.md "The outcome") · **Test:** none required. Optionally a hardware-screen echo line.

## Stage and display

- [ ] **S1** The mission: gorgeous, bespoke, immediately accessible, easy with the MK4, "so inviting people don't want to stop playing". The principle: complexity revealed in layers. (mvp-sprint-simplify, 2026-09-30) · **Test:** the Stage shows only the style, chord, section, next, tempo and the sounds, plus the band.
- [ ] **S2** Cohesion: one label style, one tile/button style, one control height (44 px primary, 32 px small), one gap (12 px), even density. (mvp-sprint-simplify) · **Test:** spot-check 3 boards for mixed heights and gaps.
- [ ] **S3** The design is Ink B, dark, used as the system. Hairlines, huge numerals, thin-stroke controls, colour as structure. The owner will supply the palette later; no Claude orange+blue. (mvp-sprint-simplify, 2026-09-30) · **Test:** there are colour tokens with roles, and no #d97757.
  - **⚠ CONFLICT:** round 3 said "every style ships dark AND light mode"; the Ink brief says "this round is dark", and the app has a theme switch today. The newer brief narrows the scope to dark for now. A light twin is still owed.
- [ ] **S4** Per-style artwork is the Stage background, and its key colour tints the stage and the chord. (ui-redesign-decisions v7; mvp round 3) · **Test:** drawn on the Stage. Its notes say **nothing generates artwork yet** (a new feature).
- [ ] **S5** Liveness: beat and bar motion, audio-reactive parts, the chord as a visual. It stays calm. (mvp round 3) · **Test:** a beat lamp, bar progress and meters are drawn.
- [ ] **S6** Home shows the style, tempo, bar and beat, the section playing and the queued one (with the fill's **landing** Main), the chord and its tones, the rack and OTS names, and the **band sends**. The sections use the real Main patterns. (ui-redesign-decisions round 2; #300) · **Test:** "Next" and the landing are shown; the band sends appear somewhere on Home.
- [ ] **S7** The header transport stays: Start/Stop, Sync Start/Stop, Intro, Ending, Tempo −/value/+, Tap, bar·beat. It is reachable whatever the pad page. (handoff; post-review pushback accepted 2026-09-30) · **Test:** a transport row is present on the Stage, Library and Settings boards.
- [ ] **S8** Tooltips on every control, a help footer or help mode, and a status line for `state.message` (refusals, errors). (AGENTS.md "Controls"; post-review pushback accepted) · **Test:** a status line and a help affordance on the Stage.
- [ ] **S9** The display tabs replace only the Stage zone; the band and the keys stay. The tabs are Home, Channel, Effects, Multi Pads, Looper, Harm/Arp. Settings is no longer one of them (see [NEW] G1). (Ink IA 2026-09-30; handoff tabs) · **Test:** **one** identical tab row on every display board. Rack is reachable from the Stage (for example from the rack name).
- [ ] **S10** The Channel page opens from a part: a click on a fader's name, or ‹ ›. ‹ › steps through **all 12** parts, style parts included. It shows Level, Pan (keyboard parts only), EQ, Compressor, 2 inserts, sends 1–6, Tone, Play (Mono, Portamento, Octave, Bend) and Solo. (#461, #468; round 2 "clicking a strip jumps to Channel") · **Test:** the Channel board reads "PART 1 OF 12". Tone and Play are drawn, and the compressor types are Natural/Rich/Punchy/Electronic/Loud.
- [ ] **S11** Compressor presets have unity makeup; the player adds makeup. (#464) · **Test:** make-up starts at 0 dB.
- [ ] **S12** Insert kinds: None, Distortion, Compressor, Auto Wah, Tremolo, Rotary, Phaser. Each has 2–4 named settings. The compressor and EQ are **not** insert kinds on the strip. (#459, #464) · **Test:** no "Tape Warmth" or "EQ · Warm body" insert.
- [ ] **S13** The first run needs a loaded style, because the engine always loads one. Launchkey hotplug is automatic. (`session/library.rs`; `devices.rs`) · **Test:** there is no "No style loaded" state and no "Reconnect" / "Look again" button, unless marked as new engine work.
- [ ] **S14** The layout adapts from 1024×700 up to 1920, and to the iPad. (ui-redesign-decisions round 2; #491, #347) · **Test:** the notes on the Stage and Settings boards say what shrinks at 1024×700.
- [ ] **S15** The on-screen keys show the held notes by part, the split, the chord-detection area and the chord tones. There are 49, 61 or 88 keys. (#301+; KeyStrip) · **Test:** the detection area is drawn.
- [ ] **S16** Voice-name click → a **quick list** (recents and favourites, with numbers) → "More…" goes to Library. (ui-redesign-decisions; handoff) · **Test:** the Stage-SoundPick board.
- [ ] **S17** Left Hold, Manual Bass (Upper only), Unison, Metronome and Panic each have a screen control. (#317, #457; tooltips) · **Test:** the reviewer can find each of them.

## Library

- [ ] **L1** Library is a full-screen page with a Stage | Library switch (Alt+B). The band keeps playing. The Ink tabs are Styles, Sounds, Instruments, Racks, Style map. (racks.md "Screens"; Ink IA; handoff "Browser full screen") · **Test:** a one-line bar with the tabs and "Back to stage", plus the always-visible fader block (H1).
  - **⚠ CONFLICT:** the handoff's browser tabs (Styles/Voices/Pads/Songs, 2026-09-26) versus racks.md's Library (Racks/Sounds/Instruments/Style map, 2026-09-28) versus Ink (Styles + those four, 2026-09-30). The newest is Ink. Pads (Multi Pad banks) and Songs have no tab.
- [ ] **L2** The style list order is fixed (folder, then name) so Track ◀ ▶ agree with it. Preview/audition only while **stopped**; while playing, Load queues for the next bar. (Browser.svelte; types.ts `PreviewCmd`) · **Test:** there is no Sort control. The preview is shown with the band stopped. While playing, the Load button reads "at next bar".
- [ ] **L3** Clicking a sound plays it on the target part at once (`assignSound`); ↑ ↓ step. The "Loads into R1 R2 R3 L" target is shown. (racks.md "Screens") · **Test:** Sounds shows the target selector. The row click is the load (no separate Load step required).
- [ ] **L4** Sound badges: Mine, Factory, SoundFont (and Style). There are 13 categories: Piano, E.Piano, Organ, Guitar, Bass, Strings, Brass, Sax/Woodwind, Synth Lead, Pad, Choir, Drums/Perc, SFX. (racks.md; `sound-library.ts:5`) · **Test:** category names match the list.
- [ ] **L5** Sounds are raw instruments with no mix. "One save makes one record": picking a preset adds nothing. (#378, #367) · **Test:** the sound details show no level, pan or effects.
- [ ] **L6** Plugins: **New** until first opened. **Missing** keeps racks and sounds, silences the parts and shows ⚠. The "Needs attention" filter; **Replace…** keeps the mix. (racks.md "Plugins coming and going"; #376) · **Test:** the Instruments and Racks boards show New, Missing, the filter and Replace…. There is no "Locate…".
- [ ] **L7** The Style map is a Library tab: global or this style, 16 families plus drums, per-program override, auto from the most GM-complete font, and a rule's level. A style's rules beat the global ones. (sound-library.md Decisions; #321, #323) · **Test:** the Global/This-style switch, the family levels, the auto badge.
- [ ] **L8** The Library's Quick Racks bar and its docked Rack panel stay or move somewhere named. (racks.md "Screens"; #388) · **Test:** both are placed on the Library boards.

## Settings

- [ ] **G1 [NEW]** Settings is a **full-screen page** like Library, with the band and keys kept per H1. (owner, post-review 2026-09-30) · **Test:** the Settings boards are full-window, with a page list on the left and "Back to stage".
  - **⚠ CONFLICT:** the handoff (2026-09-26) had Settings as a display tab, the app has it as a drawer, and the first Ink IA (2026-09-30) had it as a display page. The newest is [NEW] G1.
- [ ] **G2** The pages follow the Genos menus: Chord, Split, Transpose, Style, Pedals, Lock, then Audio, MIDI, Launchkey, Library. (Settings.svelte) · **Test:** the page list has these 10, or merges that are named.
- [ ] **G3** Fingering has 7 types; Upper is a separate switch, and Manual Bass works in Upper only. There is one split point; the chord-settle window is 0–30 ms (default 10). (types.ts 14; genos-features.md:955) · **Test:** Upper is not in the fingering radio. The settle control is present.
- [ ] **G4** Style settings use only the real options. Main timing: Immediate / Next bar. Intro/Ending timing: Next bar / End of section. Sync Stop window, Fade in/out/**hold** times, Section Reset (default **ON**), Retrigger on plus its rate, Stop ACMP Off/Style/Fixed, OTS Link timing Immediate / At Main change, Change Behavior tempo and parts Lock/Hold/Reset plus Section Set, Swing plus its grid, Section tempo, Dynamics (control, level, Touch, Accent, threshold, mode, source). (types.ts 779-842; genos-questions memory; fills-and-rules.md:163) · **Test:** no "Start on chord", "Stop at bar end", "After a fill", "Fade holds tempo" or "Dynamics: Medium".
- [ ] **G5** Pedals: 3 pedals, each with a CC, a function from the assignable table, a control type, reverse and range, plus Learn. The defaults are CC 64/66/67. Per part: sustain, pitch bend and modulation reach, and a per-part bend range. (controllers.md:107-134) · **Test:** there is no "Expression" function or row. The bend range is per part.
- [ ] **G6** Parameter Lock has only Split point and Fingering type. It applies to rack and OTS recalls. (types.ts 727-734; LockPage) · **Test:** exactly 2 lock toggles.
- [ ] **G7** Audio: synth on/off, the output **pair**, buffer 64–1024, master volume. MIDI: inputs (all or picked), palette-LED mode (RGB / Palette). The virtual output is always on, and the Launchkey reconnects automatically. (types.ts 172-199) · **Test:** there is no LED "Dim/Off", no output-device picker and no Reconnect.
- [ ] **G8** Launchkey: the pad page order, with drag to reorder and **leave-out** toggles; Sections is fixed. (#487) · **Test:** a leave-out control exists.
- [ ] **G9** Library: the style folders (read-only today), the indexed counts, Rescan. (LibraryPage) · **Test:** "+ Add folder" is marked as needing a new command, or it's dropped.

## Effects

- [ ] **F1** Reverb, Chorus and Delay cards, each with **From style / Mine**. Choosing a type or turning a parameter makes it Mine. (#237; Effects.svelte) · **Test:** both chip states are drawn.
- [ ] **F2** The types exist only as defined. Reverb: Hall, Room, Stage, Plate. Chorus: Chorus, Celeste, Flanger. Delay: 1/8, dotted 1/8, 1/4, Ping-pong. Parameters: reverb Time, Pre-delay, Tone; chorus Rate, Depth (no feedback); delay Sync, Note/Time, Feedback, Tone, Ping-pong. (types.ts 537-563; knobs.rs) · **Test:** no "Ensemble", no "Mono/Stereo", no chorus FDBK.
- [ ] **F3** Band send and **Pad send** per block (percent), plus the return level. (#236, #267) · **Test:** both sends appear on each card.
- [ ] **F4** Added sends 4–6 (any kind, the phaser included) and Add send; the rack's "keeps type" override on sends 1–3 with a "Set by rack" badge. (#459, #461, #463) · **Test:** one added-send card and one override badge.
- [ ] **F5** The style's insertion effects: all on/off, per part on/off and amount, rotary fast. (#269, #299) · **Test:** an Inserts section on Effects.
- [ ] **F6** Keyboard parts' sends start **dry**, and sends the player dials in stick (OTS Link doesn't reset them). (#308) · **Test:** fresh R1–L sends read 0 (except the style defaults noted).

## Other display pages

- [ ] **D1** Multi Pads: the bank, 4 pads with their lamps (empty, ready, armed, queued, playing), Select and Stop per pad, STOP, Repeat and Chord Match per pad. Synchro Stop has **two** switches (Style stop defaults on, Ending off). The M.Pad volume is fader 6. Pads 6–8 of the top row stay empty. (multipad.md Decisions; handoff "pad level (fader 6)") · **Test:** two Synchro Stop switches; the armed lamp is drawn; the M.Pad level is shown.
- [ ] **D2** Looper: REC/STOP and ON/OFF with their states (recArmed, recording, loopArmed, looping); the sequence with a playhead; memories 1–8 (auto-named, no rename); Memory and Clear latches; New bank, Load, Save As. Fader button 8, and Shift + 8 = REC. (Looper.svelte; #201) · **Test:** memory names are like CLD_001, not "Verse", unless marked as a new command.
- [ ] **D3** Harmony/Arp: one switch (fader button 5, J) and one type, harmony or arpeggio. Harmony types come in Data List order (Standard Duet 1 …, Echo/Tremolo/Trill, Multi Assign). Arp categories: UpDown, Random, AsPlayed, ChordStab, BrokenChord, Guitar, Sequence. The settings depend on the type. The Arp Hold pedal is separate from the Hold setting. (harmony.rs; arpeggio.md:80,175) · **Test:** the category names match.

## Out of scope for v1

- [ ] **X1** **Charts** (the iReal chart player and chord lane) are punted from the first release; there is no Charts tab. (mvp-sprint-simplify "Scope", 2026-09-30) · **Test:** no Charts tab. The notes say the shipped Charts drawer and lane are hidden, not deleted.
  - **⚠ CONFLICT:** the handoff (2026-09-26) had "Looper & Charts" as one tab, and Charts ships in the app today. The newer decision (2026-09-30) punts it.
- [ ] **X2** Setlist, Playlist and Registration Sequence are gone; a setlist comes later. (racks.md "Migration") · **Test:** none drawn.
- [ ] **X3** Plugin macros ("learn" a plugin parameter) in the controller map may ship after v1. (racks.md "Deviations") · **Test:** none drawn, or marked "later".
- [ ] **X4** Style Editor, and the browser's Voices, Pads and Songs tabs: not drawn. (handoff "Not drawn yet") · **Test:** absent.
- [ ] **X5** The follow-up issues wait; prefer removing or hiding controls over adding them, and new surface area needs a reason. (mvp-sprint-simplify) · **Test:** each control the redraw adds beyond this list has a one-line reason in the board notes.
- [ ] **X6** Genos ground-truth capture isn't needed for v1; v1 is a presentation and reach pass on `develop`'s features. (eyes-free.md "What v1 is") · **Test:** no board depends on a feature the app lacks unless it's marked "new work".

---

## Conflicts, summarised (the newer wins)

| # | Older | Newer (wins) |
|---|---|---|
| M1 | 12 always-visible strips (2026-09-25/26 owner; #457, #496 on 09-29/30) | 9-fader hardware band ([NEW] 2026-09-30) |
| R1 | Registration 1–10 + Freeze (handoff 09-26) → Snapshots ×8 (09-26) → drawn again in the Ink round 1 (09-30) | Quick Racks A–H × 8 (racks.md 09-28, #383; [NEW] 09-30) |
| G1 | Settings as a display tab (handoff 09-26) / a drawer (the app) / a display page (Ink IA 09-30) | full-screen page ([NEW] 09-30) |
| H5 | prev/next rack on the Quick Racks page (racks.md 09-28) | the Racks page with OTS, Bank ±, Store and a spare; prev/next on Shift + Track (eyes-free 09-29/30) |
| L1 | browser tabs Styles/Voices/Pads/Songs (handoff 09-26); Library without Styles (racks.md 09-28) | Library: Styles, Sounds, Instruments, Racks, Style map (Ink 09-30) |
| X1 | "Looper & Charts" tab (handoff 09-26) | Charts punted (09-30) |
| S3 | every style dark AND light (round 3, 09-30) | dark only this round (Ink brief, 09-30); the light twin is still owed |
| M3 | small REV/DLY knobs on every strip (handoff 09-26) | fader layers replace the per-strip minis (round 2 09-26; #481) |
| — | the Launchkey mirror beside the mixer (handoff 09-26) → into the header (canvas v7 09-26) → a flat knobs+pads row with no faders (#484, 09-30) | the full hardware band with faders always visible ([NEW] H1, 09-30) |

Open owner calls (not decided anywhere): whether the pad colours change on the hardware too (H9), where the Quick Racks sit when the pads aren't on page 2 (R10), and the Channel entry point without strips (S10).

## Decided by the orchestrator on 2026-09-30 (owner can veto)

- [ ] **Pad colours:** the screen draws each pad in its hardware LED family (Intro yellow/ochre, Main green/lime, Ending red/rose, Break violet, Auto Fill / Sync / Tap / Start cyan-white; Racks page: loaded red, stored blue, empty dark). The engine's LED palette does not change. Test: every pad on screen matches the family the engine lights on the hardware.
- [ ] **Quick Racks visibility:** no permanent Quick Racks row on the Stage. Quick Racks are reachable from pad page 2, from the Sound hold on any page, from the Quick Racks display page (tab row) and from the lit slot + live rack name in the Stage header. Test: the header shows the live rack name (● when modified) and its slot; the tab row has QUICK RACKS.
- [ ] **Opening Channel without strips:** clicking a part's name on its fader strip, or its sound in the Stage SOUNDS list, opens the Channel page for that part; Shift + fader button 1–4 (select part) does the same from the hardware; ‹ › steps all 12 parts. Test: the fader strip's part name is a button; the Channel page shows which gesture opened it.
