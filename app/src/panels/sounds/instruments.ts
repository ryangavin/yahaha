// The Sound Browser's Instruments tab (docs/sound-browser.md, O2): the SoundFonts and
// plugins found in the scanned folders, each with its own presets. Pure functions over the
// catalog, the plugin list and the library, so they are cheap to test.

import { originOfPresetKey, sameOrigin } from '../../lib/api/sound-library'
import { parsePluginId, parsePresetId, type FontSummary, type SoundCatalog, type SoundEntry } from '../../lib/api/sounds'
import type { KeyboardPart, Patch, PluginEntry, PluginStatus } from '../../lib/api/types'

export type Instrument =
  | { kind: 'font'; id: string; name: string; font: FontSummary }
  | { kind: 'plugin'; id: string; name: string; plugin: PluginEntry; entry: SoundEntry | undefined }

/** The fonts (`font:<file>`, in the folder's order), then the plugins (`au:<component>`, in
 * the catalog's order: by maker, then name). An engine without `catalog.fonts` gets them
 * from the entries. */
export function instruments(catalog: SoundCatalog, plugins: PluginEntry[]): Instrument[] {
  const fonts = catalog.fonts ?? fontsFromEntries(catalog.entries)
  const out: Instrument[] = fonts.map((font) => ({ kind: 'font', id: `font:${font.file}`, name: font.file.replace(/\.sf2$/i, ''), font }))
  const byId = new Map(plugins.map((p) => [`au:${p.id}`, p]))
  const seen = new Set<string>()
  for (const e of catalog.entries) {
    const p = e.source === 'plugin' && !e.parent ? byId.get(e.id) : undefined
    if (!p) continue
    seen.add(e.id)
    out.push({ kind: 'plugin', id: e.id, name: p.name, plugin: p, entry: e })
  }
  // A plugin the catalog has not caught up with yet.
  for (const [id, p] of byId) if (!seen.has(id)) out.push({ kind: 'plugin', id, name: p.name, plugin: p, entry: undefined })
  return out
}

function fontsFromEntries(entries: SoundEntry[]): FontSummary[] {
  const by = new Map<string, { bank: number; program: number }[]>()
  for (const e of entries) {
    const p = e.source === 'soundFont' ? parsePresetId(e.id) : null
    if (p) by.set(p.file, [...(by.get(p.file) ?? []), p])
  }
  return [...by].map(([file, ps]) => {
    const kits = ps.filter((p) => p.bank >= 128).length
    return { file, presets: ps.length - kits, kits, gmPrograms: new Set(ps.filter((p) => p.bank === 0).map((p) => p.program)).size, gmKit: kits > 0 }
  })
}

/** An instrument's presets, as indices into `catalog.entries`: a font's presets (bank then
 * program), or a plugin's factory presets and `.aupreset` files. */
export function instrumentPresets(catalog: SoundCatalog, inst: Instrument): number[] {
  const file = inst.kind === 'font' ? inst.font.file : null
  return catalog.entries.flatMap((e, i) => {
    if (file !== null) return e.source === 'soundFont' && parsePresetId(e.id)?.file === file ? [i] : []
    return e.parent === inst.id ? [i] : []
  })
}

/** "260 presets · 12 kits · GM 128/128 + kit": a font row's only details. */
export function fontLine(f: FontSummary): string {
  const n = (k: number, one: string) => `${k.toLocaleString()} ${one}${k === 1 ? '' : 's'}`
  return `${n(f.presets, 'preset')} · ${n(f.kits, 'kit')} · GM ${f.gmPrograms}/128${f.gmKit ? ' + kit' : ''}`
}

/** A font preset's place: "0:34", or "Kit 1" on bank 128. */
export function presetPlace(id: string): string {
  const p = parsePresetId(id)
  if (!p) return ''
  return p.bank >= 128 ? `Kit ${p.program + 1}` : `${p.bank}:${p.program + 1}`
}

/** A catalog entry is in My Sounds already: the library has its patch (the one
 * `addToMySounds` would add). A plugin's own row is never "in": each New sound is new. */
export function inMySounds(patches: Pick<Patch, 'source'>[], id: string): boolean {
  const pre = parsePresetId(id)
  if (pre) return patches.some((p) => p.source.kind === 'soundFont' && p.source.file === pre.file && p.source.bank === pre.bank && p.source.program === pre.program)
  const au = parsePluginId(id)
  const origin = au?.key ? originOfPresetKey(au.key) : undefined
  if (!au || !origin) return false
  return patches.some((p) => p.source.kind === 'plugin' && p.source.componentId === au.plugin && sameOrigin(p.source.origin, origin))
}

/** Where plugin `id` (a component id) is loaded: each keyboard part playing it, with its
 * load status and CPU. */
export function pluginLoads(parts: KeyboardPart[], id: string): { part: number; name: string; status: PluginStatus; cpu: number; editor: boolean; error: string | null }[] {
  return parts.flatMap((k, part) => (k.plugin?.id === id ? [{ part, name: k.name, status: k.plugin.status, cpu: k.plugin.cpu, editor: k.plugin.editor, error: k.plugin.error }] : []))
}

/** The scan's word on a plugin: it failed to load last time, or how many presets it has. */
export function scanLine(p: PluginEntry, e: SoundEntry | undefined, listing: boolean): string {
  if (p.lastError) return `⚠ ${p.lastError}`
  if (listing) return 'listing presets…'
  if (e?.plugin?.presetsError) return `⚠ presets not listed: ${e.plugin.presetsError}`
  const n = e?.plugin?.presets
  return n == null ? 'presets not listed yet' : `${n} preset${n === 1 ? '' : 's'}`
}
