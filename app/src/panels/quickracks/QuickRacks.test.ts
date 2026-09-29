// Quick Racks on the mock session (docs/racks.md item 6): the bar's buttons load racks and
// light like pad page 4, Store puts the live rack on a button (saving it first when it has
// to), the unsaved-changes guard asks, banks step, Clear empties; Library › Racks, page 4's pads
// and Shift + Track run the same commands.

import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import App from '../../App.svelte'
import { BINDINGS } from '../../lib/keys'
import { MockSession } from '../../lib/api/mock'
import { quickLook } from '../../lib/api/quick-racks'
import { app, ui } from '../../lib/store.svelte'
import Launchkey from '../launchkey/Launchkey.svelte'
import QuickBar from './QuickBar.svelte'

function setup() {
  const session = new MockSession({ manual: true })
  app.attach(session)
  return session
}

const q = <T extends Element = HTMLButtonElement>(sel: string) => document.querySelector<T>(sel)!
const tipped = <T extends Element = HTMLButtonElement>(key: string) => [...document.querySelectorAll<T>(`[data-tip="${key}"]`)]
const names = () => [...document.querySelectorAll('.qbar .bname .txt')].map((e) => e.textContent)

async function click(el: Element) {
  await fireEvent.click(el)
  flushSync()
}

/** Store the live rack on button `slot` as a new rack `name`, through the bar. */
async function storeAs(slot: number, name: string) {
  await click(tipped('quick.store')[0])
  await click(tipped(`quick.${slot + 1}`)[0])
  const input = q<HTMLInputElement>('[data-tip="quick.save_name"]')
  await fireEvent.input(input, { target: { value: name } })
  await click(tipped('quick.save')[0])
}

afterEach(() => {
  cleanup()
  app.detach()
  ui.view = 'stage'
})

describe('Quick Racks bar', () => {
  it('replaces the Registration bar in the keyboard strip\'s panel, above the keys', () => {
    render(App, { props: { session: new MockSession({ demo: true, manual: true }) } })
    flushSync()
    const strip = q<HTMLElement>('section[aria-label="Keyboard"]')
    const bar = strip.querySelector('section[aria-label="Quick Racks"]')!
    expect(bar).toBeTruthy()
    expect(document.querySelector('section[aria-label="Registration"]')).toBeNull()
    expect(bar.compareDocumentPosition(strip.querySelector('[data-tip="keystrip.keys"]')!) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    // Bank ◀ ▶, eight buttons and Store; no Freeze, Sequence, Playlist or Panel.
    expect(bar.querySelectorAll('[data-tip^="quick."]').length).toBeGreaterThanOrEqual(11)
    expect(bar.textContent).not.toMatch(/Freeze|Sequence|Playlist|Panel/)
  })

  it('starts on bank A with eight empty buttons', () => {
    setup()
    render(QuickBar)
    expect(q('.qbar .letter').textContent).toBe('A')
    expect(names()).toEqual(Array(8).fill('empty'))
    expect(document.querySelectorAll('.qbar .clear')).toHaveLength(0)
  })

  it('Store on a never-saved rack asks its name, saves it and stores it on the button', async () => {
    const s = setup()
    render(QuickBar)
    await click(tipped('quick.store')[0])
    expect(s.state.quickRacks.store).toBe(true)
    expect(quickLook(s.state.quickRacks, 5).anim).toBe('flash')
    await click(tipped('quick.3')[0])
    // Nothing stored yet: the button waits for the save, and the bar asks.
    expect(s.state.quickRacks.storeWaiting).toBe(2)
    expect(s.state.quickRacks.buttons[2].rack).toBeNull()
    expect(q('.qbar').textContent).toContain('Save the rack to store it on A3')
    const input = q<HTMLInputElement>('[data-tip="quick.save_name"]')
    expect(input.value).toBe('New rack')
    await fireEvent.input(input, { target: { value: 'Ballad' } })
    await click(tipped('quick.save')[0])
    const qr = s.state.quickRacks
    expect(s.state.racks.map((r) => r.name)).toEqual(['Ballad'])
    expect(qr).toMatchObject({ store: false, storeWaiting: null })
    expect(qr.buttons[2]).toMatchObject({ rack: s.state.racks[0].id, name: 'Ballad', loaded: true, missing: false })
    expect(quickLook(qr, 2)).toMatchObject({ rgb: [127, 0, 0], anim: 'solid' })
    expect(names()[2]).toBe('Ballad')
  })

  it('Store with a saved, unmodified rack stores at once; a modified one is saved first', async () => {
    const s = setup()
    render(QuickBar)
    await storeAs(0, 'Ballad')
    await click(tipped('quick.store')[0])
    await click(tipped('quick.2')[0])
    expect(s.state.quickRacks.buttons[1]).toMatchObject({ name: 'Ballad', loaded: true })
    expect(s.state.quickRacks.store).toBe(false)
    // Changed since: Save (over Ballad) first, then it goes on the button.
    s.send({ type: 'setPartVoice', part: 0, program: 12 })
    flushSync()
    await click(tipped('quick.store')[0])
    await click(tipped('quick.4')[0])
    expect(s.state.quickRacks.storeWaiting).toBe(3)
    expect(q('.qbar').textContent).toContain('Save Ballad to store it on A4')
    await click(tipped('quick.save')[0])
    expect(s.state.liveRack).toMatchObject({ name: 'Ballad', modified: false })
    expect(s.state.racks).toHaveLength(1)
    expect(s.state.quickRacks.buttons[3].name).toBe('Ballad')
  })

  it('Cancel while waiting disarms Store and stores nothing', async () => {
    const s = setup()
    render(QuickBar)
    await click(tipped('quick.store')[0])
    await click(tipped('quick.1')[0])
    await click(tipped('quick.cancel_store')[0])
    expect(s.state.quickRacks).toMatchObject({ store: false, storeWaiting: null })
    expect(s.state.racks).toHaveLength(0)
    expect(tipped('quick.1')).toHaveLength(1)
  })

  it('a waiting Store asks names for edited presets, then saves and stores', async () => {
    const s = setup()
    render(QuickBar)
    s.send({ type: 'listPluginPresets', id: 'au:aumu Smp7 Fake' })
    s.send({ type: 'setPartPluginPreset', part: 0, id: 'aumu Smp7 Fake', preset: 'f:1' })
    s.advance(5000)
    s.pluginWindow(0, 5)
    flushSync()
    await storeAs(0, 'Grand')
    expect(s.state.liveRack.prompt).toMatchObject({ kind: 'soundNames', saveAs: 'Grand' })
    expect(s.state.quickRacks.storeWaiting).toBe(0)
    const field = q<HTMLInputElement>('[data-tip="quick.sound_name"]')
    expect(field.value).toBe('Bright Grand')
    await fireEvent.input(field, { target: { value: 'My Grand' } })
    await click(tipped('quick.sound_names_save')[0])
    expect(s.state.liveRack).toMatchObject({ name: 'Grand', modified: false, prompt: null })
    expect(s.state.keyboardParts[0].sound?.name).toBe('My Grand')
    expect(s.state.quickRacks.buttons[0]).toMatchObject({ name: 'Grand', loaded: true })
  })

  it('a button loads its rack; with unsaved changes it asks: Keep editing, Discard and switch, Save first', async () => {
    const s = setup()
    render(QuickBar)
    s.send({ type: 'setPartVoice', part: 0, program: 40 })
    await storeAs(0, 'Strings')
    const strings = s.state.liveRack.id!
    s.send({ type: 'newRack' })
    flushSync()
    expect(s.state.quickRacks.buttons[0].loaded).toBe(false)
    await click(tipped('quick.1')[0])
    expect(s.state.liveRack.id).toBe(strings)
    expect(s.state.keyboardParts[0].program).toBe(40)
    expect(s.state.quickRacks.buttons[0].loaded).toBe(true)

    s.send({ type: 'newRack' })
    s.send({ type: 'setPartVoice', part: 0, program: 5 })
    flushSync()
    await click(tipped('quick.1')[0])
    expect(s.state.liveRack.prompt).toEqual({ kind: 'unsavedChanges', then: { kind: 'load', id: strings, name: 'Strings' } })
    expect(q('.qbar').textContent).toContain('Switch to Strings?')
    await click(tipped('quick.keep_editing')[0])
    expect(s.state.liveRack).toMatchObject({ id: null, modified: true, prompt: null })

    await click(tipped('quick.1')[0])
    await click(tipped('quick.discard')[0])
    expect(s.state.liveRack).toMatchObject({ id: strings, modified: false, prompt: null })
    expect(s.state.racks).toHaveLength(1)

    s.send({ type: 'newRack' })
    s.send({ type: 'setPartVoice', part: 0, program: 7 })
    flushSync()
    await click(tipped('quick.1')[0])
    await click(tipped('quick.save_first')[0])
    // The new rack is saved (as "New rack"), then the switch goes ahead.
    expect(s.state.racks.map((r) => r.name)).toEqual(['New rack', 'Strings'])
    expect(s.state.liveRack).toMatchObject({ id: strings, modified: false, prompt: null })
    expect(s.state.keyboardParts[0].program).toBe(40)
  })

  it('an empty button says so; Bank −/+ step A–H and stop at the ends', async () => {
    const s = setup()
    render(QuickBar)
    await click(tipped('quick.5')[0])
    expect(s.state.message).toMatchObject({ error: true, text: 'Quick Rack A5 is empty' })
    await click(tipped('quick.bank_prev')[0])
    expect(s.state.quickRacks.bank).toBe(0)
    await click(tipped('quick.bank_next')[0])
    expect(s.state.quickRacks.bank).toBe(1)
    expect(q('.qbar .letter').textContent).toBe('B')
    for (let i = 0; i < 9; i++) await click(tipped('quick.bank_next')[0])
    expect(s.state.quickRacks.bank).toBe(7)
    expect(q('.qbar .letter').textContent).toBe('H')
  })

  it('each bank has its own buttons, and ✕ clears one', async () => {
    const s = setup()
    render(QuickBar)
    await storeAs(0, 'Ballad')
    await click(tipped('quick.bank_next')[0])
    expect(s.state.quickRacks.buttons[0].rack).toBeNull()
    await click(tipped('quick.store')[0])
    await click(tipped('quick.8')[0])
    expect(s.state.quickRacks.buttons[7].name).toBe('Ballad')
    await click(tipped('quick.bank_prev')[0])
    expect(names()[0]).toBe('Ballad')
    await click(q('.qbar .clear'))
    expect(s.state.quickRacks.buttons[0].rack).toBeNull()
    await click(tipped('quick.bank_next')[0])
    expect(s.state.quickRacks.buttons[7].name).toBe('Ballad')
  })
})

describe('Quick Racks in the mock', () => {
  function withRacks() {
    const s = new MockSession({ manual: true })
    const store = (slot: number, name: string, program: number) => {
      s.send({ type: 'setPartVoice', part: 0, program })
      s.send({ type: 'toggleQuickRackStore' })
      s.send({ type: 'pressQuickRack', slot })
      s.send({ type: 'saveRackAs', name })
    }
    store(1, 'One', 10)
    store(4, 'Two', 20)
    s.send({ type: 'newRack' })
    return s
  }

  it('Rack −/+ step the stored buttons from the lit one, and stop at the ends', () => {
    const s = withRacks()
    const lit = () => s.state.quickRacks.buttons.findIndex((b) => b.loaded)
    s.send({ type: 'stepQuickRack', delta: 1 })
    expect(lit()).toBe(1)
    s.send({ type: 'stepQuickRack', delta: 1 })
    expect(lit()).toBe(4)
    s.send({ type: 'stepQuickRack', delta: 1 })
    expect(lit()).toBe(4)
    s.send({ type: 'stepQuickRack', delta: -1 })
    expect(lit()).toBe(1)
    expect(s.state.keyboardParts[0].program).toBe(10)
    s.send({ type: 'newRack' })
    s.send({ type: 'stepQuickRack', delta: -1 })
    expect(lit()).toBe(4)
    s.send({ type: 'stepQuickRackBank', delta: 1 })
    s.send({ type: 'stepQuickRack', delta: 1 })
    expect(s.state.message).toMatchObject({ error: true, text: 'Bank B has no racks' })
  })

  it('Regist 9 and 10 run on into the next bank; the pedal functions run Quick Racks', () => {
    const s = withRacks()
    s.send({ type: 'toggleQuickRackStore' })
    s.send({ type: 'pressQuickRack', slot: 9 })
    s.send({ type: 'saveRackAs', name: 'Three' })
    s.send({ type: 'stepQuickRackBank', delta: 1 })
    expect(s.state.quickRacks.buttons[1].name).toBe('Three')
    s.send({ type: 'stepQuickRackBank', delta: -1 })
    s.send({ type: 'triggerFunction', function: 'regist2' })
    expect(s.state.liveRack.name).toBe('One')
    s.send({ type: 'triggerFunction', function: 'registNext' })
    expect(s.state.liveRack.name).toBe('Two')
    s.send({ type: 'triggerFunction', function: 'registMemory' })
    expect(s.state.quickRacks.store).toBe(true)
    s.send({ type: 'triggerFunction', function: 'snapshotBankNext' })
    expect(s.state.quickRacks.bank).toBe(1)
  })

  it('deleting a rack empties its buttons; dismissing the prompt lets a waiting Store go', () => {
    const s = withRacks()
    const one = s.state.quickRacks.buttons[1].rack!
    s.send({ type: 'deleteRack', id: one })
    expect(s.state.quickRacks.buttons[1].rack).toBeNull()
    expect(s.state.quickRacks.buttons[4].name).toBe('Two')
    s.send({ type: 'toggleQuickRackStore' })
    s.send({ type: 'pressQuickRack', slot: 0 })
    expect(s.state.quickRacks.storeWaiting).toBe(0)
    s.send({ type: 'dismissRackPrompt' })
    expect(s.state.quickRacks.storeWaiting).toBeNull()
  })

  it('pad page 4: Quick 1–8, Bank −/+, Store, Rack −/+ and dark pads, with page 4 lamps', () => {
    const s = new MockSession({ manual: true })
    s.send({ type: 'setPadPage', page: 'quickRacks' })
    const pad = (n: number) => s.state.pads.pads.find((p) => p.note === n)!
    expect(s.state.pads.pageName).toBe('Quick Racks')
    expect([pad(96).label, pad(96).key, pad(96).action]).toEqual(['QUICK 1', '⇧Q', { type: 'pressQuickRack', slot: 0 }])
    expect([pad(103).label, pad(103).key]).toEqual(['QUICK 8', '⇧I'])
    expect([pad(112).action, pad(112).level, pad(113).level]).toEqual([{ type: 'stepQuickRackBank', delta: -1 }, 'off', 'dim'])
    for (const n of [114, 115, 117]) expect([pad(n).label, pad(n).key, pad(n).action, pad(n).level]).toEqual(['', '', null, 'off'])
    expect([pad(116).label, pad(116).key, pad(116).action]).toEqual(['STORE', 'F5', { type: 'toggleQuickRackStore' }])
    expect([pad(118).label, pad(118).key, pad(118).action, pad(118).level]).toEqual(['RACK -', 'F7', { type: 'stepQuickRack', delta: -1 }, 'off'])
    expect([pad(119).label, pad(119).key, pad(119).action]).toEqual(['RACK +', 'F8', { type: 'stepQuickRack', delta: 1 }])
    s.send({ type: 'toggleQuickRackStore' })
    expect([pad(96).anim, pad(116).anim]).toEqual(['flash', 'flash'])
    s.send({ type: 'pressQuickRack', slot: 0 })
    s.send({ type: 'saveRackAs', name: 'Ballad' })
    s.send({ type: 'newRack' })
    expect([pad(96).rgb, pad(96).level, pad(97).level, pad(118).level]).toEqual([[0, 40, 127], 'bright', 'off', 'dim'])
    s.send({ type: 'pressQuickRack', slot: 0 })
    expect([pad(96).rgb, pad(96).level]).toEqual([[127, 0, 0], 'bright'])
    for (let i = 0; i < 7; i++) s.send({ type: 'stepQuickRackBank', delta: 1 })
    expect([pad(112).level, pad(113).level]).toEqual(['dim', 'off'])
  })

  it('Shift + Track is the previous/next Quick Rack, dark while the bank has none', () => {
    const s = new MockSession({ manual: true })
    const c = (id: string) => s.state.surface.controls.find((x) => x.id === id)!
    expect([c('trackPrev').shiftLabel, c('trackPrev').shiftAction]).toEqual(['', null])
    s.send({ type: 'toggleQuickRackStore' })
    s.send({ type: 'pressQuickRack', slot: 5 })
    s.send({ type: 'saveRackAs', name: 'Ballad' })
    expect([c('trackPrev').shiftLabel, c('trackPrev').shiftAction]).toEqual(['◀ RACK', { type: 'stepQuickRack', delta: -1 }])
    expect([c('trackNext').shiftLabel, c('trackNext').shiftAction]).toEqual(['RACK ▶', { type: 'stepQuickRack', delta: 1 }])
  })

  it('the keys: Shift + Q–I press, O/P step the bank, F5 Store, F7/F8 Rack −/+', () => {
    expect(BINDINGS.Q).toEqual({ cmd: { type: 'pressQuickRack', slot: 0 } })
    expect(BINDINGS.I).toEqual({ cmd: { type: 'pressQuickRack', slot: 7 } })
    expect(BINDINGS.O).toEqual({ cmd: { type: 'stepQuickRackBank', delta: -1 } })
    expect(BINDINGS.P).toEqual({ cmd: { type: 'stepQuickRackBank', delta: 1 } })
    expect(BINDINGS.F5).toEqual({ cmd: { type: 'toggleQuickRackStore' } })
    expect(BINDINGS.F7).toEqual({ cmd: { type: 'stepQuickRack', delta: -1 } })
    expect(BINDINGS.F8).toEqual({ cmd: { type: 'stepQuickRack', delta: 1 } })
    for (const k of ['F6', 'F11', 'F12', '<', '>']) expect(BINDINGS[k], k).toBeUndefined()
  })

  it('a page 4 pad on the mirror loads its rack', async () => {
    const s = setup()
    s.send({ type: 'toggleQuickRackStore' })
    s.send({ type: 'pressQuickRack', slot: 2 })
    s.send({ type: 'saveRackAs', name: 'Organ' })
    s.send({ type: 'newRack' })
    s.send({ type: 'setPadPage', page: 'quickRacks' })
    flushSync()
    render(Launchkey)
    await click(q('.pad[data-note="98"]'))
    expect(s.state.liveRack.name).toBe('Organ')
    expect(s.state.quickRacks.buttons[2].loaded).toBe(true)
  })
})

describe('Library › Racks', () => {
  it('opens from nav "Quick Racks" and loads your racks, labelled with their Quick Rack button', async () => {
    const s = setup()
    render(App, { props: { session: s } })
    s.send({ type: 'toggleQuickRackStore' })
    s.send({ type: 'pressQuickRack', slot: 0 })
    s.send({ type: 'saveRackAs', name: 'Ballad' })
    s.send({ type: 'newRack' })
    s.send({ type: 'setPartVoice', part: 0, program: 3 })
    flushSync()
    const nav = [...document.querySelectorAll('nav.quick-nav button')].find((b) => b.textContent!.trim() === 'Quick Racks')!
    await click(nav)
    expect(ui.view).toBe('library')
    expect(ui.libraryTab).toBe('racks')
    const rack = tipped('quick.rack')
    expect(rack.map((b) => b.textContent)).toEqual([expect.stringContaining('Ballad')])
    expect(rack[0].textContent).toContain('A1')
    await click(rack[0])
    // The guard asks in Library's Quick Racks bar.
    expect(tipped('quick.discard')).toHaveLength(1)
    await click(tipped('quick.discard')[0])
    expect(s.state.liveRack).toMatchObject({ name: 'Ballad', modified: false })
  })
})
