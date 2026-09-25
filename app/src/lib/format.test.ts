import { describe, expect, it } from 'vitest'
import { formatTempo } from './format'

describe('formatTempo', () => {
  it('rounds to whole BPM, not truncating', () => {
    expect(formatTempo(105.00028874)).toBe('105')
    expect(formatTempo(134.000058)).toBe('134')
    expect(formatTempo(69.000017)).toBe('69')
    expect(formatTempo(69.99998)).toBe('70')
    expect(formatTempo(120)).toBe('120')
    expect(formatTempo(99.5)).toBe('100')
  })

  it('shows nothing for a missing tempo', () => {
    expect(formatTempo(null)).toBe('')
    expect(formatTempo(undefined)).toBe('')
    expect(formatTempo(Number.NaN)).toBe('')
    expect(formatTempo(Number.POSITIVE_INFINITY)).toBe('')
  })
})
