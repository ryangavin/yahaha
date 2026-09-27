<!--
  The selected library sound (a plugin sound, or a font preset in My Sounds), under the
  Sounds list: rename, recategorise, tags, the defaults a part takes when it picks it,
  duplicate and delete (asked first). This is what the Sound Library drawer's Patches tab
  was; it folded into the Sounds tab.

  Keys (from the filter): F2 renames (Enter keeps it, Esc goes back), Ctrl/⌘+Delete asks
  to delete. Commands: updatePatch, setSoundCategory, duplicatePatch, deletePatch.
-->
<script lang="ts">
  import { tick } from 'svelte'
  import { CATEGORY_LABELS, type PatchCategory, type PatchDefaults, type PatchFields, type PatchInfo } from '../../lib/api/sound-library'
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import { sourceText } from '../sound/nav.svelte'

  let { patch, onback }: { patch: PatchInfo; onback: () => void } = $props()

  const CATS = Object.keys(CATEGORY_LABELS) as PatchCategory[]
  let nameEl: HTMLInputElement | undefined = $state()
  let deleteEl: HTMLElement | undefined = $state()
  let asking = $state(false)
  let more = $state(false)
  // A new selection closes the question.
  $effect(() => {
    void patch.id
    asking = false
  })

  /** F2: focus the name field. */
  export function rename() {
    nameEl?.focus()
    nameEl?.select()
  }
  /** Ctrl/⌘+Delete: ask to delete. */
  export async function askDelete() {
    asking = true
    await tick()
    deleteEl?.querySelector('button')?.focus()
  }

  const fields = (p: PatchInfo): PatchFields => ({ name: p.name, category: p.category, tags: p.tags, favourite: p.favourite, source: p.source, defaults: p.defaults })
  const update = (change: Partial<PatchFields>) => app.send({ type: 'updatePatch', id: patch.id, patch: { ...fields(patch), ...change } })
  const setDefault = (key: keyof PatchDefaults, v: number | null) => update({ defaults: { ...patch.defaults, [key]: v } })
  function numberOrNull(e: Event & { currentTarget: HTMLInputElement }): number | null {
    const t = e.currentTarget.value.trim()
    if (t === '') return null
    const n = Math.round(Number(t))
    return Number.isFinite(n) ? Math.max(0, Math.min(127, n)) : null
  }
  function commitName(el: HTMLInputElement) {
    const n = el.value.trim()
    if (n && n !== patch.name) update({ name: n })
    else el.value = patch.name
  }
  function nameKey(e: KeyboardEvent & { currentTarget: HTMLInputElement }) {
    if (e.key === 'Enter') {
      e.preventDefault()
      commitName(e.currentTarget)
      onback()
    } else if (e.key === 'Escape') {
      e.preventDefault()
      e.stopPropagation()
      e.currentTarget.value = patch.name
      onback()
    }
  }
  const enterBlurs = (e: KeyboardEvent & { currentTarget: HTMLInputElement }) => e.key === 'Enter' && e.currentTarget.blur()
  function remove() {
    app.send({ type: 'deletePatch', id: patch.id })
    asking = false
    onback()
  }
</script>

<section class="edit" aria-label="Edit {patch.name}">
  <div class="line">
    <input bind:this={nameEl} class="name mat-well" aria-label="Sound name" value={patch.name} spellcheck="false" use:tip={'sound.name'} onchange={(e) => commitName(e.currentTarget)} onkeydown={nameKey} />
    <select aria-label="Category of {patch.name}" value={patch.category} use:tip={'sound.edit_category'} onchange={(e) => (app.send({ type: 'setSoundCategory', id: `saved:${patch.id}`, category: e.currentTarget.value as PatchCategory }), onback())}>
      {#each CATS as c (c)}<option value={c}>{CATEGORY_LABELS[c]}</option>{/each}
    </select>
    <HwButton tip="sounds.more" pressed={more} onclick={() => (more = !more)}>Details</HwButton>
    <HwButton tip="sound.duplicate" onclick={() => app.send({ type: 'duplicatePatch', id: patch.id })}>Duplicate</HwButton>
    {#if asking}
      <span class="ask" role="alert">Delete ‘{patch.name}’?</span>
      <span bind:this={deleteEl}><HwButton tip="sounds.delete_confirm" onclick={remove}>Delete</HwButton></span>
      <HwButton tip="sounds.delete_cancel" onclick={() => ((asking = false), onback())}>Keep</HwButton>
    {:else}
      <HwButton tip="sound.delete" onclick={() => void askDelete()}>Delete…</HwButton>
    {/if}
  </div>
  {#if more}
    <div class="line">
      <input class="tags" aria-label="Tags" placeholder="tags, comma separated" value={patch.tags.join(', ')} use:tip={'sound.tags'} onchange={(e) => update({ tags: e.currentTarget.value.split(',').map((t) => t.trim()).filter(Boolean) })} onkeydown={enterBlurs} />
      <span class="src engraved">{sourceText(patch)}</span>
    </div>
    <div class="defaults">
      {#each [['volume', 'Vol', 'sound.default_volume'], ['pan', 'Pan', 'sound.default_pan'], ['reverb', 'Rev', 'sound.default_reverb'], ['chorus', 'Cho', 'sound.default_chorus']] as const as [key, lab, t] (key)}
        <label class="num">
          <small class="engraved">{lab}</small>
          <input type="number" min="0" max="127" placeholder="—" value={patch.defaults[key] ?? ''} use:tip={t} onchange={(e) => setDefault(key, numberOrNull(e))} onkeydown={enterBlurs} />
        </label>
      {/each}
      <label class="num">
        <small class="engraved">Oct</small>
        <select value={String(patch.defaults.octave)} aria-label="Octave" use:tip={'sound.default_octave'} onchange={(e) => update({ defaults: { ...patch.defaults, octave: Number(e.currentTarget.value) } })}>
          {#each [-2, -1, 0, 1, 2] as o (o)}<option value={String(o)}>{o > 0 ? `+${o}` : o}</option>{/each}
        </select>
      </label>
    </div>
  {/if}
</section>

<style>
  .edit {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 0.5rem;
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
  .name,
  .tags {
    flex: 1;
    min-width: 6rem;
  }
  .name {
    font-family: var(--font-display);
    font-weight: 600;
  }
  .src {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ask {
    color: var(--accent);
    white-space: nowrap;
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
</style>
