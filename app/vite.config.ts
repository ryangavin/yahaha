/// <reference types="vitest/config" />
import { viteFinal as storybookViteFinal } from '@storybook/svelte-vite/preset'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig, type Plugin, type PluginOption } from 'vite'

/**
 * Storybook's Svelte docgen plugin, for the story tests (src/ui/stories.test.ts): it attaches each
 * component's props to the component, so the composed stories get the same inferred controls as
 * Storybook. Limited to the UI library's components.
 */
async function storybookDocgen(): Promise<PluginOption[]> {
  const options = { presets: { apply: async () => '@storybook/svelte-vite' } }
  const config = await storybookViteFinal({ plugins: [] }, options as never)
  return (config.plugins ?? []).flat().map((plugin) => {
    const p = plugin as Plugin
    if (p?.name !== 'storybook:svelte-docgen-plugin' || typeof p.transform !== 'object') return plugin
    return { ...p, transform: { ...p.transform, filter: { id: { include: /\/src\/ui\/.*\.svelte$/ } } } }
  })
}

// Tauri expects a fixed port and wants to see Rust errors, so don't clear the screen.
export default defineConfig(async () => ({
  plugins: [svelte(), ...(process.env.VITEST ? await storybookDocgen() : [])],
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  build: { target: 'es2022' },
  resolve: process.env.VITEST ? { conditions: ['browser'] } : undefined,
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.ts', 'scripts/**/*.test.ts'],
  },
}))
