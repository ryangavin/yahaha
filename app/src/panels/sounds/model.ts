// The Sound Browser's list (#117): which catalog entries a sidebar choice and the filter
// show, in what order. Pure functions over `app.sounds`, so they are cheap to test.

import { CATEGORY_LABELS } from '../../lib/api/sound-library'
import type { PatchCategory, SoundCatalog, SoundEntry, SoundSource } from '../../lib/api/types'

/** A sidebar choice. */
export type SoundView = { kind: 'all' } | { kind: 'favourites' } | { kind: 'recents' } | { kind: 'saved' } | { kind: 'category'; id: PatchCategory }

export const SOURCE_BADGE: Record<SoundSource, string> = { soundFont: 'SF', plugin: 'AU', saved: 'Saved' }

/** The categories in the Genos order, each with how many entries it holds. */
export function categoryCounts(entries: SoundEntry[]): { id: PatchCategory; label: string; count: number }[] {
  const n = new Map<PatchCategory, number>()
  for (const e of entries) n.set(e.category, (n.get(e.category) ?? 0) + 1)
  return (Object.keys(CATEGORY_LABELS) as PatchCategory[]).map((id) => ({ id, label: CATEGORY_LABELS[id], count: n.get(id) ?? 0 }))
}

/**
 * The entries shown, as indices into `catalog.entries`: the sidebar's choice, then the
 * filter (every word must be in the name, the detail or the source badge). Recents come
 * in their order (most recent first); everything else in the catalog's order.
 */
export function visibleSounds(catalog: SoundCatalog, view: SoundView, query: string): number[] {
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
  const keep = (e: SoundEntry) => {
    switch (view.kind) {
      case 'favourites': return e.favourite
      case 'saved': return e.source === 'saved'
      case 'category': return e.category === view.id
      default: return true
    }
  }
  return entries.flatMap((e, i) => (keep(e) && match(e) ? [i] : []))
}

/** The entry a keyboard part plays now, if the catalog has it: its saved sound, its
 * plugin, or its voice on the default sound set. */
export function playingId(p: { program: number; patch: string | null; plugin?: { id: string } | null; playsBass: boolean }, soundFontFile: string | null): string | null {
  if (p.patch) return `saved:${p.patch}`
  if (p.plugin) return `au:${p.plugin.id}`
  return soundFontFile ? `sf:${soundFontFile}:0:${p.program}` : null
}
