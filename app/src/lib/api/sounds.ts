// The sound catalog (#117, docs/app-api.md "Sound catalog"): every SoundFont preset,
// instrument plugin and saved sound as one list for the Sound Browser. The list is fetched
// (`session.sounds()`) whenever `state.sounds.revision` moves; it is not in the state.
//
// Entry ids: `sf:<file>:<bank>:<program>`, `au:<component id>`, `saved:<patch id>`.

import type { PatchCategory } from './sound-library'

export type SoundsCmd =
  /** Mark or unmark a favourite (a saved sound's is its patch's `favourite`). */
  | { type: 'setSoundFavourite'; id: string; on: boolean }
  /** Play a sound on its own for a moment (the band must be stopped). */
  | { type: 'auditionSound'; id: string }
  | { type: 'stopSoundAudition' }
  /** Keyboard part `part` (0-3) plays the sound, by the route its source has. */
  | { type: 'assignSound'; part: number; id: string }
  /** A plugin's or a saved sound's category (a preset's is its GM family). */
  | { type: 'setSoundCategory'; id: string; category: PatchCategory }

export type SoundSource = 'soundFont' | 'plugin' | 'saved'

export interface SoundEntry {
  id: string
  name: string
  category: PatchCategory
  source: SoundSource
  /** The SoundFont file, the plugin's maker, or what a saved sound plays. */
  detail: string
  favourite: boolean
  recent: boolean
  /** Plugins only. */
  plugin: { format: string; lastError: string | null } | null
}

export interface SoundCatalog {
  /** `state.sounds.revision` when it was built. */
  revision: number
  entries: SoundEntry[]
  /** Ids last assigned, most recent first (at most 20). */
  recents: string[]
}

/** `state.sounds`: the catalog's summary. */
export interface SoundsState {
  revision: number
  count: number
  /** Plugins are being scanned: more may come. */
  scanning: boolean
  /** The id being auditioned. */
  auditioning: string | null
}

export const MAX_RECENTS = 20

export function presetId(file: string, bank: number, program: number): string {
  return `sf:${file}:${bank}:${program}`
}

/** A preset id's parts (the file may hold ':' itself), as `api::parse_preset_id`. */
export function parsePresetId(id: string): { file: string; bank: number; program: number } | null {
  const m = /^sf:(.+):(\d+):(\d+)$/.exec(id)
  if (!m) return null
  const program = Number(m[3])
  return program < 128 ? { file: m[1], bank: Number(m[2]), program } : null
}

/** A plugin's likely category from its name and maker, as `api::plugin_category`. */
export function pluginCategory(name: string, maker: string): PatchCategory {
  const n = `${name} ${maker}`.toLowerCase()
  const has = (words: string[]) => words.some((w) => n.includes(w))
  if (has(['rhodes', 'wurli', 'e.piano', 'epiano', 'electric piano', 'e-piano', 'ep-', 'clav'])) return 'ePiano'
  if (has(['piano', 'grand', 'keyscape', 'pianoteq'])) return 'piano'
  if (has(['organ', 'b-3', 'b3', 'hammond', 'vox continental', 'farfisa'])) return 'organ'
  if (has(['drum', 'perc', 'kit', 'beat', 'battery', '808', '909'])) return 'drumsPerc'
  if (has(['bass'])) return 'bass'
  if (has(['guitar', 'strum'])) return 'guitar'
  if (has(['string', 'violin', 'cello', 'orchestra', 'symphon'])) return 'strings'
  if (has(['brass', 'trumpet', 'horn', 'trombone'])) return 'brass'
  if (has(['sax', 'flute', 'clarinet', 'oboe', 'wind'])) return 'saxWoodwind'
  if (has(['choir', 'vocal', 'voice', 'vox'])) return 'choir'
  if (has(['pad', 'ambient', 'atmos'])) return 'pad'
  return 'synthLead'
}
