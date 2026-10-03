import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { expect, fn, within } from 'storybook/test'
import StatusLine from './StatusLine.svelte'

/**
 * The one line between the band and the keys that says what the app just did or refused
 * (`state.message`). An error leads with the ⚠; the text stays white. A click, Enter or Space
 * calls `onclear`. Empty, the 20px row stays so the keys never move.
 */
const meta = {
  title: 'Primitives/StatusLine',
  component: StatusLine,
  parameters: { layout: 'centered' },
  args: { onclear: fn(), tip: 'display.status', tipAction: fn() },
  argTypes: {
    text: { control: 'text' },
    error: { control: 'boolean' },
    seq: { control: 'number' },
    width: { control: 'number' },
    tip: { control: 'text' },
  },
} satisfies Meta<typeof StatusLine>

export default meta
type Story = StoryObj<typeof meta>

/** The Stage at the board fixture: no message, so an empty 20px row on the ground. */
export const Board: Story = {
  args: { text: null, width: 1392 },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement)
    const status = canvas.getByRole('status')
    await expect(status.textContent).toBe('')
    await expect(canvas.queryByRole('button')).toBeNull()
  },
}
