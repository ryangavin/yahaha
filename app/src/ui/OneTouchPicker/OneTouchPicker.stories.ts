import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { fn } from 'storybook/test'
import OneTouchPicker from './OneTouchPicker.svelte'

/**
 * One Touch on the style line: "One Touch OTS" muted, then 1-4 as faceless light numbers, the
 * applied one in the accent over a 2px accent bar.
 */
const meta = {
  title: 'Primitives/OneTouchPicker',
  component: OneTouchPicker,
  parameters: { layout: 'centered' },
  args: { tipAction: fn(), onapply: fn() },
  argTypes: {
    applied: { control: { type: 'inline-radio' }, options: [0, 1, 2, 3, 4] },
    count: { control: { type: 'number', min: 1, max: 4, step: 1 } },
    label: { control: 'text' },
    code: { control: 'text' },
    name: { control: 'text' },
  },
} satisfies Meta<typeof OneTouchPicker>

export default meta
type Story = StoryObj<typeof meta>

/** The board: One Touch 2 applied. */
export const Board: Story = { args: { applied: 2 } }

/** None applied: every number muted, no bar. */
export const NoneApplied: Story = { args: { applied: 0 } }
