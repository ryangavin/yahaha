// The Sound Browser's Sounds tab (#117, docs/sound-browser.md "The Sounds tab"): which
// catalog entries a chip and the filter show, in what order. Pure functions over
// `app.sounds` and the sound library, so they are cheap to test.
//
// "All sounds" is not every preset of every font (about 1,200 rows, mostly GM
// duplicates). It is the GM map's resolved sounds (one row per program and the kit), every
// plugin sound and every sound in My Sounds: the library's patches. Every other font
// preset, and a plugin's factory presets and .aupreset files, are reached through that
// instrument's chip.

import { CATEGORY_LABELS } from '../../lib/api/sound-library'
import type { GmMapRow, PatchCategory, PatchInfo, SoundCatalog, SoundEntry, SoundSource } from '../../lib/api/types'

/** A chip. An instrument is `sf:<file>` (a SoundFont) or `au:<component id>` (a plugin). */
export type SoundView =
  | { kind: 'all' }
  | { kind: 'favourites' }
  | { kind: 'recents' }
  | { kind: 'mine' }
  | { kind: 'category'; id: PatchCategory }
  | { kind: 'instrument'; id: string }

export const SOURCE_BADGE: Record<SoundSource, string> = { soundFont: 'SF', plugin: 'AU', saved: 'Mine' }

/** An instrument chip. */
export interface Instrument {
  id: string
  name: string
  kind: 'soundFont' | 'plugin'
}

/** What the browser needs besides the catalog: the library's patches and the GM map. */
export interface SoundContext {
  patches: PatchInfo[]
  gmMap: GmMapRow[]
}

/** The instrument a library patch plays. */
export function patchInstrument(p: PatchInfo): string {
  return p.source.kind === 'plugin' ? `au:${p.source.componentId}` : `sf:${p.source.file}`
}

/** The instrument an entry belongs to (its chip), if any. */
export function instrumentOf(e: SoundEntry, patches: ReadonlyMap<string, PatchInfo>): string | null {
  if (e.source === 'soundFont') return `sf:${e.detail}`
  if (e.source === 'plugin') return e.parent ?? e.id
  const p = patches.get(e.id)
  return p ? patchInstrument(p) : null
}

/** One chip per instrument: the SoundFonts in the catalog's order, then the plugins. */
export function instruments(catalog: SoundCatalog): Instrument[] {
  const out: Instrument[] = []
  const seen = new Set<string>()
  for (const e of catalog.entries) {
    if (e.source === 'soundFont' && !seen.has(`sf:${e.detail}`)) {
      seen.add(`sf:${e.detail}`)
      out.push({ id: `sf:${e.detail}`, name: e.detail.replace(/\.sf2$/i, ''), kind: 'soundFont' })
    }
  }
  for (const e of catalog.entries) {
    if (e.source === 'plugin' && !e.parent && !seen.has(e.id)) {
      seen.add(e.id)
      out.push({ id: e.id, name: e.name, kind: 'plugin' })
    }
  }
  return out
}

/** The ids "All sounds" holds: the map's resolved sounds and every library sound. */
export function allSoundIds(ctx: SoundContext): Set<string> {
  const ids = new Set<string>()
  for (const r of ctx.gmMap) if (r.resolved.sound) ids.add(r.resolved.sound)
  for (const p of ctx.patches) ids.add(`saved:${p.id}`)
  return ids
}

/** The GM map's program (or 'drums') each id resolves for, first row wins: the row's
 * detail in All sounds says which program it covers. */
export function mapSlots(gmMap: GmMapRow[]): Map<string, number | 'drums'> {
  const at = new Map<string, number | 'drums'>()
  for (const r of gmMap) if (r.resolved.sound && !at.has(r.resolved.sound)) at.set(r.resolved.sound, r.program ?? 'drums')
  return at
}

/** Entries by id, for the library's patches. */
export function patchesById(patches: PatchInfo[]): Map<string, PatchInfo> {
  return new Map(patches.map((p) => [`saved:${p.id}`, p]))
}

/** The categories in the Genos order, each with how many of `entries` it holds. */
export function categoryCounts(entries: SoundEntry[]): { id: PatchCategory; label: string; count: number }[] {
  const n = new Map<PatchCategory, number>()
  for (const e of entries) n.set(e.category, (n.get(e.category) ?? 0) + 1)
  return (Object.keys(CATEGORY_LABELS) as PatchCategory[]).map((id) => ({ id, label: CATEGORY_LABELS[id], count: n.get(id) ?? 0 }))
}

/**
 * The entries shown, as indices into `catalog.entries`: the chip, then the filter (every
 * word must be in the name, the detail or the source badge). Recents come in their order
 * (most recent first); everything else in the catalog's order.
 *
 * All sounds and a category hold only All sounds' members (`allSoundIds`); Favourites and
 * Recents hold whatever you starred or picked; an instrument's chip holds all its presets
 * and sounds.
 */
export function visibleSounds(catalog: SoundCatalog, view: SoundView, query: string, ctx: SoundContext = { patches: [], gmMap: [] }): number[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean)
  const match = (e: SoundEntry) => {
    if (!words.length) return true
    const hay = `${e.name} ${e.detail} ${SOURCE_BADGE[e.source]}`.toLowerCase()
    return words.every((w) => hay.includes(w))
  }
  const { entries } = catalog
  if (view.kind === 'recents') {
    const at = new Map(entries.map((e, i) => [e.id, i]))
    return catalog.recents.flatMap((id) => {
      const i = at.get(id)
      return i !== undefined && match(entries[i]) ? [i] : []
    })
  }
  const all = view.kind === 'all' || view.kind === 'category' ? allSoundIds(ctx) : null
  const byId = view.kind === 'instrument' ? patchesById(ctx.patches) : null
  const keep = (e: SoundEntry) => {
    switch (view.kind) {
      case 'all': return all!.has(e.id)
      case 'category': return all!.has(e.id) && e.category === view.id
      case 'favourites': return e.favourite
      case 'mine': return e.source === 'saved'
      case 'instrument': return instrumentOf(e, byId!) === view.id
      default: return true
    }
  }
  return entries.flatMap((e, i) => (keep(e) && match(e) ? [i] : []))
}

/** A keyboard part, as the browser reads it. */
export interface PlayingPart {
  program: number
  patch: string | null
  plugin?: { id: string; name?: string; presetKey?: string | null } | null
  playsBass: boolean
  sound?: { id: string; name: string } | null
}

/** The entry a keyboard part plays now: the Sound its state names; else its plugin
 * preset or plugin; else what the GM map resolves its program to; else its voice on the
 * synth's main font. */
export function playingId(p: PlayingPart, soundFontFile: string | null, gmMap: GmMapRow[] = []): string | null {
  if (p.sound) return p.sound.id
  if (p.patch) return `saved:${p.patch}`
  if (p.plugin?.presetKey) return `au:${p.plugin.id}#${p.plugin.presetKey}`
  if (p.plugin) return `au:${p.plugin.id}`
  const row = gmMap.find((r) => r.program === p.program)
  if (row?.resolved.sound) return row.resolved.sound
  return soundFontFile ? `sf:${soundFontFile}:0:${p.program}` : null
}

/** The footer's "<Part> plays <instrument> · <sound>": the instrument's name. */
export function instrumentName(p: PlayingPart, ctx: SoundContext, plugins: { id: string; name: string }[], soundFontFile: string | null): string {
  if (p.plugin) return p.plugin.name ?? plugins.find((x) => x.id === p.plugin!.id)?.name ?? p.plugin.id
  const id = playingId(p, soundFontFile, ctx.gmMap)
  const font = (f: string) => f.replace(/\.sf2$/i, '')
  const sf = id && /^sf:(.+):\d+:\d+$/.exec(id)
  if (sf) return font(sf[1])
  const patch = id?.startsWith('saved:') ? ctx.patches.find((x) => `saved:${x.id}` === id) : undefined
  const src = patch?.source
  if (src?.kind === 'plugin') return plugins.find((x) => x.id === src.componentId)?.name ?? src.componentId
  if (src) return font(src.file)
  return soundFontFile ? font(soundFontFile) : 'the synth'
}

/** The file name a preset name saves as (as `presets::safe_name`). */
export const presetFileName = (n: string) => n.replace(/[/:\\]/g, '-').trim().replace(/^\.+/, '').trim() || 'Untitled'
