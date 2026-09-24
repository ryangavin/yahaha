<!--
  PANEL SLOT: Launchkey. Not built yet; see app/CONTRIBUTING.md.
  State: app.state.pads (page, pageName, the 16 `pads` of the current page, connected)
  and app.state.io.unmapped. Each Pad has its label, LED look and `action`: send it to
  press the pad; `tipFor(pad.action)` (help/actions.ts) is its tooltip; draw it with
  lib/ui/LampKey or `brightness()`/`css()` from lib/leds.ts on `clock.beats`.
  Tooltips ready in the catalog: padpage.*, launchkey.shift, launchkey.unused, plus each
  pad's own entry.
-->
<script lang="ts">
  import Panel from '../../lib/ui/Panel.svelte'
  import PanelSlot from '../../lib/ui/PanelSlot.svelte'
</script>

<Panel id="launchkey" title="Launchkey">
  <PanelSlot
    items={[
      'Pad pages 1–3 (Sections, Chord/Setup, OTS/Parts) and Pad Bank ▲/▼',
      'The 16 pads of the current page, lit like the hardware LEDs; click to press',
      'Buttons: Play, Stop, < Track / Track >, Scene Launch, Function, Shift',
      'The last unmapped note or CC',
    ]}
  />
</Panel>
