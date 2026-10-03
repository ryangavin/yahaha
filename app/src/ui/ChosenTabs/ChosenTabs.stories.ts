import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { expect, fn, userEvent, within } from 'storybook/test'
import ChosenTabs from './ChosenTabs.svelte'
import { displayPageTabs } from './ChosenTabs.fixtures'

/**
 * A short run of choices side by side, the chosen one on a white block: the app's pages,
 * the fader page and the fader layer. The parent owns `chosen`; a click only calls `onchoose`.
 */
const meta = {
  title: 'Primitives/ChosenTabs',
  component: ChosenTabs,
  args: { tabs: displayPageTabs, onchoose: fn(), tipAction: fn() },
  argTypes: {
    tabs: { control: 'object' },
    chosen: { control: 'select', options: [null, ...displayPageTabs.map((tab) => tab.id)] },
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
