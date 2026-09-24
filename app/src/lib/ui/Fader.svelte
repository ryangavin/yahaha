<!--
  A hardware fader (0–127): a recessed slot with a tick scale, a ribbed cap and a value
  readout. Mirrors a Launchkey fader. Drag, click, scroll, or with focus use the arrows
  (±1), PgUp/PgDn (±10), Home/End. `pickup` shows the soft-takeover mark (↕, amber): the
  hardware fader won't move this level until it catches up. `disabled` draws an unused
  fader (Panel page, faders 5–8): still hoverable for its tooltip.
  The cap moves with `transform`, so dragging and 60 Hz updates stay on the compositor.
-->
<script lang="ts">
  import type { TipKey } from '../../help/tooltips'
  import { tip as tipAction, tips } from '../tooltip/tip.svelte'

  let {
    value,
    tip,
    label,
    onchange,
    pickup = false,
    lit = true,
    disabled = false,
    max = 127,
  }: {
    value: number
    tip: TipKey
    /** Short name under the fader, and its accessible name. */
    label: string
    onchange: (v: number) => void
    pickup?: boolean
    /** Dim the readout when the part is off or muted. */
    lit?: boolean
    disabled?: boolean
    max?: number
  } = $props()

  let slot: HTMLDivElement | undefined = $state()
  let dragging = $state(false)
  const frac = $derived(Math.max(0, Math.min(max, value)) / max)

  function fromY(y: number) {
    if (!slot || disabled) return
    const r = slot.getBoundingClientRect()
    onchange(Math.round(Math.max(0, Math.min(1, (r.bottom - y) / r.height)) * max))
  }
  function down(e: PointerEvent) {
    if (disabled) return
    dragging = true
    ;(e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId)
    fromY(e.clientY)
  }
  function move(e: PointerEvent) {
    if (dragging) {
      fromY(e.clientY)
      tips.refresh()
    }
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
  function wheel(e: WheelEvent) {
    if (disabled) return
    e.preventDefault()
    onchange(Math.max(0, Math.min(max, value + (e.deltaY < 0 ? 2 : -2))))
  }
</script>

<div class="fader" class:lit class:disabled>
  <div class="readout mat-screen" aria-hidden="true">
    <span class="glow-text">{disabled ? '' : value}</span>
    <span class="pickup" class:on={pickup && !disabled} use:tipAction={'mixer.pickup'}>↕</span>
  </div>
  <div
    class="track"
    role="slider"
    tabindex="0"
    aria-label={label}
    aria-orientation="vertical"
    aria-disabled={disabled || undefined}
    aria-valuemin={0}
    aria-valuemax={max}
    aria-valuenow={value}
    aria-valuetext={disabled ? 'unused' : pickup ? `${value}, waiting for the Launchkey fader` : `${value}`}
    use:tipAction={tip}
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={up}
    onkeydown={key}
    onwheel={wheel}
  >
    <div class="ticks" aria-hidden="true"></div>
    <div class="slot mat-well" bind:this={slot}></div>
    <div class="cap-rail">
      <div class="carrier" style:transform="translateY({(1 - frac) * 100}%)"><div class="cap mat-raised"></div></div>
    </div>
  </div>
  <div class="name engraved">{label}</div>
</div>

<style>
  .fader {
    display: grid;
    grid-template-rows: auto 1fr auto;
    justify-items: center;
    gap: 0.4em;
    min-width: 0;
    height: 100%;
  }
  .readout {
    position: relative;
    width: 2.9em;
    height: 1.45em;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.95em;
  }
  .fader:not(.lit) .readout {
    color: var(--screen-dim);
  }
  .pickup {
    position: absolute;
    right: -0.15em;
    top: -0.55em;
    font-size: 0.8em;
    font-weight: 700;
    color: var(--accent-ink);
    background: var(--accent);
    border-radius: 3px;
    padding: 0 0.2em;
    line-height: 1.1;
    visibility: hidden;
    box-shadow: 0 0 8px color-mix(in srgb, var(--accent) 70%, transparent);
  }
  .pickup.on {
    visibility: visible;
  }
  .track {
    position: relative;
    width: 2.6em;
    min-height: 8em;
    height: 100%;
    cursor: ns-resize;
    touch-action: none;
    border-radius: 4px;
  }
  .disabled .track {
    cursor: default;
  }
  /* Tick scale either side of the slot. */
  .ticks {
    position: absolute;
    inset: 0.55em 0.2em;
    background:
      repeating-linear-gradient(180deg, var(--engrave) 0 1px, transparent 1px 12.5%) left / 5px 100% no-repeat,
      repeating-linear-gradient(180deg, var(--engrave) 0 1px, transparent 1px 12.5%) right / 5px 100% no-repeat;
    opacity: 0.45;
  }
  .slot {
    position: absolute;
    left: 50%;
    top: 0.55em;
    bottom: 0.55em;
    width: 6px;
    transform: translateX(-50%);
    border-radius: 3px;
  }
  /* The cap rides a full-height carrier moved with transform (a % of the carrier is a %
     of the travel), so the cap never triggers layout while it moves. */
  .cap-rail {
    position: absolute;
    left: 0.35em;
    right: 0.35em;
    top: 0;
    bottom: 1.1em;
  }
  .carrier {
    position: absolute;
    inset: 0;
    will-change: transform;
  }
  .cap {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    height: 1.1em;
    border-radius: 3px;
    background:
      linear-gradient(180deg, transparent calc(50% - 1px), var(--ink) calc(50% - 1px) calc(50% + 1px), transparent calc(50% + 1px)),
      repeating-linear-gradient(180deg, rgb(255 255 255 / 0.1) 0 1px, transparent 1px 3px),
      linear-gradient(180deg, var(--raised-hi), var(--raised-lo));
  }
  .disabled .cap {
    opacity: 0.35;
  }
  .name {
    white-space: nowrap;
    max-width: 4.2em;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .disabled .name,
  .disabled .readout {
    opacity: 0.45;
  }
</style>
