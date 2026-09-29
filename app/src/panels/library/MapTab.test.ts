// Library › Style map: the GM map and Add from SoundFont pages (moved here from the Sound
// Library drawer's tests when the drawer went, racks item 13).

import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { GM, MockSession } from '../../lib/api/mock'
import { app, ui } from '../../lib/store.svelte'
import { nav, sourceText } from '../sound/nav.svelte'
import MapTab from './MapTab.svelte'
import { libraryNav } from './nav.svelte'

function setup() {
  const session = new MockSession({ manual: true })
  app.attach(session)
  flushSync()
  render(MapTab)
  return session
}

afterEach(() => {
  cleanup()
  app.detach()
  libraryNav.reset()
  nav.styleScope = false
  ui.soundPick = null
})

const tipped = (key: string) => [...document.querySelectorAll<HTMLElement>(`[data-tip="${key}"]`)]
const tab = (id: 'gm' | 'add') => document.querySelector<HTMLButtonElement>(`#lib-map-${id}`)!
/** A map picker (#117): open it, and pick in the sound picker (catalog id; null clears). */
const pickIn = async (el: HTMLElement, id: string | null) => {
  if (id === null) {
    await fireEvent.click(el.parentElement!.querySelector<HTMLElement>('[data-tip="sound.rule_clear"]')!)
  } else {
    await fireEvent.click(el)
    expect(ui.soundPick).not.toBe(null)
    ui.soundPick!.onpick(id)
    ui.soundPick = null
  }
  flushSync()
}
const change = async (el: HTMLSelectElement | HTMLInputElement, value: string) => {
  el.value = value
  await fireEvent.change(el)
  flushSync()
}

describe('Library › Style map', () => {
  it('the GM map page: drums and 128 programs by family, each with its deciding layer', async () => {
    const s = setup()
    await fireEvent.click(tab('gm'))
    flushSync()
    const page = document.querySelector('#lib-map-page-gm')!
    expect(tipped('sound.export')).toHaveLength(1)
    expect(tipped('sound.family')).toHaveLength(16)
    expect(tipped('sound.drums')).toHaveLength(1)
    expect(tipped('sound.override_patch')).toHaveLength(128)
    const badges = tipped('sound.map_layer')
    expect(badges).toHaveLength(129)
    // Every badge is the row's deciding layer; auto slots are marked.
    const layers = s.state.soundLibrary.gmMap.map((r) => r.resolved.layer)
    expect(badges.map((b) => b.classList[1])).toEqual(layers)
    expect(page.querySelectorAll('.row.auto')).toHaveLength(layers.filter((l) => l === 'auto').length)
    expect(layers).toContain('auto')
    expect(badges[0].textContent).toBe('Drums')
    // The tab counts the auto slots.
    expect(tab('gm').querySelector('small')!.textContent).toBe(String(layers.filter((l) => l === 'auto').length))
  })

  it('the GM map page sets and clears family, override and drum rules, globally or for this style', async () => {
    const s = setup()
    const families = tipped('sound.family')
    await pickIn(families[10], 'saved:warm-rhodes')
    expect(s.state.soundLibrary.map.families[10]).toBe('warm-rhodes')
    flushSync()
    // Program 81 (family 10) now resolves by its family rule.
    expect(tipped('sound.map_layer')[1 + 80].textContent).toBe('Family')
    await pickIn(tipped('sound.drums')[0], null)
    expect(s.state.soundLibrary.map.drums).toBe(null)
    // An override on program 1: the badge says so, and ✕ clears it.
    await pickIn(tipped('sound.override_patch')[0], 'saved:warm-rhodes')
    expect(s.state.soundLibrary.map.overrides.find((o) => o.program === 0)?.patch).toBe('warm-rhodes')
    flushSync()
    expect(tipped('sound.map_layer')[1].textContent).toBe('Override')
    expect(document.querySelector('#lib-map-page-gm .plays')!.textContent).toBe('Warm Rhodes')
    await pickIn(tipped('sound.override_patch')[0], null)
    expect(s.state.soundLibrary.map.overrides.some((o) => o.program === 0)).toBe(false)
    // Any sound: a SoundFont preset becomes a library patch the rule names.
    const n = s.state.soundLibrary.patches.length
    await pickIn(tipped('sound.override_patch')[48], 'sf:FluidR3_GM.sf2:0:48')
    expect(s.state.soundLibrary.patches.length).toBe(n + 1)
    expect(s.state.soundLibrary.map.overrides.find((o) => o.program === 48)?.patch).toBe(s.state.soundLibrary.lastAdded)
    // This style's own rule; the global map is untouched, and the badge marks it.
    await fireEvent.click(tipped('sound.scope')[1])
    flushSync()
    expect(tipped('sound.family')[4].textContent).toContain('↳ Finger Bass')
    await pickIn(tipped('sound.family')[4], 'saved:soft-pad')
    expect(s.state.soundLibrary.styleMap.families[4]).toBe('soft-pad')
    expect(s.state.soundLibrary.map.families[4]).toBe('finger-bass')
    flushSync()
    expect(tipped('sound.map_layer')[1 + 32].textContent).toBe('Familystyle')
    await fireEvent.click(tipped('sound.clear_style_map')[0])
    expect(s.state.soundLibrary.styleMap.families[4]).toBe(null)
  })

  it('every rule picker is keyboard-reachable and opens the sound picker', async () => {
    setup()
    const page = document.querySelector('#lib-map-page-gm')!
    const controls = [...page.querySelectorAll<HTMLElement>('button, select, input')]
    expect(controls.length).toBeGreaterThan(145)
    for (const c of controls) {
      expect(c.tabIndex, c.outerHTML).toBeGreaterThanOrEqual(0)
      expect(c.dataset.tip, c.outerHTML).toBeTruthy()
    }
    // Enter on a picker opens the picker (a native button: Enter clicks it).
    await fireEvent.click(tipped('sound.override_patch')[5])
    expect(ui.soundPick?.title).toBe(`Override for ${GM[5]}`)
  })

  it('browses a SoundFont and adds a preset as a patch', async () => {
    const s = setup()
    await fireEvent.click(tab('add'))
    flushSync()
    expect(libraryNav.mapPage).toBe('add')
    await change(tipped('sound.soundfont')[0] as HTMLSelectElement, 'GeneralUser-GS.sf2')
    expect(s.state.soundLibrary.browse?.presets.length).toBeGreaterThan(128)
    const n = s.state.soundLibrary.patches.length
    await fireEvent.click(tipped('sound.preset_add')[33])
    expect(s.state.soundLibrary.patches).toHaveLength(n + 1)
    expect(s.state.soundLibrary.patches[n].category).toBe('bass')
  })

  it('a patch names its source', () => {
    const s = new MockSession({ manual: true })
    expect(sourceText(s.state.soundLibrary.patches[3])).toBe('GeneralUser-GS · drum kit 1')
    expect(sourceText(s.state.soundLibrary.patches[2])).toBe('GeneralUser-GS · 0:34')
  })
})

describe('library patches on a part (mock session)', () => {
  it('a plugin patch on a part plays its plugin, until the part leaves the patch', () => {
    const s = new MockSession({ manual: true })
    s.send({ type: 'setPartPatch', part: 0, id: 'keys-au' })
    const r1 = () => s.state.keyboardParts[0]
    expect(r1().patch).toBe('keys-au')
    expect(r1().voiceName).toBe('Keys (AU)')
    expect(r1().plugin?.id).toBe('aumu dls  appl')
    // A SoundFont patch: the plugin goes.
    s.send({ type: 'setPartPatch', part: 0, id: 'stage-grand' })
    expect(r1().plugin).toBeUndefined()
    // A GM voice ends a plugin patch and its plugin.
    s.send({ type: 'setPartPatch', part: 0, id: 'keys-au' })
    s.send({ type: 'setPartVoice', part: 0, program: 0 })
    expect([r1().patch, r1().plugin]).toEqual([null, undefined])
    // A plugin picked on the Plugins tab ends the patch, and stays when a patch is left.
    s.send({ type: 'setPartPatch', part: 0, id: 'keys-au' })
    s.send({ type: 'setPartPlugin', part: 0, id: 'aumu dls  appl', state: null })
    expect([r1().patch, r1().plugin?.id]).toEqual([null, 'aumu dls  appl'])
  })
})
