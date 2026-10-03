import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { fn } from 'storybook/test'
import Knob from './Knob.svelte'

/**
 * One band knob: the plain name, the value in the accent, a bare 270° ring arc with a tip dot,
 * and the Genos code beneath. ↑ / ↓ and the wheel call `onstep`; a click calls `onpress`.
 */
const meta = {
  title: 'Primitives/Knob',
  component: Knob,
  parameters: { layout: 'centered' },
  args: {
    label: 'Dynamics',
    code: 'DynCtrl',
    value: '127',
    unit: '',
    fraction: 1,
    unused: false,
    tip: 'knobs.knob',
    onpress: fn(),
    onstep: fn(),
    tipAction: fn(),
  },
  argTypes: {
    label: { control: 'text' },
    code: { control: 'text' },
    value: { control: 'text' },
    unit: { control: 'text' },
    fraction: { control: { type: 'range', min: 0, max: 1, step: 0.01 } },
    unused: { control: 'boolean' },
    name: { control: 'text' },
    tip: { control: 'text' },
  },
} satisfies Meta<typeof Knob>

export default meta
type Story = StoryObj<typeof meta>

/** Knob 1 on the board: Dynamics at 127, the arc full round. */
export const Board: Story = {}

/** Part way round: Retrig rate at 1/8. */
export const Partial: Story = {
  args: { label: 'Retrig rate', code: 'RtgRate', value: '1/8', fraction: 0.4 },
}

/** At zero, with a unit: Swing 0%. Only the tip dot is lit. */
export const Unit: Story = {
  args: { label: 'Swing', code: 'Swing', value: '0', unit: '%', fraction: 0 },
}

/** Nothing mapped: the dim "---" and an empty ring. */
export const Unused: Story = {
  args: { label: '---', code: '', value: '', fraction: 0, unused: true },
}
