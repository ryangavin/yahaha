// The mock's sound catalog (#117): what `src/session/sounds.rs` does, built from the mock's
// fonts, plugins and sound library. No audio. MockSession calls `cmd` for its commands
// (running what it hands back), `advance` for the audition, and `derive` after every change.
// The twin of app/src-tauri/src/mock_sounds.rs.

import { guessCategory, presetsOf } from './mock-sound-library'
import { MAX_RECENTS, parsePresetId, pluginCategory, presetId, type SoundCatalog, type SoundEntry, type SoundsCmd, type SoundsState } from './sounds'
import type { PatchCategory } from './sound-library'
import type { AppCmd, AppState } from './types'

export function initialSounds(): SoundsState {
  return { revision: 0, count: 0, scanning: false, auditioning: null }
}

export class MockSounds {
  private favourites = new Set<string>()
  private recents: string[] = []
  private categories = new Map<string, PatchCategory>()
  private audition: { id: string; left: number } | null = null
  private key = ''
  private revision = 0

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
      entries.push({
        id,
        name: p.name,
        category: this.categories.get(id) ?? pluginCategory(p.name, p.manufacturer),
        source: 'plugin',
        detail: p.manufacturer,
        favourite: this.favourites.has(id),
        recent: recent(id),
        plugin: { format: p.format, lastError: p.lastError },
      })
    }
    for (const p of st.soundLibrary.patches) {
      const id = `saved:${p.id}`
      const detail = p.source.kind === 'soundFont' ? p.source.file : p.source.componentId
      entries.push({ id, name: p.name, category: p.category, source: 'saved', detail, favourite: p.favourite, recent: recent(id), plugin: null })
    }
    return { revision: this.revision, entries, recents: [...this.recents] }
  }

  private known(st: AppState, id: string): boolean {
    const pre = parsePresetId(id)
    if (pre) return st.io.soundFonts.includes(pre.file) && presetsOf(pre.file).some((p) => p.bank === pre.bank && p.program === pre.program)
    if (id.startsWith('au:')) return st.plugins.list.some((p) => `au:${p.id}` === id)
    if (id.startsWith('saved:')) return st.soundLibrary.patches.some((p) => `saved:${p.id}` === id)
    return false
  }

  /** Run a command: an error's text, or the commands the rest of the mock runs for it (an
   * `assignLastAdded` part: then give that part the patch just added). */
  cmd(st: AppState, c: SoundsCmd): { error?: string; run?: AppCmd[]; assignLastAdded?: number } {
    if (c.type === 'stopSoundAudition') {
      this.audition = null
      return {}
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
        if (c.id.startsWith('au:')) return { run: [{ type: 'setPartPlugin', part: c.part, id: c.id.slice(3), state: null }] }
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
    const plugin = pre ? null : id.slice(3)
    const have = st.soundLibrary.patches.find((p) =>
      pre ? p.source.kind === 'soundFont' && p.source.file === pre.file && p.source.bank === pre.bank && p.source.program === pre.program : p.source.kind === 'plugin' && p.source.componentId === plugin && !p.source.state,
    )
    if (have) return { patch: have.id }
    if (pre) run({ type: 'addPresetAsPatch', file: pre.file, bank: pre.bank, program: pre.program, name: null })
    else {
      const e = st.plugins.list.find((p) => p.id === plugin)!
      const category = this.categories.get(id) ?? pluginCategory(e.name, e.manufacturer)
      run({ type: 'createPatch', patch: { name: e.name, category, tags: [], favourite: false, source: { kind: 'plugin', componentId: e.id, state: '' }, defaults: { volume: null, pan: null, reverb: null, chorus: null, octave: 0 } } })
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
    const key = JSON.stringify([st.io.soundFonts, st.io.soundFontFile, st.plugins.list, st.soundLibrary.patches, [...this.favourites], this.recents, [...this.categories]])
    if (key !== this.key || this.revision === 0) {
      this.key = key
      this.revision++
    }
    const presets = st.io.soundFonts.reduce((n, f) => n + presetsOf(f).length, 0)
    st.sounds = {
      revision: this.revision,
      count: presets + st.plugins.list.length + st.soundLibrary.patches.length,
      scanning: st.plugins.scanning,
      auditioning: this.audition?.id ?? null,
    }
  }
}
