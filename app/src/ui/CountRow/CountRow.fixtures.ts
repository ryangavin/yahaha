import type { ComponentProps } from 'svelte'
import type CountRow from './CountRow.svelte'

/** The dark board's count row: beat 3 of 4, bar 3 of 4, Main B playing, Main C next, the fill after bar 4. */
export const countRowBoard = {
  beat: 3,
  beats: 4,
  bar: 3,
  bars: 4,
  playing: 'Main B',
  hue: 'main',
  next: 'Main C',
  nextHue: 'main',
  fill: 'fill after bar 4',
} satisfies ComponentProps<typeof CountRow>

/** The intro counting in, nothing queued. */
export const countRowIntro = {
  beat: 1,
  beats: 4,
  bar: 1,
  bars: 2,
  playing: 'Intro I',
  hue: 'intro',
  next: '',
  nextHue: 'main',
  fill: '',
} satisfies ComponentProps<typeof CountRow>

/** A fill on its last beat, the ending waiting. */
export const countRowFill = {
  beat: 4,
  beats: 4,
  bar: 1,
  bars: 1,
  playing: 'Fill',
  hue: 'fill',
  next: 'Ending II',
  nextHue: 'ending',
  fill: '',
} satisfies ComponentProps<typeof CountRow>
