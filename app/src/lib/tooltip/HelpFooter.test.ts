import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { TIPS } from '../../help/tooltips'
import { MockSession } from '../api/mock'
import { app } from '../store.svelte'
import HelpFooter from './HelpFooter.svelte'
import { lastControl } from './lastControl'
import { CLEAR_MS, tip, tips, TOOLTIP_ID } from './tip.svelte'

const pack = (s: number, d1: number, d2: number) => (s << 16) | (d1 << 8) | d2

function setup() {
  const session = new MockSession({ manual: true, demo: false })
  app.attach(session)
  flushSync()
  render(HelpFooter)
  // Two controls with tips, as any panel would have them.
  const a = document.createElement('button')
  const b = document.createElement('button')
  document.body.append(a, b)
  const ta = tip(a, 'transport.sync_start')
  const tb = tip(b, 'section.main_b')
  return { session, a, b, destroy: () => (ta.destroy(), tb.destroy()) }
}

const footer = () => document.querySelector<HTMLElement>('.help-footer')!
const hover = (el: HTMLElement) => fireEvent.pointerEnter(el, { pointerType: 'mouse' })
const leave = (el: HTMLElement) => fireEvent.pointerLeave(el, { pointerType: 'mouse' })

beforeEach(() => vi.useFakeTimers())
afterEach(() => {
  vi.useRealTimers()
  cleanup()
  document.body.innerHTML = ''
  app.detach()
  tips.hide()
  tips.help = false
  tips.pinned = null
  tips.focused = null
  tips.setFloating(false)
})

describe('help footer', () => {
  it('shows a quiet hint when nothing is hovered', () => {
    setup()
    expect(footer().textContent).toContain('Hover any control to learn what it does')
  })

  it('shows the hovered control’s catalog entry at once: title, body, Genos, key, Launchkey', async () => {
    const { a } = setup()
    await hover(a)
    const t = TIPS['transport.sync_start']
    const text = footer().textContent!
    expect(text).toContain(t.title)
    expect(text).toContain(t.body)
    expect(text).toContain(`Genos: ${t.genos}`)
    expect(text).toContain('Y')
    expect(text).toContain(t.launchkey!)
  })

  it('keeps the entry briefly after the pointer leaves, so crossing a gap does not flicker', async () => {
    const { a, b } = setup()
    await hover(a)
    await leave(a)
    vi.advanceTimersByTime(CLEAR_MS - 50)
    flushSync()
    expect(footer().textContent).toContain(TIPS['transport.sync_start'].title)
    await hover(b)
    vi.advanceTimersByTime(CLEAR_MS)
    flushSync()
    expect(footer().textContent).toContain(TIPS['section.main_b'].title)
    await leave(b)
    vi.advanceTimersByTime(CLEAR_MS)
    flushSync()
    expect(footer().textContent).toContain('Hover any control')
  })

  it('nothing floats over the UI unless the player opts in', async () => {
    const { a } = setup()
    await hover(a)
    expect(tips.rect).toBeNull()
    tips.setFloating(true)
    await leave(a)
    await hover(a)
    vi.advanceTimersByTime(400)
    expect(tips.rect).not.toBeNull()
  })

  it('help mode grows the footer to the full entry and pins it after the pointer leaves', async () => {
    const { a } = setup()
    tips.toggleHelp()
    await hover(a)
    await leave(a)
    vi.advanceTimersByTime(CLEAR_MS * 3)
    flushSync()
    expect(footer().classList.contains('expanded')).toBe(true)
    expect(footer().textContent).toContain(TIPS['transport.sync_start'].title)
  })

  it('describes the keyboard-focused control to screen readers, whatever the pointer is on', async () => {
    const { a, b } = setup()
    a.focus() // jsdom: no :focus-visible support, so focus counts as keyboard focus
    await hover(b)
    flushSync()
    expect(a.getAttribute('aria-describedby')).toBe(TOOLTIP_ID)
    const desc = document.getElementById(TOOLTIP_ID)!.textContent!
    expect(desc).toContain(TIPS['transport.sync_start'].body)
    expect(desc).toContain('Launchkey:')
    a.blur()
    expect(a.hasAttribute('aria-describedby')).toBe(false)
  })

  it('shows the Launchkey control pressed last, from io.lastControl', async () => {
    const { session } = setup()
    expect(footer().querySelector('.lk')!.textContent).toContain('press a pad or button')
    session.state.io.lastControl = pack(0x90, 113, 127)
    session.advance(16)
    flushSync()
    expect(footer().querySelector('.lk')!.textContent).toContain('PAD 10 (page 1)')
    expect(footer().querySelector('.lk')!.textContent).toContain('MAIN B')
  })
})

describe('lastControl', () => {
  function state(page?: 'chordSetup') {
    const s = new MockSession({ manual: true })
    if (page) s.send({ type: 'setPadPage', page })
    return s.state
  }

  it('names a pad by number and page, with what it does now', () => {
    const s = state('chordSetup')
    const pad = s.pads.pads.find((p) => p.note === 101)!
    const d = lastControl(pack(0x90, 101, 127), s)!
    expect(d.where).toBe('PAD 6 (page 2)')
    expect(d.what).toBe(pad.label)
    // A note off names the same pad.
    expect(lastControl(pack(0x80, 101, 0), s)!.where).toBe('PAD 6 (page 2)')
  })

  it('names buttons from the surface and faders by number', () => {
    const s = state()
    const play = s.surface.controls.find((c) => c.id === 'play')!
    expect(lastControl(pack(0xb0, play.cc, 127), s)!.where).toBe('PLAY')
    const f = lastControl(pack(0xb0, 6, 90), s)!
    expect(f.where).toBe('FADER 2')
    expect(f.what).toContain(s.surface.faders[1].label)
    expect(lastControl(pack(0xb0, 13, 90), s)!.where).toBe('MASTER FADER')
  })

  it('ignores nothing-yet, Shift on its own and channel-7 mode reports', () => {
    const s = state()
    expect(lastControl(0, s)).toBeNull()
    expect(lastControl(pack(0xb0, 63, 127), s)).toBeNull()
    expect(lastControl(pack(0xb6, 29, 1), s)).toBeNull()
  })
})
