import type { ComponentProps } from 'svelte'
import { nowPlayingBoard, nowPlayingStopped } from '../NowPlaying/NowPlaying.fixtures'
import { soundRowBoard, soundRowClean } from '../SoundRow/SoundRow.fixtures'
import { styleLineBoard, styleLineQueued } from '../StyleLine/StyleLine.fixtures'
import type Display from './Display.svelte'

/** The dark board's display: Sunday Drive Pop, Am7 in Main B → Main C at 104, Sunday drive's sounds. */
export const displayBoard = {
  styleLine: styleLineBoard,
  nowPlaying: nowPlayingBoard,
  soundRow: soundRowBoard,
} satisfies ComponentProps<typeof Display>

/** Stopped, a style queued, a clean rack. */
export const displayStopped = {
  styleLine: styleLineQueued,
  nowPlaying: nowPlayingStopped,
  soundRow: soundRowClean,
} satisfies ComponentProps<typeof Display>
