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
