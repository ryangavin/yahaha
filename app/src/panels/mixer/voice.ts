// How a mixer strip words a part's voice. Display only: it splits the engine's own label
// (src/api.rs `voice_label`) into what the synth plays and what the style was written
// for, and never works out a voice itself.

import type { KeyboardPart, PartPlugin, Voice } from '../../lib/api/types'

export interface VoiceLines {
  /** What the channel plays, e.g. "≈ Strings", "Finger Bass", "Drum kit". */
  plays: string
  /** What it was written for: a Yamaha bank MSB/LSB/program "104/0/49", "GM 34", "kit 127/0/1" ('' if unknown). */
  writtenFor: string
}

/** A style part's voice from the engine's `Voice.label`. */
export function styleVoice(v: Voice | null): VoiceLines {
  if (!v) return { plays: '—', writtenFor: '' }
  const label = v.label.trim()
  let m = /^(≈\s*.+?)\s*\[Yamaha ([^\]]+)\]$/.exec(label)
  if (m) return { plays: m[1], writtenFor: m[2] }
  m = /^(.+?)\s*\((GM \d+)\)$/.exec(label)
  if (m) return { plays: m[1], writtenFor: m[2] }
  m = /^drum kit (.+)$/i.exec(label)
  if (m) return { plays: 'Drum kit', writtenFor: `kit ${m[1]}` }
  return { plays: label || '—', writtenFor: '' }
}

/** A keyboard part's voice: its GM voice, or the Style's Bass voice under Manual Bass. */
export function partVoice(p: KeyboardPart): VoiceLines {
  if (p.playsBass) return { plays: p.voiceName, writtenFor: 'Style Bass (Manual Bass)' }
  if (p.plugin?.status === 'playing' || p.plugin?.status === 'muted') return { plays: p.plugin.name, writtenFor: p.plugin.manufacturer }
  return { plays: p.voiceName, writtenFor: `GM ${p.program + 1}` }
}

/** The mixer strip's plugin badge: the plugin is what the fader controls, with its state. */
export function pluginBadge(p: PartPlugin): string {
  switch (p.status) {
    case 'loading':
      return 'Plugin loading…'
    case 'failed':
      return 'Plugin failed'
    case 'muted':
      return 'Plugin muted'
    default:
      return `Plugin · ${Math.round(p.cpu * 100)}% CPU`
  }
}
