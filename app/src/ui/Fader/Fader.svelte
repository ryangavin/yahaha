<!--
  Fader: one strip's 252px fader button, Push's mix meter. The level value sits at the top in the
  strip's hue; two meter bars and a peak line show the sound; a 3px bracket in the hue with a cap
  tick shows the set level and outranks the meter. When the hardware fader is away from the set
  level, a dashed ghost line marks where it is and ↕ says "move it through". Kinds: `part`,
  `group` and `master` are live; `off` dims the bracket and drops the meter; `parked` (an unused
  fader) draws a dashed groove only. `layered` is the non-Vol layer look: no meters, a white
  bracket, the value carrying the layer word. Fully controlled: a drag or an arrow key asks for a
  level through `onlevel`; it moves nothing itself.
-->
<script lang="ts">
  import type { Action } from 'svelte/action'

  type Hue = 'r1' | 'r2' | 'r3' | 'l' | 'a' | 't2' | 't'

  type Props = {
    /** The accessible name ("Right 1 · Stage Grand, level 90"). */
    name: string
    /** The text at the top: the level ("90") or the layer and its value ("Rev 40"). Empty when parked. */
    value?: string
    /** The set level, 0–127: where the bracket's cap sits. */
    level?: number
    /** The left meter, 0–1 of the travel. */
    meter?: number
    /** The right meter, 0–1 of the travel. */
    meter2?: number
    /** The peak hold line, 0–1 of the travel. */
    peak?: number
    /** Where the hardware fader is, 0–127, while it's away from the level (soft takeover). Undefined: not away. */
    away?: number
    /** `part`, `group`, `master` live; `off` a part switched off; `parked` an unused fader. */
    kind?: 'part' | 'group' | 'master' | 'off' | 'parked'
    /** The strip's colour token (without `--`): a part hue, `a` for Style, `t2` for Multi Pad, `t` for Master. */
    hue?: Hue
    /** The non-Vol layer look (Pan, Reverb, Chorus, Delay): meters hidden, bracket and value white, no glow. */
    layered?: boolean
    /** A fixed width in px. Default: fills its container. */
    width?: number
    /** The tooltip key, rendered as `data-tip`. */
    tip?: string
    /** The app's `use:tip` action, applied with `tip` when both are set. */
    tipAction?: Action<HTMLElement, string>
    /** Called with the level asked for (0–127) on a drag along the track, or ↑ / ↓ (±1). Not when parked. */
    onlevel?: (level: number) => void
  }

  let {
    name,
    value = '',
    level = 0,
    meter = 0,
    meter2 = 0,
    peak = 0,
    away,
    kind = 'part',
    hue = 't',
    layered = false,
    width,
    tip,
    tipAction,
    onlevel,
  }: Props = $props()

  const MAX = 127
  const clamp = (n: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, n))

  let parked = $derived(kind === 'parked')
  let off = $derived(kind === 'off')
  let live = $derived(!parked && !off)
  let showMeters = $derived(!parked && !layered)
  let ink = $derived(layered ? 't' : hue)
  let frac = $derived(clamp(level, 0, MAX) / MAX)

  let track: HTMLSpanElement | undefined = $state()
  let dragId: number | null = null

  function levelAt(clientY: number) {
    if (!track) return level
    const box = track.getBoundingClientRect()
    if (box.height === 0) return level
    return Math.round(clamp(1 - (clientY - box.top) / box.height, 0, 1) * MAX)
  }

  function pointerdown(event: PointerEvent) {
    if (parked || event.button !== 0) return
    dragId = event.pointerId
    try {
      ;(event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId)
    } catch {
      // jsdom and synthetic events have no pointer capture.
    }
    onlevel?.(levelAt(event.clientY))
  }

  function pointermove(event: PointerEvent) {
    if (dragId === event.pointerId) onlevel?.(levelAt(event.clientY))
  }

  function pointerend(event: PointerEvent) {
    if (dragId === event.pointerId) dragId = null
  }

  function keydown(event: KeyboardEvent) {
    if (parked) return
    const step = event.key === 'ArrowUp' ? 1 : event.key === 'ArrowDown' ? -1 : 0
    if (step === 0) return
    event.preventDefault()
    onlevel?.(clamp(level + step, 0, MAX))
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
  class="fader"
  class:parked
  class:off
  class:live
  class:layered
  style:width={width === undefined ? undefined : `${width}px`}
  style:--hue="var(--{ink})"
  style:--glow={live && !layered ? `var(--fader-glow-${hue})` : 'none'}
  style:--level={frac}
  data-kind={kind}
  data-hue={ink}
  data-tip={tip}
  aria-label={name}
  aria-disabled={parked ? 'true' : undefined}
  onpointerdown={pointerdown}
  onpointermove={pointermove}
  onpointerup={pointerend}
  onpointercancel={pointerend}
  onkeydown={keydown}
  use:tipped={tip}
>
  <span class="value" aria-hidden="true">{parked ? '' : value}</span>
  <span class="track" bind:this={track} aria-hidden="true">
    {#if showMeters}
      <span class="meter-bg m1"></span>
      <span class="meter-bg m2"></span>
      {#if live}
        <span class="meter m1" style:--m={clamp(meter, 0, 1)}></span>
        <span class="meter m2" style:--m={clamp(meter2, 0, 1)}></span>
      {/if}
      <span class="peak" style:--p={clamp(peak, 0, 1)}></span>
    {/if}
    <span class="groove"></span>
    {#if !parked}
      <span class="fill"></span>
      <span class="cap"></span>
    {/if}
    {#if away !== undefined && !parked}
      <span class="ghost" style:--away={clamp(away, 0, MAX) / MAX}></span>
    {/if}
  </span>
  {#if away !== undefined && !parked}<span class="wait" aria-hidden="true">↕</span>{/if}
</button>

<style>
  .fader {
    position: relative;
    display: block;
    flex: none;
    box-sizing: border-box;
    width: 100%;
    min-width: 0;
    height: var(--fader-height);
    margin: 0;
    padding: 0;
    border: 0;
    border-radius: 0;
    background: none;
    font-family: var(--font-sans);
    font-variant-numeric: tabular-nums;
    cursor: ns-resize;
    touch-action: none;
  }
  .parked {
    cursor: default;
  }
  .value {
    position: absolute;
    top: 0;
    right: 0;
    left: 0;
    height: var(--fader-value-height);
    line-height: var(--fader-value-height);
    color: var(--hue);
    font-size: var(--text-18);
    font-weight: var(--weight-light);
    text-align: center;
    white-space: nowrap;
  }
  .off .value {
    color: var(--d);
  }
  .track {
    position: absolute;
    top: var(--fader-travel-top);
    right: 0;
    bottom: var(--fader-travel-bottom);
    left: 0;
  }
  .meter-bg,
  .meter {
    position: absolute;
    bottom: 0;
    width: var(--fader-meter-width);
  }
  .meter-bg {
    top: 0;
    background: var(--mbg);
  }
  .m1 {
    left: calc(50% + var(--fader-meter-left));
  }
  .m2 {
    left: calc(50% + var(--fader-meter2-left));
  }
  .meter {
    height: calc(var(--m) * 100%);
    background: var(--hue);
    opacity: var(--fader-meter-opacity);
  }
  .peak {
    position: absolute;
    bottom: calc(var(--p) * 100%);
    left: calc(50% + var(--fader-meter-left));
    width: var(--fader-peak-width);
    height: var(--line-width);
    background: var(--fader-peak);
  }
  .groove,
  .fill {
    position: absolute;
    bottom: 0;
    left: calc(50% + var(--fader-bracket-left));
    width: var(--fader-bracket-width);
  }
  .groove {
    top: 0;
    background: var(--fader-track);
  }
  .parked .groove {
    background: repeating-linear-gradient(
      to bottom,
      var(--line) 0 var(--fader-parked-dash),
      transparent var(--fader-parked-dash) var(--fader-parked-gap)
    );
  }
  .fill {
    height: calc(var(--level) * 100%);
    background: var(--hue);
    box-shadow: var(--glow);
  }
  .cap {
    position: absolute;
    top: calc((1 - var(--level)) * 100% - var(--fader-cap-lift));
    left: calc(50% + var(--fader-cap-left));
    width: var(--fader-cap-width);
    height: var(--fader-bracket-width);
    background: var(--hue);
  }
  .off .fill,
  .off .cap {
    opacity: var(--fader-off-opacity);
  }
  .ghost {
    position: absolute;
    top: calc((1 - var(--away)) * 100%);
    left: calc(50% + var(--fader-ghost-left));
    width: var(--fader-ghost-width);
    height: 0;
    border-top: var(--line-width) dashed var(--m);
  }
  .wait {
    position: absolute;
    top: 0;
    left: 0;
    color: var(--m);
    font-size: var(--text-13);
    line-height: var(--text-13);
  }
  .fader:focus-visible {
    outline: var(--line-width) solid var(--focus);
    outline-offset: var(--focus-offset);
  }
</style>
