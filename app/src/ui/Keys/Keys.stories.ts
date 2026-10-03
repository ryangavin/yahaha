import type { Meta, StoryObj } from '@storybook/svelte-vite'
import Keys from './Keys.svelte'
import { boardKeys, range49, range61 } from './Keys.fixtures'

/**
 * The 56px OLED key strip: dark keys over the keyboard's range. With a split, the left zone is
 * tinted under a teal top line and a 2px white line marks the split; held keys fill with their
 * part's hue and glow (no glow in light). Notes are MIDI numbers, C3 = 60.
 */
const meta = {
  title: 'Primitives/Keys',
  component: Keys,
  parameters: { layout: 'centered' },
  args: { ...boardKeys },
  argTypes: {
    range: { control: 'object' },
    split: { control: { type: 'number', min: 0, max: 127, step: 1 } },
    heldLeft: { control: 'object' },
    heldRight: { control: 'object' },
    rightPart: { control: 'inline-radio', options: ['r1', 'r2', 'r3'] },
    width: { control: { type: 'number', min: 200, step: 1 } },
  },
} satisfies Meta<typeof Keys>

export default meta
type Story = StoryObj<typeof meta>

/** The Stage board: 61 keys, split F#2, the left hand's Am7 in teal, E4 and A4 in Right 1's blue. */
export const Board: Story = {}

/** Nothing held: the split and the left zone's tint only. */
export const Idle: Story = {
  args: { heldLeft: [], heldRight: [] },
}

/** No split: one zone, every held key in the right part's hue (here Right 2, with a black key). */
export const NoSplit: Story = {
  args: { split: null, heldLeft: [], heldRight: [60, 63, 67], rightPart: 'r2' },
}

/** A 49-key keyboard at a narrower width, split B2, held black keys in both zones. */
export const Keys49: Story = {
  args: { range: range49, split: 59, heldLeft: [49, 54, 58], heldRight: [73, 78], rightPart: 'r3', width: 960 },
}

/** The full 61 keys with Right 1 playing high, split at C3. */
export const HighSplit: Story = {
  args: { range: range61, split: 60, heldLeft: [48, 52, 55], heldRight: [84, 88, 91] },
}
