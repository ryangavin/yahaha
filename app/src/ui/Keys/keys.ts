/** The keys' layout and accessible label, from MIDI note numbers (Yamaha octaves: C3 = 60). */

/** A part hue a held key fills with. */
export type KeyPart = 'r1' | 'r2' | 'r3' | 'l'

/** A lowest and highest MIDI note, both included. */
export type KeyRange = { low: number; high: number }

export type WhiteKey = {
  note: number
  /** Left edge, px from the strip's inner left. */
  x: number
  /** "C3" on every C, else "". */
  label: string
  left: boolean
  held: KeyPart | null
}

export type BlackKey = {
  note: number
  /** The centre: the boundary between the two white keys around it, px from the inner left. */
  x: number
  left: boolean
  held: KeyPart | null
}

export type KeysLayout = {
  whites: WhiteKey[]
  blacks: BlackKey[]
  /** The width of one white key, px. */
  whiteWidth: number
  /** The split line's x (the right edge of the highest white key at or below the split note); null without a split. */
  splitX: number | null
}

const NAMES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B']
const BLACK = new Set([1, 3, 6, 8, 10])

export const isBlack = (note: number) => BLACK.has(((note % 12) + 12) % 12)

/** The pitch name without its octave ("F#"). */
export const pitchName = (note: number) => NAMES[((note % 12) + 12) % 12]

/** The note's name with its Yamaha octave, C3 = 60 ("F#2"). */
export const noteName = (note: number) => `${pitchName(note)}${Math.floor(note / 12) - 2}`

/** Where each key sits in a strip `inner` px wide. Held right notes draw over held left ones. */
export function layout(
  range: KeyRange,
  inner: number,
  split: number | null,
  heldLeft: number[],
  heldRight: number[],
  rightPart: KeyPart,
): KeysLayout {
  const held = new Map<number, KeyPart>()
  for (const note of heldLeft) held.set(note, 'l')
  for (const note of heldRight) held.set(note, rightPart)
  const isLeft = (note: number) => split !== null && note <= split

  const whiteNotes: number[] = []
  for (let note = range.low; note <= range.high; note++) if (!isBlack(note)) whiteNotes.push(note)
  const whiteWidth = whiteNotes.length > 0 ? inner / whiteNotes.length : 0
  const index = new Map(whiteNotes.map((note, i) => [note, i]))

  const whites = whiteNotes.map((note, i) => ({
    note,
    x: i * whiteWidth,
    label: pitchName(note) === 'C' ? noteName(note) : '',
    left: isLeft(note),
    held: held.get(note) ?? null,
  }))

  const blacks: BlackKey[] = []
  for (let note = range.low; note <= range.high; note++) {
    if (!isBlack(note)) continue
    const below = index.get(note - 1)
    blacks.push({
      note,
      x: below === undefined ? 0 : (below + 1) * whiteWidth,
      left: isLeft(note),
      held: held.get(note) ?? null,
    })
  }

  let splitX: number | null = null
  if (split !== null && split >= range.low && split < range.high) {
    const highest = whiteNotes.filter((note) => note <= split).length
    splitX = Math.floor(highest * whiteWidth)
  }

  return { whites, blacks, whiteWidth, splitX }
}

/** "Keys: split F#2, left hand G A C E, right hand E4 A4, 61 keys" (the left hand as a chord, without octaves). */
export function keysLabel(range: KeyRange, split: number | null, heldLeft: number[], heldRight: number[]): string {
  const count = Math.max(0, range.high - range.low + 1)
  const sorted = (notes: number[]) => [...new Set(notes)].sort((a, b) => a - b)
  const list = (names: string[]) => (names.length > 0 ? names.join(' ') : 'none')
  if (split === null) {
    const all = sorted([...heldLeft, ...heldRight]).map(noteName)
    return `Keys: no split, held ${list(all)}, ${count} keys`
  }
  const chord = [...new Set(sorted(heldLeft).map(pitchName))]
  const right = sorted(heldRight).map(noteName)
  return `Keys: split ${noteName(split)}, left hand ${list(chord)}, right hand ${list(right)}, ${count} keys`
}
