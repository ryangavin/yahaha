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

  it('a long press on a keyboard part\'s On holds swap for it (setLayer), latched until On is clicked again', async () => {
    const { session, send } = setup()
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] })
    try {
      const on = session.state.keyboardParts[1].on
      const press = async (ms: number) => {
        await fireEvent.pointerDown(onOf(1), { button: 0, pointerId: 1 })
        vi.advanceTimersByTime(ms)
        await fireEvent.pointerUp(onOf(1), { button: 0, pointerId: 1 })
        await fireEvent.click(onOf(1), { detail: 1 })
        flushSync()
      }
      await press(400)
      expect(send.mock.calls.map((c) => c[0])).toEqual([{ type: 'setLayer', layer: { type: 'swap', part: 1 } }])
      expect(session.state.surface.layer).toEqual({ type: 'swap', part: 1 })
      expect(session.state.keyboardParts[1].on).toBe(on)
      expect(onOf(1).dataset.swap).toBe('true')
      send.mockClear()
      // A click on the lit On lets go, without toggling the part.
      await press(50)
      expect(send.mock.calls.map((c) => c[0])).toEqual([{ type: 'setLayer', layer: { type: 'none' } }])
      expect(session.state.surface.layer).toEqual({ type: 'none' })
      expect(session.state.keyboardParts[1].on).toBe(on)
      expect(onOf(1).dataset.swap).toBeUndefined()
      send.mockClear()
      // A plain click toggles the part, as before.
      await press(50)
      expect(send.mock.calls.map((c) => c[0])).toEqual([{ type: 'togglePart', part: 1 }])
      expect(session.state.keyboardParts[1].on).toBe(!on)
    } finally {
      vi.useRealTimers()
    }
  })

  it('a Style part\'s On has no swap hold', async () => {
    const { send } = setup()
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] })
    try {
      await fireEvent.pointerDown(onOf(6), { button: 0, pointerId: 1 })
      vi.advanceTimersByTime(400)
      await fireEvent.pointerUp(onOf(6), { button: 0, pointerId: 1 })
      await fireEvent.click(onOf(6), { detail: 1 })
      expect(send.mock.calls.map((c) => c[0])).toEqual([{ type: 'toggleStylePart', part: 2 }])
    } finally {
      vi.useRealTimers()
    }
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

  it('a keyboard part\'s sound shows ⚠ when its plugin is missing and ● when it is edited, as the fader bank did', () => {
    const { session } = setup()
    const voiceOf = (i: number) => strip(i).querySelector<HTMLButtonElement>('button.voice')!
    const marks = (i: number) => [...voiceOf(i).querySelectorAll<HTMLElement>('.mark')].map((m) => [m.dataset.mark, m.textContent])
    expect(marks(0)).toEqual([])
    expect(marks(2)).toEqual([])
    expect(voiceOf(0).dataset.tip).toBe('mixer.strip.voice')
    // Right 3 plays a plugin preset, then its window turns a knob: the sound is edited.
    session.send({ type: 'listPluginPresets', id: 'au:aumu Smp7 Fake' })
    session.send({ type: 'setPartPluginPreset', part: 2, id: 'aumu Smp7 Fake', preset: 'f:1' })
    session.advance(5000)
    flushSync()
    expect(marks(2)).toEqual([])
    session.pluginWindow(2, 1)
    session.missingPlugin(0)
    flushSync()
    expect(session.state.keyboardParts[2].soundEdited).toBe(true)
    expect(marks(0)).toEqual([['missing', '⚠']])
    expect(voiceOf(0).classList.contains('missing')).toBe(true)
    expect(voiceOf(0).getAttribute('aria-label')).toMatch(/^Right 1 sound: ⚠ /)
    expect(marks(2)).toEqual([['edited', '●']])
    expect(voiceOf(2).classList.contains('edited')).toBe(true)
    expect(voiceOf(2).getAttribute('aria-label')).toMatch(/ ●$/)
    for (const i of [0, 2]) expect(voiceOf(i).dataset.tip).toBe('launchkey.fader_sound')
    expect(marks(1)).toEqual([])
  })
})
