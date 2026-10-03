<!--
  AccentBlock: "the device" in a solid accent block: the style's name on the style line (a button
  that opens the Browser) and the knob page's name over the knobs (a plain label). Always `--g` on
  `--a`; never means on, chosen or waiting. A long label ends in an ellipsis; an empty one shows
  `empty` so the block never collapses.
-->
<script lang="ts">
  import type { Action } from 'svelte/action'

  type Props = {
    /** The text in the block ("Sunday Drive Pop", "Style", "Swap R1"). */
    label: string
    /** Shown when the label is empty or whitespace only. */
    empty?: string
    /** `button` calls onpress (the style name); `span` is a plain label (the knob page). */
    as?: 'span' | 'button'
    /** `line` 26 tall, 18px medium (the style line); `knob` 22 tall, 13px regular (a band header row). */
    size?: 'line' | 'knob'
    /** A fixed width in px; a longer label ends in an ellipsis. Default: as wide as the label. */
    width?: number
    /** The button's accessible name when the label alone isn't enough ("Sunday Drive Pop: open the Browser"). */
    name?: string
    /** The tooltip key, set as `data-tip` on the button. Ignored for `span`. */
    tip?: string
    /** The app's tooltip action (`use:tip`), applied to the button when `tip` is set. Ignored for `span`. */
    tipAction?: Action<HTMLElement, string>
    /** Called on a click, Space or Enter (`button` only). */
    onpress?: () => void
  }

  let {
    label,
    empty = '—',
    as = 'span',
    size = 'line',
    width,
    name,
    tip,
    tipAction,
    onpress,
  }: Props = $props()

  const shown = $derived(label.trim() === '' ? empty : label)
  const style = $derived(width === undefined ? undefined : `${width}px`)

  function tipOn(node: HTMLElement, key: string | undefined) {
    if (!tipAction || !key) return
    return tipAction(node, key)
  }
</script>

{#if as === 'button'}
  <button
    type="button"
    class="block {size}"
    style:width={style}
    aria-label={name}
    data-face="accent"
    data-hue="a"
    data-size={size}
    data-tip={tip}
    use:tipOn={tip}
    onclick={() => onpress?.()}>{shown}</button
  >
{:else}
  <span class="block {size}" style:width={style} data-face="accent" data-hue="a" data-size={size}>{shown}</span>
{/if}

<style>
  .block {
    display: inline-block;
    vertical-align: top;
    box-sizing: border-box;
    flex: 0 1 auto;
    min-width: 0;
    max-width: 100%;
    margin: 0;
    border: 0;
    border-radius: 0;
    appearance: none;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    text-align: left;
    background: var(--a);
    color: var(--g);
    font-family: var(--font-sans);
    font-variant-numeric: tabular-nums;
    cursor: default;
  }
  .line {
    height: var(--block-height);
    line-height: var(--block-height);
    padding: 0 var(--space-10);
    font-size: var(--text-18);
    font-weight: var(--weight-medium);
  }
  .knob {
    height: var(--block-height-knob);
    line-height: var(--block-height-knob);
    padding: 0 var(--space-8);
    font-size: var(--text-13);
    font-weight: var(--weight-regular);
  }
  button.block {
    cursor: pointer;
  }
  button.block:focus-visible {
    outline: var(--line-width) solid var(--focus);
    outline-offset: var(--focus-offset);
  }
</style>
