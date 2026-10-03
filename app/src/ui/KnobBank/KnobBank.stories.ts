import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { fn } from 'storybook/test'
import KnobBank from './KnobBank.svelte'
import { styleKnobPage, styleKnobs } from './KnobBank.fixtures'

/**
 * The band's Knobs section: the knob page's accent block and counter, the page ▲ ▼ buttons and
 * eight Knobs. Every change is a callback carrying the knob's position.
 */
const meta = {
  title: 'Components/KnobBank',
  component: KnobBank,
  parameters: { layout: 'centered' },
  args: {
    knobs: styleKnobs,
    pageLabel: styleKnobPage.label,
    count: styleKnobPage.count,
    tipAction: fn(),
    onpageup: fn(),
    onpagedown: fn(),
    onpress: fn(),
    onstep: fn(),
  },
  argTypes: {
    knobs: { control: 'object' },
    pageLabel: { control: 'text' },
    count: { control: 'text' },
  },
} satisfies Meta<typeof KnobBank>

export default meta
type Story = StoryObj<typeof meta>

/** The board: knob page 1 of 6, Style; knob 7 unused. */
export const Board: Story = {}
