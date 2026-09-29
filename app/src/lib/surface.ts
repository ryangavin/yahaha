// The Launchkey surface beyond the pads: what every button and fader does on the current
// page and Shift layer, how it's lit, and the beat clock. The engine sends it as
// `state.surface` (#77, docs/app-api.md "surface"); the mocks send the same
// (lib/api/mock-surface.ts). These helpers turn it into tooltips and commands.

import { tipFor } from '../help/actions'
import type { TipKey } from '../help/tooltips'
import type { AppCmd, AppState, LibraryList, SurfaceControl, SurfaceFader, SurfaceState } from './api/types'

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
  if (!action) return c.id.startsWith('faderButton') ? 'launchkey.fader_unused' : 'launchkey.unused'
  return tipFor(action)
}

/** The catalog entry for a fader. */
export function faderTip(f: SurfaceFader): TipKey {
  if (!f.set) return 'launchkey.fader_unused'
  if (f.set.type === 'setPartVolume' || f.set.type === 'setPartPan' || f.set.type === 'setPartSend' || f.set.type === 'setStylePartSend') return tipFor(f.set)
  if (f.set.type === 'setStylePartVolume') return 'mixer.style.volume'
  if (f.set.type === 'setStyleVolume') return 'mixer.style_level'
  if (f.set.type === 'setMultiPadVolume') return 'mixer.pad_level'
  if (f.set.type === 'moveRackFader') return 'launchkey.fader_rack'
  return 'mixer.master'
}

/**
 * The command moving a fader to `v` sends: its `set` with the value filled in, in the
 * field that command names it (`pan`, `value` for a send, else `volume`). In a send layer
 * the faders move pan or sends (#409).
 */
export function faderCmd(f: SurfaceFader, v: number): AppCmd | null {
  if (!f.set) return null
  const field = f.set.type === 'setPartPan' ? 'pan' : f.set.type === 'setPartSend' || f.set.type === 'setStylePartSend' ? 'value' : 'volume'
  return { ...f.set, [field]: v } as AppCmd
}
