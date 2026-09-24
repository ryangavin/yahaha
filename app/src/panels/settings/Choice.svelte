<!--
  A row of mutually exclusive buttons (a segmented switch) for a setting with a few
  values: Lower/Upper, the output pair, All/Selected inputs. Each option has its own
  tooltip key. `disabled` keeps the options hoverable (for their tooltips) but inert.
-->
<script lang="ts" generics="T extends string | number">
  import type { TipKey } from '../../help/tooltips'
  import { tip } from '../../lib/tooltip/tip.svelte'

  let {
    label,
    options,
    value,
    onselect,
    disabled = false,
    columns = 0,
  }: {
    /** Accessible name of the group. */
    label: string
    options: { id: T; label: string; tip: TipKey }[]
    value: T | null
    onselect: (id: T) => void
    disabled?: boolean
    /** Wrap into a grid of this many columns (0: one row). */
    columns?: number
  } = $props()
</script>

<div
  class="choice mat-well"
  class:disabled
  class:grid={columns > 0}
  style:--cols={columns || options.length}
  role="radiogroup"
  aria-label={label}
  aria-disabled={disabled || undefined}
>
  {#each options as o (o.id)}
    <button
      type="button"
      role="radio"
      class="opt"
      class:mat-raised={o.id === value}
      class:on={o.id === value}
      aria-checked={o.id === value}
      aria-disabled={disabled || undefined}
      use:tip={o.tip}
      onclick={() => !disabled && o.id !== value && onselect(o.id)}
    >
      {o.label}
    </button>
  {/each}
</div>

<style>
  .choice {
    display: grid;
    grid-template-columns: repeat(var(--cols), minmax(0, 1fr));
    gap: 3px;
    padding: 3px;
    border-radius: 6px;
  }
  .opt {
    min-height: 2.2rem;
    padding: 0.2rem 0.5rem;
    border: 1px solid transparent;
    border-radius: 4px;
    background: transparent;
    color: var(--screen-dim);
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.95rem;
    letter-spacing: 0.02em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .opt:not(.on):hover {
    color: var(--screen-ink);
    background: rgb(255 255 255 / 0.04);
  }
  .opt.on {
    background: linear-gradient(180deg, var(--raised-hi), var(--raised-lo));
    color: var(--ink);
    box-shadow:
      inset 0 1px 0 rgb(255 255 255 / 0.18),
      inset 0 -2px 0 var(--accent),
      0 1px 0 rgb(0 0 0 / 0.45);
  }
  .disabled .opt {
    cursor: not-allowed;
    opacity: 0.5;
  }
  .disabled .opt:hover {
    background: transparent;
    color: var(--screen-dim);
  }
</style>
