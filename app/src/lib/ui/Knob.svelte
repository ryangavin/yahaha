<!--
  A rotary knob that sends relative steps. Drag up/down (about 1 step per 4 px; Shift
  for fine, 1 per 16 px), the mouse wheel, or the arrow keys when focused turn it.
  With a `level` (0-127) the arc and pointer show where it is; with none it is drawn
  as an endless encoder whose pointer shows the last movement, not a value.
  Double-click calls `onreset` only when one is given.
  Its size is `--knob-size` (default 2.4rem); the drawing fills that box edge to edge.
-->
<script lang="ts">
  import { tip } from '../tooltip/tip.svelte'
  import type { TipKey } from '../../help/tooltips'

  // The cap's raised-button gradient needs an id unique to this knob.
  const uid = $props.id()
  const capFill = `knob-cap-${uid}`

  let {
    label,
    level = null,
    disabled = false,
    tipKey,
    onturn,
    onreset,
  }: {
    label: string
    level?: number | null
    disabled?: boolean
    tipKey: TipKey
    onturn: (delta: number) => void
    onreset?: () => void
  } = $props()

  const SWEEP = 270
  const START = -135
  let spin = $state(0) // endless mode: pointer angle from the last movement
  let acc = 0
  let lastY = 0
  let dragging = $state(false)

  const angle = $derived(level === null ? spin : START + (Math.max(0, Math.min(127, level)) / 127) * SWEEP)

  function turn(delta: number) {
    if (disabled || delta === 0) return
    spin = (spin + delta * 12) % 360
    onturn(delta)
  }

  function polar(deg: number, r = 15) {
    const a = ((deg - 90) * Math.PI) / 180
    return [20 + r * Math.cos(a), 20 + r * Math.sin(a)]
  }
  function arc(from: number, to: number) {
    const [x1, y1] = polar(from)
    const [x2, y2] = polar(to)
    return `M ${x1} ${y1} A 15 15 0 ${to - from > 180 ? 1 : 0} 1 ${x2} ${y2}`
  }

  function down(e: PointerEvent) {
    if (disabled || e.button !== 0) return
    ;(e.currentTarget as Element).setPointerCapture(e.pointerId)
    dragging = true
    lastY = e.clientY
    acc = 0
  }
  function move(e: PointerEvent) {
    if (!dragging) return
    acc += (lastY - e.clientY) / (e.shiftKey ? 16 : 4)
    lastY = e.clientY
    const steps = Math.trunc(acc)
    if (steps) {
      acc -= steps
      turn(steps)
    }
  }
  function up() {
    dragging = false
  }
  function wheel(e: WheelEvent) {
    if (disabled) return
    e.preventDefault()
    turn(e.deltaY < 0 ? 1 : -1)
  }
  function key(e: KeyboardEvent) {
    const d = { ArrowUp: 1, ArrowRight: 1, ArrowDown: -1, ArrowLeft: -1 }[e.key]
    if (d === undefined) return
    e.preventDefault()
    turn(d)
  }
</script>

<div
  class="knob"
  class:dragging
  role="slider"
  tabindex={disabled ? -1 : 0}
  aria-label={label}
  aria-disabled={disabled || undefined}
  aria-valuemin={level === null ? undefined : 0}
  aria-valuemax={level === null ? undefined : 127}
  aria-valuenow={level ?? undefined}
  use:tip={tipKey}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={up}
  onwheel={wheel}
  onkeydown={key}
  ondblclick={onreset && !disabled ? onreset : undefined}
>
  <!-- Cropped to the arc's outer edge (r 15 + half its 3-unit stroke), so no margin is wasted. -->
  <svg viewBox="3.5 3.5 33 33" aria-hidden="true">
    <defs>
      <linearGradient id={capFill} x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" class="cap-hi" />
        <stop offset="1" class="cap-lo" />
      </linearGradient>
    </defs>
    {#if level === null}
      <circle cx="20" cy="20" r="15" class="track" />
    {:else}
      <path d={arc(START, START + SWEEP)} class="track" />
      {#if angle > START + 0.5}<path d={arc(START, angle)} class="fill" />{/if}
    {/if}
    <circle cx="20" cy="20" r="11" class="cap" fill="url(#{capFill})" />
    <line x1="20" y1="20" x2={polar(angle, 10)[0]} y2={polar(angle, 10)[1]} class="ptr" />
  </svg>
</div>

<style>
  .knob {
    width: var(--knob-size, 2.4rem);
    height: var(--knob-size, 2.4rem);
    flex: none;
    cursor: ns-resize;
    touch-action: none;
    border-radius: 50%;
    outline: none;
  }
  .knob:focus-visible {
    box-shadow: 0 0 0 2px var(--accent);
  }
  .knob[aria-disabled='true'] {
    opacity: 0.4;
    cursor: default;
  }
  svg {
    width: 100%;
    height: 100%;
    display: block;
  }
  .track {
    fill: none;
    stroke: var(--line-strong);
    stroke-width: 3;
    stroke-linecap: round;
  }
  .fill {
    fill: none;
    stroke: var(--accent);
    stroke-width: 3;
    stroke-linecap: round;
  }
  /* A raised cap, lit from above like the fader caps and buttons (.mat-raised). */
  .cap {
    stroke: rgb(0 0 0 / 0.45);
    stroke-width: 0.6;
  }
  .cap-hi {
    stop-color: var(--raised-hi);
  }
  .cap-lo {
    stop-color: var(--raised-lo);
  }
  .dragging .cap {
    fill: var(--raised-hi);
  }
  .ptr {
    stroke: var(--ink);
    stroke-width: 2;
    stroke-linecap: round;
  }
</style>
