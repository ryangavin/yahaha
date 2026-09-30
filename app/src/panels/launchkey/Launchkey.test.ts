import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { MockSession, LIBRARY } from '../../lib/api/mock'
import { app, ui } from '../../lib/store.svelte'
import Launchkey from './Launchkey.svelte'
import { neighbours } from '../../lib/surface'
import { PAD_PAGES } from '../../lib/api/types'

function setup(page?: 'chord' | 'racks') {
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
  ui.multipad = false
})

const pad = (note: number) => document.querySelector<HTMLButtonElement>(`.pad[data-note="${note}"]`)!
const knob = (i: number) => document.querySelectorAll<HTMLElement>('[aria-label="Knobs"] [role="slider"]')[i]

describe('Launchkey mirror', () => {
  it('shows the 16 pads of the current page with their labels and lights', () => {
    setup()
    expect(document.querySelectorAll('.pad')).toHaveLength(16)
    expect(pad(113).textContent).toContain('MAIN B')
    expect(pad(113).dataset.level).toBe('bright')
    expect(pad(115).dataset.level).toBe('off') // this style has no Main D
  })

  it('is the knobs over the pads, with no fader bank or status display', () => {
    setup()
    expect(document.querySelector('[aria-label="Faders"]')).toBeNull()
    expect(document.querySelector('[aria-label="Status display"]')).toBeNull()
    expect(document.querySelectorAll('[aria-label="Knobs"] [role="slider"][data-tip="knobs.knob"]')).toHaveLength(8)
    // Every button beside the pads is there.
    for (const key of ['launchkey.shift', 'padpage.prev', 'padpage.next', 'style.prev', 'style.next', 'launchkey.rotary_fast', 'drawer.multipad', 'launchkey.status']) {
      expect(document.querySelector(`[data-tip="${key}"]`), key).toBeTruthy()
    }
    expect(document.querySelectorAll('[aria-label="Scene Launch and Function"] button')).toHaveLength(2)
    expect(document.querySelectorAll('[aria-label="Transport"] button')).toHaveLength(2)
  })

  it('reads like the hardware: the knob page and knobs, then Shift and the pad buttons, the pads, then the transport', () => {
    setup()
    const before = (a: string, b: string) =>
      !!(document.querySelector(a)!.compareDocumentPosition(document.querySelector(b)!) & Node.DOCUMENT_POSITION_FOLLOWING)
    expect(before('[aria-label="Knob page"]', '[aria-label="Knobs"]')).toBe(true)
    expect(before('[aria-label="Knobs"]', '[data-tip="launchkey.shift"]')).toBe(true)
    expect(before('[data-tip="launchkey.shift"]', '.pads')).toBe(true)
    expect(before('.pads', '[aria-label="Transport"]')).toBe(true)
  })

  it('shows the hardware\'s knob page, and ◀ ▶ step it as the encoder page buttons do', async () => {
    const { session } = setup()
    const name = () => document.querySelector('.page-name')!.textContent
    expect(name()).toBe(session.state.knobs.pageName)
    expect(document.body.textContent).toContain(`Knobs ${session.state.knobs.pageNumber}/${session.state.knobs.pageCount}`)
    const n = session.state.knobs.pageNumber
    await fireEvent.click(document.querySelector('[aria-label="Next knob page"]')!)
    expect(session.state.knobs.pageNumber).toBe(n + 1)
    flushSync()
    expect(name()).toBe(session.state.knobs.pageName)
    await fireEvent.click(document.querySelector('[aria-label="Previous knob page"]')!)
    expect(session.state.knobs.pageNumber).toBe(n)
    // The page the hardware moves to shows here too.
    session.send({ type: 'setKnobPage', page: 'reverb' })
    flushSync()
    expect(name()).toBe(session.state.knobs.pageName)
  })

  it('each knob shows its label and lit readout, turns relatively and resets on double-click', async () => {
    const { session } = setup()
    const cells = document.querySelectorAll('[aria-label="Knobs"] .cell')
    session.state.knobs.knobs.forEach((k, i) => {
      expect(cells[i].querySelector('.name')!.textContent).toBe(k.short)
      expect(cells[i].querySelector('.readout')!.textContent).toBe(k.value)
      expect(knob(i).getAttribute('aria-label')).toBe(k.name)
    })
    const level = session.state.dynamics.level
    knob(0).focus()
    await fireEvent.keyDown(knob(0), { key: 'ArrowDown' })
    expect(session.state.dynamics.level).toBeLessThan(level)
    await fireEvent.dblClick(knob(0))
    expect(session.state.dynamics.level).toBe(127)
  })

  it('follows the pad page', () => {
    const { session } = setup()
    session.send({ type: 'setPadPage', page: 'racks' })
    flushSync()
    expect(pad(112).textContent).toContain('OTS 1')
    expect(pad(112).dataset.tip).toBe('ots.1')
  })

  it('the Rotary button (Shift + encoder ▲ on the hardware) toggles the rotary speed and lights while fast', async () => {
    const { session } = setup()
    const rotary = () => document.querySelector<HTMLButtonElement>('[data-tip="launchkey.rotary_fast"]')!
    expect(rotary().getAttribute('aria-label')).toBe('Rotary Fast')
    expect(session.state.effects.rotaryFast).toBe(false)
    expect(rotary().getAttribute('aria-pressed')).toBe('false')
    await fireEvent.click(rotary())
    expect(session.state.effects.rotaryFast).toBe(true)
    flushSync()
    expect(rotary().getAttribute('aria-pressed')).toBe('true')
    await fireEvent.click(rotary())
    expect(session.state.effects.rotaryFast).toBe(false)
    flushSync()
    expect(rotary().getAttribute('aria-pressed')).toBe('false')
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
    await fireEvent.click(down())
    await fireEvent.click(down())
    expect(session.state.pads.page).toBe('setup')
  })

  it('a pad-page tab sets the page', async () => {
    const { session } = setup()
    await fireEvent.click(document.querySelector('[role="tab"][data-tip="padpage.chord"]')!)
    expect(session.state.pads.page).toBe('chord')
    flushSync()
    expect(document.querySelector('[role="tab"][data-tip="padpage.chord"]')!.getAttribute('aria-selected')).toBe('true')
  })

  it('the Shift layer turns Pad Bank into Left on/off and OTS Link', async () => {
    const { session } = setup()
    ui.shiftLatched = true
    flushSync()
    expect(document.querySelector('[data-tip="padpage.prev"]')).toBeNull()
    await fireEvent.click(document.querySelector<HTMLButtonElement>('[data-tip="ots.link"]')!)
    expect(session.state.ots.link).toBe(true)
  })

  it('Scene, Function, Stop and Play keep their legend and engrave what they do on the layer showing', () => {
    setup()
    const stop = () => document.querySelector('[aria-label="Transport"] .hw')!
    expect(stop().querySelector('.legend')!.textContent).toBe('■')
    expect(stop().querySelector('.caption')!.textContent).toBe('STOP')
    ui.shiftLatched = true
    flushSync()
    expect(stop().querySelector('.legend')!.textContent).toBe('■')
    expect(stop().querySelector('.caption')!.textContent).toBe('FADE')
  })

  it('the Multi Pads button opens its drawer', async () => {
    setup()
    await fireEvent.click(document.querySelector('[data-tip="drawer.multipad"]')!)
    expect(ui.multipad).toBe(true)
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
    app.attach({ kind: 'mock', subscribe: (fn) => (fn(st), () => {}), send: (c) => session.send(c), library: () => session.library(), meters: () => session.meters(), dispose: () => {} })
    flushSync()
    render(Launchkey)
    expect(document.body.textContent).toContain('Fill Up')
    const btn = document.querySelector<HTMLButtonElement>('button[aria-label="Fill Up"]')!
    expect(btn.dataset.tip).toBe('section.main_c')
    btn.click()
    expect(session.state.transport.queued).toBe('Main C')
  })

  it('names every pad page on its tab, and the connection in words', () => {
    setup()
    const tabs = [...document.querySelectorAll('[role="tab"]')]
    expect(tabs.map((t) => t.querySelector('.pname')?.textContent)).toEqual(PAD_PAGES.map((p) => p.name))
    expect(document.querySelector('.lk')!.getAttribute('data-tip')).toBe('launchkey.status')
    expect(document.querySelector('.lk .lk-text')!.textContent).toMatch(/Launchkey/)
  })

  it('under Shift a button carries both its Shift function and its printed legend', () => {
    setup()
    ui.shiftLatched = true
    flushSync()
    const up = document.querySelector('[aria-label="Pad Bank"] .btn')!
    expect(up.querySelector('.fn')?.textContent).toBeTruthy()
    expect(up.querySelector('.legend')?.textContent).toBe('▲')
  })
})
