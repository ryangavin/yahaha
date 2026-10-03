<!--
  ChosenTabs: a short run of choices side by side, the chosen one on a white block at the bottom.
  `page` tabs are page navigation (buttons with aria-current, inside the parent's nav); `header` and
  `compact` tabs are a tablist with roving focus and automatic activation (the band's fader page
  and layer). Controlled: it draws `chosen` as given and only calls onchoose; the parent moves it.
-->
<script lang="ts">
  import type { Action } from 'svelte/action'
  import type { TabItem } from './types'

  type Props = {
    /** The choices, left to right. */
    tabs: TabItem[]
    /** The `id` of the chosen tab; `null` = none is chosen. A click never moves it: the parent does. */
    chosen?: string | null
    /** `page` 36 tall, 14px, page navigation; `header` 35 tall, 13px, a tablist; `compact` as `header` with 8px sides. */
    size?: 'page' | 'header' | 'compact'
    /** The tablist's accessible name (`header`, `compact`). Ignored at `page` (the parent's nav carries it). */
    label?: string
    /** The app's tooltip action (`use:tip`), applied to every tab whose item has a `tip`. */
    tipAction?: Action<HTMLElement, string>
    /** Called with a tab's `id` on a click (the chosen tab too), or when an arrow, Home or End moves to it. */
    onchoose?: (id: string) => void
  }

  let { tabs, chosen = null, size = 'header', label, tipAction, onchoose }: Props = $props()

  const isList = $derived(size !== 'page')

  // The tab holding tabindex 0 in a tablist: follows focus, reset to `chosen` when it changes.
  let active = $derived(chosen)

  const buttons: HTMLButtonElement[] = $state([])

  const enabled = $derived(tabs.flatMap((tab, i) => (tab.disabled ? [] : [i])))

  /** The index of the tab that gets tabindex 0 (D15). */
  const stop = $derived.by(() => {
    const i = tabs.findIndex((tab) => tab.id === active)
    if (i >= 0 && !tabs[i].disabled) return i
    return enabled[0] ?? 0
  })

  function tipOn(node: HTMLElement, key: string | undefined) {
    if (!tipAction || !key) return
    return tipAction(node, key)
  }

  function choose(tab: TabItem) {
    if (tab.disabled) return
    onchoose?.(tab.id)
  }

  function onkeydown(event: KeyboardEvent, index: number) {
    const { key } = event
    if (key !== 'ArrowLeft' && key !== 'ArrowRight' && key !== 'Home' && key !== 'End') return
    event.preventDefault()
    event.stopPropagation()
    if (enabled.length === 0) return
    let target: number
    if (key === 'Home') target = enabled[0]
    else if (key === 'End') target = enabled[enabled.length - 1]
    else {
      const step = key === 'ArrowRight' ? 1 : -1
      const at = enabled.indexOf(index)
      const from = at >= 0 ? at : step > 0 ? -1 : 0
      target = enabled[(from + step + enabled.length) % enabled.length]
    }
    if (target === index) return
    const tab = tabs[target]
    active = tab.id
    buttons[target]?.focus()
    onchoose?.(tab.id)
  }

  function onfocusout(event: FocusEvent) {
    const next = event.relatedTarget
    if (!(next instanceof Node) || !(event.currentTarget as HTMLElement).contains(next)) active = chosen
  }

  function face(tab: TabItem): 'chosen' | 'off' | 'disabled' {
    if (tab.disabled) return 'disabled'
    return tab.id === chosen ? 'chosen' : 'off'
  }
</script>

{#if isList}
  <div
    class="run {size}"
    data-size={size}
    role="tablist"
    aria-label={label || undefined}
    aria-orientation="horizontal"
    {onfocusout}
  >
    {#each tabs as tab, i (tab.id)}
      <button
        type="button"
        role="tab"
        class="tab"
        class:chosen={tab.id === chosen}
        bind:this={buttons[i]}
        aria-selected={tab.id === chosen}
        aria-disabled={tab.disabled ? 'true' : undefined}
        aria-label={tab.name}
        tabindex={i === stop ? 0 : -1}
        data-face={face(tab)}
        data-contrast={tab.disabled ? 'dim' : undefined}
        data-tip={tab.tip}
        use:tipOn={tab.tip}
        onclick={() => choose(tab)}
        onfocus={() => (active = tab.id)}
        onkeydown={(event) => onkeydown(event, i)}
      >
        {tab.label}
      </button>
    {/each}
  </div>
{:else}
  <div class="run {size}" data-size={size}>
    {#each tabs as tab (tab.id)}
      <button
        type="button"
        class="tab"
        class:chosen={tab.id === chosen}
        aria-current={tab.id === chosen ? 'page' : undefined}
        aria-disabled={tab.disabled ? 'true' : undefined}
        aria-label={tab.name}
        data-face={face(tab)}
        data-contrast={tab.disabled ? 'dim' : undefined}
        data-tip={tab.tip}
        use:tipOn={tab.tip}
        onclick={() => choose(tab)}
      >
        {tab.label}
      </button>
    {/each}
  </div>
{/if}

<style>
  .run {
    display: flex;
    flex: none;
    align-items: stretch;
    flex-wrap: nowrap;
  }
  .page {
    --tab-h: var(--bar-height);
    --tab-pad-top: var(--space-10);
    --tab-pad-side: var(--space-10);
    --tab-font: var(--text-14);
    --tab-blk: var(--tab-block);
  }
  .header {
    --tab-h: var(--tab-height-header);
    --tab-pad-top: var(--space-11);
    --tab-pad-side: var(--space-10);
    --tab-font: var(--text-13);
    --tab-blk: var(--tab-block-header);
  }
  .compact {
    --tab-h: var(--tab-height-header);
    --tab-pad-top: var(--space-11);
    --tab-pad-side: var(--space-8);
    --tab-font: var(--text-13);
    --tab-blk: var(--tab-block-header);
  }
  .tab {
    display: flex;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;
    height: var(--tab-h);
    margin: 0;
    padding: var(--tab-pad-top) var(--tab-pad-side) 0;
    border: 0;
    border-radius: 0;
    background: transparent;
    color: var(--m);
    font-family: var(--font-sans);
    font-size: var(--tab-font);
    font-weight: var(--weight-regular);
    font-variant-numeric: tabular-nums;
    line-height: normal;
    white-space: nowrap;
    cursor: pointer;
  }
  .chosen {
    background: linear-gradient(var(--t), var(--t)) left bottom / 100% var(--tab-blk) no-repeat;
    color: var(--g);
  }
  .tab[aria-disabled='true'] {
    color: var(--d);
    cursor: default;
  }
  .tab:focus-visible {
    outline: var(--line-width) solid var(--focus);
    outline-offset: var(--focus-offset);
  }
</style>
