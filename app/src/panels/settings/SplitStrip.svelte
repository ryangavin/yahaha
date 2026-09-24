<!--
  The split point on a mini keyboard, C0–C6 (the engine's 24–96 range). The shaded left
  part is Left and the chord section; the amber marker sits on the split key's right edge.
  Drag the marker or click a key; with focus, ←/→ move one key, PgUp/PgDn an octave,
  Home/End the ends. Each change is sent at once (`setSplit`), and the strip always
  draws the engine's split. The marker and the shading move with `transform` only.
-->
<script lang="ts">
  import { tip, tips } from '../../lib/tooltip/tip.svelte'
  import { BLACK_H, BLACK_W, keyAt, keysBetween, noteName, SPLIT_MAX, SPLIT_MIN } from './notes'

  let { split, onchange }: { split: number; onchange: (note: number) => void } = $props()

  const keys = keysBetween(SPLIT_MIN, SPLIT_MAX)
  const whites = keys.filter((k) => !k.black)
  const blacks = keys.filter((k) => k.black)
  const W = whites.length
  const MIDDLE_C = 60

  let strip: HTMLDivElement | undefined = $state()
  let dragging = $state(false)

  /** The marker's place, in white-key units: the split key's right edge. */
  const edge = $derived.by(() => {
    const k = keys.find((x) => x.note === split) ?? keys[0]
    return k.black ? k.slot + 1 + BLACK_W / 2 : k.slot + 1
  })
  const frac = $derived(edge / W)

  function at(e: PointerEvent) {
    if (!strip) return
    const r = strip.getBoundingClientRect()
    const x = ((e.clientX - r.left) / r.width) * W
    const y = (e.clientY - r.top) / r.height
    // While dragging, follow the pointer along the keys whatever its height.
    const n = keyAt(keys, x, dragging && y > 1 ? 1 : y)
    if (n !== split) onchange(n)
  }
  function down(e: PointerEvent) {
    dragging = true
    ;(e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId)
    at(e)
  }
  function move(e: PointerEvent) {
    if (!dragging) return
    at(e)
    tips.refresh()
  }
  function up() {
    dragging = false
  }
  function key(e: KeyboardEvent) {
    const step: Record<string, number> = { ArrowRight: 1, ArrowUp: 1, ArrowLeft: -1, ArrowDown: -1, PageUp: 12, PageDown: -12 }
    let n: number | null = null
    if (e.key in step) n = split + step[e.key]
    else if (e.key === 'Home') n = SPLIT_MIN
    else if (e.key === 'End') n = SPLIT_MAX
    if (n === null) return
    e.preventDefault()
    e.stopPropagation()
    onchange(Math.max(SPLIT_MIN, Math.min(SPLIT_MAX, n)))
  }
</script>

<div class="wrap">
  <div
    class="strip mat-well"
    class:dragging
    bind:this={strip}
    role="slider"
    tabindex="0"
    aria-label="Split point"
    aria-valuemin={SPLIT_MIN}
    aria-valuemax={SPLIT_MAX}
    aria-valuenow={split}
    aria-valuetext={noteName(split)}
    use:tip={'settings.split_strip'}
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={up}
    onkeydown={key}
  >
    <div class="keys" aria-hidden="true">
      {#each whites as k (k.note)}
        <div class="white" style:left="{(k.slot / W) * 100}%" style:width="{100 / W}%"></div>
      {/each}
      {#each blacks as k (k.note)}
        <div
          class="black"
          style:left="{((k.slot + 1 - BLACK_W / 2) / W) * 100}%"
          style:width="{(BLACK_W / W) * 100}%"
          style:height="{BLACK_H * 100}%"
        ></div>
      {/each}
      <div class="zone" style:transform="scaleX({frac})"></div>
    </div>
    <div class="rail" aria-hidden="true">
      <div class="carrier" style:transform="translateX({frac * 100}%)">
        <div class="marker"><span class="flag">{noteName(split)}</span></div>
      </div>
    </div>
  </div>
  <div class="octaves" aria-hidden="true">
    {#each whites.filter((k) => k.note % 12 === 0) as k (k.note)}
      <span class:mid={k.note === MIDDLE_C} style:left="{((k.slot + 0.5) / W) * 100}%">{noteName(k.note)}</span>
    {/each}
  </div>
  <div class="legend" aria-hidden="true">
    <span><i class="sw left"></i>Left + chord section</span>
    <span><i class="sw right"></i>Right 1–3</span>
    <span class="mc"><i class="dot"></i>C3 = middle C</span>
  </div>
</div>

<style>
  .wrap {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    padding-top: 1.7rem;
  }
  .strip {
    position: relative;
    height: 4.2rem;
    border-radius: 5px;
    padding: 3px;
    cursor: ew-resize;
    touch-action: none;
  }
  .keys {
    position: absolute;
    inset: 3px;
    overflow: hidden;
    border-radius: 3px;
  }
  .white {
    position: absolute;
    top: 0;
    bottom: 0;
    background: linear-gradient(180deg, #d9dde1, #f4f5f6 70%, #e2e5e8);
    border-right: 1px solid #8c939b;
    border-radius: 0 0 2px 2px;
  }
  .black {
    position: absolute;
    top: 0;
    background: linear-gradient(180deg, #121417, #2a2e34 85%, #3a3f46);
    border-radius: 0 0 2px 2px;
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.5);
  }
  /* The left hand's keys: an amber wash from the left edge to the marker. */
  .zone {
    position: absolute;
    inset: 0;
    transform-origin: left center;
    background: color-mix(in srgb, var(--accent) 34%, transparent);
    mix-blend-mode: multiply;
    pointer-events: none;
    will-change: transform;
  }
  .rail {
    position: absolute;
    inset: 3px;
    pointer-events: none;
  }
  .carrier {
    position: absolute;
    inset: 0;
    will-change: transform;
  }
  .marker {
    position: absolute;
    left: -1.5px;
    top: -0.45rem;
    bottom: -0.25rem;
    width: 3px;
    border-radius: 2px;
    background: var(--accent);
    box-shadow: 0 0 6px color-mix(in srgb, var(--accent) 70%, transparent);
  }
  .flag {
    position: absolute;
    bottom: 100%;
    left: 50%;
    transform: translate(-50%, -2px);
    padding: 0.05rem 0.4rem;
    border-radius: 3px;
    background: var(--accent);
    color: var(--accent-ink);
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 0.9rem;
    line-height: 1.25;
    white-space: nowrap;
  }
  .dragging .flag {
    transform: translate(-50%, -2px) scale(1.08);
  }
  .octaves {
    position: relative;
    height: 1rem;
    margin: 0 3px;
  }
  .octaves span {
    position: absolute;
    transform: translateX(-50%);
    font-family: var(--font-display);
    font-size: 0.75rem;
    color: var(--muted);
  }
  .octaves .mid {
    color: var(--ink);
    font-weight: 700;
  }
  .octaves .mid::after {
    content: '';
    position: absolute;
    left: 50%;
    bottom: -0.3rem;
    width: 4px;
    height: 4px;
    margin-left: -2px;
    border-radius: 50%;
    background: var(--accent);
  }
  .legend {
    display: flex;
    gap: 1rem;
    font-size: var(--fs-small);
    color: var(--muted);
  }
  .sw {
    display: inline-block;
    width: 0.8em;
    height: 0.8em;
    margin-right: 0.35em;
    border-radius: 2px;
    vertical-align: -0.05em;
    border: 1px solid var(--line-strong);
  }
  .sw.left {
    background: color-mix(in srgb, var(--accent) 45%, #f4f5f6);
  }
  .mc {
    margin-left: auto;
  }
  .dot {
    display: inline-block;
    width: 4px;
    height: 4px;
    margin-right: 0.35em;
    border-radius: 50%;
    background: var(--accent);
    vertical-align: 0.15em;
  }
  .sw.right {
    background: #f4f5f6;
  }
</style>
