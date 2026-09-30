import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { MockSession } from '../../lib/api/mock'
import { app, ui } from '../../lib/store.svelte'
import { isTipKey } from '../../help/tooltips'
import { FLAT_EQ, type Meters } from '../../lib/api/types'
import MixerBar from './MixerBar.svelte'
import StripDetail from './StripDetail.svelte'
import MasterStrip from './MasterStrip.svelte'

/** Controls without a catalog tooltip (the full check is help/coverage.test.ts). */
const untipped = (root: ParentNode) =>
  [...root.querySelectorAll('button, select, [role="slider"], [role="tab"], [tabindex]:not([tabindex="-1"])')]
    .filter((e) => !isTipKey(e.getAttribute('data-tip') ?? ''))
    .map((e) => e.outerHTML.slice(0, 120))

function attach() {
  const s = new MockSession({ manual: true, demo: true })
  app.attach(s)
  flushSync()
  return s
}

/** A distinct reading per channel: channel n takes n %, its worst buffer 2n %. */
const METERS: Meters = {
  atMs: 0,
  channels: Array.from({ length: 16 }, (_, i) => ({ channel: i + 1, peak: 0, rms: 0, cpu: (i + 1) / 100, cpuPeak: (2 * (i + 1)) / 100 })),
  master: [0, 0],
  masterRms: [0, 0],
  clips: 0,
  cpu: { total: 1.36, peak: 0.4, bufferUs: 1333 },
}

const q = <T extends Element = HTMLElement>(sel: string) => document.querySelector<T>(sel)!
const qa = <T extends Element = HTMLElement>(sel: string) => [...document.querySelectorAll<T>(sel)]
const slider = (label: string) => q(`[role="slider"][aria-label="${label}"]`)

afterEach(() => {
  cleanup()
  app.detach()
  ui.mixer = false
  ui.effects = false
  ui.rack = false
  ui.settings = false
  ui.view = 'stage'
  ui.libraryTab = 'sounds'
})

describe('MixerBar, always shown', () => {
  const rackText = () => q('[data-testid="rack-name"]').textContent!.trim()

  it('names the loaded rack on the Panel page, ● while modified, and the band on the Style page', async () => {
    const s = attach()
    render(MixerBar, { meters: null })
    expect(q('[data-testid="rack-name"]').dataset.tip).toBe('stage.rack_name')
    expect(rackText()).toBe('Rack: Untitled rack')
    // The mock marks the rack modified from its second publish on.
    s.advance(16)
    s.send({ type: 'setPartVolume', part: 0, volume: 12 })
    s.advance(16)
    flushSync()
    expect(s.state.liveRack.modified).toBe(true)
    expect(rackText()).toBe('Rack: Untitled rack ●')
    s.send({ type: 'toggleFaderPage' })
    s.advance(16)
    flushSync()
    expect(rackText()).toBe('Style: the band')
  })

  it('Rack opens the Rack drawer, Library the sounds page, Mixer the details (and back)', async () => {
    attach()
    render(MixerBar, { meters: null })
    const btn = (key: string) => q<HTMLButtonElement>(`button[data-tip="${key}"]`)
    expect(ui.rack).toBe(false)
    await fireEvent.click(btn('drawer.rack'))
    expect(ui.rack).toBe(true)
    flushSync()
    expect(btn('drawer.rack').getAttribute('aria-pressed')).toBe('true')

    ui.libraryTab = 'racks'
    await fireEvent.click(btn('drawer.library'))
    expect(ui.view).toBe('library')
    expect(ui.libraryTab).toBe('sounds')
    flushSync()
    expect(btn('drawer.library').getAttribute('aria-pressed')).toBe('true')
    await fireEvent.click(btn('drawer.library'))
    expect(ui.view).toBe('stage')

    expect(document.querySelector('[role="tablist"]')).toBeNull()
    await fireEvent.click(btn('drawer.mixer'))
    expect(ui.mixer).toBe(true)
    flushSync()
    expect(btn('drawer.mixer').getAttribute('aria-pressed')).toBe('true')
    expect(document.querySelector('[role="tablist"]')).not.toBeNull()
    await fireEvent.click(btn('drawer.mixer'))
    expect(ui.mixer).toBe(false)
    // Opening the details never closed the drawer.
    expect(ui.rack).toBe(true)
  })

  it('the layer selector shows without the details and sends setFaderLayer', async () => {
    const s = attach()
    render(MixerBar, { meters: null })
    const layer = (n: string) => qa<HTMLButtonElement>('[data-tip="mixer.layer"]').find((b) => b.textContent?.includes(n))!
    expect(qa('[data-tip="mixer.layer"]').map((b) => b.textContent?.trim())).toEqual(['VOL', 'PAN', 'REV', 'CHO', 'DLY'])
    await fireEvent.click(layer('DLY'))
    expect(s.state.mixer.faderLayer).toBe('delay')
    flushSync()
    expect(layer('DLY').getAttribute('aria-checked')).toBe('true')
    expect(layer('VOL').getAttribute('aria-checked')).toBe('false')
  })

  it('every control has a tooltip, details hidden', () => {
    attach()
    render(MixerBar, { meters: METERS })
    expect(untipped(document.body)).toEqual([])
  })
})

describe('MixerBar, details shown', () => {
  beforeEach(() => {
    ui.mixer = true
  })

  it('the page tabs send setFaderPage, so the Launchkey follows, and follow the Launchkey', async () => {
    const s = attach()
    render(MixerBar, { meters: null })
    const tab = (n: string) => qa<HTMLButtonElement>('[role="tab"]').find((t) => t.textContent?.includes(n))!
    expect(tab('Panel').getAttribute('aria-selected')).toBe('true')
    await fireEvent.click(tab('Style'))
    expect(s.state.mixer.faderPage).toBe('style')
    flushSync()
    expect(tab('Style').getAttribute('aria-selected')).toBe('true')
    expect(document.body.textContent).toContain('Launchkey faders: Style')
    await fireEvent.keyDown(tab('Style'), { key: 'ArrowLeft' })
    expect(s.state.mixer.faderPage).toBe('panel')
    s.send({ type: 'toggleFaderPage' })
    s.advance(16)
    flushSync()
    expect(tab('Style').getAttribute('aria-selected')).toBe('true')
  })

  it('the layer switches send setFaderLayer', async () => {
    const s = attach()
    render(MixerBar, { meters: null })
    const rev = qa<HTMLButtonElement>('[data-tip="mixer.layer"]').find((b) => b.textContent?.includes('REV'))!
    await fireEvent.click(rev)
    expect(s.state.mixer.faderLayer).toBe('reverb')
    flushSync()
    expect(rev.getAttribute('aria-checked')).toBe('true')
  })

  it('the metronome switch, and Style Track Mute on either page', async () => {
    const s = attach()
    render(MixerBar, { meters: null })
    await fireEvent.click(q('button[data-tip="metronome.on"]'))
    expect(s.state.metronome.on).toBe(true)
    // Two parts switched off by hand: choosing an order leaves them off.
    s.send({ type: 'toggleStylePart', part: 0 })
    s.send({ type: 'toggleStylePart', part: 5 })
    flushSync()
    const b = qa<HTMLButtonElement>('button[data-tip="mixer.track_mute_order"]').find((x) => x.textContent === 'B')!
    await fireEvent.click(b)
    flushSync()
    expect(b.getAttribute('aria-pressed')).toBe('true')
    expect(s.state.mixer.styleParts.filter((p) => !p.on)).toHaveLength(2)
    expect(s.state.mixer.faderPage).toBe('panel')
    await fireEvent.keyDown(q('[data-tip="mixer.track_mute"]'), { key: 'Home' })
    flushSync()
    expect(s.state.mixer.styleParts.map((p) => p.on)).toEqual([false, false, false, true, false, false, false, false])
  })

  it('Sends: style resets every Style part\'s own sends', async () => {
    const s = attach()
    render(MixerBar, { meters: null })
    const reset = q<HTMLButtonElement>('button[data-tip="mixer.style.reset_sends"]')
    expect(reset.disabled).toBe(true)
    s.send({ type: 'setStylePartSend', part: 2, send: 'reverb', value: 100 })
    flushSync()
    expect(reset.disabled).toBe(false)
    await fireEvent.click(reset)
    expect(s.state.mixer.styleParts.every((p) => p.sendsSet.length === 0)).toBe(true)
  })

  it('the Style and Multi Pad volumes scale their CC 7s; the parts\' faders stay', async () => {
    const s = attach()
    render(MixerBar, { meters: null })
    const before = s.state.mixer.styleParts.map((p) => p.volume)
    expect(slider('Style volume').dataset.tip).toBe('mixer.style_level')
    await fireEvent.keyDown(slider('Style volume'), { key: 'PageDown' })
    expect(s.state.mixer.styleVolume).toBe(90)
    expect(s.state.mixer.styleParts.map((p) => p.volume)).toEqual(before)
    expect(slider('Multi Pad volume').dataset.tip).toBe('mixer.pad_level')
    await fireEvent.keyDown(slider('Multi Pad volume'), { key: 'Home' })
    expect(s.state.mixer.multiPadVolume).toBe(0)
  })

  it('shows every track\'s CPU together and the plugin instances from the meters', () => {
    const s = new MockSession({ manual: true, demo: true })
    s.send({ type: 'setPartPlugin', part: 1, id: 'aumu dls  appl', state: null })
    s.advance(1000)
    app.attach(s)
    flushSync()
    render(MixerBar, { meters: METERS })
    const total = q('[data-testid="cpu-total"]').textContent!.replace(/\s+/g, ' ')
    expect(total).toContain('CPU 136% · pk 40% of a 1.3 ms buffer')
    expect(total).toContain('Plugins 1')
    cleanup()
    render(MixerBar, { meters: null })
    expect(q('[data-testid="cpu-total"]').textContent).toContain('(synth off)')
  })

  it('shows the Style\'s and the Multi Pads\' CPU beside their volumes, red by the average only', () => {
    attach()
    render(MixerBar, { meters: METERS })
    const text = (id: string) => q(`[data-testid="${id}"]`).textContent!.replace(/\s+/g, ' ').trim()
    // Style: channels 9–16 (9 % … 16 %); Multi Pads: channels 5–8.
    expect(text('style-cpu')).toBe('100% ≤ pk 200%')
    expect(text('pad-cpu')).toBe('26% ≤ pk 52%')
    expect(q('[data-testid="style-cpu"]').dataset.tip).toBe('mixer.cpu_group')
    expect(q('[data-testid="pad-cpu"]').dataset.tip).toBe('mixer.cpu_group')
    // Style by its 100 % average; the pads' summed peak (52 %) is past the line but only an upper bound.
    expect(q('[data-testid="style-cpu"]').classList.contains('warn')).toBe(true)
    expect(q('[data-testid="pad-cpu"]').classList.contains('warn')).toBe(false)
    cleanup()
    render(MixerBar, { meters: null })
    expect(document.querySelector('[data-testid="style-cpu"]')).toBeNull()
    expect(document.querySelector('[data-testid="pad-cpu"]')).toBeNull()
  })

  it('Effects… names what each block plays and opens the Effects screen', async () => {
    const s = attach()
    render(MixerBar, { meters: null })
    const row = q('[aria-label="Effects"]')
    expect(row.textContent?.replace(/\s+/g, ' ')).toContain('Reverb Hall')
    s.send({ type: 'setEffectType', block: 'reverb', effect: 'plate' })
    flushSync()
    expect(row.textContent).toContain('Plate')
    await fireEvent.click(row.querySelector<HTMLElement>('[data-tip="drawer.effects"]')!)
    expect(ui.effects).toBe(true)
  })

  it('every control has a tooltip', () => {
    attach()
    render(MixerBar, { meters: METERS })
    expect(untipped(document.body)).toEqual([])
  })
})

describe('StripDetail', () => {
  it('a keyboard part: its channel, Chorus (CC 93, double-click back to 0) and CPU', async () => {
    const s = attach()
    render(StripDetail, { part: 1, meters: METERS })
    expect(q('[data-tip="mixer.channel"]').textContent?.replace(/\s+/g, ' ').trim()).toBe('Ch 3')
    const cho = slider('Right 2 chorus')
    expect(cho.dataset.tip).toBe('mixer.part.chorus')
    await fireEvent.keyDown(cho, { key: 'End' })
    expect(s.state.keyboardParts[1].chorus).toBe(127)
    await fireEvent.dblClick(cho)
    expect(s.state.keyboardParts[1].chorus).toBe(0)
    expect(q('[data-testid="cpu"]').textContent?.replace(/\s+/g, ' ').trim()).toBe('3.0% pk 6.0%')
    expect(q('[data-testid="cpu"] [data-tip="mixer.cpu"]')).not.toBeNull()
  })

  it('a keyboard part\'s EQ knobs send setPartEq (#247)', async () => {
    const s = attach()
    render(StripDetail, { part: 3, meters: null })
    const eq = qa('[aria-label="Left EQ"] .knob')
    expect(eq.map((e) => e.getAttribute('aria-label'))).toEqual(['Left EQ low', 'Left EQ low frequency', 'Left EQ high', 'Left EQ high frequency'])
    expect(eq.map((e) => e.getAttribute('aria-valuetext'))).toEqual(['0', '80', '0', '10k'])
    await fireEvent.keyDown(eq[0], { key: 'ArrowUp' })
    expect(s.state.keyboardParts[3].eq).toEqual({ ...FLAT_EQ, lowGain: 1 })
    await fireEvent.keyDown(eq[3], { key: 'Home' })
    expect(s.state.keyboardParts[3].eq.highFreq).toBe(500)
    flushSync()
    expect(slider('Left EQ high frequency').getAttribute('aria-valuetext')).toBe('500')
  })

  it('a keyboard part\'s insert slot: effect, on and amount', async () => {
    const s = attach()
    render(StripDetail, { part: 0, meters: null })
    const kind = q<HTMLSelectElement>('select[aria-label="Right 1 insert effect"]')
    expect(kind.dataset.tip).toBe('mixer.part.insert_effect')
    await fireEvent.change(kind, { target: { value: 'rotary' } })
    expect(s.state.keyboardParts[0].insert.effect).toBe('rotary')
    const on = q<HTMLButtonElement>('button[aria-label="Right 1 insert on"]')
    await fireEvent.click(on)
    expect(s.state.keyboardParts[0].insert.on).toBe(true)
    await fireEvent.keyDown(slider('Right 1 insert amount'), { key: 'End' })
    expect(s.state.keyboardParts[0].insert.amount).toBe(127)
    flushSync()
    expect(on.getAttribute('aria-pressed')).toBe('true')
  })

  it('a Style part: Chorus is the style\'s until turned, then its own; no EQ or insert', async () => {
    const s = attach()
    render(StripDetail, { part: 7, meters: METERS })
    expect(q('[data-tip="mixer.channel"]').textContent).toContain(String(s.state.mixer.styleParts[3].channel))
    const style = s.state.mixer.styleParts[3].chorus
    const cho = slider('Chord 1 chorus')
    expect(cho.dataset.tip).toBe('mixer.style.chorus')
    expect(cho.getAttribute('aria-valuenow')).toBe(String(style))
    await fireEvent.keyDown(cho, { key: 'PageUp' })
    expect(s.state.mixer.styleParts[3].chorus).toBe(style + 10)
    expect(s.state.mixer.styleParts[3].sendsSet).toEqual(['chorus'])
    flushSync()
    expect(cho.classList.contains('own')).toBe(true)
    await fireEvent.dblClick(cho)
    expect(s.state.mixer.styleParts[3].chorus).toBe(style)
    expect(s.state.mixer.styleParts[3].sendsSet).toEqual([])
    expect(document.querySelector('select')).toBeNull()
    expect(document.querySelector('[data-tip^="mixer.part.eq"]')).toBeNull()
    const ch = s.state.mixer.styleParts[3].channel
    expect(q('[data-testid="cpu"]').textContent).toContain(`${ch}%`)
  })

  it('the badge: a plugin part names its plugin state', () => {
    const s = new MockSession({ manual: true, demo: true })
    s.send({ type: 'setPartPlugin', part: 0, id: 'aumu dls  appl', state: null })
    s.advance(1000)
    app.attach(s)
    flushSync()
    render(StripDetail, { part: 0, meters: null })
    const tag = q('.tag')
    expect(tag.textContent).toContain('CPU')
    expect(tag.dataset.tip).toBe('mixer.plugin')
  })

  it('every control has a tooltip, keyboard and Style', () => {
    attach()
    render(StripDetail, { part: 0, meters: METERS })
    render(StripDetail, { part: 4, meters: METERS })
    expect(untipped(document.body)).toEqual([])
  })
})

describe('MasterStrip', () => {
  it('the master fader sends setMasterVolume', async () => {
    const s = attach()
    render(MasterStrip)
    const f = slider('Master')
    expect(f.dataset.tip).toBe('mixer.master')
    await fireEvent.keyDown(f, { key: 'PageDown' })
    expect(s.state.mixer.master).not.toBeNull()
    const was = Number(f.getAttribute('aria-valuenow'))
    await fireEvent.keyDown(f, { key: 'Home' })
    expect(s.state.mixer.master).toBe(0)
    expect(was).toBeGreaterThan(0)
  })

  it('the Master editor opens over the screen, and closes with ✕ or Escape', async () => {
    const s = attach()
    render(MasterStrip)
    const edit = q<HTMLButtonElement>('button[data-tip="fx.master_edit"]')
    expect(document.querySelector('[aria-label="Master Compressor and EQ"]')).toBeNull()
    await fireEvent.click(edit)
    const editor = q('[aria-label="Master Compressor and EQ"]')
    expect(editor).not.toBeNull()
    await fireEvent.change(q<HTMLSelectElement>('select[aria-label="Master EQ type"]'), { target: { value: 'bright' } })
    expect(s.state.effects.master.eq.preset).toBe('bright')
    await fireEvent.click(q('button[aria-label="Close master settings"]'))
    expect(document.querySelector('[aria-label="Master Compressor and EQ"]')).toBeNull()
    await fireEvent.click(edit)
    await fireEvent.keyDown(window, { key: 'Escape' })
    expect(document.querySelector('[aria-label="Master Compressor and EQ"]')).toBeNull()
    // A press inside keeps it open; one outside closes it.
    await fireEvent.click(edit)
    await fireEvent.pointerDown(q('[aria-label="Master Compressor and EQ"] select'))
    expect(document.querySelector('[aria-label="Master Compressor and EQ"]')).not.toBeNull()
    await fireEvent.pointerDown(document.body)
    expect(document.querySelector('[aria-label="Master Compressor and EQ"]')).toBeNull()
    await fireEvent.click(q('button[data-tip="fx.master_comp"]'))
    expect(s.state.effects.master.compressor.on).toBe(true)
  })

  it('Details toggles the row\'s details (ui.mixer)', async () => {
    attach()
    render(MasterStrip)
    const d = q<HTMLButtonElement>('button[data-tip="drawer.mixer"]')
    expect(d.textContent?.trim()).toBe('Details')
    await fireEvent.click(d)
    expect(ui.mixer).toBe(true)
    flushSync()
    expect(d.getAttribute('aria-pressed')).toBe('true')
    await fireEvent.click(d)
    expect(ui.mixer).toBe(false)
  })

  it('from Library, Details goes back to Stage with the details shown', () => {
    ui.view = 'library'
    ui.toggleMixer()
    expect(ui.view).toBe('stage')
    expect(ui.mixer).toBe(true)
    // Again from Library: still shows them (never hides what can't be seen).
    ui.view = 'library'
    ui.toggleMixer()
    expect(ui.mixer).toBe(true)
    ui.toggleMixer()
    expect(ui.mixer).toBe(false)
  })

  it('opening or closing a drawer leaves the details as they were', () => {
    ui.mixer = true
    ui.toggleDrawer('rack')
    expect(ui.rack).toBe(true)
    expect(ui.mixer).toBe(true)
    ui.toggleDrawer('effects')
    ui.toggleDrawer('effects')
    expect(ui.mixer).toBe(true)
    ui.mixer = false
    ui.toggleDrawer('settings')
    expect(ui.mixer).toBe(false)
    ui.toggleDrawer('settings')
  })

  it('every control has a tooltip, the editor open too', async () => {
    attach()
    render(MasterStrip)
    await fireEvent.click(q('button[data-tip="fx.master_edit"]'))
    expect(untipped(document.body)).toEqual([])
  })
})
