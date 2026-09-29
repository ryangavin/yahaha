import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

// Vitest runs from app/.
const css = readFileSync(resolve('src/app.css'), 'utf8');

// Only form fields may select text (a tap on the iPad otherwise starts a selection).
// jsdom neither computes nor keeps user-select, so this reads the rules that set
// it from the stylesheet text and checks which elements they match.
describe('text selection', () => {
  const rules = [...css.replace(/\/\*[\s\S]*?\*\//g, '').matchAll(/([^{}]+)\{([^{}]*)\}/g)]
    .map(([, sel, body]) => ({ sel: sel.trim(), value: /(?:^|[;\s])user-select:\s*([\w-]+)/.exec(body)?.[1] }))
    .filter((r): r is { sel: string; value: string } => r.value !== undefined);
  document.body.innerHTML =
    '<div id="plain">label</div><button id="b">Go</button><input id="i"><textarea id="t"></textarea><select id="s"></select><div id="ce" contenteditable="true"></div>';
  const el = (id: string) => document.getElementById(id)!;
  const valuesFor = (id: string) => rules.filter((r) => el(id).matches(r.sel)).map((r) => r.value);

  it('is off from the root down', () => {
    expect(rules.some((r) => r.sel === 'html' && r.value === 'none')).toBe(true);
    for (const id of ['plain', 'b']) expect(valuesFor(id).every((v) => v === 'none'), id).toBe(true);
  });

  it('is on for form fields', () => {
    for (const id of ['i', 't', 's', 'ce']) expect(valuesFor(id), id).toContain('text');
  });
});
