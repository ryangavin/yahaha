import { describe, expect, it } from 'vitest'
import { initialState } from '../../lib/api/mock'
import { RANGES, boundary, detectionArea, heldFill, isBlack, layout, noteAt, noteName, rangeFor } from './keyboard'

describe('keyboard strip geometry', () => {
  it('lays out 49, 61 and 88 keys with the right whites and blacks', () => {
    for (const [n, whites] of [[49, 29], [61, 36], [88, 52]] as const) {
      const keys = layout(RANGES[n])
      expect(keys).toHaveLength(n)
      expect(keys.filter((k) => !k.black)).toHaveLength(whites)
    }
    expect(noteName(RANGES[49][0])).toBe('C1')
    expect(noteName(RANGES[88][0])).toBe('A-1')
    expect(noteName(60)).toBe('C3')
  })

  it('whites tile the strip; blacks sit over the gap between two whites', () => {
    const keys = layout(RANGES[61])
    const whites = keys.filter((k) => !k.black)
    expect(whites[0].x).toBe(0)
    expect(whites.at(-1)!.x + whites.at(-1)!.w).toBeCloseTo(1)
    const cs = keys.find((k) => k.note === 37)!
    const c = keys.find((k) => k.note === 36)!
    expect(cs.x + cs.w / 2).toBeCloseTo(c.x + c.w)
  })

  it('finds the key under a point, black keys first', () => {
    const keys = layout(RANGES[49])
    for (const k of keys) expect(noteAt(keys, k.x + k.w / 2)).toBe(k.note)
    expect(isBlack(54)).toBe(true)
  })

  it('draws a split after a black key through its middle, between two whites on their edge', () => {
    const keys = layout(RANGES[49])
    const fs = keys.find((k) => k.note === 54)!
    expect(boundary(keys, 54)).toBeCloseTo(fs.x + fs.w / 2)
    expect(boundary(keys, 52)).toBeCloseTo(keys.find((k) => k.note === 53)!.x)
    expect(boundary(keys, 10)).toBe(0)
    expect(boundary(keys, 100)).toBe(1)
  })

  it('matches the connected Launchkey unless a size is chosen', () => {
    expect(rangeFor(null, ['Launchkey 49 MK4 LKMK4 MIDI Out'])).toBe(49)
    expect(rangeFor(null, ['Launchkey 61 MK4 LKMK4 MIDI Out'])).toBe(61)
    expect(rangeFor(null, ['Launchkey Mini 37 MK4'])).toBe(49)
    expect(rangeFor(null, [])).toBe(61)
    expect(rangeFor(88, ['Launchkey 49 MK4'])).toBe(88)
  })

  it('puts the chord-detection area where the engine listens', () => {
    const s = initialState()
    expect(detectionArea(s, RANGES[49])).toEqual([36, 54])
    s.chord.upper = true
    expect(detectionArea(s, RANGES[49])).toEqual([55, 84])
    s.chord.upper = false
    s.chord.fingering = 'fullKeyboard'
    expect(detectionArea(s, RANGES[49])).toEqual([36, 84])
  })

  it('colours a held key by its parts: bands when layered, grey when it only feeds detection', () => {
    expect(heldFill({ note: 60, zone: 'right', parts: [0] })).toBe('var(--part-r1)')
    expect(heldFill({ note: 60, zone: 'right', parts: [0, 1] })).toContain('var(--part-r2) 50% 100%')
    expect(heldFill({ note: 40, zone: 'left', parts: [] })).toBe('var(--part-chord)')
  })
})
