import { RuleTester } from 'eslint'
import svelteParser from 'svelte-eslint-parser'
import { describe, it } from 'vitest'
import rule from './no-colour-literals.js'

RuleTester.describe = describe
RuleTester.it = it
RuleTester.itOnly = it.only

const tester = new RuleTester({ languageOptions: { parser: svelteParser } })
const filename = 'Lamp.svelte'
const error = (literal: string) => ({ message: `Colour literal "${literal}": use a token from src/ui/tokens/.` })

tester.run('no-colour-literals', rule, {
  valid: [
    { filename, code: '<b class="red">x</b>\n<style>\n  .red:hover { color: var(--rec); border: 0 solid transparent }\n</style>' },
    { filename, code: '<b style="color: var(--m); outline-color: currentColor">x</b>' },
    { filename, code: "<script>let on = true</script>\n<b style:color={on ? 'var(--lamp-ink)' : 'inherit'}>x</b>" },
    { filename, code: '<b style:width="{w}px">x</b>\n<style>\n  /* was #fff */ b { color: var(--t) }\n</style>' },
    { filename, code: '<p>Main B #1, red light</p>' },
  ],
  invalid: [
    { filename, code: '<b>x</b>\n<style>\n  b { color: #fff; background: rgb(0 0 0) }\n</style>', errors: [error('#fff'), error('rgb(')] },
    { filename, code: '<b style="color: red">x</b>', errors: [error('red')] },
    { filename, code: '<b style:color="#abc">x</b>', errors: [error('#abc')] },
    // A ternary inside a directive: both branches are checked, past the `:`.
    {
      filename,
      code: "<script>let on = true</script>\n<b style:color={on ? 'red' : 'var(--m)'}>x</b>",
      errors: [error('red')],
    },
    {
      filename,
      code: "<script>let on = true</script>\n<b style:background={on ? `var(--a)` : 'hsl(0 0% 0%)'}>x</b>",
      errors: [error('hsl(')],
    },
    { filename, code: "<b style={`color: ${'#123456'}`}>x</b>", errors: [error('#123456')] },
  ],
})
