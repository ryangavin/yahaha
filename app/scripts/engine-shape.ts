// The engine's JSON shape, recorded from #16's `yahaha state-json` so the mock (and
// types.ts) can be checked against the real thing without committing real style data.
//
//   yahaha state-json <style> "C Am F G7" > state.json
//   yahaha state-json <folder> --library   > library.json
//   node scripts/engine-shape.ts state.json library.json   # writes src/lib/api/engine-shape.json
//
// Only key paths and value kinds are stored ("transport.lamps[].action.type: string").

import { readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { shape } from '../src/lib/api/shape.ts'

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  const [state, library] = process.argv.slice(2).map((f) => JSON.parse(readFileSync(f, 'utf8')))
  const out = { state: [...new Set(shape(state))].sort(), library: [...new Set(shape(library))].sort() }
  writeFileSync(new URL('../src/lib/api/engine-shape.json', import.meta.url), JSON.stringify(out, null, 1) + '\n')
  console.log(`${out.state.length} state paths, ${out.library.length} library paths`)
}
