import type { LibraryEntry, LibraryList } from '../../lib/api/types'

/**
 * The styles < Track and Track > would load: the previous and next entries in library
 * order from `position`, skipping files that don't load (as `stepStyle` does), wrapping.
 */
export function neighbours(lib: LibraryList, position: number): { prev: LibraryEntry | null; next: LibraryEntry | null } {
  const n = lib.entries.length
  const step = (d: number): LibraryEntry | null => {
    let i = position
    for (let k = 0; k < n; k++) {
      i = (((i + d) % n) + n) % n
      if (i === position) return null
      if (lib.entries[i].status !== 'error') return lib.entries[i]
    }
    return null
  }
  return n ? { prev: step(-1), next: step(1) } : { prev: null, next: null }
}
