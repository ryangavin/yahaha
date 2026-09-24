<!--
  Patches: your list by category, with search, a category filter and favourites. A row
  auditions its patch (▶) and selects it for the editor under the list: name, category,
  tags, the defaults a part takes when it picks the patch, order, duplicate, delete, and
  "play on" a keyboard part. Then: save a part's sound as a patch, and the library file.

  State: soundLibrary, keyboardParts, transport.running. Commands: auditionPatch,
  stopPatchAudition, setPatchFavourite, updatePatch, movePatch, duplicatePatch,
  deletePatch, setPartPatch, savePartAsPatch, setPortSendsMapped, exportSoundLibrary,
  importSoundLibrary.
-->
<script lang="ts">
  import { CATEGORY_LABELS, type PatchCategory, type PatchDefaults, type PatchFields, type PatchInfo } from '../../lib/api/sound-library'
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'
  import { byCategory, filterPatches, nav, sourceText } from './nav.svelte'

  const sl = $derived(app.state.soundLibrary)
  const parts = $derived(app.state.keyboardParts)
  const running = $derived(app.state.transport.running)
  let query = $state('')
  let category = $state<PatchCategory | 'all'>('all')
  let favourites = $state(false)
  let savePart = $state(0)
  let importPath = $state('')

  const shown = $derived(filterPatches(sl.patches, query, category, favourites))
  const groups = $derived(byCategory(shown))
  const sel = $derived(sl.patches.find((p) => p.id === nav.selected) ?? null)
  const selIndex = $derived(sel ? sl.patches.indexOf(sel) : -1)
  const CATS = Object.keys(CATEGORY_LABELS) as PatchCategory[]
  const SHORT = ['R1', 'R2', 'R3', 'L']

  // A new patch (added, duplicated, saved) opens in the editor.
  let lastSeen: string | null = null
  $effect(() => {
    const id = sl.lastAdded
    if (id && id !== lastSeen) nav.selected = id
    lastSeen = id
  })

  const fields = (p: PatchInfo): PatchFields => ({ name: p.name, category: p.category, tags: p.tags, favourite: p.favourite, source: p.source, defaults: p.defaults })
  const update = (p: PatchInfo, change: Partial<PatchFields>) => app.send({ type: 'updatePatch', id: p.id, patch: { ...fields(p), ...change } })
  const setDefault = (p: PatchInfo, key: keyof PatchDefaults, v: number | null) => update(p, { defaults: { ...p.defaults, [key]: v } })

  function numberOrNull(e: Event & { currentTarget: HTMLInputElement }): number | null {
    const t = e.currentTarget.value.trim()
    if (t === '') return null
    const n = Math.round(Number(t))
    return Number.isFinite(n) ? Math.max(0, Math.min(127, n)) : null
  }
  const enterBlurs = (e: KeyboardEvent & { currentTarget: HTMLInputElement }) => e.key === 'Enter' && e.currentTarget.blur()
  const audition = (id: string) => app.send(sl.auditioning === id ? { type: 'stopPatchAudition' } : { type: 'auditionPatch', id })
</script>

<div class="filters">
  <input class="search" type="search" placeholder="Search patches" aria-label="Search patches" bind:value={query} use:tip={'sound.search'} />
  <select class="cat" aria-label="Category" bind:value={category} use:tip={'sound.category'}>
    <option value="all">All categories</option>
    {#each CATS as c (c)}<option value={c}>{CATEGORY_LABELS[c]}</option>{/each}
  </select>
  <Toggle on={favourites} tip="sound.favourites" onclick={() => (favourites = !favourites)}>★</Toggle>
</div>

{#if sl.patches.length === 0}
  <p class="explain">
    Your library is empty. Add presets from a SoundFont (Add from SoundFont), or save a keyboard part's sound below.
    About 20 patches, reused by every style through the Program Map, is the idea.
  </p>
{:else if shown.length === 0}
  <p class="explain">No patch matches.</p>
{/if}

<div class="list" role="listbox" aria-label="Patches">
  {#each groups as g (g.category)}
    <span class="group engraved">{g.label}</span>
    {#each g.patches as p (p.id)}
      <div class="row" class:sel={nav.selected === p.id} class:fallback={!p.available}>
        <button type="button" class="star" class:on={p.favourite} aria-label="Favourite {p.name}" aria-pressed={p.favourite} use:tip={'sound.favourite'} onclick={() => app.send({ type: 'setPatchFavourite', id: p.id, favourite: !p.favourite })}>★</button>
        <button type="button" class="pname" role="option" aria-selected={nav.selected === p.id} use:tip={'sound.patch'} onclick={() => (nav.selected = nav.selected === p.id ? null : p.id)}>
          <span class="n">{p.name}</span>
          <span class="src">{p.note ?? sourceText(p)}</span>
        </button>
        <button
          type="button"
          class="play mat-raised"
          class:on={sl.auditioning === p.id}
          aria-label="{sl.auditioning === p.id ? 'Stop' : 'Audition'} {p.name}"
          aria-disabled={!p.available || running}
          use:tip={sl.auditioning === p.id ? 'sound.audition_stop' : 'sound.audition'}
          onclick={() => audition(p.id)}
        >{sl.auditioning === p.id ? '■' : '▶'}</button>
      </div>
    {/each}
  {/each}
</div>
{#if running && sl.patches.length}<p class="explain">Stop the band to audition.</p>{/if}

{#if sel}
  <section class="editor mat-well" aria-label="Edit {sel.name}">
    <div class="line">
      <input class="name" aria-label="Patch name" value={sel.name} use:tip={'sound.name'} onchange={(e) => update(sel, { name: e.currentTarget.value })} onkeydown={enterBlurs} />
      <select aria-label="Patch category" value={sel.category} use:tip={'sound.edit_category'} onchange={(e) => update(sel, { category: e.currentTarget.value as PatchCategory })}>
        {#each CATS as c (c)}<option value={c}>{CATEGORY_LABELS[c]}</option>{/each}
      </select>
    </div>
    <input
      aria-label="Tags"
      placeholder="tags, comma separated"
      value={sel.tags.join(', ')}
      use:tip={'sound.tags'}
      onchange={(e) => update(sel, { tags: e.currentTarget.value.split(',').map((t) => t.trim()).filter(Boolean) })}
      onkeydown={enterBlurs}
    />
    <p class="src engraved">{sourceText(sel)}{sel.note ? ` · plays the SoundFont fallback: ${sel.note}` : ''}</p>
    <div class="defaults">
      {#each [['volume', 'Vol', 'sound.default_volume'], ['pan', 'Pan', 'sound.default_pan'], ['reverb', 'Rev', 'sound.default_reverb'], ['chorus', 'Cho', 'sound.default_chorus']] as const as [key, lab, t] (key)}
        <label class="num">
          <small class="engraved">{lab}</small>
          <input type="number" min="0" max="127" placeholder="—" value={sel.defaults[key] ?? ''} use:tip={t} onchange={(e) => setDefault(sel, key, numberOrNull(e))} onkeydown={enterBlurs} />
        </label>
      {/each}
      <label class="num">
        <small class="engraved">Oct</small>
        <select value={String(sel.defaults.octave)} aria-label="Octave" use:tip={'sound.default_octave'} onchange={(e) => update(sel, { defaults: { ...sel.defaults, octave: Number(e.currentTarget.value) } })}>
          {#each [-2, -1, 0, 1, 2] as o (o)}<option value={String(o)}>{o > 0 ? `+${o}` : o}</option>{/each}
        </select>
      </label>
    </div>
    <div class="line">
      <span class="engraved">Play on</span>
      {#each SHORT as s, i (i)}
        <HwButton tip="sound.use_on_part" pressed={parts[i]?.patch === sel.id} label="Play {sel.name} on {parts[i]?.name}" onclick={() => app.send({ type: 'setPartPatch', part: i, id: parts[i]?.patch === sel.id ? null : sel.id })}>{s}</HwButton>
      {/each}
    </div>
    <div class="line tools">
      <HwButton tip="sound.move_up" label="Move up" onclick={() => app.send({ type: 'movePatch', id: sel.id, to: Math.max(0, selIndex - 1) })}>↑</HwButton>
      <HwButton tip="sound.move_down" label="Move down" onclick={() => app.send({ type: 'movePatch', id: sel.id, to: selIndex + 1 })}>↓</HwButton>
      <HwButton tip="sound.duplicate" onclick={() => app.send({ type: 'duplicatePatch', id: sel.id })}>Duplicate</HwButton>
      <HwButton tip="sound.delete" onclick={() => app.send({ type: 'deletePatch', id: sel.id })}>Delete</HwButton>
    </div>
  </section>
{/if}

<section class="block" aria-labelledby="sl-save">
  <h3 id="sl-save" class="engraved">Save a part's sound</h3>
  <div class="line">
    <select aria-label="Keyboard part" bind:value={savePart} use:tip={'sound.save_part'}>
      {#each parts as p, i (i)}<option value={i}>{p.name} · {p.voiceName}</option>{/each}
    </select>
    <HwButton tip="sound.save_part" onclick={() => app.send({ type: 'savePartAsPatch', part: savePart, name: null })}>Save as patch</HwButton>
  </div>
</section>

<section class="block" aria-labelledby="sl-file">
  <h3 id="sl-file" class="engraved">Library file</h3>
  <p class="explain">{sl.file ? `Saved to ${sl.file} after every change.` : 'This session saves nowhere (offline).'}</p>
  <Toggle on={sl.portSendsMapped} tip="sound.port_mapped" onclick={() => app.send({ type: 'setPortSendsMapped', on: !sl.portSendsMapped })}>MIDI port sends mapped programs</Toggle>
  <div class="line">
    <HwButton tip="sound.export" onclick={() => app.send({ type: 'exportSoundLibrary', path: null })}>Export</HwButton>
    <input aria-label="Library file to import" placeholder="/path/to/sound-library.json" bind:value={importPath} use:tip={'sound.import_path'} />
    <HwButton tip="sound.import" onclick={() => importPath.trim() && app.send({ type: 'importSoundLibrary', path: importPath.trim(), replace: false, maps: true })}>Import</HwButton>
  </div>
</section>

<style>
  .filters,
  .line {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .filters .search,
  .line input:not([type='number']) {
    flex: 1;
    min-width: 0;
  }
  input,
  select {
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
  .list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .group {
    margin: 0.4rem 0 0.1rem;
    font-size: 0.72rem;
  }
  .row {
    display: grid;
    grid-template-columns: 2rem 1fr 2.4rem;
    align-items: center;
    gap: 0.3rem;
    border-radius: 5px;
    border: 1px solid transparent;
  }
  .row.sel {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 8%, transparent);
  }
  .row.fallback .n {
    color: var(--muted);
  }
  .star {
    min-height: 2.2rem;
    border: 0;
    background: transparent;
    color: var(--lamp-off);
    font-size: 1rem;
  }
  .star.on {
    color: var(--accent);
  }
  .pname {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    min-width: 0;
    min-height: 2.2rem;
    padding: 0.15rem 0.3rem;
    border: 0;
    background: transparent;
    color: var(--ink);
    text-align: left;
    font: inherit;
  }
  .n {
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.98rem;
  }
  .src {
    max-width: 100%;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-size: 0.74rem;
    color: var(--muted);
  }
  .play {
    min-height: 2.2rem;
    border-radius: 5px;
    color: var(--ink);
  }
  .play.on {
    color: var(--accent);
  }
  .play[aria-disabled='true'] {
    opacity: 0.45;
  }
  .editor {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    padding: 0.6rem;
    border-radius: 7px;
  }
  .editor .name {
    font-family: var(--font-display);
    font-weight: 600;
  }
  p.src {
    margin: 0;
    white-space: normal;
  }
  .defaults {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 0.35rem;
  }
  .num {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .num input,
  .num select {
    width: 100%;
    min-width: 0;
  }
  .tools {
    flex-wrap: wrap;
  }
  .block {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }
  h3 {
    margin: 0;
    font-size: 0.78rem;
  }
</style>
