import type { ComponentProps } from 'svelte'
import type SectionRow from './SectionRow.svelte'

/** The dark board's section row: Accomp on; Metronome, Unison and help off. */
export const sectionRowBoard = {
  accomp: true,
  metronome: false,
  metronomeOpen: false,
  unison: false,
  help: false,
} satisfies ComponentProps<typeof SectionRow>
