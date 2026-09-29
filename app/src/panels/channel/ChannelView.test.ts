import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { MockSession } from '../../lib/api/mock'
import { app } from '../../lib/store.svelte'
import { isTipKey } from '../../help/tooltips'
import ChannelView from './ChannelView.svelte'

/** Controls without a catalog tooltip. */
const untipped = (root: ParentNode) =>
  [...root.querySelectorAll('button, select, [role="slider"], [role="switch"], [tabindex]:not([tabindex="-1"])')]
    .filter((e) => !isTipKey(e.getAttribute('data-tip') ?? ''))
    .map((e) => e.outerHTML.slice(0, 120))

function setup(part = 0) {
  const session = new MockSession({ manual: true, demo: true })
  app.attach(session)
  flushSync()
  const sent = vi.spyOn(session, 'send')
  const onpart = vi.fn()
  const onclose = vi.fn()
  render(ChannelView, { props: { part, onpart, onclose } })
  return { session, sent, onpart, onclose }
}

afterEach(() => {
  cleanup()
  app.detach()
})

const control = (label: string) => document.querySelector<HTMLElement>(`[aria-label="${label}"]`)
const card = (name: string) => document.querySelector<HTMLElement>(`section.card[aria-label="${name}"]`)!
const toggle = (root: ParentNode) => root.querySelector<HTMLElement>('[role="switch"]')!

describe('Channel view', () => {
  it('shows a keyboard part\'s strip: level, EQ, compressor, two inserts, sends and play', () => {
    setup(0)
    expect(document.querySelector('h2')?.textContent).toBe('Right 1')
    expect([...document.querySelectorAll('section.card')].map((e) => e.getAttribute('aria-label'))).toEqual(['Level', 'EQ', 'Compressor', 'Insert 1', 'Insert 2', 'Sends', 'Play'])
    expect(untipped(document.body)).toEqual([])
  })

  it('‹ › call onpart with the wrapped strip index; × closes', async () => {
    const { onpart, onclose } = setup(0)
    await fireEvent.click(control('Previous part')!)
    expect(onpart).toHaveBeenLastCalledWith(11)
    await fireEvent.click(control('Next part')!)
    expect(onpart).toHaveBeenLastCalledWith(1)
    await fireEvent.click(control('Close channel view')!)
    expect(onclose).toHaveBeenCalledOnce()
    cleanup()
    const last = vi.fn()
    render(ChannelView, { props: { part: 11, onpart: last, onclose: () => {} } })
    await fireEvent.click(control('Next part')!)
    expect(last).toHaveBeenLastCalledWith(0)
  })

  it('pan on a keyboard part sends setPartPan', async () => {
    const { sent } = setup(1)
    const pan = control('Right 2 pan')!
    await fireEvent.keyDown(pan, { key: 'ArrowRight' })
    expect(sent).toHaveBeenLastCalledWith({ type: 'setPartPan', part: 1, pan: 65 })
  })

  it('a Style part has no pan, no Play section, and its first slot is the style insert', async () => {
    const { session, sent } = setup(4)
    const name = session.state.mixer.styleParts[0].name
    expect(document.querySelector('h2')?.textContent).toBe(name)
    expect(control(`${name} pan`)).toBeNull()
    expect(document.querySelector('[data-tip="channel.pan"]')).toBeNull()
    expect(document.querySelector('section.card[aria-label="Play"]')).toBeNull()
    expect(card('Style insert')).toBeTruthy()
    await fireEvent.keyDown(control(`${name} level`)!, { key: 'Home' })
    expect(sent).toHaveBeenLastCalledWith({ type: 'setStylePartVolume', part: 0, volume: 0 })
  })

  it('choosing an insert kind sends setStripInsertKind, and its settings appear from the state', async () => {
    const { session, sent } = setup(0)
    const slot = card('Insert 2')
    expect(slot.querySelectorAll('[role="slider"]').length).toBe(0)
    const pick = control('Right 1 insert 2 type') as HTMLSelectElement
    expect([...pick.options].map((o) => o.value)).toEqual(['none', 'distortion', 'compressor', 'autoWah', 'tremolo', 'rotary', 'phaser'])
    await fireEvent.change(pick, { target: { value: 'tremolo' } })
    expect(sent).toHaveBeenLastCalledWith({ type: 'setStripInsertKind', strip: 0, slot: 1, kind: 'tremolo' })
    flushSync()
    const names = session.state.keyboardParts[0].strip.inserts[1].settings.map((s) => s.name)
    expect(names.length).toBeGreaterThan(0)
    expect([...slot.querySelectorAll('[role="slider"]')].map((e) => e.getAttribute('aria-label'))).toEqual(names.map((n) => `Right 1 insert 2 ${n}`))
    expect(control('Right 1 insert 2 Note')!.getAttribute('aria-valuetext')).toBe('1/8')
  })

  it('an insert setting sends setStripInsertSetting with its index; on/off sends setStripInsertOn', async () => {
    const { session, sent } = setup(0)
    await fireEvent.change(control('Right 1 insert 2 type')!, { target: { value: 'phaser' } })
    flushSync()
    const rate = session.state.keyboardParts[0].strip.inserts[1].settings[1]
    await fireEvent.keyDown(control(`Right 1 insert 2 ${rate.name}`)!, { key: 'ArrowUp' })
    expect(sent).toHaveBeenLastCalledWith({ type: 'setStripInsertSetting', strip: 0, slot: 1, setting: 1, value: rate.value + 1 })
    await fireEvent.dblClick(control(`Right 1 insert 2 ${rate.name}`)!)
    expect(session.state.keyboardParts[0].strip.inserts[1].settings[1].value).toBe(rate.default)
    await fireEvent.click(toggle(card('Insert 2')))
    expect(sent).toHaveBeenLastCalledWith({ type: 'setStripInsertOn', strip: 0, slot: 1, on: true })
  })

  it('the compressor: on, type, a parameter, and the edited mark', async () => {
    const { sent } = setup(2)
    const comp = card('Compressor')
    expect(comp.textContent).not.toContain('edited')
    await fireEvent.click(toggle(comp))
    expect(sent).toHaveBeenLastCalledWith({ type: 'setStripCompressorOn', strip: 2, on: true })
    await fireEvent.change(control('Right 3 compressor type')!, { target: { value: 'punchy' } })
    expect(sent).toHaveBeenLastCalledWith({ type: 'setStripCompressorPreset', strip: 2, preset: 'punchy' })
    flushSync()
    const ratio = control('Right 3 compressor Ratio')!
    expect(ratio.getAttribute('aria-valuetext')).toMatch(/^\d+\.\d:1$/)
    await fireEvent.keyDown(ratio, { key: 'End' })
    expect(sent).toHaveBeenLastCalledWith({ type: 'setStripCompressorParam', strip: 2, param: 'ratio', value: 200 })
    flushSync()
    expect(control('Right 3 compressor Ratio')!.getAttribute('aria-valuetext')).toBe('20.0:1')
    expect(comp.textContent).toContain('edited')
  })

  it('a send knob per send that exists sends setStripSend', async () => {
    const { session, sent } = setup(5)
    const name = session.state.mixer.styleParts[1].name
    const sends = session.state.effects.sends
    expect(card('Sends').querySelectorAll('[role="slider"]').length).toBe(sends.length)
    const s = sends[1]
    await fireEvent.keyDown(control(`${name} send 2 ${s.name}`)!, { key: 'End' })
    expect(sent).toHaveBeenLastCalledWith({ type: 'setStripSend', strip: 5, send: 1, level: 127 })
    flushSync()
    expect(control(`${name} send 2 ${s.name}`)!.getAttribute('aria-valuenow')).toBe('127')
  })

  it('EQ knobs send setStripEq with the whole EQ', async () => {
    const { session, sent } = setup(3)
    const eq = session.state.keyboardParts[3].strip.eq
    await fireEvent.keyDown(control('Left EQ high')!, { key: 'ArrowUp' })
    expect(sent).toHaveBeenLastCalledWith({ type: 'setStripEq', strip: 3, eq: { ...eq, highGain: eq.highGain + 1 } })
  })

  it('octave and bend range steppers', async () => {
    const { session, sent } = setup(0)
    const octave = session.state.keyboardParts[0].octave
    await fireEvent.click(control('Right 1 octave up')!)
    expect(sent).toHaveBeenLastCalledWith({ type: 'setPartOctave', part: 0, octave: octave + 1 })
    const bend = session.state.controllers.parts[0].bendRange
    await fireEvent.click(control('Right 1 bend range down')!)
    expect(sent).toHaveBeenLastCalledWith({ type: 'setBendRange', part: 0, semitones: bend - 1 })
  })
})
