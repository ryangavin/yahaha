import type { Meta, StoryObj } from '@storybook/svelte-vite'
import SectionName from './SectionName.svelte'

/** The playing section's name on the display: 44px light text in its hue, with its glow. */
const meta = {
  title: 'Primitives/SectionName',
  component: SectionName,
  parameters: { layout: 'centered' },
  argTypes: {
    label: { control: 'text' },
    hue: { control: 'select', options: ['intro', 'main', 'ending', 'brk', 'fill'] },
  },
} satisfies Meta<typeof SectionName>

export default meta
type Story = StoryObj<typeof meta>

/** The board: "Main B", 44px light green with its glow. */
export const Board: Story = {
  args: { label: 'Main B', hue: 'main' },
}

/** An intro, in the gold intro hue. */
export const Intro: Story = {
  args: { label: 'Intro II', hue: 'intro' },
}
