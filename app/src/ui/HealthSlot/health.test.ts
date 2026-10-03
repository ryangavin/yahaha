import { describe, expect, it } from 'vitest'
import { firstFailedPart, health, suggestBuffer } from './health'

describe('health', () => {
  it('picks the rows in order', () => {
    expect(health({ failedPart: 2, synthOn: false, dropouts: 5, bufferFrames: 128, cpu: 0.9 })).toEqual({
      text: 'R3 failed',
      hue: 'ending',
      target: { page: 'channel', part: 2 },
      label: 'Audio health: R3 failed',
    })
    expect(health({ synthOn: false, dropouts: 5 })).toMatchObject({
      text: 'Audio off',
      hue: 'm',
      target: { page: 'settings', tab: 'system' },
    })
    expect(health({ dropouts: 3, bufferFrames: 128, cpu: 0.9 }).text).toBe('3 dropouts · buffer 256?')
    expect(health({ dropouts: 4, bufferFrames: 512 }).text).toBe('4 dropouts · buffer 1024?')
    expect(health({ dropouts: 3, bufferFrames: 1024 }).text).toBe('3 dropouts')
    expect(health({ dropouts: 3, bufferFrames: null }).text).toBe('3 dropouts')
    expect(health({ dropouts: 1 })).toMatchObject({ text: '1 dropout', hue: 'ending' })
  })

  it('tests the CPU threshold on the raw value and rounds the text', () => {
    expect(health({ cpu: 0.695 }).text).toBe('Audio')
    expect(health({ cpu: 0.7 })).toMatchObject({ text: 'CPU 70%', label: 'Audio health: CPU 70%' })
    expect(health({ cpu: 0.744 }).text).toBe('CPU 74%')
    expect(health({ cpu: null })).toMatchObject({ text: 'Audio', label: 'Audio health: fine', target: null })
  })

  it('defaults every field', () => {
    expect(health({})).toEqual({ text: 'Audio', hue: 'm', target: null, label: 'Audio health: fine' })
  })
})

describe('suggestBuffer', () => {
  it('doubles up to 1024', () => {
    expect([64, 128, 256, 512, 1024].map(suggestBuffer)).toEqual([128, 256, 512, 1024, 1024])
  })
})

describe('firstFailedPart', () => {
  it('skips missing plugins', () => {
    expect(
      firstFailedPart([undefined, { status: 'playing' }, { status: 'failed', missing: true }, { status: 'failed' }]),
    ).toBe(3)
    expect(firstFailedPart([undefined, { status: 'playing' }, undefined, undefined])).toBeNull()
  })
})
