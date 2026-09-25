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
