<!--
  LampButton: the canvas's on/off control (Accomp, Metronome, part On, Sound, Looper).
  On is the lime lamp face with an ink label; off is the plain button face with a grey label;
  waiting is the armed lamp, outlined in its hue. Fully controlled: the face and aria-pressed
  follow `on` alone, and a click only asks for `!on` through ontoggle. Long press (and
  right-click) comes from the shared longpress action and never toggles.
-->
<script lang="ts">
  import type { Action } from 'svelte/action'
  import { longpress } from '../actions/longpress'

  type Props = {
    /** The word on the face. */
    label: string
    /** Lit (lamp face) or off. Controlled: a click asks for `!on` through `ontoggle` and changes nothing itself. */
    on?: boolean
    /** Small code after the label, e.g. `ACMP`. */
    code?: string
    /** Shown, not pressable: no toggle and no long press. Stays focusable. */
    disabled?: boolean
    /** Record lamp: lit is the solid record-red face instead of the lamp face. */
    rec?: boolean
    /** The waiting (armed) face while not on: no fill, a 1px outline and the label in `hue`. */
    waiting?: boolean
    /** The waiting face's colour: `lamp` (Loop armed, in `--lamp-line`) or `rec` (Rec armed). */
    hue?: 'lamp' | 'rec'
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
    code,
    disabled = false,
    rec = false,
    waiting = false,
    hue = 'lamp',
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
  data-hue={face === 'waiting' ? hue : undefined}
  data-contrast={disabled ? 'dim' : undefined}
  data-tip={tip}
  aria-pressed={on}
  aria-disabled={disabled ? 'true' : undefined}
  aria-label={name ?? (code ? `${label} ${code}` : label)}
  onclick={toggle}
  use:tipped={tip}
  use:longpress={{ onlongpress, onlongrelease, disabled: disabled || onlongpress === undefined }}
>
  {label}{#if code}<span class="sub">{code}</span>{/if}
</button>

<style>
  .lamp {
    box-sizing: border-box;
    height: var(--control-height);
    margin: 0;
    padding: 0 var(--space-16);
    border: 0;
    border-radius: var(--radius);
    background: var(--btn);
    color: var(--m);
    font-family: var(--font-sans);
    font-size: var(--text-14);
    font-weight: var(--weight-regular);
    font-variant-numeric: tabular-nums;
    line-height: normal;
    white-space: nowrap;
    cursor: pointer;
  }
  .sm {
    height: var(--control-height-compact);
    padding: 0 var(--space-14);
    font-size: var(--text-13);
  }
  .cell {
    width: 100%;
    min-width: 0;
    padding: 0;
    font-size: var(--text-13);
  }
  .fixed {
    padding: 0;
  }
  .join-start {
    border-radius: var(--radius) 0 0 var(--radius);
  }
  .join-end {
    border-radius: 0 var(--radius) var(--radius) 0;
  }
  .sub {
    margin-left: var(--space-6);
    color: var(--m);
    font-size: var(--text-12);
    font-weight: var(--weight-regular);
  }
  .face-on {
    background: var(--lamp);
    color: var(--lamp-ink);
    font-weight: var(--weight-medium);
  }
  .face-on .sub {
    color: var(--lamp-ink);
    opacity: var(--code-opacity);
  }
  .face-record {
    background: var(--rec);
    color: var(--solid-ink);
    font-weight: var(--weight-medium);
  }
  /* The code on the record face is full ink: at --code-opacity it fails AA on --rec. */
  .face-record .sub {
    color: var(--solid-ink);
  }
  /* Armed: an inset outline, so the box is the same size as every other face. */
  .face-waiting {
    --hue: var(--lamp-line);
    background: transparent;
    box-shadow: inset 0 0 0 var(--line-width) var(--hue);
    color: var(--hue);
  }
  .face-waiting[data-hue='rec'] {
    --hue: var(--rec);
  }
  .face-waiting .sub {
    color: var(--hue);
  }
  .disabled,
  .disabled .sub {
    color: var(--d);
    opacity: 1;
    cursor: default;
  }
  .lamp:focus-visible {
    outline: var(--line-width) solid var(--focus);
    outline-offset: var(--focus-offset);
  }
</style>
