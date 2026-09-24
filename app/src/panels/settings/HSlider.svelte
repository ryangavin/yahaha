<!--
  A horizontal fader for the drawer (0–max): a recessed slot, a ribbed cap moved with
  `transform`, a unity tick, and a readout. Drag or click; with focus, the arrows move ±1,
  PgUp/PgDn ±10, Home/End the ends. No wheel: the drawer scrolls.
-->
<script lang="ts">
  import type { TipKey } from '../../help/tooltips'
  import { tip, tips } from '../../lib/tooltip/tip.svelte'

  let {
    value,
    tip: tipKey,
    label,
    onchange,
    max = 127,
    unity = null,
    disabled = false,
  }: {
    value: number
    tip: TipKey
    label: string
    onchange: (v: number) => void
    max?: number
    /** A tick at this value (100 = unity for the synth master). */
    unity?: number | null
    disabled?: boolean
  } = $props()

  let slot: HTMLDivElement | undefined = $state()
  let dragging = $state(false)
  const frac = $derived(Math.max(0, Math.min(max, value)) / max)

  function fromX(x: number) {
    if (!slot || disabled) return
    const r = slot.getBoundingClientRect()
    const v = Math.round(Math.max(0, Math.min(1, (x - r.left) / r.width)) * max)
    if (v !== value) onchange(v)
  }
  function down(e: PointerEvent) {
    if (disabled) return
    dragging = true
    ;(e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId)
    fromX(e.clientX)
  }
  function move(e: PointerEvent) {
    if (!dragging) return
    fromX(e.clientX)
    tips.refresh()
  }
  function up() {
    dragging = false
  }
  function key(e: KeyboardEvent) {
    if (disabled) return
    const step: Record<string, number> = { ArrowUp: 1, ArrowRight: 1, ArrowDown: -1, ArrowLeft: -1, PageUp: 10, PageDown: -10 }
    let v: number | null = null
    if (e.key in step) v = value + step[e.key]
    else if (e.key === 'Home') v = 0
    else if (e.key === 'End') v = max
    if (v === null) return
    e.preventDefault()
    e.stopPropagation()
    onchange(Math.max(0, Math.min(max, v)))
  }
</script>

<div class="hslider" class:disabled>
  <div
    class="track"
    role="slider"
    tabindex="0"
    aria-label={label}
    aria-disabled={disabled || undefined}
    aria-valuemin={0}
    aria-valuemax={max}
    aria-valuenow={value}
    use:tip={tipKey}
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={up}
    onkeydown={key}
  >
    <div class="slot mat-well" bind:this={slot}>
      {#if unity !== null}<span class="unity" style:left="{(unity / max) * 100}%"></span>{/if}
    </div>
    <div class="rail" aria-hidden="true">
      <div class="carrier" style:transform="translateX({frac * 100}%)"><div class="cap mat-raised"></div></div>
    </div>
  </div>
  <span class="readout mat-screen" aria-hidden="true"><span class="glow-text">{disabled ? '—' : value}</span></span>
</div>

<style>
  .hslider {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    width: 100%;
  }
  .track {
    position: relative;
    flex: 1;
    height: 2.2rem;
    cursor: ew-resize;
    touch-action: none;
    border-radius: 4px;
  }
  .disabled .track {
    cursor: default;
    opacity: 0.45;
  }
  .slot {
    position: absolute;
    left: 0.7rem;
    right: 0.7rem;
    top: 50%;
    height: 6px;
    transform: translateY(-50%);
    border-radius: 3px;
  }
  .unity {
    position: absolute;
    top: -7px;
    bottom: -7px;
    width: 1px;
    background: var(--engrave);
    opacity: 0.7;
  }
  .rail {
    position: absolute;
    left: 0.7rem;
    right: 0.7rem;
    top: 0;
    bottom: 0;
    pointer-events: none;
  }
  .carrier {
    position: absolute;
    inset: 0;
    will-change: transform;
  }
  .cap {
    position: absolute;
    left: -0.55rem;
    top: 50%;
    width: 1.1rem;
    height: 1.7rem;
    margin-top: -0.85rem;
    border-radius: 3px;
    background:
      linear-gradient(90deg, transparent calc(50% - 1px), var(--ink) calc(50% - 1px) calc(50% + 1px), transparent calc(50% + 1px)),
      repeating-linear-gradient(90deg, rgb(255 255 255 / 0.1) 0 1px, transparent 1px 3px),
      linear-gradient(180deg, var(--raised-hi), var(--raised-lo));
  }
  .readout {
    width: 2.9rem;
    height: 1.6rem;
    border-radius: 4px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-display);
    font-weight: 600;
  }
</style>
