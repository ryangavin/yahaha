<!--
  A small rotary knob for a mixer strip's pan or effect send (0–127): the Genos Mixer's
  Pan/Volume and Effect values. Drag up/down (or scroll-free: arrows ±1, PgUp/PgDn ±10,
  Home/End the ends); double-click puts it back to its default. `centre` draws the arc from
  the middle (pan) instead of from the left.
-->
<script lang="ts">
  import type { TipKey } from '../../help/tooltips'
  import { tip, tips } from '../../lib/tooltip/tip.svelte'

  let {
    value,
    tip: tipKey,
    label,
    caption,
    onchange,
    reset,
    centre = false,
    format = String,
  }: {
    value: number
    tip: TipKey
    label: string
    /** The short name under the knob ("Pan", "Rev", "Cho"). */
    caption: string
    onchange: (v: number) => void
    /** The value a double-click goes back to. */
    reset: number
    centre?: boolean
    format?: (v: number) => string
  } = $props()

  const MAX = 127
  /** The knob turns through 270°, from 7:30 to 4:30. */
  const SWEEP = 270
  const angle = (v: number) => -SWEEP / 2 + (Math.max(0, Math.min(MAX, v)) / MAX) * SWEEP
  const R = 10
  const point = (deg: number) => {
    const a = ((deg - 90) * Math.PI) / 180
    return `${12 + R * Math.cos(a)} ${12 + R * Math.sin(a)}`
  }
  function arc(from: number, to: number) {
    const [a, b] = from <= to ? [from, to] : [to, from]
    if (b - a < 0.5) return ''
    return `M ${point(a)} A ${R} ${R} 0 ${b - a > 180 ? 1 : 0} 1 ${point(b)}`
  }
  const lit = $derived(arc(centre ? 0 : -SWEEP / 2, angle(value)))

  let drag: { y: number; v: number } | null = null
  const set = (v: number) => {
    const c = Math.max(0, Math.min(MAX, Math.round(v)))
    if (c !== value) onchange(c)
  }
  function down(e: PointerEvent) {
    drag = { y: e.clientY, v: value }
    ;(e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId)
  }
  function move(e: PointerEvent) {
    if (!drag) return
    set(drag.v + (drag.y - e.clientY) / 1.5)
    tips.refresh()
  }
  function up() {
    drag = null
  }
  function key(e: KeyboardEvent) {
    const step: Record<string, number> = { ArrowUp: 1, ArrowRight: 1, ArrowDown: -1, ArrowLeft: -1, PageUp: 10, PageDown: -10 }
    let v: number | null = null
    if (e.key in step) v = value + step[e.key]
    else if (e.key === 'Home') v = 0
    else if (e.key === 'End') v = MAX
    if (v === null) return
    e.preventDefault()
    e.stopPropagation()
    set(v)
  }
</script>

<div
  class="knob"
  role="slider"
  tabindex="0"
  aria-label={label}
  aria-valuemin={0}
  aria-valuemax={MAX}
  aria-valuenow={value}
  aria-valuetext={format(value)}
  use:tip={tipKey}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={up}
  ondblclick={() => set(reset)}
  onkeydown={key}
>
  <svg viewBox="0 0 24 24" aria-hidden="true">
    <path class="track" d={arc(-SWEEP / 2, SWEEP / 2)} />
    {#if lit}<path class="lit" d={lit} />{/if}
    <circle class="cap" cx="12" cy="12" r="7" />
    <line class="pointer" x1="12" y1="12" x2="12" y2="6" style:transform="rotate({angle(value)}deg)" />
  </svg>
  <span class="caption">{caption}</span>
  <span class="value">{format(value)}</span>
</div>

<style>
  .knob {
    display: grid;
    justify-items: center;
    gap: 0.05rem;
    min-width: 0;
    cursor: ns-resize;
    touch-action: none;
    border-radius: 4px;
    user-select: none;
  }
  /* Up to 1.7rem, smaller when four knobs share a narrow strip. */
  svg {
    width: min(1.7rem, 100%);
    height: auto;
    aspect-ratio: 1;
    overflow: visible;
  }
  .track,
  .lit {
    fill: none;
    stroke-width: 2.2;
    stroke-linecap: round;
  }
  .track {
    stroke: var(--lamp-off);
  }
  .lit {
    stroke: var(--accent);
  }
  .cap {
    fill: var(--raised-lo);
    stroke: var(--raised-hi);
    stroke-width: 1;
  }
  .pointer {
    stroke: var(--ink);
    stroke-width: 1.6;
    stroke-linecap: round;
    transform-origin: 12px 12px;
  }
  .caption,
  .value {
    font-family: var(--font-display);
    line-height: 1.05;
    white-space: nowrap;
  }
  .caption {
    font-size: 0.65rem;
    color: var(--muted);
  }
  .value {
    font-size: 0.72rem;
    font-weight: 600;
    color: var(--ink);
  }
</style>
