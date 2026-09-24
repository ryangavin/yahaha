<!--
  DRAWER SLOT: Mixer detail (right-side drawer, open while `ui.mixer`). Not built yet; see
  app/CONTRIBUTING.md. The Launchkey mirror already shows the faders of the active page;
  this drawer shows both pages at once, with each band part's voice.
  State: app.state.mixer (faderPage, styleParts with voice/waiting/mutedByManualBass,
  master, masterWaiting) and app.state.keyboardParts. Use lib/ui/Fader and HwButton.
  Tooltips ready in the catalog: mixer.page, mixer.panel.*, mixer.style.volume,
  mixer.style.mute, mixer.master, mixer.pickup, drawer.close.
-->
<script lang="ts">
  import { ui } from '../../lib/store.svelte'
  import Overlay from '../../lib/ui/Overlay.svelte'
  import PanelSlot from '../../lib/ui/PanelSlot.svelte'
</script>

<Overlay id="mixer" title="Mixer" closeTip="drawer.close" onclose={() => (ui.mixer = false)}>
  <PanelSlot
    items={[
      'Panel: Right 1–3 and Left, volume and on/off',
      'Style: the eight band parts with the voice each was written for, volume and mute (z … ,)',
      'Master; ↕ on any level waiting for its Launchkey fader',
    ]}
  />
</Overlay>
