import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { MockSession } from '../../lib/api/mock'
import { app, ui } from '../../lib/store.svelte'
import { isTipKey } from '../../help/tooltips'
import Looper from './Looper.svelte'
import { barsOf, beatText, looperLeds, modeText } from './looper'

function setup() {
  const session = new MockSession({ manual: true, demo: true })
  app.attach(session)
  ui.looper = true
  flushSync()
  render(Looper)
  return session
}

afterEach(() => {
  cleanup()
  app.detach()
  ui.looper = false
})

const button = (tip: string) => document.querySelector<HTMLButtonElement>(`button[data-tip="${tip}"]`)!
const barMs = (s: MockSession) => (60000 / s.state.transport.tempo) * s.state.transport.beatsPerBar

describe('Chord Looper drawer', () => {
  it('records while the band plays, loops from the next bar, and stores a memory', async () => {
    const s = setup()
    expect(s.state.transport.running).toBe(true)
    await fireEvent.click(button('looper.rec'))
    expect(s.state.looper.mode).toBe('recArmed')
    // Recording starts on the next bar line.
    for (let ms = 0; s.state.looper.mode === 'recArmed' && ms < 2 * barMs(s); ms += 20) s.advance(20)
    expect(s.state.looper.mode).toBe('recording')
    s.advance(4 * barMs(s) - 200)
    await fireEvent.click(button('looper.on_off'))
    expect(s.state.looper.mode).toBe('loopArmed')
    s.advance(400)
    flushSync()
    expect(s.state.looper.mode).toBe('looping')
    expect(s.state.looper.bars).toBe(4)
    expect(s.state.looper.chords.length).toBeGreaterThan(0)
    expect(document.querySelectorAll('.bars .bar')).toHaveLength(4)

    await fireEvent.click(button('looper.store'))
    const mems = [...document.querySelectorAll<HTMLButtonElement>('button[data-tip="looper.memory"]')]
    expect(mems).toHaveLength(8)
    await fireEvent.click(mems[2])
    flushSync()
    expect(s.state.looper.memories[2].name).toBe('CLD_001')
    expect(s.state.looper.memory).toBe(2)
    expect(mems[2].textContent).toContain('CLD_001')

    await fireEvent.click(button('looper.on_off'))
    expect(s.state.looper.mode).toBe('off')
    expect(s.state.looper.hasData).toBe(true)
  })

  it('every control has a tooltip', () => {
    setup()
    const bad = [...document.querySelectorAll('button, [role="slider"], [tabindex]:not([tabindex="-1"])')].filter(
      (e) => !isTipKey(e.getAttribute('data-tip') ?? ''),
    )
    expect(bad).toEqual([])
  })
})

describe('looper view logic', () => {
  it('lights REC and ON/OFF as the Genos does', () => {
    expect(looperLeds({ mode: 'off', hasData: false })).toEqual({ rec: null, onOff: null })
    expect(looperLeds({ mode: 'recArmed', hasData: false }).rec?.anim).toBe('flash')
    expect(looperLeds({ mode: 'recording', hasData: false }).rec?.anim).toBe('solid')
    expect(looperLeds({ mode: 'loopArmed', hasData: true }).onOff?.anim).toBe('flash')
    expect(looperLeds({ mode: 'looping', hasData: true }).onOff?.anim).toBe('solid')
    expect(looperLeds({ mode: 'off', hasData: true }).onOff?.rgb[2]).toBe(127)
  })
  it('lays the sequence out in bars', () => {
    const bars = barsOf({ bars: 3, chords: [{ bar: 1, beat: 1, chord: 'C' }, { bar: 3, beat: 2.5, chord: 'G7' }] })
    expect(bars.map((b) => b.chords.length)).toEqual([1, 0, 1])
    expect(beatText(2.5)).toBe('b2½')
    expect(beatText(3)).toBe('b3')
    expect(modeText('recArmed', false)).toContain('Play a chord')
  })
})
