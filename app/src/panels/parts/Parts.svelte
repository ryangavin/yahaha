<!--
  Keyboard parts + One Touch Settings (right-side drawer, open while `ui.parts`). The
  content is PartsPanel.svelte, which Library also docks on its right.
-->
<script lang="ts">
  import { ui } from '../../lib/store.svelte'
  import Overlay from '../../lib/ui/Overlay.svelte'
  import PartsPanel from './PartsPanel.svelte'
</script>

<Overlay id="parts" title="Keyboard parts and OTS" closeTip="drawer.close" onclose={() => (ui.parts = false)}>
  <PartsPanel />
</Overlay>
