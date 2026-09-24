import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { MockSession } from '../../lib/api/mock'
import { app, ui } from '../../lib/store.svelte'
import KeyStrip from './KeyStrip.svelte'

function setup() {
  const session = new MockSession({ manual: true, demo: true })
  app.attach(session)
  flushSync()
  render(KeyStrip)
  return session
}

afterEach(() => {
  cleanup()
  app.detach()
  ui.keyRange = null
})

const keys = () => document.querySelectorAll('.key')
const held = () => [...document.querySelectorAll<HTMLElement>('.key.held')]

describe('keyboard strip', () => {
  it('matches the connected Launchkey 49, and switches size', async () => {
    setup()
    expect(keys()).toHaveLength(49)
    await fireEvent.click([...document.querySelectorAll('button.size')].find((b) => b.textContent === '88')!)
    expect(keys()).toHaveLength(88)
    expect(ui.keyRange).toBe(88)
    // The chosen one again: back to matching the Launchkey.
    await fireEvent.click([...document.querySelectorAll('button.size')].find((b) => b.textContent === '88')!)
    expect(keys()).toHaveLength(49)
  })

  it('lights held keys in their part colour: grey chord-only left hand, Right 1 + Right 2 layered', () => {
    const session = setup()
    const kb = session.state.keyboard!
    expect(held()).toHaveLength(kb.held.length)
    const fills = held().map((k) => k.style.getPropertyValue('--fill'))
    expect(fills.some((f) => f === 'var(--part-chord)')).toBe(true)
    expect(fills.some((f) => f.includes('var(--part-r1)') && f.includes('var(--part-r2)'))).toBe(true)
  })

  it('marks the chord tones and the detection area', () => {
    const session = setup()
    expect(document.querySelectorAll('.dot').length).toBeGreaterThan(8)
    expect(document.querySelector('.tones')!.textContent).toContain('A')
    expect(document.querySelector('.area')!.textContent).toContain('Lower')
    session.send({ type: 'toggleUpper' })
    flushSync()
    expect(document.querySelector('.area')!.textContent).toContain('Upper')
  })

  it('updates as keys are pressed and released (every state)', () => {
    const session = setup()
    const before = held().map((k) => k.dataset.note).join()
    session.advance((1.1 * 60000) / session.state.transport.tempo) // a beat on: the right hand moves
    flushSync()
    expect(held().map((k) => k.dataset.note).join()).not.toBe(before)
  })

  it('moves the split with the arrow keys, through the engine', async () => {
    const session = setup()
    const split = session.state.chord.split
    const marker = document.querySelector<HTMLElement>('[role="slider"][aria-label="Split point"]')!
    await fireEvent.keyDown(marker, { key: 'ArrowRight' })
    expect(session.state.chord.split).toBe(split + 1)
    expect(marker.getAttribute('aria-valuenow')).toBe(String(split + 1))
    await fireEvent.keyDown(marker, { key: 'ArrowLeft' })
    expect(session.state.chord.split).toBe(split)
  })

  it('on an engine that sends no held keys, says so rather than showing none', () => {
    const session = new MockSession({ manual: true, demo: true })
    const st = { ...session.state, keyboard: undefined }
    app.attach({ ...session, kind: 'tauri', subscribe: (fn) => (fn(st), () => {}), send: () => {}, library: () => session.library(), dispose: () => {} })
    flushSync()
    render(KeyStrip)
    expect(held()).toHaveLength(0)
    expect(document.querySelector('.cheek')!.textContent).toContain('Not reported yet')
    expect(document.querySelector('.bed')!.getAttribute('aria-label')).toContain('not reported')
    // The split and the detection area are the engine's own: still shown.
    expect(document.querySelector('[role="slider"]')).not.toBeNull()
    expect(document.querySelector('.area')).not.toBeNull()
  })
})
