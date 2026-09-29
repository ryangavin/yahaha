import { describe, expect, it } from 'vitest'
import { MockSession } from './mock'
import { parsePluginId, parsePresetId, pluginCategory, presetId } from './sounds'
import { visibleSounds } from '../../panels/sounds/model'

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

describe('now playing, Save and Save as… (O3)', () => {
  it('a preset names its sound; its window changing a value marks it edited; Save and Save as… clear it', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'listPluginPresets', id: 'au:aumu Smp7 Fake' })
    const n = m.state.soundLibrary.patches.length
    m.send({ type: 'setPartPluginPreset', part: 0, id: 'aumu Smp7 Fake', preset: 'f:1' })
    m.advance(5000)
    // Picking a preset adds no library record (docs/racks.md "One save makes one record").
    expect(m.state.soundLibrary.patches.length).toBe(n)
    const kp = () => m.state.keyboardParts[0]
    expect(kp().sound).toEqual({ id: 'au:aumu Smp7 Fake#f:1', name: 'Bright Grand' })
    expect(kp().soundEdited).toBeUndefined()
    const factory = kp().sound!
    // Closing the window alone is no edit.
    m.send({ type: 'savePartPluginState', part: 0 })
    expect(kp().soundEdited).toBeUndefined()
    // A value changed in the open window shows at once; changing it back clears it.
    m.pluginWindow(0, 5)
    expect(kp().soundEdited).toBe(true)
    m.pluginWindow(0, 0)
    expect(kp().soundEdited).toBeUndefined()
    m.pluginWindow(0, 5)
    expect(kp().soundEdited).toBe(true)
    // A factory preset is never overwritten: Save makes one sound named after it.
    m.send({ type: 'saveSound', part: 0 })
    expect(m.state.soundLibrary.patches.length).toBe(n + 1)
    const mine = kp().sound!
    expect(mine.id).not.toBe(factory.id)
    expect(mine.name).toBe('Bright Grand')
    expect(kp().soundEdited).toBeUndefined()
    // Now the user's own: Save overwrites it, and its value is the sound's from here.
    m.pluginWindow(0, 7)
    expect(kp().soundEdited).toBe(true)
    m.send({ type: 'saveSound', part: 0 })
    expect(m.state.soundLibrary.patches.length).toBe(n + 1)
    expect(kp().sound).toEqual(mine)
    expect(kp().soundEdited).toBeUndefined()
    m.pluginWindow(0, 5)
    expect(kp().soundEdited).toBe(true)
    m.pluginWindow(0, 7)
    expect(kp().soundEdited).toBeUndefined()
    // The demo window (no real host): opening it edits, opening it again undoes that.
    m.pluginEditor(0, true)
    expect(kp().soundEdited).toBe(true)
    m.pluginEditor(0, true)
    expect(kp().soundEdited).toBeUndefined()
    m.send({ type: 'saveSoundAs', part: 0, name: 'Mine 2' })
    expect(kp().sound?.name).toBe('Mine 2')
  })

  it('saving a GM voice part again and again makes one record, which the part then plays', () => {
    const m = new MockSession({ manual: true })
    const n = m.state.soundLibrary.patches.length
    for (let i = 0; i < 3; i++) m.send({ type: 'saveSound', part: 2 })
    expect(m.state.soundLibrary.patches.length).toBe(n + 1)
    expect(m.state.keyboardParts[2].patch).toBe(m.state.soundLibrary.patches.at(-1)!.id)
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

  it('a plugin expands into its AU presets; each part plays its own preset (AU presets)', async () => {
    const m = new MockSession({ manual: true })
    const id = 'au:aumu Smp7 Fake'
    expect(parsePluginId(`${id}#f:1`)).toEqual({ plugin: 'aumu Smp7 Fake', key: 'f:1' })
    let cat = await m.sounds()
    expect(cat.entries.length).toBe(m.state.sounds.count)
    // The .aupreset files are listed from the start; the factory presets once expanded.
    const kids = () => cat.entries.filter((e) => e.parent === id)
    expect(kids().map((e) => e.name)).toEqual(['Arco Strings', 'Upright Piano'])
    // Its .aupreset files are not its count: unknown until the factory presets are listed.
    expect(cat.entries.find((e) => e.id === id)?.plugin?.presets).toBe(null)
    // Categories: guessed from the name and folder.
    expect(kids().map((e) => e.category)).toEqual(['strings', 'piano'])
    // Not in All sounds (O6): under the plugin's own chip, filtered too.
    const at = (ids: number[]) => ids.map((i) => cat.entries[i].id)
    const ctx = { patches: m.state.soundLibrary.patches, gmMap: m.state.soundLibrary.gmMap }
    expect(at(visibleSounds(cat, { kind: 'all' }, '', ctx)).some((x) => x.startsWith(`${id}#`))).toBe(false)
    expect(at(visibleSounds(cat, { kind: 'instrument', id }, '', ctx)).filter((x) => x.startsWith(`${id}#`)).length).toBe(2)
    expect(at(visibleSounds(cat, { kind: 'instrument', id }, 'upright', ctx))).toEqual([`${id}#${kids()[1].id.split('#')[1]}`])

    m.send({ type: 'listPluginPresets', id })
    cat = await m.sounds()
    expect(kids().map((e) => e.name)).toEqual(['Init', 'Bright Grand', 'Brass Stabs', 'Arco Strings', 'Upright Piano'])
    expect(cat.entries.find((e) => e.id === id)?.plugin?.presets).toBe(5)
    expect(cat.entries.length).toBe(m.state.sounds.count)
    // A plugin that does not load: its listing ends with the reason, and no count.
    m.send({ type: 'listPluginPresets', id: 'au:aumu Mock Demo' })
    const broken = (await m.sounds()).entries.find((e) => e.id === 'au:aumu Mock Demo')?.plugin
    expect(broken).toMatchObject({ presets: null, presetsError: 'timed out after 20.0 s' })
    expect(m.state.sounds.listingPresets ?? []).toEqual([])

    m.send({ type: 'assignSound', part: 0, id: `${id}#f:1` })
    m.send({ type: 'assignSound', part: 1, id: kids()[3].id })
    expect(m.state.keyboardParts[0].plugin).toMatchObject({ id: 'aumu Smp7 Fake', preset: 'Bright Grand', presetKey: 'f:1' })
    expect(m.state.keyboardParts[1].plugin).toMatchObject({ id: 'aumu Smp7 Fake', preset: 'Arco Strings' })
    m.send({ type: 'assignSound', part: 2, id: `${id}#f:9` })
    expect(m.state.message?.error).toBe(true)

    // Save as preset: a new .aupreset under the plugin, in the category picked.
    m.send({ type: 'savePartAsPluginPreset', part: 0, name: 'My Grand', category: 'organ' })
    expect(m.state.message?.error).toBe(true) // still loading
    m.advance(1000)
    m.send({ type: 'savePartAsPluginPreset', part: 0, name: 'My Grand', category: 'organ' })
    cat = await m.sounds()
    const mine = kids().find((e) => e.name === 'My Grand')
    expect(mine?.category).toBe('organ')
    expect(m.state.keyboardParts[0].plugin?.preset).toBe('My Grand')
    // The same name again: refused unless replacing (Logic's presets are shared).
    m.send({ type: 'savePartAsPluginPreset', part: 0, name: 'My Grand', category: 'pad' })
    expect(m.state.message?.error).toBe(true)
    expect((await m.sounds()).entries.find((e) => e.name === 'My Grand' && e.parent)?.category).toBe('organ')
    m.send({ type: 'savePartAsPluginPreset', part: 0, name: 'My Grand', category: 'pad', overwrite: true })
    expect((await m.sounds()).entries.find((e) => e.name === 'My Grand' && e.parent)?.category).toBe('pad')
    m.send({ type: 'savePartAsPluginPreset', part: 3, name: 'x', category: 'organ' })
    expect(m.state.message?.error).toBe(true)
  })

  it('the Instruments tab: a summary per font, and Add to my sounds adds once without playing', async () => {
    const m = new MockSession({ manual: true })
    const cat = await m.sounds()
    expect(cat.fonts?.map((f) => f.file)).toEqual(m.state.io.soundFonts)
    expect(cat.fonts?.every((f) => f.presets > 0 && f.gmPrograms <= 128)).toBe(true)
    const n = m.state.soundLibrary.patches.length
    const parts = JSON.stringify(m.state.keyboardParts)
    const upright = 'au:aumu Smp7 Fake#u:/Users/mock/Library/Audio/Presets/Fake Instruments/Sampler Deluxe/Pianos/Upright Piano.aupreset'
    for (const id of ['sf:FluidR3_GM.sf2:0:48', 'sf:FluidR3_GM.sf2:0:48', upright, upright, 'saved:stage-grand']) m.send({ type: 'addToMySounds', id })
    expect(m.state.message?.error ?? false).toBe(false)
    expect(m.state.soundLibrary.patches.length).toBe(n + 2)
    expect(m.state.soundLibrary.patches.at(-1)!.source).toMatchObject({ kind: 'plugin', origin: { kind: 'file' } })
    expect(JSON.stringify(m.state.keyboardParts)).toBe(parts)
    m.send({ type: 'addToMySounds', id: 'sf:FluidR3_GM.sf2:9:9' })
    expect(m.state.message?.error).toBe(true)
  })

  it('a factory preset is a plugin sound that can be a map rule, found again by its origin', async () => {
    const m = new MockSession({ manual: true })
    const id = 'au:aumu Smp7 Fake'
    m.send({ type: 'listPluginPresets', id })
    await m.sounds()
    m.send({ type: 'setFamilyRule', family: 0, patch: `${id}#f:1`, style: false })
    expect(m.state.message?.error ?? false).toBe(false)
    const p = m.state.soundLibrary.patches.at(-1)!
    expect(p.source).toEqual({ kind: 'plugin', componentId: 'aumu Smp7 Fake', state: '', origin: { kind: 'factory', number: 1 } })
    expect(m.state.soundLibrary.map.families[0]).toBe(p.id)
    // The same preset again is the same sound, not a copy.
    const n = m.state.soundLibrary.patches.length
    m.send({ type: 'setProgramOverride', program: 40, patch: `${id}#f:1`, style: false })
    expect(m.state.soundLibrary.patches.length).toBe(n)
    expect(m.state.soundLibrary.map.overrides.find((o) => o.program === 40)?.patch).toBe(p.id)
  })
})

describe('GM map (docs/sound-browser.md)', () => {
  it("shows each program's sound and deciding layer, the style's rules first", () => {
    const m = new MockSession({ manual: true })
    const rows = m.state.soundLibrary.gmMap
    expect(rows).toHaveLength(129)
    expect(rows[0].program).toBe(null)
    expect(rows[0].resolved.layer).toBe('drums')
    expect(rows[1 + 4].resolved).toMatchObject({ layer: 'override', sound: 'saved:warm-rhodes' })
    expect(rows[1 + 33].resolved.layer).toBe('family')
    // Organ has no rule: auto-fill from the most GM-complete font, with its provenance.
    expect(rows[1 + 16].resolved).toMatchObject({ layer: 'auto', sound: 'sf:GeneralUser-GS.sf2:0:16', font: { file: 'GeneralUser-GS.sf2', bank: 0, program: 16 } })
    m.send({ type: 'setFamilyRule', family: 2, patch: 'stage-grand', style: true })
    expect(m.state.soundLibrary.gmMap[1 + 16].resolved).toMatchObject({ layer: 'family', fromStyle: true })
  })
})
