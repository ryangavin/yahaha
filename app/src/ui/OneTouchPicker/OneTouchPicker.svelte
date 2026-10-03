<!--
  OneTouchPicker: One Touch on the style line, drawn the way the display draws a readout: "One
  Touch" and its small code "OTS" muted, then the numbers 1-4 as faceless buttons in light 18px
  numbers. The applied one is the accent with a 2px accent bar beneath; the rest are muted. A
  click asks for that One Touch through `onapply` and changes nothing itself.
-->
<script lang="ts">
  import type { Action } from 'svelte/action'

  type Props = {
    /** The applied One Touch, 1 to `count`; 0 when none is. */
    applied?: number
    /** How many One Touch buttons. */
    count?: number
    /** The words before the numbers. */
    label?: string
    /** The small code after the words. */
    code?: string
    /** The group's accessible name. Default: says which is applied and the Launchkey's Shift + pads 9 to 12. */
    name?: string
    /** The app's tooltip action (`use:tip`), applied to each number with its key `ots.<n>`. */
    tipAction?: Action<HTMLElement, string>
    /** Called with 1 to `count` when a number is pressed (applies it at once). */
    onapply?: (n: number) => void
  }

  let {
    applied = 0,
    count = 4,
    label = 'One Touch',
    code = 'OTS',
    name,
    tipAction,
    onapply,
  }: Props = $props()

  const numbers = $derived(Array.from({ length: Math.max(0, count) }, (_, i) => i + 1))
  const spoken = $derived(
    name ??
      `One Touch Setting (OTS): ${applied ? `${applied} applied` : 'none applied'}. ` +
        'Click to apply; on the Launchkey, Shift + pads 9 to 12',
  )

  function tipOn(node: HTMLElement, key: string) {
    if (!tipAction) return
    return tipAction(node, key)
  }
</script>

<div class="ots" role="group" aria-label={spoken}>
  <span class="label" aria-hidden="true">{label}{#if code}<span class="code">{code}</span>{/if}</span>
  {#each numbers as n (n)}
    <button
      type="button"
      class="number"
      class:applied={n === applied}
      aria-pressed={n === applied}
      aria-label={n === applied ? `One Touch ${n}, applied (Shift + pad ${n + 8})` : `Apply One Touch ${n} (Shift + pad ${n + 8})`}
      data-tip="ots.{n}"
      use:tipOn={`ots.${n}`}
      onclick={() => onapply?.(n)}>{n}</button
    >
  {/each}
</div>

<style>
  .ots {
    display: inline-flex;
    align-items: center;
    flex: none;
    height: var(--control-height);
    font-family: var(--font-sans);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .label {
    display: inline-flex;
    align-items: baseline;
    gap: var(--space-6);
    margin-right: var(--space-4);
    font-size: var(--text-14);
    font-weight: var(--weight-regular);
    color: var(--m);
  }
  .code {
    font-size: var(--text-12);
  }
  .number {
    position: relative;
    width: var(--ots-number-width);
    height: var(--control-height);
    margin: 0;
    padding: 0 0 var(--bar-lift);
    border: 0;
    border-radius: var(--radius);
    background: none;
    font-family: inherit;
    font-size: var(--text-18);
    font-weight: var(--weight-light);
    color: var(--m);
    cursor: pointer;
  }
  .number:hover {
    color: var(--t2);
  }
  .number.applied {
    color: var(--a);
  }
  .number.applied::after {
    content: '';
    position: absolute;
    left: var(--ots-bar-inset);
    right: var(--ots-bar-inset);
    bottom: var(--bar-bottom);
    height: var(--bar-line);
    background: var(--a);
  }
  .number:focus-visible {
    outline: var(--line-width) solid var(--focus);
    outline-offset: calc(-1 * var(--line-width));
  }
</style>
