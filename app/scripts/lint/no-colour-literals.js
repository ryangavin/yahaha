// ESLint rule ui/no-colour-literals (eslint.config.js): a UI library component reads colour
// tokens, never writes a colour itself (docs/factory/storybook-axioms.md, axiom 2). It checks the
// CSS of a .svelte file: the <style> block, style="…" attributes and style: directives, including
// every string inside a directive's {expression}. `transparent`, `currentColor` and `inherit` are
// fine. Tested in no-colour-literals.test.ts.

const HEX = /#[0-9a-f]{3,8}\b/gi
const COLOUR_FUNCTION = /\b(?:rgba?|hsla?|hwb|lab|lch|oklab|oklch|color|color-mix)\(/gi
const NAMED = new RegExp(
  '(?<=^|[\\s,(\'"`:])(?:' +
    'black|white|red|green|blue|yellow|orange|purple|pink|violet|brown|gr[ae]y|silver|gold|' +
    'maroon|navy|teal|lime|olive|aqua|cyan|magenta|fuchsia|indigo|coral|salmon|crimson|tomato' +
    ')(?=$|[\\s,);!\'"`])',
  'gi',
)

/**
 * The end of a `{…}` expression starting at `open`, skipping braces inside strings.
 * @param {string} text
 * @param {number} open index of the `{`
 * @returns {number} index of the matching `}`, or -1
 */
function closingBrace(text, open) {
  let depth = 0
  let quote = ''
  for (let i = open; i < text.length; i++) {
    const c = text[i]
    if (quote) {
      if (c === '\\') i++
      else if (c === quote) quote = ''
    } else if (c === '"' || c === "'" || c === '`') quote = c
    else if (c === '{') depth++
    else if (c === '}' && --depth === 0) return i
  }
  return -1
}

/**
 * Each piece of CSS in a .svelte file with its offset. A <style> block is scanned declaration by
 * declaration (so selectors don't count); an attribute or directive value is scanned whole.
 * @param {string} text
 * @returns {Generator<{ css: string, start: number, declarations: boolean }>}
 */
function* cssPieces(text) {
  for (const m of text.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)) {
    yield { css: m[1], start: m.index + m[0].indexOf(m[1]), declarations: true }
  }
  for (const m of text.matchAll(/\sstyle(?::[\w-]+(?:\|important)?)?=/g)) {
    const at = m.index + m[0].length
    const open = text[at]
    let end
    if (open === '"' || open === "'") end = text.indexOf(open, at + 1)
    else if (open === '{') end = closingBrace(text, at)
    else end = at + (/[\s>]/.exec(text.slice(at))?.index ?? text.length - at)
    if (end < 0) end = text.length
    const start = open === '"' || open === "'" || open === '{' ? at + 1 : at
    yield { css: text.slice(start, end), start, declarations: false }
  }
}

/**
 * Every colour literal in a piece of CSS or an expression: [offset, the literal].
 * @param {string} value
 * @returns {[number, string][]}
 */
function colours(value) {
  return [HEX, COLOUR_FUNCTION, NAMED].flatMap((re) =>
    [...value.matchAll(re)].map((m) => /** @type {[number, string]} */ ([m.index, m[0]])),
  )
}

/** @type {import('eslint').Rule.RuleModule} */
export default {
  meta: {
    type: 'problem',
    docs: { description: 'UI library components use colour tokens, never colour literals' },
    schema: [],
  },
  create(context) {
    return {
      'Program:exit'() {
        const source = context.sourceCode
        for (const { css, start, declarations } of cssPieces(source.text)) {
          // Blank out comments, keeping offsets.
          const clean = css.replace(/\/\*[\s\S]*?\*\//g, (c) => ' '.repeat(c.length))
          /** @type {[string, number][]} values and their offsets in `clean` */
          const values = declarations
            ? [...clean.matchAll(/:\s*([^;{}]+)/g)].map((d) => [d[1], d.index + d[0].indexOf(d[1])])
            : [[clean, 0]]
          for (const [value, offset] of values) {
            for (const [index, literal] of colours(value)) {
              context.report({
                loc: source.getLocFromIndex(start + offset + index),
                message: `Colour literal "${literal}": use a token from src/ui/tokens/.`,
              })
            }
          }
        }
      },
    }
  },
}
