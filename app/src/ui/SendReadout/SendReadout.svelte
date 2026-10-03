<!--
  SendReadout: the band's effect sends on the style line, "Band Reverb 40 Chorus 12 Delay 0":
  a muted lead word, each send's name in secondary text and its 0-127 level in light accent
  numbers. One flat button with no face; a click opens Effects.
-->
<script lang="ts">
  import type { Action } from 'svelte/action'

  type Props = {
    /** The reverb send, 0-127. */
    reverb: number
    /** The chorus send, 0-127. */
    chorus: number
    /** The delay send, 0-127. */
    delay: number
    /** The lead word ("Band"). */
    label?: string
    /** The accessible name. Default: "Band sends: reverb 40, chorus 12, delay 0. Opens Effects". */
    name?: string
    /** The tooltip key, set as `data-tip`. */
    tip?: string
    /** The app's tooltip action (`use:tip`), applied when `tip` is set. */
    tipAction?: Action<HTMLElement, string>
    /** Called on a click, Space or Enter (opens Effects). */
    onpress?: () => void
  }

  let { reverb, chorus, delay, label = 'Band', name, tip, tipAction, onpress }: Props = $props()

  const sends = $derived([
    ['Reverb', reverb],
    ['Chorus', chorus],
    ['Delay', delay],
  ] as const)
  const spoken = $derived(
    name ?? `${label} sends: reverb ${reverb}, chorus ${chorus}, delay ${delay}. Opens Effects`,
  )

  function tipOn(node: HTMLElement, key: string | undefined) {
    if (!tipAction || !key) return
    return tipAction(node, key)
  }
</script>

<button type="button" class="sends" aria-label={spoken} data-tip={tip} use:tipOn={tip} onclick={() => onpress?.()}>
  {label}
  {#each sends as [word, level] (word)}
    <span class="send">{word} <span class="level">{level}</span></span>
  {/each}
</button>

<style>
  .sends {
    display: inline-flex;
    align-items: baseline;
    gap: var(--space-10);
    flex: none;
    height: var(--send-height);
    margin: 0;
    padding: 0;
    border: 0;
    background: none;
    color: var(--m);
    font-family: var(--font-sans);
    font-size: var(--text-14);
    font-weight: var(--weight-regular);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    cursor: pointer;
  }
  .send {
    color: var(--t2);
  }
  .level {
    font-size: var(--text-18);
    font-weight: var(--weight-light);
    color: var(--a);
  }
  .sends:focus-visible {
    outline: var(--line-width) solid var(--focus);
    outline-offset: var(--focus-offset);
  }
</style>
