// The transport section in the app bar, on the mock session: its buttons send the
// transport commands, light from the page-1 pad lamps, and the readouts follow the state.

import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import App from '../../App.svelte'
import { MockSession } from '../../lib/api/mock'
import { app } from '../../lib/store.svelte'
import TransportBar from './TransportBar.svelte'

function setup() {
  const session = new MockSession({ manual: true })
  app.attach(session)
  render(TransportBar)
  return session
}

const q = <T extends Element = HTMLButtonElement>(sel: string) => document.querySelector<T>(sel)!
const bar = () => q<HTMLElement>('section[aria-label="Transport"]')

afterEach(() => cleanup())

describe('transport section', () => {
  it('is in the app bar itself, not a row of its own', () => {
    render(App, { props: { session: new MockSession({ demo: true, manual: true }) } })
    flushSync()
    expect(document.querySelectorAll('section[aria-label="Transport"]')).toHaveLength(1)
    const bar = document.querySelector('header.bar')!
    expect(bar.querySelector('section[aria-label="Transport"] [data-tip="transport.start_stop"]')).toBeTruthy()
    expect(bar.querySelector('[data-tip="settings.open"]')).toBeTruthy()
  })

  it('Start / Stop starts and stops the band, and shows which it will do', async () => {
    const s = setup()
    if (s.state.transport.running) await fireEvent.click(q('[data-tip="transport.start_stop"]'))
    flushSync()
    expect(s.state.transport.running).toBe(false)
    expect(bar().dataset.state).not.toBe('playing')
    expect(q('[data-tip="transport.start_stop"]').textContent).toContain('Start')

    await fireEvent.click(q('[data-tip="transport.start_stop"]'))
    flushSync()
    expect(s.state.transport.running).toBe(true)
    expect(bar().dataset.state).toBe('playing')
    expect(q('[data-tip="transport.start_stop"]').textContent).toContain('Stop')
    expect(q('.pos .bar').textContent).toMatch(/^\d+\.\d+$/)
  })

  it('Sync Start arms and shows SYNC; Sync Stop toggles', async () => {
    const s = setup()
    if (s.state.transport.running) await fireEvent.click(q('[data-tip="transport.start_stop"]'))
    if (s.state.transport.syncStart) await fireEvent.click(q('[data-tip="transport.sync_start"]'))
    flushSync()
    await fireEvent.click(q('[data-tip="transport.sync_start"]'))
    flushSync()
    expect(s.state.transport.syncStart).toBe(true)
    expect(bar().dataset.state).toBe('armed')
    expect(q('.pos .bar').textContent).toBe('SYNC')
    expect(q('[data-tip="transport.sync_start"]').getAttribute('aria-pressed')).toBe('true')

    const before = s.state.transport.syncStop
    await fireEvent.click(q('[data-tip="transport.sync_stop"]'))
    expect(s.state.transport.syncStop).toBe(!before)
  })

  it('Intro and Ending buttons send their section, and light from the pad lamps', async () => {
    const s = setup()
    if (s.state.transport.running) await fireEvent.click(q('[data-tip="transport.start_stop"]'))
    await fireEvent.click(q('[data-tip="section.intro2"]'))
    flushSync()
    expect(s.state.transport.pendingIntro).toBe(1)
    expect(q('.pos .now').textContent).toBe('Intro II armed')
    const lamp = s.state.transport.lamps.find((p) => p.action?.type === 'intro' && p.action.index === 1)!
    const btn = q<HTMLElement>('[data-tip="section.intro2"]')
    expect(btn.style.getPropertyValue('--led')).not.toBe('transparent')
    expect(lamp.level).not.toBe('off')
    expect(q('[data-tip="section.ending1"]')).toBeTruthy()
    expect(q('[data-tip="section.ending3"]')).toBeTruthy()
  })

  it('tempo − / + step by 1 BPM and the readout shows whole BPM', async () => {
    const s = setup()
    const t0 = s.state.transport.tempo
    await fireEvent.click(q('[data-tip="tempo.up"]'))
    expect(s.state.transport.tempo).toBe(t0 + 1)
    await fireEvent.click(q('[data-tip="tempo.down"]'))
    await fireEvent.click(q('[data-tip="tempo.down"]'))
    flushSync()
    expect(s.state.transport.tempo).toBe(t0 - 1)
    expect(q('[data-tip="display.tempo"]').textContent).toContain(String(Math.round(t0 - 1)))
    expect(q('[data-tip="tempo.tap"]')).toBeTruthy()
  })
})
