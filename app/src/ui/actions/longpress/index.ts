/**
 * longpress: a Svelte action that lets a player hold a button for a moment (or right-click it) to
 * reach its second function, such as a part's swap mode, Loop rec, or Sound held down.
 *
 * Use: `<button use:longpress={{ onlongpress, onlongrelease, disabled }}>`.
 *
 * - A primary-button press held for `--long-press` (350 ms) without moving more than 4px calls
 *   `onlongpress` while still held; its release calls `onlongrelease`, and the click that follows
 *   is swallowed, so the component's own click handler never sees it.
 * - A right-click (or the Menu key / Shift+F10) calls `onlongpress`, then `onlongrelease` when the
 *   right button comes up (at once if it is already up). The browser's menu never opens.
 * - It draws nothing and adds no attribute; it follows the press with `window` listeners, never
 *   pointer capture, so the browser's own click rules are unchanged.
 */
import type { ActionReturn } from 'svelte/action'

export type LongPressParams = {
  /** Called once when a press has been held for the long-press time, or on a right-click. Undefined: the action is off. */
  onlongpress?: () => void
  /** Called once when a press that fired `onlongpress` ends (released, cancelled, or the node destroyed). */
  onlongrelease?: () => void
  /** No long press: a press starts nothing and a right-click calls nothing (its menu is still prevented). */
  disabled?: boolean
}

/** The hold time used when the `--long-press` token can't be read (jsdom, a missing token). */
export const LONG_PRESS_FALLBACK_MS = 350

/** A press that moves further than this from where it went down is not a long press. */
export const MOVE_TOLERANCE_PX = 4

/** Parses a CSS time (`350ms`, `0.5s`) into milliseconds; anything else is `null`. */
export function parseDuration(value: string): number | null {
  const match = /^(\d+(?:\.\d+)?|\.\d+)(ms|s)$/.exec(value.trim())
  if (!match) return null
  const n = Number(match[1])
  return match[2] === 's' ? n * 1000 : n
}

type Press =
  | { kind: 'primary'; pointerId: number; x: number; y: number; fired: boolean; timer?: ReturnType<typeof setTimeout> }
  | { kind: 'right' }

/** Calls `onlongpress` after a held press or on a right-click, and `onlongrelease` when it ends. */
export function longpress(node: HTMLElement, params: LongPressParams = {}): ActionReturn<LongPressParams> {
  let current = params
  let press: Press | null = null
  let swallow = false

  const off = () => current.disabled === true || current.onlongpress === undefined

  function holdTime(): number {
    const value = window.getComputedStyle(node).getPropertyValue('--long-press')
    return parseDuration(value ?? '') ?? LONG_PRESS_FALLBACK_MS
  }

  function listen() {
    window.addEventListener('pointermove', onMove)
    window.addEventListener('pointerup', onUp)
    window.addEventListener('pointercancel', onCancel)
  }

  function unlisten() {
    window.removeEventListener('pointermove', onMove)
    window.removeEventListener('pointerup', onUp)
    window.removeEventListener('pointercancel', onCancel)
  }

  /** Ends the press in progress; calls `onlongrelease` when it had fired. */
  function end() {
    const fired = press !== null && (press.kind === 'right' || press.fired)
    if (press?.kind === 'primary') clearTimeout(press.timer)
    press = null
    unlisten()
    if (fired) current.onlongrelease?.()
  }

  function onPointerDown(event: PointerEvent) {
    if (press) return
    swallow = false
    if (off() || event.button !== 0) return
    const started: Press = { kind: 'primary', pointerId: event.pointerId, x: event.clientX, y: event.clientY, fired: false }
    started.timer = setTimeout(() => {
      started.fired = true
      started.timer = undefined
      swallow = true
      current.onlongpress?.()
    }, holdTime())
    press = started
    listen()
  }

  function onMove(event: PointerEvent) {
    if (press?.kind !== 'primary' || press.fired || event.pointerId !== press.pointerId) return
    if (Math.hypot(event.clientX - press.x, event.clientY - press.y) > MOVE_TOLERANCE_PX) end()
  }

  function onUp(event: PointerEvent) {
    if (press?.kind === 'primary' ? event.pointerId === press.pointerId : event.button === 2) end()
  }

  function onCancel(event: PointerEvent) {
    if (press?.kind === 'primary' ? event.pointerId === press.pointerId : press !== null) end()
  }

  function onContextMenu(event: MouseEvent) {
    event.preventDefault()
    if (press || off()) return
    current.onlongpress?.()
    if (event.buttons & 2) {
      press = { kind: 'right' }
      listen()
    } else {
      current.onlongrelease?.()
    }
  }

  function onClick(event: MouseEvent) {
    if (!swallow) return
    swallow = false
    event.stopImmediatePropagation()
    event.preventDefault()
  }

  function onKeyDown() {
    swallow = false
  }

  node.addEventListener('pointerdown', onPointerDown)
  node.addEventListener('contextmenu', onContextMenu)
  node.addEventListener('click', onClick, { capture: true })
  node.addEventListener('keydown', onKeyDown, { capture: true })

  return {
    update(next) {
      current = next ?? {}
      // Turned off mid-press: a primary press that hasn't fired is cancelled; a fired one, or a
      // held right-click, still gets its release.
      if (off() && press?.kind === 'primary' && !press.fired) end()
    },
    destroy() {
      end()
      swallow = false
      node.removeEventListener('pointerdown', onPointerDown)
      node.removeEventListener('contextmenu', onContextMenu)
      node.removeEventListener('click', onClick, { capture: true })
      node.removeEventListener('keydown', onKeyDown, { capture: true })
    },
  }
}
