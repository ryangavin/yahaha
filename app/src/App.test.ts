// The stage shell (App.svelte): five full-width rows, the hand surface's slot with its
// tip, and the mixer's details (ui.mixer) taking the display's place.

import { cleanup, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import App from './App.svelte'
import { MockSession } from './lib/api/mock'
import { ui } from './lib/store.svelte'

afterEach(() => {
  cleanup()
  ui.mixer = false
})

function setup() {
  const session = new MockSession({ demo: true, manual: true })
  render(App, { props: { session } })
  session.advance(16)
  flushSync()
  return session
}

describe('the stage', () => {
  it('renders its five rows in order: display, hand surface, mixer row, Quick Racks, keys', () => {
    setup()
    const rows = [...document.querySelector('main.stage > .stack')!.children]
    expect(rows).toHaveLength(5)
    expect(rows[0].classList.contains('display-slot')).toBe(true)
    expect(rows[1].classList.contains('hand-slot')).toBe(true)
    expect(rows[1].querySelector('section[aria-label="Launchkey"]')).toBeTruthy()
    expect(rows[2].classList.contains('mixer-slot')).toBe(true)
    expect(rows[3].querySelector('[data-tip="quick.store"]')).toBeTruthy()
    expect(rows[4].classList.contains('strip-slot')).toBe(true)
  })

  it('the hand surface slot carries its tip', () => {
    setup()
    expect(document.querySelector('.hand-slot')!.getAttribute('data-tip')).toBe('stage.hand_surface')
  })

  it('shows the mixer details in the display\'s place while ui.mixer is on', () => {
    setup()
    const has = (sel: string, c: string) => document.querySelector(sel)!.classList.contains(c)
    expect([has('.hand-slot', 'details'), has('.mixer-slot', 'details'), has('.display-slot', 'hidden')]).toEqual([false, false, false])
    ui.mixer = true
    flushSync()
    expect([has('.hand-slot', 'details'), has('.mixer-slot', 'details'), has('.display-slot', 'hidden')]).toEqual([true, true, true])
    ui.mixer = false
    flushSync()
    expect([has('.hand-slot', 'details'), has('.mixer-slot', 'details'), has('.display-slot', 'hidden')]).toEqual([false, false, false])
  })
})
