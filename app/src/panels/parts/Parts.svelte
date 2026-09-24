<!--
  DRAWER SLOT: Keyboard parts + One Touch Settings (right-side drawer, open while
  `ui.parts`; the app bar's "Parts & OTS" toggles it). Not built yet; see app/CONTRIBUTING.md.
  State: app.state.keyboardParts (on, sounding, selected, volume, waiting, program,
  voiceName, playsBass, octave) and app.state.ots (settings, applied, link).
  Commands: togglePart / setPartOn, selectPart, setPartVoice / stepVoice, setPartVolume,
  setPartOctave, recallOts, toggleOtsLink / setOtsLink.
  Tooltips ready in the catalog: part.*.on, part.*.select, part.voice_*, part.octave_*,
  ots.1–4, ots.link, mixer.panel.*, drawer.close.
-->
<script lang="ts">
  import { ui } from '../../lib/store.svelte'
  import Overlay from '../../lib/ui/Overlay.svelte'
  import PanelSlot from '../../lib/ui/PanelSlot.svelte'
</script>

<Overlay id="parts" title="Keyboard parts and OTS" closeTip="drawer.close" onclose={() => (ui.parts = false)}>
  <PanelSlot
    items={[
      'Right 1, Right 2, Right 3, Left: voice (with a GM voice picker), on/off, volume, octave',
      'Which part Voice −/+ edits; Left playing the bass under Manual Bass',
      'OTS 1–4 with what each sets (voices, on/off, volume, octave), the last recalled lit, and OTS Link',
    ]}
  />
</Overlay>
