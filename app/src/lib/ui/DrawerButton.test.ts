// The drawer buttons on the stage, next to the controls they detail: each one opens its
// drawer (and closes it again), shows that it's open, and the app bar keeps only the
// app's own controls.

import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import App from '../../App.svelte'
import { MockSession } from '../api/mock'
import { ui } from '../store.svelte'

afterEach(() => {
  cleanup()
  ui.rack = ui.mixer = ui.effects = ui.multipad = ui.charts = ui.harmony = ui.looper = ui.settings = false
  ui.browser = false
  ui.view = 'stage'
  ui.libraryTab = 'sounds'
  ui.libraryPart = 0
})

function setup() {
  render(App, { props: { session: new MockSession({ demo: true, manual: true }) } })
  flushSync()
}

const btn = (sel: string) => document.querySelector<HTMLButtonElement>(sel)!

describe('drawer buttons on the stage', () => {
  const PLACES = [
    ['drawer.mixer', 'mixer', '.master'],
    ['drawer.multipad', 'multipad', '.pagebar'],
    ['drawer.charts', 'charts', 'section[aria-label="Lead sheet"]'],
    ['drawer.harmony', 'harmony', 'section[aria-label="Keyboard"]'],
    ['drawer.looper', 'looper', 'section[aria-label="Keyboard"]'],
  ] as const

  for (const [tip, drawer, place] of PLACES) {
    it(`${tip} sits in ${place} and toggles its drawer`, async () => {
      setup()
      const b = btn(`${place} .drawer-btn[data-tip="${tip}"]`)
      expect(b).toBeTruthy()
      expect(b.getAttribute('aria-pressed')).toBe('false')
      await fireEvent.click(b)
      flushSync()
      expect(ui[drawer]).toBe(true)
      expect(b.getAttribute('aria-pressed')).toBe('true')
      await fireEvent.click(b)
      flushSync()
      expect(ui[drawer]).toBe(false)
    })
  }

  const BAR = '.bar[aria-label="Mixer"]'

  it('drawer.rack sits in the mixer bar (with the details shown) and toggles its drawer', async () => {
    setup()
    expect(document.querySelector(BAR)).toBeNull()
    ui.mixer = true
    flushSync()
    const b = btn(`${BAR} .drawer-btn[data-tip="drawer.rack"]`)
    expect(b.getAttribute('aria-pressed')).toBe('false')
    await fireEvent.click(b)
    flushSync()
    expect(ui.rack).toBe(true)
    expect(b.getAttribute('aria-pressed')).toBe('true')
    await fireEvent.click(b)
    flushSync()
    expect(ui.rack).toBe(false)
  })

  it('drawer.library on the mixer bar opens Library on Sounds, loading into the selected part', async () => {
    const session = new MockSession({ demo: true, manual: true })
    render(App, { props: { session } })
    session.send({ type: 'selectPart', part: 2 })
    session.advance(16)
    ui.mixer = true
    flushSync()
    await fireEvent.click(btn(`${BAR} .drawer-btn[data-tip="drawer.library"]`))
    flushSync()
    expect(ui.view).toBe('library')
    expect(ui.libraryTab).toBe('sounds')
    expect(ui.libraryPart).toBe(2)
  })

  it('Styles in the quick nav opens the style browser (the status display is gone)', async () => {
    setup()
    await fireEvent.click(btn('[data-tip="nav.styles"]'))
    flushSync()
    expect(ui.browser).toBe(true)
  })

  it('the app bar keeps Settings, help and theme, and no drawer buttons', () => {
    setup()
    const bar = document.querySelector('header.bar')!
    expect(bar.querySelector('[data-tip="settings.open"]')).toBeTruthy()
    expect(bar.querySelector('[data-tip="app.help"]')).toBeTruthy()
    expect(bar.querySelector('[data-tip^="drawer."], [data-tip="browser.open"]')).toBeNull()
  })
})
