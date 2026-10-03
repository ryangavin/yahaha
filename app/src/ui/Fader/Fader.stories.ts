import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { fn } from 'storybook/test'
import Fader from './Fader.svelte'

const NONE = 'none'

/**
 * One strip's fader button: the level value in the strip's hue, two meter bars with a peak line,
 * and the 3px set-level bracket with its cap tick, which outranks the meter. A dashed ghost and ↕
 * show a hardware fader that is away from the level. Controlled: a drag or ↑ / ↓ asks for a level
 * through `onlevel`.
 */
const meta = {
  title: 'Primitives/Fader',
  component: Fader,
  parameters: { layout: 'centered' },
  args: {
    name: 'Right 1 · Stage Grand, level 90',
    value: '90',
    level: 90,
    meter: 0.62,
    meter2: 0.58,
    peak: 0.7,
    kind: 'part',
    hue: 'r1',
    layered: false,
    width: 66,
    tip: 'mixer.panel.right1',
    onlevel: fn(),
    tipAction: fn(),
  },
  argTypes: {
    name: { control: 'text' },
    value: { control: 'text' },
    level: { control: { type: 'range', min: 0, max: 127, step: 1 } },
    meter: { control: { type: 'range', min: 0, max: 1, step: 0.01 } },
    meter2: { control: { type: 'range', min: 0, max: 1, step: 0.01 } },
    peak: { control: { type: 'range', min: 0, max: 1, step: 0.01 } },
    away: {
      control: 'select',
      options: [NONE, 0, 32, 50, 64, 100, 127],
      mapping: { [NONE]: undefined },
    },
    kind: { control: 'select', options: ['part', 'group', 'master', 'off', 'parked'] },
    hue: { control: 'select', options: ['r1', 'r2', 'r3', 'l', 'a', 't2', 't'] },
    layered: { control: 'boolean' },
    width: { control: 'number' },
    tip: { control: 'text' },
  },
} satisfies Meta<typeof Fader>

export default meta
type Story = StoryObj<typeof meta>

/** Right 1 on the board: level 90 in blue, meters at 0.62, the glowing blue bracket. */
export const Board: Story = {}

/** Right 2, its hardware fader away at 50: the dashed ghost line and ↕ (soft takeover). */
export const Away: Story = {
  args: {
    name: 'Right 2 · Silk Strings, level 72, hardware fader away (soft takeover)',
    value: '72',
    level: 72,
    meter: 0.45,
    meter2: 0.42,
    peak: 0.55,
    hue: 'r2',
    away: 50,
  },
}

/** Right 3, switched off: the value dimmed, no meter, the bracket and cap at 35%. */
export const Off: Story = {
  args: {
    name: 'Right 3 · Brass Section, level 64',
    value: '64',
    level: 64,
    meter: 0,
    meter2: 0,
    peak: 0,
    kind: 'off',
    hue: 'r3',
  },
}

/** The Style volume: a group fader in the accent. */
export const Group: Story = {
  args: { name: 'Style volume, level 100', value: '100', level: 100, meter: 0.7, meter2: 0.65, peak: 0.78, kind: 'group', hue: 'a' },
}

/** Master, in white. */
export const Master: Story = {
  args: { name: 'Master, level 100', value: '100', level: 100, meter: 0.72, meter2: 0.67, peak: 0.8, kind: 'master', hue: 't' },
}

/** An unused fader: a dashed groove, nothing else. */
export const Parked: Story = {
  args: { name: 'Fader 7 unused', value: '', level: 0, meter: 0, meter2: 0, peak: 0, kind: 'parked', hue: 't' },
}

/** The Reverb layer: no meters, the white bracket, the value carrying the layer word. */
export const Layered: Story = {
  args: { name: 'Right 1 · Stage Grand, Rev 40', value: 'Rev 40', level: 40, layered: true },
}
