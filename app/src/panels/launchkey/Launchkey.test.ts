import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { MockSession, LIBRARY } from '../../lib/api/mock'
import { app, ui } from '../../lib/store.svelte'
import Launchkey from './Launchkey.svelte'
import { neighbours } from '../../lib/surface'
import type { AppCmd, AppState, KnobFunction, Layer } from '../../lib/api/types'
import { TIPS } from '../../help/tooltips'

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
    const { session } = setup()
    const tabs = [...document.querySelectorAll('[role="tab"]')]
    expect(tabs.map((t) => t.querySelector('.pname')?.textContent)).toEqual(session.state.pads.pages.map((p) => p.name))
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

/** The mirror on `st` as the engine would send it, recording what it sends. */
function withState(st: AppState) {
  const session = new MockSession({ manual: true, demo: true })
  const sent: AppCmd[] = []
  app.attach({ kind: 'mock', subscribe: (fn) => (fn(st), () => {}), send: (c) => sent.push(c), library: () => session.library(), meters: () => session.meters(), dispose: () => {} })
  flushSync()
  render(Launchkey)
  return sent
}

/** The mock's state with the held layer `layer` (the mock has no hardware to hold one). */
function holding(layer: Layer) {
  const session = new MockSession({ manual: true, demo: true })
  session.state.surface.layer = layer
  session.send({ type: 'setPadPage', page: session.state.pads.page }) // re-derive the pads
  return session
}

/** Knob 1's function in swap mode as the engine sends it (src/session/knobs.rs); types.ts's
 *  KnobFunction doesn't name it yet (a contract follow-up). */
const SWAP_SOUND = 'swapSound' as KnobFunction

const soundButton = () => document.querySelector<HTMLButtonElement>('[data-tip="launchkey.sound"]')!
const tabNames = () => [...document.querySelectorAll('[role="tab"] .pname')].map((t) => t.textContent)

describe('Launchkey mirror: page order and held layers', () => {
  it('has one tab per page in the player\'s order, and Pad Bank ▲ ▼ walk that order', async () => {
    const { session } = setup()
    session.send({ type: 'setPadPageOrder', pages: ['multiPads', 'racks'] })
    flushSync()
    expect(tabNames()).toEqual(['Sections', 'Multi Pads', 'Racks'])
    expect(document.querySelector('[role="tab"][data-tip="padpage.chord"]')).toBeNull()
    expect([...document.querySelectorAll('[role="tab"] .num')].map((n) => n.textContent)).toEqual(['1', '2', '3'])
    const down = () => document.querySelector<HTMLButtonElement>('[data-tip="padpage.next"]')!
    await fireEvent.click(down())
    expect(session.state.pads.page).toBe('multiPads')
    flushSync()
    expect(document.querySelector('.page-num b')!.textContent).toBe('2')
    expect(document.querySelector('[role="tab"][aria-selected="true"] .pname')!.textContent).toBe('Multi Pads')
    await fireEvent.click(down())
    expect(session.state.pads.page).toBe('racks')
    flushSync()
    // The last page: ▼ does nothing more.
    await fireEvent.click(document.querySelector<HTMLButtonElement>('[aria-label="Pad Bank"] .hw:nth-child(2) button')!)
    expect(session.state.pads.page).toBe('racks')
    await fireEvent.click(document.querySelector<HTMLButtonElement>('[data-tip="padpage.prev"]')!)
    expect(session.state.pads.page).toBe('multiPads')
  })

  it('names the tabs as the state does', () => {
    const session = new MockSession({ manual: true, demo: true })
    const st = structuredClone(session.state)
    st.pads.pages = [{ page: 'sections', name: 'Sections' }, { page: 'setup', name: 'My Setup' }]
    withState(st)
    expect(tabNames()).toEqual(['Sections', 'My Setup'])
  })

  it('lights Sound while it is held, with the Racks page on the pads and its tab marked', () => {
    app.attach(holding({ type: 'sound' }))
    flushSync()
    render(Launchkey)
    expect(soundButton().getAttribute('aria-pressed')).toBe('true')
    expect(soundButton().closest('.sound')!.getAttribute('data-held')).toBe('true')
    // The page on view stays Sections; the pads are the Racks page's.
    expect(document.querySelector('[role="tab"][aria-selected="true"] .pname')!.textContent).toBe('Sections')
    expect(document.querySelector('[role="tab"].held .pname')!.textContent).toBe('Racks')
    expect(pad(112).textContent).toContain('OTS 1')
    expect(document.querySelector('.pads')!.getAttribute('aria-label')).toBe('Pads: Sound held, Racks')
  })

  it('shows Sound up, and no page marked, with no layer or a swap', () => {
    for (const layer of [{ type: 'none' }, { type: 'swap', part: 0 }] as Layer[]) {
      app.attach(holding(layer))
      flushSync()
      render(Launchkey)
      expect(soundButton().getAttribute('aria-pressed'), layer.type).toBe('false')
      expect(document.querySelector('[role="tab"].held'), layer.type).toBeNull()
      expect(pad(113).textContent, layer.type).toContain('MAIN B')
      cleanup()
      app.detach()
    }
  })

  it('under the Sound hold a pad sends the action the state gives it (storeRack on a capture pad)', async () => {
    const session = holding({ type: 'sound' })
    const st = structuredClone(session.state)
    st.pads.pads.find((p) => p.note === 96)!.action = { type: 'storeRack', slot: 0 }
    const sent = withState(st)
    await fireEvent.click(pad(96))
    expect(sent).toEqual([{ type: 'storeRack', slot: 0 }])
    expect(pad(96).dataset.tip).toBe('quick.store_rack')
  })

  it('a click on Sound latches the Sound layer (setLayer) and the next click lets go', async () => {
    const { session } = setup()
    const sent = vi.spyOn(app, 'send')
    const now = vi.spyOn(performance, 'now').mockReturnValue(1000)
    const click = async () => {
      const b = soundButton()
      await fireEvent.pointerDown(b, { button: 0, pointerId: 1 })
      now.mockReturnValue(1100)
      await fireEvent.pointerUp(b, { button: 0, pointerId: 1 })
      await fireEvent.click(b, { detail: 1 })
      flushSync()
    }
    await click()
    expect(sent.mock.calls.map((c) => c[0])).toEqual([{ type: 'setLayer', layer: { type: 'sound' } }])
    expect(session.state.surface.layer).toEqual({ type: 'sound' })
    expect(soundButton().getAttribute('aria-pressed')).toBe('true')
    expect(pad(112).textContent).toContain('OTS 1')
    sent.mockClear()
    await click()
    expect(sent.mock.calls.map((c) => c[0])).toEqual([{ type: 'setLayer', layer: { type: 'none' } }])
    expect(session.state.surface.layer).toEqual({ type: 'none' })
    expect(pad(113).textContent).toContain('MAIN B')
    now.mockRestore()
    sent.mockRestore()
  })

  it('Sound held down is momentary: setLayer sound on press, none on release', async () => {
    const { session } = setup()
    const sent = vi.spyOn(app, 'send')
    const now = vi.spyOn(performance, 'now').mockReturnValue(1000)
    await fireEvent.pointerDown(soundButton(), { button: 0, pointerId: 1 })
    flushSync()
    expect(sent.mock.calls.map((c) => c[0])).toEqual([{ type: 'setLayer', layer: { type: 'sound' } }])
    expect(pad(112).textContent).toContain('OTS 1')
    now.mockReturnValue(1500)
    await fireEvent.pointerUp(soundButton(), { button: 0, pointerId: 1 })
    await fireEvent.click(soundButton(), { detail: 1 })
    expect(sent.mock.calls.map((c) => c[0])).toEqual([
      { type: 'setLayer', layer: { type: 'sound' } },
      { type: 'setLayer', layer: { type: 'none' } },
    ])
    expect(session.state.surface.layer).toEqual({ type: 'none' })
    now.mockRestore()
    sent.mockRestore()
  })

  it('Enter on Sound latches and lets go; with Shift on the Style page it mutes the Pad part', async () => {
    const { session } = setup()
    // A keyboard press (a click with no pointer) latches, the next one lets go.
    await fireEvent.click(soundButton())
    expect(session.state.surface.layer).toEqual({ type: 'sound' })
    flushSync()
    await fireEvent.click(soundButton())
    expect(session.state.surface.layer).toEqual({ type: 'none' })
    // Style page with Shift: it mutes the sixth Style part, as on the hardware.
    session.send({ type: 'toggleFaderPage' })
    ui.shiftLatched = true
    flushSync()
    const on = session.state.mixer.styleParts[5].on
    const b = document.querySelector<HTMLButtonElement>('.sound button')!
    await fireEvent.click(b)
    expect(session.state.mixer.styleParts[5].on).toBe(!on)
  })

  it('in swap mode knob 1 is the part\'s sound, and turning it steps the sound as the hardware\'s knob 1 does', async () => {
    const session = new MockSession({ manual: true, demo: true })
    const st = structuredClone(session.state)
    st.surface.layer = { type: 'swap', part: 1 }
    st.knobs.pageName = 'Swap R2'
    st.knobs.knobs[0] = { function: SWAP_SOUND, name: 'Right 2 Sound', short: 'Sound', value: '23 Rhodes Soft', level: null }
    const sent = withState(st)
    const cell = document.querySelector('[aria-label="Knobs"] .cell')!
    expect(cell.querySelector('.name')!.textContent).toBe('Sound')
    expect(cell.querySelector('.readout')!.textContent).toBe('23 Rhodes Soft')
    expect(cell.classList.contains('swapsound')).toBe(true)
    expect(knob(0).dataset.tip).toBe('part.swap')
    expect(document.querySelector('.page-name')!.textContent).toBe('Swap R2')
    knob(0).focus()
    await fireEvent.keyDown(knob(0), { key: 'ArrowUp' })
    await fireEvent.keyDown(knob(1), { key: 'ArrowUp' })
    await fireEvent.keyDown(knob(7), { key: 'ArrowDown' })
    expect(knob(1).dataset.tip).toBe('part.swap')
    // The swap knobs have no default to go back to: a reset sends nothing.
    await fireEvent.dblClick(knob(0))
    await fireEvent.dblClick(knob(1))
    expect(sent).toEqual([
      { type: 'swapSound', part: 1, step: 1 },
      { type: 'turnSwapKnob', part: 1, knob: 1, delta: 1 },
      { type: 'turnSwapKnob', part: 1, knob: 7, delta: -1 },
    ])
  })

  it('outside swap mode knob 1 is the knob page\'s', async () => {
    const session = new MockSession({ manual: true, demo: true })
    const sent = withState(structuredClone(session.state))
    await fireEvent.keyDown(knob(0), { key: 'ArrowUp' })
    expect(sent).toEqual([{ type: 'turnKnob', knob: 0, delta: 1 }])
    expect(knob(0).dataset.tip).toBe('knobs.knob')
  })

  it('every control has a catalog tooltip under each layer', () => {
    for (const layer of [{ type: 'none' }, { type: 'sound' }, { type: 'swap', part: 2 }] as Layer[]) {
      for (const shift of [false, true]) {
        const st = structuredClone(holding(layer).state)
        if (layer.type === 'swap') st.knobs.knobs[0] = { function: SWAP_SOUND, name: 'Right 3 Sound', short: 'Sound', value: '-', level: null }
        ui.shiftLatched = shift
        withState(st)
        const els = document.querySelectorAll('button, [role="slider"], [role="tab"]')
        expect(els.length).toBeGreaterThan(30)
        for (const el of els) {
          const key = el.getAttribute('data-tip')
          expect(key && key in TIPS, `${layer.type}${shift ? '+shift' : ''}: ${el.outerHTML.slice(0, 80)}`).toBe(true)
        }
        cleanup()
        app.detach()
      }
    }
  })
})
