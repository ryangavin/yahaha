import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { fn } from 'storybook/test'
import StyleLine from './StyleLine.svelte'
import { styleLineBoard, styleLineQueued } from './StyleLine.fixtures'

/**
 * The display's top line: ◀ the style in its accent block ▶, its category and time signature,
 * a queued style, One Touch 1-4 and the band's sends. 814 wide, the display's content column.
 */
const meta = {
  title: 'Components/StyleLine',
  component: StyleLine,
  parameters: { layout: 'centered' },
  args: {
    ...styleLineBoard,
    tipAction: fn(),
    onprev: fn(),
    onnext: fn(),
    onbrowse: fn(),
    ononetouch: fn(),
    onsends: fn(),
  },
  argTypes: {
    styleName: { control: 'text', table: { category: 'AccentBlock' } },
    category: { control: 'text' },
    timeSignature: { control: 'text' },
    queued: { control: 'text', table: { category: 'WaitingChip' } },
    oneTouch: { control: { type: 'inline-radio' }, options: [0, 1, 2, 3, 4], table: { category: 'OneTouchPicker' } },
    reverb: { control: { type: 'range', min: 0, max: 127, step: 1 }, table: { category: 'SendReadout' } },
    chorus: { control: { type: 'range', min: 0, max: 127, step: 1 }, table: { category: 'SendReadout' } },
    delay: { control: { type: 'range', min: 0, max: 127, step: 1 }, table: { category: 'SendReadout' } },
  },
} satisfies Meta<typeof StyleLine>

export default meta
type Story = StoryObj<typeof meta>

/** The dark board: Sunday Drive Pop, Pop · 4/4, One Touch 2 applied, Reverb 40 Chorus 12 Delay 0. */
export const Board: Story = {}

/** Coastal Highway waits for the bar line, outlined in the accent after the time signature. */
export const Queued: Story = { args: { ...styleLineQueued } }

/** No One Touch applied: all four on the plain face. */
export const NoneApplied: Story = { args: { oneTouch: 0 } }
