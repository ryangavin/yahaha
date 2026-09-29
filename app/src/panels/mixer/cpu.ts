// #340: the tracks' CPU as the Mixer shows it, from the meters (`Meters.channels[].cpu`).
import type { Meters } from '../../lib/api/types'

/** A track's (or a group's) CPU: its average share of the buffer and its worst buffer. */
export interface TrackCpu {
  avg: number
  peak: number
}

/** How often the Mixer reads the meters; a reading changes once a second. */
export const CPU_POLL_MS = 500

/** A worst buffer past half the buffer's time: where dropouts start (the plugin host's
 * overrun line too). */
export const CPU_WARN = 0.5

/** The CPU of MIDI channels `channels` (1-based) together; null before any meters. */
export function cpuOf(m: Meters | null, channels: number[]): TrackCpu | null {
  if (!m || m.channels.length === 0) return null
  let avg = 0
  let peak = 0
  for (const c of m.channels) {
    if (!channels.includes(c.channel)) continue
    avg += c.cpu ?? 0
    // Tracks' worst buffers need not coincide: their sum is an upper bound.
    peak += c.cpuPeak ?? 0
  }
  return { avg, peak }
}

/** "0.4%", "12%": a share as a percentage, one decimal below 10%. */
export function pct(share: number): string {
  const p = share * 100
  return p < 10 ? `${p.toFixed(1)}%` : `${Math.round(p)}%`
}
