import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync, tick } from 'svelte'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { LIBRARY, MockSession } from '../../lib/api/mock'
import { app, ui } from '../../lib/store.svelte'
import Browser from './Browser.svelte'
import { BrowserPrefs, prefs, type PrefsStorage } from './prefs.svelte'

async function setup(opts: { demo?: boolean; styles?: number; preview?: boolean } = {}) {
  const session = new MockSession({ manual: true, demo: opts.demo ?? false, styles: opts.styles })
  // An engine without the preview (older than #21): the browser hides its controls.
  if (opts.preview === false) delete (session.state as Partial<typeof session.state>).preview
  app.attach(session)
  // The store fetches a library when its revision changes; each test's session starts at
  // the same revision, so hand it over directly.
  app.library = await session.library()
  ui.browser = true
  await tick()
  flushSync()
  render(Browser)
  await tick()
  flushSync()
  return session
}

const input = () => document.querySelector<HTMLInputElement>('input[role="combobox"]')!
const rows = () => [...document.querySelectorAll<HTMLElement>('[role="option"]')]
const active = () => document.querySelector<HTMLElement>('[role="option"][aria-selected="true"]')
const key = (k: string, o: KeyboardEventInit = {}) => fireEvent.keyDown(input(), { key: k, ...o })
async function type(q: string) {
  await fireEvent.input(input(), { target: { value: q } })
  flushSync()
}

beforeEach(() => {
  prefs.setCategory({ kind: 'all' })
  prefs.setAutoPreview(false)
  for (const p of prefs.favourites) prefs.toggleFavourite(p)
})

afterEach(() => {
  cleanup()
  app.detach()
  ui.browser = false
})

describe('style browser', () => {
  it('opens on the loaded style, marked ▶, with the filter focused', async () => {
    const s = await setup()
    expect(active()?.textContent).toContain(s.state.style.name)
    expect(active()?.querySelector('.mark')?.textContent).toBe('▶')
    expect(document.activeElement).toBe(input())
  })

  it('shows tempo, time signature, section lamps and the SFF version', async () => {
    await setup()
    const r = rows()[0]
    expect(r.textContent).toContain('104')
    expect(r.textContent).toContain('4/4')
    expect(r.textContent).toContain('SFF2')
    expect(r.querySelectorAll('.lamps i.on').length).toBeGreaterThan(5)
  })

  it('filters as you type and loads with Enter, closing the browser', async () => {
    const s = await setup()
    await type('bossa')
    expect(rows()).toHaveLength(1)
    await key('Enter')
    expect(s.state.style.name).toBe('Bossa Terrace')
    expect(ui.browser).toBe(false)
  })

  it('↓/↑ move the highlight; a click loads that row', async () => {
    const s = await setup()
    await key('ArrowDown')
    flushSync()
    expect(active()?.textContent).toContain(LIBRARY.entries[1].name)
    await fireEvent.click(rows()[2])
    expect(s.state.style.id).toBe(LIBRARY.entries[2].id)
  })

  it('a folder shows its styles (and subfolders), and is remembered', async () => {
    await setup()
    const folder = [...document.querySelectorAll<HTMLButtonElement>('[data-tip="browser.folder"]')].find((b) => b.textContent?.includes('My Styles'))!
    await fireEvent.click(folder)
    flushSync()
    expect(rows().map((r) => r.querySelector('.name')!.textContent)).toEqual(['Opener Medley', 'Wedding First Dance', 'Encore Rock', 'Broken Download'])
    expect(prefs.category).toEqual({ kind: 'folder', path: 'My Styles' })
  })

  it('an unreadable style is an error row that never loads', async () => {
    const s = await setup()
    await type('broken')
    const r = rows()[0]
    expect(r.dataset.tip).toBe('browser.row_error')
    expect(r.textContent).toContain('missing MThd')
    const before = s.state.style.id
    await key('Enter')
    expect(s.state.style.id).toBe(before)
  })

  it('stars a style (☆ or Ctrl+D) into Favourites', async () => {
    await setup()
    await fireEvent.click(rows()[3].querySelector('.star')!)
    await key('d', { ctrlKey: true }) // the highlighted (loaded) row
    flushSync()
    expect(prefs.favourites.size).toBe(2)
    await fireEvent.click(document.querySelector('[data-tip="browser.favourites"]')!)
    flushSync()
    expect(rows()).toHaveLength(2)
  })

  it('stopped: Shift+Enter previews the highlighted style without loading it; ■ stops it', async () => {
    const s = await setup()
    const loaded = s.state.style.id
    await key('ArrowDown')
    await key('Enter', { shiftKey: true })
    flushSync()
    expect(s.state.preview?.audition?.id).toBe(LIBRARY.entries[1].id)
    expect(s.state.style.id).toBe(loaded)
    expect(document.body.textContent).toContain('Previewing')
    s.advance(2000) // a bar at 126 BPM
    flushSync()
    expect(s.state.preview?.audition?.bar).toBe(2)
    expect(s.state.preview?.audition?.chord).toBe('Am')
    await fireEvent.click(document.querySelector('.foot [data-tip="browser.preview_stop"]')!)
    expect(s.state.preview?.audition).toBeNull()
  })

  it('Preview on select previews the highlighted or hovered row after a pause', async () => {
    const s = await setup()
    vi.useFakeTimers()
    try {
      await fireEvent.click(document.querySelector('[data-tip="browser.auto_preview"]')!)
      expect(prefs.autoPreview).toBe(true)
      await key('ArrowDown')
      await key('ArrowDown') // moving on restarts the pause
      vi.advanceTimersByTime(300)
      expect(s.state.preview?.audition).toBeNull()
      vi.advanceTimersByTime(400)
      expect(s.state.preview?.audition?.id).toBe(LIBRARY.entries[2].id)
      await fireEvent.pointerEnter(rows()[5])
      vi.advanceTimersByTime(700)
      expect(s.state.preview?.audition?.id).toBe(LIBRARY.entries[5].id)
    } finally {
      vi.useRealTimers()
    }
  })

  it('an audition plays a short progression and then stops by itself', async () => {
    const s = await setup()
    s.send({ type: 'auditionStyle', id: 2 })
    s.advance(20000)
    expect(s.state.preview?.audition).toBeNull()
  })

  it('closing the browser stops a preview', async () => {
    const s = await setup()
    s.send({ type: 'auditionStyle', id: 1 })
    ui.browser = false
    cleanup()
    expect(s.state.preview?.audition).toBeNull()
  })

  it('playing: no preview; Shift+Enter queues the style for the next bar line', async () => {
    const s = await setup({ demo: true })
    expect(document.querySelector('[data-tip="browser.preview"]')).toBeNull()
    expect(document.querySelectorAll('[data-tip="browser.queue"]').length).toBeGreaterThan(0)
    s.send({ type: 'auditionStyle', id: 1 })
    expect(s.state.preview?.audition).toBeNull() // refused while playing
    await type('bossa')
    await key('Enter', { shiftKey: true })
    flushSync()
    const bossa = LIBRARY.entries.find((e) => e.name === 'Bossa Terrace')!.id
    expect(s.state.preview?.queued).toBe(bossa)
    expect(s.state.style.id).not.toBe(bossa)
    expect(ui.browser).toBe(true)
    s.advance(2500) // past the bar line
    expect(s.state.style.id).toBe(bossa)
    expect(s.state.transport.running).toBe(true)
    expect(s.state.preview?.queued).toBeNull()
  })

  it('the band starting swaps ▶ Preview for Next bar on every row, and ends a preview', async () => {
    const s = await setup()
    s.send({ type: 'auditionStyle', id: 1 })
    expect(document.querySelectorAll('[data-tip="browser.queue"]')).toHaveLength(0)
    s.send({ type: 'startStop' })
    flushSync()
    expect(s.state.preview?.audition).toBeNull()
    expect(document.querySelector('[data-tip="browser.preview"]')).toBeNull()
    expect(document.querySelectorAll('[data-tip="browser.queue"]').length).toBeGreaterThan(5)
  })

  it('without the engine preview API, no preview or queue controls show', async () => {
    await setup({ preview: false })
    expect(document.querySelector('[data-tip="browser.preview"]')).toBeNull()
    expect(document.querySelector('[data-tip="browser.auto_preview"]')).toBeNull()
  })

  it('< Track / Track > load the neighbours, and the highlight follows', async () => {
    const s = await setup()
    await fireEvent.click(document.querySelector('[data-tip="style.next"]')!)
    flushSync()
    expect(s.state.style.id).toBe(LIBRARY.entries[1].id)
    expect(active()?.textContent).toContain(LIBRARY.entries[1].name)
    // The Launchkey's own Track button works the same way.
    s.send({ type: 'stepStyle', delta: 1 })
    flushSync()
    expect(active()?.textContent).toContain(LIBRARY.entries[2].name)
    expect(document.querySelector('[data-tip="style.next"]')!.closest('.hw')!.textContent).toContain(LIBRARY.entries[3].name)
  })

  it('takes the Track neighbours from the engine (state.surface), not its own guess', async () => {
    await setup()
    const next = LIBRARY.entries[9]
    const surface = app.state.surface!
    app.state = { ...app.state, surface: { ...surface, trackNext: { id: next.id, name: next.name, path: next.path } } }
    flushSync()
    expect(document.querySelector('[data-tip="style.next"]')!.closest('.hw')!.textContent).toContain(next.name)
    expect(document.getElementById(`style-${next.id}`)?.querySelector('.mark')?.textContent).toBe('›')
  })

  it('Recent lists loaded styles, newest first', async () => {
    const s = await setup()
    s.send({ type: 'loadStyle', id: 5 })
    flushSync()
    s.send({ type: 'loadStyle', id: 7 })
    flushSync()
    await fireEvent.click(document.querySelector('[data-tip="browser.recents"]')!)
    flushSync()
    const names = rows().map((r) => r.querySelector('.name')!.textContent)
    expect(names.slice(0, 2)).toEqual([LIBRARY.entries.find((e) => e.id === 7)!.name, LIBRARY.entries.find((e) => e.id === 5)!.name])
  })

  it('stays virtualised with 60k styles: only the rows in view exist', async () => {
    const s = await setup({ styles: 60000 })
    expect(s.state.library.count).toBe(60043)
    expect(rows().length).toBeLessThan(60)
    expect(document.querySelector<HTMLElement>('[role="listbox"]')!.style.height).toBe(`${60043 * 36}px`)
    await type('blue sw')
    expect(document.querySelector('.count')!.textContent).toMatch(/^\s*\d+ of 60,043/)
  })
})

describe('prefs', () => {
  it('persist through the storage and survive junk', () => {
    let saved: string | null = null
    const store: PrefsStorage = { load: () => (saved ? JSON.parse(saved) : null), save: (s) => (saved = JSON.stringify(s)) }
    const a = new BrowserPrefs(store)
    a.toggleFavourite('/x.sty')
    a.noteLoaded('/a.sty')
    a.noteLoaded('/b.sty')
    a.noteLoaded('/a.sty')
    a.setAutoPreview(true)
    a.setCategory({ kind: 'folder', path: 'Pop' })
    const b = new BrowserPrefs(store)
    expect([...b.favourites]).toEqual(['/x.sty'])
    expect(b.recents).toEqual(['/a.sty', '/b.sty'])
    expect(b.autoPreview).toBe(true)
    expect(b.category).toEqual({ kind: 'folder', path: 'Pop' })
    const junk = new BrowserPrefs({ load: () => ({ favourites: 'no', category: { kind: 'nope' } }) as never, save: () => {} })
    expect(junk.favourites.size).toBe(0)
    expect(junk.category).toEqual({ kind: 'all' })
  })
})
