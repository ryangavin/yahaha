import type { Meta, StoryObj } from '@storybook/svelte-vite'
import NowPlaying from './NowPlaying.svelte'
import { nowPlayingBoard, nowPlayingStopped } from './NowPlaying.fixtures'

const HUES = ['intro', 'main', 'ending', 'brk', 'fill']

/**
 * The display's middle, in glance order: the chord, the playing section (44px) with the next one
 * waiting, then the tempo and the Running light. 814 × 162.
 */
const meta = {
  title: 'Components/NowPlaying',
  component: NowPlaying,
  parameters: { layout: 'centered' },
  args: { ...nowPlayingBoard },
  argTypes: {
    chord: { control: 'object', table: { category: 'ChordReadout' } },
    playing: { control: 'text', table: { category: 'SectionName' } },
    hue: { control: 'select', options: HUES, table: { category: 'SectionName' } },
    next: { control: 'text', table: { category: 'WaitingChip' } },
    nextHue: { control: 'select', options: HUES, table: { category: 'WaitingChip' } },
    bpm: { control: { type: 'number', min: 5, max: 500, step: 1 }, table: { category: 'TempoReadout' } },
    running: { control: 'boolean', table: { category: 'StatusDot' } },
  },
} satisfies Meta<typeof NowPlaying>

export default meta
type Story = StoryObj<typeof meta>

/** The dark board: Am7, Main B → Main C, 104 BPM, Running. */
export const Board: Story = {}

/** Stopped on Main A, the last chord held, nothing next. */
export const Stopped: Story = { args: { ...nowPlayingStopped } }
