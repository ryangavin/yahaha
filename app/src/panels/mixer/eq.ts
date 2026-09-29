// A keyboard part's channel-strip EQ knobs (#247, `setPartEq`): each gain knob runs over
// −12..+12 dB (0–24, centre 0 dB), each frequency knob over the XG EQ frequency steps of
// its band (Data List, Table#3: low 32 Hz–2 kHz, high 500 Hz–16 kHz).

import type { PartEq } from '../../lib/api/types'

/** The XG EQ frequency table's steps, 32 Hz to 16 kHz (data 04H–3AH). */
const XG_STEPS = [
  32, 36, 40, 45, 50, 56, 63, 70, 80, 90, 100, 110, 125, 140, 160, 180, 200, 225, 250, 280, 315, 355, 400, 450, 500, 560, 630, 700, 800, 900, 1000,
  1100, 1200, 1400, 1600, 1800, 2000, 2200, 2500, 2800, 3200, 3600, 4000, 4500, 5000, 5600, 6300, 7000, 8000, 9000, 10000, 11000, 12000, 14000, 16000,
]
/** The low band's steps (32 Hz–2 kHz) and the high band's (500 Hz–16 kHz). */
export const LOW_STEPS = XG_STEPS.filter((f) => f <= 2000)
export const HIGH_STEPS = XG_STEPS.filter((f) => f >= 500)

/** The step of `steps` nearest `hz` (a frequency set elsewhere may fall between two). */
export function stepOf(steps: number[], hz: number): number {
  let best = 0
  for (let i = 1; i < steps.length; i++) if (Math.abs(steps[i] - hz) < Math.abs(steps[best] - hz)) best = i
  return best
}

/** "80", "1.2k", "10k". */
export function hzText(hz: number): string {
  if (hz < 1000) return String(hz)
  const k = hz / 1000
  return `${Number.isInteger(k) ? k : k.toFixed(1)}k`
}

/** "0", "+3", "−6" (dB). */
export function dbText(db: number): string {
  return db === 0 ? '0' : db > 0 ? `+${db}` : `−${-db}`
}

/** A knob's value for a gain, and back. */
export const GAIN_KNOB_MAX = 24
export const gainKnob = (db: number) => db + 12
export const knobGain = (v: number) => v - 12

/** The EQ with one field changed. */
export function withEq(eq: PartEq, change: Partial<PartEq>): PartEq {
  return { ...eq, ...change }
}
