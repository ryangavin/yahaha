import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { expect, fn, within } from 'storybook/test'
import LampButton from './LampButton.svelte'

/**
 * The on/off control the whole canvas uses: Accomp, Metronome, part On, Sound, Looper.
 * Lit is the lamp face with an ink label; off is the plain button face with a grey label; armed
 * is the waiting face, outlined in its hue. Controlled: a click asks for `!on` through `ontoggle`.
 * A long press (or right-click) calls `onlongpress` / `onlongrelease` and never toggles.
 */
const meta = {
  title: 'Primitives/LampButton',
  component: LampButton,
  args: { ontoggle: fn(), onlongpress: fn(), onlongrelease: fn(), tipAction: fn() },
  argTypes: {
    label: { control: 'text' },
    code: { control: 'text' },
    name: { control: 'text' },
    tip: { control: 'text' },
    on: { control: 'boolean' },
    disabled: { control: 'boolean' },
    rec: { control: 'boolean' },
    waiting: { control: 'boolean' },
    size: { control: 'select', options: ['md', 'sm', 'cell'] },
    hue: { control: 'select', options: ['lamp', 'rec'] },
    join: { control: 'select', options: ['none', 'start', 'end'], mapping: { none: undefined } },
    width: { control: 'number' },
  },
} satisfies Meta<typeof LampButton>

export default meta
type Story = StoryObj<typeof meta>

/** The first LampButton on the Stage board: Accomp, lit, with its Genos code (section row). */
export const Board: Story = {
  args: { label: 'Accomp', code: 'ACMP', on: true, tip: 'transport.acmp' },
  play: async ({ canvasElement, args }) => {
    const button = within(canvasElement).getByRole('button', { name: 'Accomp ACMP' })
    await expect(button).toHaveAttribute('aria-pressed', 'true')
    await expect(button).toHaveAttribute('data-face', 'on')
    await expect(button).toHaveAttribute('data-tip', 'transport.acmp')
    await expect(args.tipAction).toHaveBeenCalledWith(button, 'transport.acmp')
  },
}
