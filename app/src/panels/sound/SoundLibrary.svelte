<!--
  The Sound Library drawer (right side, not modal: performance keys keep working), #103.
  Your short list of patches, and the program map that makes every style play them:

  1. Patches: the list by category, search, favourites, audition, the editor, save a
     part's sound, the library file.
  2. Program Map: the drum rule, the 16 GM families, program overrides; every style's map
     or this style's own.
  3. This style: what the current style sends each part, and what it plays; remap here.
  4. Add from SoundFont: browse a SoundFont's presets, audition, add.

  All pages stay mounted (inactive ones `hidden`), so switching is instant and the
  tooltip coverage test sees every control. docs/sound-library.md has the behaviour.
-->
<script lang="ts">
  import { app, ui } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import Overlay from '../../lib/ui/Overlay.svelte'
  import AddPage from './AddPage.svelte'
  import MapPage from './MapPage.svelte'
  import PatchesPage from './PatchesPage.svelte'
  import StyleUsePage from './StyleUsePage.svelte'
  import { nav, TABS, type SoundTab } from './nav.svelte'

  const sl = $derived(app.state.soundLibrary)
  let strip: HTMLDivElement | undefined = $state()

  function tabKey(e: KeyboardEvent) {
    const i = TABS.findIndex((t) => t.id === nav.tab)
    const to: Record<string, number> = { ArrowRight: i + 1, ArrowLeft: i - 1, Home: 0, End: TABS.length - 1 }
    if (!(e.key in to)) return
    e.preventDefault()
    e.stopPropagation()
    const next = TABS[(to[e.key] + TABS.length) % TABS.length]
    nav.tab = next.id
    strip?.querySelector<HTMLElement>(`#sound-tab-${next.id}`)?.focus()
  }
  const count = (id: SoundTab) => (id === 'patches' ? sl.patches.length : id === 'style' ? sl.usage.length : null)
</script>

<Overlay id="sound" title="Sound Library" closeTip="drawer.close" onclose={() => (ui.sound = false)}>
  <div class="sound">
    <div class="tabs mat-well" role="tablist" aria-label="Sound Library pages" tabindex="-1" bind:this={strip} onkeydown={tabKey}>
      {#each TABS as t (t.id)}
        <button
          type="button"
          role="tab"
          id="sound-tab-{t.id}"
          class="tab"
          class:on={nav.tab === t.id}
          aria-selected={nav.tab === t.id}
          aria-controls="sound-page-{t.id}"
          tabindex={nav.tab === t.id ? 0 : -1}
          use:tip={t.tip}
          onclick={() => (nav.tab = t.id)}
        >
          {t.label}{#if count(t.id) !== null}<small>{count(t.id)}</small>{/if}
        </button>
      {/each}
    </div>

    {#if sl.auditioning}
      <p class="aud engraved" role="status">Auditioning {sl.auditioning === 'preset' ? 'a preset' : (sl.patches.find((p) => p.id === sl.auditioning)?.name ?? '')}…</p>
    {/if}
    {#if sl.extraSoundFonts.length}
      <p class="fonts engraved">Also loaded: {sl.extraSoundFonts.map((f) => f.replace(/\.sf2$/i, '')).join(', ')}</p>
    {/if}

    {#snippet page(id: SoundTab, Body: typeof PatchesPage)}
      <div class="page" id="sound-page-{id}" role="tabpanel" aria-labelledby="sound-tab-{id}" hidden={nav.tab !== id}>
        <Body />
      </div>
    {/snippet}
    {@render page('patches', PatchesPage)}
    {@render page('map', MapPage)}
    {@render page('style', StyleUsePage)}
    {@render page('add', AddPage)}
  </div>
</Overlay>

<style>
  .sound {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .tabs {
    display: grid;
    grid-template-columns: 0.8fr 1fr 0.9fr 1.5fr;
    gap: 2px;
    padding: 3px;
    border-radius: 6px;
    outline: none;
  }
  .tab {
    min-height: 2.2rem;
    padding: 0 0.2rem;
    border: 1px solid transparent;
    border-radius: 4px;
    background: transparent;
    color: var(--screen-dim);
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.86rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tab small {
    margin-left: 0.3em;
    font-size: 0.75em;
    opacity: 0.7;
  }
  .tab:not(.on):hover {
    color: var(--screen-ink);
    background: rgb(255 255 255 / 0.04);
  }
  .tab.on {
    background: linear-gradient(180deg, var(--raised-hi), var(--raised-lo));
    color: var(--ink);
    box-shadow:
      inset 0 1px 0 rgb(255 255 255 / 0.18),
      inset 0 -2px 0 var(--accent),
      0 1px 0 rgb(0 0 0 / 0.45);
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
