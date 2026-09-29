import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { isTipKey } from '../../help/tooltips'
import { MockSession } from '../../lib/api/mock'
import type { AppCmd, PartPlugin, PluginEntry } from '../../lib/api/types'
import { mirror } from '../../lib/mirror.svelte'
import { app, ui } from '../../lib/store.svelte'
import Launchkey from '../launchkey/Launchkey.svelte'
import { pluginBadge, pluginTip } from '../mixer/voice'
import { inProcessPending, pluginStatusLine } from '../parts/parts'
import { rackName, soundBadge, soundLabel, targetLabel } from './rack'
import RackPanel from './RackPanel.svelte'

function setup(demo = true, props: { docked?: boolean } = {}) {
  const session = new MockSession({ manual: true, demo })
  app.attach(session)
  flushSync()
  const r = render(RackPanel, { props })
  return { session, ...r }
}

/** Every command the panel sends, by type. */
function spy(session: MockSession): AppCmd['type'][] {
  const sent: AppCmd['type'][] = []
  const orig = session.send.bind(session)
  session.send = (c) => (sent.push(c.type), orig(c))
  return sent
}

afterEach(() => {
  cleanup()
  app.detach()
  mirror.panelFader = null
  ui.view = 'stage'
  ui.libraryPart = 0
  ui.rack = false
})

const tipped = (key: string) => document.querySelector<HTMLElement>(`[data-tip="${key}"]`)!
const slot = (name: string) => document.querySelector<HTMLElement>(`.slot[aria-label="${name}"]`)!
const inSlot = (name: string, key: string) => slot(name).querySelector<HTMLElement>(`[data-tip="${key}"]`)!
const head = () => document.querySelector('.rackhead')!.textContent!.replace(/\s+/g, ' ')

describe('Rack panel: the head', () => {
  it('names the live rack, says when it is modified, and has no Mine badge until saved', () => {
    const { session } = setup()
    session.advance(16) // the first publish: what the live rack holds from here
    flushSync()
    expect(head()).toContain('Untitled rack')
    expect(head()).not.toContain('Mine')
    expect(head()).not.toContain('modified')
    session.send({ type: 'setPartVolume', part: 0, volume: 12 })
    flushSync()
    expect(head()).toContain('● modified')
    // A rack loaded from a saved one (the rack commands set this).
    session.state.liveRack = { ...session.state.liveRack, name: 'Soul Ballad', id: 'r1', modified: false }
    session.advance(16)
    flushSync()
    expect(head()).toContain('Soul Ballad')
    expect(head()).toContain('Mine')
    expect(head()).not.toContain('modified')
  })

  /** A live rack saved as `name`, then changed. */
  async function savedAndChanged(session: MockSession, name = 'Ballad') {
    session.advance(16)
    session.send({ type: 'setPartVolume', part: 0, volume: 30 })
    session.send({ type: 'saveRackAs', name })
    session.advance(16)
    session.send({ type: 'setPartVolume', part: 0, volume: 99 })
    session.advance(16)
    flushSync()
  }

  it('Save rack saves over the rack; off when nothing changed; Revert only while modified', async () => {
    const { session } = setup()
    await savedAndChanged(session)
    expect(head()).toContain('Mine')
    expect(tipped('rack.save').getAttribute('aria-disabled')).toBe('false')
    await fireEvent.click(tipped('rack.revert'))
    session.advance(16)
    flushSync()
    expect(session.state.keyboardParts[0].volume).toBe(30)
    expect(tipped('rack.revert')).toBeNull()
    expect(tipped('rack.save').getAttribute('aria-disabled')).toBe('true')
    const sent = spy(session)
    await fireEvent.click(tipped('rack.save'))
    expect(sent).toEqual([])
    session.send({ type: 'setPartVolume', part: 0, volume: 50 })
    session.advance(16)
    flushSync()
    await fireEvent.click(tipped('rack.save'))
    session.advance(16)
    flushSync()
    expect(sent).toEqual(['setPartVolume', 'saveRack'])
    expect(session.state.liveRack).toMatchObject({ name: 'Ballad', modified: false })
    expect(session.state.racks).toHaveLength(1)
  })

  it('Save as… asks for a rack name and saves a new rack under it', async () => {
    const { session } = setup()
    await savedAndChanged(session)
    await fireEvent.click(tipped('rack.save_as'))
    const input = tipped('rack.save_as_name') as HTMLInputElement
    expect(input.value).toBe('Ballad copy')
    await fireEvent.input(input, { target: { value: 'Sunday Gospel' } })
    await fireEvent.click(tipped('rack.save_as_commit'))
    session.advance(16)
    flushSync()
    expect(session.state.racks.map((r) => r.name)).toEqual(['Ballad', 'Sunday Gospel'])
    expect(head()).toContain('Sunday Gospel')
    expect(tipped('rack.save_as_name')).toBeNull()
  })

  it('Save as… stays open on a taken name: the error shows inline and the name is kept', async () => {
    const { session } = setup()
    await savedAndChanged(session)
    await fireEvent.click(tipped('rack.save_as'))
    const input = tipped('rack.save_as_name') as HTMLInputElement
    await fireEvent.input(input, { target: { value: 'Ballad' } })
    await fireEvent.click(tipped('rack.save_as_commit'))
    session.advance(16)
    flushSync()
    expect(session.state.racks.map((r) => r.name)).toEqual(['Ballad'])
    expect((tipped('rack.save_as_name') as HTMLInputElement).value).toBe('Ballad')
    expect(document.querySelector('#rack-saveas-error')!.textContent).toContain('there is a rack called Ballad already')
    // A free name then saves and closes the form.
    await fireEvent.input(tipped('rack.save_as_name'), { target: { value: 'Ballad 2' } })
    await fireEvent.click(tipped('rack.save_as_commit'))
    session.advance(16)
    flushSync()
    expect(session.state.racks.map((r) => r.name)).toEqual(['Ballad', 'Ballad 2'])
    expect(tipped('rack.save_as_name')).toBeNull()
  })

  it('edited presets: a name field per part, prefilled, and the save is sent again with the names', async () => {
    const { session } = setup(false)
    session.send({ type: 'listPluginPresets', id: 'au:aumu Smp7 Fake' })
    session.send({ type: 'setPartPluginPreset', part: 0, id: 'aumu Smp7 Fake', preset: 'f:1' })
    session.advance(5000)
    session.pluginWindow(0, 1)
    flushSync()
    const suggested = session.state.keyboardParts[0].sound?.name ?? session.state.keyboardParts[0].voiceName
    const sent: AppCmd[] = []
    const orig = session.send.bind(session)
    session.send = (c) => (sent.push(c), orig(c))
    await fireEvent.click(tipped('rack.save'))
    session.advance(16)
    flushSync()
    expect(session.state.liveRack.prompt?.kind).toBe('soundNames')
    const input = document.querySelector<HTMLInputElement>('#rack-sn-0')!
    expect(document.querySelector('.form.names')!.textContent).toContain('Right 1 sound')
    expect(input.value).toBe(suggested)
    await fireEvent.input(input, { target: { value: 'My Keys' } })
    // A state snapshot (a new prompt object, same prompt) arrives while typing: the name stays.
    session.advance(16)
    flushSync()
    expect(input.value).toBe('My Keys')
    await fireEvent.click(tipped('rack.save_names'))
    session.advance(1000)
    flushSync()
    expect(sent.at(-1)).toEqual({ type: 'saveRack', soundNames: { 0: 'My Keys' } })
    expect(session.state.liveRack).toMatchObject({ modified: false, prompt: null })
    expect(document.querySelector('.form.names')).toBeNull()
  })

  it('unsaved changes: Keep editing, Discard and switch, and Save first then switch', async () => {
    const { session } = setup()
    await savedAndChanged(session)
    // Whatever asks (Library › Racks, the hardware), the panel shows the prompt.
    session.send({ type: 'newRack' })
    session.advance(16)
    flushSync()
    expect(document.querySelector('.form.unsaved')!.textContent).toContain('unsaved changes')
    await fireEvent.click(tipped('rack.keep_editing'))
    session.advance(16)
    flushSync()
    expect(document.querySelector('.form.unsaved')).toBeNull()
    expect(session.state.keyboardParts[0].volume).toBe(99)

    session.send({ type: 'newRack' })
    session.advance(16)
    flushSync()
    await fireEvent.click(tipped('rack.save_first'))
    session.advance(16)
    flushSync()
    session.advance(16)
    flushSync()
    expect(session.state.racks.find((r) => r.name === 'Ballad')).toBeTruthy()
    expect(session.state.liveRack).toMatchObject({ id: null, modified: false, prompt: null })

    const id = session.state.racks[0].id
    session.send({ type: 'setPartVolume', part: 0, volume: 7 })
    session.send({ type: 'loadRack', id })
    session.advance(16)
    flushSync()
    expect(document.querySelector('.form.unsaved')!.textContent).toContain('before loading Ballad')
    await fireEvent.click(tipped('rack.discard_switch'))
    session.advance(16)
    flushSync()
    expect(session.state.liveRack).toMatchObject({ name: 'Ballad', id, modified: false, prompt: null })
    expect(session.state.keyboardParts[0].volume).toBe(99)
  })

  it('Save first: the engine makes the switch, once; the panel sends only the save', async () => {
    const { session } = setup()
    await savedAndChanged(session)
    session.send({ type: 'newRack' })
    session.advance(16)
    flushSync()
    const sent: AppCmd[] = []
    const orig = session.send.bind(session)
    session.send = (c) => (sent.push(c), orig(c))
    await fireEvent.click(tipped('rack.save_first'))
    for (let i = 0; i < 4; i++) {
      session.advance(16)
      flushSync()
    }
    expect(sent).toEqual([{ type: 'saveRack' }])
    expect(session.state.liveRack).toMatchObject({ name: 'New rack', id: null, modified: false, prompt: null })
  })

  it('a Save first whose save fails does not switch later', async () => {
    const { session } = setup()
    await savedAndChanged(session)
    // Save as… a taken name while the unsaved-changes prompt is up: refused.
    session.send({ type: 'newRack' })
    session.send({ type: 'saveRackAs', name: 'Ballad' })
    session.advance(16)
    flushSync()
    expect(session.state.liveRack).toMatchObject({ name: 'Ballad', modified: true })
    // A later save saves, and stays on Ballad.
    await fireEvent.click(tipped('rack.save'))
    session.advance(16)
    flushSync()
    session.advance(16)
    flushSync()
    expect(session.state.liveRack).toMatchObject({ name: 'Ballad', modified: false, prompt: null })
    expect(session.state.keyboardParts[0].volume).toBe(99)
  })
})

describe('Rack panel: your hands', () => {
  it('moves the split and the keyboard transpose, and picks the Harmony/Arp type', async () => {
    const { session } = setup()
    const split = session.state.chord.split
    await fireEvent.click(tipped('split.up'))
    expect(session.state.chord.split).toBe(split + 1)
    await fireEvent.click(tipped('transpose.keyboard_down'))
    flushSync()
    expect(session.state.chord.transposeKeyboard).toBe(-1)
    expect(tipped('transpose.display').textContent).toBe('−1')
    const select = tipped('rack.harmony_type') as HTMLSelectElement
    await fireEvent.change(select, { target: { value: 'h5' } })
    expect(session.state.harmonyArp).toMatchObject({ mode: 'harmony', harmonyType: 5 })
    await fireEvent.change(select, { target: { value: 'a2' } })
    expect(session.state.harmonyArp).toMatchObject({ mode: 'arpeggio', arpPattern: 2 })
    flushSync()
    expect(select.value).toBe('a2')
    await fireEvent.click(tipped('harmony.switch'))
    expect(session.state.harmonyArp.on).toBe(true)
  })

  it('the controller map shows and sets what each fader and knob does for this rack', async () => {
    const { session } = setup()
    expect(document.querySelector('table.map')).toBeNull()
    await fireEvent.click(tipped('rack.map'))
    const selects = () => [...document.querySelectorAll<HTMLSelectElement>('table.map tbody select')]
    const shown = (s: HTMLSelectElement) => s.selectedOptions[0]?.textContent
    expect(selects()).toHaveLength(12)
    expect(selects()[0].getAttribute('aria-label')).toBe('Fader 1 target')
    expect(shown(selects()[0])).toBe('Right 1 level')
    expect(shown(selects()[7])).toBe('Left level')
    expect(shown(selects()[8])).toBe('Harmony volume')
    expect(shown(selects()[11])).toBe('Tempo')
    expect([...selects()[0].options].some((o) => o.value === 'tempo')).toBe(false)
    session.state.liveRack.controls.knobs[6] = { kind: 'splitPoint' }
    session.advance(16)
    flushSync()
    expect(shown(selects()[10])).toBe('Split point')
    // Picking a target sends setRackControl: the rack is modified, the knob follows.
    const k2 = selects()[5]
    k2.value = 'partPan:2'
    await fireEvent.change(k2)
    expect(session.state.liveRack.controls.knobs[1]).toEqual({ kind: 'partPan', part: 2 })
    expect(session.state.liveRack.modified).toBe(true)
    expect(shown(selects()[5])).toBe('Right 3 pan')
    expect(tipped('rack.map').getAttribute('aria-expanded')).toBe('true')
  })

  it('under Manual Bass, Left plays the Bass voice and the style Bass is marked', () => {
    const { session } = setup()
    const mb = tipped('detection.manual_bass')
    expect(mb.getAttribute('aria-checked')).toBe('false')
    session.send({ type: 'toggleUpper' })
    flushSync()
    expect(session.state.chord.manualBassActive).toBe(true)
    expect(mb.getAttribute('aria-checked')).toBe('true')
    expect(mb.parentElement!.textContent).toContain("Left plays the style's Bass")
    expect(slot('Left').textContent).toContain('Style Bass under Manual Bass')
    expect(document.querySelector('.written .muted')!.textContent).toContain('your left hand')
  })
})

describe('Rack panel: the part slots', () => {
  it('shows R1, R2, R3 and L with their sound, instrument, badge and mix', () => {
    setup()
    for (const name of ['Right 1', 'Right 2', 'Right 3', 'Left']) expect(slot(name)).toBeTruthy()
    expect(inSlot('Right 1', 'rack.sound').textContent).toContain('Stage Grand')
    // The demo's Right 1 plays a library sound of yours.
    expect(slot('Right 1').querySelector('.badge')!.textContent).toBe('Mine')
    expect(inSlot('Right 2', 'mixer.panel.right2').getAttribute('aria-valuenow')).toBe('72')
    expect(inSlot('Right 1', 'rack.part').getAttribute('aria-pressed')).toBe('true')
    expect(slot('Right 1').classList.contains('focus')).toBe(true)
  })

  it('clicking a slot makes it the part you edit; its sound opens Library › Sounds on it', async () => {
    const { session } = setup()
    await fireEvent.pointerDown(slot('Left'))
    expect(session.state.keyboardParts[3].selected).toBe(true)
    await fireEvent.click(inSlot('Right 2', 'rack.part'))
    expect(session.state.keyboardParts[1].selected).toBe(true)
    await fireEvent.click(inSlot('Right 3', 'rack.sound'))
    expect(ui.view).toBe('library')
    expect(ui.libraryTab).toBe('sounds')
    expect(ui.libraryPart).toBe(2)
  })

  it('sends the part commands: on, level, pan, octave, reverb, chorus, voice', async () => {
    const { session } = setup()
    await fireEvent.click(inSlot('Right 3', 'part.right3.on'))
    expect(session.state.keyboardParts[2].on).toBe(true)
    const level = inSlot('Right 1', 'mixer.panel.right1')
    const v = session.state.keyboardParts[0].volume
    await fireEvent.keyDown(level, { key: 'ArrowDown' })
    expect(session.state.keyboardParts[0].volume).toBe(v - 1)
    await fireEvent.keyDown(inSlot('Right 1', 'mixer.part.pan'), { key: 'PageDown' })
    expect(session.state.keyboardParts[0].pan).toBe(54)
    await fireEvent.click(inSlot('Right 1', 'part.octave_up'))
    expect(session.state.keyboardParts[0].octave).toBe(1)
    await fireEvent.keyDown(inSlot('Left', 'mixer.part.reverb'), { key: 'End' })
    expect(session.state.keyboardParts[3].reverb).toBe(127)
    await fireEvent.keyDown(inSlot('Left', 'mixer.part.chorus'), { key: 'Home' })
    expect(session.state.keyboardParts[3].chorus).toBe(0)
    const sent = spy(session)
    await fireEvent.click(inSlot('Right 2', 'part.voice_up'))
    expect(sent).toEqual(['selectPart', 'stepVoice'])
  })

  it('Edit opens the plugin window, and is off for a SoundFont sound', async () => {
    const { session } = setup()
    const open = vi.fn()
    session.pluginEditor = open
    expect(inSlot('Right 1', 'rack.edit').getAttribute('aria-disabled')).toBe('true')
    await fireEvent.click(inSlot('Right 1', 'rack.edit'))
    expect(open).not.toHaveBeenCalled()
    session.send({ type: 'setPartPlugin', part: 0, id: 'aumu dls  appl', state: null })
    session.advance(1000)
    flushSync()
    expect(inSlot('Right 1', 'rack.edit').getAttribute('aria-disabled')).toBe('false')
    await fireEvent.click(inSlot('Right 1', 'rack.edit'))
    expect(open).toHaveBeenCalledWith(0, true)
    expect(slot('Right 1').querySelector('.badge')!.textContent).toBe('Factory')
    expect(slot('Right 1').textContent).toContain('DLSMusicDevice')
  })

  it('an edited plugin sound says so and Save sound keeps the edit', async () => {
    const { session } = setup(false)
    session.send({ type: 'listPluginPresets', id: 'au:aumu Smp7 Fake' })
    session.send({ type: 'setPartPluginPreset', part: 0, id: 'aumu Smp7 Fake', preset: 'f:1' })
    session.advance(5000)
    flushSync()
    expect(inSlot('Right 1', 'rack.save_sound').getAttribute('aria-disabled')).toBe('true')
    session.pluginWindow(0, 1)
    flushSync()
    expect(session.state.keyboardParts[0].soundEdited).toBe(true)
    expect(slot('Right 1').textContent).toContain('● sound edited')
    const sent = spy(session)
    await fireEvent.click(inSlot('Right 1', 'rack.save_sound'))
    expect(sent).toContain('saveSound')
    session.advance(1000)
    flushSync()
    expect(slot('Right 1').textContent).not.toContain('sound edited')
    expect(slot('Right 1').querySelector('.badge')!.textContent).toBe('Mine')
  })

  it('a part whose plugin is missing is marked, and Replace… opens Library › Sounds on it', async () => {
    const { session } = setup()
    session.missingPlugin(2)
    flushSync()
    expect(slot('Right 3').querySelector('.warn')!.textContent).toContain("String Deluxe isn't installed")
    await fireEvent.click(inSlot('Right 3', 'rack.replace'))
    expect(ui.view).toBe('library')
    expect(ui.libraryTab).toBe('sounds')
    expect(ui.libraryPart).toBe(2)
  })

  it('hovering a part lights its fader on the mirror', async () => {
    setup()
    render(Launchkey)
    await fireEvent.pointerEnter(slot('Right 2'))
    expect(mirror.panelFader).toBe(1)
    const bank = document.querySelector('[aria-label="Faders"]')!
    expect([...bank.children].indexOf(bank.querySelector('.linked')!)).toBe(1)
    await fireEvent.pointerLeave(slot('Right 2'))
    expect(bank.querySelector('.linked')).toBeNull()
  })
})

describe('Rack panel: One Touch Settings', () => {
  it('recalling an OTS updates the parts and marks the faders it moved as waiting', async () => {
    const { session } = setup()
    await fireEvent.click(tipped('ots.4'))
    flushSync()
    expect(session.state.ots.applied).toBe(4)
    expect(inSlot('Right 1', 'rack.sound').textContent).toContain('Alto Sax')
    expect(tipped('ots.4').getAttribute('aria-pressed')).toBe('true')
    expect(tipped('ots.4').textContent).toContain('last recalled')
    expect(document.querySelector('p.pickup')!.textContent).toContain('waiting')
  })

  it('shows OTS Link and which Main recalls which OTS', async () => {
    const { session } = setup()
    expect(tipped('ots.1').textContent).not.toContain('Main A')
    await fireEvent.click(tipped('ots.link'))
    flushSync()
    expect(session.state.ots.link).toBe(true)
    expect(tipped('ots.1').textContent).toContain('Main A')
    expect(tipped('ots.link_timing').textContent).toContain('At Main Section Change')
  })

  it('lists the voice each style part was written for', () => {
    setup()
    const rows = document.querySelectorAll('.written li')
    expect(rows).toHaveLength(8)
    expect(rows[5].textContent).toContain('ch 14')
  })
})

describe('Rack panel: docked', () => {
  it('renders without the drawer frame, every control tipped', () => {
    setup(true, { docked: true })
    expect(document.querySelector('[data-overlay]')).toBeNull()
    expect(document.querySelector('aside.dock[aria-label="Rack"]')).toBeTruthy()
    expect(slot('Left')).toBeTruthy()
    // The docked panel isn't in App.svelte, so the coverage test can't see it: check here.
    const controls = [...document.body.querySelectorAll('button, select, input, [role="slider"], [role="switch"], [tabindex]:not([tabindex="-1"])')]
    expect(controls.length).toBeGreaterThan(30)
    expect(controls.filter((el) => !isTipKey(el.getAttribute('data-tip') ?? '')).map((el) => el.outerHTML.slice(0, 120))).toEqual([])
  })
})

describe('Stage: sound names under the part faders', () => {
  const names = () => [...document.querySelectorAll<HTMLElement>('[aria-label="Faders"] button.sname')]

  it('names each part\'s sound on the Panel page, with ● edited and ⚠ missing; a click opens the picker', async () => {
    const session = new MockSession({ manual: true, demo: false })
    app.attach(session)
    flushSync()
    render(Launchkey)
    expect(names().map((b) => b.textContent)).toEqual(session.state.keyboardParts.map((p) => p.voiceName))
    expect(document.querySelector('.fader-head')!.textContent).toContain('Rack: Untitled rack')
    session.missingPlugin(1)
    session.send({ type: 'setPartVolume', part: 0, volume: 12 })
    flushSync()
    expect(names()[1].textContent!.startsWith('⚠ ')).toBe(true)
    expect(names()[1].classList.contains('missing')).toBe(true)
    expect(document.querySelector('.fader-head')!.textContent).toContain('Rack: Untitled rack ●')
    await fireEvent.click(names()[3])
    expect(ui.view).toBe('library')
    expect(ui.libraryPart).toBe(3)
    // The Style page's faders are the band: no part sounds.
    session.send({ type: 'toggleFaderPage' })
    flushSync()
    expect(names()).toHaveLength(0)
    expect(document.querySelector('.fader-head')!.textContent).toContain('Style: the band')
  })

  it('an edited sound gets ● under its fader', () => {
    const session = new MockSession({ manual: true, demo: false })
    app.attach(session)
    session.send({ type: 'listPluginPresets', id: 'au:aumu Smp7 Fake' })
    session.send({ type: 'setPartPluginPreset', part: 0, id: 'aumu Smp7 Fake', preset: 'f:1' })
    session.advance(5000)
    session.pluginWindow(0, 1)
    flushSync()
    render(Launchkey)
    expect(names()[0].textContent!.endsWith(' ●')).toBe(true)
    expect(names()[0].classList.contains('edited')).toBe(true)
  })
})

describe('rack helpers', () => {
  it('word the rack, the sounds and the controller map', () => {
    const live = { name: 'New rack', id: null, modified: false, controls: { faders: [], knobs: [] }, prompt: null }
    expect(rackName(live)).toBe('Untitled rack')
    expect(rackName({ ...live, name: 'Restored' })).toBe('Restored')
    expect(rackName({ ...live, name: 'Ballad', id: 'r1' })).toBe('Ballad')
    const s = new MockSession({ manual: true, demo: false })
    const p = s.state.keyboardParts[0]
    expect(soundLabel({ ...p, soundEdited: true })).toBe(`${p.voiceName} ●`)
    expect(soundBadge(p, 'sf:GeneralUser-GS.sf2:0:0')).toBe('SoundFont')
    expect(soundBadge(p, 'saved:warm-rhodes')).toBe('Mine')
    expect(soundBadge(p, 'au:aumu Smp7 Fake#f:1')).toBe('Factory')
    expect(targetLabel({ kind: 'partChorus', part: 3 })).toBe('Left chorus')
    expect(targetLabel({ kind: 'harmonyArp' })).toBe('Harmony/Arp on/off')
    expect(targetLabel({ kind: 'somethingNew' } as never)).toBe('Set by a newer yahaha')
  })
})

describe('plugin status', () => {
  it('In proc sets the plugin\'s run-in-process override, which the next load follows', async () => {
    const { session } = setup()
    session.send({ type: 'setPartPlugin', part: 0, id: 'aumu Mock Demo', state: null })
    session.advance(1000)
    flushSync()
    const btn = slot('Right 1').querySelector<HTMLElement>('[data-tip="part.plugin_in_process"]')!
    expect(btn.getAttribute('aria-disabled')).toBe('true')
    await fireEvent.click(btn)
    expect(session.state.plugins.list.find((p) => p.id === 'aumu Mock Demo')!.inProcess).toBe(false)
    session.send({ type: 'setPartPlugin', part: 0, id: 'aumu dls  appl', state: null })
    session.advance(1000)
    flushSync()
    expect(btn.getAttribute('aria-pressed')).toBe('false')
    await fireEvent.click(btn)
    flushSync()
    expect(session.state.plugins.list.find((p) => p.id === 'aumu dls  appl')!.inProcess).toBe(true)
    expect(btn.getAttribute('aria-pressed')).toBe('true')
    expect(session.state.message?.text).toContain('from its next load')
    await fireEvent.click(btn)
    expect(session.state.plugins.list.find((p) => p.id === 'aumu dls  appl')!.inProcess).toBe(false)
  })

  it('In proc shows ↻ while the playing plugin still runs where it loaded (#176)', () => {
    const p = { status: 'playing', outOfProcess: true, inProcessFallback: false } as PartPlugin
    const e = (id: string, inProcess: boolean, format: 'AUv2' | 'AUv3' = 'AUv2') => ({ id, inProcess, format }) as PluginEntry
    // Turned on while it runs in its own process: pending until the next load.
    expect(inProcessPending(p, e('aumu Xf2X XFER', true))).toBe(true)
    expect(inProcessPending({ ...p, outOfProcess: false }, e('aumu Xf2X XFER', true))).toBe(false)
    // Turned off while it runs in process: its next load goes to its own process.
    expect(inProcessPending({ ...p, outOfProcess: false }, e('aumu Xf2X XFER', false))).toBe(true)
    expect(inProcessPending(p, e('aumu Xf2X XFER', false))).toBe(false)
    // An Apple AUv2 runs in process either way; a fallback is not the override's.
    expect(inProcessPending({ ...p, outOfProcess: false }, e('aumu dls  appl', false))).toBe(false)
    expect(inProcessPending({ ...p, outOfProcess: false }, e('aumu Ab3X appl', false, 'AUv3'))).toBe(true)
    expect(inProcessPending({ ...p, outOfProcess: false, inProcessFallback: true }, e('aumu Xf2X XFER', false))).toBe(false)
    // Only a playing plugin.
    expect(inProcessPending({ ...p, status: 'loading' }, e('aumu Xf2X XFER', true))).toBe(false)
    expect(inProcessPending(p, undefined)).toBe(false)
  })

  it('the mixer badge reads out the plugin\'s CPU and its slow renders of the last 10 s', () => {
    const s = new MockSession({ manual: true, demo: false })
    s.send({ type: 'setPartPlugin', part: 0, id: 'aumu dls  appl', state: null })
    s.send({ type: 'setPartPlugin', part: 1, id: 'aumu samp appl', state: null })
    s.advance(1000)
    const light = s.state.keyboardParts[0].plugin!
    expect(pluginBadge(light)).toBe('Plugin · 1% CPU')
    expect(pluginTip(light)).toBe('mixer.plugin')
    const heavy = s.state.keyboardParts[1].plugin!
    expect(pluginBadge(heavy)).toBe('4 slow · 31% CPU')
    expect(pluginTip(heavy)).toBe('mixer.plugin_overruns')
    heavy.recentOverruns = 0
    expect(pluginBadge(heavy)).toBe('Plugin · 31% CPU')
  })

  it('a plugin that fell back to loading in process says so, in the part and on the mixer badge', () => {
    const s = new MockSession({ manual: true, demo: false })
    s.send({ type: 'setPartPlugin', part: 1, id: 'aumu Tiny Demo', state: null })
    s.advance(1000)
    const p = s.state.keyboardParts[1].plugin!
    expect(p.status).toBe('playing')
    expect(p.inProcessFallback).toBe(true)
    expect(p.outOfProcess).toBe(false)
    expect(s.state.message?.text).toContain("can't run in its own process")
    expect(pluginStatusLine(p, true)).toContain('⚠ in process')
    expect(pluginBadge(p)).toMatch(/^Plugin ⚠ · \d+% CPU$/)
    s.send({ type: 'setPartPlugin', part: 1, id: 'aumu dls  appl', state: null })
    s.advance(1000)
    const q = s.state.keyboardParts[1].plugin!
    expect(q.inProcessFallback).toBe(false)
    expect(pluginStatusLine(q, true)).not.toContain('⚠')
  })
})
