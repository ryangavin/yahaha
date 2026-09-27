<!--
  The library file, under the Sound Library drawer's pages: where it is saved, whether the
  MIDI port sends mapped programs, export and import. (The Patches tab it sat under folded
  into the Sound Browser's Sounds tab.) Commands: setPortSendsMapped, exportSoundLibrary,
  importSoundLibrary.
-->
<script lang="ts">
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'

  const sl = $derived(app.state.soundLibrary)
  let importPath = $state('')
</script>

<section class="block" aria-labelledby="sl-file">
  <h3 id="sl-file" class="engraved">Library file</h3>
  <p class="explain">{sl.file ? `Saved to ${sl.file} after every change.` : 'This session saves nowhere (offline).'} Your sounds are in the Sound Browser's My Sounds.</p>
  <Toggle on={sl.portSendsMapped} tip="sound.port_mapped" onclick={() => app.send({ type: 'setPortSendsMapped', on: !sl.portSendsMapped })}>MIDI port sends mapped programs</Toggle>
  <div class="line">
    <HwButton tip="sound.export" onclick={() => app.send({ type: 'exportSoundLibrary', path: null })}>Export</HwButton>
    <input aria-label="Library file to import" placeholder="/path/to/sound-library.json" bind:value={importPath} use:tip={'sound.import_path'} />
    <HwButton tip="sound.import" onclick={() => importPath.trim() && app.send({ type: 'importSoundLibrary', path: importPath.trim(), replace: false, maps: true })}>Import</HwButton>
  </div>
</section>

<style>
  .block {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  input {
    flex: 1;
    min-width: 0;
    min-height: 2.2rem;
    padding: 0 0.45rem;
    border-radius: 4px;
    border: 1px solid var(--well-edge);
    background: var(--screen-bg);
    color: var(--screen-ink);
    font: inherit;
    font-size: 0.9rem;
  }
  .explain {
    margin: 0;
    font-size: var(--fs-small);
    color: var(--muted);
    line-height: 1.35;
  }
  h3 {
    margin: 0;
    font-size: 0.78rem;
  }
</style>
