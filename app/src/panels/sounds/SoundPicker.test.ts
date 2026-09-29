import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync, tick } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { MockSession } from '../../lib/api/mock'
import { app, ui, type SoundPick } from '../../lib/store.svelte'
import SoundPicker from './SoundPicker.svelte'

async function setup(onpick: (id: string) => void = () => {}) {
  const session = new MockSession({ manual: true, demo: false })
  app.attach(session)
  app.sounds = await session.sounds()
  const pick: SoundPick = { title: 'Piano family', value: 'stage-grand', onpick }
  ui.soundPick = pick
  flushSync()
  render(SoundPicker, { props: { pick } })
  await tick()
  flushSync()
  return session
}

const input = () => document.querySelector<HTMLInputElement>('input[role="combobox"]')!
const rows = () => [...document.querySelectorAll<HTMLElement>('[role="option"]')]
const active = () => document.querySelector<HTMLElement>('[role="option"][aria-selected="true"]')!
const key = (k: string, o: KeyboardEventInit = {}) => fireEvent.keyDown(input(), { key: k, ...o })
const tipped = (k: string) => [...document.querySelectorAll<HTMLElement>(`[data-tip="${k}"]`)]

afterEach(() => {
  cleanup()
  app.detach()
  ui.soundPick = null
})

describe('the sound picker for a map rule', () => {
  it('opens on the rule\'s sound with the filter focused; Enter hands the sound over and closes', async () => {
    let got: string | null = null
    await setup((id) => (got = id))
    expect(document.activeElement).toBe(input())
    expect(active().textContent).toContain('Stage Grand')
    expect(document.body.textContent).toContain('Pick the sound for Piano family')
    await key('ArrowDown')
    const next = active().querySelector('.name')!.textContent
    await key('Enter')
    expect(got).toBeTruthy()
    expect(app.sounds.entries.find((e) => e.id === got)?.name).toBe(next)
    expect(ui.soundPick).toBe(null)
  })

  it('a click picks; it has no tabs, audition, save or edit controls', async () => {
    let got: string | null = null
    await setup((id) => (got = id))
    expect(document.querySelectorAll('[role="tab"]')).toHaveLength(0)
    for (const k of ['sounds.audition', 'sounds.save', 'sounds.save_over', 'sounds.more']) expect(tipped(k), k).toHaveLength(0)
    const first = app.sounds.entries[Number(rows()[0].id.slice('sound-'.length))].id
    await fireEvent.click(rows()[0])
    expect(got).toBe(first)
    expect(ui.soundPick).toBe(null)
  })

  it('the chips and the filter narrow the list; Ctrl+D stars', async () => {
    const s = await setup()
    const all = rows().length
    await fireEvent.click(tipped('sounds.saved')[0])
    flushSync()
    expect(rows().every((r) => r.querySelector('.badge')!.classList.contains('saved'))).toBe(true)
    await fireEvent.click(tipped('sounds.all')[0])
    await fireEvent.input(input(), { target: { value: 'grand' } })
    await tick()
    flushSync()
    expect(rows().length).toBeGreaterThan(0)
    expect(rows().length).toBeLessThan(all)
    const { id, favourite } = app.sounds.entries[Number(active().id.slice('sound-'.length))]
    await key('d', { ctrlKey: true })
    app.sounds = await s.sounds()
    flushSync()
    expect(app.sounds.entries.find((e) => e.id === id)?.favourite).toBe(!favourite)
  })
})
