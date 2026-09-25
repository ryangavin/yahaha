// The window-level key handler: performance keys go to the engine, wherever the pointer
// is, except where a key has a local job (typing in a field, Space/Enter on a focused
// button, arrows on a fader or tab). Overlays take keys while open.

import { binding } from './keys'
import type { AppCmd } from './api/types'
import { app, ui } from './store.svelte'
import { tips } from './tooltip/tip.svelte'

/** Commands that may auto-repeat when the key is held. Toggles must not. */
const REPEATS: AppCmd['type'][] = ['tempoUp', 'tempoDown', 'moveSplit', 'stepTranspose', 'stepVoice']

function isTextField(el: EventTarget | null): boolean {
  return el instanceof HTMLElement && (el.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName))
}

/** Holding Shift shows the Launchkey mirror's Shift layer. */
export function handleKeyUp(e: KeyboardEvent) {
  if (e.key === 'Shift') ui.shiftHeld = false
}

export function handleBlur() {
  ui.shiftHeld = false
}

export function handleKey(e: KeyboardEvent) {
  const target = e.target
  if (e.key === 'Shift') ui.shiftHeld = true
  if (e.key === 'Escape') {
    if (tips.key) tips.hide()
    if (ui.escape()) e.preventDefault()
    else if (document.activeElement instanceof HTMLElement && document.activeElement !== document.body) document.activeElement.blur()
    return
  }
  // The style and sound browsers own the keyboard while open (their filters take typed
  // keys). Side drawers don't: performance keys keep working next to them.
  if (ui.browser || ui.soundBrowser !== null || ui.soundPick !== null) return
  if (isTextField(target)) return
  if (target instanceof HTMLButtonElement && (e.key === ' ' || e.key === 'Enter')) return
  const b = binding(e)
  if (!b) return
  e.preventDefault()
  if ('app' in b) {
    if (e.repeat) return
    if (b.app === 'browser') ui.browser = true
    else if (b.app === 'help') tips.toggleHelp()
    return
  }
  if (e.repeat && !REPEATS.includes(b.cmd.type)) return
  app.send(b.cmd)
}
