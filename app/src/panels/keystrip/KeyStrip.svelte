<!--
  The keyboard strip under the Launchkey mirror: the keys you hold, coloured by the part
  that sounds them; the split point (style) and the Left split; the chord-detection area;
  and the recognised chord's tones. 49 or 61 keys like the Launchkey, or 88.

   ┌ cheek ─────┬ rail: [chord detection · Lower ░░░░░░░]  Split F#2 ▾ ─────────────────┐
   │ Am7        │ ┌──┬┬─┬┬──┬──┬┬─┬┬─┬┬──┬ keys ─────────────────────────────────────┐ │
   │ A C E G    │ │  ││ ││  │  ││ ││ ││  │   held keys lit in their part's colour     │ │
   │ [49 61 88] │ │ ●│  │ ● │  ●  │  ◉ │   ● chord tone  ◉ bass                     │ │
   └────────────┴─┴──┴──┴───┴─────┴────┴──────────────────────────────────────────┴─┘

  Held keys, chord tones and the detection area come from the engine's `state.keyboard`.
  Everything is percentage-positioned boxes: a key only changes a
  class and one custom property when it's pressed, so 60 Hz updates stay cheap.

  The shell can put a row above all this, in the same panel (`children`): App.svelte puts
  the Registration bar there, above the keys, as the Genos has its Registration buttons.
-->
<script lang="ts">
  import type { Snippet } from 'svelte'
  import { app, ui, type KeyRange } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import type { HeldNote } from '../../lib/api/types'
  import { RANGES, boundary, detectionArea, heldFill, layout, noteAt, noteName, pcName, rangeFor } from './keyboard'

  const s = $derived(app.state)
  const c = $derived(s.chord)
  const kb = $derived(s.keyboard)
  const size = $derived(rangeFor(ui.keyRange, s.io.inputs))
  const range = $derived(RANGES[size])
  const keys = $derived(layout(range))
  const whites = $derived(keys.filter((k) => !k.black))
  const blacks = $derived(keys.filter((k) => k.black))

  const held = $derived(new Map<number, HeldNote>((kb?.held ?? []).map((h) => [h.note, h])))
  const below = $derived((kb?.held ?? []).filter((h) => h.note < range[0]).length)
  const above = $derived((kb?.held ?? []).filter((h) => h.note > range[1]).length)
  const tones = $derived(new Set(kb?.chordTones ?? []))
  const bass = $derived(kb?.chordBass ?? null)

  // An engine older than `keyboard` doesn't say where detection listens: no area.
  const area = $derived(kb ? detectionArea(kb.detection, range) : null)
  const areaX = $derived(area ? [boundary(keys, area[0] - 1), boundary(keys, area[1])] : null)
  const areaName = $derived(
    c.upper ? 'Upper' : c.fingering === 'fullKeyboard' || c.fingering === 'aiFullKeyboard' ? 'Full Keyboard' : 'Lower',
  )
  const splitX = $derived(boundary(keys, c.split))
  const leftSplit = $derived(kb?.leftSplit ?? c.split)
  const leftX = $derived(boundary(keys, leftSplit))

  const pct = (x: number) => `${(x * 100).toFixed(3)}%`
  const pc = (n: number) => ((n % 12) + 12) % 12

  // ── The split marker: drag it, or arrows with focus. It sends the engine's own commands. ──
  let bed: HTMLDivElement | undefined = $state()
  let dragging = false
  function dragTo(clientX: number) {
    if (!bed) return
    const r = bed.getBoundingClientRect()
    const note = noteAt(keys, (clientX - r.left) / r.width)
    if (note !== c.split) app.send({ type: 'setSplit', note })
  }
  function down(e: PointerEvent) {
    dragging = true
    ;(e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId)
  }
  function move(e: PointerEvent) {
    if (dragging) dragTo(e.clientX)
  }
  function up() {
    dragging = false
  }
  function key(e: KeyboardEvent) {
    const d = e.key === 'ArrowLeft' || e.key === 'ArrowDown' ? -1 : e.key === 'ArrowRight' || e.key === 'ArrowUp' ? 1 : 0
    if (!d) return
    e.preventDefault()
    e.stopPropagation()
    app.send({ type: 'moveSplit', delta: d })
  }

  const SIZES: KeyRange[] = [49, 61, 88]

  /** A row above the keys, across the whole strip (App.svelte puts the Registration bar here). */
  let { children }: { children?: Snippet } = $props()
</script>

<section class="strip mat-chassis" class:has-top={!!children} aria-label="Keyboard">
  <!-- Above the keys, in the same panel: what the shell puts here (the Registration bar). -->
  {#if children}<div class="top">{@render children()}</div>{/if}

  <div class="cheek">
    <span class="engraved">Chord tones</span>
    <span class="tones" aria-live="off">
      {#if !kb}
        <!-- The engine doesn't report held keys or chord tones yet: say so, not "none". -->
        <span class="none unreported">Not reported yet</span>
      {:else if kb.chordTones.length}
        {#each kb.chordTones as t, i (i)}<span class:root={i === 0} class:bass={t === bass}>{pcName(t)}</span>{/each}
      {:else}
        <span class="none">–</span>
      {/if}
    </span>
    <div class="sizes" role="group" aria-label="Keyboard size">
      {#each SIZES as n (n)}
        <button
          type="button"
          class="size"
          class:on={size === n}
          class:chosen={ui.keyRange === n}
          aria-pressed={size === n}
          use:tip={'keystrip.range'}
          onclick={() => ui.setKeyRange(ui.keyRange === n ? null : n)}>{n}</button
        >
      {/each}
    </div>
  </div>

  <div class="board">
    <div class="rail">
      {#if areaX}
        <div class="area" class:upper={c.upper} style:left={pct(areaX[0])} style:width={pct(areaX[1] - areaX[0])}>
          <span>Chord detection · {areaName}</span>
        </div>
      {/if}
      {#if below}<span class="edge lo">◀ {below}</span>{/if}
      {#if above}<span class="edge hi">{above} ▶</span>{/if}
    </div>

    <!-- svelte-ignore a11y_no_noninteractive_tabindex (focusable so its tooltip, the colour legend, is reachable from the keyboard) -->
    <div class="bed mat-well" bind:this={bed} tabindex="0" role="img" aria-label={kb ? `Keys held: ${kb.held.map((h) => noteName(h.note)).join(' ') || 'none'}` : 'Keys held: not reported by the engine yet'} use:tip={'keystrip.keys'}>
      {#each whites as k (k.note)}
        {@const h = held.get(k.note)}
        <div class="key white" data-note={k.note} class:held={h} style:left={pct(k.x)} style:width={pct(k.w)} style:--fill={h ? heldFill(h) : null}>
          {#if tones.has(pc(k.note))}<span class="dot" class:bass={pc(k.note) === bass}></span>{/if}
          {#if pc(k.note) === 0}<span class="oct">C{Math.floor(k.note / 12) - 2}</span>{/if}
        </div>
      {/each}
      {#each blacks as k (k.note)}
        {@const h = held.get(k.note)}
        <div class="key black" data-note={k.note} class:held={h} style:left={pct(k.x)} style:width={pct(k.w)} style:--fill={h ? heldFill(h) : null}>
          {#if tones.has(pc(k.note))}<span class="dot" class:bass={pc(k.note) === bass}></span>{/if}
        </div>
      {/each}
    </div>

    {#if leftSplit !== c.split}
      <div class="marker left" style:left={pct(leftX)} aria-hidden="true"><span class="tag">Left {noteName(leftSplit)}</span></div>
    {/if}
    <div
      class="marker split"
      style:left={pct(splitX)}
      role="slider"
      tabindex="0"
      aria-label="Split point"
      aria-valuemin={24}
      aria-valuemax={96}
      aria-valuenow={c.split}
      aria-valuetext={c.splitName}
      use:tip={'keystrip.split'}
      onpointerdown={down}
      onpointermove={move}
      onpointerup={up}
      onpointercancel={up}
      onkeydown={key}
    >
      <span class="tag">Split {c.splitName}{leftSplit === c.split ? '' : ' · Style'}</span>
    </div>
  </div>
</section>

<style>
  .strip {
    display: grid;
    grid-template-columns: 8.5em minmax(0, 1fr);
    gap: 1em;
    height: 100%;
    padding: 0.7em 1.5em 0.9em;
    border-radius: 1em;
  }
  .strip.has-top {
    grid-template-rows: auto minmax(0, 1fr);
    row-gap: 0.6em;
  }
  .top {
    grid-column: 1 / -1;
    min-width: 0;
    padding-bottom: 0.6em;
    border-bottom: 1px solid var(--seam);
    box-shadow: 0 1px 0 rgb(255 255 255 / 0.04);
  }
  .strip :global(.engraved) {
    font-size: 0.78em;
  }

  /* The left cheek, where the wheels are on the hardware: chord tones and the size switch. */
  .cheek {
    display: flex;
    flex-direction: column;
    gap: 0.35em;
    min-width: 0;
    padding-right: 1em;
    border-right: 1px solid var(--seam);
    box-shadow: 1px 0 0 rgb(255 255 255 / 0.04);
  }
  .tones {
    display: flex;
    flex-wrap: wrap;
    gap: 0.1em 0.45em;
    min-height: 1.4em;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 1.15em;
    color: var(--ink);
  }
  .tones .root {
    color: var(--part-chord-hi);
  }
  .tones .bass {
    text-decoration: underline;
    text-underline-offset: 0.2em;
  }
  .none {
    color: var(--muted);
  }
  .unreported {
    font-family: var(--font-body);
    font-weight: 500;
    font-size: 0.62em;
    line-height: 1.2;
  }
  .sizes {
    display: flex;
    gap: 0.25em;
    margin-top: auto;
  }
  .size {
    flex: 1;
    min-width: 0;
    height: 2.1em;
    padding: 0;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.85em;
    color: var(--engrave);
    background: linear-gradient(180deg, var(--raised-hi), var(--raised-lo));
    border: 1px solid rgb(0 0 0 / 0.45);
    border-radius: 0.3em;
    box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.14);
  }
  .size.on {
    color: var(--ink);
    box-shadow: inset 0 -2px 0 var(--engrave), inset 0 1px 0 rgb(255 255 255 / 0.14);
  }
  .size.chosen {
    box-shadow: inset 0 -2px 0 var(--accent), inset 0 1px 0 rgb(255 255 255 / 0.14);
  }

  /* The keys, with the rail above them; markers span both. */
  .board {
    position: relative;
    display: grid;
    grid-template-rows: 1.9em minmax(0, 1fr);
    gap: 0.3em;
    min-width: 0;
    min-height: 0;
  }
  .rail {
    position: relative;
  }
  .area {
    position: absolute;
    top: 0.15em;
    bottom: 0.15em;
    display: flex;
    align-items: center;
    padding: 0 0.6em;
    border-radius: 0.3em;
    background: color-mix(in srgb, var(--part-chord) 22%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--part-chord) 45%, transparent);
    overflow: hidden;
  }
  .area span {
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.8em;
    letter-spacing: 0.05em;
    color: var(--ink);
    white-space: nowrap;
  }
  /* Upper: the split tag sits at the band's left end. */
  .area.upper {
    padding-left: 7em;
  }
  .edge {
    position: absolute;
    top: 0.3em;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.8em;
    color: var(--accent);
  }
  .edge.lo {
    left: 0;
  }
  .edge.hi {
    right: 0;
  }

  .bed {
    position: relative;
    border-radius: 0.35em;
    padding: 0;
    overflow: hidden;
    outline-offset: 3px;
  }
  .key {
    position: absolute;
    top: 0;
  }
  .white {
    bottom: 0;
    border-right: 1px solid var(--key-gap);
    border-radius: 0 0 0.3em 0.3em;
    background: linear-gradient(180deg, var(--key-white-lo), var(--key-white) 12%, var(--key-white) 88%, var(--key-white-lo));
    box-shadow: inset 0 -0.25em 0 var(--key-white-lo);
  }
  .black {
    z-index: 2;
    height: 60%;
    border-radius: 0 0 0.22em 0.22em;
    background: linear-gradient(180deg, var(--key-black), var(--key-black-hi) 85%, var(--key-black));
    box-shadow:
      inset 0 -0.3em 0 rgb(255 255 255 / 0.08),
      0 0.15em 0.2em rgb(0 0 0 / 0.5);
  }
  /* A held key: its part colour over the key (bands for layered parts). */
  .key::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: var(--fill, transparent);
    opacity: 0;
    pointer-events: none;
  }
  .held::after {
    opacity: 0.88;
  }
  .white.held {
    box-shadow: inset 0 -0.1em 0 var(--key-white-lo);
  }
  .dot {
    position: absolute;
    z-index: 1;
    left: 50%;
    bottom: 1.7em;
    width: 0.55em;
    height: 0.55em;
    margin-left: -0.275em;
    border-radius: 50%;
    /* Dark on the white keys, light on the black ones, in both themes. */
    --dot: #22323f;
    background: var(--dot);
  }
  .black .dot {
    --dot: #dff4ff;
    bottom: 0.5em;
  }
  .dot.bass {
    background: transparent;
    box-shadow: 0 0 0 0.14em var(--dot);
  }
  .oct {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0.45em;
    text-align: center;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.62em;
    color: var(--key-print);
  }

  .marker {
    position: absolute;
    top: 0;
    bottom: 0;
    z-index: 3;
    width: 0;
  }
  .marker::before {
    content: '';
    position: absolute;
    top: 1.9em;
    bottom: 0;
    left: -1px;
    width: 2px;
    background: var(--accent);
    box-shadow: 0 0 0.4em color-mix(in srgb, var(--accent) 60%, transparent);
  }
  .marker.left::before {
    background: repeating-linear-gradient(180deg, var(--part-left) 0 0.4em, transparent 0.4em 0.7em);
    box-shadow: none;
  }
  .tag {
    position: absolute;
    top: 0.15em;
    left: 0.35em;
    padding: 0.1em 0.45em;
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 0.8em;
    white-space: nowrap;
    color: var(--accent-ink);
    background: var(--accent);
    border-radius: 0.25em;
  }
  .marker.left .tag {
    left: auto;
    right: 0.35em;
    color: #1b1206;
    background: var(--part-left);
  }
  .split:focus-visible {
    outline: none;
  }
  .split:focus-visible .tag {
    outline: 2px solid var(--ink);
    outline-offset: 1px;
  }
  .split {
    cursor: ew-resize;
    touch-action: none;
  }
  /* A wider grab area than the 2px line. */
  .split::after {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    left: -0.6em;
    width: 5.5em;
  }
</style>
