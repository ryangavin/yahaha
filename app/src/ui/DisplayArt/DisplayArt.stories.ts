import type { Meta, StoryObj } from '@storybook/svelte-vite'
import DisplayArt from './DisplayArt.svelte'

/** The display's background art, 528 × 300 at its right edge, its core dimmed so the chord stays brightest. */
const meta = {
  title: 'Primitives/DisplayArt',
  component: DisplayArt,
  parameters: { layout: 'centered' },
} satisfies Meta<typeof DisplayArt>

export default meta
type Story = StoryObj<typeof meta>

/** The board's art. */
export const Board: Story = {}
