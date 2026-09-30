// A rack slot's insert-settings popover belongs to the part being edited: it stays open
// while that part is edited (and across other changes), and closes when another part
// becomes the edited one.

import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { MockSession } from '../../lib/api/mock'
import { app } from '../../lib/store.svelte'
import RackPanel from './RackPanel.svelte'

function setup() {
  const session = new MockSession({ manual: true, demo: true })
  app.attach(session)
  flushSync()
  render(RackPanel, { props: { docked: true } })
  session.advance(16)
  flushSync()
  return session
}

afterEach(() => {
  cleanup()
  app.detach()
})

const slot = (name: string) => document.querySelector<HTMLElement>(`.slot[aria-label="${name}"]`)!
const popover = (name: string) => slot(name).querySelector('[role="dialog"]')
const chip = (name: string, i: number) => slot(name).querySelectorAll<HTMLElement>('[data-tip="rack.insert_chip"]')[i]

describe('Rack slot: the insert popover', () => {
  it('closes when the edited part changes', async () => {
    const session = setup()
    // Pointer-down selects Right 2, the click opens its insert 1 settings.
    await fireEvent.pointerDown(chip('Right 2', 0))
    await fireEvent.click(chip('Right 2', 0))
    session.advance(16)
    flushSync()
    expect(session.state.keyboardParts[1].selected).toBe(true)
    expect(popover('Right 2')?.getAttribute('aria-label')).toBe('Right 2 insert 1 settings')
    // Another change to the part keeps it open.
    session.send({ type: 'setPartVolume', part: 1, volume: 40 })
    session.advance(16)
    flushSync()
    expect(popover('Right 2')).not.toBeNull()
    // Another part becomes the edited one (the Launchkey's EDIT pads, say): it closes.
    session.send({ type: 'selectPart', part: 2 })
    session.advance(16)
    flushSync()
    expect(session.state.keyboardParts[2].selected).toBe(true)
    expect(popover('Right 2')).toBeNull()
  })
})
