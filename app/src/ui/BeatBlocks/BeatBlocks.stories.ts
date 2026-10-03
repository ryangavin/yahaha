import type { Meta, StoryObj } from '@storybook/svelte-vite'
import BeatBlocks from './BeatBlocks.svelte'

/**
 * The count row's beat blocks: past `--past`, the current beat in the section hue with its glow,
 * the rest the button face; beat one always has a white top edge.
 */
const meta = {
  title: 'Primitives/BeatBlocks',
  component: BeatBlocks,
  parameters: { layout: 'centered' },
  argTypes: {
    beats: { control: { type: 'number', min: 1, max: 12, step: 1 } },
    beat: { control: { type: 'range', min: 0, max: 12, step: 1 } },
    hue: { control: 'select', options: ['intro', 'main', 'ending', 'brk', 'fill'] },
  },
} satisfies Meta<typeof BeatBlocks>

export default meta
type Story = StoryObj<typeof meta>

/** The board: beat 3 of 4 in Main B, beats 1 and 2 past, beat 4 to come. */
export const Board: Story = {
  args: { beats: 4, beat: 3, hue: 'main' },
}

/** Beat one of a fill: the current block in the fill hue under beat one's white edge. */
export const BeatOne: Story = {
  args: { beats: 4, beat: 1, hue: 'fill' },
}

/** Stopped: no current beat, every block the button face. */
export const Stopped: Story = {
  args: { beats: 4, beat: 0, hue: 'main' },
}

/** A 3/4 style in the ending. */
export const ThreeFour: Story = {
  args: { beats: 3, beat: 2, hue: 'ending' },
}
