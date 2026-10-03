import { defineMain } from '@storybook/svelte-vite/node'

// The UI library's Storybook: every story and docs page under src/ui. The app shell's own
// components (src/lib, src/panels) are not in it.
export default defineMain({
  framework: '@storybook/svelte-vite',
  stories: ['../src/ui/**/*.mdx', '../src/ui/**/*.stories.ts'],
  addons: [
    '@storybook/addon-docs',
    '@storybook/addon-themes',
    '@storybook/addon-a11y',
    'storybook-addon-pseudo-states',
  ],
  core: { disableTelemetry: true, disableWhatsNewNotifications: true },
})
