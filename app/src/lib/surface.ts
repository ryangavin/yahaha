// The Launchkey surface beyond the pads: what every button and fader does on the current
// page and Shift layer, how it's lit, and the beat clock. The engine sends it as
// `state.surface` (#77, docs/app-api.md "surface"); the mocks send the same
// (lib/api/mock-surface.ts). These helpers turn it into tooltips and commands.

import { tipFor } from '../help/actions'
import type { TipKey } from '../help/tooltips'
import type { AppCmd, AppState, LibraryList, SurfaceControl, SurfaceFader, SurfaceState } from './api/types'

export { neighbours } from './api/mock-surface'

/** The surface, as the engine sends it. (`lib` is unused: kept for existing callers.) */
export function surfaceOf(s: AppState, lib?: LibraryList): SurfaceState {
  void lib
  return s.surface
}

/** A control's label and action on the layer showing. */
export function layer(c: SurfaceControl, shift: boolean): { label: string; action: AppCmd | null } {
  return shift ? { label: c.shiftLabel, action: c.shiftAction } : { label: c.label, action: c.action }
}

/** The catalog entry for a control on the layer showing. */
export function controlTip(c: SurfaceControl, shift: boolean): TipKey {
  const { action } = layer(c, shift)
  if (!shift && c.id === 'padBankUp') return 'padpage.prev'
  if (!shift && c.id === 'padBankDown') return 'padpage.next'
  if (c.id === 'trackPrev') return 'style.prev'
  if (c.id === 'trackNext') return 'style.next'
  if (!action) return c.id.startsWith('faderButton') ? 'launchkey.fader_unused' : 'launchkey.unused'
  return tipFor(action)
}

/** The catalog entry for a fader. */
export function faderTip(f: SurfaceFader): TipKey {
  if (!f.set) return 'launchkey.fader_unused'
  if (f.set.type === 'setPartVolume') return tipFor(f.set)
  if (f.set.type === 'setStylePartVolume') return 'mixer.style.volume'
  return 'mixer.master'
}

/** The command moving a fader to `v` sends. */
export function faderCmd(f: SurfaceFader, v: number): AppCmd | null {
  return f.set ? ({ ...f.set, volume: v } as AppCmd) : null
}
