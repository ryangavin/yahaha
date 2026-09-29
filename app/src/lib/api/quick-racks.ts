// Quick Racks (docs/racks.md): banks A–H of eight one-press buttons, each one of the
// user's racks. The wire types are in types.ts (`QuickRackCmd`, `QuickRacksState`); these
// are the shared helpers, as src/racks/quick.rs names things.

import type { Anim, Level, QuickRacksState, Rgb } from './types'

/** Banks A–H. */
export const QUICK_BANKS = 8
/** Buttons per bank. */
export const QUICK_SLOTS = 8

/** "A" for bank 0. */
export const bankLetter = (bank: number) => String.fromCharCode(65 + bank)
/** "A3" for bank 0, slot 2. */
export const quickLabel = (bank: number, slot: number) => `${bankLetter(bank)}${slot + 1}`

/** Red = the loaded rack's button, blue = stored (the Registration colours, OM p.97); the
 * same as pad page 4 (src/launchkey.rs `quick_looks`). */
export const QUICK_LOADED: Rgb = [127, 0, 0]
export const QUICK_STORED: Rgb = [0, 40, 127]

export type Look = { rgb: Rgb; level: Level; anim: Anim }

/** Button `slot`'s lamp: flashing red while Store is armed, red when its rack is loaded,
 * blue when it holds a rack, dark when empty. */
export function quickLook(q: QuickRacksState, slot: number): Look {
  const b = q.buttons[slot]
  const stored = !!b?.rack
  if (q.store) return { rgb: QUICK_LOADED, level: 'bright', anim: 'flash' }
  if (stored && b.loaded) return { rgb: QUICK_LOADED, level: 'bright', anim: 'solid' }
  return { rgb: QUICK_STORED, level: stored ? 'bright' : 'off', anim: 'solid' }
}

/** What a button shows under it: its rack's name, "missing" or "empty". */
export function quickName(q: QuickRacksState, slot: number): string {
  const b = q.buttons[slot]
  if (!b?.rack) return 'empty'
  return b.missing ? 'missing' : b.name
}

/** Quick Racks as the engine starts with no file: bank A, eight empty buttons. */
export function emptyQuickRacks(): QuickRacksState {
  return {
    bank: 0,
    buttons: Array.from({ length: QUICK_SLOTS }, () => ({ rack: null, name: '', missing: false, loaded: false })),
    store: false,
    storeWaiting: null,
    readOnly: false,
  }
}
