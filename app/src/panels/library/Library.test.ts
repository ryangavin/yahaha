// Library (docs/racks.md, "Screens", journeys 1 and 2): the Stage | Library switch, Alt+B
// and Esc, and each tab on the mock session.

import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync, tick } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import App from '../../App.svelte'
import { MockSession } from '../../lib/api/mock'
import type { SoundCatalog } from '../../lib/api/types'
import { app, ui } from '../../lib/store.svelte'
import { emptyQuickRacks } from '../../lib/api/quick-racks'
import { NO_FILTER, badgeOf, libraryCategories, librarySounds, quickButtonsOf, searchRacks } from './model'
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
  ui.rack = false
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
    ui.rack = true
    flushSync()
    await fireEvent.keyDown(window, { key: 'Escape' })
    flushSync()
    expect(ui.rack).toBe(false)
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
    expect(labels).not.toContain('Parts & OTS')
    expect(labels.filter((l) => l === 'Rack')).toHaveLength(1)
    await altKey('y')
    flushSync()
    expect(ui.view).toBe('library')
    expect(ui.libraryTab).toBe('map')
    expect(tipped('sound.tab_gm')).toHaveLength(1)
    expect(q('[data-overlay="sound"]')).toBeNull()
  })

  it('a part\'s sound name in the Rack drawer opens Library › Sounds on that part', async () => {
    await setup()
    ui.rack = true
    flushSync()
    await click(all('[aria-label="Right 3"] [data-tip="rack.sound"]')[0])
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

describe('Library › Save as…', () => {
  it('a plugin part can also keep an .aupreset, asking before it replaces one; Esc cancels', async () => {
    const s = await setup((m) => {
      m.send({ type: 'stop' })
      m.send({ type: 'listPluginPresets', id: 'au:aumu Smp7 Fake' })
      m.send({ type: 'setPartPluginPreset', part: 0, id: 'aumu Smp7 Fake', preset: 'f:1' })
      m.advance(5000)
      m.send({ type: 'savePartAsPluginPreset', part: 0, name: 'Taken', category: 'piano' })
    })
    ui.openLibrary('sounds', 0)
    flushSync()
    await tick()
    await click(tipped('sounds.save')[0])
    const name = tipped('sounds.save_as_name')[0] as HTMLInputElement
    await fireEvent.input(name, { target: { value: 'Taken' } })
    await click(tipped('sounds.save_preset')[0])
    expect(tipped('sounds.preset_category')).toHaveLength(1)
    const n = s.state.soundLibrary.patches.length
    await click(tipped('sounds.save_as_confirm')[0])
    expect(q('.foot [role="alert"]')!.textContent).toContain('Replace ‘Taken’?')
    expect(s.state.soundLibrary.patches.length).toBe(n)
    await click(tipped('sounds.preset_replace')[0])
    expect(s.state.message?.error).not.toBe(true)
    expect(s.state.soundLibrary.patches.length).toBe(n + 1)
    expect(tipped('sounds.save_as_name')).toHaveLength(0)
    // Esc closes the form without saving.
    await click(tipped('sounds.save')[0])
    await fireEvent.keyDown(tipped('sounds.save_as_name')[0], { key: 'Escape' })
    flushSync()
    expect(tipped('sounds.save_as_name')).toHaveLength(0)
    expect(ui.view).toBe('library')
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
  /** Ballad on Quick Rack A1 and A3, then Evening saved and loaded: Ballad isn't loaded. */
  async function twoRacks() {
    const s = await setup((m) => {
      m.send({ type: 'toggleQuickRackStore' })
      m.send({ type: 'pressQuickRack', slot: 0 })
      m.send({ type: 'saveRackAs', name: 'Ballad' })
      m.send({ type: 'toggleQuickRackStore' })
      m.send({ type: 'pressQuickRack', slot: 2 })
      m.send({ type: 'saveRackAs', name: 'Evening' })
    })
    ui.openLibrary('racks', 0)
    await refresh(s)
    return s
  }
  const rackRows = () => tipped('library.rack_row')
  const rackRow = (name: string) => rackRows().find((r) => r.querySelector('.name')!.firstChild!.textContent === name)
  const idOf = (s: MockSession, name: string) => s.state.racks.find((r) => r.name === name)!.id
  const details = () => q('[aria-label="Rack details"]')!

  it('lists the live rack with its parts; a never-saved rack\'s details say how to make it yours', async () => {
    const s = await setup()
    ui.openLibrary('racks', 0)
    flushSync()
    const live = tipped('library.rack_live')[0]
    expect(live.textContent).toContain(s.state.liveRack.name)
    expect(live.textContent).toContain(s.state.keyboardParts[0].voiceName)
    expect(tipped('library.rack_name')).toHaveLength(0)
    expect(details().textContent).toContain('Never saved')
    expect(q('.foot .now')!.textContent).toContain(`Loaded: ${s.state.liveRack.name}`)
  })

  it('a click selects a rack and shows its details without loading it; double-click or Load loads it', async () => {
    const s = await twoRacks()
    // With nothing clicked, the details are the loaded rack's, with its split.
    expect((tipped('library.rack_name')[0] as HTMLInputElement).value).toBe('Evening')
    expect(details().textContent).toContain('Split')
    await click(rackRow('Ballad'))
    expect(s.state.liveRack.name).toBe('Evening')
    expect(rackRow('Ballad')!.getAttribute('aria-selected')).toBe('true')
    expect((tipped('library.rack_name')[0] as HTMLInputElement).value).toBe('Ballad')
    expect(details().textContent).toContain('A1, A3')
    expect(details().textContent).toContain('show once it\'s loaded')
    await click(tipped('library.rack_load')[0])
    await refresh(s)
    expect(s.state.liveRack.name).toBe('Ballad')
    expect(tipped('library.rack_load')).toHaveLength(0)
    await fireEvent.dblClick(rackRow('Evening')!)
    await refresh(s)
    expect(s.state.liveRack.name).toBe('Evening')
  })

  it('the search narrows the racks by name or sound; Esc clears it', async () => {
    await twoRacks()
    const search = tipped('library.racks_search')[0] as HTMLInputElement
    await fireEvent.input(search, { target: { value: 'ball' } })
    flushSync()
    expect(rackRows().map((r) => r.querySelector('.name')!.firstChild!.textContent)).toEqual(['Ballad'])
    expect(tipped('library.rack_live')).toHaveLength(0)
    await fireEvent.input(search, { target: { value: 'zzz' } })
    flushSync()
    expect(rackRows()).toHaveLength(0)
    expect(q('.racks')!.textContent).toContain('No racks match')
    await fireEvent.keyDown(search, { key: 'Escape' })
    flushSync()
    expect(rackRows()).toHaveLength(2)
    expect(ui.view).toBe('library')
  })

  it('renames a rack from its name field; a taken name is refused and the field shows the old name', async () => {
    const s = await twoRacks()
    await click(rackRow('Ballad'))
    const name = () => tipped('library.rack_name')[0] as HTMLInputElement
    await fireEvent.input(name(), { target: { value: 'Slow Ballad' } })
    await fireEvent.change(name())
    await refresh(s)
    expect(s.state.racks.map((r) => r.name)).toEqual(['Evening', 'Slow Ballad'])
    expect(name().value).toBe('Slow Ballad')
    expect(rackRow('Slow Ballad')!.getAttribute('aria-selected')).toBe('true')
    await fireEvent.input(name(), { target: { value: 'Evening' } })
    await fireEvent.change(name())
    await refresh(s)
    expect(s.state.message).toMatchObject({ error: true })
    expect(s.state.racks.map((r) => r.name)).toEqual(['Evening', 'Slow Ballad'])
    expect(name().value).toBe('Slow Ballad')
    // Esc puts the name back without renaming or leaving Library.
    await fireEvent.input(name(), { target: { value: 'Nope' } })
    await fireEvent.keyDown(name(), { key: 'Escape' })
    flushSync()
    expect(name().value).toBe('Slow Ballad')
    expect(ui.view).toBe('library')
  })

  it('Duplicate copies the rack and selects the copy', async () => {
    const s = await twoRacks()
    await click(rackRow('Ballad'))
    await click(tipped('library.rack_duplicate')[0])
    await refresh(s)
    expect(s.state.racks.map((r) => r.name)).toEqual(['Ballad', 'Ballad copy', 'Evening'])
    expect(rackRow('Ballad copy')!.getAttribute('aria-selected')).toBe('true')
    expect((tipped('library.rack_name')[0] as HTMLInputElement).value).toBe('Ballad copy')
  })

  it('a refused Duplicate doesn\'t select a rack added later', async () => {
    const s = await twoRacks()
    const send = s.send.bind(s)
    s.send = (c) => send(c.type === 'duplicateRack' ? { ...c, id: 'nope' } : c)
    await click(rackRow('Ballad'))
    await click(tipped('library.rack_duplicate')[0])
    await refresh(s)
    expect(s.state.message).toMatchObject({ error: true })
    s.send = send
    s.send({ type: 'newRack' })
    s.send({ type: 'saveRackAs', name: 'Later' })
    await refresh(s)
    expect(rackRow('Later')!.getAttribute('aria-selected')).toBe('false')
    expect(rackRow('Ballad')!.getAttribute('aria-selected')).toBe('true')
  })

  it('the confirm closes when the rack it asks about gets loaded', async () => {
    const s = await twoRacks()
    await click(rackRow('Ballad'))
    await click(tipped('library.rack_delete')[0])
    expect(q('.confirm')).not.toBeNull()
    s.send({ type: 'loadRack', id: idOf(s, 'Ballad') })
    await refresh(s)
    expect(q('.confirm')).toBeNull()
    expect(details().textContent).toContain('load another rack to delete it')
  })

  it('clicking the live rack row goes back to the loaded rack\'s details', async () => {
    await twoRacks()
    await click(rackRow('Ballad'))
    expect((tipped('library.rack_name')[0] as HTMLInputElement).value).toBe('Ballad')
    await click(tipped('library.rack_live')[0])
    expect((tipped('library.rack_name')[0] as HTMLInputElement).value).toBe('Evening')
    expect(rackRow('Evening')!.getAttribute('aria-selected')).toBe('true')
  })

  it('Delete… asks inline, naming the Quick Rack buttons it empties; Cancel keeps it, Delete deletes', async () => {
    const s = await twoRacks()
    const ballad = idOf(s, 'Ballad')
    await click(rackRow('Ballad'))
    await click(tipped('library.rack_delete')[0])
    const confirm = q('.confirm')!
    expect(confirm.textContent).toContain('Delete Ballad?')
    expect(confirm.textContent).toContain('Quick Racks A1, A3 will be emptied')
    await click(tipped('library.rack_delete_cancel')[0])
    expect(q('.confirm')).toBeNull()
    expect(s.state.racks).toHaveLength(2)
    await click(tipped('library.rack_delete')[0])
    await click(tipped('library.rack_delete_confirm')[0])
    await refresh(s)
    expect(s.state.racks.map((r) => r.name)).toEqual(['Evening'])
    expect(s.state.quickRacks.buttons.some((b) => b.rack === ballad)).toBe(false)
    // The details go back to the loaded rack's.
    expect((tipped('library.rack_name')[0] as HTMLInputElement).value).toBe('Evening')
  })

  it('the loaded rack can\'t be deleted: Delete… is disabled and says why', async () => {
    const s = await twoRacks()
    await click(rackRow('Evening'))
    const del = tipped('library.rack_delete')[0]
    expect(del.getAttribute('aria-disabled')).toBe('true')
    expect(details().textContent).toContain('load another rack to delete it')
    await click(del)
    expect(q('.confirm')).toBeNull()
    expect(s.state.racks).toHaveLength(2)
  })

  it('Style racks: an OTS row picks a rack of mine for this style; Load and the Rack panel follow; Style\'s own puts it back', async () => {
    const s = await twoRacks()
    const ballad = idOf(s, 'Ballad')
    expect(s.state.ots.settings.length).toBeGreaterThanOrEqual(2)
    expect(document.body.textContent).toContain(`Style racks: ${s.state.style.name} (OTS buttons 1–4)`)
    const rows = () => all('[aria-label="Style racks"] [role="listitem"]')
    const libSelect = (i: number) => rows()[i].querySelector('select')!
    const panelSelect = (i: number) => all('.otsrack')[i] as HTMLSelectElement
    expect(rows()).toHaveLength(Math.min(4, s.state.ots.settings.length))
    expect(rows()[1].textContent).toContain(`OTS 2 · ${s.state.style.name}'s own`)
    expect(libSelect(1).value).toBe('')

    libSelect(1).value = ballad
    await fireEvent.change(libSelect(1))
    await refresh(s)
    expect(s.state.ots.racks[1]).toEqual({ rack: ballad, name: 'Ballad', missing: false })
    expect(rows()[1].textContent).toContain('OTS 2 · Ballad')
    expect(panelSelect(1).value).toBe(ballad)
    expect(all('.otsw')[1].textContent).toContain('Ballad')

    // Load on the row recalls OTS 2: Ballad loads (Evening is loaded, unmodified).
    await click(tipped('library.style_rack_load')[1])
    await refresh(s)
    expect(s.state.liveRack.name).toBe('Ballad')
    expect(s.state.ots.applied).toBe(2)

    // Style's own, from the Rack panel's OTS card.
    panelSelect(1).value = ''
    await fireEvent.change(panelSelect(1))
    await refresh(s)
    expect(s.state.ots.racks[1]).toEqual({ rack: null, name: '', missing: false })
    expect(libSelect(1).value).toBe('')
  })

  it('+ New rack starts one; with unsaved changes the docked Rack panel asks first', async () => {
    const s = await twoRacks()
    s.send({ type: 'setPartVoice', part: 0, program: 12 })
    await refresh(s)
    expect(s.state.liveRack.modified).toBe(true)
    await click(tipped('library.rack_new')[0])
    await refresh(s)
    expect(s.state.liveRack.name).toBe('Evening')
    expect(s.state.liveRack.prompt).toMatchObject({ kind: 'unsavedChanges', then: { kind: 'new' } })
    expect(tipped('rack.discard_switch')).toHaveLength(1)
    await click(tipped('rack.discard_switch')[0])
    await refresh(s)
    expect(s.state.liveRack).toMatchObject({ name: 'New rack', id: null, modified: false })
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

  it('racks: search by name or the sounds of parts that are on; Quick Rack labels of the bank on view', () => {
    const racks = [
      { id: 'a', name: 'Ballad', parts: ['Grand', 'Strings', 'Brass', 'Bass'], on: [true, true, false, false], needsAttention: false },
      { id: 'b', name: 'Organ Night', parts: ['Organ', 'Brass', 'Pad', 'Bass'], on: [true, true, false, false], needsAttention: false },
    ]
    expect(searchRacks(racks, '').map((r) => r.id)).toEqual(['a', 'b'])
    expect(searchRacks(racks, 'brass').map((r) => r.id)).toEqual(['b'])
    expect(searchRacks(racks, 'ball strings').map((r) => r.id)).toEqual(['a'])
    expect(searchRacks(racks, 'pad')).toEqual([])
    const q = emptyQuickRacks()
    q.bank = 1
    q.buttons[0].rack = 'a'
    q.buttons[5].rack = 'a'
    q.buttons[2].rack = 'b'
    expect(quickButtonsOf(q, 'a')).toEqual(['B1', 'B6'])
    expect(quickButtonsOf(q, 'c')).toEqual([])
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
