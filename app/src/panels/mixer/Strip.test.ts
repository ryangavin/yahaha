// One mixer strip's own controls (Strip.svelte): the fader in each fader layer (VOL, PAN,
// REV, CHO, DLY), and On and Solo on a keyboard part and on a Style part. Each test checks
// the exact command sent and what the mock made of it.

import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { MockSession } from '../../lib/api/mock'
import type { FaderLayer } from '../../lib/api/types'
import { app, ui } from '../../lib/store.svelte'
import MixerRow from './MixerRow.svelte'

function setup() {
  const session = new MockSession({ manual: true, demo: true })
  app.attach(session)
  flushSync()
  render(MixerRow)
  return { session, send: vi.spyOn(app, 'send') }
}

afterEach(() => {
  cleanup()
  vi.restoreAllMocks()
  app.detach()
  ui.mixer = false
  ui.selectedPart = 0
})

const strip = (i: number) => document.querySelectorAll<HTMLElement>('.mixer-row .strip')[i]
const faderOf = (i: number) => strip(i).querySelector<HTMLElement>('.fader [role="slider"]')!
/** What the fader's readout shows: the layer's own text (pan) over the Fader's number. */
const readoutOf = (i: number) =>
  (strip(i).querySelector('.fader .fmt') ?? strip(i).querySelector('.fader .readout .glow-text'))!.textContent?.trim()
const tagOf = (i: number) => strip(i).querySelector<HTMLElement>('.tags .layer')
const onOf = (i: number) => strip(i).querySelector<HTMLButtonElement>('button.on')!
const soloOf = (i: number) => strip(i).querySelector<HTMLButtonElement>('button.solo')!

function layer(l: FaderLayer) {
  app.send({ type: 'setFaderLayer', layer: l })
  flushSync()
}

describe('Mixer strip', () => {
  it('has no mini knobs: pan and the sends are on the fader layers', () => {
    setup()
    expect(document.querySelectorAll('.mixer-row .strip .knobs, .mixer-row .strip .knob')).toHaveLength(0)
  })

  it('VOL (the default): no tag, and the fader sends the part\'s volume', async () => {
    const { session, send } = setup()
    expect(session.state.mixer.faderLayer).toBe('volume')
    expect(tagOf(1)).toBeNull()
    await fireEvent.keyDown(faderOf(1), { key: 'End' })
    expect(send).toHaveBeenLastCalledWith({ type: 'setPartVolume', part: 1, volume: 127 })
    await fireEvent.keyDown(faderOf(6), { key: 'Home' })
    expect(send).toHaveBeenLastCalledWith({ type: 'setStylePartVolume', part: 2, volume: 0 })
    flushSync()
    expect(readoutOf(1)).toBe('127')
    expect(readoutOf(6)).toBe('0')
  })

  it('PAN: a keyboard part\'s fader sends setPartPan and reads L/C/R; a Style part\'s is unused', async () => {
    const { session, send } = setup()
    layer('pan')
    expect(tagOf(2)?.textContent).toBe('PAN')
    expect(faderOf(2).getAttribute('aria-label')).toBe('Right 3 pan')
    await fireEvent.keyDown(faderOf(2), { key: 'End' })
    flushSync()
    expect(send).toHaveBeenLastCalledWith({ type: 'setPartPan', part: 2, pan: 127 })
    expect(session.state.keyboardParts[2].pan).toBe(127)
    expect(readoutOf(2)).toBe('R63')
    await fireEvent.keyDown(faderOf(2), { key: 'Home' })
    flushSync()
    expect(send).toHaveBeenLastCalledWith({ type: 'setPartPan', part: 2, pan: 0 })
    expect(readoutOf(2)).toBe('L64')
    // Double-click: back to the centre.
    await fireEvent.dblClick(faderOf(2))
    flushSync()
    expect(send).toHaveBeenLastCalledWith({ type: 'setPartPan', part: 2, pan: 64 })
    expect(readoutOf(2)).toBe('C')
    // A Style part has no pan: unused, and moving it sends nothing.
    send.mockClear()
    expect(faderOf(6).getAttribute('aria-disabled')).toBe('true')
    await fireEvent.keyDown(faderOf(6), { key: 'End' })
    expect(send).not.toHaveBeenCalled()
  })

  it.each([
    ['reverb', 'REV', 'reverb'],
    ['chorus', 'CHO', 'chorus'],
    ['delay', 'DLY', 'variation'],
  ] as const)('%s: the fader sends that send, 0–127, on a keyboard and on a Style part', async (l, tag, which) => {
    const { session, send } = setup()
    layer(l)
    expect(tagOf(0)?.textContent).toBe(tag)
    await fireEvent.keyDown(faderOf(0), { key: 'End' })
    flushSync()
    expect(send).toHaveBeenLastCalledWith({ type: 'setPartSend', part: 0, send: which, value: 127 })
    expect(session.state.keyboardParts[0][which]).toBe(127)
    expect(readoutOf(0)).toBe('127')
    await fireEvent.keyDown(faderOf(9), { key: 'Home' })
    flushSync()
    expect(send).toHaveBeenLastCalledWith({ type: 'setStylePartSend', part: 5, send: which, value: 0 })
    expect(session.state.mixer.styleParts[5][which]).toBe(0)
    expect(readoutOf(9)).toBe('0')
  })

  it('a send layer: a keyboard part\'s double-click goes to 0, a Style part\'s own send is marked and goes back to the style', async () => {
    const { session, send } = setup()
    layer('reverb')
    await fireEvent.keyDown(faderOf(3), { key: 'End' })
    await fireEvent.dblClick(faderOf(3))
    flushSync()
    expect(send).toHaveBeenLastCalledWith({ type: 'setPartSend', part: 3, send: 'reverb', value: 0 })
    expect(session.state.keyboardParts[3].reverb).toBe(0)
    expect(tagOf(8)?.classList.contains('own')).toBe(false)
    await fireEvent.keyDown(faderOf(8), { key: 'End' })
    flushSync()
    expect(session.state.mixer.styleParts[4].sendsSet).toContain('reverb')
    expect(tagOf(8)?.classList.contains('own')).toBe(true)
    await fireEvent.dblClick(faderOf(8))
    flushSync()
    expect(send).toHaveBeenLastCalledWith({ type: 'resetStylePartSends', part: 4 })
    expect(session.state.mixer.styleParts[4].sendsSet).toEqual([])
    expect(tagOf(8)?.classList.contains('own')).toBe(false)
  })

  it('On on a keyboard part sends togglePart and flips the part', async () => {
    const { session, send } = setup()
    const was = session.state.keyboardParts[1].on
    await fireEvent.click(onOf(1))
    flushSync()
    expect(send).toHaveBeenCalledWith({ type: 'togglePart', part: 1 })
    expect(session.state.keyboardParts[1].on).toBe(!was)
    expect(onOf(1).getAttribute('aria-pressed')).toBe(String(!was))
  })

  it('On on a Style part sends toggleStylePart with its Style part index', async () => {
    const { session, send } = setup()
    const was = session.state.mixer.styleParts[3].on
    await fireEvent.click(onOf(7))
    flushSync()
    expect(send).toHaveBeenCalledWith({ type: 'toggleStylePart', part: 3 })
    expect(session.state.mixer.styleParts[3].on).toBe(!was)
    expect(onOf(7).getAttribute('aria-pressed')).toBe(String(!was))
  })

  it('Solo on a keyboard part sends setPartSolo, and again clears it', async () => {
    const { session, send } = setup()
    await fireEvent.click(soloOf(3))
    flushSync()
    expect(send).toHaveBeenCalledWith({ type: 'setPartSolo', part: 3 })
    expect(session.state.mixer.partSolo).toBe(3)
    expect(soloOf(3).getAttribute('aria-pressed')).toBe('true')
    await fireEvent.click(soloOf(3))
    flushSync()
    expect(send).toHaveBeenLastCalledWith({ type: 'setPartSolo', part: null })
    expect(session.state.mixer.partSolo).toBeNull()
    expect(soloOf(3).getAttribute('aria-pressed')).toBe('false')
  })

  it('Solo on a Style part sends setStyleSolo with its Style part index, and again clears it', async () => {
    const { session, send } = setup()
    await fireEvent.click(soloOf(9))
    flushSync()
    expect(send).toHaveBeenCalledWith({ type: 'setStyleSolo', part: 5 })
    expect(session.state.mixer.styleSolo).toBe(5)
    expect(soloOf(9).getAttribute('aria-pressed')).toBe('true')
    await fireEvent.click(soloOf(9))
    flushSync()
    expect(send).toHaveBeenLastCalledWith({ type: 'setStyleSolo', part: null })
    expect(session.state.mixer.styleSolo).toBeNull()
    expect(soloOf(9).getAttribute('aria-pressed')).toBe('false')
  })
})
