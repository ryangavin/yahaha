import { describe, expect, it } from 'vitest'
import type { SoundCatalog } from '../../lib/api/types'
import { NO_FILTER, instrumentNames, libraryCategories, librarySounds } from './model'

describe('Library model memo', () => {
  const catalog: SoundCatalog = {
    revision: 1,
    recents: [],
    entries: [
      { id: 'sf:A.sf2:0:0', name: 'Grand', category: 'piano', source: 'soundFont', detail: 'A.sf2', favourite: false, recent: false, plugin: null },
      { id: 'au:synth', name: 'Synth', category: 'synthLead', source: 'plugin', detail: 'Maker', favourite: false, recent: false, plugin: { format: 'AUv2', lastError: null } },
      { id: 'au:synth#f:1', name: 'Arp', category: 'synthLead', source: 'plugin', detail: 'Maker', favourite: true, recent: false, plugin: { format: 'AUv2', lastError: null }, parent: 'au:synth' },
    ],
  }
  const patches: Parameters<typeof librarySounds>[2]['patches'] = []
  const gmMap = [{ program: 0, resolved: { sound: 'sf:A.sf2:0:0' } }] as unknown as Parameters<typeof librarySounds>[2]['gmMap']

  it('the same catalog, slices and filter fields give the same rows array, whatever the wrapping objects', () => {
    const a = librarySounds(catalog, { ...NO_FILTER }, { patches, gmMap })
    const b = librarySounds(catalog, { ...NO_FILTER }, { patches, gmMap })
    expect(b).toBe(a)
    expect(a.map((i) => catalog.entries[i].name)).toEqual(['Grand', 'Arp', 'Synth'])
    // The category column counts the same rows: asking for it doesn't recompute them.
    expect(libraryCategories(catalog, { ...NO_FILTER, category: 'piano' }, { patches, gmMap }).find((c) => c.id === 'synthLead')?.count).toBe(2)
    expect(librarySounds(catalog, { ...NO_FILTER }, { patches, gmMap }, true)).toBe(a)
    const piano = librarySounds(catalog, { ...NO_FILTER, category: 'piano' }, { patches, gmMap })
    expect(piano.map((i) => catalog.entries[i].name)).toEqual(['Grand'])
    expect(librarySounds(catalog, { ...NO_FILTER, category: 'piano' }, { patches, gmMap })).toBe(piano)
    expect(instrumentNames(catalog)).toBe(instrumentNames(catalog))
  })

  it('a new slice or a changed filter field gives new rows', () => {
    const a = librarySounds(catalog, NO_FILTER, { patches, gmMap })
    expect(librarySounds(catalog, NO_FILTER, { patches: [], gmMap })).not.toBe(a)
    const fav = librarySounds(catalog, { ...NO_FILTER, favourites: true }, { patches, gmMap })
    expect(fav.map((i) => catalog.entries[i].name)).toEqual(['Arp'])
    const q = librarySounds(catalog, { ...NO_FILTER, query: 'maker synth' }, { patches, gmMap })
    expect(q.map((i) => catalog.entries[i].name)).toEqual(['Arp', 'Synth'])
  })
})
