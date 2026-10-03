import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { fn } from 'storybook/test'
import SoundCell from './SoundCell.svelte'

/**
 * One part on the display's sound row: the short name in the part hue (opens Channel), then the
 * sound's number and name (opens the quick sound list), with the part's marks after it.
 */
const meta = {
  title: 'Primitives/SoundCell',
  component: SoundCell,
  parameters: { layout: 'centered' },
  args: { onpart: fn(), onsound: fn(), tipAction: fn() },
  argTypes: {
    part: { control: 'text' },
    partName: { control: 'text' },
    hue: { control: 'inline-radio', options: ['r1', 'r2', 'r3', 'l'] },
    number: { control: 'text' },
    sound: { control: 'text' },
    off: { control: 'boolean' },
    soundName: { control: 'text' },
    partTip: { control: 'text' },
    soundTip: { control: 'text' },
  },
} satisfies Meta<typeof SoundCell>

export default meta
type Story = StoryObj<typeof meta>

/** The board's Right 1: R1 in blue, 1 Stage Grand. */
export const Board: Story = {
  args: {
    part: 'R1',
    partName: 'Right 1',
    hue: 'r1',
    number: '1',
    sound: 'Stage Grand',
    off: false,
    partTip: 'part.right1.select',
  },
}

/** The board's Right 3, off: R3 and its number dimmed, the name muted. */
export const Off: Story = {
  args: { part: 'R3', partName: 'Right 3', hue: 'r3', number: '57', sound: 'Brass Section', off: true },
}

/** A long sound name ends in an ellipsis inside the cell. */
export const LongName: Story = {
  args: { part: 'L', partName: 'Left', hue: 'l', number: '208', sound: 'Concert Grand Piano Bright Stereo' },
}
