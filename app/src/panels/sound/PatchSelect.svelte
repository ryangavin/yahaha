<!--
  A patch picker: the library's patches by category, and "none" (the rule's fallback).
  Sends nothing itself: `onpick` gets the id (null for none).
-->
<script lang="ts">
  import type { TipKey } from '../../help/tooltips'
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import { byCategory } from './nav.svelte'

  let {
    value,
    label,
    tipKey,
    none = '—',
    onpick,
  }: { value: string | null; label: string; tipKey: TipKey; none?: string; onpick: (id: string | null) => void } = $props()

  const groups = $derived(byCategory(app.state.soundLibrary.patches))

  function change(e: Event & { currentTarget: HTMLSelectElement }) {
    const v = e.currentTarget.value
    onpick(v === '' ? null : v)
    e.currentTarget.blur()
  }
</script>

<select class="pick" value={value ?? ''} aria-label={label} use:tip={tipKey} onchange={change}>
  <option value="">{none}</option>
  {#each groups as g (g.category)}
    <optgroup label={g.label}>
      {#each g.patches as p (p.id)}<option value={p.id}>{p.name}{p.available ? '' : ' (fallback)'}</option>{/each}
    </optgroup>
  {/each}
</select>

<style>
  .pick {
    width: 100%;
    min-height: 2.2rem;
    padding: 0 0.4rem;
    border-radius: 4px;
    border: 1px solid var(--well-edge);
    background: var(--screen-bg);
    color: var(--screen-ink);
    font: inherit;
    font-size: 0.9rem;
  }
</style>
