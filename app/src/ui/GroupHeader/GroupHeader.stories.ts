import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { createRawSnippet, mount, unmount } from 'svelte'
import { expect, fn, userEvent, within } from 'storybook/test'
import AccentBlock from '../AccentBlock/AccentBlock.svelte'
import ChosenTabs from '../ChosenTabs/ChosenTabs.svelte'
import { faderPageTabs, layerTabs } from '../ChosenTabs/ChosenTabs.fixtures'
import GroupHeader from './GroupHeader.svelte'

type Content = 'faders' | 'knobs' | 'pads' | 'none'

/** The story's args: GroupHeader's own props plus the child args mapped into `children`. */
type Args = {
  title: string
  detail?: string
  count?: { label: string; value: string }
  level?: 2 | 3 | 4
  id?: string
  width?: number
  content: Content
  pageChosen: string
  onchoosePage: (id: string) => void
  layerChosen: string
  onchooseLayer: (id: string) => void
  tipAction: (node: HTMLElement, key: string) => void
  knobPageLabel: string
  padPageName: string
}

const HUES = [
  ['Intro', 'intro'],
  ['Main', 'main'],
  ['Ending', 'ending'],
  ['Break', 'brk'],
  ['Fill', 'fill'],
] as const

const FADERS =
  '<span style="display: contents">' +
  '<span data-mount="page" style="display: contents"></span>' +
  '<span aria-hidden="true" style="flex: none; align-self: flex-end; margin-bottom: var(--separator-lift); width: var(--line-width); height: var(--separator-length); background: var(--line)"></span>' +
  '<span style="display: flex; align-items: baseline">' +
  '<span style="margin-right: var(--space-4); font-size: var(--text-14); font-weight: var(--weight-regular); color: var(--m)">Layer</span>' +
  '<span data-mount="layer" style="display: contents"></span>' +
  '</span></span>'

const KNOBS = '<span data-mount="block" style="display: contents"></span>'

const PADS =
  '<span style="display: contents">' +
  '<span data-text="page" style="font-size: var(--text-14); font-weight: var(--weight-regular); color: var(--t)"></span>' +
  '<span style="margin-left: var(--space-4); display: flex; align-items: baseline; gap: var(--space-12); font-size: var(--text-12); font-weight: var(--weight-regular)">' +
  HUES.map(
    ([word, hue]) =>
      `<span style="display: flex; align-items: baseline; gap: var(--space-6); color: var(--${hue})">` +
      `<span aria-hidden="true" style="align-self: center; width: var(--space-10); height: var(--space-2); border-radius: var(--line-width); background: var(--${hue})"></span>` +
      `${word}</span>`,
  ).join('') +
  '</span></span>'

/** The `children` snippet for a story's `content`, mounting the real ChosenTabs and AccentBlock. */
function children(args: Args) {
  if (args.content === 'none') return undefined
  const html = { faders: FADERS, knobs: KNOBS, pads: PADS }[args.content]
  return createRawSnippet(() => ({
    render: () => html,
    setup: (root: Element) => {
      const host = (name: string) => root.querySelector(`[data-mount="${name}"]`) ?? root
      const mounted: Record<string, unknown>[] = []
      if (args.content === 'faders') {
        mounted.push(
          mount(ChosenTabs, {
            target: host('page'),
            props: {
              size: 'header',
              label: 'Fader page (master button)',
              tabs: faderPageTabs,
              chosen: args.pageChosen,
              onchoose: args.onchoosePage,
              tipAction: args.tipAction,
            },
          }),
          mount(ChosenTabs, {
            target: host('layer'),
            props: {
              size: 'header',
              label: 'Fader layer',
              tone: 'secondary',
              tabs: layerTabs,
              chosen: args.layerChosen,
              onchoose: args.onchooseLayer,
              tipAction: args.tipAction,
            },
          }),
        )
      } else if (args.content === 'knobs') {
        mounted.push(mount(AccentBlock, { target: host('block'), props: { label: args.knobPageLabel, size: 'knob' } }))
      } else {
        const page = root.querySelector('[data-text="page"]')
        if (page) page.textContent = args.padPageName
      }
      return () => {
        for (const instance of mounted) void unmount(instance)
      }
    },
  }))
}

/**
 * The hairline row that names a group of controls and holds its page tabs or page counter.
 * The stories mount the real children (ChosenTabs, AccentBlock) into the `children` snippet;
 * their args are grouped by child in the Controls panel.
 */
const meta: Meta<Args> = {
  title: 'Primitives/GroupHeader',
  component: GroupHeader,
  render: (args: Args) => ({
    Component: GroupHeader,
    props: {
      title: args.title,
      detail: args.detail,
      count: args.count,
      level: args.level,
      id: args.id,
      width: args.width,
      children: children(args),
      // The story's args map onto GroupHeader's props here; Storybook types `props` as the args.
    } as unknown as Args,
  }),
  args: {
    title: 'Faders',
    content: 'faders',
    pageChosen: 'panel',
    onchoosePage: fn(),
    layerChosen: 'volume',
    onchooseLayer: fn(),
    tipAction: fn(),
    knobPageLabel: 'Style',
    padPageName: 'Sections',
  },
  argTypes: {
    title: { control: 'text' },
    detail: { control: 'text' },
    count: { control: 'object' },
    level: { control: 'inline-radio', options: [2, 3, 4] },
    id: { control: 'text' },
    width: { control: 'number' },
    content: {
      control: 'select',
      options: ['faders', 'knobs', 'pads', 'none'],
      table: { category: 'children' },
    },
    pageChosen: {
      control: 'select',
      options: ['panel', 'style'],
      table: { category: 'ChosenTabs · fader page' },
    },
    onchoosePage: { table: { category: 'ChosenTabs · fader page' } },
    tipAction: { table: { category: 'ChosenTabs · fader page' } },
    layerChosen: {
      control: 'select',
      options: ['volume', 'pan', 'reverb', 'chorus', 'delay'],
      table: { category: 'ChosenTabs · layer' },
    },
    onchooseLayer: { table: { category: 'ChosenTabs · layer' } },
    knobPageLabel: { control: 'text', table: { category: 'AccentBlock' } },
    padPageName: { control: 'text', table: { category: 'Pads content' } },
  },
}

export default meta
type Story = StoryObj<Args>

/** The Faders header: title, Panel chosen, the separator, "Layer" and Vol chosen, over the hairline. */
export const Board: Story = {
  args: { title: 'Faders', width: 654, content: 'faders' },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement)
    await expect(canvas.getByRole('heading', { level: 2, name: 'Faders' })).toBeInTheDocument()
    await expect(await canvas.findByRole('tablist', { name: 'Fader page (master button)' })).toBeInTheDocument()
    await expect(canvas.getByRole('tablist', { name: 'Fader layer' })).toBeInTheDocument()
    await expect(canvas.getByRole('tab', { name: 'Panel' })).toHaveAttribute('data-tip', 'mixer.page')
    await expect(canvas.queryByText(/Page|Bank/)).toBeNull()
    await userEvent.click(canvas.getByRole('tab', { name: 'Style' }))
    await expect(args.onchoosePage).toHaveBeenCalledTimes(1)
    await expect(args.onchoosePage).toHaveBeenCalledWith('style')
    await userEvent.click(canvas.getByRole('tab', { name: 'Pan' }))
    await expect(args.onchooseLayer).toHaveBeenCalledTimes(1)
    await expect(args.onchooseLayer).toHaveBeenCalledWith('pan')
  },
}

/** Knobs, the violet "Style" page block, and "Page 1/6" at the right end. */
export const Knobs: Story = {
  args: { title: 'Knobs', width: 626, count: { label: 'Page', value: '1/6' }, content: 'knobs' },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement)
    await expect(canvas.getByRole('heading', { name: 'Knobs' })).toBeInTheDocument()
    const block = await canvas.findByText('Style')
    await expect(block).toHaveAttribute('data-face', 'accent')
    await expect(canvas.getByText('1/6').parentElement?.textContent).toBe('Page 1/6')
  },
}

/** Pads, the "Sections" page name, the five-hue section legend, and "Bank 1/5" at the right end. */
export const Pads: Story = {
  args: { title: 'Pads', width: 626, count: { label: 'Bank', value: '1/5' }, content: 'pads' },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement)
    await expect(canvas.getByRole('heading', { name: 'Pads' })).toBeInTheDocument()
    await expect(await canvas.findByText('Sections')).toBeInTheDocument()
    const text = canvasElement.textContent ?? ''
    const order = HUES.map(([word]) => text.indexOf(word))
    await expect(order.every((at) => at >= 0)).toBe(true)
    await expect([...order].sort((a, b) => a - b)).toEqual(order)
    await expect(canvas.getByText('1/5').parentElement?.textContent).toBe('Bank 1/5')
  },
}
