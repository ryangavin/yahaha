<!--
  DRAWER SLOT: Settings (right-side drawer, open while `ui.settings`). Not built yet; see
  app/CONTRIBUTING.md.
  State: app.state.chord (fingering, upper, manualBass, split) and app.state.io
  (synth: device, outputPair, channels, soundFont, muted; inputs).
  Commands: setFingering, setUpper, setManualBass, setSplit, setAudioOutput,
  setSynthMuted. Tooltips ready in the catalog: fingering.select, detection.upper,
  detection.manual_bass, split.*, audio.output, audio.soundfont, audio.synth_mute,
  midi.input, settings.close.
-->
<script lang="ts">
  import { ui } from '../../lib/store.svelte'
  import Overlay from '../../lib/ui/Overlay.svelte'
  import PanelSlot from '../../lib/ui/PanelSlot.svelte'
</script>

<Overlay id="settings" title="Settings" closeTip="settings.close" onclose={() => (ui.settings = false)}>
  <PanelSlot
    items={[
      'Fingering type; chord detection Upper/Lower and Manual Bass; split point',
      'Audio output pair, SoundFont, synth mute',
      'MIDI inputs',
    ]}
  />
</Overlay>
