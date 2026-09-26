<!--
  A backlit hardware button (Launchkey nav/transport/fader buttons, and any button that
  should look physical). The LED follows an engine-style look (`rgb` 0–127, `level`,
  `anim`) on the beat clock, so it flashes and pulses like the real one. Only opacity
  changes from frame to frame, so it stays cheap at 60 Hz.
-->
<script lang="ts">
  import type { Snippet } from 'svelte'
  import type { TipKey } from '../../help/tooltips'
  import type { Pad } from '../api/types'
  import { brightness, css } from '../leds'
  import { tip as tipAction } from '../tooltip/tip.svelte'

  let {
    tip,
    onclick,
    children,
    led = null,
    beats = 0,
    caption,
    label,
    shape = 'rect',
    pressed,
    width,
    onhold,
  }: {
    tip: TipKey
    onclick: () => void
    children: Snippet
    /** The button's light; null for an unlit button. */
    led?: Pick<Pad, 'rgb' | 'level' | 'anim'> | null
    beats?: number
    /** Engraved text under the button (e.g. the neighbouring style's name). */
    caption?: string
    /** Accessible name when the face is an icon. */
    label?: string
    shape?: 'rect' | 'square'
    /** A latching button that's on (Shift). */
    pressed?: boolean
    width?: string
    /** A button that repeats while held (TEMPO −/+): called with true when the pointer goes
     *  down on it and false when it comes up; a pointer click then doesn't also run
     *  `onclick` (Enter/Space still do). */
    onhold?: (down: boolean) => void
  } = $props()

  let holding = false
  function down(e: PointerEvent) {
    if (!onhold || e.button !== 0) return
    holding = true
    ;(e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId)
    onhold(true)
  }
  function up() {
    if (!holding) return
    holding = false
    onhold?.(false)
  }
  function click(e: MouseEvent) {
    // A pointer click on a hold button was the hold; a keyboard click (detail 0) is a press.
    if (onhold && e.detail > 0) return
    onclick()
  }

  const b = $derived(led ? brightness(led, beats) : 0)
</script>

<div class="hw" style:width>
  <button
    type="button"
    class="btn mat-raised {shape}"
    class:pressed
    class:lit={b > 0.5}
    aria-pressed={pressed}
    aria-label={label}
    style:--led={led ? css(led.rgb) : 'transparent'}
    style:--b={b}
    use:tipAction={tip}
    onclick={click}
    onpointerdown={down}
    onpointerup={up}
    onpointercancel={up}
    onlostpointercapture={up}
  >
    <span class="glow" aria-hidden="true"></span>
    <span class="face">{@render children()}</span>
  </button>
  {#if caption !== undefined}<span class="caption engraved">{caption}</span>{/if}
</div>

<style>
  .hw {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.3em;
    min-width: 0;
  }
  .btn {
    position: relative;
    width: 100%;
    min-width: 2.6em;
    height: 2.1em;
    border-radius: 5px;
    padding: 0 0.5em;
    color: var(--ink);
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.9em;
    letter-spacing: 0.03em;
    overflow: hidden;
  }
  .square {
    height: 2.6em;
  }
  /* The backlight: a pre-rendered colour wash whose opacity is the LED brightness. */
  .glow {
    position: absolute;
    inset: 0;
    background: radial-gradient(120% 90% at 50% 60%, var(--led), transparent 75%);
    opacity: calc(var(--b) * 0.85);
    pointer-events: none;
  }
  .face {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.3em;
    white-space: nowrap;
  }
  .lit .face {
    color: #fff;
    text-shadow: 0 0 6px rgb(0 0 0 / 0.6);
  }
  .pressed {
    outline: 1px solid var(--accent);
  }
  .caption {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: center;
    min-height: 1em;
  }
</style>
