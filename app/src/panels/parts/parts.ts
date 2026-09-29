// Keyboard-part helpers for the Rack panel (panels/rack) and the Sound Browser. Everything
// here reads engine state and only words it for the screen; no engine behaviour is
// decided here.

import type { TipKey } from '../../help/tooltips'
import type { OtsPart, PartPlugin, PluginEntry } from '../../lib/api/types'
import { MAINS } from '../../lib/api/types'

export const PART_KEYS = ['right1', 'right2', 'right3', 'left'] as const
export const PART_SHORT = ['R1', 'R2', 'R3', 'L']

export const onTip = (i: number) => `part.${PART_KEYS[i]}.on` as TipKey
export const selectTip = (i: number) => `part.${PART_KEYS[i]}.select` as TipKey
export const volumeTip = (i: number) => `mixer.panel.${PART_KEYS[i]}` as TipKey
export const otsTip = (i: number) => `ots.${i + 1}` as TipKey

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

const STAGE: Record<string, string> = { queued: 'queued', instantiating: 'starting', initializing: 'initialising', restoringState: 'loading preset' }

/** The line under a plugin part's name: where its load is, or why it isn't playing. */
export function pluginStatusLine(p: PartPlugin | undefined, available: boolean): string {
  if (!available) return 'Plugins need the built-in synth'
  if (!p) return 'Plugin ▾'
  switch (p.status) {
    case 'loading':
      return `Loading… ${STAGE[p.stage ?? ''] ?? ''}`.trim()
    case 'failed':
      return `Failed: ${p.error ?? 'did not load'}`
    case 'muted':
      return 'Muted: the plugin stopped'
    default:
      return `${p.manufacturer}${p.outOfProcess ? ' · own process' : p.inProcessFallback ? ' · ⚠ in process' : ''} ▾`
  }
}

/**
 * The part's plugin was loaded before its "In proc" override changed, so it still runs
 * where it did: the override applies from its next load (#176). Off, a plugin loads in
 * its own process, except an Apple AUv2, which macOS runs in process either way (the
 * engine's `load_mode`).
 */
export function inProcessPending(p: PartPlugin | null | undefined, entry: PluginEntry | undefined): boolean {
  if (!p || !entry || p.status !== 'playing') return false
  if (entry.inProcess) return p.outOfProcess
  const ownProcess = !entry.id.endsWith('appl') || entry.format === 'AUv3'
  return ownProcess && !p.outOfProcess && !p.inProcessFallback
}
