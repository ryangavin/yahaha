// Library's filters and sub-views: app-only state that survives going back to Stage. The
// page, its tab and the part it loads into are in the ui store (`ui.view`, `ui.libraryTab`,
// `ui.libraryPart`), since the header, the quick-nav and the part strips open it.

import type { PatchCategory } from '../../lib/api/types'
import type { SourceChip, SoundFilter } from './model'

class LibraryNav implements SoundFilter {
  /** Sounds: the source chip, ★, the instrument filter (Instruments › Browse), the category, the search. */
  source = $state<SourceChip>('all')
  favourites = $state(false)
  instrument = $state<string | null>(null)
  category = $state<PatchCategory | null>(null)
  query = $state('')
  /** Racks: only the racks that need attention (a part's plugin is missing). */
  attention = $state(false)
  /** Racks: the search over the racks' names and sounds. */
  rackQuery = $state('')
  /** Racks: the rack whose details show (null: the loaded one's). */
  rack = $state<string | null>(null)
  /** Style map: the GM map, or Add from SoundFont. */
  mapPage = $state<'gm' | 'add'>('gm')

  reset() {
    this.source = 'all'
    this.favourites = false
    this.instrument = null
    this.category = null
    this.query = ''
    this.attention = false
    this.rackQuery = ''
    this.rack = null
    this.mapPage = 'gm'
  }
}

export const libraryNav = new LibraryNav()
