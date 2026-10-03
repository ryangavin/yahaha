import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { fn } from 'storybook/test'
import Display from './Display.svelte'
import { displayBoard, displayStopped } from './Display.fixtures'

/**
 * The Stage's display, 1392 × 300 on the ground: the art at the right, the style line, now
 * playing and the sound row at the left.
 */
const meta = {
  title: 'Components/Display',
  component: Display,
  parameters: { layout: 'centered' },
  args: {
    ...displayBoard,
    tipAction: fn(),
    onprev: fn(),
    onnext: fn(),
    onbrowse: fn(),
    ononetouch: fn(),
    onsends: fn(),
    onrack: fn(),
    onpart: fn(),
    onsound: fn(),
  },
  argTypes: {
    styleLine: { control: 'object', table: { category: 'StyleLine' } },
    nowPlaying: { control: 'object', table: { category: 'NowPlaying' } },
    soundRow: { control: 'object', table: { category: 'SoundRow' } },
  },
} satisfies Meta<typeof Display>

export default meta
type Story = StoryObj<typeof meta>

/** The dark board's display. */
export const Board: Story = {}

/** Stopped with the chord held, Coastal Highway queued, a clean rack. */
export const Stopped: Story = { args: { ...displayStopped } }
