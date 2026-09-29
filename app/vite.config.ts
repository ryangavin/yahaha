/// <reference types="vitest/config" />
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'

// Tauri expects a fixed port and wants to see Rust errors, so don't clear the screen.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  build: { target: 'es2022' },
  resolve: process.env.VITEST ? { conditions: ['browser'] } : undefined,
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.ts', 'scripts/**/*.test.ts'],
    // Test files in one worker share their compiled modules (App.svelte and its tree are
    // compiled once per worker, not once per file): 8 App-mounting files on 2 workers took
    // 4.6 s instead of 6.9 s. A test must not rely on module state left by another file.
    isolate: false,
  },
})
