import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { expect, fn, within } from 'storybook/test'
import HealthSlot from './HealthSlot.svelte'

/**
 * The app bar's audio health: "Audio" in muted grey while the sound is fine; the trouble in
 * Ending red (a failed plugin, dropouts, a busy CPU) as a text button that opens where to fix it.
 */
const meta = {
  title: 'Primitives/HealthSlot',
  component: HealthSlot,
  parameters: { layout: 'centered' },
  args: { onopen: fn(), tip: 'app.health', tipAction: fn() },
  argTypes: {
    failedPart: { control: 'select', options: [null, 0, 1, 2, 3] },
    synthOn: { control: 'boolean' },
    dropouts: { control: { type: 'number', min: 0, step: 1 } },
    bufferFrames: { control: 'select', options: [null, 64, 128, 256, 512, 1024] },
    cpu: { control: { type: 'range', min: 0, max: 1.2, step: 0.01 } },
    width: { control: { type: 'number', min: 0, step: 1 } },
    tip: { control: 'text' },
  },
} satisfies Meta<typeof HealthSlot>

export default meta
type Story = StoryObj<typeof meta>

/** The Stage board: calm, "Audio" in muted grey at the right edge of the app bar's 100px slot. */
export const Board: Story = {
  args: { failedPart: null, synthOn: true, dropouts: 0, bufferFrames: 256, cpu: 0.2, width: 100 },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement)
    await expect(canvas.getByRole('status')).toHaveTextContent('Audio health: fine')
    await expect(canvasElement.querySelector('[data-hue]')).toHaveAttribute('data-hue', 'm')
    const text = canvas.getByText('Audio', { exact: true })
    await expect(text).toHaveAttribute('aria-hidden', 'true')
    await expect(text).toHaveAttribute('data-tip', 'app.health')
    await expect(canvas.queryByRole('button')).toBeNull()
  },
}
