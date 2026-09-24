<!--
  One rubber pad, lit exactly like the hardware LED: the engine's `Pad` (colour, level,
  animation) on the beat clock. The LED wash and the glow that bleeds onto the panel are
  pre-rendered layers whose opacity is the brightness, so flashing and pulsing only
  change opacity (compositor-friendly at 60 Hz).
-->
<script lang="ts">
  import { tipFor } from '../../help/actions'
  import { TIPS, keyLabel } from '../../help/tooltips'
  import type { Pad } from '../../lib/api/types'
  import { css, padLight } from '../../lib/leds'
  import { tip } from '../../lib/tooltip/tip.svelte'

  let { pad, beats, onpress, paletteLeds = false }: { pad: Pad; beats: number; onpress: (p: Pad) => void; paletteLeds?: boolean } = $props()

  const light = $derived(padLight(pad, paletteLeds, beats))
  const b = $derived(light.b)
  const key = $derived.by(() => {
    const t = TIPS[tipFor(pad.action)]
    const k = (t.app_keys ?? t.keys)[0]
    return pad.action && k ? keyLabel(k) : ''
  })
  const state = $derived(
    pad.level === 'off' ? 'dark' : pad.anim === 'flash' ? 'queued' : pad.anim === 'pulse' ? 'armed' : pad.level === 'bright' ? 'on' : 'available',
  )
</script>

<button
  type="button"
  class="pad"
  class:lit={b > 0.55}
  class:dark={pad.level === 'off'}
  data-note={pad.note}
  data-level={pad.level}
  data-anim={pad.anim}
  aria-disabled={!pad.action || pad.level === 'off' || undefined}
  style:--led={css(light.rgb)}
  style:--b={b}
  use:tip={tipFor(pad.action)}
  onclick={() => pad.action && onpress(pad)}
>
  <span class="bleed" aria-hidden="true"></span>
  <span class="led" aria-hidden="true"></span>
  <span class="key" aria-hidden="true">{key}</span>
  <span class="label">{pad.label || '—'}<span class="visually-hidden">, {state}</span></span>
</button>

<style>
  .pad {
    position: relative;
    aspect-ratio: 1;
    width: 100%;
    padding: 0;
    border: none;
    border-radius: 0.55em;
    background:
      radial-gradient(120% 100% at 50% 20%, rgb(255 255 255 / 0.07), transparent 60%),
      linear-gradient(180deg, color-mix(in srgb, var(--rubber) 85%, white 8%), var(--rubber));
    box-shadow:
      inset 0 1px 0 rgb(255 255 255 / 0.1),
      inset 0 -0.18em 0.25em rgb(0 0 0 / 0.35),
      0 0.18em 0 rgb(0 0 0 / 0.55),
      0 0.3em 0.5em -0.15em rgb(0 0 0 / 0.6);
    color: #c9cfd6;
    transition: transform 60ms ease-out, box-shadow 60ms ease-out;
  }
  .pad:active {
    transform: translateY(0.12em);
    box-shadow:
      inset 0 0.1em 0.3em rgb(0 0 0 / 0.5),
      0 0.05em 0 rgb(0 0 0 / 0.55);
  }
  /* The glow that spills onto the panel around the pad. */
  .bleed {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    box-shadow: 0 0 1.1em 0.15em var(--led);
    opacity: calc(var(--b) * 0.75);
    /* Its own layer: a pulsing pad then fades a cached blur instead of re-rastering it
       every frame (software raster ran at ~38 fps without this, 60 with it). */
    will-change: opacity;
    pointer-events: none;
  }
  /* The backlit rubber: brightest in the middle, like a diffused LED. */
  .led {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background:
      radial-gradient(90% 90% at 50% 45%, color-mix(in srgb, var(--led) 70%, white 30%), var(--led) 55%, color-mix(in srgb, var(--led) 70%, black 30%));
    opacity: var(--b);
    pointer-events: none;
  }
  .key {
    position: absolute;
    top: 0.35em;
    right: 0.45em;
    font-size: 0.62em;
    font-weight: 600;
    opacity: 0.7;
  }
  .label {
    position: absolute;
    left: 0.3em;
    right: 0.3em;
    bottom: 0.45em;
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 0.7em;
    line-height: 1.05;
    letter-spacing: 0.03em;
    text-align: center;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .lit {
    color: #fff;
  }
  .lit .label,
  .lit .key {
    text-shadow: 0 1px 2px rgb(0 0 0 / 0.7);
  }
  .dark {
    cursor: default;
    color: #6d747d;
  }
</style>
