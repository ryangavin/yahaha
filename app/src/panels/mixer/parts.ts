// The mixer row's 12 parts, all equal: strip 0–3 are the keyboard parts (Right 1, Right 2,
// Right 3, Left: `state.keyboardParts`), 4–11 the Style parts (Rhythm 1 … Phrase 2:
// `state.mixer.styleParts[i - 4]`). Pure helpers only; the strips send the commands.

import type { Meters } from '../../lib/api/types'

/** How many strips the mixer row has. */
export const STRIP_COUNT = 12
/** Strips 0–3 are the keyboard parts. */
export const KEYBOARD_STRIPS = 4
/** Every strip index, in row order. */
export const STRIPS: readonly number[] = Array.from({ length: STRIP_COUNT }, (_, i) => i)

/** Each part's colour, by strip index: the part's identity in both themes (the wireframe's). */
export const PART_COLORS: readonly string[] = [
  '#ff5a5a',
  '#ff9a3c',
  '#ffd23f',
  '#9be15d',
  '#3fd6c6',
  '#3fa9ff',
  '#7a7dff',
  '#c46bff',
  '#ff6bd0',
  '#ff8c8c',
  '#5de0ff',
  '#b8f05d',
]

/** Strip `i` is a keyboard part (0–3). */
export const isKeyboard = (i: number): boolean => i >= 0 && i < KEYBOARD_STRIPS

/** The Style part (0–7) strip `i` shows; only for i ≥ 4. */
export const stylePartOf = (i: number): number => i - KEYBOARD_STRIPS

/** How often the mixer row reads the meters. */
export const METER_POLL_MS = 100

/** The meter's floor: −60 dBFS reads as empty. */
const METER_FLOOR_DB = -60

/** Channel `channel`'s (1-based) peak level, or null when there are no meters (no session,
 * or no synth: `channels` empty). A channel missing from a live reading is silent (0). */
export function levelOf(meters: Meters | null, channel: number): number | null {
  if (!meters || meters.channels.length === 0) return null
  return meters.channels.find((c) => c.channel === channel)?.peak ?? 0
}

/** How full the meter is (0–1) for a linear peak: dB from −60 to 0. */
export function meterFill(peak: number): number {
  if (!(peak > 0)) return 0
  const db = 20 * Math.log10(peak)
  return Math.max(0, Math.min(1, (db - METER_FLOOR_DB) / -METER_FLOOR_DB))
}
