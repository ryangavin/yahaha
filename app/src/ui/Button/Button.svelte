<!--
  Button: does one thing when pressed (Panic, Stop, a page step, a One Touch), in the plain button
  face, and shows when that thing is chosen, switched on or waiting. Every face comes from props
  (precedence chosen, on, waiting, off); it holds no state of its own. Glyphs are the fixed
  `symbol` list. Long press comes from the shared longpress action; `hold` reports pointer down
  and up through onhold for the parent's repeat (Tempo ±). No timers.
-->
<script lang="ts">
  import type { Action } from 'svelte/action'
  import { longpress } from '../actions/longpress'

  type Glyph = 'prev' | 'next' | 'up' | 'down' | 'plus' | 'minus' | 'caret'
  type Hue =
    | 't'
    | 't2'
    | 'm'
    | 'a'
    | 'lamp'
    | 'rec'
    | 'ok'
    | 'r1'
    | 'r2'
    | 'r3'
    | 'l'
    | 'intro'
    | 'main'
    | 'ending'
    | 'brk'
    | 'fill'

  type Props = {
    /** The word or character on the face ("Panic", "Stop", "1", "?"). May be empty when `symbol` is set. */
    label?: string
    /** A glyph after the label, or alone: ◀ ▶ ▲ ▼ + − ▾. Hidden from assistive tech. */
    symbol?: Glyph
    /** `icon` 32 × 32; `md` width from the label; `band` 88 wide, left-aligned; `pair` 41 wide; `cell` fills its container; `caret` the 20 × 32 ▾. */
    size?: 'icon' | 'md' | 'band' | 'pair' | 'cell' | 'caret'
    /** A 13px label instead of 14 in `icon`, `md` and `band` (`pair`, `cell` and `caret` are always 13). */
    compact?: boolean
    /** The label in `--t` at medium weight on the off face (Start / Stop). */
    strong?: boolean
    /** The running bar, `band` only. Undefined: no bar. `false`: the bar's room kept, nothing drawn. `true`: the green bar. */
    bar?: boolean
    /** The lamp face: switched on (help mode's ?, Fade while fading). */
    on?: boolean
    /** The chosen face: the one picked from a set (the applied One Touch). */
    chosen?: boolean
    /** The waiting face: queued or armed, outlined in `hue`. */
    waiting?: boolean
    /** The colour token (without `--`) of the waiting face's outline and label; `lamp` draws in `--lamp-line`. */
    hue?: Hue
    /** Sets `aria-pressed` for a switch or a choice. Undefined: no `aria-pressed`. Never changes the look. */
    pressed?: boolean
    /** Sets `aria-haspopup` (the caret opens a dialog). */
    popup?: 'dialog' | 'menu'
    /** With `popup`: `aria-expanded`, and the caret's ▾ turns `--t` while open. */
    expanded?: boolean
    /** Sets `aria-controls`: the id of the popover this button opens. */
    controls?: string
    /** Joined to a neighbour: `start` rounds only the left corners, `end` only the right. */
    join?: 'start' | 'end'
    /** Shown, not pressable: no press, hold or long press. Stays focusable. */
    disabled?: boolean
    /** Repeat-while-held (Tempo ±): pointer down and up call `onhold`; a pointer click doesn't press; no long press. */
    hold?: boolean
    /** The accessible name. Default: the label, then the symbol's word ("Fill up"). */
    name?: string
    /** The tooltip key from `app/src/help/tooltips.ts` (e.g. `transport.panic`), rendered as `data-tip`. */
    tip?: string
    /** The app's `use:tip` action, passed in by the wiring; applied with `tip` when both are set. */
    tipAction?: Action<HTMLElement, string>
    /** Called on a click, Space or Enter; not for the click that ends a long press; with `hold`, only from the keyboard. */
    onpress?: () => void
    /** With `hold`: `true` when a primary pointer goes down, `false` once when that hold ends. */
    onhold?: (down: boolean) => void
    /** Called when the button is held for the long-press time, or right-clicked. */
    onlongpress?: () => void
    /** Called when the press that fired `onlongpress` ends. */
    onlongrelease?: () => void
  }

  let {
    label = '',
    symbol,
    size = 'md',
    compact = false,
    strong = false,
    bar,
    on = false,
    chosen = false,
    waiting = false,
    hue = 't2',
    pressed,
    popup,
    expanded = false,
    controls,
    join,
    disabled = false,
    hold = false,
    name,
    tip,
    tipAction,
    onpress,
    onhold,
    onlongpress,
    onlongrelease,
  }: Props = $props()

  const GLYPHS: Record<Glyph, string> = {
    prev: '◀',
    next: '▶',
    up: '▲',
    down: '▼',
    plus: '+',
    minus: '−',
    caret: '▾',
  }
  const WORDS: Record<Glyph, string> = {
    prev: 'previous',
    next: 'next',
    up: 'up',
    down: 'down',
    plus: 'plus',
    minus: 'minus',
    caret: 'options',
  }

  /** The face as drawn: chosen, then on, then waiting, then off. */
  let face = $derived(chosen ? 'chosen' : on ? 'on' : waiting ? 'waiting' : 'off')
  let accessibleName = $derived(name ?? [label, symbol ? WORDS[symbol] : ''].filter(Boolean).join(' '))
  let small = $derived(compact || size === 'pair' || size === 'cell' || size === 'caret')
  let barRoom = $derived(size === 'band' && bar !== undefined)

  let node: HTMLButtonElement | undefined = $state()
  /** The pointer of the hold in progress; not reactive, the effects below only read it. */
  let holdId: number | null = null

  function endHold(release: boolean) {
    const id = holdId
    if (id === null) return
    holdId = null
    if (release) {
      try {
        node?.releasePointerCapture?.(id)
      } catch {
        // No capture to release (jsdom, a pointer already gone).
      }
    }
    onhold?.(false)
  }

  // A hold ends at once when it can't continue: disabled, or no longer a hold button.
  $effect(() => {
    if (disabled || !hold) endHold(true)
  })
  // ...or when the button goes away while held.
  $effect(() => () => endHold(false))

  function pointerDown(event: PointerEvent) {
    if (!hold || disabled || event.button !== 0 || holdId !== null) return
    holdId = event.pointerId
    try {
      node?.setPointerCapture?.(event.pointerId)
    } catch {
      // jsdom and synthetic events have no pointer capture.
    }
    onhold?.(true)
  }

  function pointerEnd(event: PointerEvent) {
    if (holdId !== null && event.pointerId === holdId) endHold(false)
  }

  function click(event: MouseEvent) {
    if (disabled) return
    if (hold && event.detail !== 0) return
    onpress?.()
  }

  /** Applies the parent's tooltip action when both it and a key are given. */
  const tipped: Action<HTMLElement, string | undefined> = (target, key) => {
    if (!tipAction || key === undefined) return
    const handle = tipAction(target, key)
    return {
      update: (next) => {
        if (next !== undefined) handle?.update?.(next)
      },
      destroy: () => handle?.destroy?.(),
    }
  }
</script>

<button
  bind:this={node}
  type="button"
  class="btn {size} face-{face}"
  class:small
  class:strong
  class:alone={label === ''}
  class:expanded={popup !== undefined && expanded}
  class:bar-room={barRoom}
  class:join-start={join === 'start'}
  class:join-end={join === 'end'}
  class:disabled
  style:--hue={face === 'waiting' ? (hue === 'lamp' ? 'var(--lamp-line)' : `var(--${hue})`) : undefined}
  data-face={disabled ? 'disabled' : face}
  data-hue={face === 'waiting' ? hue : undefined}
  data-contrast={disabled ? 'dim' : undefined}
  data-tip={tip}
  aria-label={accessibleName}
  aria-pressed={pressed}
  aria-haspopup={popup}
  aria-expanded={popup !== undefined ? expanded : undefined}
  aria-controls={controls}
  aria-disabled={disabled ? 'true' : undefined}
  onclick={click}
  onpointerdown={pointerDown}
  onpointerup={pointerEnd}
  onpointercancel={pointerEnd}
  onlostpointercapture={pointerEnd}
  use:tipped={tip}
  use:longpress={{ onlongpress, onlongrelease, disabled: disabled || hold || onlongpress === undefined }}
>
  {label}{#if symbol}{label ? ' ' : ''}<span class="sym sym-{symbol}" aria-hidden="true">{GLYPHS[symbol]}</span
    >{/if}{#if barRoom && bar}<span class="bar" data-bar aria-hidden="true"></span>{/if}
</button>

<style>
  .btn {
    position: relative;
    box-sizing: border-box;
    height: var(--control-height);
    margin: 0;
    padding: 0;
    border: 0;
    border-radius: var(--radius);
    background: var(--btn);
    color: var(--t2);
    font-family: var(--font-sans);
    font-size: var(--text-14);
    font-weight: var(--weight-regular);
    font-variant-numeric: tabular-nums;
    line-height: normal;
    text-align: center;
    white-space: nowrap;
    cursor: pointer;
  }
  .small {
    font-size: var(--text-13);
  }

  /* Sizes */
  .icon {
    width: var(--control-height);
  }
  .md {
    padding: 0 var(--space-14);
  }
  .band {
    width: var(--button-band-width);
    padding: 0 var(--space-8);
    text-align: left;
  }
  .band.bar-room {
    padding: 0 var(--space-4) var(--bar-lift) var(--space-8);
  }
  .pair {
    width: var(--button-pair-width);
  }
  .cell {
    width: 100%;
    min-width: 0;
  }
  .caret {
    width: var(--caret-width);
  }

  /* Glyphs: inline on the label's baseline, no line-height of their own. */
  .sym-prev,
  .sym-next,
  .alone .sym-up,
  .alone .sym-down,
  .alone .sym-caret {
    font-size: var(--text-12);
  }
  .sym-up,
  .sym-down {
    font-size: var(--text-9);
  }
  .sym-caret,
  .caret.alone .sym-caret {
    font-size: var(--text-10);
  }
  .sym-plus,
  .sym-minus {
    font-weight: var(--weight-light);
  }

  /* Faces */
  .face-off.strong {
    color: var(--t);
    font-weight: var(--weight-medium);
  }
  .face-off.caret:not(.disabled) {
    color: var(--m);
  }
  .face-off.caret.expanded:not(.disabled) {
    color: var(--t);
  }
  .face-on {
    background: var(--lamp);
    color: var(--lamp-ink);
    font-weight: var(--weight-medium);
  }
  .face-chosen {
    background: var(--t);
    color: var(--g);
    font-weight: var(--weight-medium);
  }
  /* Waiting: an inset outline, so the box never changes size. */
  .face-waiting {
    background: transparent;
    box-shadow: inset 0 0 0 var(--line-width) var(--hue);
    color: var(--hue);
  }
  .btn.disabled {
    color: var(--d);
    cursor: default;
  }

  .join-start {
    border-radius: var(--radius) 0 0 var(--radius);
  }
  .join-end {
    border-radius: 0 var(--radius) var(--radius) 0;
  }

  .bar {
    position: absolute;
    right: var(--space-8);
    bottom: var(--bar-bottom);
    left: var(--space-8);
    height: var(--space-2);
    border-radius: calc(var(--space-2) / 2);
    background: var(--ok);
    box-shadow: var(--bg);
  }

  .btn:focus-visible {
    outline: var(--line-width) solid var(--focus);
    outline-offset: var(--focus-offset);
  }
</style>
