import { afterEach, describe, expect, it, vi } from 'vitest'
import { emptyState } from './api/constants'
import { MockSession } from './api/mock'
import type { AppState } from './api/types'
import { app, clock, clockNeeded, share } from './store.svelte'

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

  it('keeps the previous objects for every part of a new state that did not change', () => {
    const session = new MockSession({ manual: true })
    app.attach(session)
    const before = app.state
    // A fresh parse, as every push is: equal values, all new objects.
    const next = at(JSON.parse(JSON.stringify(before)) as AppState, before.version + 1, before.transport.tempo + 1)
    expect(app.apply(next)).toBe(true)
    expect(app.state).not.toBe(before)
    expect(app.state.transport).not.toBe(before.transport)
    expect(app.state.transport.tempo).toBe(before.transport.tempo + 1)
    expect(app.state.transport.lamps).toBe(before.transport.lamps)
    expect(app.state.mixer).toBe(before.mixer)
    expect(app.state.pads.pads).toBe(before.pads.pads)
    expect(app.state.surface.controls).toBe(before.surface.controls)
    app.detach()
  })
})

describe('share', () => {
  it('returns the previous value when the new one is deep-equal', () => {
    const prev = { a: 1, b: [1, { c: 'x' }], d: null }
    expect(share(prev, { a: 1, b: [1, { c: 'x' }], d: null })).toBe(prev)
    const nums = [1, 2]
    expect(share(nums, [1, 2])).toBe(nums)
    const arr = [{ x: 1 }]
    expect(share(arr, [{ x: 1 }])).toBe(arr)
  })

  it('shares the unchanged parts of a changed value, without touching either input', () => {
    const prev = { same: { deep: [1, 2, { k: true }] }, changed: { v: 1, keep: { z: 0 } }, list: [{ id: 1 }, { id: 2 }] }
    const next = { same: { deep: [1, 2, { k: true }] }, changed: { v: 2, keep: { z: 0 } }, list: [{ id: 1 }, { id: 3 }] }
    const nextCopy = JSON.parse(JSON.stringify(next))
    const prevCopy = JSON.parse(JSON.stringify(prev))
    const out = share(prev, next)
    expect(out).toEqual(next)
    expect(out).not.toBe(prev)
    expect(out.same).toBe(prev.same)
    expect(out.changed).not.toBe(prev.changed)
    expect(out.changed.keep).toBe(prev.changed.keep)
    expect(out.list).not.toBe(prev.list)
    expect(out.list[0]).toBe(prev.list[0])
    expect(out.list[1]).toBe(next.list[1])
    // Neither input changed.
    expect(next).toEqual(nextCopy)
    expect(next.same).not.toBe(prev.same)
    expect(prev).toEqual(prevCopy)
  })

  it('returns the new value as is when nothing in it can be shared', () => {
    const next = { a: 2, b: [3] }
    expect(share({ a: 1, b: [2] }, next)).toBe(next)
    expect(share(null, next)).toBe(next)
    expect(share([1], next)).toBe(next)
  })

  it('treats added, removed and retyped keys and lengths as changes', () => {
    const prev = { a: 1, b: { c: 1 }, l: [1, 2] }
    const added = share(prev, { a: 1, b: { c: 1 }, l: [1, 2], e: 0 })
    expect(added).not.toBe(prev)
    expect(added.b).toBe(prev.b)
    expect(share(prev, { a: 1, b: { c: 1 } })).toEqual({ a: 1, b: { c: 1 } })
    expect(share(prev, { a: 1, b: { c: 1 }, l: [1] }).l).toEqual([1])
    expect(share(prev, { a: 1, b: [1], l: [1, 2] }).b).toEqual([1])
    expect(share({ a: null }, { a: 0 })).toEqual({ a: 0 })
  })
})

describe('beat clock', () => {
  let visibility: DocumentVisibilityState = 'visible'
  const setVisibility = (v: DocumentVisibilityState) => {
    visibility = v
    document.dispatchEvent(new Event('visibilitychange'))
  }
  const withClock = (running: boolean): AppState => {
    const s = emptyState()
    s.version = 1
    s.surface.clock = { ...s.surface.clock, running, tempo: 120 }
    s.transport.running = running
    return s
  }

  afterEach(() => {
    clock.stop()
    vi.unstubAllGlobals()
  })

  it('runs its frame loop only while playing or a lamp moves, and never while the page is hidden', () => {
    const frames = new Map<number, FrameRequestCallback>()
    let id = 0
    vi.stubGlobal('requestAnimationFrame', (f: FrameRequestCallback) => (frames.set(++id, f), id))
    vi.stubGlobal('cancelAnimationFrame', (n: number) => frames.delete(n))
    Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => visibility })

    clock.sync(withClock(false))
    clock.start()
    expect(clock.looping).toBe(false)

    clock.sync(withClock(true))
    expect(clock.looping).toBe(true)

    setVisibility('hidden')
    expect(clock.looping).toBe(false)
    expect(frames.size).toBe(0)
    setVisibility('visible')
    expect(clock.looping).toBe(true)

    // Stopped, but a section lamp flashes (Sync Start armed).
    const armed = withClock(false)
    armed.transport.lamps = [{ note: 99, label: '', key: '', rgb: [127, 0, 0], level: 'bright', anim: 'flash', action: null, palette: null }]
    clock.sync(armed)
    expect(clock.looping).toBe(true)

    clock.sync(withClock(false))
    expect(clock.looping).toBe(false)
    expect(clock.pos).toBe(0)
    expect(frames.size).toBe(0)

    clock.sync(withClock(true))
    clock.stop()
    expect(clock.looping).toBe(false)
    expect(frames.size).toBe(0)
  })

  it('is needed for the lamps the app draws, and not for an idle state', () => {
    expect(clockNeeded(emptyState())).toBe(false)
    const s = emptyState()
    s.quickRacks = { ...s.quickRacks, store: true }
    expect(clockNeeded(s)).toBe(true)
    const pad = emptyState()
    pad.multiPad.pads[2].lamp = 'queued'
    expect(clockNeeded(pad)).toBe(true)
    const off = emptyState()
    off.pads.pads = [{ note: 96, label: '', key: '', rgb: [0, 0, 0], level: 'off', anim: 'pulse', action: null, palette: null }]
    expect(clockNeeded(off)).toBe(false)
  })
})
