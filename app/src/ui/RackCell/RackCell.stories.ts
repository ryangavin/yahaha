import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { fn } from 'storybook/test'
import RackCell from './RackCell.svelte'

/** The rack readout at the head of the sound row: "Rack · A1" over the rack's name; a click opens the Rack page. */
const meta = {
  title: 'Primitives/RackCell',
  component: RackCell,
  parameters: { layout: 'centered' },
  args: { onpress: fn(), tipAction: fn() },
  argTypes: {
    rack: { control: 'text' },
    slot: { control: 'text' },
    label: { control: 'text' },
    name: { control: 'text' },
    tip: { control: 'text' },
  },
} satisfies Meta<typeof RackCell>

export default meta
type Story = StoryObj<typeof meta>

/** The board's rack, without its modified dot (the sound row adds it): Rack · A1, Sunday drive. */
export const Board: Story = {
  args: { rack: 'Sunday drive', slot: 'A1', label: 'Rack', tip: 'nav.rack' },
}

/** A rack not on a Quick Rack slot. */
export const NoSlot: Story = {
  args: { rack: 'Ballad night', slot: '' },
}
