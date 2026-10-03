/** One of the sixteen band pads. */
export type PadItem = {
  /** The caption ("Main B"). Tie a numeral to its word with a no-break space (U+00A0), so only whole words wrap. */
  label: string
  /** A section family, `util`, or `start` (Start / Stop). Consecutive pads of one section family share a group line. */
  family: 'intro' | 'main' | 'ending' | 'brk' | 'fill' | 'util' | 'start'
  /** The face. */
  state: 'idle' | 'dark' | 'playing' | 'next' | 'armed' | 'running'
  /** The accessible name. Default: "{label} (pad {n})". */
  name?: string
  /** The tooltip key. */
  tip?: string
}

/** One entry of the header's hue legend. */
export type LegendItem = { label: string; hue: string }
