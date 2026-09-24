<!--
  A vertical fader (0–127) that mirrors a Launchkey fader. Drag, click, scroll, or use the
  arrow keys (±1), PgUp/PgDn (±10), Home/End when it has focus. `pickup` shows the soft
  takeover "waiting" mark: the hardware fader won't move this level until it catches up.
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
    max = 127,
  }: {
    value: number
    tip: TipKey
    /** Short name under the fader, and its accessible name. */
    label: string
    onchange: (v: number) => void
    pickup?: boolean
    /** Dim the fill when the part is off or muted. */
    lit?: boolean
    max?: number
  } = $props()

  let track: HTMLDivElement | undefined = $state()
  let dragging = $state(false)
  const pct = $derived((Math.max(0, Math.min(max, value)) / max) * 100)

  function fromY(y: number) {
    if (!track) return
    const r = track.getBoundingClientRect()
    onchange(Math.round(Math.max(0, Math.min(1, (r.bottom - y) / r.height)) * max))
  }
  function down(e: PointerEvent) {
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
    e.preventDefault()
    onchange(Math.max(0, Math.min(max, value + (e.deltaY < 0 ? 2 : -2))))
  }
</script>

<div class="fader" class:lit>
  <div class="value" aria-hidden="true">{value}</div>
  <div
    class="track"
    role="slider"
    tabindex="0"
    aria-label={label}
    aria-orientation="vertical"
    aria-valuemin={0}
    aria-valuemax={max}
    aria-valuenow={value}
    aria-valuetext={pickup ? `${value}, waiting for the Launchkey fader` : `${value}`}
    bind:this={track}
    use:tipAction={tip}
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={up}
    onkeydown={key}
    onwheel={wheel}
  >
    <div class="fill" style:height="{pct}%"></div>
    <div class="cap" style:bottom="{pct}%"></div>
  </div>
  <div class="pickup" class:on={pickup} use:tipAction={'mixer.pickup'} aria-hidden="true">↕</div>
  <div class="name">{label}</div>
</div>

<style>
  .fader {
    display: grid;
    grid-template-rows: auto 1fr auto auto;
    justify-items: center;
    gap: 0.3rem;
    min-width: 3rem;
    height: 100%;
    min-height: 11rem;
  }
  .value {
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 1.05rem;
    min-width: 2.5ch;
    text-align: center;
  }
  .track {
    position: relative;
    width: 2.25rem;
    border-radius: 4px;
    background: var(--well);
    border: 1px solid var(--line);
    cursor: ns-resize;
    touch-action: none;
  }
  .fill {
    position: absolute;
    left: 50%;
    bottom: 0;
    width: 6px;
    transform: translateX(-50%);
    border-radius: 3px;
    background: var(--muted);
    opacity: 0.45;
  }
  .lit .fill {
    background: var(--accent);
    opacity: 1;
  }
  .cap {
    position: absolute;
    left: 2px;
    right: 2px;
    height: 12px;
    transform: translateY(50%);
    border-radius: 3px;
    background: var(--ink);
    box-shadow: 0 1px 0 var(--line-strong);
  }
  .pickup {
    height: 1.25rem;
    font-weight: 700;
    color: var(--accent);
    visibility: hidden;
  }
  .pickup.on {
    visibility: visible;
  }
  .name {
    font-family: var(--font-display);
    font-weight: 600;
    font-size: var(--fs-small);
    color: var(--muted);
    text-align: center;
    white-space: nowrap;
  }
</style>
