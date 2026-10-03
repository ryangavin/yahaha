import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { fn } from 'storybook/test'
import SectionRow from './SectionRow.svelte'
import { sectionRowBoard } from './SectionRow.fixtures'

/**
 * The toolbar under the app bar: Accomp, the count row, then Metronome with its settings caret,
 * Unison, Panic and help mode's ?. 1392 wide, the screen inside its padding.
 */
const meta = {
  title: 'Components/SectionRow',
  component: SectionRow,
  parameters: { layout: 'centered' },
  args: {
    ...sectionRowBoard,
    tipAction: fn(),
    onaccomp: fn(),
    onmetronome: fn(),
    onmetronomesettings: fn(),
    onunison: fn(),
    onpanic: fn(),
    onhelp: fn(),
  },
  argTypes: {
    count: { control: 'object', table: { category: 'CountRow' } },
    accomp: { control: 'boolean', table: { category: 'LampButton' } },
    metronome: { control: 'boolean', table: { category: 'LampButton' } },
    unison: { control: 'boolean', table: { category: 'LampButton' } },
    metronomeOpen: { control: 'boolean', table: { category: 'Button' } },
    metronomeControls: { control: 'text', table: { category: 'Button' } },
    help: { control: 'boolean', table: { category: 'Button' } },
  },
} satisfies Meta<typeof SectionRow>

export default meta
type Story = StoryObj<typeof meta>

/** The dark board: Accomp lit, beat 3 of bar 3/4 in Main B → Main C, every helper off. */
export const Board: Story = {}

/** Metronome and Unison on, the metronome's settings open, help mode lit. */
export const AllOn: Story = {
  args: { accomp: true, metronome: true, metronomeOpen: true, unison: true, help: true },
}
