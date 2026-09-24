import { describe, expect, it } from 'vitest'
import { LIBRARY, MockSession } from '../../lib/api/mock'
import { syntheticStyles } from '../../lib/api/mock-library'
import type { LibraryEntry } from '../../lib/api/types'
import { neighbours } from '../../lib/surface'
import { folderTree, indexLibrary, keepCursor, moveCursor, sectionLamps, setOf, stem, visibleRows } from './model'

const entry = (id: number, folder: string, name: string, file = name.replace(/ /g, '') + '.sty'): LibraryEntry => ({
  id, name, folder, path: `/s/${folder ? folder + '/' : ''}${file}`, status: 'ok', error: null,
  tempo: 120, timeSignature: [4, 4], sections: 'Main AB', format: 'SFF1',
})

// Library order (folder, then name), as the engine lists it.
const LIB = [
  entry(0, '', 'Loose Tune', 'loose.sty'),
  entry(1, 'Pop', 'Alpha Pop'),
  entry(2, 'Pop', 'Beta Pop', 'zz_file_only.prs'),
  entry(3, 'Pop/Sub', 'Deep Cut'),
  entry(4, 'Pop & Rock', 'Rocker'),
  entry(5, 'Swing', 'Late Night'),
]
const none = setOf([])

describe('filter', () => {
  const ix = indexLibrary(LIB)
  const names = (rows: number[]) => rows.map((i) => LIB[i].name)

  it('matches the name, the file name or the folder, case-insensitively, in library order', () => {
    expect(names(visibleRows(ix, { kind: 'all' }, 'POP', none, []))).toEqual(['Alpha Pop', 'Beta Pop', 'Deep Cut', 'Rocker'])
    expect(names(visibleRows(ix, { kind: 'all' }, 'zz_file', none, []))).toEqual(['Beta Pop'])
    expect(names(visibleRows(ix, { kind: 'all' }, 'night', none, []))).toEqual(['Late Night'])
    expect(visibleRows(ix, { kind: 'all' }, '', none, [])).toHaveLength(LIB.length)
  })

  it('a folder shows itself and its subfolders, never a folder that merely starts the same', () => {
    expect(names(visibleRows(ix, { kind: 'folder', path: 'Pop' }, '', none, []))).toEqual(['Alpha Pop', 'Beta Pop', 'Deep Cut'])
    expect(names(visibleRows(ix, { kind: 'folder', path: '' }, '', none, []))).toEqual(['Loose Tune'])
  })

  it('favourites keep library order; recent is newest first; both by path', () => {
    const favs = setOf([LIB[5].path, LIB[1].path])
    expect(names(visibleRows(ix, { kind: 'favourites' }, '', favs, []))).toEqual(['Alpha Pop', 'Late Night'])
    const recents = [LIB[4].path, '/gone.sty', LIB[0].path]
    expect(names(visibleRows(ix, { kind: 'recents' }, '', none, recents))).toEqual(['Rocker', 'Loose Tune'])
  })

  it('the file stem is the file name without folder and extension', () => {
    expect(stem('/a/b/My.Style.sty')).toBe('My.Style')
    expect(stem('/a/.hidden')).toBe('.hidden')
  })
})

describe('folders', () => {
  it('lists every folder with its parents, as a tree, counting subfolders', () => {
    const tree = folderTree(LIB)
    expect(tree.map((f) => `${'  '.repeat(f.depth)}${f.name || '(root)'} ${f.count}`)).toEqual([
      '(root) 1',
      'Pop 3',
      '  Sub 1',
      'Pop & Rock 1',
      'Swing 1',
    ])
  })
})

describe('section lamps', () => {
  it('reads the engine summary', () => {
    const l = sectionLamps('Main ABD · Intro AB · Ending ABC · Fill ABCD · Break')
    expect(l.main).toEqual([true, true, false, true])
    expect(l.intro).toEqual([true, true, false])
    expect(l.ending).toEqual([true, true, true])
    expect(l.brk).toBe(true)
    expect(sectionLamps('').main).toEqual([false, false, false, false])
  })
})

describe('cursor', () => {
  it('moves and clamps', () => {
    expect(moveCursor('ArrowDown', 4, 5, 10)).toBe(4)
    expect(moveCursor('ArrowUp', 0, 5, 10)).toBe(0)
    expect(moveCursor('PageDown', 0, 50, 10)).toBe(10)
    expect(moveCursor('End', 0, 50, 10)).toBe(49)
    expect(moveCursor('a', 0, 50, 10)).toBeNull()
    expect(moveCursor('ArrowDown', 0, 0, 10)).toBeNull()
  })
  it('stays on the same style when the list changes, else goes to the top', () => {
    expect(keepCursor([3, 4, 5], LIB, 4)).toBe(1)
    expect(keepCursor([3, 5], LIB, 4)).toBe(0)
  })
})

describe('Track order', () => {
  it('the list order is the order < Track / Track > step through', () => {
    const s = new MockSession({ manual: true })
    // Walk the whole library with Track >: it visits the readable rows in list order.
    const seen: number[] = []
    for (let k = 0; k < LIBRARY.entries.length; k++) {
      s.send({ type: 'stepStyle', delta: 1 })
      seen.push(s.state.style.id)
    }
    const readable = LIBRARY.entries.filter((e) => e.status === 'ok').map((e) => e.id)
    const start = readable.indexOf(seen[0])
    expect(seen.slice(0, readable.length - start)).toEqual(readable.slice(start))
    const near = neighbours(LIBRARY, s.state.library.position)
    s.send({ type: 'stepStyle', delta: 1 })
    expect(s.state.style.id).toBe(near.next!.id)
  })
})

describe('a big library (60k styles)', () => {
  const big = syntheticStyles(60000, 0).map((s): LibraryEntry => ({
    id: s.id, name: s.name, folder: s.folder, path: `/s/${s.folder}/${s.file}`, status: s.error ? 'error' : 'ok',
    error: s.error ?? null, tempo: s.tempo, timeSignature: [4, 4], sections: 'Main AB', format: s.format,
  }))

  it('indexes, builds folders and filters quickly', () => {
    const t0 = performance.now()
    const ix = indexLibrary(big)
    const tree = folderTree(big)
    const t1 = performance.now()
    let n = 0
    for (const q of ['b', 'bl', 'blu', 'blue', 'blue s', 'blue sw', 'x']) n += visibleRows(ix, { kind: 'all' }, q, none, []).length
    const t2 = performance.now()
    expect(tree.length).toBeGreaterThan(100)
    expect(n).toBeGreaterThan(0)
    // Generous bounds (CI machines vary); in a browser each keystroke takes a few ms.
    expect(t1 - t0).toBeLessThan(1500)
    expect((t2 - t1) / 7).toBeLessThan(150)
  })

  it('is deterministic and has error rows and files without a name marker', () => {
    const again = syntheticStyles(60000, 0)
    expect(again[123].name).toBe(big[123].name)
    expect(big.filter((e) => e.status === 'error').length).toBeGreaterThan(300)
  })
})
