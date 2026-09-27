<!--
  The Sound Browser's Instruments tab (docs/sound-browser.md, O2): the SoundFonts and
  plugins found in the scanned folders. Housekeeping lives here, not in the Sounds footer.

  ┌ SOUNDFONTS ───────────────────────────────────────────────────────────┐
  │ ▸ GeneralUser-GS     SF  260 presets · 12 kits · GM 128/128 + kit    │
  │ PLUGINS                                                    [Rescan]   │
  │ ▾ Sampler Deluxe     AU  Fake Instruments · AUv2 · 5 presets · Right 1 playing · CPU 4% │
  │    [New sound from Sampler Deluxe] [Edit…] Category [Piano ▾] [In process]           │
  │    Upright Piano   Pianos                     [Play now] [Add to my sounds]           │
  └───────────────────────────────────────────────────────────────────────┘

  A font row shows only its counts and GM completeness. A plugin shows its scan and load
  status and CPU; open, its category, in-process override, Edit… and New sound. Each
  preset has Play now (the part plays it) and Add to my sounds (the library keeps it).
  Every control is a real button, so Tab reaches everything; ▸/▾ opens with Enter or Space.
-->
<script lang="ts">
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import { CATEGORY_LABELS } from '../../lib/api/sound-library'
  import type { PatchCategory } from '../../lib/api/types'
  import { browserNav } from './nav.svelte'
  import { fontLine, inMySounds, instrumentPresets, instruments, pluginLoads, presetPlace, scanLine, type Instrument } from './instruments'

  let { part = 0 }: { part?: number } = $props()

  const catalog = $derived(app.sounds)
  const plugins = $derived(app.state.plugins)
  const list = $derived(instruments(catalog, plugins.list))
  const fonts = $derived(list.filter((i) => i.kind === 'font'))
  const pluginRows = $derived(list.filter((i) => i.kind === 'plugin'))
  const listing = $derived(new Set(app.state.sounds?.listingPresets ?? []))
  const patches = $derived(app.state.soundLibrary.patches)
  const kp = $derived(app.state.keyboardParts[part])

  function toggle(inst: Instrument) {
    if (browserNav.open.has(inst.id)) return void browserNav.open.delete(inst.id)
    browserNav.open.add(inst.id)
    // Factory presets need a plugin instance: listed once, then cached (the engine does
    // nothing for a plugin already listed, and its .aupreset files alone don't say so).
    if (inst.kind === 'plugin' && !inst.plugin.lastError && !listing.has(inst.id)) app.send({ type: 'listPluginPresets', id: inst.id })
  }

  const play = (id: string) => app.send({ type: 'assignSound', part, id })
  const add = (id: string) => app.send({ type: 'addToMySounds', id })

  // New sound from <plugin>: the part loads the plugin's default state, and its editor
  // opens once it plays. Save as… (Sounds) keeps it.
  let pendingEditor = $state<string | null>(null)
  function newSound(id: string) {
    app.send({ type: 'setPartPlugin', part, id, state: null })
    pendingEditor = id
  }
  $effect(() => {
    const p = kp?.plugin
    if (!pendingEditor || !p || p.id !== pendingEditor) return
    if (p.status === 'playing') {
      if (p.editor) app.pluginEditor(part, true)
      pendingEditor = null
    } else if (p.status !== 'loading') pendingEditor = null
  })

  const setCategory = (id: string, category: PatchCategory) => app.send({ type: 'setSoundCategory', id, category })
</script>

<div class="inst">
  <div class="screen mat-screen">
    <div class="scroller">
      <h3 class="engraved">SoundFonts</h3>
      {#if fonts.length === 0}<p class="empty">No SoundFonts in the soundfonts folder.</p>{/if}
      {#each fonts as inst (inst.id)}
        {@const open = browserNav.open.has(inst.id)}
        <section class="card" class:open>
          <div class="head">
            <button type="button" class="tw" aria-expanded={open} aria-controls="presets-{inst.id}" use:tip={'instruments.expand'} onclick={() => toggle(inst)}>
              <span aria-hidden="true">{open ? '▾' : '▸'}</span> {inst.name}
            </button>
            <span class="badge">SF</span>
            {#if inst.kind === 'font'}<span class="meta">{fontLine(inst.font)}</span>{/if}
          </div>
          {#if open}{@render presets(inst)}{/if}
        </section>
      {/each}

      <div class="plughead">
        <h3 class="engraved">Plugins</h3>
        {#if plugins.available}
          <span class="scan">{plugins.scanning ? 'Scanning…' : `${pluginRows.length} found`}</span>
          <HwButton tip="part.plugin_rescan" onclick={() => !plugins.scanning && app.send({ type: 'rescanPlugins' })}>{plugins.scanning ? 'Scanning' : 'Rescan'}</HwButton>
        {/if}
      </div>
      {#if !plugins.available}<p class="empty">Plugins need the built-in synth.</p>
      {:else if pluginRows.length === 0}<p class="empty">No instrument plugins found.</p>{/if}
      {#each pluginRows as inst (inst.id)}
        {#if inst.kind === 'plugin'}
          {@const open = browserNav.open.has(inst.id)}
          {@const p = inst.plugin}
          {@const loads = pluginLoads(app.state.keyboardParts, p.id)}
          {@const here = loads.find((l) => l.part === part)}
          <section class="card" class:open>
            <div class="head">
              <button type="button" class="tw" aria-expanded={open} aria-controls="presets-{inst.id}" use:tip={'instruments.expand'} onclick={() => toggle(inst)}>
                <span aria-hidden="true">{open ? '▾' : '▸'}</span> {inst.name}
              </button>
              <span class="badge au">AU</span>
              <span class="meta">
                {p.manufacturer} · {p.format} · <span class:warn={!!p.lastError}>{scanLine(p, inst.entry, listing.has(inst.id))}</span>
                {#each loads as l (l.part)}&nbsp;· {l.name} {l.status === 'playing' ? `playing · CPU ${Math.round(l.cpu * 100)}%` : l.status === 'failed' ? `failed: ${l.error ?? 'did not load'}` : l.status}{/each}
              </span>
            </div>
            {#if open}
              <div class="house">
                <HwButton tip="instruments.new_sound" onclick={() => newSound(p.id)}>New sound from {p.name}</HwButton>
                {#if here?.editor && here.status === 'playing'}<HwButton tip="part.plugin_edit" onclick={() => app.pluginEditor(part, true)}>Edit…</HwButton>{/if}
                <label class="field">
                  <span class="engraved">Category</span>
                  <select aria-label="Category of {p.name}" value={inst.entry?.category ?? 'synthLead'} use:tip={'sounds.set_category'} onchange={(ev) => setCategory(inst.id, ev.currentTarget.value as PatchCategory)}>
                    {#each Object.entries(CATEGORY_LABELS) as [id, label] (id)}<option value={id}>{label}</option>{/each}
                  </select>
                </label>
                {#if p.canRunInProcess}
                  <label class="field">
                    <input type="checkbox" checked={p.inProcess} use:tip={'part.plugin_in_process'} onchange={() => app.send({ type: 'setPluginInProcess', id: p.id, inProcess: !p.inProcess })} />
                    <span>In process</span>
                  </label>
                {/if}
              </div>
              {@render presets(inst)}
            {/if}
          </section>
        {/if}
      {/each}
    </div>
  </div>
  <p class="foot">{kp?.name} plays <b>{kp?.voiceName}</b>. Play now plays a preset on {kp?.name}; Add to my sounds keeps it in Sounds.</p>
</div>

{#snippet presets(inst: Instrument)}
  {@const rows = instrumentPresets(catalog, inst)}
  <ul class="presets" id="presets-{inst.id}" aria-label="{inst.name} presets">
    {#if rows.length === 0}
      <li class="none">{inst.kind === 'plugin' && (listing.has(inst.id) || inst.entry?.plugin?.presets == null) && !inst.plugin.lastError ? 'Listing presets…' : 'No presets.'}</li>
    {/if}
    {#each rows as i (catalog.entries[i].id)}
      {@const e = catalog.entries[i]}
      {@const mine = inMySounds(patches, e.id)}
      <li class="preset">
        <span class="pname">{e.name}</span>
        <span class="where">{inst.kind === 'font' ? presetPlace(e.id) : e.detail.includes(' · ') ? e.detail.split(' · ').slice(1).join(' · ') : e.id.includes('#f:') ? 'Factory' : ''}</span>
        <button type="button" class="act" use:tip={'instruments.play'} aria-label="Play {e.name} now" onclick={() => play(e.id)}>Play now</button>
        <button type="button" class="act" class:done={mine} use:tip={'instruments.add'} aria-label={mine ? `${e.name} is in My Sounds` : `Add ${e.name} to my sounds`} disabled={mine} onclick={() => add(e.id)}>{mine ? '✓ In My Sounds' : 'Add to my sounds'}</button>
      </li>
    {/each}
  </ul>
{/snippet}

<style>
  .inst {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    min-height: 0;
    height: 100%;
  }
  .screen {
    --accent: #f3b843;
    --danger: #ff7a6b;
    flex: 1;
    min-height: 0;
    display: flex;
    border-radius: 8px;
    overflow: hidden;
  }
  .scroller {
    flex: 1;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: 0.4rem 0.6rem 0.8rem;
    color: var(--screen-ink);
  }
  h3 {
    margin: 0.6rem 0.2rem 0.3rem;
    font-size: 0.72rem;
  }
  .plughead {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  .plughead h3 {
    flex: 1;
  }
  .scan,
  .meta,
  .where,
  .none,
  .empty {
    color: var(--screen-dim);
    font-size: var(--fs-small);
  }
  .empty {
    margin: 0.3rem 0.4rem;
  }
  .card {
    border-top: 1px solid rgb(255 255 255 / 0.06);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    min-height: 2.3rem;
    min-width: 0;
  }
  .meta {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  .warn {
    color: var(--danger);
  }
  button.tw {
    border: 0;
    background: none;
    padding: 0 0.3rem;
    color: var(--screen-ink);
    font-weight: 600;
    white-space: nowrap;
    text-align: left;
  }
  .badge {
    padding: 1px 4px;
    border: 1px solid rgb(223 244 255 / 0.25);
    border-radius: 3px;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.72rem;
    letter-spacing: 0.06em;
  }
  .badge.au {
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
  }
  .house {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.6rem;
    padding: 0.2rem 0 0.4rem 1.6rem;
  }
  .field {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    font-size: var(--fs-small);
  }
  .field select {
    min-height: 2rem;
    padding: 0 0.4rem;
    border: 1px solid var(--well-edge);
    border-radius: 4px;
    background: var(--screen-bg);
    color: var(--screen-ink);
  }
  .presets {
    list-style: none;
    margin: 0 0 0.5rem;
    padding: 0 0 0 1.6rem;
  }
  .preset {
    display: grid;
    grid-template-columns: minmax(8rem, 1.4fr) minmax(0, 1fr) auto auto;
    align-items: center;
    gap: 0.5rem;
    min-height: 2rem;
  }
  .preset:hover {
    background: rgb(255 255 255 / 0.045);
  }
  .pname,
  .where {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  button.act {
    border: 1px solid rgb(223 244 255 / 0.2);
    border-radius: 4px;
    background: none;
    padding: 0.15rem 0.5rem;
    color: var(--screen-ink);
    font-size: var(--fs-small);
    white-space: nowrap;
  }
  button.act:hover:not(:disabled),
  button.act:focus-visible {
    border-color: var(--accent);
  }
  button.act.done {
    color: var(--accent);
    border-color: transparent;
  }
  .foot {
    margin: 0;
    font-size: var(--fs-small);
    color: var(--muted);
  }
  .foot b {
    color: var(--ink);
  }
</style>
