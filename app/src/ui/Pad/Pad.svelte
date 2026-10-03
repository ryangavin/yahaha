<!--
  Pad: one 68px band pad. The caption at the bottom left in its family's hue, the pad's index top
  right in mono, and a 2px bar at the bottom. Faces: idle (plain face), dark (the style lacks it),
  playing and running (solid hue, ink label), next (outlined, "NEXT", a glowing bar) and armed
  (outlined with a glow, "ARMED"). Next and armed flash: the parent passes the phase in `lit`.
  Utility pads are grey and white; Start / Stop goes solid green when running.
-->
<script lang="ts">
  import type { Action } from 'svelte/action'

  type Family = 'intro' | 'main' | 'ending' | 'brk' | 'fill' | 'util' | 'start'

  type Props = {
    /** The caption ("Main B", "Sync Start"). Tie a numeral to its word with a no-break space. */
    label: string
    /** The pad's number on the Launchkey, top right ("1" … "16"). Replaced by NEXT or ARMED. */
    index?: string
    /** The family: a section hue, `util` (grey/white) or `start` (Start / Stop, green when running). */
    family?: Family
    /** The face. */
    state?: 'idle' | 'dark' | 'playing' | 'next' | 'armed' | 'running'
    /** The flash phase of `next` and `armed`: false draws the outline and bar off. */
    lit?: boolean
    /** The accessible name. Default: "{label} (pad {index})". */
    name?: string
    /** The tooltip key, rendered as `data-tip`. */
    tip?: string
    /** The app's `use:tip` action, applied with `tip` when both are set. */
    tipAction?: Action<HTMLElement, string>
    /** Called on a click, Space or Enter. */
    onpress?: () => void
  }

  let {
    label,
    index = '',
    family = 'main',
    state = 'idle',
    lit = true,
    name,
    tip,
    tipAction,
    onpress,
  }: Props = $props()

  let util = $derived(family === 'util' || family === 'start')
  /** The hue token: the family's, white for utilities, green for a running Start / Stop. */
  let hue = $derived(family === 'start' ? (state === 'running' ? 'ok' : 't') : util ? 't' : family)
  let corner = $derived(state === 'next' ? 'NEXT' : state === 'armed' ? 'ARMED' : index)
  let solid = $derived(state === 'playing' || state === 'running')
  let waiting = $derived(state === 'next' || state === 'armed')

  const tipped: Action<HTMLElement, string | undefined> = (node, key) => {
    if (!tipAction || key === undefined) return
    const handle = tipAction(node, key)
    return {
      update: (next) => {
        if (next !== undefined) handle?.update?.(next)
      },
      destroy: () => handle?.destroy?.(),
    }
  }
</script>

<button
  type="button"
  class="pad face-{state}"
  class:util
  class:solid
  class:waiting
  class:unlit={waiting && !lit}
  style:--hue="var(--{hue})"
  style:--glow="var(--pad-glow-{hue})"
  style:--bar-glow="var(--pad-bar-glow-{hue})"
  data-face={state}
  data-hue={hue}
  data-tip={tip}
  aria-label={name ?? `${label}${index ? ` (pad ${index})` : ''}`}
  onclick={() => onpress?.()}
  use:tipped={tip}
>
  <span class="index" aria-hidden="true">{corner}</span>
  <span class="label">{label}</span>
  <span class="bar" aria-hidden="true"></span>
</button>

<style>
  .pad {
    position: relative;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    align-items: flex-start;
    box-sizing: border-box;
    width: var(--pad-size);
    min-width: 0;
    height: var(--pad-size);
    margin: 0;
    padding: 0 var(--space-4) var(--pad-label-bottom);
    border: var(--line-width) solid transparent;
    border-radius: var(--radius);
    background: var(--btn);
    color: var(--hue);
    font-family: var(--font-sans);
    text-align: left;
    cursor: pointer;
  }
  .util {
    color: var(--t2);
  }
  .label {
    font-size: var(--text-14);
    font-weight: var(--weight-medium);
    line-height: var(--pad-label-height);
    letter-spacing: var(--pad-label-tracking);
  }
  .index {
    position: absolute;
    top: var(--pad-index-top);
    right: var(--pad-index-right);
    color: var(--d);
    font-family: var(--font-mono);
    font-size: var(--text-11);
    line-height: var(--pad-index-height);
  }
  .bar {
    position: absolute;
    right: var(--pad-bar-inset);
    bottom: var(--pad-bar-bottom);
    left: var(--pad-bar-inset);
    height: var(--space-2);
    border-radius: calc(var(--space-2) / 2);
    background: transparent;
  }

  .face-dark {
    color: var(--d);
  }
  .face-dark .index {
    color: var(--pad-dark-index);
  }

  .solid {
    border-color: var(--hue);
    background: var(--hue);
    color: var(--pad-solid-ink);
    box-shadow: var(--glow);
  }
  .solid .index {
    color: var(--pad-solid-index);
  }
  .solid .bar {
    background: var(--pad-solid-bar);
  }

  .waiting {
    border-color: var(--hue);
    color: var(--t);
  }
  .waiting .index {
    color: var(--hue);
  }
  .waiting .bar {
    background: var(--hue);
  }
  .face-next .bar {
    box-shadow: var(--bar-glow);
  }
  .face-armed {
    box-shadow: var(--glow);
  }
  /* The flash's off phase: outline, glow and bar off; the words stay. */
  .waiting.unlit {
    border-color: transparent;
    box-shadow: none;
  }
  .waiting.unlit .bar {
    background: transparent;
    box-shadow: none;
  }

  .pad:focus-visible {
    outline: var(--line-width) solid var(--focus);
    outline-offset: var(--focus-offset);
  }
</style>
