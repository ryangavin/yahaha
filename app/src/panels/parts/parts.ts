// Panel-only helpers for the Keyboard parts + OTS drawer. Everything here reads engine
// state and only words it for the screen; no engine behaviour is decided here.

import type { TipKey } from '../../help/tooltips'
import type { AppState, KeyboardPart, OtsPart } from '../../lib/api/types'
import { MAINS } from '../../lib/api/types'

export const PART_KEYS = ['right1', 'right2', 'right3', 'left'] as const
export const PART_SHORT = ['R1', 'R2', 'R3', 'L']

export const onTip = (i: number) => `part.${PART_KEYS[i]}.on` as TipKey
export const selectTip = (i: number) => `part.${PART_KEYS[i]}.select` as TipKey
export const volumeTip = (i: number) => `mixer.panel.${PART_KEYS[i]}` as TipKey
export const otsTip = (i: number) => `ots.${i + 1}` as TipKey

/** The Right parts sounding together, e.g. "Right 1 + Right 2", in part order. */
export function layerOf(parts: KeyboardPart[]): number[] {
  return parts.slice(0, 3).flatMap((p, i) => (p.sounding ? [i] : []))
}

/** The layer as the screen says it. */
export function layerText(parts: KeyboardPart[]): string {
  const on = layerOf(parts)
  if (on.length === 0) return 'No right-hand voice on'
  if (on.length === 1) return `${parts[on[0]].name} alone`
  return `Layer: ${on.map((i) => PART_SHORT[i]).join(' + ')}`
}

/**
 * Who plays the keys at and below the split, as README describes the engine: Left when it
 * sounds (its voice, or the Style's Bass under Manual Bass); otherwise in Lower chord
 * detection (outside Full Keyboard) those keys only drive the chords, and anywhere else
 * the Right parts play across the whole keyboard.
 */
export function leftZone(s: AppState): { who: 'left' | 'bass' | 'chords' | 'right'; text: string } {
  const left = s.keyboardParts[3]
  if (left?.playsBass) return { who: 'bass', text: `Bass: ${left.voiceName}` }
  if (left?.sounding) return { who: 'left', text: left.voiceName }
  const fullKeyboard = s.chord.fingering === 'fullKeyboard' || s.chord.fingering === 'aiFullKeyboard'
  if (!s.chord.upper && !fullKeyboard) return { who: 'chords', text: 'Chords only' }
  return { who: 'right', text: 'Right parts' }
}

/** Where the chord detection listens, for the keyboard map. */
export function chordsWhere(s: AppState): 'left' | 'right' | 'all' {
  if (s.chord.upper) return 'right'
  const fk = s.chord.fingering === 'fullKeyboard' || s.chord.fingering === 'aiFullKeyboard'
  return fk ? 'all' : 'left'
}

/** An octave shift as a signed label: "0", "+1", "−2". */
export function octaveLabel(o: number): string {
  return o > 0 ? `+${o}` : o < 0 ? `−${-o}` : '0'
}

/** One OTS part as a card line: "Square Lead · 100 · oct −1". */
export function otsLine(o: OtsPart): string {
  const bits = [o.voiceName, String(o.volume)]
  if (o.octave) bits.push(`oct ${octaveLabel(o.octave)}`)
  return bits.join(' · ')
}

/** The Main that recalls OTS `i` under OTS Link: "Main A". */
export const linkedMain = (i: number) => MAINS[i]

/** Where a keyboard part's volume lives on the Launchkey. */
export const launchkeyPlace = (i: number) => `Fader ${i + 1}`
