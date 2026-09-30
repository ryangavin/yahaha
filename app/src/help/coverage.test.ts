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
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import App from '../App.svelte'
import { MockSession } from '../lib/api/mock'
import { ui } from '../lib/store.svelte'
import { tips } from '../lib/tooltip/tip.svelte'
import { nav as soundNav } from '../panels/sound/nav.svelte'
import { libraryNav } from '../panels/library/nav.svelte'
import { channelNav } from '../panels/channel/nav.svelte'
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

/** Clicks the first control with tooltip `key` (a state that needs a click to show). */
function click(key: string) {
  const el = document.querySelector<HTMLElement>(`[data-tip="${key}"]`)
  if (!el) throw new Error(`no control with tooltip ${key}`)
  el.click()
  flushSync()
}

/** Save the live rack as `name` through Store on Quick Rack `slot` (the bar's Store flow). */
function storeRack(s: MockSession, slot: number, name: string) {
  s.send({ type: 'toggleQuickRackStore' })
  s.send({ type: 'pressQuickRack', slot })
  s.send({ type: 'saveRackAs', name })
}

const STATES: [string, Setup][] = [
  ['main screen, playing (demo)', () => {}],
  ['stopped, Sync Start armed', (s) => s.send({ type: 'toggleSyncStart' })],
  ['pad page 2 (Racks)', (s) => s.send({ type: 'setPadPage', page: 'racks' })],
  ['pad page 3 (Chord)', (s) => s.send({ type: 'setPadPage', page: 'chord' })],
  ['pad page 4 (Multi Pads)', (s) => s.send({ type: 'setPadPage', page: 'multiPads' })],
  ['pad page 5 (Setup)', (s) => s.send({ type: 'setPadPage', page: 'setup' })],
  ['fader page Style', (s) => s.send({ type: 'toggleFaderPage' })],
  ['Upper + Manual Bass', (s) => s.send({ type: 'toggleUpper' })],
  ['help mode (expanded help footer)', () => (tips.help = true)],
  ['pop-up tips on', () => tips.setFloating(true)],
  ['style browser open', () => (ui.browser = true)],
  ['style browser open, stopped (preview buttons)', (s) => (s.send({ type: 'stop' }), (ui.browser = true))],
  ['style browser, previewing', (s) => (s.send({ type: 'stop' }), s.send({ type: 'auditionStyle', id: 1 }), (ui.browser = true))],
  ['style browser, style queued for the next bar', (s) => (s.send({ type: 'queueStyle', id: 1 }), (ui.browser = true))],
  // Library (docs/racks.md, "Screens"): each tab, and the states that show more controls.
  ['Library › Sounds on Right 1 (a SoundFont voice: Copy to My Sounds)', () => ui.openLibrary('sounds', 0)],
  ['Library › Sounds, an edited plugin sound: edited, Save as… open, instrument filter', (s) => {
    s.send({ type: 'stop' })
    s.send({ type: 'listPluginPresets', id: 'au:aumu Smp7 Fake' })
    s.send({ type: 'setPartPluginPreset', part: 0, id: 'aumu Smp7 Fake', preset: 'f:1' })
    s.advance(5000)
    s.pluginWindow(0, 1)
    libraryNav.instrument = 'au:aumu Smp7 Fake'
    ui.openLibrary('sounds', 0)
    flushSync()
    click('sounds.save')
  }],
  ['Library › Sounds, a sound of yours selected: details, delete asked', (s) => {
    s.send({ type: 'setPartPatch', part: 0, id: 'warm-rhodes' })
    ui.openLibrary('sounds', 0)
    flushSync()
    click('sounds.more')
    click('sound.delete')
  }],
  ['Library › Instruments, More… on a playing plugin (category, in process, Edit…)', (s) => {
    s.send({ type: 'setPartPlugin', part: 0, id: 'aumu Smp7 Fake', state: null })
    s.advance(1000)
    ui.openLibrary('instruments', 0)
    flushSync()
    document.querySelector<HTMLElement>('section[aria-label="Sampler Deluxe"] [data-tip="library.inst_more"]')!.click()
    flushSync()
  }],
  ['Library › Racks, Needs attention on', () => {
    ui.openLibrary('racks', 0)
    flushSync()
    click('library.racks_attention')
  }],
  ['Library › Style map: GM map, global, library file', () => ui.openLibrary('map')],
  ['Library › Style map: GM map, this style with its own rules', (s) => {
    ui.openLibrary('map')
    soundNav.styleScope = true
    s.send({ type: 'setFamilyRule', family: 4, patch: 'soft-pad', style: true })
  }],
  ['Library › Style map: SoundFont presets, auditioning', (s) => {
    ui.openLibrary('map')
    libraryNav.mapPage = 'add'
    s.send({ type: 'stop' })
    s.send({ type: 'browseSoundFont', file: 'GeneralUser-GS.sf2' })
    s.send({ type: 'auditionPreset', file: 'GeneralUser-GS.sf2', bank: 0, program: 4 })
  }],
  ['Library with the Rack drawer open over it', () => ((ui.rack = true), ui.openLibrary('sounds', 1))],
  ['sound browser picking for a map rule', () => (ui.soundPick = { title: 'Piano family', value: 'stage-grand', onpick: () => {} })],
  ['settings open', () => (ui.settings = true)],
  ['settings open, a pitch-bend pedal learning its CC', (s) => {
    s.send({ type: 'setPedal', pedal: 2, cc: 4, function: 'pitchBend', controlType: 'holdA', reverse: false, range: 'full' })
    ui.settings = true
  }],
  ['rack drawer open', () => (ui.rack = true)],
  ['rack drawer, Upper + Manual Bass, OTS Link', (s) => ((ui.rack = true), s.send({ type: 'toggleUpper' }), s.send({ type: 'toggleOtsLink' }))],
  ['rack drawer, fader page Style', (s) => ((ui.rack = true), s.send({ type: 'toggleFaderPage' }))],
  ['rack drawer, Right 1 on a plugin', (s) => ((ui.rack = true), s.send({ type: 'setPartPlugin', part: 0, id: 'aumu dls  appl', state: null }), s.advance(1000))],
  ['rack drawer: modified, controller map open, an edited sound, a missing plugin (Replace…)', (s) => {
    s.send({ type: 'stop' })
    s.send({ type: 'listPluginPresets', id: 'au:aumu Smp7 Fake' })
    s.send({ type: 'setPartPluginPreset', part: 0, id: 'aumu Smp7 Fake', preset: 'f:1' })
    s.advance(5000)
    s.pluginWindow(0, 1)
    s.missingPlugin(2)
    ui.rack = true
    flushSync()
    click('rack.map')
  }],
  ['rack drawer, Right 2 loading a plugin',(s) => ((ui.rack = true), s.send({ type: 'setPartPlugin', part: 1, id: 'aumu samp appl', state: null }))],
  ['mixer details open', () => (ui.mixer = true)],
  ['mixer details, a plugin part', (s) => ((ui.mixer = true), s.send({ type: 'setPartPlugin', part: 0, id: 'aumu dls  appl', state: null }), s.advance(1000))],
  ['effects screen open', () => (ui.effects = true)],
  ['effects screen, an added send, send 1 set by the rack', (s) => (
    (ui.effects = true), s.send({ type: 'addSend', kind: 'phaser' }), s.send({ type: 'setRackSendOverride', send: 0, on: true })
  )],
  ['channel view, a keyboard part with an insert', (s) => (
    s.send({ type: 'addSend', kind: 'room' }), s.send({ type: 'setStripInsertKind', strip: 1, slot: 1, kind: 'compressor' }), channelNav.show(1)
  )],
  ['channel view, a Style part', () => channelNav.show(6)],
  ['effects screen, delay free time, no inserts', (s) => (
    (ui.effects = true),
    s.send({ type: 'setEffectParam', block: 'variation', param: 'delaySync', value: 0 }),
    (s.state.effects.inserts = []),
    s.send({ type: 'setRotaryFast', on: true })
  )],
  ['harmony drawer open', () => (ui.harmony = true)],
  ['harmony drawer, arpeggio on, Fixed velocity', (s) => ((ui.harmony = true), s.send({ type: 'setArpPattern', index: 2 }), s.send({ type: 'setArpVelocity', mode: 'fixed', velocity: 90 }), s.send({ type: 'toggleHarmonyArp' }))],
  ['harmony drawer, Echo type', (s) => ((ui.harmony = true), s.send({ type: 'setHarmonyType', index: 20 }))],
  ['mixer details, the Master editor open', () => {
    ui.mixer = true
    flushSync()
    click('fx.master_edit')
  }],
  ['mixer details open, Style tab',(s) => ((ui.mixer = true), s.send({ type: 'setFaderPage', page: 'style' }))],
  ['charts drawer, nothing imported', () => (ui.charts = true)],
  ['charts drawer, a playlist, chart mode playing', (s) => ((ui.charts = true), s.send({ type: 'importCharts', text: 'irealb://demo' }), s.send({ type: 'setChartMode', on: true }))],
  ['chart in the lead-sheet band', (s) => (s.send({ type: 'importCharts', text: 'irealb://demo' }), s.send({ type: 'setChartMode', on: true }))],
  ['Quick Racks Store armed', (s) => s.send({ type: 'toggleQuickRackStore' })],
  ['Quick Racks: a stored button (clear), pad page 2', (s) => (storeRack(s, 0, 'Ballad'), s.send({ type: 'setPadPage', page: 'racks' }))],
  ['Quick Racks bar: Store waiting for a never-saved rack', (s) => (s.send({ type: 'toggleQuickRackStore' }), s.send({ type: 'pressQuickRack', slot: 2 }))],
  ['Quick Racks bar: Store waiting for a saved rack\'s changes', (s) => (
    storeRack(s, 0, 'Ballad'),
    s.send({ type: 'setPartVoice', part: 0, program: 12 }),
    s.send({ type: 'toggleQuickRackStore' }),
    s.send({ type: 'pressQuickRack', slot: 1 })
  )],
  ['Quick Racks bar: unsaved changes asked', (s) => (storeRack(s, 0, 'Ballad'), s.send({ type: 'newRack' }), s.send({ type: 'setPartVoice', part: 0, program: 12 }), s.send({ type: 'pressQuickRack', slot: 0 }))],
  ['Quick Racks bar: new sound names asked', (s) => {
    s.send({ type: 'listPluginPresets', id: 'au:aumu Smp7 Fake' })
    s.send({ type: 'setPartPluginPreset', part: 0, id: 'aumu Smp7 Fake', preset: 'f:1' })
    s.advance(5000)
    s.pluginWindow(0, 5)
    s.send({ type: 'saveRackAs', name: 'Grand' })
  }],
  ['Library › Racks, racks and a stored button, unsaved changes asked', (s) => (
    storeRack(s, 0, 'Ballad'),
    s.send({ type: 'newRack' }),
    s.send({ type: 'setPartVoice', part: 0, program: 12 }),
    s.send({ type: 'pressQuickRack', slot: 0 }),
    ui.openLibrary('racks')
  )],
  ['Library › Racks, a rack that isn\'t loaded selected: Load, Duplicate, Delete…', (s) => {
    storeRack(s, 0, 'Ballad')
    s.send({ type: 'saveRackAs', name: 'Evening' })
    ui.openLibrary('racks')
    libraryNav.rack = s.state.racks.find((r) => r.name === 'Ballad')!.id
  }],
  ['Library › Racks, delete asked for a rack on a Quick Rack button', (s) => {
    storeRack(s, 0, 'Ballad')
    s.send({ type: 'saveRackAs', name: 'Evening' })
    ui.openLibrary('racks')
    libraryNav.rack = s.state.racks.find((r) => r.name === 'Ballad')!.id
    s.advance(16)
    flushSync()
    click('library.rack_delete')
  }],
  ['chord looper drawer open', () => (ui.looper = true)],
  ['chord looper drawer, recording armed, Memory latched', (s) => ((ui.looper = true), s.send({ type: 'looperRec' }))],
  ['multi pad drawer, no bank', () => (ui.multipad = true)],
  ['multi pad drawer, bank loaded, pads playing and armed', (s) => (
    (ui.multipad = true),
    s.send({ type: 'loadMultiPad', id: 0 }),
    s.send({ type: 'triggerMultiPad', pad: 0 }),
    s.send({ type: 'armMultiPad', pad: 3 })
  )],
  ['rack drawer, a part on a library patch', (s) => ((ui.rack = true), s.send({ type: 'setPartPatch', part: 0, id: 'warm-rhodes' }))],
  ['audio dropout notice', (s) => s.dropouts(5)],
  ['Shift layer on', () => (ui.shiftLatched = true)],
  ['Shift layer on, fader page Style', (s) => ((ui.shiftLatched = true), s.send({ type: 'toggleFaderPage' }))],
]

// Other files leave hover and focus state in the shared tooltip module (isolate: false).
beforeEach(() => {
  ;(document.activeElement as HTMLElement | null)?.blur()
  tips.reset()
})

afterEach(() => {
  cleanup()
  ui.browser = false
  ui.soundPick = null
  ui.view = 'stage'
  ui.libraryTab = 'sounds'
  ui.libraryPart = 0
  libraryNav.reset()
  ui.settings = false
  ui.rack = false
  ui.mixer = false
  ui.effects = false
  channelNav.close()
  ui.selectedPart = 0
  ui.charts = false
  ui.looper = false
  ui.multipad = false
  ui.harmony = false
  soundNav.styleScope = false
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
