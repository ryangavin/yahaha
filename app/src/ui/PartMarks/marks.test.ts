import { describe, expect, it } from 'vitest'
import { marksText, visibleMarks } from './marks'

describe('visibleMarks', () => {
  it('draws each mark alone', () => {
    expect(visibleMarks({ edited: true })).toEqual(['edited'])
    expect(visibleMarks({ missing: true })).toEqual(['missing'])
    expect(visibleMarks({ failed: true })).toEqual(['failed'])
    expect(visibleMarks({ off: true })).toEqual(['off'])
    expect(visibleMarks({ bass: true })).toEqual(['bass'])
  })

  it('lets missing win over failed, and bass over off', () => {
    expect(visibleMarks({ missing: true, failed: true })).toEqual(['missing'])
    expect(visibleMarks({ off: true, bass: true })).toEqual(['bass'])
  })

  it('draws in order', () => {
    expect(visibleMarks({ edited: true, failed: true, off: true })).toEqual(['edited', 'failed', 'off'])
  })

  it('draws nothing with no facts', () => {
    expect(visibleMarks({})).toEqual([])
  })
})

describe('marksText', () => {
  it('speaks the marks in Stage order', () => {
    expect(marksText({ edited: true })).toBe(', edited')
    expect(marksText({ missing: true, failed: true, off: true })).toBe(', off, plugin missing')
    expect(marksText({ failed: true })).toBe(', plugin failed')
    expect(marksText({ off: true, bass: true })).toBe(', bass')
    expect(marksText({})).toBe('')
  })
})
