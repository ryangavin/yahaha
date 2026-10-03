import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { expect, fn, userEvent, within } from 'storybook/test'
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

/** A refusal: the warn mark, then the white text. */
export const Error: Story = {
  args: { text: "Can't delete Sunday drive: it's the loaded rack.", error: true, seq: 4, width: 1392 },
  play: async ({ canvasElement }) => {
    const status = within(canvasElement).getByRole('status')
    const button = within(status).getByRole('button', {
      name: "Error: Can't delete Sunday drive: it's the loaded rack.",
    })
    await expect(button).toHaveAttribute('data-tip', 'display.status')
    await expect(status.querySelector('svg[data-hue="warn"]')).toBeInTheDocument()
  },
}

/** A click, Enter or Space on the line asks the parent to clear it (`onclear`). */
export const Clears: Story = {
  args: { text: 'One Touch 3 applied · Recovered: Sunday drive', seq: 6, width: 1392 },
  play: async ({ canvasElement, args }) => {
    const button = within(canvasElement).getByRole('button', {
      name: 'One Touch 3 applied · Recovered: Sunday drive',
    })
    await expect(button).toHaveAttribute('data-tip', 'display.status')
    await userEvent.click(button)
    await expect(args.onclear).toHaveBeenCalledTimes(1)
    await expect(args.onclear).toHaveBeenLastCalledWith()
    button.focus()
    await userEvent.keyboard('{Enter}')
    await expect(args.onclear).toHaveBeenCalledTimes(2)
    await userEvent.keyboard(' ')
    await expect(args.onclear).toHaveBeenCalledTimes(3)
  },
}

const LONG_TEXT =
  'Rhodes Soft + Strings could not load: the plugin Sampler Deluxe is missing, so Right 1 plays its SoundFont voice until the plugin is installed and scanned again.'

/** A message too long for 600px: one line ending in "…", the row stays 20px; the whole text stays in the name. */
export const LongText: Story = {
  args: { text: LONG_TEXT, error: true, seq: 7, width: 600 },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement)
    const button = canvas.getByRole('button', { name: `Error: ${LONG_TEXT}` })
    await expect(button).toHaveAttribute('data-tip', 'display.status')
    await expect(canvas.getByRole('status').style.width).toBe('600px')
  },
}
