/**
 * Text contrast of the token roles, in dark and light: every text role on each surface it's drawn
 * on must reach WCAG AA, 4.5:1 (storybook-axioms.md, axiom 8). jsdom can't compute contrast on a
 * rendered story, so this checks the tokens themselves; `npm run shots` checks rendered stories
 * with axe in Chrome. A new text-on-surface pairing in a component adds a row here.
 */
import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

// Read from disk: vitest stubs CSS imports, `?raw` included.
const read = (file: string) => readFileSync(new URL(file, import.meta.url), 'utf8')
const dark = read('./dark.css')
const light = read('./light.css')
const palette = read('./palette.css')

/** [text role, surface role, where; optional opacity role for the text]. Disabled text is exempt (WCAG 1.4.3). */
const PAIRS: [string, string, string, string?][] = [
  ['--t', '--g', 'text on the ground'],
  ['--t2', '--g', 'secondary text on the ground'],
  ['--m', '--g', 'muted text on the ground'],
  ['--m', '--btn', 'off lamp label and code (LampButton)'],
  ['--t2', '--btn', 'button label'],
  ['--lamp-ink', '--lamp', 'lit lamp label (LampButton)'],
  ['--lamp-ink', '--lamp', 'lit lamp code (LampButton)', '--code-opacity'],
  ['--solid-ink', '--rec', 'record lamp label and code (LampButton)'],
]

function declarations(css: string): Map<string, string> {
  const out = new Map<string, string>()
  for (const m of css.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)) out.set(m[1], m[2].trim())
  return out
}

/** The theme's value for a role, with var() resolved through the theme and the palette. */
function resolver(theme: Map<string, string>): (name: string) => string {
  const pal = declarations(palette)
  const resolve = (name: string): string => {
    const value = theme.get(name) ?? pal.get(name)
    if (value === undefined) throw new Error(`no token ${name}`)
    const ref = /^var\((--[\w-]+)\)$/.exec(value)
    return ref ? resolve(ref[1]) : value
  }
  return resolve
}

function rgb(hex: string): [number, number, number] {
  const h = hex.replace('#', '')
  const full = h.length === 3 ? [...h].map((c) => c + c).join('') : h
  return [0, 2, 4].map((i) => parseInt(full.slice(i, i + 2), 16)) as [number, number, number]
}

function luminance([r, g, b]: number[]): number {
  const lin = (c: number) => {
    const s = c / 255
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4
  }
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

function contrast(fg: number[], bg: number[]): number {
  const [a, b] = [luminance(fg), luminance(bg)].sort((x, y) => y - x)
  return (a + 0.05) / (b + 0.05)
}

const THEMES = {
  dark: declarations(dark),
  // Light overrides dark; roles it doesn't set fall back to dark's.
  light: new Map([...declarations(dark), ...declarations(light)]),
}

describe('token contrast', () => {
  it('computes WCAG contrast', () => {
    expect(contrast(rgb('#000'), rgb('#fff'))).toBeCloseTo(21)
    expect(contrast(rgb('#6e6e6e'), rgb('#e4e3df'))).toBeLessThan(4.5)
  })

  for (const [theme, values] of Object.entries(THEMES)) {
    const resolve = resolver(values)
    for (const [text, surface, where, opacity] of PAIRS) {
      it(`${theme}: ${text} on ${surface}, ${where}`, () => {
        const bg = rgb(resolve(surface))
        const alpha = opacity ? Number(resolve(opacity)) : 1
        const fg = rgb(resolve(text)).map((c, i) => alpha * c + (1 - alpha) * bg[i])
        expect(contrast(fg, bg)).toBeGreaterThanOrEqual(4.5)
      })
    }
  }
})
