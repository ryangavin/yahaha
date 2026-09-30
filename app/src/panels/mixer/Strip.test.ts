// One mixer strip's own controls (Strip.svelte): Pan on a keyboard part, and On and Solo on
// a keyboard part and on a Style part. Each test checks the exact command sent and what the
// mock made of it.

import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { MockSession } from '../../lib/api/mock'
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
const knobOf = (s: HTMLElement, caption: string) =>
  [...s.querySelectorAll<HTMLElement>('.knobs .knob')].find((k) => k.querySelector('.caption')?.textContent === caption)
const onOf = (i: number) => strip(i).querySelector<HTMLButtonElement>('button.on')!
const soloOf = (i: number) => strip(i).querySelector<HTMLButtonElement>('button.solo')!

describe('Mixer strip', () => {
  it('Pan on a keyboard part sends setPartPan with the new value; a Style part has no pan', async () => {
    const { session, send } = setup()
    await fireEvent.keyDown(knobOf(strip(2), 'Pan')!, { key: 'End' })
    expect(send).toHaveBeenCalledWith({ type: 'setPartPan', part: 2, pan: 127 })
    expect(session.state.keyboardParts[2].pan).toBe(127)
    await fireEvent.keyDown(knobOf(strip(2), 'Pan')!, { key: 'Home' })
    expect(send).toHaveBeenLastCalledWith({ type: 'setPartPan', part: 2, pan: 0 })
    expect(session.state.keyboardParts[2].pan).toBe(0)
    expect(knobOf(strip(6), 'Pan')).toBeUndefined()
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
