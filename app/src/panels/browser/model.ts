// The style browser's list logic, kept out of the component so it's testable and fast
// with a big library (60k+ styles).
//
// Rules match the terminal browser and src/library.rs:
// - Order is the library's own (folder, then name): exactly the order < Track / Track >
//   step through. The browser never re-sorts it (Recent is the one exception: newest
//   first).
// - The category is the folder, relative to the library root, `/`-separated.
// - The filter is a case-insensitive substring match on the style name, the file name or
//   the folder.

import type { LibraryEntry } from '../../lib/api/types'

export type Category =
  | { kind: 'all' }
  | { kind: 'favourites' }
  | { kind: 'recents' }
  /** A folder and its subfolders; '' is the library root's own files. */
  | { kind: 'folder'; path: string }

/** Lowercased search keys, computed once per library revision. */
export interface Indexed {
  entries: LibraryEntry[]
  name: string[]
  stem: string[]
  folder: string[]
  byPath: Map<string, number>
}

/** The file name without its folder and extension, as the engine's `stem`. */
export function stem(path: string): string {
  const base = path.slice(path.lastIndexOf('/') + 1)
  const dot = base.lastIndexOf('.')
  return dot > 0 ? base.slice(0, dot) : base
}

export function indexLibrary(entries: LibraryEntry[]): Indexed {
  const n = entries.length
  const ix: Indexed = { entries, name: new Array(n), stem: new Array(n), folder: new Array(n), byPath: new Map() }
  for (let i = 0; i < n; i++) {
    const e = entries[i]
    ix.name[i] = e.name.toLowerCase()
    ix.stem[i] = stem(e.path).toLowerCase()
    ix.folder[i] = e.folder.toLowerCase()
    ix.byPath.set(e.path, i)
  }
  return ix
}

export interface FolderNode {
  path: string
  /** The last path segment ('' shows as the library root). */
  name: string
  depth: number
  /** Styles in it and its subfolders. */
  count: number
}

/** Every folder, with its parents, in library order (parents before children). */
export function folderTree(entries: LibraryEntry[]): FolderNode[] {
  const counts = new Map<string, number>()
  for (const e of entries) {
    if (e.folder === '') {
      counts.set('', (counts.get('') ?? 0) + 1)
      continue
    }
    const parts = e.folder.split('/')
    for (let d = 1; d <= parts.length; d++) {
      const p = parts.slice(0, d).join('/')
      counts.set(p, (counts.get(p) ?? 0) + 1)
    }
  }
  const lc = (p: string) => p.toLowerCase()
  return [...counts.keys()]
    // Compare segment by segment so "Pop/Sub" sits under "Pop", before "Pop & Rock".
    .sort((a, b) => {
      const x = lc(a).split('/')
      const y = lc(b).split('/')
      for (let i = 0; i < Math.min(x.length, y.length); i++) if (x[i] !== y[i]) return x[i] < y[i] ? -1 : 1
      return x.length - y.length
    })
    .map((path) => {
      const parts = path === '' ? [''] : path.split('/')
      return { path, name: parts[parts.length - 1], depth: parts.length - 1, count: counts.get(path)! }
    })
}

function inFolder(folderLc: string, pathLc: string): boolean {
  if (pathLc === '') return folderLc === ''
  return folderLc === pathLc || (folderLc.length > pathLc.length && folderLc.startsWith(pathLc) && folderLc[pathLc.length] === '/')
}

/**
 * The rows to show: indices into `ix.entries`, in library order (Recent: newest first).
 * `favourites` and `recents` are file paths (ids can change when the library is rescanned).
 */
export function visibleRows(
  ix: Indexed,
  category: Category,
  query: string,
  favourites: ReadonlySet<string>,
  recents: readonly string[],
): number[] {
  const q = query.trim().toLowerCase()
  const match = (i: number) => q === '' || ix.name[i].includes(q) || ix.stem[i].includes(q) || ix.folder[i].includes(q)
  const out: number[] = []
  if (category.kind === 'recents') {
    for (const p of recents) {
      const i = ix.byPath.get(p)
      if (i !== undefined && match(i)) out.push(i)
    }
    return out
  }
  const n = ix.entries.length
  if (category.kind === 'favourites') {
    for (let i = 0; i < n; i++) if (favourites.has(ix.entries[i].path) && match(i)) out.push(i)
    return out
  }
  const folder = category.kind === 'folder' ? category.path.toLowerCase() : null
  for (let i = 0; i < n; i++) {
    if (folder !== null && !inFolder(ix.folder[i], folder)) continue
    if (match(i)) out.push(i)
  }
  return out
}

export interface SectionLamps {
  intro: boolean[]
  main: boolean[]
  fill: boolean
  brk: boolean
  ending: boolean[]
}

/**
 * The engine's section summary ("Main ABCD · Intro ABC · Ending ABC · Fill ABCD · Break")
 * as lamps: Intro I–III, Main A–D, Break, Ending I–III, as the Launchkey's pads have them.
 */
export function sectionLamps(summary: string): SectionLamps {
  const groups = new Map<string, string>()
  for (const part of summary.split('·')) {
    const [k, letters = ''] = part.trim().split(/\s+/)
    if (k) groups.set(k, letters)
  }
  const has = (k: string, n: number) => Array.from({ length: n }, (_, i) => (groups.get(k) ?? '').includes('ABCD'[i]))
  return { intro: has('Intro', 3), main: has('Main', 4), fill: groups.has('Fill'), brk: groups.has('Break'), ending: has('Ending', 3) }
}

/** The row to put the cursor on after the list changed: the same style if it's still
 * shown, otherwise the first row. */
export function keepCursor(rows: number[], entries: LibraryEntry[], id: number | null): number {
  if (id === null) return 0
  const i = rows.findIndex((r) => entries[r].id === id)
  return i < 0 ? 0 : i
}

/** Cursor movement for ↑/↓, PgUp/PgDn, Home/End; null for other keys. */
export function moveCursor(key: string, cursor: number, count: number, page: number): number | null {
  if (count === 0) return null
  const clamp = (v: number) => Math.max(0, Math.min(count - 1, v))
  switch (key) {
    case 'ArrowDown': return clamp(cursor + 1)
    case 'ArrowUp': return clamp(cursor - 1)
    case 'PageDown': return clamp(cursor + page)
    case 'PageUp': return clamp(cursor - page)
    case 'Home': return 0
    case 'End': return count - 1
    default: return null
  }
}

/** An immutable set (the prefs store replaces it whole on every change). */
export function setOf(items: Iterable<string>): ReadonlySet<string> {
  return new Set(items)
}
