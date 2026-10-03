import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { expect, fireEvent, fn, userEvent, within } from 'storybook/test'
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
  parameters: { layout: 'centered' },
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

/** Lit: the lime lamp face, ink label and the small code. */
export const On: Story = {
  args: { label: 'Accomp', code: 'ACMP', on: true },
  play: async ({ canvasElement }) => {
    const button = within(canvasElement).getByRole('button', { name: 'Accomp ACMP' })
    await expect(button).toHaveAttribute('aria-pressed', 'true')
    await expect(button).toHaveAttribute('data-face', 'on')
  },
}

/**
 * Controlled: a click, Space or Enter asks for `!on` through `ontoggle` and changes nothing itself;
 * the parent moves `on`. A short press never long-presses.
 */
export const Toggles: Story = {
  args: { label: 'Unison' },
  play: async ({ canvasElement, args }) => {
    const button = within(canvasElement).getByRole('button', { name: 'Unison' })
    await userEvent.click(button)
    await expect(args.ontoggle).toHaveBeenCalledTimes(1)
    await expect(args.ontoggle).toHaveBeenLastCalledWith(true)
    await expect(button).toHaveAttribute('aria-pressed', 'false')
    await expect(button).toHaveAttribute('data-face', 'off')
    button.focus()
    await userEvent.keyboard(' ')
    await userEvent.keyboard('{Enter}')
    await expect(args.ontoggle).toHaveBeenCalledTimes(3)
    for (const n of [1, 2, 3]) await expect(args.ontoggle).toHaveBeenNthCalledWith(n, true)
    await expect(args.onlongpress).not.toHaveBeenCalled()
  },
}

/** Shown, not pressable: dimmed label on the off face, 64 × 28 (a settings row's On/Off). */
export const Disabled: Story = {
  args: { label: 'Off', size: 'sm', width: 64, disabled: true, name: 'Manual Bass, works with Upper on' },
  play: async ({ canvasElement, args }) => {
    const button = within(canvasElement).getByRole('button', { name: 'Manual Bass, works with Upper on' })
    await expect(button).toHaveAttribute('aria-disabled', 'true')
    await expect(button).toHaveAttribute('data-face', 'disabled')
    await expect(button).toHaveAttribute('data-contrast', 'dim')
    await userEvent.click(button)
    await expect(button).toHaveAttribute('aria-pressed', 'false')
    await expect(args.ontoggle).not.toHaveBeenCalled()
    const init = { pointerId: 1, button: 0, clientX: 0, clientY: 0 }
    await fireEvent.pointerDown(button, init)
    await new Promise((resolve) => setTimeout(resolve, 500))
    await expect(args.onlongpress).not.toHaveBeenCalled()
    await fireEvent.pointerUp(button, init)
    await expect(args.onlongrelease).not.toHaveBeenCalled()
  },
}

/**
 * Looper's Rec armed in the lamp row: no fill, a 1px `--rec` outline and label. A click asks to
 * toggle; the parent, not the click, lights it.
 */
export const Armed: Story = {
  args: {
    label: 'Looper',
    size: 'cell',
    waiting: true,
    hue: 'rec',
    name: 'Looper, rec armed. Long press: loop rec',
    tip: 'looper.rec',
  },
  parameters: { layout: 'padded' },
  play: async ({ canvasElement, args }) => {
    const button = within(canvasElement).getByRole('button', { name: 'Looper, rec armed. Long press: loop rec' })
    await expect(button).toHaveAttribute('data-face', 'waiting')
    await expect(button).toHaveAttribute('data-hue', 'rec')
    await expect(button).toHaveAttribute('aria-pressed', 'false')
    await userEvent.click(button)
    await expect(args.ontoggle).toHaveBeenCalledTimes(1)
    await expect(args.ontoggle).toHaveBeenLastCalledWith(true)
    await expect(button).toHaveAttribute('data-face', 'waiting')
  },
}
