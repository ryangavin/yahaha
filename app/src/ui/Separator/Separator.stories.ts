import type { Meta, StoryObj } from '@storybook/svelte-vite'
import Separator from './Separator.svelte'

/**
 * The 1px vertical hairline between groups in a row, in `--line`: 16px tall between the app bar's
 * tab runs and before its right area. Decorative; the parent spaces it.
 */
const meta = {
  title: 'Primitives/Separator',
  component: Separator,
  parameters: { layout: 'centered' },
  argTypes: {
    length: { control: 'inline-radio', options: ['short', 'full'] },
  },
} satisfies Meta<typeof Separator>

export default meta
type Story = StoryObj<typeof meta>

/** The app bar's hairline between Harm/Arp and Library: 1 × 16px. */
export const Board: Story = {
  args: { length: 'short' },
}
