// The catalog's shortcuts agree with README's "Terminal keys" list and with the keys the
// app actually binds, in both directions.

import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'
import { BINDINGS } from '../lib/keys'
import { TIPS } from './tooltips'

// Tests run from app/ (vitest's root); README is the repo's.
const README = readFileSync(resolve(process.cwd(), '..', 'README.md'), 'utf8')

/** README's Terminal keys section, as a set of keys in README notation. */
export function readmeKeys(md: string): Set<string> {
  const start = md.indexOf('### Terminal keys')
  const end = md.indexOf('\n### ', start + 1)
  expect(start, 'README has a "### Terminal keys" section').toBeGreaterThan(0)
  const section = md.slice(start, end)
  const keys = new Set<string>()
  for (const line of section.split('\n').filter((l) => l.startsWith('- '))) {
    // `shift+1`–`4` and `F1`–`F4`: ranges written across two code spans.
    for (const [, pre, a, b] of line.matchAll(/`([^`]*?)(\d+)`[–-]`(?:[A-Za-z+]*?)(\d+)`/g)) {
      for (let n = Number(a); n <= Number(b); n++) keys.add(`${pre}${n}`)
    }
    const spans = [...line.matchAll(/`([^`]+)`/g)].map((m) => m[1])
    for (const span of spans) {
      if (/^(shift\+)?\d+$/.test(span) || /^F\d+$/.test(span)) keys.add(span)
      if (span === 'z…,') {
        for (const k of 'zxcvbnm,') keys.add(k)
        continue
      }
      const range = /^(\d)-(\d)$/.exec(span)
      if (range) {
        for (let n = Number(range[1]); n <= Number(range[2]); n++) keys.add(String(n))
        continue
      }
      if (span === '←/→') {
        keys.add('←')
        keys.add('→')
        continue
      }
      for (const k of span.split(' ')) if (k) keys.add(k)
    }
  }
  return keys
}

/** README keys that aren't app controls (quitting the terminal UI). */
const TERMINAL_ONLY = new Set(['ctrl+c'])
/** App keys README doesn't list: the help toggle, and PgUp/PgDn standing in for Tab. */
const APP_ONLY = new Set(['?', 'PgUp', 'PgDn'])

const catalogKeys = new Set(Object.values(TIPS).flatMap((t) => t.keys))
const catalogAppKeys = new Set(Object.values(TIPS).flatMap((t) => t.app_keys ?? t.keys))

describe('shortcuts', () => {
  const readme = readmeKeys(README)

  it('parses README', () => {
    for (const k of ['space', 'shift+4', 'F3', ',', '←', 'tab', '\\']) expect(readme).toContain(k)
  })

  it('every catalog key is in README', () => {
    const missing = [...catalogKeys].filter((k) => !readme.has(k) && !APP_ONLY.has(k))
    expect(missing).toEqual([])
  })

  it('every README key is in the catalog', () => {
    const missing = [...readme].filter((k) => !catalogKeys.has(k) && !TERMINAL_ONLY.has(k))
    expect(missing).toEqual([])
  })

  it('every key the app binds is in the catalog', () => {
    const missing = Object.keys(BINDINGS).filter((k) => !catalogAppKeys.has(k))
    expect(missing).toEqual([])
  })

  it('every catalog key works in the app', () => {
    const missing = [...catalogAppKeys].filter((k) => !(k in BINDINGS))
    expect(missing).toEqual([])
  })
})
