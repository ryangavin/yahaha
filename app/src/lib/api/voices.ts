// The voices a keyboard part can play: the list `setPartVoice` picks from.
//
// Provisional. The engine names voices with `gm_name()` (src/api.rs) but doesn't send the
// list yet. The proposed shape is `AppState.voices` (or `LibraryList.voices`):
// `{ program: number; name: string }[]`, the GM list today, and later whatever the synth
// or a plugin can play. Until the engine sends it, this falls back to the same 128 GM
// names (the mock fixture carries the engine's list). Components read `voiceList()` only.

import type { AppState } from './types'
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

/** The voice list the engine sends (`state.voices`, proposed), else the GM list. */
export function voiceList(s: AppState): VoiceEntry[] {
  const sent = (s as AppState & { voices?: VoiceEntry[] }).voices
  return sent && sent.length ? sent : FALLBACK
}

/** Voices grouped by GM family, for `<optgroup>`s. */
export function voiceGroups(voices: VoiceEntry[]): { family: string; voices: VoiceEntry[] }[] {
  const groups = GM_FAMILIES.map((family) => ({ family, voices: [] as VoiceEntry[] }))
  for (const v of voices) groups[Math.min(15, Math.floor(v.program / 8))].voices.push(v)
  return groups.filter((g) => g.voices.length)
}
