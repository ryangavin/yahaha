<!--
  One Multi Pad: a rubber pad lit like the Genos MULTI PAD lamp (blue: data, red: playing,
  flashing red: Synchro Start standby, flashing amber: waiting for the bar line, dark:
  empty), on the beat clock. Only opacity changes from frame to frame.
-->
<script lang="ts">
  import type { MultiPadPad } from '../../lib/api/types'
  import { css } from '../../lib/leds'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import { lampLight, LAMP_TEXT } from './multipad'

  let { pad, beats, keyLabel, onpress }: { pad: MultiPadPad; beats: number; keyLabel: string; onpress: () => void } = $props()

  const light = $derived(lampLight(pad.lamp, beats))
</script>

<button
  type="button"
  class="pad"
  class:lit={light.b > 0.55}
  class:dark={pad.lamp === 'empty'}
  data-lamp={pad.lamp}
  aria-disabled={pad.lamp === 'empty' || undefined}
  style:--led={css(light.rgb)}
  style:--b={light.b}
  use:tip={'multipad.pad'}
  onclick={() => pad.lamp !== 'empty' && onpress()}
>
  <span class="bleed" aria-hidden="true"></span>
  <span class="led" aria-hidden="true"></span>
  <span class="num" aria-hidden="true">{pad.index + 1}</span>
  <span class="key" aria-hidden="true">{keyLabel}</span>
  <span class="label">{pad.name || '—'}<span class="visually-hidden">, {LAMP_TEXT[pad.lamp]}</span></span>
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
    font-size: 1rem;
    transition: transform 60ms ease-out;
  }
  .pad:active {
    transform: translateY(0.12em);
  }
  .bleed {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    box-shadow: 0 0 1.1em 0.15em var(--led);
    opacity: calc(var(--b) * 0.75);
    will-change: opacity;
    pointer-events: none;
  }
  .led {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: radial-gradient(90% 90% at 50% 45%, color-mix(in srgb, var(--led) 70%, white 30%), var(--led) 55%, color-mix(in srgb, var(--led) 70%, black 30%));
    opacity: var(--b);
    pointer-events: none;
  }
  .num,
  .key {
    position: absolute;
    top: 0.35em;
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 0.75em;
    opacity: 0.75;
  }
  .num {
    left: 0.5em;
  }
  .key {
    right: 0.5em;
  }
  .label {
    position: absolute;
    left: 0.3em;
    right: 0.3em;
    bottom: 0.5em;
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 0.82em;
    line-height: 1.05;
    text-align: center;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .lit {
    color: #fff;
  }
  .lit .label,
  .lit .key,
  .lit .num {
    text-shadow: 0 1px 2px rgb(0 0 0 / 0.7);
  }
  .dark {
    cursor: default;
    color: #6d747d;
  }
</style>
