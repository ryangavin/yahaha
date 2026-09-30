// The Channel view's small tables and readings (ChannelView.svelte).

import type { TipKey } from '../../help/tooltips'
import type { InsertType, ToneControl } from '../../lib/api/types'

/** The Tone card's knobs (a keyboard strip's voice settings, `setStripTone`), in order. */
export const TONE_KNOBS: { control: ToneControl; caption: string; tip: TipKey }[] = [
  { control: 'cutoff', caption: 'Cutoff', tip: 'mixer.channel.tone.cutoff' },
  { control: 'resonance', caption: 'Reso', tip: 'mixer.channel.tone.resonance' },
  { control: 'attack', caption: 'Attack', tip: 'mixer.channel.tone.attack' },
  { control: 'decay', caption: 'Decay', tip: 'mixer.channel.tone.decay' },
  { control: 'release', caption: 'Release', tip: 'mixer.channel.tone.release' },
  { control: 'vibratoRate', caption: 'Vib rate', tip: 'mixer.channel.tone.vibrato_rate' },
  { control: 'vibratoDepth', caption: 'Vib depth', tip: 'mixer.channel.tone.vibrato_depth' },
  { control: 'vibratoDelay', caption: 'Vib delay', tip: 'mixer.channel.tone.vibrato_delay' },
]

/** A voice setting relative to the voice's own (64): "0", "+10", "−4". */
export const toneText = (v: number) => (v === 64 ? '0' : v > 64 ? `+${v - 64}` : `−${64 - v}`)

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
