// The Sound Library drawer's page, the patch being edited, and which map the rule
// controls edit. App-only state: it survives closing and reopening the drawer. `?tab=<id>`
// opens a page (with `?open=sound`, for screenshots).

import type { TipKey } from '../../help/tooltips'
import type { FontPreset, GmLayer, PatchInfo, SoundEntry, SoundLibraryState } from '../../lib/api/types'
import { CATEGORY_LABELS, type PatchCategory } from '../../lib/api/sound-library'

export type SoundTab = 'patches' | 'gm' | 'add'

export const TABS: { id: SoundTab; label: string; tip: TipKey }[] = [
  { id: 'patches', label: 'Patches', tip: 'sound.tab_patches' },
  { id: 'gm', label: 'GM map', tip: 'sound.tab_gm' },
  { id: 'add', label: 'Add from SoundFont', tip: 'sound.tab_add' },
]

function fromUrl(): SoundTab {
  try {
    const t = new URLSearchParams(location.search).get('tab')
    return TABS.find((x) => x.id === t)?.id ?? 'patches'
  } catch {
    return 'patches'
  }
}

class SoundNav {
  tab = $state<SoundTab>(fromUrl())
  /** The patch the editor shows. */
  selected = $state<string | null>(null)
  /** The rule controls edit this style's own map instead of the global one. */
  styleScope = $state(false)
}

export const nav = new SoundNav()

/** Patches grouped by category, in category order, keeping the user's order inside. */
export function byCategory(patches: PatchInfo[]): { category: PatchCategory; label: string; patches: PatchInfo[] }[] {
  const out: { category: PatchCategory; label: string; patches: PatchInfo[] }[] = []
  for (const c of Object.keys(CATEGORY_LABELS) as PatchCategory[]) {
    const ps = patches.filter((p) => p.category === c)
    if (ps.length) out.push({ category: c, label: CATEGORY_LABELS[c], patches: ps })
  }
  return out
}

/** The patches a search and the filters leave. */
export function filterPatches(patches: PatchInfo[], q: string, category: PatchCategory | 'all', favourites: boolean): PatchInfo[] {
  const needle = q.trim().toLowerCase()
  return patches.filter(
    (p) =>
      (category === 'all' || p.category === category) &&
      (!favourites || p.favourite) &&
      (!needle || [p.name, CATEGORY_LABELS[p.category], ...p.tags].some((s) => s.toLowerCase().includes(needle))),
  )
}

/** What a patch's source says, e.g. "GeneralUser-GS · 0:34" or "Audio Unit aumu dls  appl". */
export function sourceText(p: PatchInfo): string {
  if (p.source.kind === 'plugin') return `Audio Unit · ${p.source.componentId}`
  const font = p.source.file.replace(/\.sf2$/i, '')
  return p.source.bank >= 128 ? `${font} · drum kit ${p.source.program + 1}` : `${font} · ${p.source.bank}:${p.source.program + 1}`
}

/** A patch's name by id ('—' for none). */
export function patchName(sl: SoundLibraryState, id: string | null): string {
  if (!id) return '—'
  return sl.patches.find((p) => p.id === id)?.name ?? id
}

const LAYERS: Record<GmLayer, string> = { drums: 'Drums', override: 'Override', family: 'Family', auto: 'Auto', none: 'Unset' }

/** The GM map layer's badge text. */
export const layerLabel = (l: GmLayer): string => LAYERS[l]

/** A resolved Sound's name: a library patch's, the catalog's, else its font preset
 * ("GeneralUser-GS · 0:1"). '—' for none. */
export function soundName(sl: SoundLibraryState, entries: SoundEntry[], id: string | null, font: FontPreset | null): string {
  if (!id) return '—'
  if (id.startsWith('saved:')) return patchName(sl, id.slice(6))
  const e = entries.find((x) => x.id === id)
  if (e) return e.name
  if (!font) return id
  const f = font.file.replace(/\.sf2$/i, '')
  return font.bank >= 128 ? `${f} · kit ${font.program + 1}` : `${f} · ${font.bank}:${font.program + 1}`
}
