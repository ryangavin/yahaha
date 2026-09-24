// Note names and keyboard geometry for the split strip. Formatting only: the engine owns
// the split (`state.chord.split` / `splitName`); this names the key under the pointer
// while you drag, the same way `note_name` in src/api.rs does (Yamaha numbering, C3 = 60).

const NAMES = ['C', 'C#', 'D', 'Eb', 'E', 'F', 'F#', 'G', 'Ab', 'A', 'Bb', 'B']
const BLACK = [false, true, false, true, false, false, true, false, true, false, true, false]

/** The split's range, as the engine clamps it. */
export const SPLIT_MIN = 24
export const SPLIT_MAX = 96

export function noteName(n: number): string {
  return `${NAMES[((n % 12) + 12) % 12]}${Math.floor(n / 12) - 2}`
}

export function isBlack(n: number): boolean {
  return BLACK[((n % 12) + 12) % 12]
}

export interface StripKey {
  note: number
  black: boolean
  /** White keys: index among white keys. Black keys: the white key index it sits after. */
  slot: number
}

/** The keys from `lo` to `hi` (both white), laid out like a keyboard. */
export function keysBetween(lo: number, hi: number): StripKey[] {
  const keys: StripKey[] = []
  let white = -1
  for (let n = lo; n <= hi; n++) {
    const black = isBlack(n)
    if (!black) white++
    keys.push({ note: n, black, slot: white })
  }
  return keys
}

/** Width of a black key as a fraction of a white key. */
export const BLACK_W = 0.6
/** Height of a black key as a fraction of the strip. */
export const BLACK_H = 0.62

/**
 * The key at a point on the strip: `x` in white-key units from the left edge, `y` as a
 * fraction of the height from the top.
 */
export function keyAt(keys: StripKey[], x: number, y: number): number {
  const whites = keys.filter((k) => !k.black)
  if (y < BLACK_H) {
    for (const k of keys) {
      if (k.black && Math.abs(x - (k.slot + 1)) <= BLACK_W / 2) return k.note
    }
  }
  return whites[Math.max(0, Math.min(whites.length - 1, Math.floor(x)))].note
}

/** A signed semitone count for display: "+2", "0", "−3". */
export function signed(n: number): string {
  return n > 0 ? `+${n}` : n < 0 ? `−${-n}` : '0'
}
