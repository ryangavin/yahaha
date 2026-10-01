import js from '@eslint/js'
import storybook from 'eslint-plugin-storybook'
import svelte from 'eslint-plugin-svelte'
import globals from 'globals'
import ts from 'typescript-eslint'
import noColourLiterals from './scripts/lint/no-colour-literals.js'
import svelteConfig from './svelte.config.js'

export default ts.config(
  { ignores: ['dist/', 'src-tauri/', 'node_modules/', 'storybook-static/', '.shots/'] },
  js.configs.recommended,
  ...ts.configs.recommended,
  ...svelte.configs.recommended,
  ...storybook.configs['flat/recommended'],
  {
    languageOptions: { globals: { ...globals.browser, ...globals.node } },
  },
  {
    files: ['**/*.svelte', '**/*.svelte.ts'],
    languageOptions: {
      parserOptions: { extraFileExtensions: ['.svelte'], parser: ts.parser, svelteConfig },
    },
  },
  {
    rules: {
      '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }],
    },
  },
  // The UI library (src/ui): components take props and call callbacks. No store, API or Tauri.
  {
    files: ['src/ui/**'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          patterns: [
            {
              group: ['@tauri-apps/*'],
              message: 'UI library components take props and call callbacks: no Tauri.',
            },
            {
              group: ['**/lib/*', '**/lib/**', '**/panels/**', '**/help/**', '**/store*', '**/api/**', '**/App.svelte'],
              message: 'UI library components take props and call callbacks: no store, API or app code.',
            },
          ],
        },
      ],
    },
  },
  {
    files: ['src/ui/**/*.svelte'],
    plugins: { ui: { rules: { 'no-colour-literals': noColourLiterals } } },
    rules: { 'ui/no-colour-literals': 'error' },
  },
)
