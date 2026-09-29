import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { MockSession } from '../../lib/api/mock'
import type { FxType } from '../../lib/api/types'
import { app, ui } from '../../lib/store.svelte'
import { isTipKey } from '../../help/tooltips'
import { NAV } from '../../lib/nav'
import Effects from './Effects.svelte'

/** Controls without a catalog tooltip (the full check is help/coverage.test.ts). */
const untipped = (root: ParentNode) =>
  [...root.querySelectorAll('button, [role="slider"], [role="switch"], [tabindex]:not([tabindex="-1"])')]
    .filter((e) => !isTipKey(e.getAttribute('data-tip') ?? ''))
    .map((e) => e.outerHTML.slice(0, 120))

function setup() {
  const session = new MockSession({ manual: true, demo: true })
  app.attach(session)
  ui.effects = true
  flushSync()
  render(Effects)
  return session
}

afterEach(() => {
  cleanup()
  app.detach()
  ui.effects = false
})

const card = (name: string) => document.querySelector<HTMLElement>(`section[aria-label="${name}"]`)!
const chips = (name: string) => [...card(name).querySelectorAll<HTMLButtonElement>(`[aria-label="${name} type"] button`)]
const chip = (name: string, text: string) => chips(name).find((b) => b.textContent?.trim() === text)!
const control = (label: string) => document.querySelector<HTMLElement>(`[aria-label="${label}"]`)!
const toggle = (root: ParentNode, text: string) => [...root.querySelectorAll<HTMLElement>('[role="switch"]')].find((e) => e.textContent?.includes(text))!

describe('Effects screen', () => {
  it('has a Reverb, a Chorus and a Delay card, and the style\'s inserts', () => {
    setup()
    expect([...document.querySelectorAll('section.card')].map((e) => e.getAttribute('aria-label'))).toEqual(['Reverb', 'Chorus', 'Delay', 'Style inserts'])
  })

  it('type chips come from the block\'s types in the state, and a chip sends setEffectType (#204)', async () => {
    const s = setup()
    expect(chips('Reverb').map((b) => b.textContent?.trim())).toEqual(s.state.effects.blocks[0].types.map((t) => t.name))
    expect(chips('Delay').map((b) => b.textContent?.trim())).toEqual(['Delay 1/8', 'Delay 1/8.', 'Delay 1/4', 'Ping-Pong'])
    expect(chip('Reverb', 'Hall').getAttribute('aria-pressed')).toBe('true')
    await fireEvent.click(chip('Delay', 'Ping-Pong'))
    expect(s.state.effects.blocks[2].effect).toBe('pingPong')
    flushSync()
    expect(chip('Delay', 'Ping-Pong').getAttribute('aria-pressed')).toBe('true')
    expect(chip('Delay', 'Delay 1/8.').getAttribute('aria-pressed')).toBe('false')
  })

  it('a type the engine adds later shows as a chip with no UI change', async () => {
    const s = setup()
    s.state.effects.blocks[0].types.push({ effect: 'cathedral' as FxType, name: 'Cathedral' })
    s.send({ type: 'setEffectReturn', block: 'reverb', level: 64 })
    flushSync()
    await fireEvent.click(chip('Reverb', 'Cathedral'))
    expect(s.state.effects.blocks[0].effect).toBe('cathedral')
  })

  it('the return knob sends setEffectReturn, 64 = 0 dB', async () => {
    const s = setup()
    const ret = control('Reverb return')
    expect(ret.getAttribute('aria-valuetext')).toBe('+0.0 dB')
    await fireEvent.keyDown(ret, { key: 'Home' })
    expect(s.state.effects.blocks[0].returnLevel).toBe(0)
    await fireEvent.keyDown(control('Delay return'), { key: 'End' })
    expect(s.state.effects.blocks[2].returnLevel).toBe(127)
  })

  it('reverb knobs set its parameters in their own units; a type change puts them back (#236)', async () => {
    const s = setup()
    const time = control('Reverb Time')
    expect(time.getAttribute('aria-valuetext')).toBe('2.4 s')
    expect(time.getAttribute('aria-valuemin')).toBe(String(s.state.effects.blocks[0].params[0].min))
    await fireEvent.keyDown(time, { key: 'End' })
    expect(s.state.effects.blocks[0].params[0].value).toBe(100)
    expect(s.state.effects.blocks[0].params[0].display).toBe('10.0 s')
    await fireEvent.keyDown(control('Reverb Pre-delay'), { key: 'ArrowUp' })
    expect(s.state.effects.blocks[0].params[1].value).toBe(23)
    await fireEvent.click(chip('Reverb', 'Room'))
    expect(s.state.effects.blocks[0].params.map((p) => p.value)).toEqual([9, 4, 60])
  })

  it('chorus knobs: rate and depth (#236)', async () => {
    const s = setup()
    const depth = control('Chorus Depth')
    expect(depth.getAttribute('aria-valuetext')).toBe('2.2 ms')
    await fireEvent.keyDown(depth, { key: 'ArrowUp' })
    expect(s.state.effects.blocks[1].params[1].display).toBe('2.3 ms')
    await fireEvent.keyDown(control('Chorus Rate'), { key: 'ArrowDown' })
    expect(s.state.effects.blocks[1].params[0].value).toBe(54)
  })

  it('delay: Note with tempo sync on, free Time with it off; Ping-pong switch (#236)', async () => {
    const s = setup()
    const delay = card('Delay')
    expect(control('Delay Note').getAttribute('aria-valuetext')).toBe('1/8.')
    expect(delay.querySelector('[aria-label="Delay Time"]')).toBeNull()
    await fireEvent.keyDown(control('Delay Note'), { key: 'ArrowUp' })
    expect(s.state.effects.blocks[2].params.find((p) => p.param === 'delayNote')!.value).toBe(5)
    await fireEvent.click(toggle(delay, 'Tempo sync'))
    flushSync()
    expect(s.state.effects.blocks[2].params[0].value).toBe(0)
    expect(delay.querySelector('[aria-label="Delay Note"]')).toBeNull()
    await fireEvent.keyDown(control('Delay Time'), { key: 'PageUp' })
    const time = s.state.effects.blocks[2].params.find((p) => p.param === 'delayTime')!
    expect(time.value).toBeGreaterThan(time.default)
    await fireEvent.keyDown(control('Delay Feedback'), { key: 'Home' })
    expect(s.state.effects.blocks[2].params.find((p) => p.param === 'delayFeedback')!.value).toBe(0)
    await fireEvent.click(toggle(delay, 'Ping-pong'))
    expect(s.state.effects.blocks[2].params[5].display).toBe('On')
  })

  it('From style / Mine send setFollowStyle; choosing a type makes it Mine (#237)', async () => {
    const s = setup()
    const reverb = card('Reverb')
    const from = reverb.querySelector<HTMLElement>('[data-tip="fx.follow_style"]')!
    const mine = reverb.querySelector<HTMLElement>('[data-tip="fx.mine"]')!
    expect(from.getAttribute('aria-pressed')).toBe('true')
    expect(mine.getAttribute('aria-pressed')).toBe('false')
    await fireEvent.click(mine)
    expect(s.state.effects.blocks[0].followStyle).toBe(false)
    await fireEvent.click(from)
    expect(s.state.effects.blocks[0].followStyle).toBe(true)
    await fireEvent.click(chip('Reverb', 'Plate'))
    flushSync()
    expect(s.state.effects.blocks[0].followStyle).toBe(false)
    expect(mine.getAttribute('aria-pressed')).toBe('true')
    await fireEvent.click(from)
    expect(s.state.effects.blocks[0].effect).toBe('hall')
  })

  it('shows the style\'s own type, or that it sets none', () => {
    const s = setup()
    expect(card('Chorus').textContent).toContain('Style: sets none')
    s.state.effects.blocks[0].styleEffect = { name: 'Hall3 (XG)', effect: null }
    s.send({ type: 'setEffectReturn', block: 'reverb', level: 64 })
    flushSync()
    expect(card('Reverb').textContent).toContain('Style: Hall3 (XG) (no match)')
  })

  it('band and pads sends per block, chorus and delay off at start (#236, #267)', async () => {
    const s = setup()
    const band = (n: string) => control(`${n} band send`)
    const pad = (n: string) => control(`${n} Multi Pad send`)
    expect(['Reverb', 'Chorus', 'Delay'].map((n) => band(n).getAttribute('aria-valuetext'))).toEqual(['100%', '0%', '0%'])
    expect(['Reverb', 'Chorus', 'Delay'].map((n) => pad(n).getAttribute('aria-valuetext'))).toEqual(['100%', '0%', '0%'])
    await fireEvent.keyDown(band('Delay'), { key: 'PageUp' })
    expect(s.state.effects.blocks[2].bandSend).toBe(10)
    expect(s.state.effects.blocks[2].padSend).toBe(0)
    await fireEvent.keyDown(pad('Chorus'), { key: 'PageUp' })
    expect(s.state.effects.blocks[1].padSend).toBe(10)
    await fireEvent.keyDown(pad('Reverb'), { key: 'Home' })
    expect(s.state.effects.blocks[0].padSend).toBe(0)
  })

  it('inserts: all on/off, rotary fast, each part on/off and amount (#269)', async () => {
    const s = setup()
    const group = card('Style inserts')
    expect(group.textContent).toContain('British Combo Classic')
    expect(group.textContent).toContain('→ Distortion')
    await fireEvent.click(toggle(group, 'Inserts on'))
    expect(s.state.effects.insertsOn).toBe(false)
    await fireEvent.click(toggle(group, 'Rotary fast'))
    expect(s.state.effects.rotaryFast).toBe(true)
    await fireEvent.click(toggle(group, 'Chord 1'))
    expect(s.state.effects.inserts[0].on).toBe(false)
    await fireEvent.keyDown(control('Chord 1 insert amount'), { key: 'PageUp' })
    expect(s.state.effects.inserts[0].amount).toBe(74)
  })

  it('says so when the style has no inserts', () => {
    const s = setup()
    s.state.effects.inserts = []
    s.send({ type: 'setRotaryFast', on: false })
    flushSync()
    expect(card('Style inserts').textContent).toContain('The style has no insertion effects.')
  })

  it('opens from the quick nav\'s Effects button and closes the Mixer', () => {
    ui.mixer = true
    const nav = NAV.find((n) => n.tip === 'nav.effects')!
    expect(nav.key).toBe('alt+e')
    nav.toggle()
    expect(nav.open()).toBe(true)
    expect(ui.effects).toBe(true)
    expect(ui.mixer).toBe(false)
    expect(ui.escape()).toBe(true)
    expect(ui.effects).toBe(false)
  })

  it('every control has a tooltip, with the delay\'s tempo sync on and off', async () => {
    setup()
    expect(untipped(document.body)).toEqual([])
    await fireEvent.click(toggle(card('Delay'), 'Tempo sync'))
    flushSync()
    expect(untipped(document.body)).toEqual([])
  })
})

describe('Effects screen: send effects', () => {
  const addGroup = () => document.querySelector<HTMLElement>('[role="group"][aria-label="Add send"]')
  const select = (label: string) => control(label) as HTMLSelectElement
  const spy = (s: MockSession) => vi.spyOn(s, 'send')
  const sendCards = () => [...document.querySelectorAll('section.card[data-send]')].map((e) => e.getAttribute('aria-label'))

  async function addSend(kind: string) {
    const picker = select('New send type')
    picker.value = kind
    await fireEvent.change(picker)
    await fireEvent.click(addGroup()!.querySelector<HTMLElement>('[data-tip="fx.send_add"]')!)
    flushSync()
  }

  it('Add send sends addSend with the picked kind, Hall to start, and a fourth card shows', async () => {
    const s = setup()
    const sent = spy(s)
    expect(sendCards()).toEqual([])
    expect(select('New send type').value).toBe('hall')
    await fireEvent.click(addGroup()!.querySelector<HTMLElement>('[data-tip="fx.send_add"]')!)
    expect(sent).toHaveBeenLastCalledWith({ type: 'addSend', kind: 'hall' })
    flushSync()
    expect(sendCards()).toEqual(['Send 4'])
    await addSend('phaser')
    expect(sent).toHaveBeenLastCalledWith({ type: 'addSend', kind: 'phaser' })
    expect(sendCards()).toEqual(['Send 4', 'Send 5'])
    expect(card('Send 5').textContent).toContain('Phaser')
  })

  it('the Add control goes at six sends and comes back after a remove', async () => {
    const s = setup()
    const sent = spy(s)
    await addSend('hall')
    await addSend('room')
    expect(addGroup()).not.toBeNull()
    await addSend('plate')
    expect(s.state.effects.sends).toHaveLength(6)
    expect(addGroup()).toBeNull()
    await fireEvent.click(control('Remove Send 5'))
    expect(sent).toHaveBeenLastCalledWith({ type: 'removeSend', send: 4 })
    flushSync()
    expect(sendCards()).toEqual(['Send 4', 'Send 5'])
    expect(card('Send 5').textContent).toContain('Plate')
    expect(addGroup()).not.toBeNull()
  })

  it('an added send\'s kind, parameters and return', async () => {
    const s = setup()
    await addSend('hall')
    const sent = spy(s)
    const kind = select('Send 4 type')
    expect(kind.value).toBe('hall')
    kind.value = 'pingPong'
    await fireEvent.change(kind)
    expect(sent).toHaveBeenLastCalledWith({ type: 'setSendKind', send: 3, kind: 'pingPong' })
    flushSync()
    expect(card('Send 4').textContent).toContain('Ping-Pong')

    const p = s.state.effects.sends[3].params
    const feedback = p.findIndex((x) => x.name === 'Feedback')
    const knob = control(`Send 4 ${p[feedback].name}`)
    expect(knob.getAttribute('aria-valuemin')).toBe(String(p[feedback].min))
    expect(knob.getAttribute('aria-valuemax')).toBe(String(p[feedback].max))
    expect(knob.getAttribute('aria-valuetext')).toBe(p[feedback].display)
    await fireEvent.keyDown(knob, { key: 'ArrowUp' })
    expect(sent).toHaveBeenLastCalledWith({ type: 'setSendParam', send: 3, param: feedback, value: p[feedback].value + 1 })
    flushSync()
    await fireEvent.dblClick(control(`Send 4 ${p[feedback].name}`))
    expect(sent).toHaveBeenLastCalledWith({ type: 'setSendParam', send: 3, param: feedback, value: p[feedback].default })

    await fireEvent.keyDown(control('Send 4 return'), { key: 'Home' })
    expect(sent).toHaveBeenLastCalledWith({ type: 'setSendReturn', send: 3, level: 0 })
    flushSync()
    expect(control('Send 4 return').getAttribute('aria-valuetext')).toBe('Off')
  })

  it('shows a kind this build doesn\'t list by its name', () => {
    const s = setup()
    s.send({ type: 'addSend', kind: 'hall' })
    const sends = [...s.state.effects.sends]
    sends[3] = { ...sends[3], kind: 'shimmer', name: 'Shimmer', params: [] }
    app.apply({ ...s.state, version: s.state.version + 1, effects: { ...s.state.effects, sends } })
    flushSync()
    const kind = select('Send 4 type')
    expect(kind.value).toBe('shimmer')
    expect(kind.selectedOptions[0].textContent).toBe('Shimmer')
  })

  it('sends 1-3: the rack override toggle, and a Set by rack badge with Use style\'s', async () => {
    const s = setup()
    const sent = spy(s)
    const reverb = card('Reverb')
    expect(reverb.textContent).not.toContain('Set by rack')
    expect(reverb.querySelector('[data-tip="fx.send_use_style"]')).toBeNull()
    await fireEvent.click(toggle(reverb, 'Rack keeps type'))
    expect(sent).toHaveBeenLastCalledWith({ type: 'setRackSendOverride', send: 0, on: true })
    flushSync()
    expect(toggle(reverb, 'Rack keeps type').getAttribute('aria-checked')).toBe('true')
    expect(reverb.textContent).toContain('Set by rack: Hall')
    expect(card('Chorus').textContent).not.toContain('Set by rack')

    s.send({ type: 'setRackSendOverride', send: 2, on: true })
    flushSync()
    const delay = card('Delay')
    expect(delay.textContent).toContain(`Set by rack: ${s.state.effects.sends[2].name}`)
    await fireEvent.click(delay.querySelector<HTMLElement>('[data-tip="fx.send_use_style"]')!)
    expect(sent).toHaveBeenLastCalledWith({ type: 'setRackSendOverride', send: 2, on: false })
    flushSync()
    expect(delay.textContent).not.toContain('Set by rack')
  })

  it('every send control has a tooltip', async () => {
    const s = setup()
    s.send({ type: 'setRackSendOverride', send: 1, on: true })
    await addSend('phaser')
    expect(untipped(document.body)).toEqual([])
    for (const e of document.querySelectorAll('select')) expect(isTipKey(e.getAttribute('data-tip') ?? '')).toBe(true)
  })
})
