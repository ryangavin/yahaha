/**
 * Every story in the UI library is a test. This file finds each `*.stories.ts` under src/ui,
 * composes its stories with Storybook's portable stories and, in dark and in light:
 *  - renders the story and runs its `play` function;
 *  - runs the a11y addon's axe check, which must pass (axiom 8);
 *  - checks the controls (axioms 3 and 7): every prop of a primitive is a control, no control is
 *    disabled, every arg a story sets is a control, and every callback is an action (`fn()`);
 *  - checks the title follows the taxonomy (axiom 11).
 * A new component's stories are covered with no new test file.
 */
import * as a11yAnnotations from '@storybook/addon-a11y/preview'
import { composeStories, setProjectAnnotations, type Meta } from '@storybook/svelte'
import type { Component } from 'svelte'
import { afterEach, beforeAll, describe, expect, it } from 'vitest'
import preview from '../../.storybook/preview'

type StoryModule = Parameters<typeof composeStories>[0] & { default: Meta<Component> }
/** The props Storybook's Svelte docgen attaches to a component (vite.config.ts adds the plugin). */
type DocProp = { name: string; type?: { kind?: string; text?: string } }
type Docgen = { data?: DocProp[] }
type ArgType = { control?: unknown; table?: { disable?: boolean }; type?: { name?: string } }
/** The parts of a composed story this test reads. */
type Composed = {
  args: Record<string, unknown>
  argTypes: Record<string, ArgType>
  parameters: Record<string, unknown>
  globals: Record<string, unknown>
  reporting: { reports: { type: string; result: unknown }[] }
  run: (context: { canvasElement: HTMLElement; globals: Record<string, unknown> }) => Promise<void>
}

// jsdom has no canvas; axe probes one for its colour checks and jsdom logs each probe. Colour
// contrast needs real rendering anyway, so Storybook's a11y panel checks it, not this test.
HTMLCanvasElement.prototype.getContext = (() => null) as typeof HTMLCanvasElement.prototype.getContext

const annotations = setProjectAnnotations([a11yAnnotations, preview])
beforeAll(annotations.beforeAll)

const modules = import.meta.glob<StoryModule>('./**/*.stories.ts', { eager: true })
const THEMES = ['dark', 'light'] as const

let canvas: HTMLElement | undefined
afterEach(() => {
  canvas?.remove()
  canvas = undefined
  delete document.documentElement.dataset.theme
})

function isAction(value: unknown): boolean {
  return typeof value === 'function' && 'mock' in value
}

/** A control is off when it's missing, `false` (Storybook stores it as `{ disable: true }`) or hidden. */
function controlOn(argType: ArgType | undefined): boolean {
  return Boolean(argType?.control) && !controlDisabled(argType)
}

function controlDisabled(argType: ArgType | undefined): boolean {
  const control = argType?.control as { disable?: boolean } | false | null | undefined
  return control === false || control?.disable === true || argType?.table?.disable === true
}

/** A snippet (child content), a callback (an action), or a value (a control). */
function propKind(prop: DocProp): 'snippet' | 'callback' | 'value' {
  if (/^Snippet\b/.test(prop.type?.text ?? '')) return 'snippet'
  return prop.type?.kind === 'function' ? 'callback' : 'value'
}

it('finds the stories', () => {
  expect(Object.keys(modules).length).toBeGreaterThan(0)
})

for (const [path, mod] of Object.entries(modules)) {
  const meta = mod.default
  const title = meta.title ?? path
  const docgen = (meta.component as { __docgen?: Docgen } | undefined)?.__docgen
  const props = docgen?.data ?? []
  const kinds = new Map(props.map((prop) => [prop.name, propKind(prop)]))

  describe(title, () => {
    it('follows the taxonomy: Primitives/, Components/ or Pages/', () => {
      expect(title).toMatch(/^(Primitives|Components|Pages)\/[A-Z]\w*$/)
    })

    it('has docgen for its component, so its props are known', () => {
      expect(props.length, `${path}: no props found; is meta.component set?`).toBeGreaterThan(0)
    })

    for (const theme of THEMES) {
      const stories = Object.entries(composeStories(mod)) as [string, Composed][]
      for (const [name, story] of stories) {
        it(`${name} (${theme})`, async () => {
          const { argTypes, args } = story

          // Axiom 3: controls for everything the story shows. Snippets (child content, passed with
          // createRawSnippet) are content, not controls; callbacks are actions (below).
          const kindOfArg = (key: string, value: unknown) =>
            kinds.get(key) ?? (typeof value === 'function' ? 'callback' : 'value')
          const missing: string[] = []
          if (title.startsWith('Primitives/')) {
            for (const prop of props) {
              if (kinds.get(prop.name) === 'value' && !controlOn(argTypes[prop.name])) missing.push(prop.name)
            }
          }
          for (const [key, value] of Object.entries(args)) {
            if (kindOfArg(key, value) === 'value' && !controlOn(argTypes[key])) missing.push(key)
          }
          expect([...new Set(missing)], 'props without a working control').toEqual([])
          const disabled = Object.keys(argTypes).filter((key) => controlDisabled(argTypes[key]))
          expect(disabled, 'disabled or hidden controls').toEqual([])
          const { include, exclude } = (story.parameters.controls ?? {}) as { include?: unknown; exclude?: unknown }
          expect({ include, exclude }, 'controls.include/exclude hide props').toEqual({})

          // Axiom 7: every callback is an action.
          for (const prop of props) {
            if (kinds.get(prop.name) === 'callback') {
              expect(isAction(args[prop.name]), `${prop.name}: callback is not an fn() action`).toBe(true)
            }
          }
          for (const [key, value] of Object.entries(args)) {
            if (kindOfArg(key, value) === 'callback') {
              expect(isAction(value), `${key}: callback is not an fn() action`).toBe(true)
            }
          }

          // Render, play, and the a11y check (axiom 8).
          canvas = document.createElement('div')
          canvas.id = 'storybook-root'
          document.body.append(canvas)
          await story.run({ canvasElement: canvas, globals: { ...story.globals, theme } })
          expect(document.documentElement.dataset.theme).toBe(theme)
          expect(canvas.childElementCount, 'the story rendered nothing').toBeGreaterThan(0)
          const a11y = story.reporting.reports.filter((report) => report.type === 'a11y')
          expect(a11y.length, 'the a11y check did not run').toBeGreaterThan(0)
          for (const report of a11y) {
            const result = report.result as { violations?: { id: string; help: string }[]; error?: unknown }
            expect(result.error, 'the a11y check failed to run').toBeUndefined()
            expect(result.violations?.map((v) => `${v.id}: ${v.help}`) ?? []).toEqual([])
          }
        })
      }
    }
  })
}
