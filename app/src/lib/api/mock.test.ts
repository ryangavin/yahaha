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

  it('Tap while the band plays resets the section by default (the Genos default)', () => {
    const m = new MockSession({ manual: true })
    expect(m.state.styleSettings.sectionReset).toBe(true)
    m.send({ type: 'startStop' })
    m.advance(bar(m) * 1.3)
    expect(m.state.transport.bar).toBe(2)
    const tempo = m.state.transport.tempo
    m.send({ type: 'tapTempo' })
    m.advance(400)
    m.send({ type: 'tapTempo' })
    expect(m.state.transport.tempo).toBe(tempo)
    expect(m.state.transport.bar).toBe(1)
    expect(m.state.transport.running).toBe(true)
  })

  it('Tap while the band plays sets the tempo with Section Reset off; Style Section Reset is a function of its own (#128)', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'setSectionReset', on: false })
    m.send({ type: 'startStop' })
    m.advance(bar(m) * 1.3)
    expect(m.state.transport.bar).toBe(2)
    m.send({ type: 'tapTempo' })
    m.advance(400)
    m.send({ type: 'tapTempo' })
    expect(m.state.transport.tempo).toBeCloseTo(150, 1)
    expect(m.state.transport.bar).toBe(2)
    m.advance(bar(m) * 0.1)
    expect(m.state.transport.beat).not.toBe(1)
    m.send({ type: 'triggerFunction', function: 'sectionReset' })
    expect(m.state.transport.beat).toBe(1)
    expect(m.state.transport.running).toBe(true)
  })

  it('a bar of taps while stopped starts the band a beat after the last tap (#195)', () => {
    const m = new MockSession({ manual: true })
    const beats = m.state.transport.beatsPerBar
    for (let i = 0; i < beats; i++) {
      if (i > 0) m.advance(500)
      m.send({ type: 'tapTempo' })
    }
    expect(m.state.transport.tempo).toBe(120)
    m.advance(480)
    expect(m.state.transport.running).toBe(false)
    m.advance(40)
    expect(m.state.transport.running).toBe(true)
    // STOP during the count-in calls it off.
    m.send({ type: 'startStop' })
    m.advance(20000)
    for (let i = 0; i < beats; i++) {
      if (i > 0) m.advance(500)
      m.send({ type: 'tapTempo' })
    }
    m.send({ type: 'stop' })
    m.advance(2000)
    expect(m.state.transport.running).toBe(false)
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

  it('Registration stores Keyboard Harmony/Arpeggio, as the harmonyArp registrable', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'setArpPattern', index: 4 })
    m.send({ type: 'setHarmonyArpOn', on: true })
    m.send({ type: 'setHarmonyVolume', volume: 60 })
    const want = structuredClone(m.state.harmonyArp)
    m.send({ type: 'memorizeRegist', index: 5 })
    const scramble = () => {
      m.send({ type: 'setHarmonyType', index: 1 })
      m.send({ type: 'setHarmonyArpOn', on: false })
      m.send({ type: 'setHarmonyVolume', volume: 100 })
    }
    scramble()
    m.send({ type: 'setArpPedalHold', on: true })
    m.send({ type: 'recallRegist', index: 5 })
    // The pedal's Arpeggio Hold is not recalled.
    expect(m.state.harmonyArp).toEqual({ ...want, arp: { ...want.arp, pedalHold: true } })
    scramble()
    const scrambled = structuredClone(m.state.harmonyArp)
    m.send({ type: 'setFreezeGroup', group: 'harmonyArp', on: true })
    m.send({ type: 'setFreeze', on: true })
    m.send({ type: 'recallRegist', index: 5 })
    expect(m.state.harmonyArp).toEqual(scrambled)
  })

  it('Parameter Lock keeps a locked group through a registration recall', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'setSplit', note: 60 })
    m.send({ type: 'setFingering', fingering: 'fingered' })
    m.send({ type: 'memorizeRegist', index: 4 })
    m.send({ type: 'setSplit', note: 50 })
    m.send({ type: 'setFingering', fingering: 'singleFinger' })
    m.send({ type: 'setParamLock', item: 'splitPoint', on: true })
    expect(m.state.paramLocks).toEqual({ splitPoint: true, fingeringType: false })
    m.send({ type: 'recallRegist', index: 4 })
    expect(m.state.chord.split).toBe(50)
    expect(m.state.chord.fingering).toBe('fingered')
  })

  it('Kbd Harmony/Arpeggio and Arpeggio Hold are control-side switches, as the engine keeps them', () => {
    const m = new MockSession({ manual: true })
    // Try: a press switches them; no pedal switch field is written.
    m.send({ type: 'triggerFunction', function: 'kbdHarmonyArp' })
    expect(m.state.harmonyArp.on).toBe(true)
    expect('kbdHarmonyArp' in m.state.controllers).toBe(false)
    m.send({ type: 'triggerFunction', function: 'arpHold' })
    expect(m.state.harmonyArp.arp.pedalHold).toBe(true)
    expect(m.state.harmonyArp.arp.hold).toBe(false)
    m.send({ type: 'triggerFunction', function: 'arpHold' })
    m.send({ type: 'triggerFunction', function: 'kbdHarmonyArp' })
    // A Hold B pedal picked with the pedal up turns the switch on; given another function
    // it lets go.
    const set = (fn: string, controlType: 'holdA' | 'holdB' | 'toggle') =>
      m.send({ type: 'setPedal', pedal: 1, cc: 66, function: fn, controlType, reverse: false, range: 'upper' })
    set('arpHold', 'holdB')
    expect(m.state.harmonyArp.arp.pedalHold).toBe(true)
    expect(m.state.harmonyArp.arp.hold).toBe(false)
    set('sostenuto', 'holdA')
    expect(m.state.harmonyArp.arp.pedalHold).toBe(false)
    set('kbdHarmonyArp', 'holdA')
    expect(m.state.harmonyArp.on).toBe(false)
    set('kbdHarmonyArp', 'holdB')
    expect(m.state.harmonyArp.on).toBe(true)
    // PANIC lets go of what a Hold pedal was keeping on (Hold B, up); the setting stays.
    set('arpHold', 'holdB')
    m.send({ type: 'setArpHold', on: true })
    m.send({ type: 'panic' })
    expect(m.state.harmonyArp.arp.pedalHold).toBe(false)
    expect(m.state.harmonyArp.arp.hold).toBe(true)
  })

  it('names notes Yamaha-style and transposes chords', () => {
    expect(noteName(60)).toBe('C3')
    expect(noteName(54)).toBe('F#2')
    expect(transposeChord('Am7/G', 2)).toBe('Bm7/A')
    expect(transposeChord('C', -1)).toBe('B')
  })
})
