<!--
  The selected library sound (a plugin sound, or a font preset in My Sounds), under the
  Sounds list: rename, recategorise, tags, duplicate and delete (asked first). A sound is
  the raw instrument, with no mix of its own (docs/racks.md). This is what the Sound Library drawer's Patches tab
  was; it folded into the Sounds tab.

  Keys (from the filter): F2 renames (Enter keeps it, Esc goes back), Ctrl/⌘+Delete asks
  to delete. Commands: updatePatch, setSoundCategory, duplicatePatch, deletePatch, and
  exportSoundPreset (Details, a plugin sound).
-->
<script lang="ts">
  import { tick } from 'svelte'
  import { CATEGORY_LABELS, commandSource, type PatchCategory, type PatchFields, type PatchInfo } from '../../lib/api/sound-library'
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import { sourceText } from '../sound/nav.svelte'
  import { presetFileName } from './model'

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

  const fields = (p: PatchInfo): PatchFields => ({ name: p.name, category: p.category, tags: p.tags, favourite: p.favourite, source: commandSource(p.source) })
  const update = (change: Partial<PatchFields>) => app.send({ type: 'updatePatch', id: patch.id, patch: { ...fields(patch), ...change } })
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
  // Export a plugin sound as an .aupreset (O7). A preset of that name the plugin has
  // already (the file is shared with Logic) is only replaced after Replace (#307).
  let askReplace = $state(false)
  $effect(() => {
    void patch.id
    askReplace = false
  })
  function exportPreset(overwrite: boolean) {
    if (patch.source.kind !== 'plugin') return
    const parent = `au:${patch.source.componentId}`
    const n = presetFileName(patch.name).toLowerCase()
    const clash = app.sounds.entries.some((e) => e.parent === parent && e.id.startsWith(`${parent}#u:`) && e.name.toLowerCase() === n)
    if (clash && !overwrite) {
      askReplace = true
      return
    }
    askReplace = false
    app.send({ type: 'exportSoundPreset', id: patch.id, overwrite })
  }
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
      {#if patch.source.kind === 'plugin'}
        {#if askReplace}
          <span class="ask" role="alert">Replace ‘{presetFileName(patch.name)}’?</span>
          <HwButton tip="sound.export_preset_replace" onclick={() => exportPreset(true)}>Replace</HwButton>
          <HwButton tip="sound.export_preset_cancel" onclick={() => (askReplace = false)}>Cancel</HwButton>
        {:else}
          <HwButton tip="sound.export_preset" onclick={() => exportPreset(false)}>Export .aupreset</HwButton>
        {/if}
      {/if}
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
</style>
