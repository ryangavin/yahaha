<!--
  LampButton: the canvas's on/off control (Accomp, Metronome, part On, Sound, Looper), drawn as
  Round 2's lamp: the plain button face with a 2px bar under the label. On is the bright label and
  the bar in its hue, glowing (dark theme); off is a quieter label over a dim bar; waiting (armed)
  is the label in its hue over a dashed bar; record is the bar in record red. Fully controlled: the
  face and aria-pressed follow `on` alone, and a click only asks for `!on` through ontoggle. Long
  press (and right-click) comes from the shared longpress action and never toggles.
-->
<script lang="ts">
  import type { Action } from 'svelte/action'
  import { longpress } from '../actions/longpress'

  type Props = {
    /** The word on the face. */
    label: string
    /** Lit (bright label, bar in its hue) or off. Controlled: a click asks for `!on` through `ontoggle` and changes nothing itself. */
    on?: boolean
    /** The bar's hue: `t` white (Accomp, Metronome), a part (`r1` `r2` `r3` `l`), `ok` green (Start / Stop), `m` grey (a function lamp: a soft white bar when on, no glow). */
    hue?: 't' | 'r1' | 'r2' | 'r3' | 'l' | 'ok' | 'm'
    /** Small code after the label, e.g. `ACMP`. */
    code?: string
    /** Shown, not pressable: no toggle and no long press. Stays focusable. */
    disabled?: boolean
    /** Record lamp: lit, and waiting, draw in record red instead of `hue`. */
    rec?: boolean
    /** The waiting (armed) face while not on: the label in the hue (record red with `rec`) over a dashed bar. */
    waiting?: boolean
    /** `md` 32px tall (section row), `sm` 28px (settings rows), `cell` 32px filling its container (the band's lamp row). */
    size?: 'md' | 'sm' | 'cell'
    /** A fixed width in px, label centred, no side padding (e.g. 64 for the settings rows' On/Off). Wins over the size's width. */
    width?: number
    /** Joined to a neighbour: `start` rounds only the left corners, `end` only the right. */
    join?: 'start' | 'end'
    /** The accessible name when the label alone isn't enough ("Right 1 on"). Default: label and code. */
    name?: string
    /** The tooltip key from `app/src/help/tooltips.ts` (e.g. `transport.acmp`), rendered as `data-tip`. */
    tip?: string
    /** The app's `use:tip` action, passed in by the wiring; applied with `tip` when both are set. */
    tipAction?: Action<HTMLElement, string>
    /** Called with the state asked for (`!on`) after a click, Space or Enter; not after a long press. */
    ontoggle?: (on: boolean) => void
    /** Called when the button is held for the long-press time, or right-clicked. */
    onlongpress?: () => void
    /** Called when the press that fired `onlongpress` ends. */
    onlongrelease?: () => void
  }

  let {
    label,
    on = false,
    hue = 't',
    code,
    disabled = false,
    rec = false,
    waiting = false,
    size = 'md',
    width,
    join,
    name,
    tip,
    tipAction,
    ontoggle,
    onlongpress,
    onlongrelease,
  }: Props = $props()

  /** The face as drawn: on (or record) > waiting > off. */
  let face = $derived(on ? (rec ? 'record' : 'on') : waiting ? 'waiting' : 'off')

  /** Applies the parent's tooltip action when both it and a key are given. */
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

  function toggle() {
    if (disabled) return
    ontoggle?.(!on)
  }
</script>

<button
  type="button"
  class="lamp {size} face-{face}"
  class:fixed={width !== undefined}
  class:join-start={join === 'start'}
  class:join-end={join === 'end'}
  class:disabled
  style:width={width === undefined ? undefined : `${width}px`}
  data-face={disabled ? 'disabled' : face}
  data-hue={rec && face !== 'off' ? 'rec' : hue}
  data-contrast={disabled ? 'dim' : undefined}
  data-tip={tip}
  aria-pressed={on}
  aria-disabled={disabled ? 'true' : undefined}
  aria-label={name ?? (code ? `${label} ${code}` : label)}
  onclick={toggle}
  use:tipped={tip}
  use:longpress={{ onlongpress, onlongrelease, disabled: disabled || onlongpress === undefined }}
>
  <span class="label">{label}{#if code}<span class="sub">{code}</span>{/if}</span><span class="bar" aria-hidden="true"
  ></span>
</button>

<style>
  /* Each hue sets the lit bar (--lit), its glow (--glow), the dim off bar (--dim), the off label
     (--rest) and the waiting label (--wait). */
  .lamp {
    --lit: var(--t);
    --glow: var(--lamp-glow-t);
    --dim: var(--lamp-bar-off);
    --rest: var(--t2);
    --wait: var(--t);
    --inset: var(--lamp-bar-inset);
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;
    height: var(--control-height);
    margin: 0;
    padding: 0 var(--space-16) var(--bar-lift);
    border: 0;
    border-radius: var(--radius);
    background: var(--btn);
    color: var(--rest);
    font-family: var(--font-sans);
    font-size: var(--text-14);
    font-weight: var(--weight-regular);
    font-variant-numeric: tabular-nums;
    line-height: normal;
    white-space: nowrap;
    cursor: pointer;
  }
  .lamp[data-hue='m'] {
    --lit: var(--lamp-bar-m);
    --glow: none;
  }
  .lamp[data-hue='r1'] {
    --lit: var(--r1);
    --glow: var(--lamp-glow-r1);
    --dim: var(--lamp-bar-dim-r1);
    --rest: var(--m);
    --wait: var(--r1);
  }
  .lamp[data-hue='r2'] {
    --lit: var(--r2);
    --glow: var(--lamp-glow-r2);
    --dim: var(--lamp-bar-dim-r2);
    --rest: var(--m);
    --wait: var(--r2);
  }
  .lamp[data-hue='r3'] {
    --lit: var(--r3);
    --glow: var(--lamp-glow-r3);
    --dim: var(--lamp-bar-dim-r3);
    --rest: var(--m);
    --wait: var(--r3);
  }
  .lamp[data-hue='l'] {
    --lit: var(--l);
    --glow: var(--lamp-glow-l);
    --dim: var(--lamp-bar-dim-l);
    --rest: var(--m);
    --wait: var(--l);
  }
  .lamp[data-hue='ok'] {
    --lit: var(--ok);
    --glow: var(--lamp-glow-ok);
    --dim: var(--lamp-bar-dim-ok);
    --wait: var(--ok);
  }
  .lamp[data-hue='rec'] {
    --lit: var(--rec);
    --glow: var(--lamp-glow-rec);
    --wait: var(--rec);
  }
  .sm {
    height: var(--control-height-compact);
    padding: 0 var(--space-14) var(--bar-lift);
    font-size: var(--text-13);
  }
  .cell {
    --inset: var(--lamp-bar-inset-cell);
    width: 100%;
    min-width: 0;
    padding: 0 0 var(--bar-lift);
    font-size: var(--text-13);
  }
  .fixed {
    padding-right: 0;
    padding-left: 0;
  }
  .join-start {
    border-radius: var(--radius) 0 0 var(--radius);
  }
  .join-end {
    border-radius: 0 var(--radius) var(--radius) 0;
  }
  .label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sub {
    margin-left: var(--space-6);
    color: var(--m);
    font-size: var(--text-12);
    font-weight: var(--weight-regular);
  }
  .bar {
    position: absolute;
    right: var(--inset);
    bottom: var(--bar-bottom);
    left: var(--inset);
    height: var(--lamp-bar-height);
    border-radius: calc(var(--lamp-bar-height) / 2);
    background: var(--dim);
  }

  /* On: the bright label over the bar in its hue, glowing. */
  .face-on,
  .face-record {
    color: var(--t);
  }
  .face-on:not(.cell),
  .face-record:not(.cell) {
    font-weight: var(--weight-medium);
  }
  .face-on .bar,
  .face-record .bar {
    background: var(--lit);
    box-shadow: var(--glow);
  }
  /* Waiting (armed): the label in its hue over a dashed bar of the same hue. */
  .face-waiting {
    color: var(--wait);
  }
  .face-waiting .sub {
    color: var(--wait);
  }
  .face-waiting .bar {
    background: repeating-linear-gradient(
      90deg,
      var(--wait) 0 var(--lamp-dash),
      transparent var(--lamp-dash) calc(var(--lamp-dash) + var(--lamp-dash-gap))
    );
  }

  .disabled,
  .disabled .sub {
    color: var(--d);
    cursor: default;
  }
  .disabled .bar {
    background: var(--lamp-bar-off);
    box-shadow: none;
  }
  .lamp:focus-visible {
    outline: var(--line-width) solid var(--focus);
    outline-offset: var(--focus-offset);
  }
</style>
