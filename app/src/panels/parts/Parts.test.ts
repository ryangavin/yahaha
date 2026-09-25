import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { MockSession } from '../../lib/api/mock'
import { mirror } from '../../lib/mirror.svelte'
import { app, ui } from '../../lib/store.svelte'
import Launchkey from '../launchkey/Launchkey.svelte'
import Parts from './Parts.svelte'
import { layerText, leftZone, pluginStatusLine } from './parts'
import { pluginBadge, pluginTip } from '../mixer/voice'

function setup(demo = true) {
  const session = new MockSession({ manual: true, demo })
  app.attach(session)
  flushSync()
  const r = render(Parts)
  return { session, ...r }
}

afterEach(() => {
  cleanup()
  app.detach()
  mirror.panelFader = null
  ui.soundBrowser = null
})

const tipped = (key: string) => document.querySelector<HTMLElement>(`[data-tip="${key}"]`)!
const strip = (name: string) => document.querySelector<HTMLElement>(`.strip[aria-label="${name}"]`)!

describe('Keyboard parts drawer', () => {
  it('shows the four parts with voice, on/off, volume and octave', () => {
    setup()
    for (const name of ['Right 1', 'Right 2', 'Right 3', 'Left']) expect(strip(name)).toBeTruthy()
    expect(strip('Right 1').textContent).toContain('Stage Grand')
    expect(strip('Right 2').querySelector('[role="slider"]')!.getAttribute('aria-valuenow')).toBe('72')
    expect(strip('Right 1').querySelector('.edit')!.getAttribute('aria-pressed')).toBe('true')
  })

  it('says which Right parts are layered', () => {
    const { session } = setup()
    expect(document.querySelector('.layertext')!.textContent).toBe('Layer: R1 + R2')
    session.send({ type: 'togglePart', part: 1 })
    flushSync()
    expect(document.querySelector('.layertext')!.textContent).toBe('Right 1 alone')
    expect(layerText(session.state.keyboardParts)).toBe('Right 1 alone')
  })

  it('sends the part commands', async () => {
    const { session } = setup()
    await fireEvent.click(tipped('part.right3.on'))
    expect(session.state.keyboardParts[2].on).toBe(true)
    await fireEvent.click(tipped('part.left.select'))
    expect(session.state.keyboardParts[3].selected).toBe(true)
    await fireEvent.click(strip('Right 1').querySelector('[data-tip="part.octave_up"]')!)
    expect(session.state.keyboardParts[0].octave).toBe(1)
    // The voice screen opens the Sound Browser for that part (#117).
    await fireEvent.click(strip('Right 2').querySelector('[data-tip="part.voice"]')!)
    expect(ui.soundBrowser).toBe(1)
    // What the part plays shows on the screen, with where it comes from.
    session.send({ type: 'setPartPatch', part: 1, id: 'warm-rhodes' })
    flushSync()
    expect(strip('Right 2').querySelector('[data-tip="part.voice"]')!.textContent).toContain('Warm Rhodes')
    expect(strip('Right 2').querySelector('[data-tip="part.voice"]')!.textContent).toContain('Saved')
  })

  it('recalling an OTS updates the parts and marks the faders it moved as waiting', async () => {
    const { session } = setup()
    await fireEvent.click(tipped('ots.4'))
    flushSync()
    expect(session.state.ots.applied).toBe(4)
    expect(strip('Right 1').textContent).toContain('Alto Sax')
    expect(strip('Right 1').querySelector('.pickup.on')).toBeTruthy()
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
    // The default (owner preference): at the Main section change.
    expect(tipped('ots.link_timing').textContent).toContain('At Main Section Change')
    session.send({ type: 'setOtsLinkTiming', timing: 'immediate' })
    flushSync()
    expect(tipped('ots.link_timing').textContent).toContain('Immediate')
  })

  it('under Manual Bass, Left plays the Bass voice and the style Bass is marked', () => {
    const { session } = setup()
    session.send({ type: 'toggleUpper' })
    flushSync()
    expect(session.state.chord.manualBassActive).toBe(true)
    expect(strip('Left').textContent).toContain('Finger Bass')
    expect(strip('Left').textContent).toContain('own: Strings')
    expect(leftZone(session.state).who).toBe('bass')
    expect(document.querySelector('.written .muted')!.textContent).toContain('your left hand')
  })

  it('Manual Bass is dark in Lower, where the engine ignores it, and lit only when in effect', () => {
    const { session } = setup()
    const mb = tipped('detection.manual_bass')
    expect(session.state.chord.manualBass).toBe(true) // the setting, kept for Upper
    expect(mb.getAttribute('aria-checked')).toBe('false')
    expect(mb.parentElement!.textContent).toContain('Upper only')
    session.send({ type: 'toggleUpper' })
    flushSync()
    expect(mb.getAttribute('aria-checked')).toBe('true')
  })

  it('lists the voice each style part was written for', () => {
    setup()
    const rows = document.querySelectorAll('.written li')
    expect(rows).toHaveLength(8)
    expect(rows[5].textContent).toContain('ch 14')
    expect(rows[5].textContent).toContain('≈ Strings')
  })

  it('hovering a part lights its fader on the mirror, or the page button on the Style page', async () => {
    const { session } = setup()
    render(Launchkey)
    await fireEvent.pointerEnter(strip('Right 2'))
    expect(mirror.panelFader).toBe(1)
    const bank = document.querySelector('[aria-label="Faders"]')!
    expect([...bank.children].indexOf(bank.querySelector('.linked')!)).toBe(1)
    session.send({ type: 'toggleFaderPage' })
    flushSync()
    expect([...bank.children].indexOf(bank.querySelector('.linked')!)).toBe(8)
    await fireEvent.pointerLeave(strip('Right 2'))
    expect(bank.querySelector('.linked')).toBeNull()
  })
})

describe('plugin status', () => {
  it('In proc sets the plugin\'s run-in-process override, which the next load follows', async () => {
    const { session } = setup()
    session.send({ type: 'setPartPlugin', part: 0, id: 'aumu Mock Demo', state: null })
    session.advance(1000)
    flushSync()
    const btn = strip('Right 1').querySelector<HTMLElement>('[data-tip="part.plugin_in_process"]')!
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
