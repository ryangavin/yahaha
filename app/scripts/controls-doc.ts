// Renders the tooltip catalog to app/docs/controls.md: every control, what it does, its
// Genos name, its key and where it is on the Launchkey. `npm run docs:controls` writes it;
// a test fails when the file is out of date.

import { writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { TIPS, keyLabel, type Tip } from '../src/help/tooltips.ts'

const GROUPS: [string, string][] = [
  ['transport', 'Transport'],
  ['section', 'Sections'],
  ['tempo', 'Tempo'],
  ['display', 'Displays'],
  ['style', 'Style'],
  ['browser', 'Style browser'],
  ['fingering', 'Fingering'],
  ['detection', 'Chord detection'],
  ['split', 'Split point'],
  ['transpose', 'Transpose'],
  ['ots', 'One Touch Settings'],
  ['regist', 'Registration Memory'],
  ['playlist', 'Playlist'],
  ['part', 'Keyboard parts'],
  ['harmony', 'Keyboard Harmony / Arpeggio'],
  ['mixer', 'Mixer'],
  ['metronome', 'Metronome'],
  ['looper', 'Chord Looper'],
  ['multipad', 'Multi Pads'],
  ['sound', 'Sound library'],
  ['padpage', 'Launchkey pad pages'],
  ['launchkey', 'Launchkey'],
  ['lead', 'Lead-sheet band'],
  ['keystrip', 'Keyboard strip'],
  ['drawer', 'Panels around the hardware view'],
  ['settings', 'Settings'],
  ['audio', 'Audio'],
  ['midi', 'MIDI'],
  ['pedal', 'Pedals and wheels'],
  ['app', 'App'],
]

const cell = (s: string) => s.replace(/\|/g, '\\|').replace(/\n/g, ' ')
const code = (k: string) => '`' + keyLabel(k) + '`'

function keys(t: Tip): string {
  if (t.app_keys) return `${t.app_keys.map(code).join(' ')} (terminal: ${t.keys.map(code).join(' ')})`
  return t.keys.map(code).join(' ') || '—'
}

export function render(): string {
  const out = [
    '# Controls',
    '',
    '<!-- Generated from app/src/help/tooltips.ts by `npm run docs:controls`. Do not edit. -->',
    '',
    'Every control in the app, as its tooltip describes it. Hover over any control in the app (or press `?` for help mode) to see the same text.',
    '',
  ]
  const entries = Object.entries(TIPS) as [string, Tip][]
  for (const [prefix, heading] of GROUPS) {
    const rows = entries.filter(([k]) => k.split('.')[0] === prefix)
    if (!rows.length) continue
    out.push(`## ${heading}`, '', '| control | what it does | Genos | key | Launchkey |', '|---|---|---|---|---|')
    for (const [, t] of rows) {
      out.push(`| **${cell(t.title)}** | ${cell(t.body)} | ${cell(t.genos ?? '—')} | ${cell(keys(t))} | ${cell(t.launchkey ?? '—')} |`)
    }
    out.push('')
  }
  const grouped = new Set(GROUPS.map(([p]) => p))
  const orphans = entries.filter(([k]) => !grouped.has(k.split('.')[0])).map(([k]) => k)
  if (orphans.length) throw new Error(`catalog keys with no group in controls-doc.ts: ${orphans.join(', ')}`)
  return out.join('\n')
}

export const OUT = new URL('../docs/controls.md', import.meta.url)

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  writeFileSync(OUT, render())
  console.log(`wrote ${OUT.pathname}`)
}
