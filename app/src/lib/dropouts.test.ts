import { render, cleanup, screen, fireEvent } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import App from '../App.svelte'
import { MockSession } from './api/mock'
import { DropoutWatch, HINT_WINDOW_MS, SNOOZE_MS, dropouts } from './dropouts.svelte'
import { nav } from '../panels/settings/nav.svelte'
import { ui } from './store.svelte'

const at = (n: number, buffer: number | null = 64) => ({ dropouts: n, bufferFrames: buffer })

describe('the dropout hint', () => {
  it('ignores a single blip and speaks up when dropouts keep coming', () => {
    const w = new DropoutWatch()
    w.observe(at(0), 0)
    w.observe(at(1), 1000)
    expect(w.show).toBe(false)
    w.observe(at(2), 2000)
    expect(w.show).toBe(false)
    w.observe(at(3), 3000)
    expect(w.show).toBe(true)
  })

  it('forgets dropouts older than its window', () => {
    const w = new DropoutWatch()
    w.observe(at(0), 0)
    w.observe(at(2), 0)
    w.observe(at(3), HINT_WINDOW_MS + 1)
    expect(w.show).toBe(false)
  })

  it('counts from the first sight: dropouts before the app attached do not count', () => {
    const w = new DropoutWatch()
    w.observe(at(40), 0)
    expect(w.show).toBe(false)
  })

  it('stays away once dismissed, then may come back', () => {
    const w = new DropoutWatch()
    w.observe(at(0), 0)
    w.observe(at(5), 1000)
    expect(w.show).toBe(true)
    w.dismiss(1000)
    expect(w.show).toBe(false)
    w.observe(at(10), 5000)
    expect(w.show, 'not every few seconds').toBe(false)
    w.observe(at(15), 1000 + SNOOZE_MS + 1)
    expect(w.show).toBe(true)
  })

  it('a new buffer size clears it and starts the count again', () => {
    const w = new DropoutWatch()
    w.observe(at(0), 0)
    w.observe(at(5), 1000)
    expect(w.show).toBe(true)
    w.observe(at(5, 256), 2000)
    expect(w.show).toBe(false)
    w.observe(at(6, 256), 3000)
    expect(w.show).toBe(false)
  })

  it('has nothing to suggest at the largest buffer, or without the synth', () => {
    const w = new DropoutWatch()
    w.observe(at(0, 1024), 0)
    w.observe(at(9, 1024), 1000)
    expect(w.show).toBe(false)
    w.observe(null, 2000)
    expect(w.show).toBe(false)
  })
})

describe('the dropout notice', () => {
  afterEach(() => {
    cleanup()
    ui.settings = false
    nav.tab = 'chord'
    dropouts.reset()
  })

  it('appears when the engine reports dropouts, opens Settings › Audio, and dismisses', () => {
    const session = new MockSession({ demo: true, manual: true })
    render(App, { props: { session } })
    session.advance(16)
    flushSync()
    expect(screen.queryByRole('alert')).toBeNull()
    session.dropouts(4)
    flushSync()
    expect(screen.getByRole('alert').textContent).toContain('consider increasing the buffer size')
    fireEvent.click(screen.getByText('Buffer size…'))
    flushSync()
    expect(ui.settings).toBe(true)
    expect(nav.tab).toBe('audio')
    fireEvent.click(screen.getByLabelText('Dismiss'))
    flushSync()
    expect(screen.queryByRole('alert')).toBeNull()
  })

  it('goes away when a larger buffer is chosen', () => {
    const session = new MockSession({ demo: true, manual: true })
    render(App, { props: { session } })
    session.advance(16)
    session.dropouts(4)
    flushSync()
    expect(screen.getByRole('alert')).toBeTruthy()
    session.send({ type: 'setAudioBuffer', frames: 256 })
    flushSync()
    expect(screen.queryByRole('alert')).toBeNull()
  })
})
