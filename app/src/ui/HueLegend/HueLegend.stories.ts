import type { Meta, StoryObj } from '@storybook/svelte-vite'
import HueLegend from './HueLegend.svelte'

/** A row of words, each after a short swatch in its hue: the pads header's section legend. */
const meta = {
  title: 'Primitives/HueLegend',
  component: HueLegend,
  parameters: { layout: 'centered' },
  args: {
    items: [
      { label: 'Intro', hue: 'intro' },
      { label: 'Main', hue: 'main' },
      { label: 'Ending', hue: 'ending' },
      { label: 'Break', hue: 'brk' },
      { label: 'Fill', hue: 'fill' },
    ],
  },
  argTypes: {
    items: { control: 'object' },
  },
} satisfies Meta<typeof HueLegend>

export default meta
type Story = StoryObj<typeof meta>

/** The section legend on the board: Intro, Main, Ending, Break, Fill. */
export const Board: Story = {}
