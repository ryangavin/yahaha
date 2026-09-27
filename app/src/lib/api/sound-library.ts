// The sound library (#103): patches, the program map, and what the current style uses.
// Mirrors `SoundLibraryCmd` / `SoundLibraryState` (docs/app-api.md, "Sound library").

/** The Genos voice categories. */
export type PatchCategory =
  | 'piano'
  | 'ePiano'
  | 'organ'
  | 'guitar'
  | 'bass'
  | 'strings'
  | 'brass'
  | 'saxWoodwind'
  | 'synthLead'
  | 'pad'
  | 'choir'
  | 'drumsPerc'
  | 'sfx'

/** Where a plugin sound came from (docs/sound-browser.md): made in yahaha (absent or
 * `user`), a factory preset by number, or an imported `.aupreset` file by path. */
export type PluginOrigin = { kind: 'user' } | { kind: 'factory'; number: number } | { kind: 'file'; path: string }

/** Where a patch's sound comes from: a SoundFont preset (bank 128 = drum kits), or an
 * Audio Unit with its saved state (base64). A plugin source is the one kind of plugin
 * sound; an empty state on a factory preset means it has not played yet (it is captured
 * the first time it does). */
export type PatchSource =
  | { kind: 'soundFont'; file: string; bank: number; program: number }
  | { kind: 'plugin'; componentId: string; state: string; origin?: PluginOrigin }

/** A SoundFont preset as a map resolution records it (D6 provenance). */
export interface FontPreset {
  file: string
  bank: number
  program: number
}

/** A record's Sound: its id (`sf:<file>:<bank>:<program>` or `saved:<patch id>`) and name. */
export interface SoundTag {
  id: string
  name: string
}

/** The origin a Sound Browser preset key names: `f:<number>` or `u:<path>`. */
export function originOfPresetKey(key: string): PluginOrigin | undefined {
  if (key.startsWith('f:')) {
    const number = Number(key.slice(2))
    return Number.isInteger(number) ? { kind: 'factory', number } : undefined
  }
  return key.startsWith('u:') && key.length > 2 ? { kind: 'file', path: key.slice(2) } : undefined
}

/** Same factory preset or file (a `user` origin never matches: two sounds made in yahaha
 * are two sounds). */
export function sameOrigin(a: PluginOrigin | undefined, b: PluginOrigin): boolean {
  if (!a || a.kind === 'user' || b.kind === 'user') return false
  return a.kind === 'factory' ? b.kind === 'factory' && a.number === b.number : b.kind === 'file' && a.path === b.path
}

/** Sent as CCs when the patch is picked (0–127 or null: leave as is). */
export interface PatchDefaults {
  volume: number | null
  pan: number | null
  reverb: number | null
  chorus: number | null
  /** −2..2 */
  octave: number
}

export interface PatchFields {
  name: string
  category: PatchCategory
  tags: string[]
  favourite: boolean
  source: PatchSource
  defaults: PatchDefaults
}

export interface Patch extends PatchFields {
  id: string
}

export interface PatchInfo extends Patch {
  /** It plays itself; false: the SoundFont fallback (`note` says why). */
  available: boolean
  note: string | null
}

export interface ProgramMap {
  /** 16 GM families (family i = programs 8i..8i+7): a patch id or null. */
  families: (string | null)[]
  overrides: { program: number; patch: string }[]
  drums: string | null
}

export type RuleKind = 'drums' | 'override' | 'family' | 'fallback'

/** A program the current style sends a Style part, and what it plays. */
export interface ProgramUse {
  /** 9–16 */
  channel: number
  part: string
  msb: number
  lsb: number
  program: number
  /** What the map looks up. */
  gmProgram: number
  /** The voice without the library. */
  voice: string
  drums: boolean
  patch: string | null
  rule: RuleKind
  fromStyle: boolean
  /** What sounds: the patch's name, or `voice`. */
  plays: string
}

export interface Preset {
  bank: number
  program: number
  name: string
}

export interface SoundLibraryState {
  patches: PatchInfo[]
  categories: { id: PatchCategory; label: string }[]
  families: string[]
  map: ProgramMap
  styleMap: ProgramMap
  styleKey: string
  usage: ProgramUse[]
  portSendsMapped: boolean
  /** The patch id being auditioned, or 'preset'. */
  auditioning: string | null
  browse: { file: string; presets: Preset[]; error: string | null } | null
  /** Where the library is saved; null: nowhere. */
  file: string | null
  extraSoundFonts: string[]
  lastAdded: string | null
  /** The GM map for the style playing (docs/sound-browser.md): the drums row, then
   * programs 0–127. */
  gmMap: GmMapRow[]
}

/** The GM map layer that decided a program (`patches::Layer`). */
export type GmLayer = 'drums' | 'override' | 'family' | 'auto' | 'none'

/** A font preset: file, SoundFont bank (128 = kits) and program (D6 provenance). */
export interface FontPreset {
  file: string
  bank: number
  program: number
}

/** What a program resolved to (`patches::GmResolution`). */
export interface GmResolution {
  /** A Sound id (`saved:<patch>` for a rule, `sf:<file>:<bank>:<program>` for auto); null:
   * nothing covers it. */
  sound: string | null
  layer: GmLayer
  /** The rule is the style's own. */
  fromStyle: boolean
  /** The font preset that plays, when a font does. */
  font: FontPreset | null
}

/** One row of the map page (`patches::GmMapRow`). */
export interface GmMapRow {
  /** 0–127; null for the drums row. */
  program: number | null
  /** 0–15; null for the drums row. */
  family: number | null
  overrideRule: string | null
  /** The family's rule; on the drums row, the drum rule. */
  familyRule: string | null
  resolved: GmResolution
}

export type SoundLibraryCmd =
  | { type: 'createPatch'; patch: PatchFields }
  | { type: 'updatePatch'; id: string; patch: PatchFields }
  | { type: 'deletePatch'; id: string }
  | { type: 'duplicatePatch'; id: string }
  | { type: 'movePatch'; id: string; to: number }
  | { type: 'setPatchFavourite'; id: string; favourite: boolean }
  /** Save: the part's sound as it plays now over the user's own Sound it plays (else as
   * a new one, as `saveSoundAs`). */
  | { type: 'saveSound'; part: number }
  /** Save as…: the part's sound as a new Sound, which the part then plays. */
  | { type: 'saveSoundAs'; part: number; name: string | null }
  /** The old "Save as patch": `saveSoundAs` (kept for older clients). */
  | { type: 'savePartAsPatch'; part: number; name: string | null }
  | { type: 'addPresetAsPatch'; file: string; bank: number; program: number; name: string | null }
  /** Plays it on its own for a moment (the band must be stopped). */
  | { type: 'auditionPatch'; id: string }
  | { type: 'auditionPreset'; file: string; bank: number; program: number }
  | { type: 'stopPatchAudition' }
  /** `style`: the current style's own map (may be left out: the global map). */
  | { type: 'setFamilyRule'; family: number; patch: string | null; style?: boolean }
  | { type: 'setProgramOverride'; program: number; patch: string | null; style?: boolean }
  | { type: 'setDrumRule'; patch: string | null; style?: boolean }
  | { type: 'clearStyleMap' }
  /** A keyboard part plays a library patch (null: its GM voice again). */
  | { type: 'setPartPatch'; part: number; id: string | null }
  | { type: 'setPortSendsMapped'; on: boolean }
  | { type: 'browseSoundFont'; file: string | null }
  | { type: 'importSoundLibrary'; path: string; replace?: boolean; maps?: boolean }
  /** Writes the library as a bundle (metadata, maps, every plugin sound's state; SoundFonts by file name). */
  | { type: 'exportSoundLibrary'; path: string | null }
  /** Exports a plugin sound as an `.aupreset` in its plugin's preset folder (Logic reads it); an existing one needs `overwrite`. */
  | { type: 'exportSoundPreset'; id: string; overwrite?: boolean }

export const CATEGORY_LABELS: Record<PatchCategory, string> = {
  piano: 'Piano',
  ePiano: 'E.Piano',
  organ: 'Organ',
  guitar: 'Guitar',
  bass: 'Bass',
  strings: 'Strings',
  brass: 'Brass',
  saxWoodwind: 'Sax/Woodwind',
  synthLead: 'Synth Lead',
  pad: 'Pad',
  choir: 'Choir',
  drumsPerc: 'Drums/Perc',
  sfx: 'SFX',
}

export const FAMILY_NAMES = [
  'Piano',
  'Chromatic Perc.',
  'Organ',
  'Guitar',
  'Bass',
  'Strings',
  'Ensemble',
  'Brass',
  'Reed',
  'Pipe',
  'Synth Lead',
  'Synth Pad',
  'Synth FX',
  'Ethnic',
  'Percussive',
  'Sound FX',
]

export function emptyMap(): ProgramMap {
  return { families: Array(16).fill(null), overrides: [], drums: null }
}
