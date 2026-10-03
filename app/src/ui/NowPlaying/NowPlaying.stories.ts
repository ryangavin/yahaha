import type { Meta, StoryObj } from '@storybook/svelte-vite'
import NowPlaying from './NowPlaying.svelte'
import { nowPlayingBoard, nowPlayingFill, nowPlayingStopped } from './NowPlaying.fixtures'

const HUES = ['intro', 'main', 'ending', 'brk', 'fill']

/**
 * The display's middle, in glance order: the chord, the playing section (44px) with the next one
 * and when the fill lands, the bar and beat, then the tempo and the Running light. 814 × 162.
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
    next: { control: 'text' },
    fill: { control: 'text' },
    bar: { control: { type: 'number', min: 1, step: 1 }, table: { category: 'BarBeat' } },
    bars: { control: { type: 'number', min: 1, max: 16, step: 1 }, table: { category: 'BarBeat' } },
    beat: { control: { type: 'range', min: 0, max: 12, step: 1 }, table: { category: 'BarBeat' } },
    beats: { control: { type: 'number', min: 1, max: 12, step: 1 }, table: { category: 'BarBeat' } },
    progress: { control: { type: 'range', min: 0, max: 1, step: 0.01 }, table: { category: 'BarBeat' } },
    bpm: { control: { type: 'number', min: 5, max: 500, step: 1 }, table: { category: 'TempoReadout' } },
    running: { control: 'boolean', table: { category: 'StatusDot' } },
  },
} satisfies Meta<typeof NowPlaying>

export default meta
type Story = StoryObj<typeof meta>

/** The dark board: Am7, Main B, next Main C, fill lands after bar 4, bar 3/4 on beat 3, 104 BPM, Running. */
export const Board: Story = {}

/** Stopped on Main A, the last chord held, nothing next, every beat a ring. */
export const Stopped: Story = { args: { ...nowPlayingStopped } }

/** A one-bar fill in 3/4 on its last beat, Ending II next. */
export const FillToEnding: Story = { args: { ...nowPlayingFill } }
