const fs = require('fs');
const [,, src, out] = process.argv;
const html = fs.readFileSync(src, 'utf8');
const js = html.match(/data-dc-script[^>]*>([\s\S]*?)<\/script>/)[1];
const props = JSON.parse(html.match(/data-props='([^']*)'/)[1]);
const vals = new Function('DCLogic', js + '; return new Component().renderVals();')(class { constructor() { this.props = {}; } });
const helmet = (html.match(/<helmet>([\s\S]*?)<\/helmet>/) || [,''])[1];
let body = html.match(/<x-dc>([\s\S]*?)<\/x-dc>/)[1].replace(/<helmet>[\s\S]*?<\/helmet>/, '');
const get = (p, env) => p.split('.').reduce((o, k) => o == null ? undefined : o[k], env);
function render(s, env) {
  let outS = '', i = 0;
  while (true) {
    const a = s.indexOf('<sc-for', i);
    if (a < 0) { outS += s.slice(i); break; }
    outS += s.slice(i, a);
    const headEnd = s.indexOf('>', a);
    const head = s.slice(a, headEnd + 1);
    let depth = 1, k = headEnd + 1;
    while (depth) { const o = s.indexOf('<sc-for', k), c = s.indexOf('</sc-for>', k); if (o >= 0 && o < c) { depth++; k = o + 7; } else { depth--; k = c + 9; } }
    const inner = s.slice(headEnd + 1, k - 9);
    const list = get(head.match(/list="\{\{([\w.]+)\}\}"/)[1], env) || [];
    const as = head.match(/as="(\w+)"/)[1];
    for (const item of list) outS += render(inner, Object.assign({}, env, { [as]: item }));
    i = k;
  }
  return outS.replace(/\{\{([\w.]+)\}\}/g, (_, p) => { const v = get(p, env); return v === undefined ? '' : v; });
}
fs.writeFileSync(out, `<!doctype html><html><head><meta charset="utf-8">${helmet}</head><body style="margin:0">${render(body, vals)}</body></html>`);
console.log(props.$preview.width + 'x' + props.$preview.height);
