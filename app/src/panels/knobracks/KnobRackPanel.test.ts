// The stage's knob and Quick Rack panel: knob n over rack n in shared columns, between the
// mirror and the keyboard strip. And double-clicking a Knob Assign knob puts its function
// back to its default (resetKnob), as the mock and the session do.

import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import App from '../../App.svelte'
import { MockSession } from '../../lib/api/mock'

afterEach(cleanup)

const PANEL = 'section[aria-label="Knobs and Quick Racks"]'
const knob = (name: string) => document.querySelector<HTMLElement>(`${PANEL} [role="slider"][aria-label="${name}"]`)!

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
    const cells = [...document.querySelectorAll<HTMLElement>(`${PANEL} .cell`)]
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
  it('makes both rows, and so each knob and rack button, as tall as a label + readout, in em of the stage', () => {
    const src = readFileSync(resolve(process.cwd(), 'src/panels/knobracks/KnobRackPanel.svelte'), 'utf8')
    const css = src.slice(src.indexOf('<style>'))
    const app = readFileSync(resolve(process.cwd(), 'src/App.svelte'), 'utf8')
    // Every rule whose selector is exactly `sel`, joined in source order.
    const rule = (text: string, sel: string) => {
      const esc = sel.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
      const ms = [...text.matchAll(new RegExp(`\\n\\s*${esc} \\{([^}]*)\\}`, 'g'))]
      expect(ms.length, `rule ${sel}`).toBeGreaterThan(0)
      return ms.map((m) => m[1]).join('\n')
    }
    const decl = (body: string, prop: string) => {
      const m = body.match(new RegExp(`(?:^|;|\\s)${prop}:\\s*([^;]+);`))
      expect(m, prop).not.toBeNull()
      return m![1].trim()
    }
    const em = (v: string) => Number(v.replace(/em$/, ''))
    // A row: one label line, the gap, the readout.
    const label = rule(css, '.panel :global(.engraved)')
    const font = em(decl(label, 'font-size'))
    const lh = Number(decl(label, 'line-height'))
    const gap = em(decl(rule(css, '.text'), 'gap'))
    const readout = em(decl(rule(css, '.readout'), 'height'))
    const panel = rule(css, '.panel')
    const terms = decl(panel, '--row').match(/^calc\((\S+)em \* (\S+) \+ (\S+)em \+ (\S+)em\)$/)!.slice(1).map(Number)
    expect(terms).toEqual([font, lh, gap, readout])
    expect(decl(panel, 'grid-template-rows')).toBe('var(--row) var(--row)')
    expect(decl(rule(css, '.cell'), '--knob-size')).toBe('var(--row)')
    // Everything scales with the stage: no rem sizes, and no fixed term in the stack.
    expect(css).not.toMatch(/\drem\b/)
    expect(app).not.toMatch(/--fixed|--top|--knob-bar-h/)
  })
})

describe('knobs over Quick Racks', () => {
  it('puts rack n in knob n\'s grid column, between the mirror and the keyboard strip, not in it', () => {
    render(App, { props: { session: new MockSession({ demo: true, manual: true }) } })
    flushSync()
    const panel = document.querySelector<HTMLElement>(PANEL)!
    const cells = [...panel.querySelectorAll<HTMLElement>('.cell')]
    const slots = [...panel.querySelectorAll<HTMLElement>('.slot')]
    expect(cells).toHaveLength(8)
    expect(slots).toHaveLength(8)
    // Column 1 is the pagers' cheek; knob and rack i share column i + 2.
    cells.forEach((c, i) => {
      expect(c.style.getPropertyValue('grid-column'), `knob ${i + 1}`).toBe(String(i + 2))
      expect(slots[i].style.getPropertyValue('grid-column'), `rack ${i + 1}`).toBe(String(i + 2))
      expect(slots[i].querySelector(`[data-tip="quick.${i + 1}"]`)).not.toBeNull()
    })
    // Both pagers and Store are in the panel.
    for (const key of ['knobs.page', 'quick.bank_prev', 'quick.bank', 'quick.bank_next', 'quick.store']) {
      expect(panel.querySelector(`[data-tip="${key}"]`), key).not.toBeNull()
    }
    // The keyboard strip holds no Quick Rack control, and the panel sits between it and the mirror.
    const strip = document.querySelector<HTMLElement>('section[aria-label="Keyboard"]')!
    expect(strip.querySelector('[data-tip^="quick."], [data-tip^="knobs."]')).toBeNull()
    const mirror = document.querySelector<HTMLElement>('section[aria-label="Launchkey"]')!
    expect(mirror.querySelector('[data-tip^="knobs."]')).toBeNull()
    expect(mirror.compareDocumentPosition(panel) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    expect(panel.compareDocumentPosition(strip) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })

  it('pages the knobs and steps the bank from the cheek; a waiting Store asks across the rack row', async () => {
    const m = new MockSession({ manual: true })
    render(App, { props: { session: m } })
    flushSync()
    const panel = document.querySelector<HTMLElement>(PANEL)!
    const page = m.state.knobs.pageNumber
    await fireEvent.click(panel.querySelector('[aria-label="Next knob page"]')!)
    flushSync()
    expect(m.state.knobs.pageNumber).toBe(page + 1)
    expect(panel.querySelector('.page-name')!.textContent).toBe(m.state.knobs.pageName)
    await fireEvent.click(panel.querySelector('[data-tip="quick.bank_next"]')!)
    flushSync()
    expect(panel.querySelector('.letter')!.textContent).toBe('B')
    await fireEvent.click(panel.querySelector('[data-tip="quick.store"]')!)
    await fireEvent.click(panel.querySelector('[data-tip="quick.3"]')!)
    flushSync()
    expect(panel.querySelector('.ask')!.textContent).toContain('store it on B3')
    expect(panel.querySelectorAll('.slot')).toHaveLength(0)
    expect(panel.querySelector('[data-tip="quick.store"]')).toBeNull()
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
