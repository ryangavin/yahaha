// Registration panel helpers: the button lamps (the Genos colours, as pad page 4 lights
// them in src/launchkey.rs), names and the catalog keys.

import type { TipKey } from '../../help/tooltips'
import type { RegistrationState } from '../../lib/api/registration'
import type { Anim, Level, Rgb } from '../../lib/api/types'

/** Red = the button in use, blue = stored (OM p.97); the same as the Launchkey pads. */
export const REGIST_SELECTED: Rgb = [127, 0, 0]
export const REGIST_STORED: Rgb = [0, 40, 127]

export type Look = { rgb: Rgb; level: Level; anim: Anim }

/** Button `i`'s lamp: flashing red while Memory is armed, red in use, blue stored, dark empty. */
export function buttonLook(r: RegistrationState, i: number): Look {
  const stored = r.buttons[i]?.stored ?? false
  if (r.memory) return { rgb: REGIST_SELECTED, level: 'bright', anim: 'flash' }
  if (stored && r.selected === i) return { rgb: REGIST_SELECTED, level: 'bright', anim: 'solid' }
  return { rgb: REGIST_STORED, level: stored ? 'bright' : 'off', anim: 'solid' }
}

export const REGIST_TIPS: TipKey[] = [
  'regist.1', 'regist.2', 'regist.3', 'regist.4', 'regist.5', 'regist.6', 'regist.7', 'regist.8', 'regist.9', 'regist.10',
]

/** A button's name as the bar shows it: its own, else its style, else empty. */
export function buttonName(r: RegistrationState, i: number): string {
  const b = r.buttons[i]
  if (!b?.stored) return ''
  return b.name || b.style || `Registration ${i + 1}`
}

/** "3 / 6" for the sequence position, or "– / 6" before the first step. */
export function sequenceText(r: RegistrationState): string {
  const n = r.sequence.steps.length
  if (!n) return 'no steps'
  return `${r.sequence.position === null ? '–' : r.sequence.position + 1} / ${n}`
}

/** One line for a button's contents: style, tempo, the voices that are on. */
export function buttonSummary(r: RegistrationState, i: number): string {
  const b = r.buttons[i]
  if (!b?.stored) return 'empty'
  const voices = b.voices.filter((v) => v.on && v.name).map((v) => v.name)
  return [b.style, b.tempo ? `♩=${Math.round(b.tempo)}` : null, voices.join(' + ') || null].filter(Boolean).join(' · ')
}
