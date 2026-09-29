// What a keyboard part plays, named: the twin of `api::part_sound` (src/api/parts.rs).
// The engine sends the result as `keyboardParts[i].sound` and `voiceName`; the mock
// derives them with this, and the Sounds dialog falls back on it for a part the engine
// named no Sound for.

import { originOfPresetKey, sameOrigin, type FontPreset, type GmMapRow, type Patch, type SoundTag } from './sound-library'
import type { PluginStatus } from './types'

/** What `partSound` reads of a keyboard part. */
export interface PartSoundOf {
  /** Its plugin, if it has one. */
  plugin?: { id: string; name?: string; status?: PluginStatus; preset?: string | null; presetKey?: string | null } | null
  /** The library Sound the plugin's voice names (a stored tag: its name may be stale). */
  pluginSound?: SoundTag | null
  /** Its own library patch (null under Manual Bass). */
  own?: string | null
  /** The GM program its channel plays. */
  program: number
}

export interface PartSound {
  sound: SoundTag | null
  voiceName: string
}

export interface PartSoundNames {
  /** A font preset's name (null: not known). */
  font?: (f: FontPreset) => string | null
  /** A GM program's name. */
  gm?: (program: number) => string
}

/**
 * The Sound a keyboard part plays and its voice name, from what actually sounds:
 * - a plugin loading, playing or muted: the preset it was given (unless its sound is another
 *   library sound, one the user saved), else its library sound as the library names it now,
 *   else the bare plugin. A preset (or a factory or file preset's sound) reads
 *   "<plugin> · <preset>";
 * - a plugin that failed, or none: its own SoundFont patch, else what the GM map resolves
 *   its program to (a rule's SoundFont patch, or the auto-fill's font preset); a program
 *   nothing covers (or one mapped to a plugin sound no plugin plays) has no Sound.
 */
export function partSound(of: PartSoundOf, patches: Patch[], gmMap: GmMapRow[], names: PartSoundNames = {}): PartSound {
  const library = (id: string | null | undefined) => (id?.startsWith('saved:') ? patches.find((p) => `saved:${p.id}` === id) : undefined)
  const gmName = names.gm ?? ((p: number) => `GM ${p + 1}`)
  const pl = of.plugin
  if (pl && pl.status !== 'failed') {
    const plugin = pl.name ?? pl.id
    const saved = library(of.pluginSound?.id)
    const key = pl.presetKey
    const isPreset = (k: string, p: Patch) => {
      const o = originOfPresetKey(k)
      return !!o && p.source.kind === 'plugin' && p.source.componentId === pl.id && sameOrigin(p.source.origin, o)
    }
    let sound: SoundTag
    let mine: boolean
    if (key && pl.preset && (!saved || isPreset(key, saved))) {
      sound = { id: `au:${pl.id}#${key}`, name: pl.preset }
      mine = false
    } else if (saved) {
      sound = { id: `saved:${saved.id}`, name: saved.name }
      mine = saved.source.kind === 'plugin' && (!saved.source.origin || saved.source.origin.kind === 'user')
    } else {
      sound = { id: `au:${pl.id}`, name: plugin }
      mine = true
    }
    return { sound, voiceName: mine ? sound.name : `${plugin} · ${sound.name}` }
  }
  const font = (p: Patch | undefined) => (p?.source.kind === 'soundFont' ? p : undefined)
  const own = font(of.own ? patches.find((p) => p.id === of.own) : undefined)
  if (own) return { sound: { id: `saved:${own.id}`, name: own.name }, voiceName: own.name }
  const gm = { sound: null, voiceName: gmName(of.program) }
  const r = gmMap.find((x) => x.program === of.program)?.resolved
  if (!r?.sound) return gm
  if (r.sound.startsWith('sf:')) {
    const name = (r.font && names.font?.(r.font)) || gmName(of.program)
    return { sound: { id: r.sound, name }, voiceName: name }
  }
  const mapped = font(library(r.sound))
  return mapped ? { sound: { id: `saved:${mapped.id}`, name: mapped.name }, voiceName: mapped.name } : gm
}
