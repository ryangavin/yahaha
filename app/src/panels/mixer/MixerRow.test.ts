import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { MockSession } from '../../lib/api/mock'
import type { Meters } from '../../lib/api/types'
import { app, ui } from '../../lib/store.svelte'
import { isTipKey } from '../../help/tooltips'
import MixerRow from './MixerRow.svelte'

function setup(meters?: () => Promise<Meters>) {
  const session = new MockSession({ manual: true, demo: true })
  if (meters) session.meters = meters
  app.attach(session)
  flushSync()
  render(MixerRow)
  return session
}

afterEach(() => {
  cleanup()
  vi.restoreAllMocks()
  app.detach()
  ui.mixer = false
  ui.selectedPart = 0
  ui.shiftLatched = false
  ui.view = 'stage'
  ui.libraryTab = 'sounds'
  ui.libraryPart = 0
})

const strips = () => [...document.querySelectorAll<HTMLElement>('.mixer-row .strip')]
const nameOf = (s: HTMLElement) => s.querySelector<HTMLButtonElement>('button.name')!
const faderOf = (s: HTMLElement) => s.querySelector<HTMLElement>('.fader [role="slider"]')!
const tick = async () => {
  await new Promise((r) => setTimeout(r, 0))
  flushSync()
}

const meters = (peak: number): Meters => ({
  atMs: 0,
  channels: Array.from({ length: 16 }, (_, i) => ({ channel: i + 1, peak, rms: 0, cpu: 0, cpuPeak: 0 })),
  master: [0, 0],
  masterRms: [0, 0],
  clips: 0,
  cpu: { total: 0, peak: 0, bufferUs: 0 },
})

describe('Mixer row', () => {
  it('shows 12 equal strips in order, keyboard parts then the style parts, each control tipped', () => {
    setup()
    expect(document.querySelector('section.mixer-row[aria-label="Mixer"]')).not.toBeNull()
    const s = strips()
    expect(s).toHaveLength(12)
    expect(s.map((e) => nameOf(e).textContent?.trim())).toEqual([
      'Right 1', 'Right 2', 'Right 3', 'Left',
      'Rhythm 1', 'Rhythm 2', 'Bass', 'Chord 1', 'Chord 2', 'Pad', 'Phrase 1', 'Phrase 2',
    ])
    const untipped = s.flatMap((e) => [...e.querySelectorAll('button, [role="slider"], [tabindex]:not([tabindex="-1"])')])
      .filter((e) => !isTipKey(e.getAttribute('data-tip') ?? ''))
    expect(untipped).toEqual([])
    expect(faderOf(s[0]).dataset.tip).toBe('mixer.panel.right1')
    expect(faderOf(s[4]).dataset.tip).toBe('mixer.style.volume')
  })

  it('a keyboard fader sends setPartVolume and a style fader setStylePartVolume', async () => {
    const s = setup()
    const k = s.state.keyboardParts[1].volume
    const st = s.state.mixer.styleParts[2].volume
    await fireEvent.keyDown(faderOf(strips()[1]), { key: 'PageDown' })
    expect(s.state.keyboardParts[1].volume).toBe(k - 10)
    await fireEvent.keyDown(faderOf(strips()[6]), { key: 'PageDown' })
    expect(s.state.mixer.styleParts[2].volume).toBe(st - 10)
  })

  it('the mixer bar shows with the details hidden, and the strips have no mini knobs', () => {
    setup()
    expect(ui.mixer).toBe(false)
    expect(document.querySelector('.mixer-row .bar')).not.toBeNull()
    expect(document.querySelectorAll('.mixer-row .col .detail')).toHaveLength(0)
    expect(document.querySelectorAll('.mixer-row .strip .knob')).toHaveLength(0)
  })

  it('a fader layer changes every strip\'s fader: keyboard and Style parts send that layer\'s command', async () => {
    const s = setup()
    const send = vi.spyOn(app, 'send')
    app.send({ type: 'setFaderLayer', layer: 'chorus' })
    flushSync()
    await fireEvent.keyDown(faderOf(strips()[1]), { key: 'PageUp' })
    expect(send).toHaveBeenLastCalledWith({ type: 'setPartSend', part: 1, send: 'chorus', value: s.state.keyboardParts[1].chorus })
    await fireEvent.keyDown(faderOf(strips()[11]), { key: 'PageDown' })
    expect(send).toHaveBeenLastCalledWith({ type: 'setStylePartSend', part: 7, send: 'chorus', value: s.state.mixer.styleParts[7].chorus })
    expect(strips().map((e) => e.querySelector('.tags .layer')?.textContent)).toEqual(Array(12).fill('CHO'))
    expect(faderOf(strips()[11]).dataset.tip).toBe('mixer.style.chorus')
    expect(faderOf(strips()[1]).dataset.tip).toBe('mixer.part.chorus')
  })

  it('fader badges: F1–F4 on the keyboard parts on the Panel page, F1–F8 on the Style parts on the Style page, M on the master', () => {
    const s = setup()
    const badges = () => strips().map((e) => e.querySelector<HTMLElement>('.tags .badge')?.textContent ?? null)
    const master = () => document.querySelector('.mixer-row .master .badge')?.textContent ?? null
    expect(s.state.mixer.faderPage).toBe('panel')
    expect(badges()).toEqual(['F1', 'F2', 'F3', 'F4', null, null, null, null, null, null, null, null])
    expect(master()).toBe('M')
    for (const b of document.querySelectorAll<HTMLElement>('.mixer-row .badge')) expect(b.dataset.tip).toBe('stage.fader_badge')
    app.send({ type: 'toggleFaderPage' })
    flushSync()
    expect(badges()).toEqual([null, null, null, null, 'F1', 'F2', 'F3', 'F4', 'F5', 'F6', 'F7', 'F8'])
    expect(master()).toBe('M')
    // Shift changes the buttons under the faders, not which part a fader moves.
    ui.shiftLatched = true
    flushSync()
    expect(badges()).toEqual([null, null, null, null, 'F1', 'F2', 'F3', 'F4', 'F5', 'F6', 'F7', 'F8'])
    // PAN on the Style page: the Style parts have no pan, so no fader reaches them.
    app.send({ type: 'setFaderLayer', layer: 'pan' })
    flushSync()
    expect(badges()).toEqual(Array(12).fill(null))
    app.send({ type: 'setFaderPage', page: 'panel' })
    flushSync()
    expect(badges()).toEqual(['F1', 'F2', 'F3', 'F4', null, null, null, null, null, null, null, null])
  })

  it('clicking a strip name selects it; a keyboard part also sends selectPart, a style part not', async () => {
    setup()
    const send = vi.spyOn(app, 'send')
    await fireEvent.click(nameOf(strips()[2]))
    flushSync()
    expect(ui.selectedPart).toBe(2)
    expect(send).toHaveBeenCalledWith({ type: 'selectPart', part: 2 })
    expect(nameOf(strips()[2]).getAttribute('aria-pressed')).toBe('true')
    expect(strips()[2].classList.contains('selected')).toBe(true)
    send.mockClear()
    await fireEvent.click(nameOf(strips()[9]))
    flushSync()
    expect(ui.selectedPart).toBe(9)
    expect(strips()[9].classList.contains('selected')).toBe(true)
    expect(strips()[2].classList.contains('selected')).toBe(false)
    expect(send).not.toHaveBeenCalledWith(expect.objectContaining({ type: 'selectPart' }))
  })

  it('a keyboard part\'s voice opens Library on Sounds loading into that part; a style part\'s is text', async () => {
    setup()
    ui.libraryTab = 'racks'
    const voice = strips()[3].querySelector<HTMLButtonElement>('button.voice')!
    expect(voice.title).not.toBe('')
    await fireEvent.click(voice)
    expect(ui.view).toBe('library')
    expect(ui.libraryTab).toBe('sounds')
    expect(ui.libraryPart).toBe(3)
    expect(strips()[4].querySelector('button.voice')).toBeNull()
    expect(strips()[4].querySelector<HTMLElement>('.voice')!.dataset.tip).toBe('mixer.strip.voice')
  })

  it('the meter shows the channel\'s level when the meters have channels, and a dim bar without', async () => {
    setup(() => Promise.resolve(meters(0.5)))
    await tick()
    const m = strips()[0].querySelector<HTMLElement>('[data-testid="meter"]')!
    expect(m.classList.contains('idle')).toBe(false)
    const scale = Number(/scaleY\(([^)]+)\)/.exec(m.querySelector<HTMLElement>('.fill')!.style.transform)![1])
    expect(scale).toBeGreaterThan(0.8)
    cleanup()
    app.detach()
    setup(() => Promise.resolve({ ...meters(0.5), channels: [] }))
    await tick()
    const idle = strips()[0].querySelector<HTMLElement>('[data-testid="meter"]')!
    expect(idle.classList.contains('idle')).toBe(true)
    expect(idle.querySelector<HTMLElement>('.fill')!.style.transform).toBe('scaleY(0)')
  })
})
