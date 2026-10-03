import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { expect, fn, userEvent, within } from 'storybook/test'
import Button from './Button.svelte'

const NONE = 'none'
const SIZES = ['icon', 'md', 'band', 'pair', 'cell', 'caret']
const HUES = ['t', 't2', 'm', 'a', 'lamp', 'rec', 'ok', 'r1', 'r2', 'r3', 'l', 'intro', 'main', 'ending', 'brk', 'fill']
const SYMBOLS = ['prev', 'next', 'up', 'down', 'plus', 'minus', 'caret']
/** A select whose first option is `none` (undefined), so the control can go back to "no value". */
const optional = (options: string[]) => ({ control: 'select' as const, options: [NONE, ...options], mapping: { [NONE]: undefined } })
/** none / false / true, for a boolean whose undefined means "no attribute". */
const threeWay = {
  control: 'select' as const,
  options: [NONE, 'false', 'true'],
  mapping: { [NONE]: undefined, false: false, true: true },
}

/**
 * The plain button: does one thing when pressed (Panic, Stop, a page step, a One Touch), and shows
 * when that thing is chosen, switched on or waiting. Every face is a prop; the parent acts on
 * `onpress`, `onhold`, `onlongpress` and `onlongrelease`.
 */
const meta = {
  title: 'Primitives/Button',
  component: Button,
  parameters: { layout: 'centered' },
  args: { onpress: fn(), onhold: fn(), onlongpress: fn(), onlongrelease: fn(), tipAction: fn() },
  argTypes: {
    label: { control: 'text' },
    name: { control: 'text' },
    controls: { control: 'text' },
    tip: { control: 'text' },
    compact: { control: 'boolean' },
    strong: { control: 'boolean' },
    on: { control: 'boolean' },
    chosen: { control: 'boolean' },
    waiting: { control: 'boolean' },
    expanded: { control: 'boolean' },
    disabled: { control: 'boolean' },
    hold: { control: 'boolean' },
    size: { control: 'select', options: SIZES },
    hue: { control: 'select', options: HUES },
    symbol: optional(SYMBOLS),
    join: optional(['start', 'end']),
    popup: optional(['dialog', 'menu']),
    bar: threeWay,
    pressed: threeWay,
  },
} satisfies Meta<typeof Button>

export default meta
type Story = StoryObj<typeof meta>

/** The first Button on the Stage board: the Metronome ▾ caret, joined to its lamp, closed. */
export const Board: Story = {
  args: {
    symbol: 'caret',
    size: 'caret',
    join: 'end',
    popup: 'dialog',
    expanded: false,
    name: 'Metronome settings',
    tip: 'metronome.settings',
  },
  play: async ({ canvasElement, args }) => {
    const button = within(canvasElement).getByRole('button', { name: 'Metronome settings' })
    await expect(button).toHaveAttribute('aria-haspopup', 'dialog')
    await expect(button).toHaveAttribute('aria-expanded', 'false')
    await expect(button).not.toHaveAttribute('aria-pressed')
    await expect(button).not.toHaveAttribute('aria-disabled')
    await expect(button).toHaveAttribute('data-face', 'off')
    await expect(button).toHaveAttribute('data-tip', 'metronome.settings')
    await expect(args.tipAction).toHaveBeenCalledWith(button, 'metronome.settings')
    await userEvent.click(button)
    await expect(args.onpress).toHaveBeenCalledTimes(1)
  },
}
