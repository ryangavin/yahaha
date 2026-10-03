import type { Meta, StoryObj } from '@storybook/svelte-vite'
import CountRow from './CountRow.svelte'
import { countRowBoard, countRowFill, countRowIntro } from './CountRow.fixtures'

const HUES = ['intro', 'main', 'ending', 'brk', 'fill']

/**
 * The section row's middle: beat blocks, "Bar 3/4", the playing section → the next one waiting,
 * and when the fill lands. One live status sentence for assistive tech.
 */
const meta = {
  title: 'Components/CountRow',
  component: CountRow,
  parameters: { layout: 'centered' },
  argTypes: {
    beat: { control: { type: 'range', min: 0, max: 12, step: 1 }, table: { category: 'BeatBlocks' } },
    beats: { control: { type: 'number', min: 1, max: 12, step: 1 }, table: { category: 'BeatBlocks' } },
    bar: { control: { type: 'number', min: 1, step: 1 } },
    bars: { control: { type: 'number', min: 1, step: 1 } },
    playing: { control: 'text', table: { category: 'SectionName' } },
    hue: { control: 'select', options: HUES, table: { category: 'SectionName' } },
    next: { control: 'text', table: { category: 'WaitingChip' } },
    nextHue: { control: 'select', options: HUES, table: { category: 'WaitingChip' } },
    fill: { control: 'text' },
  },
} satisfies Meta<typeof CountRow>

export default meta
type Story = StoryObj<typeof meta>

/** The dark board: beat 3 of 4 in Main B, bar 3/4, Main B → Main C, fill after bar 4. */
export const Board: Story = { args: { ...countRowBoard } }

/** Counting in on the intro, nothing queued: no arrow, no chip. */
export const Intro: Story = { args: { ...countRowIntro } }

/** A fill on its last beat, the ending waiting. */
export const FillToEnding: Story = { args: { ...countRowFill } }
