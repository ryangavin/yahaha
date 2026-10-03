import type { ComponentProps } from 'svelte'
import type StyleLine from './StyleLine.svelte'

/** The dark board's style line: Sunday Drive Pop, Pop · 4/4, One Touch 2 applied, Band Reverb 40 Chorus 12 Delay 0. */
export const styleLineBoard = {
  styleName: 'Sunday Drive Pop',
  category: 'Pop',
  timeSignature: '4/4',
  queued: '',
  oneTouch: 2,
  reverb: 40,
  chorus: 12,
  delay: 0,
} satisfies ComponentProps<typeof StyleLine>

/** The board with a style waiting for the bar line. */
export const styleLineQueued = {
  ...styleLineBoard,
  queued: 'Coastal Highway',
} satisfies ComponentProps<typeof StyleLine>
