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
    m.send({ type: 'setKnobPage', page: 'parts' })
    m.send({ type: 'resetKnob', knob: 2 })
    expect(m.state.keyboardParts[2].volume).toBe(100)
  })
})
