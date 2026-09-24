import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync, tick } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { MockSession } from '../../lib/api/mock'
import { app, ui } from '../../lib/store.svelte'
import Harmony from './Harmony.svelte'

async function setup() {
  const session = new MockSession({ manual: true, demo: true })
  app.attach(session)
  ui.harmony = true
  await tick()
  await Promise.resolve()
  flushSync()
  render(Harmony)
  return session
}

afterEach(() => {
  cleanup()
  app.detach()
  ui.harmony = false
})

const q = <T extends Element = HTMLElement>(sel: string) => [...document.querySelectorAll<T & HTMLElement>(sel)]
const option = (name: string) => q<HTMLButtonElement>('[role="option"]').find((b) => b.textContent?.trim() === name)!
const radio = (name: string) => q<HTMLButtonElement>('[role="radio"]').find((b) => b.textContent?.trim() === name)!

describe('Harmony/Arpeggio drawer', () => {
  it('lists the 23 Harmony types, the selected one marked, and switches the effect on', async () => {
    const s = await setup()
    expect(q('[data-tip="harmony.type"]')).toHaveLength(23)
    expect(option('Standard Duet 1').getAttribute('aria-selected')).toBe('true')
    await fireEvent.click(q('[data-tip="harmony.switch"]')[0])
    expect(s.state.harmonyArp.on).toBe(true)
    await fireEvent.click(option('Block'))
    expect(s.state.harmonyArp.typeName).toBe('Block')
    expect(document.querySelector('.now')?.textContent).toContain('Block')
  })

  it('shows the settings the type has: Speed for Echo, none for Multi Assign', async () => {
    const s = await setup()
    expect(q('[data-tip="harmony.chord_note_only"]')).toHaveLength(1)
    expect(q('[data-tip="harmony.speed"]')).toHaveLength(0)
    await fireEvent.click(option('Tremolo'))
    expect(q('[data-tip="harmony.speed"]')).toHaveLength(6)
    await fireEvent.click(radio('1/16'))
    expect(s.state.harmonyArp.speed).toBe('1/16')
    await fireEvent.click(option('Multi Assign'))
    expect(q('[data-tip="harmony.volume"]')).toHaveLength(0)
  })

  it('the Arpeggio list has the patterns and their settings', async () => {
    const s = await setup()
    await fireEvent.click(radio('Arpeggio'))
    expect(s.state.harmonyArp.mode).toBe('arpeggio')
    expect(q('[data-tip="harmony.pattern"]').length).toBeGreaterThan(20)
    await fireEvent.click(option('Alberti 16'))
    expect(s.state.harmonyArp.typeName).toBe('Alberti 16')
    await fireEvent.click(q('[data-tip="harmony.arp_hold"]')[0])
    expect(s.state.harmonyArp.arp.hold).toBe(true)
    expect(q('[data-tip="harmony.arp_fixed_velocity"]')).toHaveLength(0)
    await fireEvent.click(radio('Fixed'))
    expect(s.state.harmonyArp.arp.velocity).toBe('fixed')
    expect(q('[data-tip="harmony.arp_fixed_velocity"]')).toHaveLength(1)
    // Back to Harmony: the type used last.
    await fireEvent.click(radio('Harmony'))
    expect(s.state.harmonyArp.typeName).toBe('Standard Duet 1')
  })

  it('◀ ▶ step through both lists as one', async () => {
    const s = await setup()
    await fireEvent.click(q('[data-tip="harmony.prev_type"]')[0])
    expect(s.state.harmonyArp.mode).toBe('arpeggio')
    await fireEvent.click(q('[data-tip="harmony.next_type"]')[0])
    expect(s.state.harmonyArp.typeName).toBe('Standard Duet 1')
  })
})
