import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { expect, within } from 'storybook/test'
import WaitingChip from './WaitingChip.svelte'

/**
 * What comes next, as plain light text quieter than what plays (Round 2: no outline): the next
 * section on the count row and the display, or the style waiting for the bar line on the style
 * line. Neutral (`t`) is muted grey; a hue tints it. A readout, not a control.
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

/** The count row's next section, tinted in the Main hue: an 18px light green "Main C". */
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

/** Round 2's next section on the display: a 28px light, muted "Main C" (neutral `t`). */
export const Display: Story = {
  args: { label: 'Main C', hue: 't', size: 'display' },
}

/** The style line's queued style, waiting for the bar line: "Coastal Highway" in the accent. */
export const QueuedStyle: Story = {
  args: { label: 'Coastal Highway', hue: 'a', size: 'line' },
  play: async ({ canvasElement }) => {
    const chip = within(canvasElement).getByText('Coastal Highway')
    await expect(chip).toHaveAttribute('data-hue', 'a')
    await expect(chip).toHaveAttribute('data-size', 'line')
  },
}

/** Nothing waiting: an empty label draws nothing at all. */
export const Empty: Story = {
  args: { label: '', hue: 'main', size: 'count' },
  parameters: { rendersNothing: true },
  play: async ({ canvasElement }) => {
    await expect(canvasElement.querySelector('[data-face]')).toBeNull()
  },
}
