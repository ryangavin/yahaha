import { describe, expect, it } from 'vitest'
import { MockSession } from './api/mock'
import type { AppState } from './api/types'
import { app } from './store.svelte'

const at = (s: AppState, version: number, tempo: number): AppState => ({
  ...s,
  version,
  transport: { ...s.transport, tempo },
})

describe('app store', () => {
  it('applies a state only when its version is higher than the last one applied', () => {
    const session = new MockSession({ manual: true })
    app.attach(session)
    const base = app.state
    const v = base.version
    expect(app.apply(at(base, v + 2, 140))).toBe(true)
    expect(app.state.transport.tempo).toBe(140)
    // An older snapshot resolving late, and a repeat of the current one, are dropped.
    expect(app.apply(at(base, v + 1, 90))).toBe(false)
    expect(app.apply(at(base, v + 2, 91))).toBe(false)
    expect(app.state.transport.tempo).toBe(140)
    expect(app.apply(at(base, v + 3, 92))).toBe(true)
    expect(app.state.transport.tempo).toBe(92)
    app.detach()
  })

  it('starts over when a new session attaches (its versions start again)', () => {
    const a = new MockSession({ manual: true })
    app.attach(a)
    app.apply(at(app.state, 1000, 150))
    const b = new MockSession({ manual: true })
    app.attach(b)
    expect(app.state.version).toBe(b.state.version)
    b.send({ type: 'tempoUp' })
    expect(app.state.version).toBe(b.state.version)
    app.detach()
  })
})
