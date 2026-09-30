// @vitest-environment node
// The sound catalog model Library and the sound picker share (panels/sounds/model.ts), and
// what a part plays, named (moved from the old Sound Browser's tests, racks item 13).

import { describe, expect, it } from 'vitest'
import { MockSession } from '../../lib/api/mock'
import fixture from '../../lib/api/mock-fixture.json'
import { partSound, type PartSoundOf } from '../../lib/api/part-sound'
import partSoundCases from '../../../../tests/fixtures/part_sound_cases.json'
import type { FontPreset, GmMapRow, Patch, PluginOrigin } from '../../lib/api/sound-library'
import type { PluginStatus } from '../../lib/api/types'
import { categoryCounts, instrumentName, instruments, nowPlaying, playingId, visibleSounds } from './model'

describe('sound catalog model', () => {
  const e = (id: string, category: 'piano' | 'bass', favourite = false, detail = 'A.sf2') => ({ id, name: id, category, source: id.startsWith('saved:') ? ('saved' as const) : ('soundFont' as const), detail, favourite, recent: false, plugin: null })
  const catalog = { revision: 1, entries: [e('sf:A.sf2:0:0', 'piano', true), e('sf:A.sf2:0:1', 'bass'), e('sf:B.sf2:0:0', 'piano', false, 'B.sf2'), e('saved:x', 'piano')], recents: ['sf:B.sf2:0:0', 'sf:A.sf2:0:0'] }
  const patch = { id: 'x', name: 'X', category: 'piano' as const, tags: [], favourite: false, source: { kind: 'soundFont' as const, file: 'B.sf2', bank: 0, program: 3 }, available: true, note: null, number: 1 }
  const gmMap = [{ program: 0, family: 0, overrideRule: null, familyRule: null, resolved: { sound: 'sf:A.sf2:0:0', layer: 'auto' as const, fromStyle: false, font: null } }]
  const ctx = { patches: [patch], gmMap }

  it('All sounds, chips, categories and Recents', () => {
    expect(visibleSounds(catalog, { kind: 'all' }, '', ctx)).toEqual([0, 3])
    expect(visibleSounds(catalog, { kind: 'category', id: 'piano' }, '', ctx)).toEqual([0, 3])
    expect(visibleSounds(catalog, { kind: 'instrument', id: 'sf:B.sf2' }, '', ctx)).toEqual([2, 3])
    expect(visibleSounds(catalog, { kind: 'mine' }, '', ctx)).toEqual([3])
    expect(visibleSounds(catalog, { kind: 'recents' }, '')).toEqual([2, 0])
    expect(visibleSounds(catalog, { kind: 'favourites' }, '')).toEqual([0])
    expect(instruments(catalog).map((i) => i.id)).toEqual(['sf:A.sf2', 'sf:B.sf2'])
    expect(categoryCounts(catalog.entries).find((c) => c.id === 'piano')?.count).toBe(3)
  })

  it('what a part plays, and its instrument', () => {
    const kp = { program: 0, patch: null, plugin: null, playsBass: false }
    expect(playingId(kp, ctx)).toBe('sf:A.sf2:0:0')
    // Nothing covers program 5: no row plays (not a made-up main-font row).
    expect(playingId({ ...kp, program: 5 }, ctx)).toBe(null)
    expect(playingId({ ...kp, sound: { id: 'saved:x', name: 'X' } }, ctx)).toBe('saved:x')
    expect(instrumentName(kp, ctx, [], 'Main.sf2')).toBe('A')
    expect(instrumentName({ ...kp, sound: { id: 'saved:x', name: 'X' } }, ctx, [], 'Main.sf2')).toBe('B')
    expect(instrumentName({ ...kp, plugin: { id: 'p', name: 'Synth' } }, ctx, [], null)).toBe('Synth')
    // A failed plugin plays the SoundFont voice.
    expect(instrumentName({ ...kp, plugin: { id: 'p', name: 'Synth', status: 'failed' } }, ctx, [], null)).toBe('A')
    expect(playingId({ ...kp, plugin: { id: 'p', name: 'Synth', status: 'failed', presetKey: 'f:1', preset: 'One' } }, ctx)).toBe('sf:A.sf2:0:0')
    // A bare plugin is named once, a preset after its plugin.
    const bare = { ...kp, plugin: { id: 'p', name: 'Synth' }, sound: { id: 'au:p', name: 'Synth' }, voiceName: 'Synth' }
    expect(nowPlaying(bare, ctx, [], null)).toBe('Synth')
    expect(nowPlaying({ ...bare, sound: { id: 'au:p#f:1', name: 'One' } }, ctx, [], null)).toBe('Synth · One')
  })
})

describe('what a part plays, named (api::part_sound)', () => {
  const SMP = 'aumu Smp7 Fake'
  const plugin = (id: string, name: string, origin: PluginOrigin): Patch => ({ id, name, category: 'piano', tags: [], favourite: false, source: { kind: 'plugin', componentId: SMP, hasState: false, origin } })
  const patches: Patch[] = [
    { id: 'grand', name: 'Stage Grand', category: 'piano', tags: [], favourite: false, source: { kind: 'soundFont', file: 'A.sf2', bank: 0, program: 0 } },
    plugin('warm', 'Warm Keys', { kind: 'factory', number: 3 }),
    plugin('mine', 'My Keys', { kind: 'user' }),
  ]
  // As the engine's test: auto-fill on programs 0-9 ("A<n>"), program 1 → grand, 2 → mine.
  const row = (program: number, sound: string | null, font: FontPreset | null): GmMapRow => ({ program, family: program >> 3, overrideRule: null, familyRule: null, resolved: { sound, layer: 'auto', fromStyle: false, font } })
  const gmMap = Array.from({ length: 128 }, (_, p) => {
    if (p === 1) return row(1, 'saved:grand', { file: 'A.sf2', bank: 0, program: 0 })
    if (p === 2) return row(2, 'saved:mine', null)
    return p < 10 ? row(p, `sf:A.sf2:0:${p}`, { file: 'A.sf2', bank: 0, program: p }) : row(p, null, null)
  })
  const names = { font: (f: FontPreset) => `A${f.program}`, gm: (p: number) => fixture.gm[p] }

  // The engine's test reads the same table (tests/fixtures/part_sound_cases.json).
  it.each(partSoundCases.cases)('$name', ({ of, want }) => {
    const c = of as { plugin?: { status: string; preset: string[] | null }; pluginSound?: { id: string; name: string }; own?: string; program: number }
    const input: PartSoundOf = {
      plugin: c.plugin && { id: SMP, name: 'Sampler Deluxe', status: c.plugin.status as PluginStatus, presetKey: c.plugin.preset?.[0] ?? null, preset: c.plugin.preset?.[1] ?? null },
      pluginSound: c.pluginSound,
      own: c.own,
      program: c.program,
    }
    const r = partSound(input, patches, gmMap, names)
    expect([r.sound?.id ?? null, r.sound?.name ?? null, r.voiceName]).toEqual([want[0], want[1], want[2] ?? fixture.gm[c.program]])
  })
})

describe('every part names what is playing (mock session)', () => {
  /** Right 1 on the Sampler Deluxe's factory preset 1 ("Bright Grand"). */
  function onFactory(s: MockSession) {
    s.send({ type: 'stop' })
    s.send({ type: 'listPluginPresets', id: 'au:aumu Smp7 Fake' })
    s.send({ type: 'setPartPluginPreset', part: 0, id: 'aumu Smp7 Fake', preset: 'f:1' })
    s.advance(5000)
  }

  it('a preset part is named by its preset', () => {
    const s = new MockSession({ manual: true, demo: false })
    onFactory(s)
    const kp = s.state.keyboardParts[0]
    expect([kp.sound?.id, kp.voiceName]).toEqual(['au:aumu Smp7 Fake#f:1', 'Sampler Deluxe · Bright Grand'])
  })

  it('renaming or deleting the sound a part plays renames the part', () => {
    const s = new MockSession({ manual: true, demo: false })
    onFactory(s)
    s.send({ type: 'saveSoundAs', part: 0, name: 'My Grand' })
    const id = s.state.keyboardParts[0].sound!.id.slice('saved:'.length)
    const p = s.state.soundLibrary.patches.find((x) => x.id === id)!
    s.send({ type: 'updatePatch', id, patch: { name: 'Renamed', category: p.category, tags: p.tags, favourite: p.favourite, source: p.source } })
    expect([s.state.keyboardParts[0].sound?.name, s.state.keyboardParts[0].voiceName]).toEqual(['Renamed', 'Renamed'])
    s.send({ type: 'deletePatch', id })
    expect(s.state.keyboardParts[0].voiceName).not.toContain('Renamed')
    expect(s.state.keyboardParts[0].sound?.id).not.toBe(`saved:${id}`)
  })

  it('a failed plugin is shown as failed, named by the SoundFont voice that plays', () => {
    const s = new MockSession({ manual: true, demo: false })
    s.send({ type: 'setPartPlugin', part: 1, id: 'aumu Mock Demo', state: null })
    s.advance(5000)
    const kp = s.state.keyboardParts[1]
    expect(kp.plugin?.status).toBe('failed')
    expect(kp.voiceName).not.toContain('Broken Synth')
    expect(kp.sound?.id).toMatch(/^(sf|saved):/)
  })
})
