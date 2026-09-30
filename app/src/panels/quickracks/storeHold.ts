// Store by slot with no arming step, on a Quick Rack button: a long press (touch, pen or
// the mouse's main button, held STORE_HOLD_MS) or a right-click runs `store` instead of the
// button's press. It is the app's form of the Launchkey's hold Sound + tap a Racks pad, and
// the caller sends `storeRack { slot }` for it, as the pads do. Put it on an element that
// wraps the button alone (`display: contents` keeps the layout): the press that stored
// doesn't also click (load) it.
//
// A touch long press may fire `contextmenu` too (Android): the gesture stores once.

import type { Action } from 'svelte/action'

/** How long a press is held to store rather than load. */
export const STORE_HOLD_MS = 500

export const storeHold: Action<HTMLElement, () => void> = (node, param) => {
  let store = param
  let timer: ReturnType<typeof setTimeout> | null = null
  /** This press stored: swallow its click and any contextmenu it brings. */
  let stored = false

  const stop = () => {
    if (timer !== null) clearTimeout(timer)
    timer = null
  }
  const fire = () => {
    stop()
    stored = true
    store()
  }
  const down = (e: PointerEvent) => {
    stored = false
    stop()
    if (e.button !== 0 || e.isPrimary === false) return
    timer = setTimeout(fire, STORE_HOLD_MS)
  }
  const menu = (e: MouseEvent) => {
    e.preventDefault()
    if (!stored) fire()
  }
  const click = (e: MouseEvent) => {
    // A keyboard click (detail 0) is never the press that stored.
    if (!stored || e.detail === 0) return
    stored = false
    e.stopPropagation()
    e.preventDefault()
  }

  node.addEventListener('pointerdown', down)
  node.addEventListener('pointerup', stop)
  node.addEventListener('pointercancel', stop)
  node.addEventListener('pointerleave', stop)
  node.addEventListener('contextmenu', menu)
  node.addEventListener('click', click, true)
  return {
    update(p) {
      store = p
    },
    destroy() {
      stop()
      node.removeEventListener('pointerdown', down)
      node.removeEventListener('pointerup', stop)
      node.removeEventListener('pointercancel', stop)
      node.removeEventListener('pointerleave', stop)
      node.removeEventListener('contextmenu', menu)
      node.removeEventListener('click', click, true)
    },
  }
}
