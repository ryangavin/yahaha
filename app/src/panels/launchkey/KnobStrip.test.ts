// Double-clicking a Knob Assign knob puts its function back to its default (resetKnob),
// as the mock and the session do: Dynamics to max, a part's send to dry.

import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import App from '../../App.svelte'
import { MockSession } from '../../lib/api/mock'

afterEach(cleanup)

const knob = (name: string) => document.querySelector<HTMLElement>(`.knobs [role="slider"][aria-label="${name}"]`)!

describe('knob double-click reset', () => {
  it('puts Dynamics back to max and a send back to dry', async () => {
    const m = new MockSession({ demo: true, manual: true })
    render(App, { props: { session: m } })
    flushSync()
    m.send({ type: 'turnKnob', knob: 0, delta: -10 })
    flushSync()
    expect(m.state.dynamics.level).toBeLessThan(127)
    await fireEvent.dblClick(knob('Dynamics Control'))
    flushSync()
    expect(m.state.dynamics.level).toBe(127)
    m.send({ type: 'setKnobPage', page: 'reverb' })
    m.send({ type: 'turnKnob', knob: 0, delta: 5 })
    flushSync()
    expect(m.state.keyboardParts[0].reverb).toBeGreaterThan(0)
    await fireEvent.dblClick(knob('Right 1 Reverb'))
    flushSync()
    expect(m.state.keyboardParts[0].reverb).toBe(0)
  })
})

describe('knob layout', () => {
  it('puts each knob first, then its label over its readout, with the full name as tooltip', () => {
    const m = new MockSession({ demo: true, manual: true })
    render(App, { props: { session: m } })
    flushSync()
    const cells = [...document.querySelectorAll<HTMLElement>('.knobs .cell')]
    expect(cells).toHaveLength(8)
    m.state.knobs.knobs.forEach((k, i) => {
      const [dial, text] = cells[i].children
      expect(dial.getAttribute('role')).toBe('slider')
      expect([...text.children].map((c) => [c.className.split(' ')[0], c.textContent])).toEqual([
        ['name', k.short],
        ['readout', k.value],
      ])
      expect(cells[i].title).toBe(k.name)
    })
  })
})

describe('mock resetKnob', () => {
  it('goes to each function\'s default', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'setKnobPage', page: 'pan' })
    m.send({ type: 'turnKnob', knob: 1, delta: 5 })
    m.send({ type: 'resetKnob', knob: 1 })
    expect(m.state.keyboardParts[1].pan).toBe(64)
    m.send({ type: 'setKnobPage', page: 'reverb' })
    const def = m.state.effects.blocks[0].params[0].default
    m.send({ type: 'turnKnob', knob: 4, delta: 4 })
    m.send({ type: 'resetKnob', knob: 4 })
    expect(m.state.effects.blocks[0].params[0].value).toBe(def)
    m.send({ type: 'setKnobPage', page: 'rack' })
    m.send({ type: 'resetKnob', knob: 2 })
    expect(m.state.keyboardParts[2].volume).toBe(100)
  })
})

describe('mock Rack knob page', () => {
  it('is the Parts page with the default map, and follows the controller map', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'setKnobPage', page: 'rack' })
    expect(m.state.knobs.pageName).toBe('Rack')
    expect(m.state.knobs.knobs.map((k) => k.short)).toEqual(['Right1', 'Right2', 'Right3', 'Left', 'HarmVol', 'MetroVol', '---', 'Tempo'])
    m.send({ type: 'setRackControl', control: 'knob', index: 0, target: { kind: 'splitPoint' } })
    expect(m.state.liveRack.modified).toBe(true)
    expect(m.state.knobs.knobs[0]).toMatchObject({ function: 'splitPoint', short: 'Split', value: 'F#2' })
    m.send({ type: 'turnKnob', knob: 0, delta: 2 })
    expect(m.state.chord.split).toBe(56)
    // A fader the map gives another target: its label and command in the mirror.
    m.send({ type: 'setRackControl', control: 'fader', index: 1, target: { kind: 'partPan', part: 0 } })
    const f = m.state.surface.faders[1]
    expect(f).toMatchObject({ label: 'PANR1', set: { type: 'moveRackFader', fader: 1, volume: 0 } })
    m.send({ type: 'moveRackFader', fader: 1, volume: 20 })
    expect(m.state.keyboardParts[0].pan).toBe(20)
    m.send({ type: 'setRackControl', control: 'fader', index: 1, target: { kind: 'tempo' } })
    expect(m.state.liveRack.controls.faders[1]).toEqual({ kind: 'partPan', part: 0 })
  })
})
