import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { fn } from 'storybook/test'
import TransportColumn from './TransportColumn.svelte'
import { transportBoard } from './TransportColumn.fixtures'

/**
 * The band's 88px right column: Transport (Start / Stop, Stop, Reset and Fade, the Fills pair)
 * and Tempo (Tempo + and −, Style tempo). Every press is a callback; Tempo ± report the hold.
 */
const meta = {
  title: 'Components/TransportColumn',
  component: TransportColumn,
  parameters: { layout: 'centered' },
  args: {
    ...transportBoard,
    tipAction: fn(),
    onstartstop: fn(),
    onstop: fn(),
    onstoplong: fn(),
    onreset: fn(),
    onfade: fn(),
    onfillup: fn(),
    onfilldown: fn(),
    ontempoup: fn(),
    ontempodown: fn(),
    onstyletempo: fn(),
  },
  argTypes: {
    running: { control: 'boolean' },
    fading: { control: 'boolean' },
  },
} satisfies Meta<typeof TransportColumn>

export default meta
type Story = StoryObj<typeof meta>

/** The board: running, so Start / Stop carries the green bar. */
export const Board: Story = {}

/** Stopped: the bar's room kept, nothing drawn. */
export const Stopped: Story = { args: { running: false } }

/** Fading: Fade on the lamp face. */
export const Fading: Story = { args: { fading: true } }
