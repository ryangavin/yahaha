import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { fn } from 'storybook/test'
import SoundRow from './SoundRow.svelte'
import { soundRowBoard, soundRowClean } from './SoundRow.fixtures'

/**
 * The display's bottom row: the rack readout, then the four keyboard parts' sounds with their
 * marks. 814 × 44, each cell under a hairline.
 */
const meta = {
  title: 'Components/SoundRow',
  component: SoundRow,
  parameters: { layout: 'centered' },
  args: { ...soundRowBoard, tipAction: fn(), onrack: fn(), onpart: fn(), onsound: fn() },
  argTypes: {
    rack: { control: 'text', table: { category: 'RackCell' } },
    slot: { control: 'text', table: { category: 'RackCell' } },
    modified: { control: 'boolean', table: { category: 'StatusDot' } },
    parts: { control: 'object', table: { category: 'SoundCell' } },
  },
} satisfies Meta<typeof SoundRow>

export default meta
type Story = StoryObj<typeof meta>

/** The dark board: Sunday drive modified on A1; R2 edited; R3 off with its plugin missing. */
export const Board: Story = {}

/** A clean rack: every part on, R2's plugin failed, Left playing the Style's bass. */
export const Clean: Story = { args: { ...soundRowClean } }
