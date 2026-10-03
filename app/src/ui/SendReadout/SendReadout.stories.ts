import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { fn } from 'storybook/test'
import SendReadout from './SendReadout.svelte'

/** The band's effect sends on the style line; a click opens Effects. */
const meta = {
  title: 'Primitives/SendReadout',
  component: SendReadout,
  parameters: { layout: 'centered' },
  args: { reverb: 40, chorus: 12, delay: 0, onpress: fn(), tipAction: fn() },
  argTypes: {
    reverb: { control: { type: 'range', min: 0, max: 127, step: 1 } },
    chorus: { control: { type: 'range', min: 0, max: 127, step: 1 } },
    delay: { control: { type: 'range', min: 0, max: 127, step: 1 } },
    label: { control: 'text' },
    name: { control: 'text' },
    tip: { control: 'text' },
  },
} satisfies Meta<typeof SendReadout>

export default meta
type Story = StoryObj<typeof meta>

/** The board: Band Reverb 40 Chorus 12 Delay 0. */
export const Board: Story = {
  args: { reverb: 40, chorus: 12, delay: 0, label: 'Band' },
}

/** Every send at its top. */
export const Full: Story = {
  args: { reverb: 127, chorus: 127, delay: 127 },
}
