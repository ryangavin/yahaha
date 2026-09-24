import { describe, expect, it } from 'vitest'
import { MockSession, noteName, transposeChord } from './mock'

/** Milliseconds per bar at the mock's current tempo. */
const bar = (m: MockSession) => (60000 / m.state.transport.tempo) * m.state.transport.beatsPerBar

describe('mock session', () => {
  it('starts stopped with Sync Start armed; Start/Stop starts it on Main A', () => {
    const m = new MockSession({ manual: true })
    expect(m.state.transport.running).toBe(false)
    expect(m.state.transport.syncStart).toBe(true)
    m.send({ type: 'startStop' })
    expect(m.state.transport.running).toBe(true)
    expect(m.state.transport.section).toBe('Main A')
  })

  it('a queued Main takes over at the next bar', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'startStop' })
    m.send({ type: 'main', index: 2 })
    expect(m.state.transport.queued).toBe('Main C')
    m.advance(bar(m) * 0.5)
    expect(m.state.transport.section).toBe('Main A')
    m.advance(bar(m) * 0.6)
    expect(m.state.transport.section).toBe('Main C')
    expect(m.state.transport.queued).toBe(null)
    expect(m.state.transport.bar).toBe(1)
  })

  it('pressing the Main that plays queues its fill, which plays from the next beat', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'startStop' })
    m.advance(bar(m) * 0.1)
    m.send({ type: 'main', index: 0 })
    expect(m.state.transport.queued).toBe('Fill In AA')
    m.advance((60000 / m.state.transport.tempo) * 1.0)
    expect(m.state.transport.section).toBe('Fill In AA')
  })

  it('an armed Intro plays first, then the Main', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'intro', index: 0 })
    expect(m.state.transport.pendingIntro).toBe(0)
    m.send({ type: 'startStop' })
    expect(m.state.transport.section).toBe('Intro A')
    m.advance(bar(m) * 2.1)
    expect(m.state.transport.section).toBe('Main A')
  })

  it('an Ending stops the band', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'startStop' })
    m.send({ type: 'ending', index: 0 })
    m.advance(bar(m) * 3.2)
    expect(m.state.transport.running).toBe(false)
  })

  it('switching fader page makes every level on the new page wait for its fader', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'toggleFaderPage' })
    expect(m.state.mixer.styleParts.every((p) => p.waiting)).toBe(true)
    m.send({ type: 'setStylePartVolume', part: 0, volume: 90 })
    expect(m.state.mixer.styleParts[0].waiting).toBe(false)
  })

  it('Upper turns Manual Bass on: Left plays the bass and the style Bass is muted', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'toggleUpper' })
    expect(m.state.chord.manualBassActive).toBe(true)
    expect(m.state.keyboardParts[3].sounding).toBe(true)
    expect(m.state.mixer.styleParts[2].mutedByManualBass).toBe(true)
  })

  it('publishes a fresh snapshot with a new version', () => {
    const m = new MockSession({ manual: true })
    const seen: { version: number }[] = []
    m.subscribe((s) => seen.push(s))
    m.advance(16)
    expect(seen).toHaveLength(2)
    expect(seen[0]).not.toBe(seen[1])
    expect(seen[1].version).toBeGreaterThan(seen[0].version)
  })

  it('names notes Yamaha-style and transposes chords', () => {
    expect(noteName(60)).toBe('C3')
    expect(noteName(54)).toBe('F#2')
    expect(transposeChord('Am7/G', 2)).toBe('Bm7/A')
    expect(transposeChord('C', -1)).toBe('B')
  })
})
