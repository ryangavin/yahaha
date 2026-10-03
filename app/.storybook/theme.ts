/**
 * The theme toolbar, scoped to stories: nothing is set on the document, so Storybook's own UI and
 * docs pages keep their look. `currentTheme`/`onTheme` serve docs pages without stories (the
 * Foundations page), which never run the theme decorator; they follow the preview channel's globals.
 */
import { GLOBALS_UPDATED, SET_GLOBALS, UPDATE_GLOBALS } from 'storybook/internal/core-events'
import { addons } from 'storybook/preview-api'

export const THEMES = { dark: 'dark', light: 'light' } as const
export type Theme = keyof typeof THEMES

export function asTheme(value: unknown): Theme {
  return typeof value === 'string' && value in THEMES ? (value as Theme) : 'dark'
}

let current: Theme = 'dark'
const listeners = new Set<(theme: Theme) => void>()

if (addons.hasChannel()) {
  const channel = addons.getChannel()
  for (const event of [SET_GLOBALS, GLOBALS_UPDATED]) {
    channel.on(event, (payload: { globals?: { theme?: unknown } }) => {
      current = asTheme(payload.globals?.theme)
      for (const listener of listeners) listener(current)
    })
  }
}

/** The theme the last story was shown in, so a ground the user picked can be told apart. */
let shown: Theme | undefined

/**
 * The backgrounds tool's ground follows the theme: when the theme changes, the ground switches to
 * the theme's own (its option is named after the theme) unless the user picked another, that is,
 * unless the ground is set and differs from the theme shown before.
 */
export function followGround(theme: Theme, backgrounds: unknown) {
  const previous = shown
  shown = theme
  const ground = (backgrounds as { value?: unknown } | undefined)?.value
  if (ground === theme || (ground !== undefined && ground !== previous)) return
  if (!addons.hasChannel()) return
  const value = { ...(backgrounds as object | undefined), value: theme }
  addons.getChannel().emit(UPDATE_GLOBALS, { globals: { backgrounds: value } })
}

export function currentTheme(): Theme {
  return current
}

/** Calls `listener` on every theme change; returns the unsubscribe. */
export function onTheme(listener: (theme: Theme) => void): () => void {
  listeners.add(listener)
  return () => listeners.delete(listener)
}
