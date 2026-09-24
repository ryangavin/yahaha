<!--
  A row of tabs (fader pages, pad pages, browser folders). Each tab has its own catalog
  tip. Left/Right arrows move between tabs, as in any tablist.
-->
<script lang="ts" generics="T extends string">
  import type { TipKey } from '../../help/tooltips'
  import { tip as tipAction } from '../tooltip/tip.svelte'

  let {
    items,
    value,
    onselect,
    label,
  }: {
    items: { id: T; label: string; tip: TipKey; color?: string }[]
    value: T
    onselect: (id: T) => void
    label: string
  } = $props()

  let list: HTMLDivElement | undefined = $state()

  function key(e: KeyboardEvent) {
    if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') return
    e.preventDefault()
    e.stopPropagation()
    const i = items.findIndex((t) => t.id === value)
    const n = items[(i + (e.key === 'ArrowRight' ? 1 : -1) + items.length) % items.length]
    onselect(n.id)
    list?.querySelector<HTMLButtonElement>(`[data-id="${n.id}"]`)?.focus()
  }
</script>

<div class="tabs" role="tablist" aria-label={label} tabindex="-1" bind:this={list} onkeydown={key}>
  {#each items as t (t.id)}
    <button
      type="button"
      role="tab"
      data-id={t.id}
      aria-selected={t.id === value}
      tabindex={t.id === value ? 0 : -1}
      style:--tab-color={t.color ?? 'var(--accent)'}
      use:tipAction={t.tip}
      onclick={() => onselect(t.id)}
    >
      {t.label}
    </button>
  {/each}
</div>

<style>
  .tabs {
    display: inline-flex;
    gap: 2px;
    padding: 2px;
    background: var(--well);
    border: 1px solid var(--line);
    border-radius: var(--r-key);
  }
  button {
    min-height: 2rem;
    padding: 0.15rem 0.8rem;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 4px;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: var(--fs-label);
    color: var(--muted);
  }
  button:hover {
    color: var(--ink);
  }
  button[aria-selected='true'] {
    color: var(--ink);
    background: var(--raised);
    border-color: var(--tab-color);
    box-shadow: inset 0 -2px 0 var(--tab-color);
  }
</style>
