// `use:tip={'catalog.key'}` on every interactive element. It marks the element with
// `data-tip` (the coverage test looks for it), shows the catalog entry on hover (quickly)
// and on keyboard focus (at once), and never takes pointer events, so it can't get in the
// way of playing.

import { isTipKey, type TipKey } from '../../help/tooltips'

/** How long the pointer rests before the first tooltip shows. After that, moving to the
 * next control shows its tooltip at once (for `WARM_MS`). */
export const DELAY_MS = 280
const WARM_MS = 600

class TipState {
  key = $state<TipKey | null>(null)
  rect = $state<DOMRect | null>(null)
  /** Help mode: every tooltip shows at once, and the last one stays pinned in the help bar. */
  help = $state(false)
  pinned = $state<TipKey | null>(null)

  private timer: ReturnType<typeof setTimeout> | null = null
  private warmUntil = 0
  private owner: HTMLElement | null = null

  show(el: HTMLElement, key: TipKey, now: boolean) {
    this.cancel()
    const go = () => {
      this.owner = el
      this.key = key
      this.rect = el.getBoundingClientRect()
      if (this.help) this.pinned = key
    }
    if (now || this.help || performance.now() < this.warmUntil) go()
    else this.timer = setTimeout(go, DELAY_MS)
  }

  hide(el?: HTMLElement) {
    this.cancel()
    if (el && this.owner !== el) return
    if (this.key) this.warmUntil = performance.now() + WARM_MS
    this.key = null
    this.owner = null
  }

  /** Follow a control that moved or changed size (a fader cap while dragging). */
  refresh() {
    if (this.owner) this.rect = this.owner.getBoundingClientRect()
  }

  toggleHelp() {
    this.help = !this.help
    if (!this.help) this.pinned = null
  }

  private cancel() {
    if (this.timer) clearTimeout(this.timer)
    this.timer = null
  }
}

export const tips = new TipState()

export const TOOLTIP_ID = 'yahaha-tooltip'

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
    // Keyboard focus only: a click that focuses shouldn't pop a tooltip over your hand.
    let visible = true
    try {
      visible = node.matches(':focus-visible')
    } catch {
      /* engines without :focus-visible: treat focus as keyboard focus */
    }
    if (visible) {
      tips.show(node, k, true)
      node.setAttribute('aria-describedby', TOOLTIP_ID)
    }
  }
  const blur = () => {
    tips.hide(node)
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
