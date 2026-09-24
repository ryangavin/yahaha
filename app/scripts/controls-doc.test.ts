import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { expect, it } from 'vitest'
import { render } from './controls-doc.ts'

it('docs/controls.md is up to date with the tooltip catalog (npm run docs:controls)', () => {
  const file = readFileSync(resolve(process.cwd(), 'docs', 'controls.md'), 'utf8')
  expect(file).toBe(render())
})
