import type { ComponentProps } from 'svelte'
import { countRowBoard } from '../CountRow/CountRow.fixtures'
import type SectionRow from './SectionRow.svelte'

/** The dark board's section row: Accomp on, the board's count, Metronome, Unison and help off. */
export const sectionRowBoard = {
  count: countRowBoard,
  accomp: true,
  metronome: false,
  metronomeOpen: false,
  unison: false,
  help: false,
} satisfies ComponentProps<typeof SectionRow>
