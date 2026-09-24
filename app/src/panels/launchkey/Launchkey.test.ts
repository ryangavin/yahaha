import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { MockSession, LIBRARY } from '../../lib/api/mock'
import { app, ui } from '../../lib/store.svelte'
import Launchkey from './Launchkey.svelte'
import { neighbours } from '../../lib/surface'

function setup(page?: 'chordSetup' | 'otsParts') {
  const session = new MockSession({ manual: true, demo: true })
  app.attach(session)
  if (page) session.send({ type: 'setPadPage', page })
  flushSync()
  const r = render(Launchkey)
  return { session, ...r }
}

afterEach(() => {
  cleanup()
  app.detach()
  ui.shiftLatched = false
})

const pad = (note: number) => document.querySelector<HTMLButtonElement>(`.pad[data-note="${note}"]`)!

describe('Launchkey mirror', () => {
  it('shows the 16 pads of the current page with their labels and lights', () => {
    setup()
    expect(document.querySelectorAll('.pad')).toHaveLength(16)
    expect(pad(113).textContent).toContain('MAIN B')
    expect(pad(113).dataset.level).toBe('bright')
    expect(pad(115).dataset.level).toBe('off') // this style has no Main D
  })

  it('reads left to right like the hardware: faders, then the pads, then the transport', () => {
    setup()
    const before = (a: string, b: string) =>
      !!(document.querySelector(a)!.compareDocumentPosition(document.querySelector(b)!) & Node.DOCUMENT_POSITION_FOLLOWING)
    expect(before('[aria-label="Faders"]', '.pads')).toBe(true)
    expect(before('.pads', '[aria-label="Transport"]')).toBe(true)
  })

  it('follows the pad page', () => {
    const { session } = setup()
    session.send({ type: 'setPadPage', page: 'otsParts' })
    flushSync()
    expect(pad(96).textContent).toContain('OTS 1')
    expect(pad(96).dataset.tip).toBe('ots.1')
  })

  it('clicking a pad sends its action', async () => {
    const { session } = setup()
    await fireEvent.click(pad(114))
    expect(session.state.transport.queued).toBe('Main C')
  })

  it('Pad Bank ▼ steps the page and stops at the last one', async () => {
    const { session } = setup()
    const down = () => document.querySelector<HTMLButtonElement>('[data-tip="padpage.next"]')!
    await fireEvent.click(down())
    await fireEvent.click(down())
    await fireEvent.click(down())
    expect(session.state.pads.page).toBe('otsParts')
  })

  it('the Shift layer turns Pad Bank into Left on/off and OTS Link, and fader buttons into Edit', async () => {
    const { session } = setup()
    ui.shiftLatched = true
    flushSync()
    expect(document.querySelector('[data-tip="padpage.prev"]')).toBeNull()
    await fireEvent.click(document.querySelector<HTMLButtonElement>('[data-tip="ots.link"]')!)
    expect(session.state.ots.link).toBe(true)
    await fireEvent.click(document.querySelector<HTMLButtonElement>('[data-tip="part.right2.select"]')!)
    expect(session.state.keyboardParts[1].selected).toBe(true)
  })

  it('the master fader button switches the fader page, and the faders follow', async () => {
    const { session } = setup()
    expect(document.querySelectorAll('[data-tip="launchkey.fader_unused"][role="slider"]')).toHaveLength(4)
    await fireEvent.click(document.querySelector<HTMLButtonElement>('button[data-tip="mixer.page"]')!)
    expect(session.state.mixer.faderPage).toBe('style')
    flushSync()
    expect(document.querySelectorAll('[data-tip="mixer.style.volume"][role="slider"]')).toHaveLength(8)
  })

  it('a fader moves its level and clears the waiting mark', async () => {
    const { session } = setup()
    session.send({ type: 'toggleFaderPage' })
    flushSync()
    expect(document.querySelectorAll('.pickup.on').length).toBeGreaterThan(0)
    const f = document.querySelectorAll<HTMLElement>('[role="slider"]')[0]
    f.focus()
    await fireEvent.keyDown(f, { key: 'ArrowUp' })
    expect(session.state.mixer.styleParts[0].waiting).toBe(false)
  })

  it('Track buttons show the neighbouring styles, skipping unreadable files', () => {
    setup()
    const n = neighbours(LIBRARY, 0)
    expect(n.next?.name).toBe(LIBRARY.entries[1].name)
    expect(LIBRARY.entries.find((e) => e.id === n.prev?.id)?.status).toBe('ok')
    expect(document.body.textContent).toContain(n.next!.name)
  })

  it('renders buttons from state.surface when the engine sends it (nothing hard-coded)', () => {
    const session = new MockSession({ manual: true, demo: true })
    const st = structuredClone(session.state)
    const scene = st.surface!.controls.find((c) => c.id === 'scene')!
    scene.label = 'Fill Up'
    scene.action = { type: 'main', index: 2 }
    app.attach({ kind: 'mock', subscribe: (fn) => (fn(st), () => {}), send: (c) => session.send(c), library: () => session.library(), dispose: () => {} })
    flushSync()
    render(Launchkey)
    expect(document.body.textContent).toContain('Fill Up')
    const btn = document.querySelector<HTMLButtonElement>('button[aria-label="Fill Up"]')!
    expect(btn.dataset.tip).toBe('section.main_c')
    btn.click()
    expect(session.state.transport.queued).toBe('Main C')
  })

  it('marks where the hardware fader is while a level waits for it', () => {
    const { session } = setup()
    session.send({ type: 'toggleFaderPage' })
    flushSync()
    expect(document.querySelectorAll('.hw').length).toBeGreaterThan(0)
  })
})
