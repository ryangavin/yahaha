// Double-clicking a Knob Assign knob puts its function back to its default (resetKnob),
// as the mock and the session do: Dynamics to max, a part's send to dry.

import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import App from '../../App.svelte'
import { MockSession } from '../../lib/api/mock'

afterEach(cleanup)

const knob = (name: string) => document.querySelector<HTMLElement>(`.knobs [role="slider"][aria-label="${name}"]`)!

describe('knob double-click reset', () => {
  it('puts Dynamics back to max and a send back to dry', async () => {
    const m = new MockSession({ demo: true, manual: true })
    render(App, { props: { session: m } })
    flushSync()
    m.send({ type: 'turnKnob', knob: 0, delta: -10 })
    flushSync()
    expect(m.state.dynamics.level).toBeLessThan(127)
    await fireEvent.dblClick(knob('Dynamics Control'))
    flushSync()
    expect(m.state.dynamics.level).toBe(127)
    m.send({ type: 'setKnobPage', page: 'reverb' })
    m.send({ type: 'turnKnob', knob: 0, delta: 5 })
    flushSync()
    expect(m.state.keyboardParts[0].reverb).toBeGreaterThan(0)
    await fireEvent.dblClick(knob('Right 1 Reverb'))
    flushSync()
    expect(m.state.keyboardParts[0].reverb).toBe(0)
  })
})

describe('knob layout', () => {
  it('puts each knob first, then its label over its readout, with the full name as tooltip', () => {
    const m = new MockSession({ demo: true, manual: true })
    render(App, { props: { session: m } })
    flushSync()
    const cells = [...document.querySelectorAll<HTMLElement>('.knobs .cell')]
    expect(cells).toHaveLength(8)
    m.state.knobs.knobs.forEach((k, i) => {
      const [dial, text] = cells[i].children
      expect(dial.getAttribute('role')).toBe('slider')
      expect([...text.children].map((c) => [c.className.split(' ')[0], c.textContent])).toEqual([
        ['name', k.short],
        ['readout', k.value],
      ])
      expect(cells[i].title).toBe(k.name)
    })
  })

  // jsdom doesn't lay out or apply component CSS, so this reads the rules themselves.
  it('makes each knob as tall as its label + readout, and the bar that plus padding', () => {
    const css = readFileSync(resolve(process.cwd(), 'src/app.css'), 'utf8')
    const strip = readFileSync(resolve(process.cwd(), 'src/panels/launchkey/KnobStrip.svelte'), 'utf8')
    const app = readFileSync(resolve(process.cwd(), 'src/App.svelte'), 'utf8')
    // Every rule whose selector is exactly `sel`, joined in source order.
    const rule = (src: string, sel: string) => {
      const ms = [...src.matchAll(new RegExp(`\\n\\s*${sel.replace('.', '\\.')} \\{([^}]*)\\}`, 'g'))]
      expect(ms.length, `rule ${sel}`).toBeGreaterThan(0)
      return ms.map((m) => m[1]).join('\n')
    }
    const decl = (body: string, prop: string) => {
      const m = body.match(new RegExp(`(?:^|;|\\s)${prop}:\\s*([^;]+);`))
      expect(m, prop).not.toBeNull()
      return m![1].trim()
    }
    const rem = (v: string) => Number(v.replace(/rem$/, ''))
    // The stack: one engraved label line, the gap, the readout.
    const font = rem(decl(rule(css, '.engraved'), 'font-size'))
    const lh = Number(decl(rule(strip, '.name'), 'line-height'))
    const gap = rem(decl(rule(strip, '.text'), 'gap'))
    const readout = rem(decl(rule(strip, '.readout'), 'height'))
    const root = rule(css, ':root')
    const size = decl(root, '--knob-size-bar')
    const terms = size.match(/^calc\((\S+)rem \* (\S+) \+ (\S+)rem \+ (\S+)rem\)$/)!.slice(1).map(Number)
    expect(terms).toEqual([font, lh, gap, readout])
    // The knob uses it, and the bar is it plus the padding and border.
    const bar = rule(strip, '.knobs')
    expect(decl(bar, '--knob-size')).toBe('var(--knob-size-bar)')
    expect(decl(bar, 'height')).toBe('var(--knob-bar-h)')
    const padY = decl(bar, 'padding').split(/\s+/)[0]
    expect(decl(root, '--knob-bar-h')).toBe(`calc(var(--knob-size-bar) + 2 * ${padY} + 2px)`)
    // The stage leaves room for exactly that bar and its 4px margin.
    expect(decl(bar, 'margin-top')).toBe('4px')
    expect(decl(rule(app, '.stack'), '--fixed')).toBe('calc(var(--knob-bar-h) + 4px)')
  })
})

describe('mock resetKnob', () => {
  it('goes to each function\'s default', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'setKnobPage', page: 'pan' })
    m.send({ type: 'turnKnob', knob: 1, delta: 5 })
    m.send({ type: 'resetKnob', knob: 1 })
    expect(m.state.keyboardParts[1].pan).toBe(64)
    m.send({ type: 'setKnobPage', page: 'reverb' })
    const def = m.state.effects.blocks[0].params[0].default
    m.send({ type: 'turnKnob', knob: 4, delta: 4 })
    m.send({ type: 'resetKnob', knob: 4 })
    expect(m.state.effects.blocks[0].params[0].value).toBe(def)
    m.send({ type: 'setKnobPage', page: 'rack' })
    m.send({ type: 'resetKnob', knob: 2 })
    expect(m.state.keyboardParts[2].volume).toBe(100)
  })
})

describe('mock Rack knob page', () => {
  it('is the Parts page with the default map, and follows the controller map', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'setKnobPage', page: 'rack' })
    expect(m.state.knobs.pageName).toBe('Rack')
    expect(m.state.knobs.knobs.map((k) => k.short)).toEqual(['Right1', 'Right2', 'Right3', 'Left', 'HarmVol', 'MetroVol', '---', 'Tempo'])
    m.send({ type: 'setRackControl', control: 'knob', index: 0, target: { kind: 'splitPoint' } })
    expect(m.state.liveRack.modified).toBe(true)
    expect(m.state.knobs.knobs[0]).toMatchObject({ function: 'splitPoint', short: 'Split', value: 'F#2' })
    m.send({ type: 'turnKnob', knob: 0, delta: 2 })
    expect(m.state.chord.split).toBe(56)
    // A fader the map gives another target: its label and command in the mirror.
    m.send({ type: 'setRackControl', control: 'fader', index: 1, target: { kind: 'partPan', part: 0 } })
    const f = m.state.surface.faders[1]
    expect(f).toMatchObject({ label: 'PANR1', set: { type: 'moveRackFader', fader: 1, volume: 0 } })
    m.send({ type: 'moveRackFader', fader: 1, volume: 20 })
    expect(m.state.keyboardParts[0].pan).toBe(20)
    m.send({ type: 'setRackControl', control: 'fader', index: 1, target: { kind: 'tempo' } })
    expect(m.state.liveRack.controls.faders[1]).toEqual({ kind: 'partPan', part: 0 })
  })
})
