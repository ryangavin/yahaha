// Library (docs/racks.md, "Screens", journeys 1 and 2): the Stage | Library switch, Alt+B
// and Esc, and each tab on the mock session.

import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync, tick } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import App from '../../App.svelte'
import { MockSession } from '../../lib/api/mock'
import type { SoundCatalog } from '../../lib/api/types'
import { app, ui } from '../../lib/store.svelte'
import { NO_FILTER, badgeOf, libraryCategories, librarySounds } from './model'
import { libraryNav } from './nav.svelte'

async function setup(before?: (s: MockSession) => void) {
  const session = new MockSession({ manual: true, demo: false })
  before?.(session)
  render(App, { props: { session } })
  app.sounds = await session.sounds()
  session.advance(16)
  flushSync()
  await tick()
  flushSync()
  return session
}

/** The store re-fetches the catalog when its revision moves; tests hand it over. */
async function refresh(s: MockSession) {
  s.advance(16)
  app.sounds = await s.sounds()
  flushSync()
  await tick()
  flushSync()
}

const q = <T extends HTMLElement = HTMLElement>(sel: string) => document.querySelector<T>(sel)
const all = (sel: string) => [...document.querySelectorAll<HTMLElement>(sel)]
const tipped = (k: string) => all(`[data-tip="${k}"]`)
const rows = () => all('#library-sounds [role="option"]')
const rowNames = () => rows().map((r) => r.querySelector('.name')!.firstChild!.textContent)
const altKey = (letter: string) => fireEvent.keyDown(window, { key: letter, code: `Key${letter.toUpperCase()}`, altKey: true })
async function click(el: Element | null | undefined) {
  expect(el, 'element to click').toBeTruthy()
  await fireEvent.click(el!)
  flushSync()
  await tick()
  flushSync()
}

afterEach(() => {
  cleanup()
  app.detach()
  ui.view = 'stage'
  ui.libraryTab = 'sounds'
  ui.libraryPart = 0
  ui.parts = false
  libraryNav.reset()
})

describe('Stage | Library', () => {
  it('the header switch shows Library in place of the stage; Back to Stage and Esc return', async () => {
    await setup()
    expect(q('.stage')).toBeTruthy()
    await click(tipped('view.library')[0])
    expect(ui.view).toBe('library')
    expect(q('.stage')).toBeNull()
    expect(q('section[aria-label="Library"]')).toBeTruthy()
    expect(tipped('view.library')[0].getAttribute('aria-pressed')).toBe('true')
    await click(tipped('library.back')[0])
    expect(ui.view).toBe('stage')
    await click(tipped('view.library')[0])
    await fireEvent.keyDown(window, { key: 'Escape' })
    flushSync()
    expect(ui.view).toBe('stage')
  })

  it('Alt+B toggles it, loading into the selected part; a drawer over it closes first on Esc', async () => {
    const s = await setup()
    s.send({ type: 'selectPart', part: 3 })
    s.advance(16)
    flushSync()
    await altKey('b')
    flushSync()
    expect(ui.view).toBe('library')
    expect(ui.libraryTab).toBe('sounds')
    expect(ui.libraryPart).toBe(3)
    expect(q('.target button[aria-pressed="true"]')!.textContent).toBe('L')
    ui.parts = true
    flushSync()
    await fireEvent.keyDown(window, { key: 'Escape' })
    flushSync()
    expect(ui.parts).toBe(false)
    expect(ui.view).toBe('library')
    await altKey('b')
    flushSync()
    expect(ui.view).toBe('stage')
  })

  it('the nav strip has Library, not Sounds or Sound Library; Alt+Y opens the Style map', async () => {
    await setup()
    const labels = all('nav.quick-nav button').map((b) => b.textContent!.trim())
    expect(labels[0]).toBe('Library')
    expect(labels).not.toContain('Sounds')
    expect(labels).not.toContain('Sound Library')
    await altKey('y')
    flushSync()
    expect(ui.view).toBe('library')
    expect(ui.libraryTab).toBe('map')
    expect(tipped('sound.tab_gm')).toHaveLength(1)
    expect(q('[data-overlay="sound"]')).toBeNull()
  })

  it('a part\'s sound name in Parts & OTS opens Library › Sounds on that part', async () => {
    await setup()
    ui.parts = true
    flushSync()
    await click(all('.strip[aria-label="Right 3"] [data-tip="part.voice"]')[0])
    expect(ui.view).toBe('library')
    expect(ui.libraryPart).toBe(2)
    expect(q('[data-overlay="sounds"]')).toBeNull()
  })
})

describe('Library › Sounds', () => {
  it('a click plays the sound on the target part at once; ↑ ↓ step and play', async () => {
    const s = await setup()
    ui.openLibrary('sounds', 1)
    flushSync()
    await tick()
    libraryNav.source = 'soundFont'
    flushSync()
    const pick = rows()[2]
    const name = pick.querySelector('.name')!.firstChild!.textContent
    await click(pick)
    await refresh(s)
    expect(s.state.keyboardParts[1].voiceName).toBe(name)
    expect(q('.foot .now')!.textContent).toContain(`Right 2 plays`)
    expect(q('#library-sounds [aria-selected="true"]')!.querySelector('.mark')!.textContent).toBe('▶')
    // ↓ from the list steps to the next row and plays it.
    const next = rowNames()[rowNames().indexOf(name) + 1]
    await fireEvent.keyDown(q('#library-sounds')!, { key: 'ArrowDown' })
    flushSync()
    await refresh(s)
    expect(s.state.keyboardParts[1].voiceName).toBe(next)
    // ↑ from the search field too.
    await fireEvent.keyDown(q('input[aria-label="Search sounds"]')!, { key: 'ArrowUp' })
    flushSync()
    await refresh(s)
    expect(s.state.keyboardParts[1].voiceName).toBe(name)
  })

  it('the target part switch changes where a click loads', async () => {
    const s = await setup()
    ui.openLibrary('sounds', 0)
    flushSync()
    await click(all('[data-tip="library.target"]')[3])
    expect(ui.libraryPart).toBe(3)
    libraryNav.source = 'soundFont'
    flushSync()
    const name = rows()[1].querySelector('.name')!.firstChild!.textContent
    await click(rows()[1])
    await refresh(s)
    expect(s.state.keyboardParts[3].voiceName).toBe(name)
    expect(q('.foot .now')!.textContent).toContain('Left plays')
  })

  it('chips narrow by badge and star; the category column counts what they leave', async () => {
    await setup()
    ui.openLibrary('sounds', 0)
    flushSync()
    await click(tipped('library.src_mine')[0])
    expect(rows().length).toBeGreaterThan(0)
    expect(rows().every((r) => r.querySelector('.badge')!.textContent === 'Mine')).toBe(true)
    await click(tipped('library.src_starred')[0])
    expect(rows().every((r) => r.querySelector('.star')!.textContent === '★')).toBe(true)
    const cats = tipped('library.category')
    const total = Number(cats[0].querySelector('.n')!.textContent)
    const sum = cats.slice(1).reduce((n, c) => n + Number(c.querySelector('.n')!.textContent), 0)
    expect(sum).toBe(total)
    expect(rows()).toHaveLength(total)
  })

  it('details: a SoundFont voice copies to My Sounds; a sound of yours has its own editor', async () => {
    const s = await setup()
    ui.openLibrary('sounds', 0)
    libraryNav.source = 'soundFont'
    flushSync()
    await click(rows()[0])
    await refresh(s)
    const before = s.state.soundLibrary.patches.length
    await click(tipped('library.copy')[0])
    expect(s.state.soundLibrary.patches.length).toBe(before + 1)
    await refresh(s)
    expect(tipped('library.copy')[0].textContent).toContain('In My Sounds')
    // Right 1's own sound (Stage Grand, Mine) shows the editor, with Duplicate.
    libraryNav.source = 'mine'
    flushSync()
    await click(rows().find((r) => r.textContent!.includes('Stage Grand')))
    expect(tipped('sound.duplicate')).toHaveLength(1)
    expect(q('.details')!.textContent).toContain('Right 1 of the live rack')
  })
})

describe('Library › Instruments', () => {
  it('shows New plugins first; Browse filters Sounds to it and marks it seen', async () => {
    const s = await setup()
    ui.openLibrary('instruments', 0)
    flushSync()
    const first = all('.tile')[0]
    expect(first.querySelector('.tname')!.textContent).toContain('Tiny Synth')
    expect(first.querySelector('.badge.new')).toBeTruthy()
    await click(first.querySelector('[data-tip="library.inst_browse"]'))
    expect(s.state.plugins.list.find((p) => p.name === 'Tiny Synth')!.new).toBe(false)
    expect(ui.libraryTab).toBe('sounds')
    expect(libraryNav.instrument).toBe('au:aumu Tiny Demo')
    expect(tipped('library.inst_clear')[0].textContent).toContain('Tiny Synth')
  })

  it('a font\'s Browse lists every one of its presets', async () => {
    await setup()
    ui.openLibrary('instruments', 0)
    flushSync()
    await click(q('section[aria-label="FluidR3_GM"] [data-tip="library.inst_browse"]'))
    const n = app.sounds.entries.filter((e) => e.source === 'soundFont' && e.detail === 'FluidR3_GM.sf2').length
    expect(q('.rhead')!.textContent).toContain(`${n} sounds`)
  })

  it('+ New sound loads a blank plugin on the target part', async () => {
    const s = await setup()
    ui.openLibrary('instruments', 2)
    flushSync()
    await click(q('section[aria-label="Sampler Deluxe"] [data-tip="library.inst_new"]'))
    expect(s.state.keyboardParts[2].plugin?.id).toBe('aumu Smp7 Fake')
  })

  it('a missing plugin says how many racks use it; Show racks opens Racks, Needs attention on', async () => {
    await setup()
    ui.openLibrary('instruments', 0)
    flushSync()
    const tile = q('.tile.missing')!
    expect(tile.textContent).toContain('String Deluxe')
    expect(tile.textContent).toContain('⚠ Missing')
    expect(tile.textContent).toContain('used in 1 rack')
    await click(tile.querySelector('[data-tip="library.inst_show_racks"]'))
    expect(ui.libraryTab).toBe('racks')
    expect(tipped('library.racks_attention')[0].getAttribute('aria-pressed')).toBe('true')
    expect(tipped('library.rack_attention').map((r) => r.textContent)).toEqual([expect.stringContaining('Strings Night')])
    expect(tipped('library.rack_live')).toHaveLength(0)
  })
})

describe('Library › Racks', () => {
  it('lists the live rack, read-only, with its parts; rack actions say they are coming', async () => {
    const s = await setup()
    ui.openLibrary('racks', 0)
    flushSync()
    const live = tipped('library.rack_live')[0]
    expect(live.textContent).toContain(s.state.liveRack.name)
    expect(live.textContent).toContain(s.state.keyboardParts[0].voiceName)
    expect(tipped('library.rack_coming').every((b) => (b as HTMLButtonElement).disabled)).toBe(true)
    expect(q('.foot .now')!.textContent).toContain(`Loaded: ${s.state.liveRack.name}`)
  })
})

describe('Library model', () => {
  const catalog = (): SoundCatalog => ({
    revision: 1,
    recents: [],
    entries: [
      { id: 'sf:A.sf2:0:0', name: 'Grand', category: 'piano', source: 'soundFont', detail: 'A.sf2', favourite: false, recent: false, plugin: null },
      { id: 'sf:A.sf2:0:1', name: 'Bright', category: 'piano', source: 'soundFont', detail: 'A.sf2', favourite: false, recent: false, plugin: null },
      { id: 'saved:pad', name: 'Pad', category: 'pad', source: 'saved', detail: 'Synth', favourite: true, recent: false, plugin: null },
      { id: 'au:synth', name: 'Synth', category: 'synthLead', source: 'plugin', detail: 'Maker', favourite: false, recent: false, plugin: { format: 'AUv2', lastError: null } },
      { id: 'au:synth#f:1', name: 'Arp', category: 'synthLead', source: 'plugin', detail: 'Maker', favourite: false, recent: false, plugin: { format: 'AUv2', lastError: null }, parent: 'au:synth' },
    ],
  })
  const ctx = {
    patches: [{ id: 'pad', name: 'Pad', category: 'pad' as const, tags: [], favourite: true, source: { kind: 'plugin' as const, componentId: 'synth', state: '' } }],
    gmMap: [{ program: 0, resolved: { sound: 'sf:A.sf2:0:0' } }],
  } as unknown as Parameters<typeof librarySounds>[2]
  const names = (c: SoundCatalog, idx: number[]) => idx.map((i) => c.entries[i].name)

  it('All: the map\'s font voices, your sounds and plugins, by category; a font\'s other presets only under Browse', () => {
    const c = catalog()
    expect(names(c, librarySounds(c, NO_FILTER, ctx))).toEqual(['Grand', 'Arp', 'Synth', 'Pad'])
    expect(names(c, librarySounds(c, { ...NO_FILTER, instrument: 'sf:A.sf2' }, ctx))).toEqual(['Grand', 'Bright'])
    expect(names(c, librarySounds(c, { ...NO_FILTER, instrument: 'au:synth' }, ctx))).toEqual(['Pad', 'Synth', 'Arp'])
  })

  it('badges, chips, star, category and search', () => {
    const c = catalog()
    expect(c.entries.map(badgeOf)).toEqual(['soundFont', 'soundFont', 'mine', 'factory', 'factory'])
    expect(names(c, librarySounds(c, { ...NO_FILTER, source: 'factory' }, ctx))).toEqual(['Arp', 'Synth'])
    expect(names(c, librarySounds(c, { ...NO_FILTER, favourites: true }, ctx))).toEqual(['Pad'])
    expect(names(c, librarySounds(c, { ...NO_FILTER, category: 'piano' }, ctx))).toEqual(['Grand'])
    expect(names(c, librarySounds(c, { ...NO_FILTER, query: 'maker arp' }, ctx))).toEqual(['Arp'])
    const cats = libraryCategories(c, { ...NO_FILTER, category: 'piano' }, ctx)
    expect(cats.find((x) => x.id === 'synthLead')!.count).toBe(2)
    expect(cats.reduce((n, x) => n + x.count, 0)).toBe(4)
  })
})
