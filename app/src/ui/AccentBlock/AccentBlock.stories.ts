import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { expect, fn, userEvent, within } from 'storybook/test'
import AccentBlock from './AccentBlock.svelte'

/**
 * "The device" in a solid accent block: the style's name on the style line (a button that opens
 * the Browser) and the knob page's name over the knobs. Always `--g` on `--a`.
 */
const meta = {
  title: 'Primitives/AccentBlock',
  component: AccentBlock,
  args: { label: 'Sunday Drive Pop', onpress: fn(), tipAction: fn() },
  argTypes: {
    label: { control: 'text' },
    empty: { control: 'text' },
    as: { control: 'inline-radio', options: ['span', 'button'] },
    size: { control: 'inline-radio', options: ['line', 'knob'] },
    width: { control: 'number' },
    name: { control: 'text' },
    tip: { control: 'text' },
  },
} satisfies Meta<typeof AccentBlock>

export default meta
type Story = StoryObj<typeof meta>

/** The style name: the violet block, 18px medium label, square corners; a click opens the Browser. */
export const Board: Story = {
  args: {
    label: 'Sunday Drive Pop',
    as: 'button',
    size: 'line',
    name: 'Sunday Drive Pop: open the Browser',
    empty: 'No style',
    tip: 'browser.open',
  },
  play: async ({ canvasElement, args }) => {
    const button = within(canvasElement).getByRole('button', { name: 'Sunday Drive Pop: open the Browser' })
    await expect(button).toHaveAttribute('data-face', 'accent')
    await expect(button).toHaveAttribute('data-hue', 'a')
    await expect(button).toHaveAttribute('data-size', 'line')
    await expect(button).toHaveAttribute('data-tip', 'browser.open')
    await expect(button).not.toHaveAttribute('aria-pressed')
    await expect(args.tipAction).toHaveBeenCalledWith(button, 'browser.open')
    await userEvent.click(button)
    await expect(args.onpress).toHaveBeenCalledTimes(1)
    await expect(args.onpress).toHaveBeenLastCalledWith()
    button.focus()
    await userEvent.keyboard('{Enter}')
    await expect(args.onpress).toHaveBeenCalledTimes(2)
    await userEvent.keyboard(' ')
    await expect(args.onpress).toHaveBeenCalledTimes(3)
  },
}

const LONG_NAME = 'Bossa Nova Lounge Session With Strings And Brushes Deluxe 2'

/** A 59-character style name in a 240px block: one line, clipped with "…"; the full name stays in the button's name. */
export const LongName: Story = {
  args: { label: LONG_NAME, as: 'button', width: 240, name: `${LONG_NAME}: open the Browser` },
  play: async ({ canvasElement }) => {
    const button = within(canvasElement).getByRole('button', { name: `${LONG_NAME}: open the Browser` })
    await expect(button.textContent).toBe(LONG_NAME)
    // The block is fixed at 240px; the clip itself (ellipsis, one line) needs real layout, so the
    // Inspect agent judges it by eye: the test runner's jsdom applies no component CSS.
    await expect(button.style.width).toBe('240px')
  },
}

/** An empty label: the block shows its `empty` text ("No style") so it keeps its height and something to click. */
export const Empty: Story = {
  args: { label: '', as: 'button', empty: 'No style' },
  play: async ({ canvasElement }) => {
    const button = within(canvasElement).getByRole('button', { name: 'No style' })
    await expect(button).toHaveAttribute('data-face', 'accent')
  },
}
