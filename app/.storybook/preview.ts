import '@fontsource/dm-sans/200.css'
import '@fontsource/dm-sans/300.css'
import '@fontsource/dm-sans/400.css'
import '@fontsource/dm-sans/500.css'
import '@fontsource/jetbrains-mono/400.css'
import '@fontsource/jetbrains-mono/500.css'
import '../src/ui/tokens/index.css'

import { DecoratorHelpers } from '@storybook/addon-themes'
import type { Decorator, Preview } from '@storybook/svelte-vite'
import ThemeFrame from './ThemeFrame.svelte'
import { THEMES, asTheme, followGround } from './theme'

// The themes addon's toolbar (dark/light), without its decorator: that one sets the theme on the
// page's root element, which would theme Storybook's own UI and docs pages too.
DecoratorHelpers.initializeThemeState(Object.keys(THEMES), 'dark')

/** Wraps each story in an element carrying the toolbar's theme; the tokens apply under it. */
const withTheme: Decorator = (_, context) => {
  const theme = asTheme(context.globals.theme)
  followGround(theme, context.globals.backgrounds)
  return { Component: ThemeFrame, props: { theme } }
}

const preview: Preview = {
  tags: ['autodocs'],
  decorators: [withTheme],
  parameters: {
    layout: 'centered',
    // The backgrounds tool sets the canvas; its options are the two themes' ground (--g), named
    // after the theme so the ground follows the theme until the user picks one (theme.ts).
    backgrounds: {
      options: {
        dark: { name: 'Dark ground', value: '#000000' },
        light: { name: 'Light ground', value: '#f2f1ee' },
      },
    },
    controls: { expanded: true },
    a11y: { test: 'error' },
  },
  initialGlobals: { theme: 'dark' },
}

export default preview
