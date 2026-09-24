// `use:tip={'catalog.key'}` on every interactive element. It marks the element with
// `data-tip` (the coverage test looks for it) and, on hover or keyboard focus, shows the
// catalog entry in the help footer at the bottom of the window. Nothing floats over the
// instrument unless the player opts in (`tips.floating`, the footer's "Pop-up" switch).

import { isTipKey, type TipKey } from '../../help/tooltips'

/** How long the footer keeps the last entry after the pointer leaves a control, so moving
 * across the gap to the next one doesn't flash the idle hint. */
export const CLEAR_MS = 350
/** Opt-in pop-up tips only: how long the pointer rests before the first one shows. After
 * that, moving to the next control shows its tip at once (for `WARM_MS`). */
export const DELAY_MS = 280
const WARM_MS = 600
const FLOATING_KEY = 'yahaha.floatingTips'

function storedFloating(): boolean {
  try {
    return localStorage.getItem(FLOATING_KEY) === '1'
  } catch {
    return false
  }
}

class TipState {
  /** The entry the footer shows (and the pop-up, if on): the control hovered or focused. */
  key = $state<TipKey | null>(null)
  /** Where that control is: only the opt-in pop-up uses it. */
  rect = $state<DOMRect | null>(null)
  /** Help mode: the footer grows to the full entry, and the last one stays pinned. */
  help = $state(false)
  pinned = $state<TipKey | null>(null)
  /** The control with keyboard focus: the footer's screen-reader description
   * (`aria-describedby`) follows it, whatever the pointer is over. */
  focused = $state<TipKey | null>(null)
  /** Opt-in: also show the entry in a pop-up next to the control (remembered). */
  floating = $state(storedFloating())

  private showTimer: ReturnType<typeof setTimeout> | null = null
  private clearTimer: ReturnType<typeof setTimeout> | null = null
  private warmUntil = 0
  private owner: HTMLElement | null = null
  private pending: HTMLElement | null = null

  /** What the footer shows: the control under the pointer or focus, else (help mode) the
   * last one. */
  get shown(): TipKey | null {
    return this.key ?? (this.help ? this.pinned : null)
  }

  show(el: HTMLElement, key: TipKey, now: boolean) {
    this.cancel()
    const go = () => {
      this.owner = el
      this.key = key
      this.rect = this.floating ? el.getBoundingClientRect() : null
      if (this.help) this.pinned = key
    }
    // The footer covers nothing, so it follows at once. Only the pop-up waits, so it
    // doesn't flash over the controls you sweep past.
    const wait = this.floating && !now && !this.help && performance.now() >= this.warmUntil
    if (wait) {
      this.pending = el
      this.showTimer = setTimeout(go, DELAY_MS)
    } else go()
  }

  /** The pointer left `el` (or it lost focus): clear after `CLEAR_MS`, unless another
   * control takes over first. Without `el`: clear now (Esc). */
  hide(el?: HTMLElement) {
    if (el && el === this.pending && this.showTimer) {
      // Left before its pop-up delay ran out: just don't show it.
      clearTimeout(this.showTimer)
      this.showTimer = this.pending = null
      return
    }
    if (el && this.owner !== el) return
    this.cancel()
    if (!el || this.floating) {
      this.clear()
      return
    }
    this.clearTimer = setTimeout(() => this.clear(), CLEAR_MS)
  }

  /** Follow a control that moved or changed size (a fader cap while dragging). */
  refresh() {
    if (this.owner && this.floating) this.rect = this.owner.getBoundingClientRect()
  }

  toggleHelp() {
    this.help = !this.help
    if (!this.help) this.pinned = null
  }

  setFloating(on: boolean) {
    this.floating = on
    try {
      if (on) localStorage.setItem(FLOATING_KEY, '1')
      else localStorage.removeItem(FLOATING_KEY)
    } catch {
      /* private window: not remembered */
    }
  }

  private clear() {
    this.cancel()
    if (this.key) this.warmUntil = performance.now() + WARM_MS
    this.key = null
    this.owner = null
    this.rect = null
  }

  private cancel() {
    if (this.showTimer) clearTimeout(this.showTimer)
    if (this.clearTimer) clearTimeout(this.clearTimer)
    this.showTimer = this.clearTimer = null
    this.pending = null
  }
}

export const tips = new TipState()

/** The footer's plain-text description of the focused control (`aria-describedby`). */
export const TOOLTIP_ID = 'yahaha-help-entry'

export function tip(node: HTMLElement, key: TipKey) {
  let k = key
  const set = () => {
    if (!isTipKey(k)) console.warn(`tooltip key not in catalog: ${k}`)
    node.dataset.tip = k
  }
  set()
  const enter = (e: PointerEvent) => {
    if (e.pointerType !== 'touch') tips.show(node, k, false)
  }
  const leave = () => tips.hide(node)
  const focus = () => {
    // Keyboard focus only: a click that focuses shouldn't change what you're reading.
    let visible = true
    try {
      visible = node.matches(':focus-visible')
    } catch {
      /* engines without :focus-visible: treat focus as keyboard focus */
    }
    if (visible) {
      tips.show(node, k, true)
      tips.focused = k
      node.setAttribute('aria-describedby', TOOLTIP_ID)
    }
  }
  const blur = () => {
    tips.hide(node)
    if (tips.focused === k) tips.focused = null
    node.removeAttribute('aria-describedby')
  }
  // Buttons don't keep focus after a click, so Space and Enter stay performance keys.
  const down = (e: MouseEvent) => {
    if (node instanceof HTMLButtonElement) e.preventDefault()
  }
  node.addEventListener('pointerenter', enter)
  node.addEventListener('pointerleave', leave)
  node.addEventListener('focus', focus)
  node.addEventListener('blur', blur)
  node.addEventListener('mousedown', down)
  return {
    update(next: TipKey) {
      k = next
      set()
    },
    destroy() {
      tips.hide(node)
      node.removeEventListener('pointerenter', enter)
      node.removeEventListener('pointerleave', leave)
      node.removeEventListener('focus', focus)
      node.removeEventListener('blur', blur)
      node.removeEventListener('mousedown', down)
    },
  }
}
