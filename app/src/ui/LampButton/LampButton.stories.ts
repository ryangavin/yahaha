import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { expect, fn, userEvent, within } from 'storybook/test'
import LampButton from './LampButton.svelte'

/**
 * The on/off control the whole canvas uses: Accomp, Metronome, part On, Sound, Looper.
 * Lit is the lamp face with an ink label; off is the plain button face with a grey label.
 */
const meta = {
  title: 'Primitives/LampButton',
  component: LampButton,
  args: { ontoggle: fn() },
} satisfies Meta<typeof LampButton>

export default meta
type Story = StoryObj<typeof meta>

/** The off face: the plain button face, grey label (Stage section row). */
export const Off: Story = {
  args: { label: 'Metronome' },
}

/** Lit: the lamp face, ink label, the Genos code small after it (Stage section row). */
export const On: Story = {
  args: { label: 'Accomp', code: 'ACMP', on: true },
}

/** A click lights it and reports `true`; Space turns it off again, Enter back on. */
export const Toggles: Story = {
  args: { label: 'Unison' },
  play: async ({ canvasElement, args }) => {
    const button = within(canvasElement).getByRole('button', { name: 'Unison' })
    await expect(button).toHaveAttribute('aria-pressed', 'false')
    await userEvent.click(button)
    await expect(button).toHaveAttribute('aria-pressed', 'true')
    await expect(args.ontoggle).toHaveBeenLastCalledWith(true)
    await userEvent.keyboard(' ')
    await expect(button).toHaveAttribute('aria-pressed', 'false')
    await expect(args.ontoggle).toHaveBeenLastCalledWith(false)
    await userEvent.keyboard('{Enter}')
    await expect(button).toHaveAttribute('aria-pressed', 'true')
    await expect(args.ontoggle).toHaveBeenCalledTimes(3)
  },
}

/** Shown, not pressable: the dimmed label (Settings Chord: Manual Bass while Upper is off). */
export const Disabled: Story = {
  args: { label: 'Off', size: 'sm', width: 64, disabled: true, name: 'Manual Bass, works with Upper on' },
  play: async ({ canvasElement, args }) => {
    const button = within(canvasElement).getByRole('button', { name: 'Manual Bass, works with Upper on' })
    await expect(button).toHaveAttribute('aria-disabled', 'true')
    await userEvent.click(button)
    await expect(button).toHaveAttribute('aria-pressed', 'false')
    await expect(args.ontoggle).not.toHaveBeenCalled()
  },
}

/** The record lamp lit: the solid record-red face (Looper, Rec / Stop). */
export const Recording: Story = {
  args: { label: 'Rec / Stop', rec: true, on: true },
}

/**
 * A part lamp in the band's row: `cell` fills its container (here the padded canvas), 13px label;
 * the accessible name says which part.
 */
export const PartOn: Story = {
  args: { label: 'On', size: 'cell', on: true, name: 'Right 1 on' },
  parameters: { layout: 'padded' },
  play: async ({ canvasElement }) => {
    const button = within(canvasElement).getByRole('button', { name: 'Right 1 on' })
    await expect(button).toHaveAttribute('aria-pressed', 'true')
  },
}

/** The longest real lamp label, compact: stays on one line (Settings Launchkey). */
export const LongLabel: Story = {
  args: { label: 'Port sends mapped', size: 'sm', on: true },
}

/** Keyboard focus: the focus ring in the `--focus` token (pseudo-states addon). */
export const Focused: Story = {
  args: { label: 'Metronome' },
  parameters: { pseudo: { focusVisible: true } },
}
