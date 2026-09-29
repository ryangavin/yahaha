// The rack's channel strips and send effects (the mixer rework): each part slot's strip
// (delay and send 4–6 levels, compressor, insert chips with their settings popover), the
// rack's send effects (Override on 1–3; 4–6 added, typed, returned, removed) and the new
// controller-map targets. Each test checks the command sent and what the mock made of it.

import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { isTipKey } from '../../help/tooltips'
import { MockSession } from '../../lib/api/mock'
import type { AppCmd } from '../../lib/api/types'
import { app } from '../../lib/store.svelte'
import RackPanel from './RackPanel.svelte'
import { eqSummary } from './strip'

function setup() {
  const session = new MockSession({ manual: true, demo: true })
  app.attach(session)
  flushSync()
  render(RackPanel, { props: { docked: true } })
  session.advance(16) // the first publish: what the live rack holds from here
  flushSync()
  const sent: AppCmd[] = []
  const orig = session.send.bind(session)
  session.send = (c) => (sent.push(c), orig(c))
  return { session, sent }
}

afterEach(() => {
  cleanup()
  app.detach()
})

const slot = (name: string) => document.querySelector<HTMLElement>(`.slot[aria-label="${name}"]`)!
const inSlot = (name: string, key: string) => slot(name).querySelector<HTMLElement>(`[data-tip="${key}"]`)!
const allInSlot = (name: string, key: string) => [...slot(name).querySelectorAll<HTMLElement>(`[data-tip="${key}"]`)]
const all = (key: string) => [...document.querySelectorAll<HTMLElement>(`[data-tip="${key}"]`)]
const head = () => document.querySelector('.rackhead')!.textContent!.replace(/\s+/g, ' ')

async function pick(el: HTMLElement, value: string) {
  const s = el as HTMLSelectElement
  s.value = value
  await fireEvent.change(s)
  flushSync()
}

describe('Rack slot: the part\'s strip', () => {
  it('an insert chip opens its settings; a new type sends setStripInsertKind and its settings show', async () => {
    const { session, sent } = setup()
    const chips = allInSlot('Right 2', 'rack.insert_chip')
    expect(chips.map((c) => c.textContent)).toEqual(['None', 'None'])
    expect(slot('Right 2').querySelector('[role="dialog"]')).toBeNull()
    await fireEvent.click(chips[1])
    const dialog = slot('Right 2').querySelector<HTMLElement>('[role="dialog"]')!
    expect(dialog.getAttribute('aria-label')).toBe('Right 2 insert 2 settings')
    await pick(dialog.querySelector<HTMLElement>('[data-tip="mixer.strip.insert_kind"]')!, 'phaser')
    expect(sent).toContainEqual({ type: 'setStripInsertKind', strip: 1, slot: 1, kind: 'phaser' })
    expect(session.state.keyboardParts[1].strip.inserts[1].kind).toBe('phaser')
    expect(allInSlot('Right 2', 'rack.insert_chip')[1].textContent).toBe('Phaser')
    // Its settings, one slider each; moving one sends setStripInsertSetting.
    const first = inSlot('Right 2', 'mixer.strip.insert_setting_1')
    expect(first).toBeTruthy()
    await fireEvent.keyDown(first, { key: 'End' })
    const max = session.state.keyboardParts[1].strip.inserts[1].settings[0].max
    expect(sent).toContainEqual({ type: 'setStripInsertSetting', strip: 1, slot: 1, setting: 0, value: max })
    expect(session.state.keyboardParts[1].strip.inserts[1].settings[0].value).toBe(max)
    // The lamp turns the slot on; Done closes the popover.
    await fireEvent.click(allInSlot('Right 2', 'mixer.strip.insert_on')[1])
    expect(sent).toContainEqual({ type: 'setStripInsertOn', strip: 1, slot: 1, on: true })
    expect(session.state.keyboardParts[1].strip.inserts[1].on).toBe(true)
    await fireEvent.click(inSlot('Right 2', 'rack.insert_close'))
    expect(slot('Right 2').querySelector('[role="dialog"]')).toBeNull()
  })

  it('the delay send sends setStripSend 3 and marks the rack modified', async () => {
    const { session, sent } = setup()
    expect(head()).not.toContain('modified')
    await fireEvent.keyDown(inSlot('Left', 'mixer.part.variation'), { key: 'End' })
    flushSync()
    expect(sent).toContainEqual({ type: 'setStripSend', strip: 3, send: 2, level: 127 })
    expect(session.state.keyboardParts[3].variation).toBe(127)
    expect(session.state.keyboardParts[3].strip.sends[2]).toBe(127)
    session.advance(16)
    flushSync()
    expect(head()).toContain('● modified')
  })

  it('the compressor turns on and takes a type', async () => {
    const { session, sent } = setup()
    await fireEvent.click(inSlot('Right 1', 'mixer.strip.comp'))
    expect(sent).toContainEqual({ type: 'setStripCompressorOn', strip: 0, on: true })
    await pick(inSlot('Right 1', 'mixer.strip.comp_type'), 'punchy')
    expect(sent).toContainEqual({ type: 'setStripCompressorPreset', strip: 0, preset: 'punchy' })
    expect(session.state.keyboardParts[0].strip.comp).toMatchObject({ on: true, preset: 'punchy' })
  })

  it('words the EQ', () => {
    expect(eqSummary({ lowGain: 0, lowFreq: 80, highGain: 0, highFreq: 10000 })).toBe('EQ flat')
    expect(eqSummary({ lowGain: 3, lowFreq: 80, highGain: -2, highFreq: 10000 })).toBe('EQ Lo +3 @80 · Hi −2 @10k')
  })
})

describe('Rack panel: send effects', () => {
  it('Add send sends addSend; every slot gets a level to it, and it can be retyped, returned and removed', async () => {
    const { session, sent } = setup()
    expect(all('fx.send_remove')).toHaveLength(0)
    expect(allInSlot('Right 1', 'mixer.strip.send')).toHaveLength(0)
    const kind = document.querySelector<HTMLElement>('select[aria-label="New send type"]')!
    await pick(kind, 'phaser')
    await fireEvent.click(all('fx.send_add')[0])
    flushSync()
    expect(sent).toContainEqual({ type: 'addSend', kind: 'phaser' })
    expect(session.state.effects.sends.map((x) => x.name)).toEqual(['Hall', 'Chorus', 'Delay 1/8.', 'Phaser'])
    expect(all('fx.send_remove')).toHaveLength(1)
    // The part's level to send 4.
    const s4 = inSlot('Right 1', 'mixer.strip.send')
    expect(s4.getAttribute('aria-label')).toContain('send 4')
    await fireEvent.keyDown(s4, { key: 'End' })
    expect(sent).toContainEqual({ type: 'setStripSend', strip: 0, send: 3, level: 127 })
    expect(session.state.keyboardParts[0].strip.sends[3]).toBe(127)
    // Its type and return.
    await pick(document.querySelector<HTMLElement>('select[aria-label="Send 4 type"]')!, 'plate')
    expect(sent).toContainEqual({ type: 'setSendKind', send: 3, kind: 'plate' })
    expect(session.state.effects.sends[3].kind).toBe('plate')
    await fireEvent.keyDown(all('fx.send_return')[0], { key: 'Home' })
    expect(sent).toContainEqual({ type: 'setSendReturn', send: 3, level: 0 })
    // Remove.
    await fireEvent.click(all('fx.send_remove')[0])
    flushSync()
    expect(sent).toContainEqual({ type: 'removeSend', send: 3 })
    expect(session.state.effects.sends).toHaveLength(3)
    expect(allInSlot('Right 1', 'mixer.strip.send')).toHaveLength(0)
  })

  it('Add send is gone with six sends', async () => {
    const { session } = setup()
    for (const kind of ['hall', 'room', 'phaser']) session.send({ type: 'addSend', kind })
    session.advance(16)
    flushSync()
    expect(session.state.effects.sends).toHaveLength(6)
    expect(all('fx.send_add')).toHaveLength(0)
    expect(allInSlot('Left', 'mixer.strip.send')).toHaveLength(3)
  })

  it('Override on sends 1–3 sends setRackSendOverride and lights while the rack sets the type', async () => {
    const { session, sent } = setup()
    const toggles = all('fx.send_rack_override')
    expect(toggles).toHaveLength(3)
    expect(toggles[1].getAttribute('aria-checked')).toBe('false')
    await fireEvent.click(toggles[1])
    flushSync()
    expect(sent).toContainEqual({ type: 'setRackSendOverride', send: 1, on: true })
    expect(session.state.effects.sends[1].setByRack).toBe(true)
    expect(all('fx.send_rack_override')[1].getAttribute('aria-checked')).toBe('true')
    await fireEvent.click(all('fx.send_rack_override')[1])
    expect(sent).toContainEqual({ type: 'setRackSendOverride', send: 1, on: false })
  })

  it('every strip and send control, popover open and a send added, has a tooltip', async () => {
    const { session } = setup()
    session.send({ type: 'addSend', kind: 'hall' })
    session.send({ type: 'setStripInsertKind', strip: 0, slot: 0, kind: 'rotary' })
    session.advance(16)
    flushSync()
    await fireEvent.click(allInSlot('Right 1', 'rack.insert_chip')[0])
    expect(allInSlot('Right 1', 'mixer.strip.insert_setting_2').length).toBe(1)
    const controls = [...document.body.querySelectorAll('button, select, input, [role="slider"], [role="switch"], [tabindex]:not([tabindex="-1"])')]
    expect(controls.filter((el) => !isTipKey(el.getAttribute('data-tip') ?? '')).map((el) => el.outerHTML.slice(0, 120))).toEqual([])
  })
})

describe('Rack panel: the controller map\'s new targets', () => {
  it('lists delay, inserts, sends 4–6 and the rotary speed, by part, and sets one', async () => {
    const { session, sent } = setup()
    await fireEvent.click(all('rack.map')[0])
    const select = document.querySelector<HTMLSelectElement>('table.map tbody select')!
    const labels = [...select.options].map((o) => o.textContent)
    for (const l of ['Right 1 delay', 'Right 1 insert 1 on/off', 'Right 1 insert 2 setting 4', 'Left send 6', 'Rotary fast/slow']) expect(labels).toContain(l)
    expect([...select.querySelectorAll('optgroup')].map((g) => g.label)).toEqual(['Right 1', 'Right 2', 'Right 3', 'Left', 'Rack'])
    await pick(select, 'partInsertSetting:2:1:3')
    expect(sent).toContainEqual({ type: 'setRackControl', control: 'fader', index: 0, target: { kind: 'partInsertSetting', part: 2, slot: 1, setting: 3 } })
    expect(session.state.liveRack.controls.faders[0]).toEqual({ kind: 'partInsertSetting', part: 2, slot: 1, setting: 3 })
    expect(select.selectedOptions[0].textContent).toBe('Right 3 insert 2 setting 4')
  })
})
