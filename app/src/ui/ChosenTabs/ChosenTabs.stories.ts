import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { expect, fn, userEvent, within } from 'storybook/test'
import ChosenTabs from './ChosenTabs.svelte'
import { displayPageTabs, faderPageTabs, fullPageTabs, layerTabs } from './ChosenTabs.fixtures'

/** Every fixture tab's id, for the `chosen` control. */
const ALL_IDS = [...new Set([displayPageTabs, fullPageTabs, faderPageTabs, layerTabs].flat().map((tab) => tab.id))]

/**
 * A short run of choices side by side, the chosen one on a white block: the app's pages,
 * the fader page and the fader layer. The parent owns `chosen`; a click only calls `onchoose`.
 */
const meta = {
  title: 'Primitives/ChosenTabs',
  component: ChosenTabs,
  parameters: { layout: 'centered' },
  args: { tabs: displayPageTabs, onchoose: fn(), tipAction: fn() },
  argTypes: {
    tabs: { control: 'object' },
    chosen: { control: 'select', options: [null, ...ALL_IDS] },
    size: { control: 'inline-radio', options: ['page', 'header', 'compact'] },
    label: { control: 'text' },
  },
} satisfies Meta<typeof ChosenTabs>

export default meta
type Story = StoryObj<typeof meta>

/**
 * The app bar's first run: Stage on the 24px white block, Channel … Harm/Arp grey. Page tabs are
 * buttons with `aria-current`; a click calls through and leaves `chosen` to the parent.
 */
export const Board: Story = {
  args: { tabs: displayPageTabs, chosen: 'stage', size: 'page' },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement)
    const buttons = canvas.getAllByRole('button')
    await expect(buttons.map((b) => b.textContent?.trim())).toEqual(displayPageTabs.map((t) => t.label))
    await expect(canvas.queryByRole('tablist')).toBeNull()
    await expect(canvas.queryByRole('tab')).toBeNull()
    const stage = canvas.getByRole('button', { name: 'Stage' })
    await expect(stage).toHaveAttribute('aria-current', 'page')
    await expect(stage).toHaveAttribute('data-face', 'chosen')
    await expect(stage).toHaveAttribute('data-tip', 'view.stage')
    for (const button of buttons.filter((b) => b !== stage)) {
      await expect(button).not.toHaveAttribute('aria-current')
      await expect(button).toHaveAttribute('data-face', 'off')
    }
    await expect(args.tipAction).toHaveBeenCalledWith(stage, 'view.stage')
    for (const button of buttons) await expect(button).not.toHaveAttribute('tabindex')

    await userEvent.click(canvas.getByRole('button', { name: 'Effects' }))
    await expect(args.onchoose).toHaveBeenLastCalledWith('effects')
    await expect(stage).toHaveAttribute('aria-current', 'page')
    await userEvent.click(stage)
    await expect(args.onchoose).toHaveBeenLastCalledWith('stage')
    canvas.getByRole('button', { name: 'Looper' }).focus()
    await userEvent.keyboard('{Enter}')
    await expect(args.onchoose).toHaveBeenLastCalledWith('looper')
    await userEvent.keyboard(' ')
    await expect(args.onchoose).toHaveBeenLastCalledWith('looper')
    await expect(args.onchoose).toHaveBeenCalledTimes(4)
  },
}

/** The band header's fader layer: a tablist of five, Vol chosen. */
export const Layers: Story = {
  args: { tabs: layerTabs, chosen: 'volume', label: 'Fader layer' },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement)
    const tabs = canvas.getAllByRole('tab')
    await expect(tabs.map((tab) => tab.getAttribute('aria-label') ?? tab.textContent?.trim())).toEqual([
      'Volume',
      'Pan',
      'Reverb send',
      'Chorus send',
      'Delay send',
    ])
    await expect(canvas.getByRole('tab', { name: 'Volume' })).toHaveAttribute('aria-selected', 'true')
  },
}

/**
 * Roving focus with automatic activation: → ← wrap, Home and End jump, each move focuses the tab
 * and calls `onchoose`; `chosen` stays the parent's. The keys never reach the app's window handler.
 */
export const Arrows: Story = {
  args: { tabs: layerTabs, chosen: 'volume', label: 'Fader layer' },
  play: async ({ canvasElement, args }) => {
    const spy = fn()
    window.addEventListener('keydown', spy)
    try {
      const canvas = within(canvasElement)
      const tab = (name: string) => canvas.getByRole('tab', { name })
      const volume = tab('Volume')
      volume.focus()
      await userEvent.keyboard('{ArrowRight}')
      await expect(args.onchoose).toHaveBeenLastCalledWith('pan')
      await expect(tab('Pan')).toHaveFocus()
      await expect(tab('Pan')).toHaveAttribute('tabindex', '0')
      await expect(volume).toHaveAttribute('tabindex', '-1')
      await expect(volume).toHaveAttribute('aria-selected', 'true')
      await userEvent.keyboard('{End}')
      await expect(args.onchoose).toHaveBeenLastCalledWith('delay')
      await expect(tab('Delay send')).toHaveFocus()
      await userEvent.keyboard('{ArrowRight}')
      await expect(args.onchoose).toHaveBeenLastCalledWith('volume')
      await expect(volume).toHaveFocus()
      await userEvent.keyboard('{ArrowLeft}')
      await expect(args.onchoose).toHaveBeenLastCalledWith('delay')
      await userEvent.keyboard('{Home}')
      await expect(args.onchoose).toHaveBeenLastCalledWith('volume')
      await userEvent.keyboard('{Home}')
      await expect(volume).toHaveFocus()
      await expect(args.onchoose).toHaveBeenCalledTimes(5)
      await expect(spy).not.toHaveBeenCalled()
    } finally {
      window.removeEventListener('keydown', spy)
    }
  },
}

/** A disabled tab: "Style" in `--d`, skipped by clicks and arrows. */
export const Disabled: Story = {
  args: {
    tabs: [faderPageTabs[0], { ...faderPageTabs[1], disabled: true }],
    chosen: 'panel',
    label: 'Fader page (master button)',
  },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement)
    const panel = canvas.getByRole('tab', { name: 'Panel' })
    const style = canvas.getByRole('tab', { name: 'Style' })
    await expect(style).toHaveAttribute('aria-disabled', 'true')
    await expect(style).toHaveAttribute('data-face', 'disabled')
    await expect(style).toHaveAttribute('data-contrast', 'dim')
    await expect(style).toHaveAttribute('tabindex', '-1')
    await expect(panel).not.toHaveAttribute('aria-disabled')
    await expect(panel).toHaveAttribute('data-face', 'chosen')
    await userEvent.click(style)
    await expect(args.onchoose).not.toHaveBeenCalled()
    panel.focus()
    await userEvent.keyboard('{ArrowRight}')
    await expect(panel).toHaveFocus()
    await expect(args.onchoose).not.toHaveBeenCalled()
  },
}
