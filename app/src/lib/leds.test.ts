import { describe, expect, it } from 'vitest'
import { tipFor } from '../help/actions'
import { TIPS } from '../help/tooltips'
import { padsFor } from './api/mock-pads'
import { MockSession } from './api/mock'
import { PAD_PAGES } from './api/types'
import { BOTTOM, LAMP, TOP, brightness, lamp, padLocation } from './leds'

describe('pads and lamps', () => {
  const m = new MockSession({ manual: true, demo: true })

  it('every page covers the 16 pads in order', () => {
    for (const { id } of PAD_PAGES) expect(padsFor(m.state, id).map((p) => p.note)).toEqual([...TOP, ...BOTTOM])
  })

  it("each pad's tooltip names the pad it's on", () => {
    for (const { id } of PAD_PAGES) {
      for (const p of padsFor(m.state, id)) {
        if (!p.action) continue
        expect(TIPS[tipFor(p.action)].launchkey, `${id} ${p.label}`).toContain(padLocation(id, p.note))
      }
    }
  })

  it('a queued Main flashes, and so does a Main whose fill plays', () => {
    const s = new MockSession({ manual: true, demo: true })
    s.send({ type: 'main', index: 2 })
    expect(lamp(s.state, LAMP.main[2]).anim).toBe('flash')
    s.send({ type: 'main', index: 1 }) // the Main that's playing: its fill
    expect(s.state.transport.queued).toBe('Fill In BB')
    expect(lamp(s.state, LAMP.main[1]).anim).toBe('flash')
  })

  it('a section the style lacks is dark', () => {
    // The first mock style has no Main D.
    expect(lamp(m.state, LAMP.main[3]).level).toBe('off')
  })

  it('brightness follows the hardware: dim 18%, flash on the half beat, pulse 25–100%', () => {
    const l = (level: 'off' | 'dim' | 'bright', anim: 'solid' | 'flash' | 'pulse') => ({ level, anim })
    expect(brightness(l('off', 'solid'), 0)).toBe(0)
    expect(brightness(l('dim', 'flash'), 0)).toBeCloseTo(0.18)
    expect(brightness(l('bright', 'flash'), 0.25)).toBe(1)
    expect(brightness(l('bright', 'flash'), 0.75)).toBeCloseTo(0.18)
    expect(brightness(l('bright', 'pulse'), 0)).toBeCloseTo(0.25)
    expect(brightness(l('bright', 'pulse'), 1)).toBeCloseTo(1)
  })
})
