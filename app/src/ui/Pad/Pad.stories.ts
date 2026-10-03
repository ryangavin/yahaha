import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { fn } from 'storybook/test'
import Pad from './Pad.svelte'

/**
 * One 68px band pad: caption in its family's hue, index top right, a bar at the bottom. Faces:
 * idle, dark, playing, next, armed and running. `lit` is the flash phase of next and armed.
 */
const meta = {
  title: 'Primitives/Pad',
  component: Pad,
  parameters: { layout: 'centered' },
  args: {
    label: 'Main B',
    index: '10',
    family: 'main',
    state: 'playing',
    lit: true,
    tip: 'section.lamps',
    onpress: fn(),
    tipAction: fn(),
  },
  argTypes: {
    label: { control: 'text' },
    index: { control: 'text' },
    family: { control: 'select', options: ['intro', 'main', 'ending', 'brk', 'fill', 'util', 'start'] },
    state: { control: 'select', options: ['idle', 'dark', 'playing', 'next', 'armed', 'running'] },
    lit: { control: 'boolean' },
    name: { control: 'text' },
    tip: { control: 'text' },
  },
} satisfies Meta<typeof Pad>

export default meta
type Story = StoryObj<typeof meta>

/** Main B on the board, playing: solid green, black caption, the ink bar. */
export const Board: Story = { args: { name: 'Main B, playing' } }

/** An idle section pad: Intro I in gold on the plain face. */
export const Idle: Story = { args: { label: 'Intro I', index: '1', family: 'intro', state: 'idle' } }

/** A pad the style lacks: Intro III, dimmed. */
export const Dark: Story = {
  args: { label: 'Intro III', index: '3', family: 'intro', state: 'dark', name: 'Intro III (not in this style)' },
}

/** Queued: Main C, outlined, NEXT in the hue, the glowing bar. */
export const Next: Story = {
  args: { label: 'Main C', index: '11', state: 'next', name: 'Main C, queued after bar 4 (flashing)' },
}

/** Queued, the flash's off phase. */
export const NextUnlit: Story = {
  args: { label: 'Main C', index: '11', state: 'next', lit: false, name: 'Main C, queued after bar 4 (flashing)' },
}

/** Armed: Ending I, outlined with a glow, ARMED. */
export const Armed: Story = {
  args: { label: 'Ending I', index: '5', family: 'ending', state: 'armed', name: 'Ending I, armed (pulsing)' },
}

/** A utility pad: Sync Start in grey and white. */
export const Utility: Story = { args: { label: 'Sync Start', index: '4', family: 'util', state: 'idle' } }

/** Start / Stop running: solid green. */
export const Running: Story = {
  args: { label: 'Start / Stop', index: '16',family: 'start', state: 'running', name: 'Start / Stop, running (pad 16)' },
}
