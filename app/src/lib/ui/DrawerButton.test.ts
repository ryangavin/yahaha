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
  ui.parts = ui.mixer = ui.sound = ui.multipad = ui.charts = ui.harmony = ui.looper = ui.settings = false
  ui.browser = false
})

function setup() {
  render(App, { props: { session: new MockSession({ demo: true, manual: true }) } })
  flushSync()
}

const btn = (sel: string) => document.querySelector<HTMLButtonElement>(sel)!

describe('drawer buttons on the stage', () => {
  const PLACES = [
    ['drawer.parts', 'parts', '.faders'],
    ['drawer.sound', 'sound', '.faders'],
    ['drawer.mixer', 'mixer', '.faders'],
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

  it('the style name on the display opens the style browser', async () => {
    setup()
    await fireEvent.click(btn('[aria-label="Status display"] [data-tip="browser.open"]'))
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
