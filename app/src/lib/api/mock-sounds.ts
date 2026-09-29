// The mock's sound catalog (#117): what `src/session/sounds.rs` does, built from the mock's
// fonts, plugins and sound library. No audio. MockSession calls `cmd` for its commands
// (running what it hands back), `advance` for the audition, and `derive` after every change.
// The twin of app/src-tauri/src/mock_sounds.rs.

import { guessCategory, presetsOf } from './mock-sound-library'
import { MOCK_PRESETS_ID } from './mock-plugins'
import { fontSummaries, guessCategory as guessWords, MAX_RECENTS, parsePluginId, parsePresetId, pluginCategory, pluginPresetId, presetId, type PluginPresetList, type SoundCatalog, type SoundEntry, type SoundsCmd, type SoundsState } from './sounds'
import { originOfPresetKey, sameOrigin, type PatchCategory } from './sound-library'
import type { AppCmd, AppState } from './types'

export function initialSounds(): SoundsState {
  return { revision: 0, count: 0, scanning: false, auditioning: null, listingPresets: [] }
}

const PRESET_DIR = '/Users/mock/Library/Audio/Presets/Fake Instruments/Sampler Deluxe'

/** The mock sampler's presets: two .aupreset files from the start; the factory ones come
 * when the browser expands it (as app/src-tauri/src/mock_sounds.rs). */
function initialPresets(): PluginPresetList[] {
  const user = (name: string, folder: string | null) => ({ key: `u:${PRESET_DIR}/${folder ? `${folder}/` : ''}${name}.aupreset`, name, folder })
  return [{ plugin: MOCK_PRESETS_ID, listed: false, presets: [user('Arco Strings', null), user('Upright Piano', 'Pianos')] }]
}

export class MockSounds {
  private favourites = new Set<string>()
  private recents: string[] = []
  private categories = new Map<string, PatchCategory>()
  private audition: { id: string; left: number } | null = null
  private key = ''
  private revision = 0
  private presets = initialPresets()

  /** Preset `key` of plugin `id`, if listed. */
  preset(id: string, key: string) {
    return this.presets.find((l) => l.plugin === id)?.presets.find((p) => p.key === key) ?? null
  }

  private list(plugin: string): PluginPresetList {
    let l = this.presets.find((x) => x.plugin === plugin)
    if (!l) this.presets.push((l = { plugin, listed: false, presets: [] }))
    return l
  }

  catalog(st: AppState): SoundCatalog {
    const recent = (id: string) => this.recents.includes(id)
    const entries: SoundEntry[] = []
    for (const f of st.io.soundFonts) {
      for (const p of presetsOf(f)) {
        const id = presetId(f, p.bank, p.program)
        entries.push({ id, name: p.name, category: guessCategory(p.bank, p.program), source: 'soundFont', detail: f, favourite: this.favourites.has(id), recent: recent(id), plugin: null })
      }
    }
    for (const p of st.plugins.list) {
      const id = `au:${p.id}`
      const list = this.presets.find((l) => l.plugin === p.id)
      const category = this.categories.get(id) ?? pluginCategory(p.name, p.manufacturer)
      entries.push({
        id,
        name: p.name,
        category,
        source: 'plugin',
        detail: p.manufacturer,
        favourite: this.favourites.has(id),
        recent: recent(id),
        // Its .aupreset files alone are not its count: unknown until listed.
        plugin: { format: p.format, lastError: p.lastError, presets: list?.listed ? list.presets.length : null, ...(list?.error ? { presetsError: list.error } : {}) },
        parent: null,
      })
      for (const q of list?.presets ?? []) {
        const pid = pluginPresetId(p.id, q.key)
        entries.push({
          id: pid,
          name: q.name,
          category: this.categories.get(pid) ?? guessWords(`${q.name} ${q.folder ?? ''}`) ?? category,
          source: 'plugin',
          detail: q.folder ? `${p.name} · ${q.folder}` : p.name,
          favourite: this.favourites.has(pid),
          recent: recent(pid),
          plugin: { format: p.format, lastError: p.lastError, presets: null },
          parent: id,
        })
      }
    }
    for (const p of st.soundLibrary.patches) {
      const id = `saved:${p.id}`
      const detail = p.source.kind === 'soundFont' ? p.source.file : p.source.componentId
      entries.push({ id, name: p.name, category: p.category, source: 'saved', detail, favourite: p.favourite, recent: recent(id), plugin: null })
    }
    const fonts = fontSummaries(st.io.soundFonts.map((f) => [f, presetsOf(f)]))
    return { revision: this.revision, entries, recents: [...this.recents], fonts }
  }

  private known(st: AppState, id: string): boolean {
    const pre = parsePresetId(id)
    if (pre) return st.io.soundFonts.includes(pre.file) && presetsOf(pre.file).some((p) => p.bank === pre.bank && p.program === pre.program)
    const au = parsePluginId(id)
    if (au) return au.key === null ? st.plugins.list.some((p) => p.id === au.plugin) : this.preset(au.plugin, au.key) !== null
    if (id.startsWith('saved:')) return st.soundLibrary.patches.some((p) => `saved:${p.id}` === id)
    return false
  }

  /** Run a command: an error's text, or the commands the rest of the mock runs for it (an
   * `assignLastAdded` part: then give that part the patch just added). */
  cmd(st: AppState, c: SoundsCmd): { error?: string; run?: AppCmd[]; assignLastAdded?: number; saved?: string } {
    if (c.type === 'stopSoundAudition') {
      this.audition = null
      return {}
    }
    // The session mock adds the patch itself (`patchFor`), as a map rule's.
    if (c.type === 'addToMySounds') return {}
    if (c.type === 'listPluginPresets') {
      const au = parsePluginId(c.id)
      if (!au || au.key !== null) return { error: `${c.id} is not a plugin` }
      const entry = st.plugins.list.find((p) => p.id === au.plugin)
      if (!entry) return { error: `no instrument Audio Unit ${au.plugin} is installed` }
      const l = this.list(au.plugin)
      // A plugin that does not load cannot list its presets (the engine's listing fails
      // the same way): the browser stops waiting.
      if (!l.listed && !l.error && entry.lastError) l.error = entry.lastError
      else if (!l.listed && !l.error) {
        l.listed = true
        if (au.plugin === MOCK_PRESETS_ID) l.presets.unshift(...['Init', 'Bright Grand', 'Brass Stabs'].map((name, n) => ({ key: `f:${n}`, name, folder: null })))
      }
      return {}
    }
    if (c.type === 'savePartAsPluginPreset') {
      if (c.part < 0 || c.part > 3) return { error: `no keyboard part ${c.part} (0-3)` }
      const name = c.name.trim()
      if (!name) return { error: 'name the preset' }
      const pl = st.keyboardParts[c.part].plugin
      if (!pl || pl.status !== 'playing') return { error: 'the part plays its SoundFont voice, not a plugin' }
      const file = name.replace(/[/:]/g, '-')
      const key = `u:/Users/mock/Library/Audio/Presets/${pl.manufacturer}/${pl.name}/${file}.aupreset`
      const l = this.list(pl.id)
      if (!c.overwrite && l.presets.some((p) => p.key === key)) return { error: `a preset called ${file} already exists: save under another name, or replace it` }
      l.presets = [...l.presets.filter((p) => p.key !== key), { key, name: file, folder: null }]
      this.categories.set(pluginPresetId(pl.id, key), c.category)
      pl.preset = file
      pl.presetKey = key
      return { saved: file }
    }
    if (c.type === 'assignSound' && (c.part < 0 || c.part > 3)) return { error: `no keyboard part ${c.part} (0-3)` }
    if (!this.known(st, c.id)) return { error: `no sound ${c.id}` }
    const saved = c.id.startsWith('saved:') ? c.id.slice(6) : null
    switch (c.type) {
      case 'setSoundFavourite':
        if (saved) return { run: [{ type: 'setPatchFavourite', id: saved, favourite: c.on }] }
        if (c.on) this.favourites.add(c.id)
        else this.favourites.delete(c.id)
        return {}
      case 'auditionSound':
        if (st.transport.running) return { error: 'Stop the band to audition a sound' }
        this.audition = { id: c.id, left: 3000 }
        return {}
      case 'setSoundCategory': {
        if (saved) {
          const p = st.soundLibrary.patches.find((q) => q.id === saved)!
          return { run: [{ type: 'updatePatch', id: saved, patch: { name: p.name, category: c.category, tags: p.tags, favourite: p.favourite, source: p.source, defaults: p.defaults } }] }
        }
        if (!c.id.startsWith('au:')) return { error: "a preset's category is its GM family" }
        this.categories.set(c.id, c.category)
        return {}
      }
      case 'assignSound': {
        this.recents = [c.id, ...this.recents.filter((r) => r !== c.id)].slice(0, MAX_RECENTS)
        if (saved) return { run: [{ type: 'setPartPatch', part: c.part, id: saved }] }
        const au = parsePluginId(c.id)
        if (au?.key) return { run: [{ type: 'setPartPluginPreset', part: c.part, id: au.plugin, preset: au.key }] }
        if (au) return { run: [{ type: 'setPartPlugin', part: c.part, id: au.plugin, state: null }] }
        const pre = parsePresetId(c.id)!
        if (st.io.soundFontFile === pre.file && pre.bank === 0) return { run: [{ type: 'setPartVoice', part: c.part, program: pre.program }] }
        const have = st.soundLibrary.patches.find((p) => p.source.kind === 'soundFont' && p.source.file === pre.file && p.source.bank === pre.bank && p.source.program === pre.program)
        if (have) return { run: [{ type: 'setPartPatch', part: c.part, id: have.id }] }
        return { run: [{ type: 'addPresetAsPatch', file: pre.file, bank: pre.bank, program: pre.program, name: null }], assignLastAdded: c.part }
      }
    }
  }

  /** The library patch a program map rule gets for catalog entry `id` (#117): a saved
   * sound's own, else the library's patch for the preset or plugin, added once through
   * `run`. An id without a catalog prefix is a patch id already. */
  patchFor(st: AppState, id: string, run: (c: AppCmd) => void): { patch: string } | { error: string } {
    if (id.startsWith('saved:')) return { patch: id.slice(6) }
    const pre = parsePresetId(id)
    if (!pre && !id.startsWith('au:')) return { patch: id }
    if (!this.known(st, id)) return { error: `no sound ${id}` }
    const au = pre ? null : parsePluginId(id)
    const plugin = au?.plugin ?? null
    // The one plugin sound for the preset (its origin finds it again): a user preset's
    // keeps the file's settings (the mock has none to read); a factory preset's state is
    // captured when it plays, so it starts empty.
    const origin = au?.key ? originOfPresetKey(au.key) : undefined
    const state = origin?.kind === 'file' ? 'bW9jaw==' : ''
    const have = st.soundLibrary.patches.find((p) =>
      pre
        ? p.source.kind === 'soundFont' && p.source.file === pre.file && p.source.bank === pre.bank && p.source.program === pre.program
        : p.source.kind === 'plugin' && p.source.componentId === plugin && (origin ? sameOrigin(p.source.origin, origin) : p.source.state === state && !p.source.origin),
    )
    if (have) return { patch: have.id }
    if (pre) run({ type: 'addPresetAsPatch', file: pre.file, bank: pre.bank, program: pre.program, name: null })
    else {
      const e = st.plugins.list.find((p) => p.id === plugin)!
      let category = this.categories.get(`au:${e.id}`) ?? pluginCategory(e.name, e.manufacturer)
      let name = e.name
      const q = au?.key ? this.preset(e.id, au.key) : null
      if (q) {
        category = this.categories.get(id) ?? guessWords(`${q.name} ${q.folder ?? ''}`) ?? category
        name = `${e.name} · ${q.name}`
      }
      run({ type: 'createPatch', patch: { name, category, tags: [], favourite: false, source: { kind: 'plugin', componentId: e.id, state, ...(origin ? { origin } : {}) }, defaults: { volume: null, pan: null, reverb: null, chorus: null, octave: 0 } } })
    }
    return { patch: st.soundLibrary.lastAdded ?? '' }
  }

  advance(ms: number, running: boolean) {
    if (!this.audition) return
    this.audition.left -= ms
    if (this.audition.left <= 0 || running) this.audition = null
  }

  /** `state.sounds`: a new revision whenever what the catalog is built from changed. */
  derive(st: AppState) {
    const key = JSON.stringify([st.io.soundFonts, st.io.soundFontFile, st.plugins.list, st.soundLibrary.patches, [...this.favourites], this.recents, [...this.categories], this.presets])
    if (key !== this.key || this.revision === 0) {
      this.key = key
      this.revision++
    }
    const presets = st.io.soundFonts.reduce((n, f) => n + presetsOf(f).length, 0)
    const pluginPresets = this.presets.filter((l) => st.plugins.list.some((p) => p.id === l.plugin)).reduce((n, l) => n + l.presets.length, 0)
    st.sounds = {
      revision: this.revision,
      count: presets + st.plugins.list.length + pluginPresets + st.soundLibrary.patches.length,
      scanning: st.plugins.scanning,
      auditioning: this.audition?.id ?? null,
      listingPresets: [],
    }
  }
}
