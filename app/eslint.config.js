import js from '@eslint/js'
import storybook from 'eslint-plugin-storybook'
import svelte from 'eslint-plugin-svelte'
import globals from 'globals'
import ts from 'typescript-eslint'
import svelteConfig from './svelte.config.js'

// CSS colour values a UI library component may not write itself: it reads a token instead
// (docs/factory/storybook-axioms.md, axiom 2). `transparent`, `currentColor` and `inherit` are fine.
const COLOUR_FUNCTION = /\b(?:rgba?|hsla?|hwb|lab|lch|oklab|oklch|color|color-mix)\(/i
const HEX = /#[0-9a-f]{3,8}\b/i
const NAMED = new RegExp(
  '(?:^|[\\s,(])(?:' +
    'black|white|red|green|blue|yellow|orange|purple|pink|violet|brown|gr[ae]y|silver|gold|' +
    'maroon|navy|teal|lime|olive|aqua|cyan|magenta|fuchsia|indigo|coral|salmon|crimson|tomato' +
    ')(?=$|[\\s,);!])',
  'i',
)

/** Each piece of CSS in a .svelte file: the <style> block, style="…" attributes, style: directives. */
function* cssPieces(text) {
  for (const m of text.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)) yield [m[1], m.index + m[0].indexOf(m[1])]
  for (const m of text.matchAll(/\sstyle(?::[\w-]+)?=(["'{])([\s\S]*?)(?:["'}])/g)) {
    yield [m[2], m.index + m[0].indexOf(m[2], m[0].indexOf('=') + 2)]
  }
}

const noColourLiterals = {
  meta: {
    type: 'problem',
    docs: { description: 'UI library components use colour tokens, never colour literals' },
    schema: [],
  },
  create(context) {
    return {
      'Program:exit'() {
        const source = context.sourceCode
        for (const [css, start] of cssPieces(source.text)) {
          const clean = css.replace(/\/\*[\s\S]*?\*\//g, (c) => ' '.repeat(c.length))
          // Declarations, and style: directive values, which have no `prop:` in front.
          const values = clean.includes(':') ? [...clean.matchAll(/:\s*([^;{}]+)/g)] : [[clean, clean]]
          for (const decl of values) {
            const value = decl[1]
            const hit = [HEX, COLOUR_FUNCTION, NAMED].map((re) => re.exec(value)).find(Boolean)
            if (!hit) continue
            const at = start + (decl.index ?? 0) + decl[0].indexOf(value) + hit.index
            context.report({
              loc: source.getLocFromIndex(at),
              message: `Colour literal "${hit[0].trim()}": use a token from src/ui/tokens/.`,
            })
          }
        }
      },
    }
  },
}

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
