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
  nav.tab = 'patches'
  nav.styleScope = false
  nav.selected = null
})

const tipped = (key: string) => [...document.querySelectorAll<HTMLElement>(`[data-tip="${key}"]`)]
const tab = (id: string) => document.querySelector<HTMLButtonElement>(`#sound-tab-${id}`)!
const change = async (el: HTMLSelectElement | HTMLInputElement, value: string) => {
  el.value = value
  await fireEvent.change(el)
  flushSync()
}

describe('Sound Library drawer', () => {
  it('lists the patches by category; search, the category and favourites filter them', async () => {
    const s = setup()
    const names = () => tipped('sound.patch').map((b) => b.querySelector('.n')!.textContent)
    expect(names()).toHaveLength(s.state.soundLibrary.patches.length)
    expect([...document.querySelectorAll('.group')].map((g) => g.textContent)[0]).toBe('Piano')
    const search = tipped('sound.search')[0] as HTMLInputElement
    search.value = 'warm'
    await fireEvent.input(search)
    flushSync()
    expect(names()).toEqual(['Warm Rhodes', 'Silk Strings']) // a name and a tag
    search.value = ''
    await fireEvent.input(search)
    await fireEvent.click(tipped('sound.favourites')[0])
    flushSync()
    expect(names()).toEqual(['Stage Grand', 'Warm Rhodes'])
    // A plugin patch says why it plays the fallback.
    expect(filterPatches(s.state.soundLibrary.patches, 'keys', 'all', false)[0].note).toContain('#91')
  })

  it('edits a patch, plays it on a part, reorders, duplicates and deletes', async () => {
    const s = setup()
    await fireEvent.click(tipped('sound.patch')[0])
    flushSync()
    await change(tipped('sound.name')[0] as HTMLInputElement, 'Concert Grand')
    expect(s.state.soundLibrary.patches[0].name).toBe('Concert Grand')
    await change(tipped('sound.default_volume')[0] as HTMLInputElement, '90')
    expect(s.state.soundLibrary.patches[0].defaults.volume).toBe(90)
    await change(tipped('sound.default_volume')[0] as HTMLInputElement, '')
    expect(s.state.soundLibrary.patches[0].defaults.volume).toBe(null)
    await fireEvent.click(tipped('sound.use_on_part')[1])
    expect(s.state.keyboardParts[1].patch).toBe('stage-grand')
    expect(s.state.keyboardParts[1].voiceName).toBe('Concert Grand')
    await fireEvent.click(tipped('sound.move_down')[0])
    expect(s.state.soundLibrary.patches[1].id).toBe('stage-grand')
    await fireEvent.click(tipped('sound.duplicate')[0])
    flushSync()
    expect(s.state.soundLibrary.patches[2].name).toBe('Concert Grand copy')
    expect(nav.selected).toBe(s.state.soundLibrary.patches[2].id)
    await fireEvent.click(tipped('sound.delete')[0])
    expect(s.state.soundLibrary.patches.some((p) => p.name === 'Concert Grand copy')).toBe(false)
  })

  it('auditions while stopped, and not while the band plays', async () => {
    const s = setup()
    s.send({ type: 'stop' })
    flushSync()
    await fireEvent.click(tipped('sound.audition')[0])
    expect(s.state.soundLibrary.auditioning).toBe('stage-grand')
    flushSync()
    expect(document.querySelector('.aud')!.textContent).toContain('Stage Grand')
    s.advance(3500)
    expect(s.state.soundLibrary.auditioning).toBe(null)
    s.send({ type: 'startStop' })
    await fireEvent.click(tipped('sound.audition')[0])
    expect(s.state.soundLibrary.auditioning).toBe(null)
    expect(s.state.message?.error).toBe(true)
  })

  it('the program map sets family rules, overrides and the drum rule, globally or for this style', async () => {
    const s = setup()
    await fireEvent.click(tab('map'))
    flushSync()
    const families = tipped('sound.family') as HTMLSelectElement[]
    expect(families).toHaveLength(16)
    await change(families[10], 'warm-rhodes')
    expect(s.state.soundLibrary.map.families[10]).toBe('warm-rhodes')
    await change(tipped('sound.drums')[0] as HTMLSelectElement, '')
    expect(s.state.soundLibrary.map.drums).toBe(null)
    // An override.
    await change(tipped('sound.override_program')[0] as HTMLSelectElement, '0')
    const pickers = tipped('sound.override_patch') as HTMLSelectElement[]
    await change(pickers[pickers.length - 1], 'warm-rhodes')
    await fireEvent.click(tipped('sound.override_add')[0])
    expect(s.state.soundLibrary.map.overrides.find((o) => o.program === 0)?.patch).toBe('warm-rhodes')
    await fireEvent.click(tipped('sound.override_remove')[0])
    expect(s.state.soundLibrary.map.overrides.some((o) => o.program === 0)).toBe(false)
    // This style's own rule; the global map is untouched.
    await fireEvent.click(tipped('sound.scope')[1])
    flushSync()
    await change((tipped('sound.family') as HTMLSelectElement[])[4], 'soft-pad')
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
    const remaps = tipped('sound.remap') as HTMLSelectElement[]
    await change(remaps[2], 'soft-pad')
    expect(s.state.soundLibrary.map.overrides.find((o) => o.program === bass.gmProgram)?.patch).toBe('soft-pad')
    expect(s.state.soundLibrary.usage.find((u) => u.part === 'Bass')!.plays).toBe('Soft Pad')
    // A drum part's remap is the drum rule.
    await change(remaps[0], 'finger-bass')
    expect(s.state.soundLibrary.map.drums).toBe('finger-bass')
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
