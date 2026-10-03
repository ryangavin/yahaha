import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { expect } from 'storybook/test'
import PartMarks from './PartMarks.svelte'

/**
 * The small marks after a part's sound name, in the sound cell and on the fader strip's name:
 * the edited dot, the plugin's ⚠ (missing) or ✕ (failed), and the words "off" or "bass".
 * Hidden from assistive tech; the parent's `aria-label` says the same, built with `marksText`.
 * Renders nothing when no mark shows.
 */
const meta = {
  title: 'Primitives/PartMarks',
  component: PartMarks,
  parameters: { layout: 'centered' },
  argTypes: {
    edited: { control: 'boolean' },
    missing: { control: 'boolean' },
    failed: { control: 'boolean' },
    off: { control: 'boolean' },
    bass: { control: 'boolean' },
    size: { control: 'select', options: ['cell', 'strip'] },
  },
} satisfies Meta<typeof PartMarks>

export default meta
type Story = StoryObj<typeof meta>

/** Right 2's sound cell on the Stage board: the sound was edited, so the 5px dot. */
export const Board: Story = {
  args: { edited: true },
  play: async ({ canvasElement }) => {
    const marks = canvasElement.querySelectorAll('[data-mark]')
    await expect(marks).toHaveLength(1)
    await expect(marks[0]).toHaveAttribute('data-mark', 'edited')
    await expect(marks[0]).toHaveAttribute('data-hue', 't')
    await expect(marks[0].parentElement).toHaveAttribute('aria-hidden', 'true')
  },
}

/** Right 3's plugin isn't installed: the orange ⚠ alone. */
export const Missing: Story = {
  args: { missing: true },
  play: async ({ canvasElement }) => {
    const marks = canvasElement.querySelectorAll('[data-mark]')
    await expect(marks).toHaveLength(1)
    await expect(marks[0]).toHaveAttribute('data-mark', 'missing')
    await expect(marks[0]).toHaveAttribute('data-hue', 'warn')
  },
}

/** The part's plugin failed to load or crashed: the red ✕ alone. */
export const Failed: Story = {
  args: { failed: true },
  play: async ({ canvasElement }) => {
    const marks = canvasElement.querySelectorAll('[data-mark]')
    await expect(marks).toHaveLength(1)
    await expect(marks[0]).toHaveAttribute('data-mark', 'failed')
    await expect(marks[0]).toHaveAttribute('data-hue', 'ending')
  },
}
