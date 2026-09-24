// Every interactive element in the app has a tooltip from the catalog.
//
// Renders the whole app on the mock session in every state that shows different
// controls (each overlay, each pad page, each fader page, help mode) and checks every
// focusable or clickable element (the help footer's own switch included) for a `data-tip`
// key that exists in the catalog: that entry is what the help footer shows on hover.
// A new panel is covered automatically once it's in App.svelte; if it shows controls only
// in some state, add that state to STATES below.

import { render, cleanup } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import App from '../App.svelte'
import { MockSession } from '../lib/api/mock'
import { ui } from '../lib/store.svelte'
import { tips } from '../lib/tooltip/tip.svelte'
import { nav as soundNav } from '../panels/sound/nav.svelte'
import { TIPS, isTipKey } from './tooltips'

export const INTERACTIVE = [
  'button',
  'a[href]',
  'input',
  'select',
  'textarea',
  'summary',
  '[role="button"]',
  '[role="slider"]',
  '[role="tab"]',
  '[role="switch"]',
  '[role="checkbox"]',
  '[role="option"]',
  '[contenteditable="true"]',
  '[tabindex]:not([tabindex="-1"])',
].join(',')

function describeEl(el: Element): string {
  const html = el.outerHTML
  return html.length > 160 ? html.slice(0, 160) + '…' : html
}

/** Elements in `root` that someone can click or focus but that have no valid tooltip. */
export function untipped(root: ParentNode): string[] {
  const bad: string[] = []
  for (const el of root.querySelectorAll(INTERACTIVE)) {
    // Hidden tabs of a tablist are still controls: they count.
    const key = el.getAttribute('data-tip')
    if (!key) bad.push(`no data-tip: ${describeEl(el)}`)
    else if (!isTipKey(key)) bad.push(`data-tip "${key}" is not in the catalog: ${describeEl(el)}`)
  }
  return bad
}

type Setup = (s: MockSession) => void

const STATES: [string, Setup][] = [
  ['main screen, playing (demo)', () => {}],
  ['stopped, Sync Start armed', (s) => s.send({ type: 'toggleSyncStart' })],
  ['pad page 2', (s) => s.send({ type: 'setPadPage', page: 'chordSetup' })],
  ['pad page 3', (s) => s.send({ type: 'setPadPage', page: 'otsParts' })],
  ['fader page Style', (s) => s.send({ type: 'toggleFaderPage' })],
  ['Upper + Manual Bass', (s) => s.send({ type: 'toggleUpper' })],
  ['help mode (expanded help footer)', () => (tips.help = true)],
  ['pop-up tips on', () => tips.setFloating(true)],
  ['style browser open', () => (ui.browser = true)],
  ['style browser open, stopped (preview buttons)', (s) => (s.send({ type: 'stop' }), (ui.browser = true))],
  ['style browser, previewing', (s) => (s.send({ type: 'stop' }), s.send({ type: 'auditionStyle', id: 1 }), (ui.browser = true))],
  ['style browser, style queued for the next bar', (s) => (s.send({ type: 'queueStyle', id: 1 }), (ui.browser = true))],
  ['settings open', () => (ui.settings = true)],
  ['settings open, a pitch-bend pedal learning its CC', (s) => {
    s.send({ type: 'setPedal', pedal: 2, cc: 4, function: 'pitchBend', controlType: 'holdA', reverse: false, range: 'full' })
    ui.settings = true
  }],
  ['parts drawer open', () => (ui.parts = true)],
  ['parts drawer, Upper + Manual Bass, OTS Link', (s) => ((ui.parts = true), s.send({ type: 'toggleUpper' }), s.send({ type: 'toggleOtsLink' }))],
  ['parts drawer, fader page Style', (s) => ((ui.parts = true), s.send({ type: 'toggleFaderPage' }))],
  ['parts drawer, Right 1 on a plugin', (s) => ((ui.parts = true), s.send({ type: 'setPartPlugin', part: 0, id: 'aumu dls  appl', state: null }), s.advance(1000))],
  ['parts drawer, Right 2 loading a plugin', (s) => ((ui.parts = true), s.send({ type: 'setPartPlugin', part: 1, id: 'aumu samp appl', state: null }))],
  ['mixer drawer open', () => (ui.mixer = true)],
  ['mixer drawer, a plugin part', (s) => ((ui.mixer = true), s.send({ type: 'setPartPlugin', part: 0, id: 'aumu dls  appl', state: null }), s.advance(1000))],
  ['mixer drawer open, Style tab', (s) => ((ui.mixer = true), s.send({ type: 'setFaderPage', page: 'style' }))],
  ['chord looper drawer open', () => (ui.looper = true)],
  ['chord looper drawer, recording armed, Memory latched', (s) => ((ui.looper = true), s.send({ type: 'looperRec' }))],
  ['multi pad drawer, no bank', () => (ui.multipad = true)],
  ['multi pad drawer, bank loaded, pads playing and armed', (s) => (
    (ui.multipad = true),
    s.send({ type: 'loadMultiPad', id: 0 }),
    s.send({ type: 'triggerMultiPad', pad: 0 }),
    s.send({ type: 'armMultiPad', pad: 3 })
  )],
  ['sound library drawer: patches, a patch selected', (s) => {
    ui.sound = true
    s.send({ type: 'duplicatePatch', id: 'stage-grand' })
  }],
  ['sound library drawer: program map, this style', () => {
    ui.sound = true
    soundNav.tab = 'map'
    soundNav.styleScope = true
  }],
  ['sound library drawer: this style', () => ((ui.sound = true), (soundNav.tab = 'style'))],
  ['sound library drawer: SoundFont presets, auditioning', (s) => {
    ui.sound = true
    soundNav.tab = 'add'
    s.send({ type: 'stop' })
    s.send({ type: 'browseSoundFont', file: 'GeneralUser-GS.sf2' })
    s.send({ type: 'auditionPreset', file: 'GeneralUser-GS.sf2', bank: 0, program: 4 })
  }],
  ['parts drawer, a part on a library patch', (s) => ((ui.parts = true), s.send({ type: 'setPartPatch', part: 0, id: 'warm-rhodes' }))],
  ['Shift layer on', () => (ui.shiftLatched = true)],
  ['Shift layer on, fader page Style', (s) => ((ui.shiftLatched = true), s.send({ type: 'toggleFaderPage' }))],
]

afterEach(() => {
  cleanup()
  ui.browser = false
  ui.settings = false
  ui.parts = false
  ui.mixer = false
  ui.looper = false
  ui.multipad = false
  ui.sound = false
  soundNav.tab = 'patches'
  soundNav.styleScope = false
  soundNav.selected = null
  ui.shiftLatched = false
  tips.help = false
  tips.setFloating(false)
})

describe('tooltip coverage', () => {
  for (const [name, setup] of STATES) {
    it(`every interactive element has a catalog tooltip: ${name}`, () => {
      const session = new MockSession({ demo: true, manual: true })
      render(App, { props: { session } })
      setup(session)
      session.advance(16)
      flushSync()
      const found = document.body.querySelectorAll(INTERACTIVE).length
      expect(found, 'the app rendered no controls at all').toBeGreaterThan(10)
      expect(untipped(document.body)).toEqual([])
    })
  }

  it('the checker catches a control without a tooltip', () => {
    document.body.innerHTML = '<button>x</button><div role="slider" tabindex="0" data-tip="nope"></div><button data-tip="transport.start_stop">ok</button>'
    expect(untipped(document.body)).toHaveLength(2)
  })
})

describe('catalog entries', () => {
  for (const [key, t] of Object.entries(TIPS)) {
    it(`${key} is complete`, () => {
      expect(t.title.trim(), 'title').not.toBe('')
      const sentences = t.body.split(/(?<=[.!?])\s+/).filter(Boolean)
      expect(sentences.length, `body should be 1–3 plain sentences: ${t.body}`).toBeGreaterThanOrEqual(1)
      expect(sentences.length, `body should be 1–3 plain sentences (…or 5 for the lamp legend): ${t.body}`).toBeLessThanOrEqual(key === 'section.lamps' ? 5 : 3)
      expect(Array.isArray(t.keys)).toBe(true)
      expect(t.launchkey === null || t.launchkey.trim() !== '').toBe(true)
    })
  }
})
