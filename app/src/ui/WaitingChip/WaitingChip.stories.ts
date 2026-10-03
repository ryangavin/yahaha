import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { expect, within } from 'storybook/test'
import WaitingChip from './WaitingChip.svelte'

/**
 * What comes next, in an outline of its hue: the next section on the count row and the display,
 * or the style waiting for the bar line on the style line. A readout, not a control.
 */
const meta = {
  title: 'Primitives/WaitingChip',
  component: WaitingChip,
  parameters: { layout: 'centered' },
  argTypes: {
    label: { control: 'text' },
    hue: { control: 'select', options: ['intro', 'main', 'ending', 'brk', 'fill', 'a', 't'] },
    size: { control: 'inline-radio', options: ['count', 'line', 'display'] },
  },
} satisfies Meta<typeof WaitingChip>

export default meta
type Story = StoryObj<typeof meta>

/** The Stage count row's next section: a 1px green outline round an 18px green "Main C". */
export const Board: Story = {
  args: { label: 'Main C', hue: 'main', size: 'count' },
  play: async ({ canvasElement }) => {
    const chip = within(canvasElement).getByText('Main C')
    await expect(chip).toHaveAttribute('data-face', 'waiting')
    await expect(chip).toHaveAttribute('data-hue', 'main')
    await expect(chip).toHaveAttribute('data-size', 'count')
    await expect(within(canvasElement).queryByRole('button')).toBeNull()
    await expect(canvasElement.querySelector('a[href], button, input, select, textarea, [tabindex]')).toBeNull()
  },
}

/** The style line's queued style, waiting for the bar line: "Coastal Highway" outlined in the accent. */
export const QueuedStyle: Story = {
  args: { label: 'Coastal Highway', hue: 'a', size: 'line' },
  play: async ({ canvasElement }) => {
    const chip = within(canvasElement).getByText('Coastal Highway')
    await expect(chip).toHaveAttribute('data-hue', 'a')
    await expect(chip).toHaveAttribute('data-size', 'line')
  },
}

/** Nothing waiting: an empty label draws no chip at all, not an empty outline. */
export const Empty: Story = {
  args: { label: '', hue: 'main', size: 'count' },
  parameters: { rendersNothing: true },
  play: async ({ canvasElement }) => {
    await expect(canvasElement.querySelector('[data-face]')).toBeNull()
  },
}
