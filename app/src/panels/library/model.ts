// Library › Sounds (docs/racks.md, "Screens"): which catalog entries the chips, the
// category column and the search show, and each row's badge and instrument. Pure functions
// over `app.sounds` and the sound library, so they are cheap to test. The catalog and its
// commands are the Sound Browser's (#117, panels/sounds/model.ts).
//
// What "All" holds: the GM map's resolved sounds (one per program and the kit, not every
// preset of every font), every sound in My Sounds, and every plugin with the factory
// presets listed so far. Every other preset of a font, and a plugin's presets not listed
// yet, are reached through Instruments › Browse (the instrument filter), which lists all of
// that instrument's presets.

import { CATEGORY_LABELS } from '../../lib/api/sound-library'
import type { KeyboardPart, PatchCategory, SoundCatalog, SoundEntry } from '../../lib/api/types'
import { allSoundIds, instrumentOf, instruments, patchesById, playingId, type SoundContext } from '../sounds/model'

/** A row's badge: what kind of sound it is. */
export type Badge = 'mine' | 'factory' | 'soundFont'

/** The source chips: All, or one badge. */
export type SourceChip = 'all' | Badge

export const BADGE_LABEL: Record<Badge, string> = { mine: 'Mine', factory: 'Factory', soundFont: 'SoundFont' }

/** Mine: in My Sounds. Factory: a plugin or one of its presets. SoundFont: a font preset. */
export function badgeOf(e: Pick<SoundEntry, 'source'>): Badge {
  return e.source === 'saved' ? 'mine' : e.source === 'plugin' ? 'factory' : 'soundFont'
}

/** What the chips, the search and the category column ask for. */
export interface SoundFilter {
  source: SourceChip
  favourites: boolean
  /** An instrument (`sf:<file>` or `au:<component id>`): all its presets, whatever All holds. */
  instrument: string | null
  category: PatchCategory | null
  query: string
}

export const NO_FILTER: SoundFilter = { source: 'all', favourites: false, instrument: null, category: null, query: '' }

/** Each instrument's name by id (`sf:<file>`, `au:<id>`). */
export function instrumentNames(catalog: SoundCatalog): Map<string, string> {
  return new Map(instruments(catalog).map((i) => [i.id, i.name]))
}

/**
 * The rows shown, as indices into `catalog.entries`. With `ignoreCategory`, before the
 * category column (what its counts count). Sorted by category (the Genos order), then
 * name; an instrument's presets keep the catalog's order (bank and program, or the
 * plugin's own).
 */
export function librarySounds(catalog: SoundCatalog, f: SoundFilter, ctx: SoundContext, ignoreCategory = false): number[] {
  const { entries } = catalog
  const all = allSoundIds(ctx)
  const byId = patchesById(ctx.patches)
  const names = instrumentNames(catalog)
  const words = f.query.toLowerCase().split(/\s+/).filter(Boolean)
  const member = (e: SoundEntry) => (f.instrument ? instrumentOf(e, byId) === f.instrument : all.has(e.id) || e.source === 'plugin')
  const keep = (e: SoundEntry) => {
    if (!member(e)) return false
    if (f.source !== 'all' && badgeOf(e) !== f.source) return false
    if (f.favourites && !e.favourite) return false
    if (!ignoreCategory && f.category && e.category !== f.category) return false
    if (!words.length) return true
    const inst = names.get(instrumentOf(e, byId) ?? '') ?? ''
    const hay = `${e.name} ${e.detail} ${inst} ${BADGE_LABEL[badgeOf(e)]} ${CATEGORY_LABELS[e.category] ?? ''}`.toLowerCase()
    return words.every((w) => hay.includes(w))
  }
  const rows = entries.flatMap((e, i) => (keep(e) ? [i] : []))
  if (f.instrument) return rows
  const order = Object.keys(CATEGORY_LABELS) as PatchCategory[]
  const rank = new Map(order.map((c, i) => [c, i]))
  return rows.sort((a, b) => (rank.get(entries[a].category) ?? 99) - (rank.get(entries[b].category) ?? 99) || entries[a].name.localeCompare(entries[b].name) || a - b)
}

/** The category column: every category in the Genos order with how many rows it would show. */
export function libraryCategories(catalog: SoundCatalog, f: SoundFilter, ctx: SoundContext): { id: PatchCategory; label: string; count: number }[] {
  const n = new Map<PatchCategory, number>()
  for (const i of librarySounds(catalog, f, ctx, true)) n.set(catalog.entries[i].category, (n.get(catalog.entries[i].category) ?? 0) + 1)
  return (Object.keys(CATEGORY_LABELS) as PatchCategory[]).map((id) => ({ id, label: CATEGORY_LABELS[id], count: n.get(id) ?? 0 }))
}

/** Short part names, as the rack writes them. */
export const PART_SHORT = ['R1', 'R2', 'R3', 'L'] as const

/** The sound each keyboard part plays now (its catalog id), by part. */
export function playingByPart(parts: KeyboardPart[], ctx: SoundContext): (string | null)[] {
  return parts.map((p) => playingId(p, ctx))
}
