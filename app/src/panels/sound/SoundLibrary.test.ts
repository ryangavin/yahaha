import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { MockSession } from '../../lib/api/mock'
import { app, ui } from '../../lib/store.svelte'
import SoundLibrary from './SoundLibrary.svelte'
import { byCategory, filterPatches, nav, sourceText } from './nav.svelte'

function setup() {
  const session = new MockSession({ manual: true })
  app.attach(session)
  ui.sound = true
  flushSync()
  render(SoundLibrary)
  return session
}

afterEach(() => {
  cleanup()
  app.detach()
  ui.sound = false
  nav.tab = 'map'
  nav.styleScope = false
  ui.soundPick = null
})

const tipped = (key: string) => [...document.querySelectorAll<HTMLElement>(`[data-tip="${key}"]`)]
const tab = (id: string) => document.querySelector<HTMLButtonElement>(`#sound-tab-${id}`)!
/** A map picker (#117): open it, and pick in the Sound Browser (catalog id; null clears). */
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

describe('Sound Library drawer', () => {
  it('opens on the Program Map; the Patches tab folded into the Sound Browser', () => {
    const s = setup()
    expect(document.querySelector('#sound-tab-patches')).toBe(null)
    expect(tab('map').getAttribute('aria-selected')).toBe('true')
    expect(tipped('sound.export')).toHaveLength(1)
    const ps = s.state.soundLibrary.patches
    expect(filterPatches(ps, 'warm', 'all', false).map((p) => p.name)).toEqual(['Warm Rhodes', 'Silk Strings']) // a name and a tag
    expect(filterPatches(ps, '', 'all', true).map((p) => p.name)).toEqual(['Stage Grand', 'Warm Rhodes'])
    // A plugin patch plays itself (the app builds with plugin hosting).
    expect(filterPatches(ps, 'keys', 'all', false)[0].available).toBe(true)
  })

  it('a plugin patch on a part plays its plugin, until the part leaves the patch', () => {
    const s = setup()
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

  it('the program map sets family rules, overrides and the drum rule, globally or for this style', async () => {
    const s = setup()
    await fireEvent.click(tab('map'))
    flushSync()
    const families = tipped('sound.family')
    expect(families).toHaveLength(16)
    await pickIn(families[10], 'saved:warm-rhodes')
    expect(s.state.soundLibrary.map.families[10]).toBe('warm-rhodes')
    await pickIn(tipped('sound.drums')[0], null)
    expect(s.state.soundLibrary.map.drums).toBe(null)
    // An override.
    await change(tipped('sound.override_program')[0] as HTMLSelectElement, '0')
    const pickers = tipped('sound.override_patch')
    await pickIn(pickers[pickers.length - 1], 'saved:warm-rhodes')
    await fireEvent.click(tipped('sound.override_add')[0])
    expect(s.state.soundLibrary.map.overrides.find((o) => o.program === 0)?.patch).toBe('warm-rhodes')
    await fireEvent.click(tipped('sound.override_remove')[0])
    expect(s.state.soundLibrary.map.overrides.some((o) => o.program === 0)).toBe(false)
    // This style's own rule; the global map is untouched.
    await fireEvent.click(tipped('sound.scope')[1])
    flushSync()
    await pickIn(tipped('sound.family')[4], 'saved:soft-pad')
    expect(s.state.soundLibrary.styleMap.families[4]).toBe('soft-pad')
    expect(s.state.soundLibrary.map.families[4]).toBe('finger-bass')
    flushSync()
    await fireEvent.click(tipped('sound.clear_style_map')[0])
    expect(s.state.soundLibrary.styleMap.families[4]).toBe(null)
  })

  it('this style lists what each part sends and remaps it', async () => {
    const s = setup()
    await fireEvent.click(tab('style'))
    flushSync()
    const rows = document.querySelectorAll('#sound-page-style .tr:not(.head)')
    expect(rows).toHaveLength(8)
    const bass = s.state.soundLibrary.usage.find((u) => u.part === 'Bass')!
    expect(bass.plays).toBe('Finger Bass')
    const remaps = tipped('sound.remap')
    await pickIn(remaps[2], 'saved:soft-pad')
    expect(s.state.soundLibrary.map.overrides.find((o) => o.program === bass.gmProgram)?.patch).toBe('soft-pad')
    expect(s.state.soundLibrary.usage.find((u) => u.part === 'Bass')!.plays).toBe('Soft Pad')
    // A drum part's remap is the drum rule.
    await pickIn(remaps[0], 'saved:finger-bass')
    expect(s.state.soundLibrary.map.drums).toBe('finger-bass')
    // Any sound: a SoundFont preset becomes a library patch the rule names.
    const n = s.state.soundLibrary.patches.length
    await pickIn(tipped('sound.remap')[2], 'sf:FluidR3_GM.sf2:0:48')
    expect(s.state.soundLibrary.patches.length).toBe(n + 1)
    expect(s.state.soundLibrary.usage.find((u) => u.part === 'Bass')!.patch).toBe(s.state.soundLibrary.lastAdded)
  })

  it('browses a SoundFont and adds a preset as a patch', async () => {
    const s = setup()
    await fireEvent.click(tab('add'))
    flushSync()
    await change(tipped('sound.soundfont')[0] as HTMLSelectElement, 'GeneralUser-GS.sf2')
    expect(s.state.soundLibrary.browse?.presets.length).toBeGreaterThan(128)
    const n = s.state.soundLibrary.patches.length
    await fireEvent.click(tipped('sound.preset_add')[33])
    expect(s.state.soundLibrary.patches).toHaveLength(n + 1)
    expect(s.state.soundLibrary.patches[n].category).toBe('bass')
  })

  it('helpers', () => {
    const s = new MockSession({ manual: true })
    const groups = byCategory(s.state.soundLibrary.patches)
    expect(groups.map((g) => g.label)).toEqual(['Piano', 'E.Piano', 'Bass', 'Strings', 'Brass', 'Pad', 'Drums/Perc'])
    expect(sourceText(s.state.soundLibrary.patches[3])).toBe('GeneralUser-GS · drum kit 1')
    expect(sourceText(s.state.soundLibrary.patches[2])).toBe('GeneralUser-GS · 0:34')
  })
})
