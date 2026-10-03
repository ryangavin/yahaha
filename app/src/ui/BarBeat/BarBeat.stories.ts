import type { Meta, StoryObj } from '@storybook/svelte-vite'
import BarBeat from './BarBeat.svelte'

const HUES = ['intro', 'main', 'ending', 'brk', 'fill']

/**
 * The display's count under the playing section: "Bar 3/4", the beat dots, and one line per bar
 * with the current bar filling in the section hue. 490 wide, now playing's section column.
 */
const meta = {
  title: 'Primitives/BarBeat',
  component: BarBeat,
  parameters: { layout: 'centered' },
  argTypes: {
    bar: { control: { type: 'number', min: 1, step: 1 } },
    bars: { control: { type: 'number', min: 1, max: 16, step: 1 } },
    beat: { control: { type: 'range', min: 0, max: 12, step: 1 } },
    beats: { control: { type: 'number', min: 1, max: 12, step: 1 } },
    hue: { control: 'select', options: HUES },
    progress: { control: { type: 'range', min: 0, max: 1, step: 0.01 } },
  },
} satisfies Meta<typeof BarBeat>

export default meta
type Story = StoryObj<typeof meta>

/** The board: bar 3 of 4, beat 3 of 4 in Main B, the third bar 62% filled. */
export const Board: Story = {
  args: { bar: 3, bars: 4, beat: 3, beats: 4, hue: 'main', progress: 0.62 },
}

/** Counting in on a two-bar intro, beat 1. */
export const Intro: Story = {
  args: { bar: 1, bars: 2, beat: 1, beats: 4, hue: 'intro' },
}

/** Stopped: every beat a ring, no bar filling. */
export const Stopped: Story = {
  args: { bar: 1, bars: 4, beat: 0, beats: 4, hue: 'main' },
}

/** A 3/4 ending on its last beat. */
export const ThreeFour: Story = {
  args: { bar: 2, bars: 2, beat: 3, beats: 3, hue: 'ending' },
}
