import type { Meta, StoryObj } from '@storybook/svelte-vite'
import SectionName from './SectionName.svelte'

/** The playing section's name, solid in its hue: 18px on the count row, 44px with a glow on the display. */
const meta = {
  title: 'Primitives/SectionName',
  component: SectionName,
  parameters: { layout: 'centered' },
  argTypes: {
    label: { control: 'text' },
    hue: { control: 'select', options: ['intro', 'main', 'ending', 'brk', 'fill'] },
    size: { control: 'inline-radio', options: ['count', 'display'] },
  },
} satisfies Meta<typeof SectionName>

export default meta
type Story = StoryObj<typeof meta>

/** The board's display: "Main B", 44px light green with its glow. */
export const Board: Story = {
  args: { label: 'Main B', hue: 'main', size: 'display' },
}

/** The board's count row: "Main B", 18px light green. */
export const Count: Story = {
  args: { label: 'Main B', hue: 'main', size: 'count' },
}

/** An intro on the display, in the gold intro hue. */
export const Intro: Story = {
  args: { label: 'Intro II', hue: 'intro', size: 'display' },
}
