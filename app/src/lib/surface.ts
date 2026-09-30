// The Launchkey surface beyond the pads: what every button and fader does on the current
// page and Shift layer, how it's lit, and the beat clock. The engine sends it as
// `state.surface` (#77, docs/app-api.md "surface"); the mocks send the same
// (lib/api/mock-surface.ts). These helpers turn its controls into labels and tooltips.

import { tipFor } from '../help/actions'
import type { TipKey } from '../help/tooltips'
import type { AppCmd, AppState, LibraryList, SurfaceControl, SurfaceState } from './api/types'

export { neighbours } from './api/constants'

/** The surface, as the engine sends it. (`lib` is unused: kept for existing callers.) */
export function surfaceOf(s: AppState, lib?: LibraryList): SurfaceState {
  void lib
  return s.surface
}

/** A control's label and action on the layer showing. */
export function layer(c: SurfaceControl, shift: boolean): { label: string; action: AppCmd | null } {
  return shift ? { label: c.shiftLabel, action: c.shiftAction } : { label: c.label, action: c.action }
}

/**
 * Whether Shift gives a control a different function. Compared by value: the engine's
 * JSON gives `action` and `shiftAction` separate objects even when they're the same command.
 */
export function hasShiftFunction(c: SurfaceControl): boolean {
  return c.shiftLabel !== c.label || !sameValue(c.shiftAction, c.action)
}

/** Deep equality of two JSON values (a command's fields, in any key order). */
function sameValue(a: unknown, b: unknown): boolean {
  if (a === b) return true
  if (typeof a !== 'object' || typeof b !== 'object' || a === null || b === null) return false
  if (Array.isArray(a) !== Array.isArray(b)) return false
  const ka = Object.keys(a)
  if (ka.length !== Object.keys(b).length) return false
  const ra = a as Record<string, unknown>
  const rb = b as Record<string, unknown>
  return ka.every((k) => k in rb &&sameValue(ra[k], rb[k]))
}

/** The catalog entry for a control on the layer showing. */
export function controlTip(c: SurfaceControl, shift: boolean): TipKey {
  const { action } = layer(c, shift)
  if (!shift && c.id === 'padBankUp') return 'padpage.prev'
  if (!shift && c.id === 'padBankDown') return 'padpage.next'
  if (c.id === 'trackPrev') return shift && action ? 'quick.prev' : 'style.prev'
  if (c.id === 'trackNext') return shift && action ? 'quick.next' : 'style.next'
  // Fader button 6 is Sound on both fader pages: a hold, so it sends no command.
  if (!shift && c.id === 'faderButton6' && c.label === 'SOUND') return 'launchkey.sound'
  if (!action) return c.id.startsWith('faderButton') ? 'launchkey.fader_unused' : 'launchkey.unused'
  return tipFor(action)
}
