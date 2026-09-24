import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { FUNCTIONS, functionCmd } from '../../lib/api/assignable'
import { MockSession } from '../../lib/api/mock'
import { app, ui } from '../../lib/store.svelte'
import { nav } from './nav.svelte'
import Settings from './Settings.svelte'

function setup() {
  const session = new MockSession({ manual: true, demo: false })
  app.attach(session)
  ui.settings = true
  nav.tab = 'pedals'
  flushSync()
  render(Settings)
  return session
}

afterEach(() => {
  cleanup()
  app.detach()
  ui.settings = false
  nav.tab = 'chord'
})

const byTip = <T extends HTMLElement = HTMLElement>(key: string) => [...document.querySelectorAll<T>(`[data-tip="${key}"]`)]

describe('Pedals page', () => {
  it('shows three pedals as the engine starts: sustain, sostenuto, soft on CC 64, 66, 67', () => {
    setup()
    const fns = byTip<HTMLSelectElement>('pedal.function').map((s) => s.value)
    expect(fns).toEqual(['sustain', 'sostenuto', 'soft'])
    expect(byTip<HTMLInputElement>('pedal.cc').map((i) => i.value)).toEqual(['64', '66', '67'])
    // Sustain has a Control Type; Hold A is chosen.
    expect(byTip('pedal.hold_a')[0].getAttribute('aria-checked')).toBe('true')
  })

  it('assigns a function, and a pitch-bend pedal gets its range', async () => {
    setup()
    const sel = byTip<HTMLSelectElement>('pedal.function')[1]
    sel.value = 'startStop'
    await fireEvent.change(sel)
    expect(app.state.controllers.pedals[1].function).toBe('startStop')
    const third = byTip<HTMLSelectElement>('pedal.function')[2]
    third.value = 'pitchBend'
    await fireEvent.change(third)
    await fireEvent.click(byTip('pedal.range_full')[0])
    expect(app.state.controllers.pedals[2]).toMatchObject({ function: 'pitchBend', range: 'full' })
  })

  it('Try runs the function: Start/Stop starts the band', async () => {
    const s = setup()
    s.send({ type: 'setPedal', pedal: 0, cc: 64, function: 'startStop', controlType: 'holdA', reverse: false, range: 'upper' })
    flushSync()
    expect(app.state.transport.running).toBe(false)
    await fireEvent.click(byTip('pedal.try')[0])
    expect(app.state.transport.running).toBe(true)
  })

  it('per part: sustain off for Left, and the bend range steps and stops at 12', async () => {
    setup()
    await fireEvent.click(byTip('pedal.part_sustain')[3])
    expect(app.state.controllers.parts[3].sustain).toBe(false)
    for (let i = 0; i < 14; i++) await fireEvent.click(byTip('pedal.bend_up')[0])
    expect(app.state.controllers.parts[0].bendRange).toBe(12)
  })

  it('Try is off for a foot-controller function (it follows the pedal)', () => {
    const s = setup()
    s.send({ type: 'setPedal', pedal: 2, cc: 4, function: 'pitchBend', controlType: 'holdA', reverse: false, range: 'upper' })
    flushSync()
    expect(byTip<HTMLButtonElement>('pedal.try')[2].disabled).toBe(true)
    expect(byTip<HTMLButtonElement>('pedal.try')[0].disabled).toBe(false)
  })

  it('Registration Bank +/− are listed but not selectable yet', () => {
    setup()
    const opt = byTip<HTMLSelectElement>('pedal.function')[0].querySelector<HTMLOptionElement>('option[value="registBankNext"]')!
    expect(opt.disabled).toBe(true)
  })
})

describe('assignable functions', () => {
  it('every trigger the engine runs as a command maps to one here (the mock)', () => {
    const control = ['otsNext', 'otsPrev', 'registBankNext', 'registBankPrev', 'none']
    for (const f of FUNCTIONS.filter((f) => f.kind === 'trigger' && !control.includes(f.id))) {
      expect(functionCmd(f.id, { fingering: 'fingered' }), f.id).not.toBeNull()
    }
  })
})
