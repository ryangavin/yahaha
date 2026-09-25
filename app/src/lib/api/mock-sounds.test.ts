import { describe, expect, it } from 'vitest'
import { MockSession } from './mock'
import { parsePresetId, pluginCategory, presetId } from './sounds'

describe('sound catalog (#117)', () => {
  it('ids round-trip, even with a colon in the file', () => {
    expect(presetId('GM.sf2', 128, 0)).toBe('sf:GM.sf2:128:0')
    expect(parsePresetId('sf:a:b.sf2:8:4')).toEqual({ file: 'a:b.sf2', bank: 8, program: 4 })
    expect(parsePresetId('sf:GM.sf2:0:128')).toBe(null)
    expect(parsePresetId('au:aumu dls  appl')).toBe(null)
    expect(pluginCategory('Lounge Lizard EP-4', 'AAS')).toBe('ePiano')
    expect(pluginCategory('Serum', 'Xfer Records')).toBe('synthLead')
  })

  it('lists every preset, plugin and saved sound; assigning routes by source', async () => {
    const m = new MockSession({ manual: true })
    const cat = await m.sounds()
    expect(cat.entries.length).toBe(m.state.sounds.count)
    expect(cat.revision).toBe(m.state.sounds.revision)
    expect(cat.entries.find((e) => e.id === 'au:aumu dls  appl')?.plugin).not.toBe(null)

    m.send({ type: 'assignSound', part: 1, id: 'sf:GeneralUser-GS.sf2:0:33' })
    expect(m.state.keyboardParts[1].program).toBe(33)
    const n = m.state.soundLibrary.patches.length
    m.send({ type: 'assignSound', part: 0, id: 'sf:FluidR3_GM.sf2:0:48' })
    m.send({ type: 'assignSound', part: 2, id: 'sf:FluidR3_GM.sf2:0:48' })
    expect(m.state.soundLibrary.patches.length).toBe(n + 1)
    expect(m.state.keyboardParts[0].patch).toBe(m.state.keyboardParts[2].patch)
    m.send({ type: 'assignSound', part: 3, id: 'au:aumu dls  appl' })
    expect(m.state.keyboardParts[3].plugin?.id).toBe('aumu dls  appl')

    const rev = m.state.sounds.revision
    m.send({ type: 'setSoundFavourite', id: 'au:aumu dls  appl', on: true })
    expect(m.state.sounds.revision).toBeGreaterThan(rev)
    const after = await m.sounds()
    expect(after.recents[0]).toBe('au:aumu dls  appl')
    expect(after.entries.find((e) => e.id === 'au:aumu dls  appl')).toMatchObject({ favourite: true, recent: true })

    m.send({ type: 'assignSound', part: 0, id: 'sf:FluidR3_GM.sf2:0:200' })
    expect(m.state.message?.error).toBe(true)
  })

  it('a SoundFont sound ends a plugin picked for the part (#171 review)', () => {
    const m = new MockSession({ manual: true })
    const cases: [number, string][] = [
      [0, 'sf:GeneralUser-GS.sf2:0:0'], // the synth's own font: the part's GM voice
      [1, 'sf:FluidR3_GM.sf2:0:48'],
      [2, 'saved:stage-grand'],
    ]
    for (const [part, id] of cases) {
      m.send({ type: 'assignSound', part, id: 'au:aumu dls  appl' })
      expect(m.state.keyboardParts[part].plugin?.id).toBe('aumu dls  appl')
      m.send({ type: 'assignSound', part, id })
      expect(m.state.keyboardParts[part].plugin, id).toBeUndefined()
    }
    expect(m.state.keyboardParts[0].patch).toBe(null)
  })

  it('a GM voice selection ends a plugin picked for the part: Voice −/+, setPartVoice, an OTS voice (#179)', () => {
    const m = new MockSession({ manual: true })
    const pick = (part: number) => {
      m.send({ type: 'setPartPlugin', part, id: 'aumu dls  appl', state: null })
      expect(m.state.keyboardParts[part].plugin?.id).toBe('aumu dls  appl')
    }
    pick(1)
    m.send({ type: 'selectPart', part: 1 })
    m.send({ type: 'stepVoice', delta: 1 })
    expect(m.state.keyboardParts[1].plugin, 'Voice +').toBeUndefined()
    pick(2)
    m.send({ type: 'setPartVoice', part: 2, program: 40 })
    expect(m.state.keyboardParts[2].plugin, 'setPartVoice').toBeUndefined()
    const i = m.state.ots.settings.findIndex((o) => o.parts[0].program !== null)
    expect(i).toBeGreaterThanOrEqual(0)
    pick(0)
    m.send({ type: 'recallOts', index: i })
    expect(m.state.keyboardParts[0].plugin, 'OTS').toBeUndefined()
  })

  it('auditions a sound while the band is stopped', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'auditionSound', id: 'sf:GeneralUser-GS.sf2:128:0' })
    expect(m.state.sounds.auditioning).toBe('sf:GeneralUser-GS.sf2:128:0')
    m.advance(3100)
    expect(m.state.sounds.auditioning).toBe(null)
  })

  it('program map rules take catalog ids: a preset or plugin becomes a patch once', () => {
    const m = new MockSession({ manual: true })
    const n = m.state.soundLibrary.patches.length
    m.send({ type: 'setFamilyRule', family: 2, patch: 'au:aumu samp appl', style: false })
    m.send({ type: 'setDrumRule', patch: 'au:aumu samp appl', style: true })
    expect(m.state.soundLibrary.patches.length).toBe(n + 1)
    const id = m.state.soundLibrary.patches[n].id
    expect(m.state.soundLibrary.patches[n].source).toMatchObject({ kind: 'plugin', componentId: 'aumu samp appl' })
    expect(m.state.soundLibrary.map.families[2]).toBe(id)
    expect(m.state.soundLibrary.styleMap.drums).toBe(id)
    m.send({ type: 'setProgramOverride', program: 5, patch: 'sf:FluidR3_GM.sf2:0:5', style: false })
    expect(m.state.soundLibrary.patches.length).toBe(n + 2)
    m.send({ type: 'setDrumRule', patch: 'sf:Nope.sf2:0:0', style: false })
    expect(m.state.message?.error).toBe(true)
  })
})

describe('savePartAsPatch (#109)', () => {
  it('saves what the part plays: the mapped patch, else its plugin', () => {
    const m = new MockSession({ manual: true })
    const family = Math.floor(m.state.keyboardParts[1].program / 8)
    m.send({ type: 'setFamilyRule', family, patch: 'sf:FluidR3_GM.sf2:0:50', style: false })
    const mapped = m.state.soundLibrary.map.families[family]
    m.send({ type: 'savePartAsPatch', part: 1, name: null })
    const saved = m.state.soundLibrary.patches.at(-1)!
    expect(saved.id).not.toBe(mapped)
    expect(saved.source).toMatchObject({ kind: 'soundFont', file: 'FluidR3_GM.sf2', program: 50 })

    m.send({ type: 'assignSound', part: 3, id: 'au:aumu dls  appl' })
    m.send({ type: 'savePartAsPatch', part: 3, name: 'Mine' })
    const p = m.state.soundLibrary.patches.at(-1)!
    expect(p.name).toBe('Mine')
    expect(p.source).toMatchObject({ kind: 'plugin', componentId: 'aumu dls  appl' })
  })
})
