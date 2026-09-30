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
    expect(labels(m)).toEqual(['', 'PAGE ▼', '◀ STYLE', 'STYLE ▶', 'PLAY', 'STOP', 'TEMPO +', 'TEMPO -', 'RIGHT 1', 'RIGHT 2', 'RIGHT 3', 'LEFT', 'HARM/ARP', 'SOUND', 'L HOLD', 'LOOPER', 'PANEL'])
    expect(m.state.surface.controls.map((c) => c.shiftLabel).slice(0, 2)).toEqual(['LEFT', 'OTS LINK'])
    expect(m.state.surface.controls[8].shiftLabel).toBe('EDIT R1')
    expect(m.state.surface.faders).toHaveLength(9)
    expect(m.state.surface.faders.map((f) => f.label)).toEqual(['RIGHT 1', 'RIGHT 2', 'RIGHT 3', 'LEFT', 'STYLE', 'M.PAD', '', '', 'MASTER'])
    m.send({ type: 'toggleFaderPage' })
    expect(labels(m).slice(8)).toEqual(['RHYTHM 1', 'RHYTHM 2', 'BASS', 'CHORD 1', 'CHORD 2', 'SOUND', 'PHRASE 1', 'PHRASE 2', 'STYLE'])
  })

  it('each fader layer lights the Panel part buttons and master button in its colour (src/launchkey.rs layer_colour)', () => {
    const m = new MockSession({ manual: true, demo: true })
    const b = (id: string) => m.state.surface.controls.find((x) => x.id === id)!
    const want = { volume: [0, 0, 127], pan: [127, 127, 0], reverb: [0, 100, 127], chorus: [127, 0, 70], delay: [127, 127, 127] } as const
    for (const [layer, rgb] of Object.entries(want)) {
      m.send({ type: 'setFaderLayer', layer: layer as keyof typeof want })
      expect([layer, b('masterButton').rgb, b('masterButton').level, b('faderButton1').rgb]).toEqual([layer, rgb, 'bright', rgb])
      expect(b('faderButton5').rgb).toEqual([90, 0, 127])
    }
    m.send({ type: 'togglePart', part: 0 })
    expect([b('faderButton1').level, b('faderButton1').rgb]).toEqual(['dim', [127, 127, 127]])
    m.send({ type: 'toggleFaderPage' })
    expect(b('masterButton').rgb).toEqual([0, 127, 0])
  })

  it('Panel button 5 is the HARMONY/ARPEGGIO switch: dim purple off, bright on', () => {
    const m = new MockSession({ manual: true, demo: true })
    const b5 = () => m.state.surface.controls.find((x) => x.id === 'faderButton5')!
    expect([b5().action, b5().level, b5().rgb]).toEqual([{ type: 'toggleHarmonyArp' }, 'dim', [90, 0, 127]])
    m.send({ type: 'toggleHarmonyArp' })
    expect(b5().level).toBe('bright')
  })

  it('button 6 is Sound on both fader pages: a hold, no action; Shift on the Style page mutes the Pad part', () => {
    const m = new MockSession({ manual: true, demo: true })
    const b6 = () => m.state.surface.controls.find((x) => x.id === 'faderButton6')!
    expect(m.state.surface.layer).toEqual({ type: 'none' })
    expect([b6().label, b6().action, b6().level, b6().rgb]).toEqual(['SOUND', null, 'dim', [127, 127, 127]])
    expect([b6().shiftLabel, b6().shiftAction]).toEqual(['', null])
    m.send({ type: 'toggleFaderPage' })
    expect([b6().label, b6().action, b6().shiftLabel, b6().shiftAction]).toEqual(['SOUND', null, 'PAD', { type: 'toggleStylePart', part: 5 }])
  })

  it('plugin reload is an app command only: it reloads the selected part\'s plugin', () => {
    const m = new MockSession({ manual: true, demo: true })
    m.send({ type: 'setPartPlugin', part: 0, id: 'aumu Mock Demo', state: null })
    m.advance(1000)
    m.send({ type: 'reloadPartPlugin', part: null })
    expect(m.state.keyboardParts[0].plugin!.status).toBe('loading')
    m.advance(1000)
    expect(m.state.keyboardParts[0].plugin!.status).toBe('failed')
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
      'padBankUp', 'padBankDown', 'trackPrev', 'trackNext', 'play', 'stop', 'scene', 'function',
      // Button 6: Sound is a plain (Shift off) hold; Shift + it does nothing on the Panel page.
      'faderButton1', 'faderButton2', 'faderButton3', 'faderButton4', 'faderButton6', 'faderButton8', 'masterButton',
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
