import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { fn } from 'storybook/test'
import { faderPageTabs, layerTabs } from '../ChosenTabs/ChosenTabs.fixtures'
import FaderBank from './FaderBank.svelte'
import { functionLamps, panelStrips, partLamps, reverbStrips } from './FaderBank.fixtures'

/**
 * The band's Faders section: the page and layer tabs, nine strips (Fader, name, marks), the
 * caption row, and the lamp row of part lamps, Launchkey functions and the Panel page button.
 * Every change is a callback carrying the strip's or lamp's id.
 */
const meta = {
  title: 'Components/FaderBank',
  component: FaderBank,
  parameters: { layout: 'centered' },
  args: {
    strips: panelStrips,
    pageTabs: faderPageTabs,
    page: 'panel',
    layerTabs,
    layer: 'volume',
    partLamps,
    functionLamps,
    tipAction: fn(),
    onchoosePage: fn(),
    onchooseLayer: fn(),
    onlevel: fn(),
    onopen: fn(),
    onlamp: fn(),
    onlamplong: fn(),
    onlamprelease: fn(),
    onpagebutton: fn(),
  },
  argTypes: {
    strips: { control: 'object' },
    pageTabs: { control: 'object' },
    page: { control: 'select', options: faderPageTabs.map((tab) => tab.id) },
    layerTabs: { control: 'object' },
    layer: { control: 'select', options: layerTabs.map((tab) => tab.id) },
    partLamps: { control: 'object' },
    functionLamps: { control: 'object' },
  },
} satisfies Meta<typeof FaderBank>

export default meta
type Story = StoryObj<typeof meta>

/** The board: Panel page, Vol layer, Right 2 away, Right 3 off and missing, faders 7 and 8 parked. */
export const Board: Story = {}

/** The Reverb layer: "Faders · Reverb", strips 1–4 white with the layer word, no meters. */
export const ReverbLayer: Story = {
  args: { layer: 'reverb', strips: reverbStrips },
}
