<!--
  RackCell: the rack readout at the head of the display's sound row: "Rack · A1" small and muted
  over the rack's name, with its marks (the modified dot) after the name, passed in as `mark`.
  One flat button with no face; a click opens the Rack page.
-->
<script lang="ts">
  import type { Snippet } from 'svelte'
  import type { Action } from 'svelte/action'

  type Props = {
    /** The rack's name ("Sunday drive"). Empty: "No rack". */
    rack: string
    /** The Quick Rack slot it's on ("A1"). Empty: no slot shown. */
    slot?: string
    /** The small word before the slot. */
    label?: string
    /** The accessible name. Default: "Rack: Sunday drive, on Quick Rack A1. Opens the Rack page". */
    name?: string
    /** The tooltip key, set as `data-tip`. */
    tip?: string
    /** The app's tooltip action (`use:tip`), applied when `tip` is set. */
    tipAction?: Action<HTMLElement, string>
    /** The marks after the name (the modified dot). */
    mark?: Snippet
    /** Called on a click, Space or Enter (opens the Rack page). */
    onpress?: () => void
  }

  let { rack, slot = '', label = 'Rack', name, tip, tipAction, mark, onpress }: Props = $props()

  /** Between the label and the slot; an expression, so Svelte keeps its spaces. */
  const SEPARATOR = ' · '
  const shown = $derived(rack.trim() === '' ? 'No rack' : rack)
  const spoken = $derived(
    name ?? `${label}: ${shown}${slot ? `, on Quick Rack ${slot}` : ''}. Opens the Rack page`,
  )

  function tipOn(node: HTMLElement, key: string | undefined) {
    if (!tipAction || !key) return
    return tipAction(node, key)
  }
</script>

<button type="button" class="rack" aria-label={spoken} data-tip={tip} use:tipOn={tip} onclick={() => onpress?.()}>
  <span class="head">{label}{#if slot}{SEPARATOR}<span class="slot">{slot}</span>{/if}</span>
  <span class="name">{shown}{@render mark?.()}</span>
</button>

<style>
  .rack {
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: flex-start;
    gap: var(--space-2);
    width: var(--rack-cell-width);
    min-width: 0;
    height: var(--sound-row-height);
    margin: 0;
    padding: 0;
    border: 0;
    border-top: var(--line-width) solid var(--line);
    background: none;
    font-family: var(--font-sans);
    font-variant-numeric: tabular-nums;
    text-align: left;
    white-space: nowrap;
    cursor: pointer;
  }
  .head {
    font-size: var(--text-12);
    font-weight: var(--weight-regular);
    color: var(--m);
  }
  .slot {
    color: var(--t);
  }
  .name {
    display: flex;
    align-items: center;
    gap: var(--space-6);
    max-width: 100%;
    overflow: hidden;
    font-size: var(--text-14);
    line-height: var(--leading-16);
    color: var(--t);
  }
  .rack:focus-visible {
    outline: var(--line-width) solid var(--focus);
    outline-offset: calc(-1 * var(--line-width));
  }
</style>
