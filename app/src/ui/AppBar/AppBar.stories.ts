import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { fn } from 'storybook/test'
import AppBar from './AppBar.svelte'
import { boardAppBar } from './AppBar.fixtures'
import { displayPageTabs, fullPageTabs } from '../ChosenTabs/ChosenTabs.fixtures'

const PAGE_IDS = [...displayPageTabs, ...fullPageTabs].map((tab) => tab.id)

/**
 * The 36px bar on top of every page: "yahaha", the page nav (display pages, a hairline, full
 * pages), then the fixed 196px right area with the Launchkey status and the audio health. A white
 * 1px line underneath. The parent owns `chosen`; a click only calls `onchoose`.
 */
const meta = {
  title: 'Components/AppBar',
  component: AppBar,
  parameters: { layout: 'centered' },
  args: { ...boardAppBar, onchoose: fn(), tipAction: fn(), onhealth: fn() },
  argTypes: {
    width: { control: { type: 'number', min: 0, step: 1 } },
    launchkey: { control: 'boolean', table: { category: 'StatusDot' } },
    displayTabs: { control: 'object', table: { category: 'ChosenTabs' } },
    fullTabs: { control: 'object', table: { category: 'ChosenTabs' } },
    chosen: { control: 'select', options: [null, ...PAGE_IDS], table: { category: 'ChosenTabs' } },
    onchoose: { table: { category: 'ChosenTabs' } },
    tipAction: { table: { category: 'ChosenTabs' } },
    failedPart: { control: 'select', options: [null, 0, 1, 2, 3], table: { category: 'HealthSlot' } },
    synthOn: { control: 'boolean', table: { category: 'HealthSlot' } },
    dropouts: { control: { type: 'number', min: 0, step: 1 }, table: { category: 'HealthSlot' } },
    bufferFrames: { control: 'select', options: [null, 64, 128, 256, 512, 1024], table: { category: 'HealthSlot' } },
    cpu: { control: { type: 'range', min: 0, max: 1.2, step: 0.01 }, table: { category: 'HealthSlot' } },
    onhealth: { table: { category: 'HealthSlot' } },
  },
} satisfies Meta<typeof AppBar>

export default meta
type Story = StoryObj<typeof meta>

/** The Stage board: Stage chosen, the Launchkey connected, "Audio" calm at the right edge. */
export const Board: Story = {}

/** Library chosen: the block moves past the hairline. */
export const Library: Story = {
  args: { chosen: 'library' },
}

/** The Launchkey unplugged: a hollow grey ring before the word. */
export const NoLaunchkey: Story = {
  args: { launchkey: false },
}

/** Trouble: Right 3's plugin failed, "R3 failed" in Ending red; a click opens its Channel page. */
export const Trouble: Story = {
  args: { failedPart: 2 },
}
