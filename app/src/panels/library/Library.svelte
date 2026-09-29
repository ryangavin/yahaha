<!--
  Library (docs/racks.md, "Screens"; the wireframe's `libraryPage`): the one browser, a
  top-level page in place of the stage (`ui.view === 'library'`). Stage | Library in the
  header, Alt+B, a part's sound name or the fader head's Library button open it; Esc or
  Back to Stage returns. Drawers (Mixer, Effects, …) open over it, and performance keys
  keep working outside its search field.

  ┌ LIBRARY [Racks][Sounds][Instruments][Style map]   Loads into [R1][R2][R3][L] ┐┌ RACK ────┐
  │ the tab (SoundsTab, InstrumentsTab, RacksTab, MapTab)                        ││ docked   │
  │ Right 1 plays Sampler Deluxe · Silk Strings  edited  Save  Save as…  [Back]  ││ panel    │
  └──────────────────────────────────────────────────────────────────────────────┘└──────────┘
  [ Quick Racks bar ]

  The target part ("Loads into") is where Sounds and Instruments load: it follows where
  Library was opened from (the selected part, or the part whose sound name was clicked).
-->
<script lang="ts">
  import { tick } from 'svelte'
  import { app, ui, type LibraryTab } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import RackPanel from '../rack/RackPanel.svelte'
  import { pluginStatusLine } from '../parts/parts'
  import QuickBar from '../quickracks/QuickBar.svelte'
  import { nowPlaying, presetFileName } from '../sounds/model'
  import { CATEGORY_LABELS } from '../../lib/api/sound-library'
  import type { PatchCategory } from '../../lib/api/types'
  import InstrumentsTab from './InstrumentsTab.svelte'
  import MapTab from './MapTab.svelte'
  import RacksTab from './RacksTab.svelte'
  import SoundsTab from './SoundsTab.svelte'

  const TABS: { id: LibraryTab; label: string; tip: 'library.tab_racks' | 'library.tab_sounds' | 'library.tab_instruments' | 'library.tab_map' }[] = [
    { id: 'racks', label: 'Racks', tip: 'library.tab_racks' },
    { id: 'sounds', label: 'Sounds', tip: 'library.tab_sounds' },
    { id: 'instruments', label: 'Instruments', tip: 'library.tab_instruments' },
    { id: 'map', label: 'Style map', tip: 'library.tab_map' },
  ]
  const SHORT = ['R1', 'R2', 'R3', 'L']

  const tab = $derived(ui.libraryTab)
  const part = $derived(ui.libraryPart)
  const parts = $derived(app.state.keyboardParts)
  const kp = $derived(parts[part])
  const sl = $derived(app.state.soundLibrary)
  const ctx = $derived({ patches: sl.patches, gmMap: sl.gmMap ?? [] })
  const plugins = $derived(app.state.plugins)
  const targets = $derived(tab === 'sounds' || tab === 'instruments')
  let tabStrip: HTMLDivElement | undefined = $state()

  function tabKey(e: KeyboardEvent) {
    const i = TABS.findIndex((t) => t.id === tab)
    const to: Record<string, number> = { ArrowRight: i + 1, ArrowLeft: i - 1, Home: 0, End: TABS.length - 1 }
    if (!(e.key in to)) return
    e.preventDefault()
    e.stopPropagation()
    const next = TABS[(to[e.key] + TABS.length) % TABS.length]
    ui.libraryTab = next.id
    void tick().then(() => tabStrip?.querySelector<HTMLElement>(`#library-tab-${next.id}`)?.focus())
  }

  // Save as… a new sound from what the target part plays (Save keeps it over its own sound).
  // A plugin part can also keep it as an .aupreset that Logic reads, asking before it
  // replaces a user preset of that name (#307, #367).
  let saveForm = $state<{ name: string; aupreset: boolean; category: PatchCategory; replace: boolean } | null>(null)
  let saveInput: HTMLInputElement | undefined = $state()
  const canPreset = $derived(kp?.plugin?.status === 'playing')
  const clash = $derived.by(() => {
    if (!saveForm?.aupreset || !kp?.plugin) return false
    const parent = `au:${kp.plugin.id}`
    const n = presetFileName(saveForm.name).toLowerCase()
    return app.sounds.entries.some((e) => e.parent === parent && e.id.startsWith(`${parent}#u:`) && e.name.toLowerCase() === n)
  })
  function openSaveAs() {
    const cur = kp?.sound ? sl.patches.find((p) => p.id === kp.sound!.id) : undefined
    saveForm = { name: kp?.sound?.name ?? kp?.voiceName ?? '', aupreset: false, category: cur?.category ?? 'synthLead', replace: false }
    void tick().then(() => (saveInput?.focus(), saveInput?.select()))
  }
  function saveAs(overwrite = false) {
    const name = saveForm?.name.trim()
    if (!saveForm || !name) return
    if (saveForm.aupreset && canPreset) {
      if (clash && !overwrite) {
        saveForm.replace = true
        return
      }
      app.send({ type: 'savePartAsPluginPreset', part, name, category: saveForm.category, overwrite })
    }
    app.send({ type: 'saveSoundAs', part, name })
    saveForm = null
  }
  function saveKey(e: KeyboardEvent) {
    if (e.key !== 'Escape') return
    e.preventDefault()
    e.stopPropagation()
    saveForm = null
  }
  // Another target part closes the form.
  $effect(() => {
    void part
    saveForm = null
  })
</script>

<div class="library">
  <section class="browse mat-chassis" aria-label="Library">
    <div class="head">
      <h2 class="engraved">Library</h2>
      <div class="tabs" role="tablist" aria-label="Library" tabindex="-1" bind:this={tabStrip} onkeydown={tabKey}>
        {#each TABS as t (t.id)}
          <button type="button" role="tab" id="library-tab-{t.id}" class="tab" class:on={tab === t.id} aria-selected={tab === t.id} aria-controls="library-page" tabindex={tab === t.id ? 0 : -1} use:tip={t.tip} onclick={() => (ui.libraryTab = t.id)}>{t.label}</button>
        {/each}
      </div>
      {#if targets}
        <div class="target" role="group" aria-label="Loads into">
          <span class="engraved">Loads into</span>
          {#each parts as p, i (i)}
            <button type="button" class:on={part === i} aria-pressed={part === i} aria-label="Load into {p.name}" use:tip={'library.target'} onclick={() => (ui.libraryPart = i)}>{SHORT[i]}</button>
          {/each}
        </div>
      {:else if tab === 'racks'}
        <span class="hint engraved">loads the whole rack</span>
      {:else}
        <span class="hint engraved">what styles play for each GM voice</span>
      {/if}
    </div>

    <div class="page" id="library-page" role="tabpanel" aria-labelledby="library-tab-{tab}">
      {#if tab === 'sounds'}<SoundsTab />
      {:else if tab === 'instruments'}<InstrumentsTab />
      {:else if tab === 'racks'}<RacksTab />
      {:else}<MapTab />{/if}
    </div>

    <footer class="foot">
      {#if targets && kp}
        {#if saveForm}
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions (Esc in the form closes it, not Library) -->
          <form class="saveform" aria-label="Save as a new sound" onsubmit={(ev) => (ev.preventDefault(), saveAs())} onkeydown={saveKey}>
            <span class="engraved">Save as</span>
            <input bind:this={saveInput} value={saveForm.name} oninput={(ev) => saveForm && ((saveForm.name = ev.currentTarget.value), (saveForm.replace = false))} class="mat-well" type="text" placeholder="Sound name" aria-label="New sound's name" spellcheck="false" use:tip={'sounds.save_as_name'} />
            {#if canPreset}
              <label class="check"><input type="checkbox" checked={saveForm.aupreset} onchange={(ev) => saveForm && ((saveForm.aupreset = ev.currentTarget.checked), (saveForm.replace = false))} use:tip={'sounds.save_preset'} /> .aupreset</label>
              {#if saveForm.aupreset}
                <select aria-label="Category of the preset" value={saveForm.category} onchange={(ev) => saveForm && (saveForm.category = ev.currentTarget.value as PatchCategory)} use:tip={'sounds.preset_category'}>
                  {#each Object.entries(CATEGORY_LABELS) as [id, label] (id)}<option value={id}>{label}</option>{/each}
                </select>
              {/if}
            {/if}
            {#if saveForm.replace && clash}
              <span class="ask" role="alert">Replace ‘{presetFileName(saveForm.name)}’?</span>
              <HwButton tip="sounds.preset_replace" onclick={() => saveAs(true)}>Replace</HwButton>
              <HwButton tip="sounds.preset_replace_cancel" onclick={() => saveForm && ((saveForm.replace = false), saveInput?.focus())}>Cancel</HwButton>
            {:else}
              <HwButton tip="sounds.save_as_confirm" onclick={() => saveAs()}>Save</HwButton>
              <HwButton tip="sounds.preset_cancel" onclick={() => (saveForm = null)}>Cancel</HwButton>
            {/if}
          </form>
        {:else}
          <span class="now">{kp.name} plays <b>{nowPlaying(kp, ctx, plugins.list, app.state.io.soundFontFile)}</b>{#if kp.plugin?.missing}<span class="warn">&nbsp;· ⚠ plugin missing: silent</span>{:else if kp.plugin && kp.plugin.status !== 'playing'}&nbsp;· {pluginStatusLine(kp.plugin, plugins.available).replace(/ ▾$/, '')}{/if}</span>
          {#if kp.soundEdited}<span class="edited" use:tip={'sounds.edited'}>edited</span>{/if}
          <HwButton tip="sounds.save_over" onclick={() => app.send({ type: 'saveSound', part })}>Save</HwButton>
          <HwButton tip="sounds.save" onclick={openSaveAs}>Save as…</HwButton>
        {/if}
      {:else}
        <span class="now">Loaded: <b>{app.state.liveRack.name}</b>{app.state.liveRack.modified ? ' ●' : ''}</span>
      {/if}
      <HwButton tip="library.back" onclick={() => (ui.view = 'stage')}>Back to Stage</HwButton>
    </footer>
  </section>

  <!-- The Rack panel, docked (docs/racks.md "Screens"). While its Stage drawer is open
       over Library the drawer shows it instead. -->
  <div class="dock-slot">
    {#if ui.rack}
      <aside class="dock mat-chassis" aria-label="Rack"><p class="hint">The Rack is open in its drawer.</p></aside>
    {:else}
      <RackPanel docked />
    {/if}
  </div>

  <!-- The Quick Racks bar (docs/racks.md, item 6), as on the stage. -->
  <div class="strip mat-chassis"><QuickBar /></div>
</div>

<style>
  .library {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(20rem, 27rem);
    grid-template-rows: minmax(0, 1fr) auto;
    gap: 0.6rem;
    font-size: 15px;
  }
  .browse,
  .dock {
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-radius: var(--r-panel);
    padding: 0.6rem 0.8rem;
    gap: 0.6rem;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 0.9rem;
    flex-wrap: wrap;
  }
  h2 {
    margin: 0;
    font-size: 0.8rem;
  }
  .tabs {
    display: flex;
    gap: 0.3rem;
    outline: none;
  }
  .tab {
    min-height: 2.1rem;
    padding: 0 0.9rem;
    border: 1px solid transparent;
    border-radius: 5px;
    background: none;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.95rem;
    color: var(--ink);
  }
  .tab:hover {
    background: rgb(127 127 127 / 0.1);
  }
  .tab.on {
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
    background: color-mix(in srgb, var(--accent) 16%, transparent);
  }
  .target {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }
  .target .engraved {
    margin-right: 0.2rem;
  }
  .target button {
    min-width: 2.3rem;
    min-height: 1.9rem;
    border: 1px solid var(--line-strong);
    border-radius: 4px;
    background: none;
    color: var(--ink);
    font-family: var(--font-display);
    font-weight: 600;
  }
  .target button.on {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--accent-ink);
  }
  .hint {
    margin-left: auto;
    color: var(--muted);
  }
  .page {
    flex: 1;
    min-height: 0;
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    min-height: 2.6rem;
    border-top: 1px solid var(--line);
    padding-top: 0.5rem;
  }
  .now {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-small);
    color: var(--muted);
  }
  .now b {
    color: var(--ink);
  }
  .warn {
    color: var(--danger);
  }
  .edited {
    padding: 1px 6px;
    border: 1px solid color-mix(in srgb, var(--accent) 70%, transparent);
    border-radius: 3px;
    color: var(--accent);
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.72rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .saveform {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
  }
  .saveform input[type='text'] {
    flex: 1;
    min-width: 6rem;
    height: 2.2rem;
    padding: 0 0.6rem;
    border: 1px solid var(--well-edge);
    border-radius: 5px;
    color: var(--screen-ink);
  }
  .check {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    white-space: nowrap;
    font-size: var(--fs-small);
  }
  .ask {
    color: var(--accent);
    white-space: nowrap;
  }
  .saveform select {
    min-height: 2.2rem;
    padding: 0 0.45rem;
    border: 1px solid var(--well-edge);
    border-radius: 4px;
    background: var(--screen-bg);
    color: var(--screen-ink);
  }
  .dock-slot {
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .dock-slot > :global(*) {
    flex: 1;
  }
  .strip {
    grid-column: 1 / -1;
    border-radius: var(--r-panel);
    padding: 0.5em 0.9em;
    /* The bar is sized in em for the stage (93em wide): fit it to the page. */
    font-size: min(14px, calc((100vw - 64px) / 96));
  }
  @media (max-width: 1100px) {
    .library {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: minmax(0, 1fr) auto;
    }
    .dock-slot {
      display: none;
    }
  }
</style>
