// The voices a keyboard part can play: the list `setPartVoice` picks from.
//
// The engine sends it with the library (`LibraryList.voices`: `{ program, bankMsb,
// bankLsb, name }[]`), the GM list today, and later whatever the synth or a plugin can
// play. Before the first library arrives this falls back to the same 128 GM names.
// Components read `voiceList()` only.

import type { LibraryList } from './types'
import { GM } from './mock'

export interface VoiceEntry {
  program: number
  name: string
}

/** The GM families, eight programs each, for grouping a voice picker. */
export const GM_FAMILIES = [
  'Piano', 'Chromatic Perc.', 'Organ', 'Guitar', 'Bass', 'Strings', 'Ensemble', 'Brass',
  'Reed', 'Pipe', 'Synth Lead', 'Synth Pad', 'Synth FX', 'Ethnic', 'Percussive', 'Sound FX',
]

const FALLBACK: VoiceEntry[] = GM.map((name, program) => ({ program, name }))

/** The voice list the engine sends (`library.voices`), else the GM list. */
export function voiceList(lib: Pick<LibraryList, 'voices'> | null | undefined): VoiceEntry[] {
  const sent = lib?.voices
  return sent && sent.length ? sent : FALLBACK
}

/** Voices grouped by GM family, for `<optgroup>`s. */
export function voiceGroups(voices: VoiceEntry[]): { family: string; voices: VoiceEntry[] }[] {
  const groups = GM_FAMILIES.map((family) => ({ family, voices: [] as VoiceEntry[] }))
  for (const v of voices) groups[Math.min(15, Math.floor(v.program / 8))].voices.push(v)
  return groups.filter((g) => g.voices.length)
}
