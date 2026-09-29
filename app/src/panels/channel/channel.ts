// The Channel view's small tables and readings (ChannelView.svelte).

import type { TipKey } from '../../help/tooltips'
import type { InsertType } from '../../lib/api/types'

/** Strips 0–11: the 4 keyboard parts, then the 8 Style parts. */
export const STRIP_COUNT = 12

/** What an insert slot's picker offers, in order ('none' empties the slot). */
export const INSERT_KINDS: { kind: InsertType; name: string }[] = [
  { kind: 'none', name: 'None' },
  { kind: 'distortion', name: 'Distortion' },
  { kind: 'compressor', name: 'Compressor' },
  { kind: 'autoWah', name: 'Auto Wah' },
  { kind: 'tremolo', name: 'Tremolo' },
  { kind: 'rotary', name: 'Rotary' },
  { kind: 'phaser', name: 'Phaser' },
]

/** An insert's settings 1–4, by index. */
export const INSERT_SETTING_TIPS: TipKey[] = ['mixer.strip.insert_setting_1', 'mixer.strip.insert_setting_2', 'mixer.strip.insert_setting_3', 'mixer.strip.insert_setting_4']

/** Pan as the mixer shows it: "C", "L12", "R5". */
export const panText = (v: number) => (v === 64 ? 'C' : v < 64 ? `L${64 - v}` : `R${v - 64}`)

/** A linear peak (1 = full scale) as the meter's height 0–1, over the top 60 dB. */
export function meterFrac(peak: number): number {
  if (!(peak > 0)) return 0
  return Math.max(0, Math.min(1, (20 * Math.log10(peak) + 60) / 60))
}
