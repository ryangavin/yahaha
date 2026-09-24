<!--
  A lamp button: the arranger's lit key. Its lamp is a `Pad` look from the engine
  (`state.transport.lamps`, `state.pads.pads`: exactly how the Launchkey pad is lit),
  animated on the beat clock (`clock.beats`):
  dim = available, bright = playing/on, flashing = queued, pulsing = armed, dark = absent.
  Its size never changes with state, so nothing shifts.
-->
<script lang="ts">
  import type { Snippet } from 'svelte'
  import { TIPS, keyLabel, type TipKey } from '../../help/tooltips'
  import type { Pad } from '../api/types'
  import { brightness, css } from '../leds'
  import { tip as tipAction } from '../tooltip/tip.svelte'

  let {
    look,
    beats,
    tip,
    onclick,
    children,
    sub,
    size = 'm',
    hint = true,
  }: {
    look: Pick<Pad, 'rgb' | 'level' | 'anim'>
    /** The beat clock (`clock.beats` from lib/store.svelte). */
    beats: number
    tip: TipKey
    onclick: () => void
    children: Snippet
    /** A second line under the label (e.g. "Fill" while a Main's fill plays). */
    sub?: string
    size?: 'm' | 'l'
    hint?: boolean
  } = $props()

  const b = $derived(brightness(look, beats))
  const lampState = $derived(
    look.level === 'off'
      ? 'not available'
      : look.anim === 'flash'
        ? 'queued'
        : look.anim === 'pulse'
          ? 'armed'
          : look.level === 'bright'
            ? 'on'
            : 'available',
  )
  const keyHint = $derived.by(() => {
    const t = TIPS[tip]
    const k = (t.app_keys ?? t.keys)[0]
    return hint && k ? keyLabel(k) : null
  })
</script>

<button
  type="button"
  class="lampkey {size}"
  class:off={look.level === 'off'}
  data-level={look.level}
  data-anim={look.anim}
  aria-disabled={look.level === 'off' || undefined}
  style:--led={css(look.rgb)}
  style:--b={b}
  use:tipAction={tip}
  onclick={() => look.level !== 'off' && onclick()}
>
  <span class="lamp" aria-hidden="true"></span>
  <span class="label">{@render children()}<span class="visually-hidden">, {lampState}</span></span>
  <span class="foot" aria-hidden="true">
    <span class="sub">{sub ?? ''}</span>
    {#if keyHint}<span class="hint">{keyHint}</span>{/if}
  </span>
</button>

<style>
  .lampkey {
    position: relative;
    display: grid;
    grid-template-rows: 6px 1fr auto;
    gap: 0.3rem;
    min-height: 4.25rem;
    min-width: 0;
    padding: 0.45rem 0.55rem 0.4rem;
    background: var(--raised);
    border: 1px solid var(--line);
    border-bottom-color: var(--line-strong);
    border-radius: var(--r-key);
    text-align: left;
  }
  .lampkey:hover {
    border-color: var(--line-strong);
  }
  .lampkey:active {
    transform: translateY(1px);
  }
  .l {
    min-height: 5.25rem;
  }
  .lamp {
    border-radius: 3px;
    background: color-mix(in srgb, var(--led) calc(var(--b) * 100%), var(--lamp-off));
    box-shadow: 0 0 calc(var(--b) * 16px) calc(var(--b) * 1px) color-mix(in srgb, var(--led) calc(var(--b) * 75%), transparent);
  }
  .label {
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 1.15rem;
    line-height: 1.1;
    letter-spacing: 0.01em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .l .label {
    font-size: 1.45rem;
  }
  .foot {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.3rem;
    min-height: 1rem;
    font-size: 0.72rem;
  }
  .sub {
    color: var(--led);
    font-weight: 600;
  }
  .hint {
    color: var(--muted);
    border: 1px solid var(--line);
    border-radius: 3px;
    padding: 0 0.25em;
    line-height: 1.3;
  }
  .off {
    cursor: default;
  }
  .off .label {
    color: var(--muted);
    opacity: 0.5;
  }
  .off .hint {
    opacity: 0.5;
  }
</style>
