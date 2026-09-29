import { describe, expect, it } from 'vitest'
import { emptyState, GM, noteName } from './constants'
import fixture from './mock-fixture.json'
import { GM as MOCK_GM, noteName as mockNoteName } from './mock'
import { KEYBOARD_PART_NAMES, STYLE_PART_NAMES } from './types'

describe('app constants', () => {
  it('has the same GM names as the fixture the Rust dev mock reads', () => {
    expect(GM).toEqual(fixture.gm)
    expect(MOCK_GM).toBe(GM)
    expect(mockNoteName).toBe(noteName)
    expect(noteName(60)).toBe('C3')
  })

  it('starts empty: every part and control present, nothing loaded, older than any session state', () => {
    const s = emptyState()
    expect(s.version).toBe(0)
    expect(s.keyboardParts.map((p) => p.name)).toEqual(KEYBOARD_PART_NAMES)
    expect(s.mixer.styleParts.map((p) => p.name)).toEqual(STYLE_PART_NAMES)
    expect(s.surface.controls).toHaveLength(17)
    expect(s.surface.faders).toHaveLength(9)
    expect(s.transport.running).toBe(false)
    expect(emptyState()).not.toBe(s)
  })
})
