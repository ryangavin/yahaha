// The sound catalog (#117, docs/app-api.md "Sound catalog"): every SoundFont preset,
// instrument plugin and saved sound as one list for the Sound Browser. The list is fetched
// (`session.sounds()`) whenever `state.sounds.revision` moves; it is not in the state.
//
// Entry ids: `sf:<file>:<bank>:<program>`, `au:<component id>`, `saved:<patch id>`, and a
// plugin's preset `au:<component id>#<key>` (`f:<number>` factory, `u:<path>` .aupreset).

import type { PatchCategory } from './sound-library'

export type SoundsCmd =
  /** Mark or unmark a favourite (a saved sound's is its patch's `favourite`). */
  | { type: 'setSoundFavourite'; id: string; on: boolean }
  /** Keyboard part `part` (0-3) plays the sound, by the route its source has. */
  | { type: 'assignSound'; part: number; id: string }
  /** As assignSound, keeping the part's mix (level, pan, sends, octave, voice settings,
   * bend range, on/off): Library › Replace… for a part whose plugin is missing. */
  | { type: 'replacePartSound'; part: number; id: string }
  /** A plugin's, plugin preset's or saved sound's category (a SoundFont preset's is its GM family). */
  | { type: 'setSoundCategory'; id: string; category: PatchCategory }
  /** List a plugin's (`au:<id>`) presets: the browser expanded it. */
  | { type: 'listPluginPresets'; id: string }
  /** Add a font preset, plugin or plugin preset to My Sounds (the library), once; nothing plays it. */
  | { type: 'addToMySounds'; id: string }
  /** Save the part's plugin as it plays now as an .aupreset (Logic reads it too), filed under `category`. */
  | { type: 'savePartAsPluginPreset'; part: number; name: string; category: PatchCategory; overwrite?: boolean }

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
  /** Plugins (and plugin presets) only. `presets`: how many it has, null (unknown) while
   * its factory presets were never listed (listPluginPresets), even with .aupreset files
   * in. `presetsError`: why listing them failed; not tried again until the next scan. */
  plugin: { format: string; lastError: string | null; presets?: number | null; presetsError?: string } | null
  /** A plugin preset's plugin (`au:<id>`): listed under it. */
  parent?: string | null
}

/** A plugin's presets, for the catalog (as `api::PluginPresetList`). */
export interface PluginPresetList {
  plugin: string
  listed: boolean
  presets: { key: string; name: string; folder: string | null }[]
  /** Listing its factory presets failed: why. */
  error?: string
}

export interface SoundCatalog {
  /** `state.sounds.revision` when it was built. */
  revision: number
  entries: SoundEntry[]
  /** Ids last assigned, most recent first (at most 20). */
  recents: string[]
  /** Each SoundFont in the folder, for the Instruments tab (absent: an engine before it). */
  fonts?: FontSummary[]
}

/** A SoundFont as the Instruments tab shows it (as `api::FontSummary`). */
export interface FontSummary {
  file: string
  /** Melodic presets (banks below 128). */
  presets: number
  /** Drum kits (bank 128). */
  kits: number
  /** GM programs it has on bank 0, of 128. */
  gmPrograms: number
  gmKit: boolean
}

/** Each font's summary, as `api::font_summaries` (`patches::gm::gm_completeness`). */
export function fontSummaries(fonts: [string, { bank: number; program: number }[]][]): FontSummary[] {
  return fonts.map(([file, presets]) => {
    const kits = presets.filter((p) => p.bank >= 128).length
    const gm = new Set(presets.filter((p) => p.bank === 0).map((p) => p.program & 127))
    return { file, presets: presets.length - kits, kits, gmPrograms: gm.size, gmKit: kits > 0 }
  })
}

/** `state.sounds`: the catalog's summary. */
export interface SoundsState {
  revision: number
  count: number
  /** Plugins are being scanned: more may come. */
  scanning: boolean
  /** Plugins (`au:<id>`) whose presets are being listed. */
  listingPresets?: string[]
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

/** A plugin preset's catalog id. */
export function pluginPresetId(component: string, key: string): string {
  return `au:${component}#${key}`
}

/** A plugin entry id's component id and preset key, as `api::parse_plugin_id`. */
export function parsePluginId(id: string): { plugin: string; key: string | null } | null {
  if (!id.startsWith('au:')) return null
  const rest = id.slice(3)
  const i = rest.indexOf('#')
  return i < 0 ? { plugin: rest, key: null } : { plugin: rest.slice(0, i), key: rest.slice(i + 1) }
}

/** A plugin's likely category from its name and maker, as `api::plugin_category`. */
export function pluginCategory(name: string, maker: string): PatchCategory {
  return guessCategory(`${name} ${maker}`) ?? 'synthLead'
}

/** The category words in `text` suggest, if any, as `api::guess_category`. */
export function guessCategory(text: string): PatchCategory | null {
  const n = text.toLowerCase()
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
  if (has(['synth', 'lead'])) return 'synthLead'
  return null
}
