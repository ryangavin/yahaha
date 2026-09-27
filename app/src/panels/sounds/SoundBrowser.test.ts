import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync, tick } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { MockSession } from '../../lib/api/mock'
import { app, ui } from '../../lib/store.svelte'
import { categoryCounts, playingId, visibleSounds } from './model'
import { fontLine } from './instruments'
import { browserNav } from './nav.svelte'
import SoundBrowser from './SoundBrowser.svelte'

async function setup(part = 0) {
  const session = new MockSession({ manual: true, demo: false })
  app.attach(session)
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
}

const input = () => document.querySelector<HTMLInputElement>('input[role="combobox"]')!
const rows = () => [...document.querySelectorAll<HTMLElement>('[role="option"]')]
const active = () => document.querySelector<HTMLElement>('[role="option"][aria-selected="true"]')!
const key = (k: string, o: KeyboardEventInit = {}) => fireEvent.keyDown(input(), { key: k, ...o })
const tipped = (k: string) => [...document.querySelectorAll<HTMLElement>(`[data-tip="${k}"]`)]

afterEach(() => {
  cleanup()
  app.detach()
  ui.soundBrowser = null
  ui.soundPick = null
})

describe('sound browser (#117)', () => {
  it('opens on what the part plays, with the filter focused', async () => {
    const s = await setup()
    expect(document.activeElement).toBe(input())
    // Right 1's GM voice on the main font (the program map plays it as a patch).
    expect(s.state.keyboardParts[0].patch).toBe(null)
    expect(active().textContent).toContain('Grand Piano')
    expect(active().textContent).toContain('GeneralUser-GS.sf2')
    expect(active().querySelector('.mark')!.textContent).toBe('▶')
  })

  it('filters, and Enter plays the sound on the part', async () => {
    const s = await setup(1)
    await fireEvent.input(input(), { target: { value: 'fluidr3 cello' } })
    await tick()
    flushSync()
    expect(rows().length).toBe(1)
    expect(rows()[0].textContent).toContain('SF')
    await key('Enter')
    const saved = s.state.keyboardParts[1].patch
    expect(saved).not.toBe(null)
    expect(s.state.soundLibrary.patches.find((p) => p.id === saved)?.source).toMatchObject({ kind: 'soundFont', file: 'FluidR3_GM.sf2', program: 42 })
    expect(ui.soundBrowser).toBe(1)
  })

  it('stars, lists Favourites and Recents, and auditions while stopped', async () => {
    const s = await setup()
    await key('ArrowDown')
    const name = active().querySelector('.name')!.textContent!
    await key('d', { ctrlKey: true })
    await refresh(s)
    await fireEvent.click(tipped('sounds.favourites')[0])
    flushSync()
    expect(rows().map((r) => r.querySelector('.name')!.textContent)).toContain(name)
    await fireEvent.click(rows().find((r) => r.querySelector('.name')!.textContent === name)!)
    await refresh(s)
    await fireEvent.click(tipped('sounds.recents')[0])
    flushSync()
    expect(rows()[0].querySelector('.name')!.textContent).toBe(name)
    await key('Enter', { shiftKey: true })
    expect(s.state.sounds.auditioning).not.toBe(null)
  })

  it('a plugin row shows its source and a failed load', async () => {
    await setup()
    await fireEvent.input(input(), { target: { value: 'AU' } })
    await tick()
    flushSync()
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

describe('saved sounds (#117)', () => {
  it('Save as sound keeps what the part plays and shows it under Saved', async () => {
    const s = await setup(1)
    const before = s.state.soundLibrary.patches.length
    await fireEvent.click(tipped('sounds.save')[0])
    expect(s.state.soundLibrary.patches.length).toBe(before + 1)
    const id = s.state.soundLibrary.lastAdded!
    await refresh(s)
    await tick()
    flushSync()
    expect(tipped('sounds.saved')[0].getAttribute('aria-pressed')).toBe('true')
    expect(rows().every((r) => r.textContent!.includes('Saved'))).toBe(true)
    expect(active().id).toBe(`sound-${app.sounds.entries.findIndex((e) => e.id === `saved:${id}`)}`)
    // Picking it plays it on the part.
    await key('Enter')
    expect(s.state.keyboardParts[1].patch).toBe(id)
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
    await key('ArrowDown')
    const id = active().id
    await key('Enter')
    expect(got).toMatch(/^saved:/)
    expect(id).toBeTruthy()
    expect(ui.soundPick).toBe(null)
  })
})

describe('sound browser model', () => {
  it('categories count their entries; Recents keep their order', () => {
    const e = (id: string, category: 'piano' | 'bass', favourite = false) => ({ id, name: id, category, source: 'soundFont' as const, detail: 'A.sf2', favourite, recent: false, plugin: null })
    const catalog = { revision: 1, entries: [e('a', 'piano', true), e('b', 'bass'), e('c', 'piano')], recents: ['c', 'a'] }
    expect(categoryCounts(catalog.entries).find((c) => c.id === 'piano')?.count).toBe(2)
    expect(visibleSounds(catalog, { kind: 'recents' }, '')).toEqual([2, 0])
    expect(visibleSounds(catalog, { kind: 'favourites' }, '')).toEqual([0])
    expect(visibleSounds(catalog, { kind: 'category', id: 'piano' }, 'c')).toEqual([2])
    expect(playingId({ program: 5, patch: null, plugin: null, playsBass: false }, 'A.sf2')).toBe('sf:A.sf2:0:5')
  })
})
