import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { expect, within } from 'storybook/test'
import StatusDot from './StatusDot.svelte'

/**
 * The small round light that says connected, running, waiting or changed, in the hue of what it
 * belongs to: solid with a soft glow (dark only), solid without, or a hollow 1px ring. Decorative
 * unless `name` is given; `visible: false` hides it but keeps its box.
 */
const meta = {
  title: 'Primitives/StatusDot',
  component: StatusDot,
  parameters: { layout: 'centered' },
  argTypes: {
    hue: { control: 'select', options: ['ok', 'd', 't', 'intro', 'main', 'ending', 'brk', 'fill'] },
    hollow: { control: 'boolean' },
    glow: { control: 'boolean' },
    size: { control: 'select', options: ['md', 'sm'] },
    visible: { control: 'boolean' },
    name: { control: 'text' },
  },
} satisfies Meta<typeof StatusDot>

export default meta
type Story = StoryObj<typeof meta>

/** The Launchkey status on the Stage board, connected: solid green with its glow (no glow in light). */
export const Board: Story = {
  args: { hue: 'ok' },
  play: async ({ canvasElement }) => {
    const dot = canvasElement.querySelector('[data-hue]')
    await expect(dot).toHaveAttribute('aria-hidden', 'true')
    await expect(dot).toHaveAttribute('data-hue', 'ok')
    await expect(dot).toHaveAttribute('data-face', 'solid')
    await expect(dot).toHaveAttribute('data-glow', 'true')
    await expect(dot).toHaveAttribute('data-size', 'md')
    await expect(within(canvasElement).queryByRole('img')).toBeNull()
  },
}

/** The dot before "Section" while Main B plays: Main green with its glow (no glow in light). */
export const Section: Story = {
  args: { hue: 'main' },
  play: async ({ canvasElement }) => {
    const dot = canvasElement.querySelector('[data-hue]')
    await expect(dot).toHaveAttribute('data-hue', 'main')
    await expect(dot).toHaveAttribute('data-face', 'solid')
    await expect(dot).toHaveAttribute('data-glow', 'true')
  },
}

/** The section dot while stopped: nothing visible, but the 6 × 6 box stays so the text after it doesn't move. */
export const Stopped: Story = {
  args: { hue: 'main', visible: false },
  play: async ({ canvasElement }) => {
    const dot = canvasElement.querySelector<HTMLElement>('[data-hue="main"]')
    await expect(dot).toBeInTheDocument()
    await expect(dot?.style.visibility).toBe('hidden')
  },
}

/** Sync start armed: a 1px green ring, empty inside, no glow. */
export const SyncStart: Story = {
  args: { hue: 'ok', hollow: true },
  play: async ({ canvasElement }) => {
    const dot = canvasElement.querySelector('[data-hue]')
    await expect(dot).toHaveAttribute('data-face', 'hollow')
    await expect(dot).toHaveAttribute('data-hue', 'ok')
    await expect(dot).toHaveAttribute('data-glow', 'false')
  },
}
