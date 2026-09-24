// The Multi Pad panel's helpers: lamp colours and text, the keys, the bank list grouping.

import type { MultiPadBankEntry, PadLamp, Rgb } from '../../lib/api/types'
import { brightness } from '../../lib/leds'

const BLUE: Rgb = [20, 60, 127]
const RED: Rgb = [127, 12, 8]
const AMBER: Rgb = [127, 80, 10]

/** The Genos lamp colours (OM p.74), plus amber for a press waiting for the bar line. */
export function lampLight(lamp: PadLamp, beats: number): { rgb: Rgb; b: number } {
  switch (lamp) {
    case 'empty':
      return { rgb: BLUE, b: 0 }
    case 'ready':
      return { rgb: BLUE, b: brightness({ level: 'dim', anim: 'solid' }, beats) }
    case 'playing':
      return { rgb: RED, b: 1 }
    case 'armed':
      return { rgb: RED, b: brightness({ level: 'bright', anim: 'flash' }, beats) }
    case 'queued':
      return { rgb: AMBER, b: brightness({ level: 'bright', anim: 'flash' }, beats) }
  }
}

export const LAMP_TEXT: Record<PadLamp, string> = {
  empty: 'empty',
  ready: 'ready',
  playing: 'playing',
  armed: 'Synchro Start standby',
  queued: 'starts at the next bar',
}

/** The pads' keys (README "Terminal keys"): Shift + z x c v. */
export const PAD_KEYS = ['Z', 'X', 'C', 'V']

/** Banks by folder, in list order. */
export function byFolder(banks: MultiPadBankEntry[]): { folder: string; banks: MultiPadBankEntry[] }[] {
  const out: { folder: string; banks: MultiPadBankEntry[] }[] = []
  for (const b of banks) {
    const last = out[out.length - 1]
    if (last && last.folder === b.folder) last.banks.push(b)
    else out.push({ folder: b.folder, banks: [b] })
  }
  return out
}
