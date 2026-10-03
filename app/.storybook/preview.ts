import '@fontsource/dm-sans/200.css'
import '@fontsource/dm-sans/300.css'
import '@fontsource/dm-sans/400.css'
import '@fontsource/dm-sans/500.css'
import '@fontsource/jetbrains-mono/400.css'
import '@fontsource/jetbrains-mono/500.css'
import '../src/ui/tokens/index.css'
import './preview.css'

import { withThemeByDataAttribute } from '@storybook/addon-themes'
import type { Preview } from '@storybook/svelte-vite'
import { GLOBALS_UPDATED, SET_GLOBALS } from 'storybook/internal/core-events'
import { addons } from 'storybook/preview-api'

const THEMES = { dark: 'dark', light: 'light' }

/** Sets the theme on the root element, as the themes addon does for stories. */
function applyTheme(theme: unknown) {
  if (typeof theme === 'string' && theme in THEMES) document.documentElement.dataset.theme = theme
}

// Docs pages without stories (the Foundations page) never run the theme decorator, so they
// follow the toolbar here instead.
if (addons.hasChannel()) {
  const channel = addons.getChannel()
  for (const event of [SET_GLOBALS, GLOBALS_UPDATED]) {
    channel.on(event, (payload: { globals?: { theme?: unknown } }) => applyTheme(payload.globals?.theme))
  }
}

const preview: Preview = {
  tags: ['autodocs'],
  decorators: [
    withThemeByDataAttribute({ themes: THEMES, defaultTheme: 'dark', attributeName: 'data-theme' }),
  ],
  parameters: {
    layout: 'centered',
    // The ground comes from the theme's --g token, not from the backgrounds tool.
    backgrounds: { disable: true },
    controls: { expanded: true },
    a11y: { test: 'error' },
  },
}

export default preview
