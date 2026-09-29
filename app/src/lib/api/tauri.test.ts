// TauriSession's state fetching: one `state` fetch in flight at a time, one more after it
// when changes came meanwhile, and never an older snapshot after a newer one.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { TauriSession } from './tauri'
import type { AppState, SessionEvent } from './types'

const calls: { resolve: (st: AppState) => void; reject: (e: unknown) => void }[] = []
let onEvent: ((e: { payload: SessionEvent }) => void) | null = null

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn((cmd: string) => {
    if (cmd !== 'state') return Promise.resolve(null)
    return new Promise((resolve, reject) => calls.push({ resolve, reject }))
  }),
}))
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (_name: string, f: (e: { payload: SessionEvent }) => void) => {
    onEvent = f
    return () => {}
  }),
}))


let frames: (() => void)[] = []
const frame = () => {
  const fs = frames
  frames = []
  for (const f of fs) f()
}
const flush = () => new Promise((r) => setTimeout(r, 0))
const state = (version: number) => ({ version }) as AppState
const changed = (version: number) => onEvent!({ payload: { type: 'stateChanged', version } })

async function connected() {
  const p = TauriSession.connect()
  await flush()
  calls.shift()!.resolve(state(1))
  const s = await p
  const seen: number[] = []
  s.subscribe((st) => seen.push(st.version))
  return { s, seen }
}

describe('TauriSession', () => {
  beforeEach(() => {
    calls.length = 0
    frames = []
    vi.stubGlobal('requestAnimationFrame', (f: () => void) => frames.push(f))
  })
  afterEach(() => vi.unstubAllGlobals())

  it('keeps one state fetch in flight and fetches once more after it', async () => {
    const { seen } = await connected()
    changed(2)
    changed(3) // same frame: one fetch
    frame()
    expect(calls.length).toBe(1)
    changed(4)
    changed(5) // while it is in flight: no second fetch yet, nor a frame
    frame()
    expect(calls.length).toBe(1)
    calls.shift()!.resolve(state(3))
    await flush()
    expect(seen).toEqual([1, 3])
    frame() // the one refetch
    expect(calls.length).toBe(1)
    calls.shift()!.resolve(state(5))
    await flush()
    frame()
    expect(calls.length).toBe(0)
    expect(seen).toEqual([1, 3, 5])
  })

  it('never emits an older snapshot, and a failed fetch does not wedge it', async () => {
    const { seen } = await connected()
    changed(2)
    frame()
    calls.shift()!.reject(new Error('gone'))
    await flush()
    changed(3)
    frame()
    calls.shift()!.resolve(state(1)) // older than the last one out
    await flush()
    expect(seen).toEqual([1])
    changed(4)
    frame()
    calls.shift()!.resolve(state(4))
    await flush()
    expect(seen).toEqual([1, 4])
  })
})
