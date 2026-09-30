// The Channel view in the app: a strip's name opens it in the display; the same strip
// again, Esc or × closes it and the lead sheet comes back; ‹ › step the selected part.

import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import App from '../../App.svelte'
import { MockSession } from '../../lib/api/mock'
import { ui } from '../../lib/store.svelte'
import { channelNav } from './nav.svelte'

afterEach(() => {
  cleanup()
  channelNav.close()
  ui.selectedPart = 0
  ui.effects = false
  ui.mixer = false
})

function setup() {
  const session = new MockSession({ demo: true, manual: true })
  render(App, { props: { session } })
  session.advance(16)
  flushSync()
  return session
}

const channel = () => document.querySelector('.display-slot section.channel')
const stripName = (part: number) => document.querySelector<HTMLElement>(`[data-part="${part}"] [data-tip="mixer.strip.select"]`)!

function clickStrip(part: number) {
  stripName(part).click()
  flushSync()
}

describe('Channel view in the display', () => {
  it('opens on the clicked strip and closes when it is clicked again', () => {
    setup()
    expect(channel()).toBeNull()
    clickStrip(2)
    expect(channel()?.getAttribute('aria-label')).toBe('Right 3 channel')
    expect(ui.selectedPart).toBe(2)
    clickStrip(5)
    expect(channel()?.getAttribute('aria-label')).toMatch(/channel$/)
    expect(ui.selectedPart).toBe(5)
    clickStrip(5)
    expect(channel()).toBeNull()
  })

  it('Esc closes it, but first what is open over the stage', async () => {
    setup()
    clickStrip(0)
    ui.effects = true
    flushSync()
    await fireEvent.keyDown(window, { key: 'Escape' })
    expect(ui.effects).toBe(false)
    expect(channel()).not.toBeNull()
    await fireEvent.keyDown(window, { key: 'Escape' })
    expect(channel()).toBeNull()
  })

  it('with the Details layer shown a strip name only selects the part', () => {
    setup()
    ui.mixer = true
    flushSync()
    clickStrip(2)
    expect(ui.selectedPart).toBe(2)
    expect(channelNav.open).toBe(false)
    ui.mixer = false
    flushSync()
    clickStrip(2)
    expect(channelNav.open).toBe(true)
    ui.mixer = true
    flushSync()
    clickStrip(2)
    expect(channelNav.open).toBe(true)
  })

  it('‹ › step the selected part and × closes', async () => {
    const session = setup()
    clickStrip(0)
    const sent: unknown[] = []
    const send = session.send.bind(session)
    session.send = (c) => (sent.push(c), send(c))
    await fireEvent.click(document.querySelector<HTMLElement>('[data-tip="mixer.channel.prev"]')!)
    expect(ui.selectedPart).toBe(11)
    await fireEvent.click(document.querySelector<HTMLElement>('[data-tip="mixer.channel.next"]')!)
    await fireEvent.click(document.querySelector<HTMLElement>('[data-tip="mixer.channel.next"]')!)
    expect(ui.selectedPart).toBe(1)
    expect(sent).toContainEqual({ type: 'selectPart', part: 1 })
    expect(channel()?.getAttribute('aria-label')).toBe('Right 2 channel')
    await fireEvent.click(document.querySelector<HTMLElement>('[data-tip="mixer.channel.close"]')!)
    expect(channel()).toBeNull()
  })
})
