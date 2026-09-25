// The mock's sound library (#103): what `src/session/sound_library.rs` does, closely enough
// to build and screenshot the Sound Library drawer. No audio. MockSession calls `cmd` for
// its commands and `derive` after every change (the usage list, part voice names).

import fixture from './mock-fixture.json'
import { CATEGORY_LABELS, FAMILY_NAMES, emptyMap, type PatchCategory, type PatchFields, type PatchInfo, type ProgramMap, type RuleKind, type SoundLibraryCmd, type SoundLibraryState } from './sound-library'
import type { AppState } from './types'

const GM: string[] = fixture.gm
const SF2 = 'GeneralUser-GS.sf2'
/** The SoundFonts in the mock's folder (as `io.soundFonts`). */
const FONTS = [SF2, 'FluidR3_GM.sf2']

/** The category a GM program (or a preset: bank 128 = drums) most likely belongs to, as
 * `Category::guess` in src/patches/mod.rs. */
export function guessCategory(bank: number, program: number): PatchCategory {
  if (bank >= 128) return 'drumsPerc'
  const p = program & 127
  if (p === 4 || p === 5) return 'ePiano'
  if (p <= 7) return 'piano'
  if (p <= 15) return 'drumsPerc'
  if (p <= 23) return 'organ'
  if (p <= 31) return 'guitar'
  if (p <= 39) return 'bass'
  if (p <= 51) return 'strings'
  if (p <= 54) return 'choir'
  if (p === 55) return 'sfx'
  if (p <= 63) return 'brass'
  if (p <= 79) return 'saxWoodwind'
  if (p <= 87) return 'synthLead'
  if (p <= 103) return 'pad'
  if (p <= 107) return 'guitar'
  if (p === 108) return 'drumsPerc'
  if (p <= 111) return 'saxWoodwind'
  if (p <= 119) return 'drumsPerc'
  return 'sfx'
}

/** Which rule of which map a program resolves by, as `patches::resolve`. */
export function resolveProgram(global: ProgramMap, style: ProgramMap | null, drums: boolean, program: number): { patch: string | null; rule: RuleKind; fromStyle: boolean } {
  const rule = (m: ProgramMap): { patch: string; rule: RuleKind } | null => {
    if (drums) return m.drums ? { patch: m.drums, rule: 'drums' } : null
    const o = m.overrides.find((x) => x.program === program)
    if (o) return { patch: o.patch, rule: 'override' }
    const f = m.families[Math.floor((program & 127) / 8)]
    return f ? { patch: f, rule: 'family' } : null
  }
  const s = style ? rule(style) : null
  if (s) return { ...s, fromStyle: true }
  const g = rule(global)
  if (g) return { ...g, fromStyle: false }
  return { patch: null, rule: 'fallback', fromStyle: false }
}

function sf(id: string, name: string, bank: number, program: number, extra: Partial<PatchInfo> = {}): PatchInfo {
  return {
    id,
    name,
    category: guessCategory(bank, program),
    tags: [],
    favourite: false,
    source: { kind: 'soundFont', file: SF2, bank, program },
    defaults: { volume: null, pan: null, reverb: null, chorus: null, octave: 0 },
    available: true,
    note: null,
    ...extra,
  }
}

/** A small curated library, as a player might have built it. */
export function initialSoundLibrary(): SoundLibraryState {
  const patches: PatchInfo[] = [
    sf('stage-grand', 'Stage Grand', 0, 0, { favourite: true, tags: ['bright'] }),
    sf('warm-rhodes', 'Warm Rhodes', 0, 4, { favourite: true, defaults: { volume: 96, pan: null, reverb: 40, chorus: 30, octave: 0 } }),
    sf('finger-bass', 'Finger Bass', 0, 33, { defaults: { volume: 100, pan: 64, reverb: 10, chorus: null, octave: 0 } }),
    sf('studio-kit', 'Studio Kit', 128, 0),
    sf('silk-strings', 'Silk Strings', 0, 48, { tags: ['warm'] }),
    sf('brass-section', 'Brass Section', 0, 61),
    sf('soft-pad', 'Soft Pad', 0, 89),
    {
      ...sf('keys-au', 'Keys (AU)', 0, 4),
      category: 'ePiano',
      source: { kind: 'plugin', componentId: 'aumu dls  appl', state: '' },
    },
  ]
  const map = emptyMap()
  map.families[0] = 'stage-grand'
  map.families[4] = 'finger-bass'
  map.families[5] = 'silk-strings'
  map.families[6] = 'silk-strings'
  map.families[7] = 'brass-section'
  map.families[11] = 'soft-pad'
  map.overrides = [
    { program: 4, patch: 'warm-rhodes' },
    { program: 5, patch: 'warm-rhodes' },
  ]
  map.drums = 'studio-kit'
  return {
    patches,
    categories: (Object.keys(CATEGORY_LABELS) as PatchCategory[]).map((id) => ({ id, label: CATEGORY_LABELS[id] })),
    families: [...FAMILY_NAMES],
    map,
    styleMap: emptyMap(),
    styleKey: '',
    usage: [],
    portSendsMapped: false,
    auditioning: null,
    browse: null,
    file: '/Users/me/Documents/yahaha/sound-library.json',
    extraSoundFonts: [],
    lastAdded: null,
  }
}

function slug(name: string, taken: string[]): string {
  const base = name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '').slice(0, 32) || 'patch'
  if (!taken.includes(base)) return base
  for (let n = 2; ; n++) if (!taken.includes(`${base}-${n}`)) return `${base}-${n}`
}

function info(id: string, f: PatchFields): PatchInfo {
  const missing = f.source.kind === 'soundFont' && !FONTS.includes(f.source.file)
  return {
    id,
    ...f,
    // The desktop app builds with plugin hosting (#91): a plugin patch plays itself.
    available: !missing,
    note: missing ? `${(f.source as { file: string }).file} is not in the SoundFont folder` : null,
  }
}

export class MockSoundLibrary {
  /** Per-style maps, by style key. */
  private styleMaps = new Map<string, ProgramMap>()
  /** The keyboard parts' own patches. */
  parts: (string | null)[] = [null, null, null, null]
  private auditionLeft = 0

  constructor(private get: () => AppState) {}

  private get sl() {
    return this.get().soundLibrary
  }

  private map(style: boolean | undefined): ProgramMap {
    if (!style) return this.sl.map
    const key = this.sl.styleKey
    let m = this.styleMaps.get(key)
    if (!m) this.styleMaps.set(key, (m = emptyMap()))
    return m
  }

  private add(f: PatchFields): string {
    const id = slug(f.name, this.sl.patches.map((p) => p.id))
    this.sl.patches.push(info(id, f))
    this.sl.lastAdded = id
    return id
  }

  private forget(id: string) {
    for (const m of [this.sl.map, ...this.styleMaps.values()]) {
      m.families = m.families.map((f) => (f === id ? null : f))
      m.overrides = m.overrides.filter((o) => o.patch !== id)
      if (m.drums === id) m.drums = null
    }
    this.parts = this.parts.map((p) => (p === id ? null : p))
  }

  /** A GM voice was picked for a part (the voice list, an OTS): its own patch goes. */
  partVoice(part: number) {
    this.parts[part] = null
  }

  /** The plugin patch whose plugin each part was given (`partPlugins`). */
  private pluginParts: (string | null)[] = [null, null, null, null]

  /** A plugin picked (or, while a plugin patch plays, cleared) on the Plugins tab: the
   * part's own patch goes. */
  partPlugin(part: number, picked: boolean) {
    if (!picked && !this.pluginParts[part & 3]) return
    this.pluginParts[part & 3] = null
    this.parts[part & 3] = null
  }

  /** Whether part `part` plays a plugin the Plugins tab picked (not a plugin patch's). */
  ownPlugin(part: number): boolean {
    return !this.pluginParts[part & 3]
  }

  /** The parts' plugins to change for their own plugin patches, as the session's
   * `sync_part_plugins` does: [part, the plugin to load] or [part, null] to clear it. */
  partPlugins(): [number, { componentId: string; state: string } | null][] {
    const out: [number, { componentId: string; state: string } | null][] = []
    this.parts.forEach((id, i) => {
      const p = id ? this.sl.patches.find((q) => q.id === id) : undefined
      const want = p && p.source.kind === 'plugin' ? p : null
      if ((want?.id ?? null) === this.pluginParts[i]) return
      const had = this.pluginParts[i]
      this.pluginParts[i] = want?.id ?? null
      if (want && want.source.kind === 'plugin') out.push([i, { componentId: want.source.componentId, state: want.source.state }])
      else if (had) out.push([i, null])
    })
    return out
  }

  /** Runs a command; returns an error message if it is refused. */
  cmd(c: SoundLibraryCmd, running: boolean): string | null {
    const sl = this.sl
    const has = (id: string | null) => id === null || sl.patches.some((p) => p.id === id)
    const at = (id: string) => sl.patches.findIndex((p) => p.id === id)
    switch (c.type) {
      case 'createPatch':
        this.add(c.patch)
        break
      case 'updatePatch': {
        const i = at(c.id)
        if (i < 0) return `no patch ${c.id} in the sound library`
        sl.patches[i] = info(c.id, { ...c.patch, name: c.patch.name.trim() || sl.patches[i].name })
        break
      }
      case 'deletePatch': {
        const i = at(c.id)
        if (i < 0) return `no patch ${c.id} in the sound library`
        sl.patches.splice(i, 1)
        this.forget(c.id)
        break
      }
      case 'duplicatePatch': {
        const i = at(c.id)
        if (i < 0) return `no patch ${c.id} in the sound library`
        const p = sl.patches[i]
        const id = slug(`${p.name} copy`, sl.patches.map((q) => q.id))
        sl.patches.splice(i + 1, 0, { ...structuredClone(p), id, name: `${p.name} copy` })
        sl.lastAdded = id
        break
      }
      case 'movePatch': {
        const i = at(c.id)
        if (i < 0) return `no patch ${c.id} in the sound library`
        const [p] = sl.patches.splice(i, 1)
        sl.patches.splice(Math.min(c.to, sl.patches.length), 0, p)
        break
      }
      case 'setPatchFavourite': {
        const i = at(c.id)
        if (i < 0) return `no patch ${c.id} in the sound library`
        sl.patches[i].favourite = c.favourite
        break
      }
      case 'savePartAsPatch': {
        // What the part plays: its plugin, else its own patch, else the patch the map
        // sends its GM voice to, else its GM voice (as the session's).
        const kp = this.get().keyboardParts[c.part & 3]
        const own = kp.playsBass ? null : this.parts[c.part & 3]
        const style = this.styleMaps.get(sl.styleKey) ?? null
        const id = own ?? resolveProgram(sl.map, style, false, kp.program).patch
        const base = id ? sl.patches.find((p) => p.id === id) : null
        const plugin = kp.plugin && kp.plugin.status !== 'failed' ? kp.plugin : null
        const blank = { volume: null, pan: null, reverb: null, chorus: null, octave: 0 }
        let f: PatchFields = base
          ? { name: base.name, category: base.category, tags: [...base.tags], favourite: false, source: structuredClone(base.source), defaults: { ...base.defaults } }
          : { name: GM[kp.program], category: guessCategory(0, kp.program), tags: [], favourite: false, source: { kind: 'soundFont', file: SF2, bank: 0, program: kp.program }, defaults: blank }
        if (plugin) {
          const source = { kind: 'plugin' as const, componentId: plugin.id, state: '' }
          const same = base?.source.kind === 'plugin' && base.source.componentId === plugin.id
          f = same ? { ...f, source } : { name: plugin.name, category: base?.category ?? guessCategory(0, kp.program), tags: [], favourite: false, source, defaults: blank }
        }
        f.defaults.volume = kp.volume
        f.defaults.octave = kp.octave
        if (c.name?.trim()) f.name = c.name
        this.add(f)
        break
      }
      case 'addPresetAsPatch': {
        if (!FONTS.includes(c.file)) return `no SoundFont ${c.file} in the SoundFont folder`
        const name = c.name?.trim() || presetsOf(c.file).find((p) => p.bank === c.bank && p.program === c.program)?.name || `${c.file} ${c.bank}:${c.program + 1}`
        this.add({ name, category: guessCategory(c.bank, c.program), tags: [], favourite: false, source: { kind: 'soundFont', file: c.file, bank: c.bank, program: c.program }, defaults: { volume: null, pan: null, reverb: null, chorus: null, octave: 0 } })
        break
      }
      case 'auditionPatch':
      case 'auditionPreset': {
        if (running) return 'Stop the band to audition a sound'
        if (c.type === 'auditionPatch') {
          const p = sl.patches.find((q) => q.id === c.id)
          if (!p) return `no patch ${c.id} in the sound library`
          if (!p.available) return `${p.name}: ${p.note}`
        }
        sl.auditioning = c.type === 'auditionPatch' ? c.id : 'preset'
        this.auditionLeft = 3000
        break
      }
      case 'stopPatchAudition':
        sl.auditioning = null
        break
      case 'setFamilyRule':
        if (c.family < 0 || c.family > 15) return `no GM family ${c.family} (0-15)`
        if (!has(c.patch)) return `no patch ${c.patch} in the sound library`
        this.map(c.style).families[c.family] = c.patch
        break
      case 'setProgramOverride': {
        if (!has(c.patch)) return `no patch ${c.patch} in the sound library`
        const m = this.map(c.style)
        m.overrides = m.overrides.filter((o) => o.program !== c.program)
        if (c.patch) m.overrides = [...m.overrides, { program: c.program & 127, patch: c.patch }].sort((a, b) => a.program - b.program)
        break
      }
      case 'setDrumRule':
        if (!has(c.patch)) return `no patch ${c.patch} in the sound library`
        this.map(c.style).drums = c.patch
        break
      case 'clearStyleMap':
        this.styleMaps.delete(sl.styleKey)
        break
      case 'setPartPatch': {
        if (!has(c.id)) return `no patch ${c.id} in the sound library`
        const kp = this.get().keyboardParts[c.part & 3]
        this.parts[c.part & 3] = c.id
        const d = sl.patches.find((p) => p.id === c.id)?.defaults
        if (d) {
          if (d.volume !== null) kp.volume = d.volume
          kp.octave = d.octave
        }
        break
      }
      case 'setPortSendsMapped':
        sl.portSendsMapped = c.on
        break
      case 'browseSoundFont':
        if (c.file === null) sl.browse = null
        else if (!FONTS.includes(c.file)) return `no SoundFont ${c.file} in the SoundFont folder`
        else sl.browse = { file: c.file, presets: presetsOf(c.file), error: null }
        break
      case 'importSoundLibrary':
        return `${c.path}: the mock has no files to import`
      case 'exportSoundLibrary':
        return null
    }
    return null
  }

  /** Time passes (an audition ends by itself). */
  advance(ms: number) {
    if (this.sl.auditioning && (this.auditionLeft -= ms) <= 0) this.sl.auditioning = null
  }

  /** The fields that follow from the others: the style's own map, the usage list, the
   * keyboard parts' patch and voice names. */
  derive(st: AppState) {
    const sl = st.soundLibrary
    sl.styleKey = st.style.path.split('/').pop() ?? ''
    const style = this.styleMaps.get(sl.styleKey) ?? null
    sl.styleMap = style ? structuredClone(style) : emptyMap()
    const name = (id: string | null) => (id ? (sl.patches.find((p) => p.id === id)?.name ?? null) : null)
    sl.usage = st.mixer.styleParts.flatMap((p, i) => {
      if (!p.voice) return []
      const drums = i < 2 || p.voice.kit
      const r = resolveProgram(sl.map, style, drums, p.voice.program)
      return [{
        channel: p.channel,
        part: p.name,
        msb: p.voice.bankMsb,
        lsb: p.voice.bankLsb,
        program: p.voice.program,
        gmProgram: p.voice.program,
        voice: p.voice.label,
        drums,
        patch: r.patch,
        rule: r.rule,
        fromStyle: r.fromStyle,
        plays: name(r.patch) ?? p.voice.label,
      }]
    })
    st.keyboardParts.forEach((p, i) => {
      p.patch = this.parts[i]
      if (p.playsBass) return
      const own = name(this.parts[i])
      const mapped = name(resolveProgram(sl.map, style, false, p.program).patch)
      p.voiceName = own ?? mapped ?? GM[p.program]
    })
  }
}

/** The presets of a mock SoundFont: the GM set on bank 0 and a few kits on bank 128. */
export function presetsOf(file: string) {
  const kits = ['Standard', 'Room', 'Power', 'Electronic', 'Jazz', 'Brush']
  return [
    ...GM.map((name, program) => ({ bank: 0, program, name: file === SF2 ? name : `${name} (Fluid)` })),
    ...kits.map((name, i) => ({ bank: 128, program: [0, 8, 16, 24, 32, 40][i], name })),
  ]
}
