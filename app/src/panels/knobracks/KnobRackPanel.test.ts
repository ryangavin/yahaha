// The stage's Quick Racks row: the bank pager, eight rack buttons in eight columns and
// Store, between the mixer row and the keyboard strip, one row tall. Every control here
// sends its command (checked on the session's send, not only on the state it leaves).

import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it, vi } from 'vitest'
import App from '../../App.svelte'
import { MockSession } from '../../lib/api/mock'
import type { AppCmd } from '../../lib/api/types'
import { ui } from '../../lib/store.svelte'

afterEach(() => {
  cleanup()
  ui.rack = false
})

const PANEL = 'section[aria-label="Quick Racks"]'

/** The stage on a mock session, with every command it sends recorded. */
function stage(opts: { demo?: boolean } = {}) {
  const m = new MockSession({ ...opts, manual: true })
  const sent: AppCmd[] = []
  const send = m.send.bind(m)
  vi.spyOn(m, 'send').mockImplementation((cmd: AppCmd) => {
    sent.push(cmd)
    send(cmd)
  })
  render(App, { props: { session: m } })
  flushSync()
  const panel = document.querySelector<HTMLElement>(PANEL)!
  const at = <T extends Element = HTMLButtonElement>(sel: string) => panel.querySelector<T>(sel)!
  const click = async (sel: string) => {
    const el = at(sel)
    expect(el, sel).not.toBeNull()
    await fireEvent.click(el)
    flushSync()
  }
  /** The commands sent since the last call. */
  const took = () => sent.splice(0)
  return { m, panel, at, click, took }
}

describe('Quick Racks row layout', () => {
  it('puts rack n in column n + 1, the pager on the left cheek and Store on the right, between the mixer row and the keyboard strip', () => {
    const { panel } = stage({ demo: true })
    const slots = [...panel.querySelectorAll<HTMLElement>('.slot')]
    expect(slots).toHaveLength(8)
    slots.forEach((s, i) => {
      expect(s.style.getPropertyValue('grid-column'), `rack ${i + 1}`).toBe(String(i + 2))
      expect(s.querySelector(`[data-tip="quick.${i + 1}"]`)).not.toBeNull()
    })
    for (const key of ['quick.bank_prev', 'quick.bank', 'quick.bank_next', 'quick.store']) {
      expect(panel.querySelector(`[data-tip="${key}"]`), key).not.toBeNull()
    }
    const mixer = document.querySelector<HTMLElement>('section[aria-label="Mixer"]')!
    const strip = document.querySelector<HTMLElement>('section[aria-label="Keyboard"]')!
    expect(mixer.compareDocumentPosition(panel) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    expect(panel.compareDocumentPosition(strip) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    expect(strip.querySelector('[data-tip^="quick."]')).toBeNull()
  })

  it('holds no knobs: no dial, no knob page, no knob tooltip', () => {
    const { panel } = stage({ demo: true })
    expect(panel.querySelector('[role="slider"]')).toBeNull()
    expect(panel.querySelector('[data-tip^="knobs."]')).toBeNull()
    expect(panel.querySelector('.cell')).toBeNull()
    expect(panel.textContent).not.toMatch(/Knobs/)
  })

  // jsdom doesn't lay out or apply component CSS, so this reads the rules themselves.
  it('is one row, as tall as a label + readout, in em of the stage', () => {
    const src = readFileSync(resolve(process.cwd(), 'src/panels/knobracks/KnobRackPanel.svelte'), 'utf8')
    const css = src.slice(src.indexOf('<style>'))
    // Every rule whose selector is exactly `sel`, joined in source order.
    const rule = (sel: string) => {
      const esc = sel.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
      const ms = [...css.matchAll(new RegExp(`\\n\\s*${esc} \\{([^}]*)\\}`, 'g'))]
      expect(ms.length, `rule ${sel}`).toBeGreaterThan(0)
      return ms.map((m) => m[1]).join('\n')
    }
    const decl = (body: string, prop: string) => {
      const m = body.match(new RegExp(`(?:^|;|\\s)${prop}:\\s*([^;]+);`))
      expect(m, prop).not.toBeNull()
      return m![1].trim()
    }
    const em = (v: string) => Number(v.replace(/em$/, ''))
    const label = rule('.panel :global(.engraved)')
    const font = em(decl(label, 'font-size'))
    const lh = Number(decl(label, 'line-height'))
    const gap = em(decl(rule('.pager'), 'gap'))
    const readout = em(decl(rule('.readout'), 'height'))
    const panel = rule('.panel')
    const terms = decl(panel, '--row').match(/^calc\((\S+)em \* (\S+) \+ (\S+)em \+ (\S+)em\)$/)!.slice(1).map(Number)
    expect(terms).toEqual([font, lh, gap, readout])
    expect(decl(panel, 'grid-template-rows')).toBe('var(--row)')
    for (const sel of ['.pager', '.slot', '.store', '.ask']) expect(decl(rule(sel), 'grid-row'), sel).toBe('1')
    // The panel's height: the row plus its padding above and below, under 4em.
    const pad = em(decl(panel, 'padding').split(' ')[0])
    expect(font * lh + gap + readout + 2 * pad).toBeCloseTo(3.99, 2)
    // Everything scales with the stage: no rem sizes.
    expect(css).not.toMatch(/\drem\b/)
    // No two-row fallback for tall windows.
    expect(css).not.toMatch(/@container/)
  })
})

describe('Quick Racks row commands', () => {
  it('each rack button sends pressQuickRack for its slot', async () => {
    const { click, took } = stage()
    for (let i = 0; i < 8; i++) {
      await click(`[data-tip="quick.${i + 1}"]`)
      expect(took()).toEqual([{ type: 'pressQuickRack', slot: i }])
    }
  })

  it('◀ and ▶ step the bank and the letter follows', async () => {
    const { m, at, click, took } = stage()
    expect(at('.letter').textContent).toBe('A')
    await click('[data-tip="quick.bank_next"]')
    expect(took()).toEqual([{ type: 'stepQuickRackBank', delta: 1 }])
    expect(m.state.quickRacks.bank).toBe(1)
    expect(at('.letter').textContent).toBe('B')
    // Rack labels follow the bank.
    expect(at('[data-col="2"]').title).toMatch(/^B3:/)
    await click('[data-tip="quick.bank_prev"]')
    expect(took()).toEqual([{ type: 'stepQuickRackBank', delta: -1 }])
    expect(at('.letter').textContent).toBe('A')
  })

  it('Store arms (every button flashing), a press asks across the row, Cancel disarms', async () => {
    const { m, panel, at, click, took } = stage()
    await click('[data-tip="quick.store"]')
    expect(took()).toEqual([{ type: 'toggleQuickRackStore' }])
    expect(m.state.quickRacks.store).toBe(true)
    expect(at('[data-tip="quick.store"]').getAttribute('aria-pressed')).toBe('true')
    await click('[data-tip="quick.3"]')
    expect(took()).toEqual([{ type: 'pressQuickRack', slot: 2 }])
    expect(m.state.quickRacks.storeWaiting).toBe(2)
    // The prompt takes the buttons' place, Store included (Cancel is in it).
    expect(at('.ask').textContent).toContain('store it on A3')
    expect(panel.querySelectorAll('.slot')).toHaveLength(0)
    expect(panel.querySelector('[data-tip="quick.store"]')).toBeNull()
    expect(panel.querySelector('.seam.right')).toBeNull()
    await click('[data-tip="quick.cancel_store"]')
    expect(took()).toEqual([{ type: 'toggleQuickRackStore' }])
    expect(m.state.quickRacks).toMatchObject({ store: false, storeWaiting: null })
    expect(panel.querySelectorAll('.slot')).toHaveLength(8)
  })

  it('Save in the prompt stores the rack on the button; ✕ clears it', async () => {
    const { m, panel, at, click, took } = stage()
    await click('[data-tip="quick.store"]')
    await click('[data-tip="quick.2"]')
    await fireEvent.input(at<HTMLInputElement>('[data-tip="quick.save_name"]'), { target: { value: 'Ballad' } })
    took()
    await click('[data-tip="quick.save"]')
    expect(took()).toEqual([{ type: 'saveRackAs', name: 'Ballad' }])
    expect(m.state.quickRacks.buttons[1]).toMatchObject({ name: 'Ballad', loaded: true })
    expect(at('[data-col="1"] .bname').textContent).toBe('Ballad')
    // Only a stored button has ✕.
    expect(panel.querySelectorAll('.clear')).toHaveLength(1)
    await click('[data-col="1"] .clear')
    expect(took()).toEqual([{ type: 'clearQuickRack', bank: 0, slot: 1 }])
    expect(m.state.quickRacks.buttons[1].rack).toBeNull()
    expect(panel.querySelectorAll('.clear')).toHaveLength(0)
  })
})
