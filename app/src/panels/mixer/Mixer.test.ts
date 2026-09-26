import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { MockSession } from '../../lib/api/mock'
import { app, ui } from '../../lib/store.svelte'
import { isTipKey } from '../../help/tooltips'
import Mixer from './Mixer.svelte'
import { partVoice, styleVoice } from './voice'

/** Controls without a catalog tooltip (the full check is help/coverage.test.ts). */
const untipped = (root: ParentNode) =>
  [...root.querySelectorAll('button, [role="slider"], [role="tab"], [tabindex]:not([tabindex="-1"])')]
    .filter((e) => !isTipKey(e.getAttribute('data-tip') ?? ''))
    .map((e) => e.outerHTML.slice(0, 120))

function setup() {
  const session = new MockSession({ manual: true, demo: true })
  app.attach(session)
  ui.mixer = true
  flushSync()
  render(Mixer)
  return session
}

afterEach(() => {
  cleanup()
  app.detach()
  ui.mixer = false
})

/** The channel faders and the master (not the Panel strips' pan/send knobs). */
const sliders = () => [...document.querySelectorAll<HTMLElement>('.strips [role="slider"]:not(.knob)')]
const knobs = () => [...document.querySelectorAll<HTMLElement>('.strips .knob')]
const tab = (name: string) => [...document.querySelectorAll<HTMLButtonElement>('[role="tab"]')].find((t) => t.textContent?.includes(name))!

describe('Mixer drawer', () => {
  it('the Panel tab shows Right 1–3 and Left with their CC 7, channels and voices, then 4 unused faders and the master', () => {
    const s = setup()
    expect(tab('Panel').getAttribute('aria-selected')).toBe('true')
    const f = sliders()
    expect(f).toHaveLength(9)
    expect(f.slice(0, 4).map((e) => e.getAttribute('aria-label'))).toEqual(['Right 1', 'Right 2', 'Right 3', 'Left'])
    expect(f[1].getAttribute('aria-valuenow')).toBe(String(s.state.keyboardParts[1].volume))
    expect(f[4].dataset.tip).toBe('mixer.style_level')
    expect(f[4].getAttribute('aria-label')).toBe('Style')
    expect(f[5].dataset.tip).toBe('mixer.pad_level')
    expect(f.slice(6, 8).every((e) => e.dataset.tip === 'launchkey.fader_unused')).toBe(true)
    expect(f[8].dataset.tip).toBe('mixer.master')
    const chans = [...document.querySelectorAll('.strips [data-tip="mixer.channel"]')].map((e) => e.textContent?.replace(/\s+/g, ' ').trim())
    expect(chans).toEqual(['Ch 1', 'Ch 3', 'Ch 4', 'Ch 2'])
  })

  it('Panel fader 5 is the Style volume, a scale on the Style parts; their faders stay', async () => {
    const s = setup()
    const before = s.state.mixer.styleParts.map((p) => p.volume)
    expect(sliders()[4].getAttribute('aria-valuenow')).toBe('100')
    await fireEvent.keyDown(sliders()[4], { key: 'PageDown' })
    expect(s.state.mixer.styleVolume).toBe(90)
    expect(s.state.mixer.styleParts.map((p) => p.volume)).toEqual(before)
    expect(s.state.surface.faders[4].label).toBe('STYLE')
  })

  it('Panel fader 6 is the Multi Pad volume', async () => {
    const s = setup()
    expect(sliders()[5].getAttribute('aria-label')).toBe('M.Pad')
    await fireEvent.keyDown(sliders()[5], { key: 'Home' })
    expect(s.state.mixer.multiPadVolume).toBe(0)
    expect(s.state.surface.faders[5].label).toBe('M.PAD')
  })

  it('Panel strip 5 has the HARMONY/ARPEGGIO button, as the Launchkey button under fader 5', async () => {
    const s = setup()
    const strips = [...document.querySelectorAll<HTMLElement>('.strips .strip')]
    const buttons = (i: number) => [...strips[i].querySelectorAll<HTMLButtonElement>('.buttons button')]
    expect(buttons(4)).toHaveLength(1)
    expect(buttons(4)[0].dataset.tip).toBe('harmony.switch')
    expect(buttons(4)[0].textContent?.trim()).toBe('Harm/Arp')
    for (const i of [5, 6, 7]) expect(buttons(i)).toHaveLength(0)
    const was = s.state.harmonyArp.on
    await fireEvent.click(buttons(4)[0])
    expect(s.state.harmonyArp.on).toBe(!was)
    // The Launchkey surface agrees: faderButton5 on the Panel page is HARM/ARP.
    expect(s.state.surface.controls.find((c) => c.id === 'faderButton5')?.label).toBe('HARM/ARP')
  })

  it('switching tabs sends the fader page, so the Launchkey follows, and shows the 8 style parts', async () => {
    const s = setup()
    await fireEvent.click(tab('Style'))
    expect(s.state.mixer.faderPage).toBe('style')
    flushSync()
    const f = sliders()
    expect(f.slice(0, 8).map((e) => e.getAttribute('aria-label'))).toEqual(['Rhythm 1', 'Rhythm 2', 'Bass', 'Chord 1', 'Chord 2', 'Pad', 'Phrase 1', 'Phrase 2'])
    expect(document.body.textContent).toContain('104/0/49')
  })

  it('Effects: each block\'s type and return level (#204)', async () => {
    const s = setup()
    const types = document.querySelectorAll<HTMLSelectElement>('select[aria-label$=" type"]')
    expect([...types].map((e) => e.getAttribute('aria-label'))).toEqual(['Reverb type', 'Chorus type', 'Variation type'])
    expect(types[0].value).toBe('hall')
    await fireEvent.change(types[2], { target: { value: 'pingPong' } })
    expect(s.state.effects.blocks[2].effect).toBe('pingPong')
    const ret = document.querySelector<HTMLElement>('[aria-label="Reverb return"]')!
    expect(ret.getAttribute('aria-valuetext')).toBe('+0.0 dB')
    await fireEvent.keyDown(ret, { key: 'Home' })
    expect(s.state.effects.blocks[0].returnLevel).toBe(0)
  })

  it('Effects: a block\'s editor sets its parameters; a type change puts them back (#236)', async () => {
    const s = setup()
    const open = document.querySelector<HTMLElement>('[aria-label="Reverb settings"]')!
    expect(open.getAttribute('aria-expanded')).toBe('false')
    await fireEvent.click(open)
    flushSync()
    const time = document.querySelector<HTMLElement>('[aria-label="Reverb Time"]')!
    expect(time.getAttribute('aria-valuetext')).toBe('2.4 s')
    await fireEvent.keyDown(time, { key: 'End' })
    expect(s.state.effects.blocks[0].params[0].value).toBe(100)
    expect(s.state.effects.blocks[0].params[0].display).toBe('10.0 s')
    const types = document.querySelectorAll<HTMLSelectElement>('select[aria-label$=" type"]')
    await fireEvent.change(types[0], { target: { value: 'room' } })
    expect(s.state.effects.blocks[0].params.map((p) => p.value)).toEqual([9, 4, 60])
  })

  it('Effects: the delay editor, note or free time, and its switches (#236)', async () => {
    const s = setup()
    await fireEvent.click(document.querySelector<HTMLElement>('[aria-label="Variation settings"]')!)
    flushSync()
    const note = document.querySelector<HTMLElement>('[aria-label="Variation Note"]')!
    const time = document.querySelector<HTMLElement>('[aria-label="Variation Time"]')!
    expect(note.getAttribute('aria-valuetext')).toBe('1/8.')
    expect(time.getAttribute('aria-disabled')).toBe('true')
    const sync = [...document.querySelectorAll<HTMLElement>('[role="switch"]')].find((e) => e.textContent?.includes('Tempo sync'))!
    await fireEvent.click(sync)
    flushSync()
    expect(s.state.effects.blocks[2].params[0].value).toBe(0)
    expect(document.querySelector<HTMLElement>('[aria-label="Variation Time"]')!.getAttribute('aria-disabled')).toBeNull()
    const pp = [...document.querySelectorAll<HTMLElement>('[role="switch"]')].find((e) => e.textContent?.includes('Ping-pong'))!
    await fireEvent.click(pp)
    expect(s.state.effects.blocks[2].params[5].display).toBe('On')
  })

  it('Effects: the chorus editor has rate and depth (#236)', async () => {
    const s = setup()
    await fireEvent.click(document.querySelector<HTMLElement>('[aria-label="Chorus settings"]')!)
    flushSync()
    const depth = document.querySelector<HTMLElement>('[aria-label="Chorus Depth"]')!
    expect(depth.getAttribute('aria-valuetext')).toBe('2.2 ms')
    await fireEvent.keyDown(depth, { key: 'ArrowUp' })
    expect(s.state.effects.blocks[1].params[1].display).toBe('2.3 ms')
  })

  it('Effects: a band send per block, the band\'s chorus and delay off at start (#236)', async () => {
    const s = setup()
    const band = (name: string) => document.querySelector<HTMLElement>(`[aria-label="${name} band send"]`)!
    expect(['Reverb', 'Chorus', 'Variation'].map((n) => band(n).getAttribute('aria-valuetext'))).toEqual(['100%', '0%', '0%'])
    await fireEvent.keyDown(band('Variation'), { key: 'PageUp' })
    expect(s.state.effects.blocks[2].bandSend).toBe(10)
    await fireEvent.keyDown(band('Reverb'), { key: 'Home' })
    expect(s.state.effects.blocks[0].bandSend).toBe(0)
  })

  it('follows the page when the Launchkey switches it', () => {
    const s = setup()
    s.send({ type: 'toggleFaderPage' })
    s.advance(16)
    flushSync()
    expect(tab('Style').getAttribute('aria-selected')).toBe('true')
  })

  it('a fader sends the CC 7 value, and on/off toggles the part', async () => {
    const s = setup()
    await fireEvent.click(tab('Style'))
    flushSync()
    await fireEvent.keyDown(sliders()[2], { key: 'End' })
    expect(s.state.mixer.styleParts[2].volume).toBe(127)
    const on = document.querySelectorAll<HTMLButtonElement>('button[data-tip="mixer.style.mute"]')
    expect(on).toHaveLength(8)
    await fireEvent.click(on[0])
    expect(s.state.mixer.styleParts[0].on).toBe(false)
  })

  it('Panel strips have Pan, Reverb, Chorus and Delay knobs that send the part\'s CC 10/91/93/94; the Style tab has none', async () => {
    const s = setup()
    const k = knobs()
    expect(k).toHaveLength(16)
    expect(k.slice(12).map((e) => e.getAttribute('aria-label'))).toEqual(['Left pan', 'Left reverb', 'Left chorus', 'Left delay'])
    expect(k[0].getAttribute('aria-valuetext')).toBe('C')
    await fireEvent.keyDown(k[12], { key: 'Home' })
    expect(s.state.keyboardParts[3].pan).toBe(0)
    flushSync()
    expect(knobs()[12].getAttribute('aria-valuetext')).toBe('L64')
    await fireEvent.keyDown(k[5], { key: 'PageUp' })
    expect(s.state.keyboardParts[1].reverb).toBe(60)
    await fireEvent.keyDown(k[2], { key: 'End' })
    expect(s.state.keyboardParts[0].chorus).toBe(127)
    await fireEvent.dblClick(k[2])
    expect(s.state.keyboardParts[0].chorus).toBe(10)
    await fireEvent.keyDown(k[3], { key: 'PageUp' })
    expect(s.state.keyboardParts[0].variation).toBe(10)
    await fireEvent.dblClick(k[1])
    expect(s.state.keyboardParts[0].reverb).toBe(50)
    await fireEvent.click(tab('Style'))
    flushSync()
    expect(knobs()).toHaveLength(0)
  })

  it('shows the soft-takeover mark and the ghost of the hardware fader while a level waits', async () => {
    const s = setup()
    await fireEvent.click(tab('Style'))
    s.advance(16)
    flushSync()
    const pad = sliders()[5]
    expect(s.state.mixer.styleParts[5].waiting).toBe(true)
    expect(pad.getAttribute('aria-valuetext')).toContain('waiting')
    expect(pad.querySelector('.hw')).not.toBeNull()
  })

  it('Solo solos a keyboard part on the Panel tab and a band part on the Style tab; again ends it', async () => {
    const s = setup()
    const solos = () => [...document.querySelectorAll<HTMLButtonElement>('button[data-tip="mixer.solo"]')]
    expect(solos()).toHaveLength(4)
    await fireEvent.click(solos()[1])
    flushSync()
    expect(s.state.mixer.partSolo).toBe(1)
    expect(s.state.keyboardParts.map((p) => p.sounding)).toEqual([false, true, false, false])
    expect(solos()[1].getAttribute('aria-pressed')).toBe('true')
    await fireEvent.click(solos()[1])
    flushSync()
    expect(s.state.mixer.partSolo).toBeNull()
    await fireEvent.click(tab('Style'))
    flushSync()
    expect(solos()).toHaveLength(8)
    await fireEvent.click(solos()[2])
    flushSync()
    expect(s.state.mixer.styleSolo).toBe(2)
    expect(s.state.mixer.partSolo).toBeNull()
  })

  it('the metronome and Style Track Mute', async () => {
    const s = setup()
    const metronome = document.querySelector<HTMLButtonElement>('button[data-tip="metronome.on"]')!
    await fireEvent.click(metronome)
    flushSync()
    expect(s.state.metronome.on).toBe(true)
    expect(document.querySelector('[data-tip="mixer.track_mute"]')).toBeNull()
    await fireEvent.click(tab('Style'))
    flushSync()
    // Two parts switched off by hand: choosing an order leaves them off (it only chooses
    // what the knob does next).
    s.send({ type: 'toggleStylePart', part: 0 })
    s.send({ type: 'toggleStylePart', part: 5 })
    flushSync()
    const b = [...document.querySelectorAll<HTMLButtonElement>('button[data-tip="mixer.track_mute_order"]')].find((x) => x.textContent === 'B')!
    await fireEvent.click(b)
    flushSync()
    expect(b.getAttribute('aria-pressed')).toBe('true')
    expect(s.state.mixer.styleParts.filter((p) => !p.on)).toHaveLength(2)
    const knob = document.querySelector<HTMLElement>('[data-tip="mixer.track_mute"]')!
    await fireEvent.keyDown(knob, { key: 'Home' })
    flushSync()
    expect(s.state.mixer.styleParts.map((p) => p.on)).toEqual([false, false, false, true, false, false, false, false])
  })

  it('every control has a tooltip, on both tabs', async () => {
    setup()
    expect(untipped(document.body)).toEqual([])
    await fireEvent.click(tab('Style'))
    flushSync()
    expect(untipped(document.body)).toEqual([])
  })
})

describe('voice lines', () => {
  const v = (label: string) => ({ bankMsb: 0, bankLsb: 0, program: 0, kit: false, label })
  it('splits the engine label into what plays and what it was written for', () => {
    expect(styleVoice(v('≈ Strings  [Yamaha 104/0/49]'))).toEqual({ plays: '≈ Strings', writtenFor: '104/0/49' })
    expect(styleVoice(v('Finger Bass (GM 34)'))).toEqual({ plays: 'Finger Bass', writtenFor: 'GM 34' })
    expect(styleVoice(v('drum kit 127/0/1'))).toEqual({ plays: 'Drum kit', writtenFor: 'kit 127/0/1' })
    expect(styleVoice(null)).toEqual({ plays: '—', writtenFor: '' })
  })
  it('a keyboard part under Manual Bass plays the Style Bass', () => {
    const p = { name: 'Left', channel: 2, on: false, sounding: true, selected: false, volume: 100, waiting: false, program: 48, voiceName: 'Finger Bass', playsBass: true, octave: 0, pan: 64, reverb: 40, chorus: 0, variation: 0, fader: null, patch: null }
    expect(partVoice(p).writtenFor).toContain('Manual Bass')
    expect(partVoice({ ...p, playsBass: false, voiceName: 'Strings' })).toEqual({ plays: 'Strings', writtenFor: 'GM 49' })
  })
})
