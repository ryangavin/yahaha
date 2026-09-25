import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync, tick } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { MockSession } from '../../lib/api/mock'
import { app, ui } from '../../lib/store.svelte'
import { categoryCounts, playingId, visibleSounds } from './model'
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
})

describe('sound browser (#117)', () => {
  it('opens on what the part plays, with the filter focused', async () => {
    const s = await setup()
    expect(document.activeElement).toBe(input())
    // Right 1's GM voice on the default sound set (the program map plays it as a patch).
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
