<!--
  PANEL SLOT: Style browser (overlay, open while `ui.browser`). Not built yet; see
  app/CONTRIBUTING.md.
  Data: `app.library.entries` (LibraryEntry[] in folder-then-name order, kept fresh by
  the store); `app.state.library.position` is the loaded style. Load with
  `app.send({ type: 'loadStyle', id })`, then `ui.browser = false`.
  While it's open the shell stops sending performance keys (typed keys go to the filter);
  Esc closes it. Tooltips ready in the catalog: browser.filter, browser.folder,
  browser.row, browser.close.
-->
<script lang="ts">
  import { ui } from '../../lib/store.svelte'
  import Overlay from '../../lib/ui/Overlay.svelte'
  import PanelSlot from '../../lib/ui/PanelSlot.svelte'
</script>

<Overlay id="browser" title="Styles" closeTip="browser.close" onclose={() => (ui.browser = false)}>
  <PanelSlot
    items={[
      'Folders (categories) and filter-as-you-type on name or folder',
      'Rows: name, folder, tempo, time signature, sections; ▶ on the loaded style',
      'Enter or click loads, even while the band plays; unreadable files in red',
    ]}
  />
</Overlay>
