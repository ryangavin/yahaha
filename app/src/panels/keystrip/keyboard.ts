// Keyboard-strip geometry: which keys a 49/61/88-key strip shows and where each key sits,
// as fractions of the strip's width, so the strip is plain percentage-positioned boxes
// that stay crisp at any size. Display maths only; what the keys *do* is the engine's.

import type { HeldNote } from '../../lib/api/types'
import type { KeyRange } from '../../lib/store.svelte'

/** Lowest and highest MIDI note: Launchkey 49 = C1–C5, 61 = C1–C6 (Yamaha numbering,
 * C3 = 60, at the default octave), 88 = A-1–C7. */
export const RANGES: Record<KeyRange, [number, number]> = { 49: [36, 84], 61: [36, 96], 88: [21, 108] }

const BLACK = new Set([1, 3, 6, 8, 10])
export const isBlack = (n: number) => BLACK.has(((n % 12) + 12) % 12)

/** A black key's width, in white-key widths. */
const BLACK_W = 0.58

export interface Key {
  note: number
  black: boolean
  /** Left edge and width, 0–1 of the strip. */
  x: number
  w: number
}

/** Every key from `lo` to `hi` (whites first, so blacks draw on top). */
export function layout([lo, hi]: [number, number]): Key[] {
  const whites = Array.from({ length: hi - lo + 1 }, (_, i) => lo + i).filter((n) => !isBlack(n))
  const unit = 1 / whites.length
  const at = new Map(whites.map((n, i) => [n, i]))
  const keys: Key[] = whites.map((n, i) => ({ note: n, black: false, x: i * unit, w: unit }))
  for (let n = lo; n <= hi; n++) {
    if (!isBlack(n)) continue
    // Between the white below it and the one above.
    const below = at.get(n - 1)
    const x = below === undefined ? 0 : (below + 1 - BLACK_W / 2) * unit
    keys.push({ note: n, black: true, x, w: BLACK_W * unit })
  }
  return keys
}

/** The x (0–1) of the boundary just above `note`: where a split after that key is drawn. */
export function boundary(keys: Key[], note: number): number {
  const byNote = new Map(keys.map((k) => [k.note, k]))
  const [lo, hi] = [keys[0].note, keys.reduce((m, k) => Math.max(m, k.note), 0)]
  if (note < lo) return 0
  if (note >= hi) return 1
  const a = byNote.get(note)!
  const b = byNote.get(note + 1)!
  // A white next to a black: the black key's middle; two whites: their shared edge.
  if (a.black) return a.x + a.w / 2
  if (b.black) return b.x + b.w / 2
  return b.x
}

/** The key under `x` (0–1): black keys first, since they sit on top. */
export function noteAt(keys: Key[], x: number, blackToo = true): number {
  const hit = (k: Key) => x >= k.x && x < k.x + k.w
  return ((blackToo && keys.find((k) => k.black && hit(k))) || keys.find((k) => !k.black && hit(k)) || keys[x < 0.5 ? 0 : keys.length - 1]).note
}

/** The strip's size: the user's choice, else the connected Launchkey's (by its port name), else 61. */
export function rangeFor(choice: KeyRange | null, inputs: string[]): KeyRange {
  if (choice) return choice
  const m = inputs.map((s) => /Launchkey\s*(?:Mini\s*)?(25|37|49|61|88)/i.exec(s)).find(Boolean)
  const n = m ? Number(m[1]) : 61
  return n <= 49 ? 49 : n <= 61 ? 61 : 88
}

/** The part of the strip chord detection listens to: the engine's `keyboard.detection`
 * clipped to the keys shown, as [lo, hi] MIDI notes (inclusive), or null for none. */
export function detectionArea([dlo, dhi]: [number, number], [lo, hi]: [number, number]): [number, number] | null {
  const a = Math.max(dlo, lo)
  const b = Math.min(dhi, hi)
  return a <= b ? [a, b] : null
}

/** The CSS colour tokens for the parts a held key sounds on (see app.css, "Keyboard parts"). */
const PART_VARS = ['var(--part-r1)', 'var(--part-r2)', 'var(--part-r3)', 'var(--part-left)']

/** A held key's fill: one colour per part, in bands; the chord colour if it only feeds detection. */
export function heldFill(h: HeldNote): string {
  const cols = h.parts.map((p) => PART_VARS[p] ?? 'var(--part-chord)')
  if (!cols.length) return 'var(--part-chord)'
  if (cols.length === 1) return cols[0]
  const step = 100 / cols.length
  return `linear-gradient(180deg, ${cols.map((c, i) => `${c} ${i * step}% ${(i + 1) * step}%`).join(', ')})`
}

const NAMES = ['C', 'C#', 'D', 'Eb', 'E', 'F', 'F#', 'G', 'Ab', 'A', 'Bb', 'B']

/** Yamaha numbering (C3 = 60), as the engine names notes. */
export const noteName = (n: number) => `${NAMES[((n % 12) + 12) % 12]}${Math.floor(n / 12) - 2}`
export const pcName = (pc: number) => NAMES[((pc % 12) + 12) % 12]
