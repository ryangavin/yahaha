import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { MockSession } from '../../lib/api/mock'
import { binding } from '../../lib/keys'
import { app, ui } from '../../lib/store.svelte'
import MultiPad from './MultiPad.svelte'
import { byFolder, lampLight } from './multipad'

function setup(demo = false) {
  const session = new MockSession({ manual: true, demo })
  app.attach(session)
  ui.multipad = true
  flushSync()
  render(MultiPad)
  return session
}

afterEach(() => {
  cleanup()
  app.detach()
  ui.multipad = false
})

const pads = () => [...document.querySelectorAll<HTMLButtonElement>('[data-tip="multipad.pad"]')]
const lamps = () => pads().map((p) => p.dataset.lamp)
const bank = (name: string) => [...document.querySelectorAll<HTMLButtonElement>('[data-tip="multipad.bank"]')].find((b) => b.textContent?.includes(name))!

describe('Multi Pad drawer', () => {
  it('lists the banks by folder; loading one names and lights its pads', async () => {
    setup()
    expect(lamps()).toEqual(['empty', 'empty', 'empty', 'empty'])
    expect([...document.querySelectorAll('.folder')].map((f) => f.textContent)).toEqual(['Pads', 'Pads/Latin'])
    await fireEvent.click(bank('Latin Perc'))
    flushSync()
    expect(lamps()).toEqual(['ready', 'ready', 'ready', 'empty'])
    expect(pads()[0].textContent).toContain('Conga Loop')
    expect(bank('Latin Perc').getAttribute('aria-selected')).toBe('true')
  })

  it('a pad plays at once when stopped, waits for the bar while the band plays, and STOP stops it', async () => {
    const s = setup()
    await fireEvent.click(bank('Demo'))
    await fireEvent.click(pads()[0])
    flushSync()
    expect(lamps()[0]).toBe('playing')
    s.send({ type: 'startStop' })
    await fireEvent.click(pads()[2])
    flushSync()
    expect(lamps()[2]).toBe('queued')
    s.advance(60000 / s.state.transport.tempo * s.state.transport.beatsPerBar + 50)
    flushSync()
    expect(lamps()[2]).toBe('playing')
    await fireEvent.click(document.querySelector<HTMLButtonElement>('[data-tip="multipad.stop_all"]')!)
    flushSync()
    expect(lamps()).toEqual(['ready', 'ready', 'ready', 'ready'])
  })

  it('Sync arms a pad; the switches send Repeat, Chord Match and Synchro Stop', async () => {
    const s = setup()
    s.send({ type: 'loadMultiPad', id: 0 })
    flushSync()
    await fireEvent.click(document.querySelectorAll<HTMLButtonElement>('[data-tip="multipad.arm"]')[1])
    flushSync()
    expect(lamps()[1]).toBe('armed')
    await fireEvent.click(document.querySelectorAll<HTMLButtonElement>('[data-tip="multipad.repeat"]')[1])
    await fireEvent.click(document.querySelector<HTMLButtonElement>('[data-tip="multipad.synchro_ending"]')!)
    flushSync()
    expect(s.state.multiPad.pads[1].repeat).toBe(true)
    expect(s.state.multiPad.synchroStop).toEqual({ styleStop: true, ending: true })
  })

  it('Z X C V and B are the pad keys', () => {
    const key = (k: string) => binding({ key: k, code: '', shiftKey: true, ctrlKey: false, altKey: false, metaKey: false })
    expect(key('Z')).toEqual({ cmd: { type: 'triggerMultiPad', pad: 0 } })
    expect(key('V')).toEqual({ cmd: { type: 'triggerMultiPad', pad: 3 } })
    expect(key('B')).toEqual({ cmd: { type: 'stopAllMultiPads' } })
  })

  it('lamp colours: dark, dim blue, red, flashing', () => {
    expect(lampLight('empty', 0).b).toBe(0)
    expect(lampLight('playing', 0.7).b).toBe(1)
    expect(lampLight('armed', 0.2).b).toBeGreaterThan(lampLight('armed', 0.7).b)
    expect(byFolder([{ id: 0, name: 'a', folder: 'x', path: '' }, { id: 1, name: 'b', folder: 'x', path: '' }])).toHaveLength(1)
  })
})
