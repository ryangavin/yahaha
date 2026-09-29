# Hardware / software parity

The owner's rule: **nothing may work only on the Launchkey.** Every hardware function
(pads, buttons, faders, knobs, pedal/controller functions) needs a mouse-reachable app
control that sends the same command. The app has one generic Tauri command,
`send(cmd)` (`app/src-tauri/src/lib.rs`), which takes any `AppCmd` for the live and mock
backends, so parity is a question of UI, not of plumbing.

The Launchkey mirror (`app/src/panels/launchkey/Launchkey.svelte`) is clickable. Its pads,
page tabs, buttons (with a latchable Shift) and faders send the same actions as the
hardware. "Mirror only" below means the mirror is the only place to click it. That counts
as reachable, but it deserves a dedicated control where a player would look for it.
**MISSING** means no mouse-reachable UI sends the command.

Audited 2026-09-26 against `develop` at e54b512.

## Pads

| Page | Hardware function | AppCmd | App control |
|---|---|---|---|
| Sections | Intro I/II/III | `intro` | header/TransportBar.svelte |
| Sections | Sync Start / Sync Stop | `toggleSyncStart` / `toggleSyncStop` | TransportBar; settings/StylePage.svelte |
| Sections | Ending I/II/III | `ending` | TransportBar |
| Sections | Auto Fill | `toggleAutoFill` | StylePage |
| Sections | Main A–D | `main` | mirror only |
| Sections | Break | `break` | mirror only (FillButtons "Break" is `fillBreak`) |
| Sections | Tap / Start-Stop | `tapTempo` / `startStop` | TransportBar |
| Chord/Setup | Fingering types | `setFingering` | settings/ChordPage.svelte |
| Chord/Setup | Upper, Manual Bass, Stop Acmp | `toggleUpper` / `toggleManualBass` / `toggleStopAcmp` | ChordPage, parts/Parts.svelte, StylePage (set* equivalents) |
| Chord/Setup | Split −/+ | `moveSplit` | KeyStrip, Parts, settings/SplitPage.svelte |
| Chord/Setup | Transpose −/+/reset | `stepTranspose` / `resetTranspose` | settings/TransposePage.svelte |
| Chord/Setup | Retrigger | `toggleRetrigger` | StylePage |
| OTS/Parts | OTS 1–4, OTS Link | `recallOts` / `toggleOtsLink` | parts/Parts.svelte |
| OTS/Parts | Fade | `toggleFade` | StylePage |
| OTS/Parts | Voice −/+ | `stepVoice` | mirror only |
| OTS/Parts | Part on/off, part select | `togglePart` / `selectPart` | parts/PartStrip.svelte; mixer/Mixer.svelte |
| Quick Racks | Quick Racks 1–8, Bank −/+, Store, Rack −/+ (the bank-file and Freeze pads are dark) | `pressQuickRack` / `stepQuickRackBank` / `toggleQuickRackStore` / `stepQuickRack` | the Quick Racks bar (in the keyboard strip) and the Quick Racks drawer |
| Multi Pads | Pads 1–4, STOP, arm, stop one | `triggerMultiPad` / `stopAllMultiPads` / `armMultiPad` / `stopMultiPad` | multipad/MultiPad.svelte |

## Buttons

| Hardware function | AppCmd | App control |
|---|---|---|
| Pad Bank ▲/▼ | `setPadPage` / `cyclePadPage` | Launchkey mirror page tabs |
| Shift+▲ Left on/off, Shift+▼ OTS Link | `togglePart` / `toggleOtsLink` | PartStrip, Parts |
| Play | `startStop` | TransportBar |
| Stop | `stop` | mirror only (TransportBar's button is `startStop`) |
| Shift+Play Section Reset | `sectionReset` | mirror only (StylePage "Section Reset" is the `setSectionReset` setting) |
| Shift+Stop Fade | `toggleFade` | StylePage |
| Tempo −/+ (and both: reset) | `tempoDown` / `tempoUp` / `resetTempo` | TransportBar |
| Shift+tempo: Retrigger length | `stepRetriggerRate` | StylePage (`setRetriggerRate`) |
| Track ◀/▶ style | `stepStyle` | browser/Browser.svelte |
| Shift+Track ◀/▶ previous/next Quick Rack | `stepQuickRack` | the Quick Racks bar |
| Knob Assign ▲/▼ | `stepKnobPage` | **MISSING** |

## Faders

| Hardware function | AppCmd | App control |
|---|---|---|
| Panel faders 1–4, Style level, Multi Pad level, Master | `setPartVolume` / `setStyleVolume` / `setMultiPadVolume` / `setMasterVolume` | mixer/Mixer.svelte, PartStrip, AudioPage |
| Style faders 1–8 | `setStylePartVolume` | Mixer |
| Fader page (Master button) | `toggleFaderPage` | Mixer (`setFaderPage`) |
| Panel buttons 1–4 / Shift+1–4 | `togglePart` / `selectPart` | PartStrip, Mixer |
| Button 5 HARM/ARP | `toggleHarmonyArp` | harmony/Harmony.svelte, Mixer |
| Button 6 plugin reload | `reloadPartPlugin` | mirror only |
| Button 7 L Hold | `toggleLeftHold` | Parts, ChordPage |
| Button 8 / Shift+8 Looper | `looperOnOff` / `looperRec` | looper/Looper.svelte |
| Style buttons 1–8 | `toggleStylePart` | Mixer |
| Fader layers VOL/PAN/REV/CHO/DLY, Snapshots | — | covered by ui-prep (not on develop yet) |

## Knobs

| Page | Function | AppCmd | App control |
|---|---|---|---|
| any | page select / turn | `setKnobPage` / `stepKnobPage` / `turnKnob` | **MISSING** (the mirror has no encoders) |
| Style | DynCtrl | `setDynamics` | StylePage slider |
| Style | RtgRate / RtgOnOff | `stepRetriggerRate` / `toggleRetrigger` | StylePage |
| Style | StyMuteA/B | `styleTrackMute` | Mixer |
| Style | Swing (feel/swing PR) | `setSwing` | StylePage slider |
| all but Effects | Tempo | `setTempo` | **MISSING** (TransportBar has only −/+; the BPM readout is not editable) |
| Parts | part volumes, HarmVol, MetroVol | `setPartVolume` / `setHarmonyVolume` / `setMetronomeVolume` | Mixer, PartStrip, Harmony |
| Pan | part pans, effect returns | `setPartPan` / `setEffectReturn` | Mixer |
| Effects | part Reverb/Chorus sends | `setPartSend` | mixer/Strip.svelte |
| FX | effect parameters, delay time | `setEffectParam` | Mixer |
| Rack (controller map) | a part's insert slot 1–2 on/off (`partInsertOn`) | `setStripInsertOn` | the part's channel strip (mixer rework) |
| Rack (controller map) | a part's insert setting 1–4 (`partInsertSetting`) | `setStripInsertSetting` | the part's channel strip |
| Rack (controller map) | a part's send 4–6 level (`partSend`, sends 1–3 are the Reverb/Chorus/Delay sends) | `setStripSend` | the part's channel strip |
| Rack (controller map) | a part's delay send (`partDelay`) | `setPartSend` (`variation`) | mixer/Strip.svelte |
| Rack (controller map) | rotary speed (`rotaryFast`) | `setRotaryFast` | effects/Effects.svelte "Rotary fast" |

Panel faders 1–4 on the Volume layer run the same targets (`moveRackFader`): a level or
setting across its range, a switch on from 64. The Rack drawer's controller map editor
assigns them (`setRackControl`).

## Pedal and controller functions

settings/PedalsPage.svelte assigns them (`setPedal`, `learnPedal`) and can fire any switch
function with its "Try" button (`triggerFunction`).

| Function | AppCmd | App control |
|---|---|---|
| Sustain / Sostenuto / Soft, Modulation, Pitch Bend | engine | PedalsPage only: expression-pedal jobs by design |
| Dynamics Control | `setDynamics` | StylePage slider |
| Start/Stop, Sync, Intro, Ending, Auto Fill, Stop Acmp, Fade, tempo, Tap | as the pads/buttons | TransportBar, StylePage |
| Main A–D | `main` | mirror only |
| Fill Down/Self/Break/Up | `fillDown` / `fillSelf` / `fillBreak` / `fillUp` | settings/FillButtons.svelte |
| Section Reset | `sectionReset` | mirror only |
| OTS Link, OTS 1–4, OTS next/prev | `toggleOtsLink` / `recallOts` | Parts |
| Unison (pedal function `unison`; no Launchkey pad) | `toggleUnison` / `setUnisonHeld` / `setUnisonType` | settings/StylePage.svelte (`transport.unison`, `settings.unison_type` tips) |
| Quick Racks Bank ± (`snapshotBankNext`/`snapshotBankPrev`), Quick Rack 1–10 (`regist1`–`regist10`), Store (`registMemory`), Next/Previous Quick Rack (`registNext`/`registPrev`) | as the pads | the Quick Racks bar, PedalsPage. Registration Bank ±, Freeze and Sequence are no longer available. |
| Transpose ±, part on/off, Fingered On Bass, Harmony/Arp, Chord Looper, Left Hold | as above | TransposePage, PartStrip, ChordPage, Harmony, Looper, Parts |
| Arp Hold (pedal) | `toggleArpPedalHold` | **MISSING** (Harmony's "Arp Hold" is the panel switch `toggleArpHold`) |

## To add

MISSING, where it belongs:

1. `setTempo`: an editable BPM readout in header/TransportBar.svelte.
2. `setKnobPage`, `stepKnobPage`, `turnKnob`: the eight encoders and the Knob Assign ▲/▼ buttons in launchkey/Launchkey.svelte. Every knob function already has its own control.
3. `toggleArpPedalHold`: beside Arp Hold in harmony/Harmony.svelte.

Mirror only, which needs a dedicated control:

5. `main` and `break`: in TransportBar, next to Intro and Ending.
6. `stop`: in TransportBar.
7. `sectionReset`: in TransportBar or FillButtons.
8. `stepVoice`: in PartStrip.
9. `reloadPartPlugin`: in PartStrip, next to the plugin toggle.

## Insertion effects

The style's per-part insertion effects (#269) have no Launchkey control; the app has them
all, in mixer/Mixer.svelte (Inserts row):

| Function | AppCmd | App control |
|---|---|---|
| All inserts on/off | `setInsertsOn` | Mixer "Inserts" |
| One part's insert on/off | `setPartInsertOn` | Mixer, the part's name toggle |
| One part's insert amount (drive, squeeze, sensitivity, depth) | `setPartInsertAmount` | Mixer, the part's "Amt" knob |
| Rotary speaker fast/slow | `setRotaryFast` | Mixer "Rotary Fast" |

## AU presets (sound browser)

The Launchkey has no sound-catalog browser (its Voice −/+ steps the GM voices, `stepVoice`),
so it has no way to browse AU presets either; nothing about them is hardware-only. A
preset reaches the Launchkey the way every catalog sound does: store the part in a
Registration Memory or an OTS and recall that from the pads. The recall restores the
plugin's state, which is the preset.

