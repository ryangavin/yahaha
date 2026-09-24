// state.surface (#77): the mock sends what the engine sends, the beat clock runs on the
// engine's anchors, and palette-mode pads light from `pad.palette`.

import { afterEach, describe, expect, it, vi } from 'vitest'
import { MockSession } from './api/mock'
import type { Pad } from './api/types'
import { DIM, padLight } from './leds'
import { app, clock } from './store.svelte'
import { hasShiftFunction, surfaceOf } from './surface'

afterEach(() => {
  app.detach()
  vi.useRealTimers()
})

const labels = (m: MockSession) => m.state.surface.controls.map((c) => c.label)

describe('the mock surface matches the engine (src/session.rs surface)', () => {
  it('17 controls and 9 faders with the engine labels, on both fader pages', () => {
    const m = new MockSession({ manual: true, demo: true })
    expect(surfaceOf(m.state)).toBe(m.state.surface)
    expect(m.state.surface.controls.map((c) => c.id)).toEqual([
      'padBankUp', 'padBankDown', 'trackPrev', 'trackNext', 'play', 'stop', 'scene', 'function',
      'faderButton1', 'faderButton2', 'faderButton3', 'faderButton4', 'faderButton5', 'faderButton6', 'faderButton7', 'faderButton8', 'masterButton',
    ])
    expect(labels(m)).toEqual(['', 'PAGE ▼', '◀ STYLE', 'STYLE ▶', 'PLAY', 'STOP', 'TEMPO +', 'TEMPO -', 'RIGHT 1', 'RIGHT 2', 'RIGHT 3', 'LEFT', '', '', '', '', 'PANEL'])
    expect(m.state.surface.controls.map((c) => c.shiftLabel).slice(0, 2)).toEqual(['LEFT', 'OTS LINK'])
    expect(m.state.surface.controls[8].shiftLabel).toBe('EDIT R1')
    expect(m.state.surface.faders).toHaveLength(9)
    expect(m.state.surface.faders.map((f) => f.label)).toEqual(['RIGHT 1', 'RIGHT 2', 'RIGHT 3', 'LEFT', '', '', '', '', 'MASTER'])
    m.send({ type: 'toggleFaderPage' })
    expect(labels(m).slice(8)).toEqual(['RHYTHM 1', 'RHYTHM 2', 'BASS', 'CHORD 1', 'CHORD 2', 'PAD', 'PHRASE 1', 'PHRASE 2', 'STYLE'])
  })

  it('Play, Stop, Scene and Function are not driven: off, no colour', () => {
    const m = new MockSession({ manual: true, demo: true })
    for (const id of ['play', 'stop', 'scene', 'function']) {
      const c = m.state.surface.controls.find((x) => x.id === id)!
      expect([c.level, c.colour]).toEqual(['off', null])
    }
    expect(m.state.surface.controls.find((x) => x.id === 'padBankDown')!.colour).toBe(3)
  })

  it('reports where the hardware faders are, on the surface and the parts', () => {
    const m = new MockSession({ manual: true, demo: true })
    expect(m.state.surface.faders[1].position).toBe(72)
    expect(m.state.keyboardParts[1].fader).toBe(72)
    expect(m.state.mixer.styleParts[1].fader).toBe(72)
  })
})

describe('Shift layer, as the engine JSON arrives', () => {
  it('only controls whose Shift function differs count as shifted (compared by value)', () => {
    const m = new MockSession({ manual: true, demo: true })
    // Over IPC, `action` and `shiftAction` are separate objects even when equal.
    const wire = JSON.parse(JSON.stringify(m.state.surface)) as typeof m.state.surface
    expect(wire.controls.filter(hasShiftFunction).map((c) => c.id)).toEqual([
      'padBankUp', 'padBankDown', 'play', 'stop', 'scene', 'function', 'faderButton1', 'faderButton2', 'faderButton3', 'faderButton4',
    ])
  })
})

describe('beat clock on the engine anchors', () => {
  it('runs the section position and the LED clock on from the last state', () => {
    vi.useFakeTimers()
    const m = new MockSession({ manual: true, demo: true })
    app.attach(m)
    const c = m.state.surface.clock
    const at = c.running ? c.sectionAnchorBeats + ((c.atMs - c.sectionAnchorMs) * c.tempo) / 60000 : 0
    expect(clock.pos).toBeCloseTo(at, 3)
    const led = clock.beats
    const beatMs = 60000 / c.tempo
    vi.advanceTimersByTime(beatMs / 2)
    clock.tick()
    expect(clock.pos).toBeCloseTo(at + 0.5, 1)
    expect(clock.beats).toBeCloseTo(led + 0.5, 1)
  })

  it('the mock anchors match its position: bar and beat from the clock are the transport’s', () => {
    const m = new MockSession({ manual: true, demo: true })
    m.advance(1234)
    const c = m.state.surface.clock
    expect([c.bar, c.beat]).toEqual([m.state.transport.bar, m.state.transport.beat])
  })

  it('stopped: the section position is 0 but the LED clock runs (an armed lamp breathes)', () => {
    const m = new MockSession({ manual: true, demo: false })
    m.advance(1000)
    const c = m.state.surface.clock
    expect(c.running).toBe(false)
    expect(c.ledAnchorBeats + ((c.atMs - c.ledAnchorMs) * c.tempo) / 60000).toBeGreaterThan(1)
  })
})

describe('palette-mode pads', () => {
  const pad: Pad = {
    note: 96, label: 'X', key: '', rgb: [127, 0, 0], level: 'bright', anim: 'flash', action: null,
    palette: { mode: 'flash', colour: 5, rgb: [127, 0, 0], level: 'bright', flashColour: 21, flashRgb: [0, 127, 0], flashLevel: 'dim' },
  }
  it('RGB mode ignores palette', () => {
    expect(padLight(pad, false, 0.75)).toEqual({ rgb: [127, 0, 0], b: DIM })
  })
  it('a palette flash alternates between its two colours', () => {
    expect(padLight(pad, true, 0.25)).toEqual({ rgb: [127, 0, 0], b: 1 })
    expect(padLight(pad, true, 0.75)).toEqual({ rgb: [0, 127, 0], b: DIM })
  })
})
