<!--
  Library › Style map (docs/racks.md, "Deviations from the wireframe"): the program map that
  makes every style play your sounds, moved here from the Sound Library drawer, which
  Library replaces. Two pages, as in the drawer: the GM map, and Add from SoundFont
  (browse a font's presets, audition, add). The library file (import, export) sits under
  them. Both pages stay mounted (the inactive one `hidden`), as they did in the drawer.
-->
<script lang="ts">
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import AddPage from '../sound/AddPage.svelte'
  import GmMapPage from '../sound/GmMapPage.svelte'
  import LibraryFile from '../sound/LibraryFile.svelte'
  import { libraryNav } from './nav.svelte'

  const sl = $derived(app.state.soundLibrary)
  const auto = $derived(sl.gmMap.filter((r) => r.resolved.layer === 'auto').length)
</script>

<div class="map">
  <div class="col">
    <div class="pages mat-well" role="tablist" aria-label="Style map pages">
      <button type="button" role="tab" id="lib-map-gm" class="page-tab" class:on={libraryNav.mapPage === 'gm'} aria-selected={libraryNav.mapPage === 'gm'} aria-controls="lib-map-page-gm" use:tip={'sound.tab_gm'} onclick={() => (libraryNav.mapPage = 'gm')}>
        GM map{#if auto}<small>{auto}</small>{/if}
      </button>
      <button type="button" role="tab" id="lib-map-add" class="page-tab" class:on={libraryNav.mapPage === 'add'} aria-selected={libraryNav.mapPage === 'add'} aria-controls="lib-map-page-add" use:tip={'sound.tab_add'} onclick={() => (libraryNav.mapPage = 'add')}>Add from SoundFont</button>
    </div>

    {#if sl.auditioning}
      <p class="aud engraved" role="status">Auditioning {sl.auditioning === 'preset' ? 'a preset' : (sl.patches.find((p) => p.id === sl.auditioning)?.name ?? '')}…</p>
    {/if}
    {#if sl.extraSoundFonts.length}
      <p class="fonts engraved">Also loaded: {sl.extraSoundFonts.map((f) => f.replace(/\.sf2$/i, '')).join(', ')}</p>
    {/if}

    <div class="page" id="lib-map-page-gm" role="tabpanel" aria-labelledby="lib-map-gm" hidden={libraryNav.mapPage !== 'gm'}><GmMapPage /></div>
    <div class="page" id="lib-map-page-add" role="tabpanel" aria-labelledby="lib-map-add" hidden={libraryNav.mapPage !== 'add'}><AddPage /></div>
    <LibraryFile />
  </div>
</div>

<style>
  .map {
    height: 100%;
    min-height: 0;
    overflow: auto;
  }
  /* The pages were drawn for a 30rem drawer: a readable column, not the page's width. */
  .col {
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
    max-width: 44rem;
    padding-bottom: 0.6rem;
  }
  .pages {
    display: grid;
    grid-template-columns: 1fr 1.5fr;
    gap: 2px;
    padding: 3px;
    border-radius: 6px;
    max-width: 26rem;
  }
  .page-tab {
    min-height: 2.2rem;
    border: 1px solid transparent;
    border-radius: 4px;
    background: transparent;
    color: var(--screen-dim);
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.86rem;
    white-space: nowrap;
  }
  .page-tab small {
    margin-left: 0.3em;
    font-size: 0.75em;
    opacity: 0.7;
  }
  .page-tab.on {
    background: linear-gradient(180deg, var(--raised-hi), var(--raised-lo));
    color: var(--ink);
    box-shadow:
      inset 0 1px 0 rgb(255 255 255 / 0.18),
      inset 0 -2px 0 var(--accent);
  }
  .aud,
  .fonts {
    margin: 0;
  }
  .aud {
    color: var(--accent);
  }
  .page {
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
  }
  .page[hidden] {
    display: none;
  }
</style>
