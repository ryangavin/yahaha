<!--
  PANEL SLOT: Mixer. Not built yet; see app/CONTRIBUTING.md.
  State: app.state.mixer (faderPage, styleParts, master, masterWaiting) and
  app.state.keyboardParts (the Panel page's volumes and `waiting`).
  Use lib/ui/Fader (it has the soft-takeover mark) and lib/ui/Tabs for Panel/Style.
  Tooltips ready in the catalog: mixer.page, mixer.panel.*, mixer.style.volume,
  mixer.style.mute, mixer.master, mixer.pickup.
-->
<script lang="ts">
  import Panel from '../../lib/ui/Panel.svelte'
  import PanelSlot from '../../lib/ui/PanelSlot.svelte'
</script>

<Panel id="mixer" title="Mixer">
  <PanelSlot
    items={[
      'Panel / Style tabs, mirroring the Launchkey fader pages (F9)',
      'Panel: Right 1–3 and Left volumes, on/off under each',
      'Style: the eight band parts, volume and mute (z … ,)',
      'Master fader; ↕ on a fader waiting for soft takeover',
    ]}
  />
</Panel>
