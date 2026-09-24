import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { MockSession } from '../../lib/api/mock'
import { settings } from '../../lib/api/settings.svelte'
import { app, ui } from '../../lib/store.svelte'
import { nav } from './nav.svelte'
import { keyAt, keysBetween, noteName, SPLIT_MAX, SPLIT_MIN } from './notes'
import Settings from './Settings.svelte'

function setup() {
  const session = new MockSession({ manual: true, demo: false })
  app.attach(session)
  ui.settings = true
  flushSync()
  render(Settings)
  return session
}

afterEach(() => {
  cleanup()
  app.detach()
  ui.settings = false
  nav.tab = 'chord'
  settings.reset()
})

const q = <T extends HTMLElement = HTMLElement>(sel: string) => document.querySelector<T>(sel)!
const byTip = (key: string) => [...document.querySelectorAll<HTMLElement>(`[data-tip="${key}"]`)]
const page = (id: string) => q(`#settings-page-${id}`)

describe('Settings drawer', () => {
  it('groups the pages like the Genos menus, one visible at a time', async () => {
    setup()
    const tabs = [...document.querySelectorAll('[role="tab"]')].map((t) => t.textContent?.trim())
    expect(tabs).toEqual(['Chord', 'Split', 'Transpose', 'Style', 'Audio', 'MIDI', 'Library'])
    expect(page('chord').hidden).toBe(false)
    expect(page('audio').hidden).toBe(true)
    await fireEvent.click(q('#settings-tab-audio'))
    expect(page('audio').hidden).toBe(false)
    expect(page('chord').hidden).toBe(true)
    expect(q('#settings-tab-audio').getAttribute('aria-selected')).toBe('true')
  })

  it('arrow keys move between tabs', async () => {
    setup()
    await fireEvent.keyDown(q('#settings-tab-chord'), { key: 'ArrowLeft' })
    expect(nav.tab).toBe('library')
    await fireEvent.keyDown(q('#settings-tab-library'), { key: 'ArrowRight' })
    expect(nav.tab).toBe('chord')
  })

  it('has no Save button: every control applies at once', () => {
    setup()
    const labels = [...document.querySelectorAll('button')].map((b) => b.textContent?.trim().toLowerCase())
    expect(labels.some((l) => l === 'save' || l === 'apply')).toBe(false)
  })

  it('picks a fingering type; each of the seven has its own tooltip', async () => {
    const s = setup()
    const radios = [...page('chord').querySelectorAll<HTMLButtonElement>('.fing')]
    expect(radios).toHaveLength(7)
    expect(new Set(radios.map((r) => r.dataset.tip)).size).toBe(7)
    await fireEvent.click(radios.find((r) => r.dataset.tip === 'fingering.single_finger')!)
    expect(s.state.chord.fingering).toBe('singleFinger')
    expect(radios.find((r) => r.dataset.tip === 'fingering.single_finger')!.getAttribute('aria-checked')).toBe('true')
    expect(page('chord').querySelectorAll('.chart svg')).toHaveLength(4)
  })

  it('switches the detection area; Manual Bass only works in Upper', async () => {
    const s = setup()
    const manual = () => byTip('detection.manual_bass')[0]
    const msg = s.state.message
    await fireEvent.click(manual())
    expect(s.state.message).toBe(msg) // Lower: not even sent (the engine would refuse it)
    const [, upper] = byTip('detection.upper')
    await fireEvent.click(upper)
    expect(s.state.chord.upper).toBe(true)
    expect(s.state.chord.manualBass).toBe(true)
    await fireEvent.click(manual())
    expect(s.state.chord.manualBass).toBe(false)
  })

  it('moves the split with the keyboard and names it in Yamaha numbering', async () => {
    const s = setup()
    const strip = byTip('settings.split_strip')[0]
    const start = s.state.chord.split
    await fireEvent.keyDown(strip, { key: 'ArrowRight' })
    expect(s.state.chord.split).toBe(start + 1)
    await fireEvent.keyDown(strip, { key: 'PageDown' })
    expect(s.state.chord.split).toBe(start - 11)
    await fireEvent.keyDown(strip, { key: 'End' })
    expect(s.state.chord.split).toBe(SPLIT_MAX)
    await fireEvent.keyDown(strip, { key: 'Home' })
    expect(s.state.chord.split).toBe(SPLIT_MIN)
    expect(strip.getAttribute('aria-valuetext')).toBe('C0')
    expect(page('split').textContent).toContain('C0')
  })

  it('drags the split on the strip', async () => {
    const s = setup()
    const strip = byTip('settings.split_strip')[0]
    // 43 white keys, 430 px wide: 10 px a key. Middle C (C3) is the 22nd white key.
    vi.spyOn(strip, 'getBoundingClientRect').mockReturnValue({ left: 0, top: 0, width: 430, height: 60, right: 430, bottom: 60, x: 0, y: 0, toJSON: () => ({}) })
    await fireEvent.pointerDown(strip, { clientX: 215, clientY: 55, pointerId: 1 })
    expect(s.state.chord.split).toBe(60)
    await fireEvent.pointerMove(strip, { clientX: 225, clientY: 55, pointerId: 1 })
    expect(s.state.chord.split).toBe(62)
    await fireEvent.pointerUp(strip, { pointerId: 1 })
    await fireEvent.pointerMove(strip, { clientX: 300, clientY: 55, pointerId: 1 })
    expect(s.state.chord.split).toBe(62)
  })

  it('steps and resets transpose', async () => {
    const s = setup()
    await fireEvent.click(byTip('transpose.keyboard_up')[0])
    await fireEvent.click(byTip('transpose.master_down')[0])
    expect([s.state.chord.transposeKeyboard, s.state.chord.transposeMaster]).toEqual([1, -1])
    expect(page('transpose').textContent).toContain('+1')
    expect(page('transpose').textContent).toContain('−1')
    await fireEvent.click(byTip('transpose.reset')[0])
    expect([s.state.chord.transposeKeyboard, s.state.chord.transposeMaster]).toEqual([0, 0])
  })

  it('style toggles send the transport commands; M5 placeholders are disabled', async () => {
    const s = setup()
    const before = s.state.transport.autoFill
    await fireEvent.click(byTip('transport.auto_fill')[0])
    expect(s.state.transport.autoFill).toBe(!before)
    for (const key of ['settings.section_timing', 'settings.ots_link_timing', 'settings.synchro_stop_window']) {
      const els = byTip(key)
      expect(els.length).toBeGreaterThan(0)
      for (const el of els) expect(el.getAttribute('aria-disabled')).toBe('true')
    }
    expect(page('style').textContent).toContain('Coming soon')
  })

  it('audio: synth on/off, output pair, master volume', async () => {
    const s = setup()
    await fireEvent.click(byTip('audio.synth_on')[0])
    expect(s.state.io.synth!.muted).toBe(true)
    expect(byTip('audio.output')[0].getAttribute('aria-checked')).toBe('true')
    const master = byTip('mixer.master')[0]
    await fireEvent.keyDown(master, { key: 'PageDown' })
    expect(s.state.mixer.master).toBe(90)
  })

  it('switches the SoundFont through the settings adapter', async () => {
    setup()
    const fonts = byTip('audio.soundfont')
    expect(fonts.length).toBeGreaterThan(1)
    expect(fonts[0].getAttribute('aria-checked')).toBe('true') // GeneralUser-GS, from io.synth
    await fireEvent.click(fonts[1])
    expect(fonts[1].getAttribute('aria-checked')).toBe('true')
    expect(settings.soundFontFile).toBe('FluidR3_GM.sf2')
  })

  it('MIDI: merging all inputs, or picking sources', async () => {
    setup()
    const sources = () => byTip('midi.input')
    expect(sources().every((b) => b.getAttribute('aria-checked') === 'true')).toBe(true)
    await fireEvent.click(sources()[0]) // switching one off in All mode moves to Selected
    expect(settings.allInputs).toBe(false)
    expect(sources()[0].getAttribute('aria-checked')).toBe('false')
    expect(sources()[1].getAttribute('aria-checked')).toBe('true')
    await fireEvent.click(byTip('midi.merge_all')[0])
    expect(settings.allInputs).toBe(true)
    expect(page('midi').textContent).toContain('yahaha')
    expect(page('midi').textContent).toContain('Connected')
    await fireEvent.click(byTip('midi.palette_leds')[0])
    expect(settings.paletteLeds).toBe(true)
  })

  it('on the real engine, settings it lacks are badged, inert and show no sample data', async () => {
    const s = new MockSession({ manual: true, demo: false })
    app.attach(s)
    app.kind = 'tauri' // what a Tauri session reports; the mock state stands in for the engine's
    ui.settings = true
    flushSync()
    render(Settings)
    expect(document.querySelectorAll('.badge.mock').length).toBe(4) // SoundFont, Inputs, Palette LEDs, Style folders

    const fonts = byTip('audio.soundfont')
    expect(fonts).toHaveLength(1) // only the one the synth plays, no made-up files
    await fireEvent.click(fonts[0])
    expect(settings.soundFontFile).toBe(null)

    const sources = byTip('midi.input')
    expect(sources).toHaveLength(s.state.io.inputs.length) // no mock-only sources
    await fireEvent.click(sources[0])
    expect(settings.chosen).toBe(null)
    expect(sources[0].getAttribute('aria-checked')).toBe('true')
    expect(byTip('midi.merge_all').every((b) => b.getAttribute('aria-checked') === 'false')).toBe(true)
    await fireEvent.click(byTip('midi.merge_all')[1])
    expect(settings.allInputs).toBe(true)

    await fireEvent.click(byTip('midi.palette_leds')[0])
    expect(settings.paletteLeds).toBe(false)
    expect(byTip('midi.palette_leds')[0].textContent).toContain('Set at launch')

    expect(byTip('settings.style_folders')[0].textContent).toContain("doesn't report")
    await fireEvent.click(byTip('settings.rescan')[0])
    expect(page('library').textContent).not.toContain('Scanning')
  })

  it('library: lists the folders and rescans', async () => {
    vi.useFakeTimers()
    try {
      setup()
      expect(byTip('settings.style_folders')[0].querySelectorAll('li').length).toBeGreaterThan(0)
      await fireEvent.click(byTip('settings.rescan')[0])
      expect(page('library').textContent).toContain('Scanning')
      vi.advanceTimersByTime(1500)
      flushSync()
      expect(page('library').textContent).toContain('Rescan styles')
    } finally {
      vi.useRealTimers()
    }
  })
})

describe('split strip geometry', () => {
  const keys = keysBetween(SPLIT_MIN, SPLIT_MAX)

  it('covers C0–C6 with 43 white keys', () => {
    expect(keys.filter((k) => !k.black)).toHaveLength(43)
    expect(noteName(SPLIT_MIN)).toBe('C0')
    expect(noteName(SPLIT_MAX)).toBe('C6')
    expect(noteName(60)).toBe('C3')
    expect(noteName(54)).toBe('F#2')
    expect(noteName(56)).toBe('Ab2')
  })

  it('finds black keys in the top part and white keys below', () => {
    // White key 0 is C0 (24); the C#0 black key straddles the line at x = 1.
    expect(keyAt(keys, 0.5, 0.9)).toBe(24)
    expect(keyAt(keys, 1.0, 0.3)).toBe(25)
    expect(keyAt(keys, 1.0, 0.9)).toBe(26)
    expect(keyAt(keys, 2.1, 0.3)).toBe(27)
    expect(keyAt(keys, 2.9, 0.3)).toBe(28)
    expect(keyAt(keys, 3.0, 0.3)).toBe(29) // E–F: no black key between them
    expect(keyAt(keys, 99, 0.9)).toBe(96)
  })
})
