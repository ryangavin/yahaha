import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { AppCmd } from './api/types'
import { MAX_HOLD_MS, REPEAT_DELAY_MS, TempoHold } from './tempoHold'

describe('TempoHold', () => {
  let sent: AppCmd['type'][]
  let hold: TempoHold
  beforeEach(() => {
    vi.useFakeTimers()
    sent = []
    hold = new TempoHold((c) => sent.push(c.type))
  })
  afterEach(() => vi.useRealTimers())

  it('steps once on a press', () => {
    hold.set(1, true)
    hold.set(1, false)
    vi.advanceTimersByTime(5000)
    expect(sent).toEqual(['tempoUp'])
  })

  it('repeats while held, faster and faster, until let go', () => {
    hold.set(-1, true)
    vi.advanceTimersByTime(REPEAT_DELAY_MS - 1)
    expect(sent.length).toBe(1)
    vi.advanceTimersByTime(1)
    expect(sent.length).toBe(2)
    // 7 more at 100 ms, then 50 ms apart.
    vi.advanceTimersByTime(700)
    expect(sent.length).toBe(9)
    vi.advanceTimersByTime(500)
    expect(sent.length).toBe(19)
    hold.set(-1, false)
    vi.advanceTimersByTime(5000)
    expect(sent.length).toBe(19)
    expect(new Set(sent)).toEqual(new Set(['tempoDown']))
  })

  it('both together reset the tempo and stop repeating', () => {
    hold.set(-1, true)
    hold.set(1, true)
    vi.advanceTimersByTime(5000)
    expect(sent).toEqual(['tempoDown', 'resetTempo'])
    hold.set(1, false)
    hold.set(-1, false)
    hold.set(1, true)
    expect(sent.at(-1)).toBe('tempoUp')
    hold.releaseAll()
  })

  it('gives up on a hold that never ends', () => {
    hold.set(1, true)
    vi.advanceTimersByTime(MAX_HOLD_MS + 5000)
    const n = sent.length
    vi.advanceTimersByTime(10_000)
    expect(sent.length).toBe(n)
  })
})
