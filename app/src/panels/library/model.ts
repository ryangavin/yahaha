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

import { quickLabel } from '../../lib/api/quick-racks'
import { CATEGORY_LABELS } from '../../lib/api/sound-library'
import type { KeyboardPart, PatchCategory, QuickRacksState, RackEntry, SoundCatalog, SoundEntry } from '../../lib/api/types'
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

// The catalog and the library's slices are immutable snapshots (a new object whenever they
// change: `app.sounds` is replaced per revision and `app.state` shares unchanged slices),
// so what is built from them is memoised on their identity, not rebuilt per call.

const namesMemo = new WeakMap<SoundCatalog, Map<string, string>>()

/** Each instrument's name by id (`sf:<file>`, `au:<id>`); built once per catalog. */
export function instrumentNames(catalog: SoundCatalog): Map<string, string> {
  let names = namesMemo.get(catalog)
  if (!names) namesMemo.set(catalog, (names = new Map(instruments(catalog).map((i) => [i.id, i.name]))))
  return names
}

/** Per catalog: every entry index sorted by category (the Genos order), then name. */
const sortedMemo = new WeakMap<SoundCatalog, number[]>()
function sortedIndices(catalog: SoundCatalog): number[] {
  let sorted = sortedMemo.get(catalog)
  if (!sorted) {
    const { entries } = catalog
    const rank = new Map((Object.keys(CATEGORY_LABELS) as PatchCategory[]).map((c, i) => [c, i]))
    sorted = entries.map((_, i) => i)
    sorted.sort((a, b) => (rank.get(entries[a].category) ?? 99) - (rank.get(entries[b].category) ?? 99) || entries[a].name.localeCompare(entries[b].name) || a - b)
    sortedMemo.set(catalog, sorted)
  }
  return sorted
}

/** Per catalog and library patches: each entry's instrument and lowercase search text. */
interface EntryIndex {
  patches: SoundContext['patches']
  inst: (string | null)[]
  hay: string[]
}
const indexMemo = new WeakMap<SoundCatalog, EntryIndex>()
function entryIndex(catalog: SoundCatalog, patches: SoundContext['patches']): EntryIndex {
  const hit = indexMemo.get(catalog)
  if (hit && hit.patches === patches) return hit
  const byId = patchesById(patches)
  const names = instrumentNames(catalog)
  const inst = catalog.entries.map((e) => instrumentOf(e, byId))
  const hay = catalog.entries.map((e, i) => `${e.name} ${e.detail} ${names.get(inst[i] ?? '') ?? ''} ${BADGE_LABEL[badgeOf(e)]} ${CATEGORY_LABELS[e.category] ?? ''}`.toLowerCase())
  const index = { patches, inst, hay }
  indexMemo.set(catalog, index)
  return index
}

/** All's members for the library's patches and GM map (the last pair asked for). */
let allMemo: { patches: SoundContext['patches']; gmMap: SoundContext['gmMap']; ids: Set<string> } | null = null
function allIds(ctx: SoundContext): Set<string> {
  if (allMemo?.patches !== ctx.patches || allMemo.gmMap !== ctx.gmMap) allMemo = { patches: ctx.patches, gmMap: ctx.gmMap, ids: allSoundIds(ctx) }
  return allMemo.ids
}

/** The last rows asked for, before and after the category: the same inputs (by identity,
 * and the filter by field) give the same array. */
interface RowsMemo {
  catalog: SoundCatalog
  patches: SoundContext['patches']
  gmMap: SoundContext['gmMap']
  source: SourceChip
  favourites: boolean
  instrument: string | null
  query: string
  rows: number[]
  category: PatchCategory | null
  inCategory: number[]
}
let rowsMemo: RowsMemo | null = null

/** The rows before the category column: what its counts count. */
function uncategorised(catalog: SoundCatalog, f: SoundFilter, ctx: SoundContext): RowsMemo {
  const m = rowsMemo
  if (m && m.catalog === catalog && m.patches === ctx.patches && m.gmMap === ctx.gmMap && m.source === f.source && m.favourites === f.favourites && m.instrument === f.instrument && m.query === f.query) return m
  const { entries } = catalog
  const all = allIds(ctx)
  const { inst, hay } = entryIndex(catalog, ctx.patches)
  const words = f.query.toLowerCase().split(/\s+/).filter(Boolean)
  const keep = (i: number) => {
    const e = entries[i]
    if (f.instrument ? inst[i] !== f.instrument : !(all.has(e.id) || e.source === 'plugin')) return false
    if (f.source !== 'all' && badgeOf(e) !== f.source) return false
    if (f.favourites && !e.favourite) return false
    return words.every((w) => hay[i].includes(w))
  }
  // An instrument's presets keep the catalog's order; the rest the sorted order.
  const rows = f.instrument ? entries.flatMap((_, i) => (keep(i) ? [i] : [])) : sortedIndices(catalog).filter(keep)
  rowsMemo = { catalog, patches: ctx.patches, gmMap: ctx.gmMap, source: f.source, favourites: f.favourites, instrument: f.instrument, query: f.query, rows, category: null, inCategory: rows }
  return rowsMemo
}

/**
 * The rows shown, as indices into `catalog.entries`. With `ignoreCategory`, before the
 * category column (what its counts count). Sorted by category (the Genos order), then
 * name; an instrument's presets keep the catalog's order (bank and program, or the
 * plugin's own). The same inputs return the same array.
 */
export function librarySounds(catalog: SoundCatalog, f: SoundFilter, ctx: SoundContext, ignoreCategory = false): number[] {
  const m = uncategorised(catalog, f, ctx)
  if (ignoreCategory || !f.category) return m.rows
  if (m.category !== f.category) {
    m.category = f.category
    m.inCategory = m.rows.filter((i) => catalog.entries[i].category === f.category)
  }
  return m.inCategory
}

/** The category column: every category in the Genos order with how many rows it would
 * show, counted over the rows `librarySounds` gives before the category. */
export function libraryCategories(catalog: SoundCatalog, f: SoundFilter, ctx: SoundContext): { id: PatchCategory; label: string; count: number }[] {
  const n = new Map<PatchCategory, number>()
  for (const i of librarySounds(catalog, f, ctx, true)) n.set(catalog.entries[i].category, (n.get(catalog.entries[i].category) ?? 0) + 1)
  return (Object.keys(CATEGORY_LABELS) as PatchCategory[]).map((id) => ({ id, label: CATEGORY_LABELS[id], count: n.get(id) ?? 0 }))
}

/** Short part names, as the rack writes them. */
export const PART_SHORT = ['R1', 'R2', 'R3', 'L'] as const

/** Racks: the racks whose name or part sounds hold every word of `query`. */
export function searchRacks(racks: RackEntry[], query: string): RackEntry[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean)
  if (!words.length) return racks
  return racks.filter((r) => {
    const hay = `${r.name} ${r.parts.filter((_, i) => r.on[i]).join(' ')}`.toLowerCase()
    return words.every((w) => hay.includes(w))
  })
}

/** The Quick Rack buttons of the bank on view holding rack `id` (["A1", "A3"]); Quick
 * Racks state only carries the bank on view. */
export function quickButtonsOf(q: QuickRacksState, id: string): string[] {
  return q.buttons.flatMap((b, i) => (b.rack === id ? [quickLabel(q.bank, i)] : []))
}

/** The sound each keyboard part plays now (its catalog id), by part. */
export function playingByPart(parts: KeyboardPart[], ctx: SoundContext): (string | null)[] {
  return parts.map((p) => playingId(p, ctx))
}
