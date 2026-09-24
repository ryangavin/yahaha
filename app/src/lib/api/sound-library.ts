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

/** Where a patch's sound comes from: a SoundFont preset (bank 128 = drum kits), or an
 * Audio Unit with its saved state (base64). */
export type PatchSource =
  | { kind: 'soundFont'; file: string; bank: number; program: number }
  | { kind: 'plugin'; componentId: string; state: string }

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
}

export type SoundLibraryCmd =
  | { type: 'createPatch'; patch: PatchFields }
  | { type: 'updatePatch'; id: string; patch: PatchFields }
  | { type: 'deletePatch'; id: string }
  | { type: 'duplicatePatch'; id: string }
  | { type: 'movePatch'; id: string; to: number }
  | { type: 'setPatchFavourite'; id: string; favourite: boolean }
  /** A keyboard part's sound as a new patch. */
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
  | { type: 'exportSoundLibrary'; path: string | null }

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
