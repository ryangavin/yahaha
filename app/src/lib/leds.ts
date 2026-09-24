// Drawing the Launchkey's LEDs on screen. The engine gives each pad a `Pad` (colour,
// level, animation); this turns it into a brightness on the beat clock exactly as the
// hardware does (docs/app-api.md, "Pad"), so screen and pads always agree.

import type { AppState, Pad, PadPage, Rgb } from './api/types'

/** Brightness of "dim" relative to full, as on the hardware. */
export const DIM = 0.18

/** How bright a pad is at a point on the beat clock (0–1). */
export function brightness(pad: Pick<Pad, 'level' | 'anim'>, beats: number): number {
  if (pad.level === 'off') return 0
  if (pad.level === 'dim') return DIM
  if (pad.anim === 'flash') return beats - Math.floor(beats) < 0.5 ? 1 : DIM
  if (pad.anim === 'pulse') {
    const p = beats / 2 - Math.floor(beats / 2)
    const tri = p < 0.5 ? p * 2 : 2 - p * 2
    return 0.25 + 0.75 * tri
  }
  return 1
}

/**
 * What a pad shows at a point on the LED clock: its colour and brightness. In RGB mode
 * that's `rgb` at `brightness()`. In palette mode (`pads.paletteLeds`) it's what the pad
 * was sent (`pad.palette`): a flash alternates between its two palette colours every half
 * beat, a pulse breathes; the hardware runs these on its own timing (docs/app-api.md).
 */
export function padLight(pad: Pad, paletteLeds: boolean, beats: number): { rgb: Rgb; b: number } {
  const p = paletteLeds ? pad.palette : null
  if (!p) return { rgb: pad.rgb, b: brightness(pad, beats) }
  if (p.mode === 'flash' && p.flashRgb && beats - Math.floor(beats) >= 0.5) {
    return { rgb: p.flashRgb, b: brightness({ level: p.flashLevel ?? 'off', anim: 'solid' }, beats) }
  }
  const anim = p.mode === 'flash' ? 'solid' : p.mode
  return { rgb: p.rgb, b: brightness({ level: p.level, anim }, beats) }
}

/** CSS colour for an LED colour at full brightness (0–127 per channel → 0–255). */
export function css([r, g, b]: Rgb, alpha = 1): string {
  const s = (c: number) => Math.round((c / 127) * 255)
  return `rgb(${s(r)} ${s(g)} ${s(b)} / ${alpha})`
}

export const TOP = [96, 97, 98, 99, 100, 101, 102, 103]
export const BOTTOM = [112, 113, 114, 115, 116, 117, 118, 119]

/** Page 1 notes of the section lamps (`state.transport.lamps`). */
export const LAMP = {
  intro: [96, 97, 98],
  syncStart: 99,
  ending: [100, 101, 102],
  autoFill: 103,
  main: [112, 113, 114, 115],
  break: 116,
  tap: 117,
  syncStop: 118,
  startStop: 119,
} as const

const OFF_PAD: Pad = { note: 0, label: '', key: '', rgb: [0, 0, 0], level: 'off', anim: 'solid', action: null, palette: null }

/** The section lamp on page-1 note `note`. */
export function lamp(s: AppState, note: number): Pad {
  return s.transport.lamps.find((p) => p.note === note) ?? { ...OFF_PAD, note }
}

/** Where a pad is, in words: "Pad page 2 (Chord/Setup), top row, pad 3". */
export function padLocation(page: PadPage, note: number): string {
  const n = ['sections', 'chordSetup', 'otsParts'].indexOf(page) + 1
  const name = ['Sections', 'Chord/Setup', 'OTS/Parts'][n - 1]
  const row = note >= 112 ? 'bottom' : 'top'
  const col = (note >= 112 ? note - 112 : note - 96) + 1
  return `Pad page ${n} (${name}), ${row} row, pad ${col}`
}
