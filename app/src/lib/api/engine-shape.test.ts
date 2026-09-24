// The mock speaks the engine's JSON: every key path the real engine emits (recorded from
// #16's `yahaha state-json` in engine-shape.json; see scripts/engine-shape.ts) exists in
// the mock's state and library with the same kind of value. Null in the recording (an
// offline session has no synth) matches anything.

import { describe, expect, it } from 'vitest'
import { shape } from './shape'
import recorded from './engine-shape.json'
import { LIBRARY, MockSession } from './mock'

function check(real: string[], mock: string[]) {
  const have = new Map(mock.map((l) => l.split(': ') as [string, string]))
  const bad: string[] = []
  for (const line of real) {
    const [path, kind] = line.split(': ')
    const got = have.get(path)
    if (got === undefined) bad.push(`missing ${path}`)
    else if (kind !== 'null' && got !== 'null' && got !== kind) bad.push(`${path}: engine ${kind}, mock ${got}`)
  }
  return bad
}

describe('mock matches the engine JSON', () => {
  it('AppState', () => {
    const m = new MockSession({ manual: true, demo: true })
    m.send({ type: 'intro', index: 0 }) // make optional fields non-null somewhere
    expect(check(recorded.state, shape(m.state))).toEqual([])
  })
  it('LibraryList', () => {
    expect(check(recorded.library, shape(LIBRARY))).toEqual([])
  })
})
