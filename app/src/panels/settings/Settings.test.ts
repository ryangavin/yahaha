import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { MockSession } from '../../lib/api/mock'
import type { AppState } from '../../lib/api/types'
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
})

const q = <T extends HTMLElement = HTMLElement>(sel: string) => document.querySelector<T>(sel)!
const byTip = (key: string) => [...document.querySelectorAll<HTMLElement>(`[data-tip="${key}"]`)]
const page = (id: string) => q(`#settings-page-${id}`)

describe('Settings drawer', () => {
  it('groups the pages like the Genos menus, one visible at a time', async () => {
    setup()
    const tabs = [...document.querySelectorAll('[role="tab"]')].map((t) => t.textContent?.trim())
    expect(tabs).toEqual(['Chord', 'Split', 'Transpose', 'Style', 'Pedals', 'Lock', 'Audio', 'MIDI', 'Launchkey', 'Library'])
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

  it('Style page: Style Dynamics controls send the dynamics commands (#180)', async () => {
    const s = setup()
    await fireEvent.click(q('#settings-tab-style'))
    const style = page('style')
    const at = (key: string) => style.querySelector<HTMLElement>(`[data-tip="${key}"]`)!
    for (const k of ['dynamics.control', 'dynamics.level', 'dynamics.touch', 'dynamics.accent', 'dynamics.accent_threshold']) {
      expect(at(k), k).not.toBeNull()
    }
    await fireEvent.click(at('dynamics.accent'))
    await fireEvent.click(at('dynamics.touch'))
    await fireEvent.click(at('dynamics.control'))
    expect(s.state.dynamics).toMatchObject({ control: false, touch: true, accent: true })
  })

  it('Style page: Tempo and the section buttons the Launchkey has (hardware parity)', async () => {
    const s = setup()
    await fireEvent.click(q('#settings-tab-style'))
    const style = page('style')
    const at = (key: string) => style.querySelector<HTMLElement>(`[data-tip="${key}"]`)!
    for (const k of ['tempo.set', 'section.main_a', 'section.main_d', 'section.break', 'transport.stop', 'transport.section_reset']) {
      expect(at(k), k).not.toBeNull()
    }
    const sent = vi.spyOn(s, 'send')
    await fireEvent.click(at('section.main_c'))
    expect(sent).toHaveBeenCalledWith({ type: 'main', index: 2 })
  })

  it('Lock page: a toggle per Parameter Lock group, wired to setParamLock', async () => {
    const s = setup()
    await fireEvent.click(q('#settings-tab-lock'))
    expect(page('lock').hidden).toBe(false)
    const split = page('lock').querySelector<HTMLElement>('[data-tip="settings.param_lock_split_point"]')!
    const fing = page('lock').querySelector<HTMLElement>('[data-tip="settings.param_lock_fingering_type"]')!
    expect(split.getAttribute('aria-checked')).toBe('false')
    await fireEvent.click(split)
    expect(s.state.paramLocks).toEqual({ splitPoint: true, fingeringType: false })
    expect(split.getAttribute('aria-checked')).toBe('true')
    await fireEvent.click(fing)
    await fireEvent.click(split)
    expect(s.state.paramLocks).toEqual({ splitPoint: false, fingeringType: true })
    expect(fing.textContent?.trim()).toBe('Locked')
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

  it('style toggles send the transport commands', async () => {
    const s = setup()
    const before = s.state.transport.autoFill
    await fireEvent.click(byTip('transport.auto_fill')[0])
    expect(s.state.transport.autoFill).toBe(!before)
  })

  it('style: OTS Link timing, Stop Accompaniment mode, Half Bar Fill, fills and Change Behavior are live', async () => {
    const s = setup()
    const opt = (key: string, label: string) => byTip(key).find((b) => b.textContent?.trim() === label)!
    for (const el of byTip('settings.ots_link_timing')) expect(el.getAttribute('aria-disabled')).toBeNull()
    await fireEvent.click(opt('settings.ots_link_timing', 'At Main Section Change'))
    expect(s.state.ots.linkTiming).toBe('mainChange')
    await fireEvent.click(byTip('settings.stop_acmp_fixed')[0])
    expect([s.state.transport.stopAcmpMode, s.state.transport.stopAcmp]).toEqual(['fixed', true])
    await fireEvent.click(byTip('settings.stop_acmp_off')[0])
    expect(s.state.transport.stopAcmp).toBe(false)
    await fireEvent.click(byTip('transport.half_bar_fill')[0])
    expect(s.state.transport.halfBarFill).toBe(true)
    await fireEvent.click(byTip('transport.fill_up')[0])
    expect(s.state.transport.main).toBe(1)
    await fireEvent.click(opt('settings.tempo_change', 'Lock'))
    await fireEvent.click(opt('settings.parts_change', 'Reset'))
    await fireEvent.click(opt('settings.section_set', 'C'))
    expect(s.state.styleChange).toEqual({ tempo: 'lock', parts: 'reset', sectionSet: 2 })
    await fireEvent.click(opt('settings.section_set', 'Off'))
    expect(s.state.styleChange.sectionSet).toBeNull()
  })

  it('section change timing, Synchro Stop window, fade, Section Reset and Retrigger are live', async () => {
    const s = setup()
    const [nextBar, immediate] = byTip('settings.section_timing')
    expect(nextBar.getAttribute('aria-checked')).toBe('true')
    await fireEvent.click(immediate)
    expect(s.state.styleSettings.mainTiming).toBe('immediate')
    await fireEvent.click(byTip('settings.intro_ending_timing')[1])
    expect(s.state.styleSettings.introEndingTiming).toBe('endOfSection')
    const win = byTip('settings.synchro_stop_window')[0]
    expect(win.getAttribute('aria-disabled')).toBeNull()
    expect(win.getAttribute('aria-valuetext')).toBe('Off')
    await fireEvent.keyDown(win, { key: 'PageUp' })
    expect(s.state.styleSettings.syncStopWindowMs).toBe(1000)
    await fireEvent.keyDown(byTip('settings.fade_out')[0], { key: 'ArrowRight' })
    expect(s.state.styleSettings.fadeOutMs).toBe(5100)
    await fireEvent.click(byTip('settings.section_reset')[0])
    expect(s.state.styleSettings.sectionReset).toBe(false)
    const rates = byTip('settings.retrigger_rate')
    expect(rates.map((r) => r.textContent?.trim())).toEqual(['1', '1/2', '1/4', '1/8', '1/16', '1/32'])
    await fireEvent.click(rates[4])
    expect(s.state.styleSettings.retriggerRate).toBe(16)
    await fireEvent.click(byTip('transport.retrigger')[0])
    expect(s.state.transport.retrigger).toBe(true)
    // Stopped: Fade arms a fade in.
    s.send({ type: 'stop' })
    flushSync()
    await fireEvent.click(byTip('transport.fade')[0])
    expect(s.state.transport.fade).toBe('armed')
    expect(page('style').textContent).toContain('Armed')
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

  it('has no default sound set: the GM map decides (docs/sound-browser.md)', () => {
    setup()
    expect(page('audio').textContent).not.toContain('Default sound set')
    expect(document.querySelector('[data-tip^="audio.soundfont"]')).toBeNull()
  })

  it('MIDI: merging all inputs, or picking sources', async () => {
    const s = setup()
    const sources = () => byTip('midi.input')
    expect(sources().every((b) => b.getAttribute('aria-checked') === 'true')).toBe(true)
    await fireEvent.click(sources()[0]) // switching one off in All mode moves to Selected
    expect(s.state.io.allInputs).toBe(false)
    expect(sources()[0].getAttribute('aria-checked')).toBe('false')
    expect(sources()[1].getAttribute('aria-checked')).toBe('true')
    await fireEvent.click(byTip('midi.merge_all')[0])
    expect(s.state.io.allInputs).toBe(true)
    expect(page('midi').textContent).toContain('yahaha')
    expect(page('midi').textContent).toContain('Connected')
    await fireEvent.click(byTip('midi.palette_leds')[0])
    expect(s.state.pads.paletteLeds).toBe(true)
  })

  it('on the real engine every setting is live: no badges', async () => {
    const s = setup()
    app.kind = 'tauri'
    flushSync()
    expect(document.querySelectorAll('.badge.mock').length).toBe(0)
    await fireEvent.click(byTip('settings.rescan')[0])
    expect(s.state.library.scanning).toBe(true)
  })

  it('on an engine older than these settings, they are badged, inert and show no sample data', async () => {
    const s = new MockSession({ manual: true, demo: false })
    const st = structuredClone(s.state) as unknown as { io: Record<string, unknown>; library: Record<string, unknown> }
    for (const k of ['sources', 'allInputs', 'soundFonts', 'soundFontFile', 'soundFontLoading', 'defaultSoundSet', 'autoSoundSet']) delete st.io[k]
    for (const k of ['roots', 'scanning']) delete st.library[k]
    const sent = vi.fn()
    app.attach({ kind: 'tauri', subscribe: (fn) => (fn(st as unknown as AppState), () => {}), send: sent, library: () => s.library(), meters: () => s.meters(), dispose: () => {} })
    ui.settings = true
    flushSync()
    render(Settings)
    expect(document.querySelectorAll('.badge.mock').length).toBe(3) // Inputs, Palette LEDs, Style folders

    const sources = byTip('midi.input')
    expect(sources).toHaveLength(s.state.io.inputs.length) // the inputs it has open
    await fireEvent.click(sources[0])
    expect(sources[0].getAttribute('aria-checked')).toBe('true')
    expect(byTip('midi.merge_all').every((b) => b.getAttribute('aria-checked') === 'false')).toBe(true)
    await fireEvent.click(byTip('midi.merge_all')[1])
    await fireEvent.click(byTip('midi.palette_leds')[0])
    expect(byTip('settings.style_folders')[0].textContent).toContain("doesn't report")
    await fireEvent.click(byTip('settings.rescan')[0])
    expect(page('library').textContent).not.toContain('Scanning')
    expect(sent).not.toHaveBeenCalled() // nothing sent that the engine can't run
  })

  it('library: lists the folders and rescans', async () => {
    const s = setup()
    expect(byTip('settings.style_folders')[0].querySelectorAll('li').length).toBeGreaterThan(0)
    await fireEvent.click(byTip('settings.rescan')[0])
    expect(page('library').textContent).toContain('Scanning')
    s.advance(1500)
    flushSync()
    expect(page('library').textContent).toContain('Rescan styles')
  })

  describe('Launchkey page: the pad page order (docs/eyes-free.md)', () => {
    const rows = () => [...page('launchkey').querySelectorAll<HTMLElement>('ol .row')].map((r) => r.textContent?.replace(/[▲▼✕\s]+/g, ' ').trim())
    const inRow = (p: string, key: string) => page('launchkey').querySelector<HTMLButtonElement>(`ol [data-page="${p}"] [data-tip="${key}"]`)!

    it('shows Sections fixed, then the order from the state', async () => {
      setup()
      await fireEvent.click(q('#settings-tab-launchkey'))
      expect(page('launchkey').hidden).toBe(false)
      expect(rows()).toEqual(['1 Sections Fixed', '2 Racks', '3 Chord', '4 Multi Pads', '5 Setup'])
      expect(page('launchkey').querySelector('.fixed button')).toBeNull()
      expect(inRow('racks', 'settings.pad_pages.up').disabled).toBe(true)
      expect(inRow('setup', 'settings.pad_pages.down').disabled).toBe(true)
      expect(byTip('settings.pad_pages.reset')[0]).toHaveProperty('disabled', true)
    })

    it('move up/down sends setPadPageOrder with the new order', async () => {
      const s = setup()
      const sent = vi.spyOn(s, 'send')
      await fireEvent.click(inRow('setup', 'settings.pad_pages.up'))
      expect(sent).toHaveBeenLastCalledWith({ type: 'setPadPageOrder', pages: ['racks', 'chord', 'setup', 'multiPads'] })
      flushSync()
      expect(rows()).toEqual(['1 Sections Fixed', '2 Racks', '3 Chord', '4 Setup', '5 Multi Pads'])
      await fireEvent.click(inRow('racks', 'settings.pad_pages.down'))
      expect(sent).toHaveBeenLastCalledWith({ type: 'setPadPageOrder', pages: ['chord', 'racks', 'setup', 'multiPads'] })
      expect(s.state.settings.padPages).toEqual(['chord', 'racks', 'setup', 'multiPads'])
    })

    it('trims a page, adds it back last, and resets to the default', async () => {
      const s = setup()
      const sent = vi.spyOn(s, 'send')
      s.send({ type: 'setPadPage', page: 'chord' })
      await fireEvent.click(inRow('chord', 'settings.pad_pages.remove'))
      expect(sent).toHaveBeenLastCalledWith({ type: 'setPadPageOrder', pages: ['racks', 'multiPads', 'setup'] })
      expect(s.state.pads.page).toBe('sections')
      flushSync()
      expect(rows()).toEqual(['1 Sections Fixed', '2 Racks', '3 Multi Pads', '4 Setup'])
      const add = byTip('settings.pad_pages.add')
      expect(add).toHaveLength(1)
      expect(add[0].closest('[data-page]')?.getAttribute('data-page')).toBe('chord')
      await fireEvent.click(add[0])
      expect(sent).toHaveBeenLastCalledWith({ type: 'setPadPageOrder', pages: ['racks', 'multiPads', 'setup', 'chord'] })
      flushSync()
      expect(byTip('settings.pad_pages.add')).toHaveLength(0)
      await fireEvent.click(byTip('settings.pad_pages.reset')[0])
      expect(sent).toHaveBeenLastCalledWith({ type: 'setPadPageOrder', pages: ['racks', 'chord', 'multiPads', 'setup'] })
    })

    it('every page can be left out, leaving Sections alone', async () => {
      const s = setup()
      for (const p of ['racks', 'chord', 'multiPads', 'setup']) {
        await fireEvent.click(inRow(p, 'settings.pad_pages.remove'))
        flushSync()
      }
      expect(s.state.settings.padPages).toEqual([])
      expect(rows()).toEqual(['1 Sections Fixed'])
      expect(byTip('settings.pad_pages.add')).toHaveLength(4)
    })
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
