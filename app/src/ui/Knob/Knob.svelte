<!--
  Knob: one of the eight band knobs. The plain name above, the value and unit in the accent, a
  bare 44px 270° ring arc with a small tip dot at the arc's end, and the Genos code beneath. An
  unused knob shows only its dim name and an empty ring. Fully controlled: ↑ / ↓ and the wheel ask
  for a step through `onstep`; a click calls `onpress`.
-->
<script lang="ts">
  import type { Action } from 'svelte/action'

  type Props = {
    /** The plain name ("Dynamics", "Retrig rate"). For an unused knob, the board's "---". */
    label: string
    /** The Genos code beneath the ring ("DynCtrl"). */
    code?: string
    /** The value as shown ("127", "1/8", "Off"). */
    value?: string
    /** A small unit after the value ("%"). */
    unit?: string
    /** How far round the arc is, 0–1 of its 270°. */
    fraction?: number
    /** Nothing is mapped to this knob: dim name, an empty ring, no value. */
    unused?: boolean
    /** The accessible name. Default: "{label} ({code}) {value}{unit}", or "unused". */
    name?: string
    /** The tooltip key, rendered as `data-tip`. */
    tip?: string
    /** The app's `use:tip` action, applied with `tip` when both are set. */
    tipAction?: Action<HTMLElement, string>
    /** Called on a click, Space or Enter. */
    onpress?: () => void
    /** Called with +1 or −1 on ↑ / ↓ or a wheel step. Not when unused. */
    onstep?: (delta: number) => void
  }

  let {
    label,
    code = '',
    value = '',
    unit = '',
    fraction = 0,
    unused = false,
    name,
    tip,
    tipAction,
    onpress,
    onstep,
  }: Props = $props()

  // The ring's geometry in its own 44 × 44 box: radius 21, the arc starting 225° clockwise from
  // the top (bottom left) and sweeping 270°.
  const C = 22
  const R = 21
  const START = 225
  const SWEEP = 270

  function point(deg: number) {
    const th = (deg * Math.PI) / 180
    return { x: C + R * Math.sin(th), y: C - R * Math.cos(th) }
  }
  function arc(from: number, to: number) {
    if (to - from <= 0) return ''
    const a = point(from)
    const b = point(to)
    const large = to - from > 180 ? 1 : 0
    return `M ${a.x.toFixed(2)} ${a.y.toFixed(2)} A ${R} ${R} 0 ${large} 1 ${b.x.toFixed(2)} ${b.y.toFixed(2)}`
  }

  let deg = $derived(unused ? 0 : Math.round(Math.min(1, Math.max(0, fraction)) * SWEEP))
  let lit = $derived(arc(START, START + deg))
  let rest = $derived(arc(START + deg, START + SWEEP))
  let tipAt = $derived(point(START + deg))
  let spoken = $derived(name ?? (unused ? 'unused' : `${label}${code ? ` (${code})` : ''} ${value}${unit}`))

  function keydown(event: KeyboardEvent) {
    if (unused) return
    const step = event.key === 'ArrowUp' ? 1 : event.key === 'ArrowDown' ? -1 : 0
    if (step === 0) return
    event.preventDefault()
    onstep?.(step)
  }

  function wheel(event: WheelEvent) {
    if (unused || event.deltaY === 0) return
    event.preventDefault()
    onstep?.(event.deltaY < 0 ? 1 : -1)
  }

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
  class="knob"
  class:unused
  data-tip={tip}
  aria-label={spoken}
  onclick={() => onpress?.()}
  onkeydown={keydown}
  onwheel={wheel}
  use:tipped={tip}
>
  <span class="label">{label}</span>
  <span class="value">{unused ? '' : value}{#if unit && !unused}<span class="unit">{unit}</span>{/if}</span>
  <svg class="ring" viewBox="0 0 44 44" aria-hidden="true">
    {#if unused}
      <path class="empty" d={arc(START, START + SWEEP)} />
    {:else}
      {#if rest}<path class="rest" d={rest} />{/if}
      {#if lit}<path class="lit" d={lit} />{/if}
      <circle class="dot" cx={tipAt.x} cy={tipAt.y} r="3" />
    {/if}
  </svg>
  <span class="code">{code}</span>
</button>

<style>
  .knob {
    display: flex;
    flex-direction: column;
    align-items: center;
    box-sizing: border-box;
    min-width: 0;
    height: var(--knob-height);
    margin: 0;
    padding: 0;
    border: 0;
    background: none;
    font-family: var(--font-sans);
    font-variant-numeric: tabular-nums;
    text-align: center;
    white-space: nowrap;
    cursor: pointer;
  }
  .label,
  .code {
    height: var(--knob-label-height);
    line-height: var(--knob-label-height);
    font-size: var(--text-12);
    font-weight: var(--weight-regular);
  }
  .label {
    color: var(--t2);
  }
  .unused .label {
    color: var(--d);
  }
  .code {
    color: var(--m);
  }
  .value {
    height: var(--knob-value-height);
    line-height: var(--knob-value-height);
    color: var(--a);
    font-size: var(--text-22);
    font-weight: var(--weight-light);
  }
  .unit {
    margin-left: var(--space-2);
    font-size: var(--text-12);
    font-weight: var(--weight-regular);
  }
  .ring {
    display: block;
    flex: none;
    width: var(--knob-ring);
    height: var(--knob-ring);
    margin-top: var(--knob-ring-gap);
    overflow: visible;
  }
  .ring path {
    fill: none;
    stroke-width: var(--knob-stroke);
  }
  .lit {
    stroke: var(--a);
  }
  .rest {
    stroke: var(--knob-rest);
  }
  .empty {
    stroke: var(--knob-unused);
  }
  .dot {
    fill: var(--a);
  }
  .knob:focus-visible {
    outline: var(--line-width) solid var(--focus);
    outline-offset: var(--focus-offset);
  }
</style>
