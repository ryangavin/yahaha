import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync, tick } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { MockSession } from '../../lib/api/mock'
import { app, ui } from '../../lib/store.svelte'
import { allSoundIds, categoryCounts, instrumentName, instruments, playingId, visibleSounds } from './model'
import { fontLine } from './instruments'
import { browserNav } from './nav.svelte'
import SoundBrowser from './SoundBrowser.svelte'

async function setup(part = 0, before?: (s: MockSession) => void) {
  const session = new MockSession({ manual: true, demo: false })
  app.attach(session)
  before?.(session)
  app.sounds = await session.sounds()
  ui.soundBrowser = part
  flushSync()
  render(SoundBrowser, { props: { part } })
  await tick()
  flushSync()
  return session
}

/** The store re-fetches the catalog when its revision moves; tests hand it over. */
async function refresh(s: MockSession) {
  app.sounds = await s.sounds()
  flushSync()
  await tick()
  flushSync()
}

const input = () => document.querySelector<HTMLInputElement>('input[role="combobox"]')!
const rows = () => [...document.querySelectorAll<HTMLElement>('[role="option"]')]
const names = () => rows().map((r) => r.querySelector('.name')!.textContent)
const active = () => document.querySelector<HTMLElement>('[role="option"][aria-selected="true"]')!
const activeEntry = () => app.sounds.entries[Number(active().id.slice('sound-'.length))]
const key = (k: string, o: KeyboardEventInit = {}) => fireEvent.keyDown(input(), { key: k, ...o })
const tipped = (k: string) => [...document.querySelectorAll<HTMLElement>(`[data-tip="${k}"]`)]
const foot = () => document.querySelector('.foot')!.textContent!.replace(/\s+/g, ' ')
async function filter(q: string) {
  await fireEvent.input(input(), { target: { value: q } })
  await tick()
  flushSync()
}
async function chip(name: string) {
  await fireEvent.click(tipped('sounds.instrument').find((b) => b.textContent!.includes(name))!)
  flushSync()
}
/** Right 1 on the Sampler Deluxe's factory preset 1 ("Bright Grand"). */
function onFactory(s: MockSession) {
  s.send({ type: 'stop' })
  s.send({ type: 'listPluginPresets', id: 'au:aumu Smp7 Fake' })
  s.send({ type: 'setPartPluginPreset', part: 0, id: 'aumu Smp7 Fake', preset: 'f:1' })
  s.advance(5000)
}

afterEach(() => {
  cleanup()
  app.detach()
  ui.soundBrowser = null
  ui.soundPick = null
})

describe('Sounds tab (#117, O6)', () => {
  it('opens on what the part plays, with the filter focused', async () => {
    const s = await setup()
    expect(document.activeElement).toBe(input())
    const kp = s.state.keyboardParts[0]
    expect(active().querySelector('.mark')!.textContent).toBe('▶')
    expect(activeEntry().id).toBe(playingId(kp, s.state.io.soundFontFile, s.state.soundLibrary.gmMap))
  })

  it('All sounds is the map\'s resolved sounds and My Sounds, not every preset', async () => {
    const s = await setup()
    const ids = allSoundIds({ patches: s.state.soundLibrary.patches, gmMap: s.state.soundLibrary.gmMap })
    expect(rows().length).toBeGreaterThan(0)
    expect(document.querySelector('.count')!.textContent).toContain(`of ${app.sounds.entries.length.toLocaleString()}`)
    const all = visibleSounds(app.sounds, { kind: 'all' }, '', { patches: s.state.soundLibrary.patches, gmMap: s.state.soundLibrary.gmMap })
    expect(all.length).toBeLessThan(app.sounds.entries.length / 2)
    expect(all.every((i) => ids.has(app.sounds.entries[i].id))).toBe(true)
    // Every library sound is there, and a map row says which program it covers.
    for (const p of s.state.soundLibrary.patches) expect(all.some((i) => app.sounds.entries[i].id === `saved:${p.id}`)).toBe(true)
    expect(rows().some((r) => /GM (\d+|drums) · /.test(r.querySelector('.detail')!.textContent!))).toBe(true)
    // A preset nothing maps is not in All sounds, only under its font's chip.
    await filter('fluidr3 cello')
    expect(rows()).toHaveLength(0)
  })

  it('an instrument chip lists every preset of the font; Enter plays one on the part', async () => {
    const s = await setup(1)
    await chip('FluidR3_GM')
    expect(tipped('sounds.instrument').find((b) => b.getAttribute('aria-pressed') === 'true')!.textContent).toContain('FluidR3_GM')
    await filter('cello')
    expect(rows().length).toBe(1)
    await key('Enter')
    const saved = s.state.keyboardParts[1].patch
    expect(saved).not.toBe(null)
    expect(s.state.soundLibrary.patches.find((p) => p.id === saved)?.source).toMatchObject({ kind: 'soundFont', file: 'FluidR3_GM.sf2', program: 42 })
    expect(ui.soundBrowser).toBe(1)
  })

  it('a plugin\'s chip lists its presets (listed once) and its sounds', async () => {
    const s = await setup()
    await chip('Sampler Deluxe')
    expect(s.state.sounds.listingPresets ?? []).not.toContain('au:aumu Smp7 Fake')
    await refresh(s)
    expect(names()).toEqual(expect.arrayContaining(['Sampler Deluxe', 'Bright Grand']))
    // Another plugin's chip holds its library sound.
    await chip('DLS')
    expect(rows().some((r) => r.textContent!.includes('Mine'))).toBe(true)
  })

  it('stars, lists Favourites and Recents, and auditions while stopped', async () => {
    const s = await setup()
    await key('ArrowDown')
    while (activeEntry().favourite) await key('ArrowDown')
    const name = active().querySelector('.name')!.textContent!
    await key('d', { ctrlKey: true })
    await refresh(s)
    await fireEvent.click(tipped('sounds.favourites')[0])
    flushSync()
    expect(names()).toContain(name)
    await fireEvent.click(rows().find((r) => r.querySelector('.name')!.textContent === name)!)
    await refresh(s)
    await fireEvent.click(tipped('sounds.recents')[0])
    flushSync()
    expect(names()[0]).toBe(name)
    await key('Enter', { shiftKey: true })
    expect(s.state.sounds.auditioning).not.toBe(null)
  })

  it('a plugin that failed shows why under its chip', async () => {
    await setup()
    await chip('Broken Synth')
    const broken = rows().find((r) => r.textContent!.includes('Broken Synth'))!
    expect(broken.getAttribute('data-tip')).toBe('sounds.row_failed')
    expect(broken.textContent).toContain('⚠')
  })

  it('the footer keeps no plugin housekeeping: it moved to Instruments (O2)', async () => {
    const s = await setup()
    s.send({ type: 'setPartPlugin', part: 0, id: 'aumu dls  appl', state: null })
    s.advance(1000)
    flushSync()
    for (const k of ['sounds.set_category', 'part.plugin_edit', 'part.plugin_rescan', 'part.plugin_in_process']) expect(tipped(k), k).toHaveLength(0)
  })
})

describe('Instruments tab (O2)', () => {
  const tab = (name: string) => [...document.querySelectorAll<HTMLElement>('[role="tab"]')].find((t) => t.textContent === name)!
  const card = (name: string) => [...document.querySelectorAll<HTMLElement>('.card')].find((c) => c.querySelector('.tw')!.textContent!.includes(name))!
  const expand = async (name: string) => {
    await fireEvent.click(card(name).querySelector('.tw')!)
    flushSync()
  }
  const preset = (c: HTMLElement, name: string) => [...c.querySelectorAll<HTMLElement>('.preset')].find((p) => p.querySelector('.pname')!.textContent === name)!

  afterEach(() => {
    browserNav.tab = 'sounds'
    browserNav.open.clear()
  })

  it('lists each font with only its counts and GM completeness; expanding shows its presets', async () => {
    const s = await setup(1)
    await fireEvent.click(tab('Instruments'))
    flushSync()
    expect(tab('Instruments').getAttribute('aria-selected')).toBe('true')
    const f = app.sounds.fonts!.find((x) => x.file === 'FluidR3_GM.sf2')!
    const c = card('FluidR3_GM')
    expect(c.querySelector('.meta')!.textContent).toBe(fontLine(f))
    expect(c.querySelector('.tw')!.getAttribute('aria-expanded')).toBe('false')
    expect(c.querySelectorAll('.preset')).toHaveLength(0)
    await expand('FluidR3_GM')
    expect(card('FluidR3_GM').querySelector('.tw')!.getAttribute('aria-expanded')).toBe('true')
    const n = app.sounds.entries.filter((e) => e.source === 'soundFont' && e.detail === 'FluidR3_GM.sf2').length
    expect(card('FluidR3_GM').querySelectorAll('.preset')).toHaveLength(n)
    // Play now plays it on the part; Add to my sounds keeps it once.
    const cello = preset(card('FluidR3_GM'), 'Cello (Fluid)')
    await fireEvent.click(cello.querySelector('[data-tip="sounds.inst_play"]')!)
    expect(s.state.soundLibrary.patches.find((p) => p.id === s.state.keyboardParts[1].patch)?.source).toMatchObject({ kind: 'soundFont', file: 'FluidR3_GM.sf2', program: 42 })
    const violin = preset(card('FluidR3_GM'), 'Violin (Fluid)')
    const before = s.state.soundLibrary.patches.length
    await fireEvent.click(violin.querySelector('[data-tip="sounds.inst_add"]')!)
    expect(s.state.soundLibrary.patches.length).toBe(before + 1)
    flushSync()
    const done = preset(card('FluidR3_GM'), 'Violin (Fluid)').querySelector<HTMLButtonElement>('[data-tip="sounds.inst_add"]')!
    expect(done.disabled).toBe(true)
    expect(done.textContent).toContain('In My Sounds')
    // Collapsing hides them again.
    await expand('FluidR3_GM')
    expect(card('FluidR3_GM').querySelectorAll('.preset')).toHaveLength(0)
  })

  it('expanding a plugin lists its factory presets and .aupreset files, with its housekeeping', async () => {
    const s = await setup()
    await fireEvent.click(tab('Instruments'))
    flushSync()
    expect(card('Broken Synth').querySelector('.meta')!.textContent).toContain('⚠ timed out')
    await expand('Sampler Deluxe')
    await refresh(s)
    const c = card('Sampler Deluxe')
    expect([...c.querySelectorAll('.pname')].map((x) => x.textContent)).toEqual(['Init', 'Bright Grand', 'Brass Stabs', 'Arco Strings', 'Upright Piano'])
    // Play now: the part plays that preset.
    await fireEvent.click(preset(c, 'Bright Grand').querySelector('[data-tip="sounds.inst_play"]')!)
    expect(s.state.keyboardParts[0].plugin).toMatchObject({ id: 'aumu Smp7 Fake', presetKey: 'f:1' })
    // Category and in-process override.
    const pick = c.querySelector<HTMLSelectElement>('[data-tip="sounds.set_category"]')!
    await fireEvent.change(pick, { target: { value: 'sfx' } })
    expect((await s.sounds()).entries.find((x) => x.id === 'au:aumu Smp7 Fake')!.category).toBe('sfx')
    await fireEvent.click(c.querySelector('[data-tip="part.plugin_in_process"]')!)
    expect(s.state.plugins.list.find((p) => p.id === 'aumu Smp7 Fake')!.inProcess).toBe(true)
    // Rescan sits with the plugins.
    expect(tipped('part.plugin_rescan')).toHaveLength(1)
  })

  it('New sound from a plugin loads its default state and opens its editor once it plays', async () => {
    const s = await setup()
    const opened: number[] = []
    const orig = s.pluginEditor.bind(s)
    s.pluginEditor = (part: number, open: boolean) => (open && opened.push(part), orig(part, open))
    await fireEvent.click(tab('Instruments'))
    flushSync()
    await expand('Tiny Synth')
    await fireEvent.click(card('Tiny Synth').querySelector('[data-tip="sounds.inst_new"]')!)
    expect(s.state.keyboardParts[0].plugin).toMatchObject({ id: 'aumu Tiny Demo', status: 'loading', presetKey: null })
    flushSync()
    expect(opened).toEqual([])
    s.advance(1000)
    flushSync()
    await tick()
    flushSync()
    expect(s.state.keyboardParts[0].plugin?.status).toBe('playing')
    expect(opened).toEqual([0])
    expect(card('Tiny Synth').querySelector('[data-tip="part.plugin_edit"]')).not.toBe(null)
  })

  it('picking for a map rule shows no tabs', async () => {
    const session = new MockSession({ manual: true, demo: false })
    app.attach(session)
    app.sounds = await session.sounds()
    const pick = { title: 'Piano family', value: 'stage-grand', onpick: () => {} }
    ui.soundPick = pick
    browserNav.tab = 'instruments'
    flushSync()
    render(SoundBrowser, { props: { pick } })
    flushSync()
    expect(document.querySelectorAll('[role="tab"]')).toHaveLength(0)
    expect(input()).toBeTruthy()
  })
})

describe('the Save flow (O3)', () => {
  it('the footer says what plays; an edit shows the badge; Save keeps a factory preset as a new sound', async () => {
    const s = await setup(0, onFactory)
    expect(foot()).toContain('Right 1 plays Sampler Deluxe · Bright Grand')
    expect(tipped('sounds.edited')).toHaveLength(0)
    s.send({ type: 'savePartPluginState', part: 0 })
    flushSync()
    expect(tipped('sounds.edited')[0].textContent).toBe('edited')
    // The two old save buttons are gone: one Save and one Save as….
    expect(tipped('sounds.save_over')).toHaveLength(1)
    expect(tipped('sounds.save')).toHaveLength(1)
    expect(document.body.textContent).not.toContain('Save as preset')
    const n = s.state.soundLibrary.patches.length
    await key('s', { ctrlKey: true })
    flushSync()
    expect(s.state.soundLibrary.patches.length).toBe(n + 1)
    expect(tipped('sounds.edited')).toHaveLength(0)
    // Now the part plays its own sound: Save overwrites it.
    const mine = s.state.keyboardParts[0].sound!
    s.send({ type: 'savePartPluginState', part: 0 })
    flushSync()
    await fireEvent.click(tipped('sounds.save_over')[0])
    flushSync()
    expect(s.state.soundLibrary.patches.length).toBe(n + 1)
    expect(s.state.keyboardParts[0].sound).toEqual(mine)
    expect(tipped('sounds.edited')).toHaveLength(0)
  })

  it('Save as… names a new sound, which the part plays, shown in My Sounds', async () => {
    const s = await setup(0, onFactory)
    await key('S', { ctrlKey: true, shiftKey: true })
    await tick()
    const name = tipped('sounds.save_as_name')[0] as HTMLInputElement
    expect(document.activeElement).toBe(name)
    expect(name.value).toBe('Bright Grand')
    await fireEvent.input(name, { target: { value: 'My Grand' } })
    await fireEvent.submit(name.form!)
    flushSync()
    expect(s.state.keyboardParts[0].sound?.name).toBe('My Grand')
    expect(tipped('sounds.save_as_name')).toHaveLength(0)
    await refresh(s)
    expect(tipped('sounds.saved')[0].getAttribute('aria-pressed')).toBe('true')
    expect(activeEntry().id).toBe(s.state.keyboardParts[0].sound!.id)
    expect(foot()).toContain('Sampler Deluxe · My Grand')
  })

  it('Save as… can also keep an .aupreset, asking before it replaces one; Esc cancels', async () => {
    const s = await setup(0, (m) => (onFactory(m), m.send({ type: 'savePartAsPluginPreset', part: 0, name: 'Taken', category: 'piano' })))
    await fireEvent.click(tipped('sounds.save')[0])
    await tick()
    const name = tipped('sounds.save_as_name')[0] as HTMLInputElement
    await fireEvent.input(name, { target: { value: 'Taken' } })
    await fireEvent.click(tipped('sounds.save_preset')[0])
    flushSync()
    const n = s.state.soundLibrary.patches.length
    await fireEvent.click(tipped('sounds.save_as_confirm')[0])
    flushSync()
    expect(document.querySelector('.saveform [role="alert"]')!.textContent).toContain('Replace ‘Taken’?')
    expect(s.state.soundLibrary.patches.length).toBe(n)
    await fireEvent.click(tipped('sounds.preset_replace')[0])
    flushSync()
    expect(s.state.message?.error).not.toBe(true)
    expect(s.state.soundLibrary.patches.length).toBe(n + 1)
    // Esc closes the form without saving.
    await fireEvent.click(tipped('sounds.save')[0])
    await tick()
    await fireEvent.keyDown(tipped('sounds.save_as_name')[0], { key: 'Escape' })
    flushSync()
    expect(tipped('sounds.save_as_name')).toHaveLength(0)
    expect(document.activeElement).toBe(input())
    expect(ui.soundBrowser).toBe(0)
  })
})

describe('your sounds: rename, recategorise, delete (O6)', () => {
  it('F2 renames, the category files it, Ctrl+Delete asks and deletes', async () => {
    const s = await setup(0, (m) => m.send({ type: 'setPartPatch', part: 0, id: 'keys-au' }))
    expect(activeEntry().id).toBe('saved:keys-au')
    await key('F2')
    const name = tipped('sound.name')[0] as HTMLInputElement
    expect(document.activeElement).toBe(name)
    name.value = 'Church Keys'
    await fireEvent.keyDown(name, { key: 'Enter' })
    flushSync()
    expect(s.state.soundLibrary.patches.find((p) => p.id === 'keys-au')!.name).toBe('Church Keys')
    expect(document.activeElement).toBe(input())
    await fireEvent.change(tipped('sound.edit_category')[0], { target: { value: 'organ' } })
    expect(s.state.soundLibrary.patches.find((p) => p.id === 'keys-au')!.category).toBe('organ')
    // Details: the defaults a part takes.
    await fireEvent.click(tipped('sounds.more')[0])
    flushSync()
    await fireEvent.change(tipped('sound.default_volume')[0], { target: { value: '90' } })
    expect(s.state.soundLibrary.patches.find((p) => p.id === 'keys-au')!.defaults.volume).toBe(90)
    // Delete asks first; Keep keeps it.
    await key('Delete', { ctrlKey: true })
    await tick()
    flushSync()
    expect(document.activeElement).toBe(tipped('sounds.delete_confirm')[0])
    await fireEvent.click(tipped('sounds.delete_cancel')[0])
    flushSync()
    expect(s.state.soundLibrary.patches.some((p) => p.id === 'keys-au')).toBe(true)
    await fireEvent.click(tipped('sound.delete')[0])
    await tick()
    flushSync()
    await fireEvent.click(tipped('sounds.delete_confirm')[0])
    expect(s.state.soundLibrary.patches.some((p) => p.id === 'keys-au')).toBe(false)
  })

  it('Details exports a plugin sound as an .aupreset (O7, from the old Patches tab)', async () => {
    const s = await setup(0, (m) => {
      const p = m.state.soundLibrary.patches.find((q) => q.id === 'keys-au')!
      p.source = { kind: 'plugin', componentId: 'aumu Smp7 Fake', state: 'c2FtcGxlciBkZWx1eGU=' }
      m.send({ type: 'setPartPatch', part: 0, id: 'keys-au' })
    })
    expect(activeEntry().id).toBe('saved:keys-au')
    expect(tipped('sound.export_preset')).toHaveLength(0)
    await fireEvent.click(tipped('sounds.more')[0])
    flushSync()
    await fireEvent.click(tipped('sound.export_preset')[0])
    expect(s.state.message).toMatchObject({ error: false, text: 'Keys (AU) exported to ~/Library/Audio/Presets' })
  })
})

describe('sound browser picking for a map rule', () => {
  it('Enter hands the sound over and closes', async () => {
    const session = new MockSession({ manual: true, demo: false })
    app.attach(session)
    app.sounds = await session.sounds()
    let got: string | null = null
    const pick = { title: 'Piano family', value: 'stage-grand', onpick: (id: string) => (got = id) }
    ui.soundPick = pick
    flushSync()
    render(SoundBrowser, { props: { pick } })
    await tick()
    flushSync()
    expect(active().textContent).toContain('Stage Grand')
    expect(document.body.textContent).toContain('Pick the sound for Piano family')
    expect(tipped('sounds.save')).toHaveLength(0)
    await key('ArrowDown')
    await key('Enter')
    expect(got).toBeTruthy()
    expect(ui.soundPick).toBe(null)
  })
})

describe('sound browser model', () => {
  const e = (id: string, category: 'piano' | 'bass', favourite = false, detail = 'A.sf2') => ({ id, name: id, category, source: id.startsWith('saved:') ? ('saved' as const) : ('soundFont' as const), detail, favourite, recent: false, plugin: null })
  const catalog = { revision: 1, entries: [e('sf:A.sf2:0:0', 'piano', true), e('sf:A.sf2:0:1', 'bass'), e('sf:B.sf2:0:0', 'piano', false, 'B.sf2'), e('saved:x', 'piano')], recents: ['sf:B.sf2:0:0', 'sf:A.sf2:0:0'] }
  const patch = { id: 'x', name: 'X', category: 'piano' as const, tags: [], favourite: false, source: { kind: 'soundFont' as const, file: 'B.sf2', bank: 0, program: 3 }, defaults: { volume: null, pan: null, reverb: null, chorus: null, octave: 0 }, available: true, note: null }
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
    expect(playingId(kp, 'Main.sf2', gmMap)).toBe('sf:A.sf2:0:0')
    expect(playingId({ ...kp, program: 5 }, 'Main.sf2', gmMap)).toBe('sf:Main.sf2:0:5')
    expect(playingId({ ...kp, sound: { id: 'saved:x', name: 'X' } }, 'Main.sf2', gmMap)).toBe('saved:x')
    expect(instrumentName(kp, ctx, [], 'Main.sf2')).toBe('A')
    expect(instrumentName({ ...kp, sound: { id: 'saved:x', name: 'X' } }, ctx, [], 'Main.sf2')).toBe('B')
    expect(instrumentName({ ...kp, plugin: { id: 'p', name: 'Synth' } }, ctx, [], null)).toBe('Synth')
  })
})
