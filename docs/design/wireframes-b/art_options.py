# Style artwork options: four generated looks, each shown big (Home) and small (browser
# list). Run by gen.py; writes project/BArtOptions.dc.html.

def art_tile(look, size, var):
    """Markup for one tile of `look`, iterating `var` (a list of styles)."""
    name = '<span style="font-size: %dpx; font-weight: 700; color: #f2f2f2; white-space: nowrap; overflow: hidden; text-overflow: ellipsis">{{t.name}}</span>' % (13 if size > 100 else 10)
    if look == "print":
        body = '''<svg width="%d" height="%d" viewBox="0 0 160 160" aria-hidden="true" style="border-radius: 8px; background: #15151a"><sc-for list="{{t.print}}" as="c" hint-placeholder-count="40"><rect x="{{c.x}}" y="{{c.y}}" width="{{c.w}}" height="{{c.h}}" rx="1.5" fill="{{c.col}}" opacity="{{c.o}}"></rect></sc-for></svg>''' % (size, size)
    elif look == "poster":
        body = '''<svg width="%d" height="%d" viewBox="0 0 160 160" aria-hidden="true" style="border-radius: 8px"><rect width="160" height="160" fill="{{t.cat}}"></rect><circle cx="{{t.cx}}" cy="{{t.cy}}" r="{{t.cr}}" fill="#0e0e10" opacity=".85"></circle><rect x="0" y="{{t.by}}" width="160" height="10" fill="#0e0e10" opacity=".85"></rect><text x="12" y="146" font-family="Archivo Narrow, sans-serif" font-size="46" font-weight="700" fill="#0e0e10">{{t.mono}}</text></svg>''' % (size, size)
    elif look == "glyph":
        body = '''<div style="width: %dpx; height: %dpx; border-radius: 8px; background: {{t.grad}}; display: flex; align-items: center; justify-content: center"><svg width="%d" height="%d" viewBox="0 0 48 48" aria-hidden="true"><path d="{{t.glyph}}" fill="none" stroke="#ffffff" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" opacity=".9"></path></svg></div>''' % (size, size, size // 2, size // 2)
    else:  # contour
        body = '''<svg width="%d" height="%d" viewBox="0 0 160 160" aria-hidden="true" style="border-radius: 8px; background: #0f0f12"><sc-for list="{{t.lines}}" as="l" hint-placeholder-count="12"><polyline points="{{l.pts}}" fill="#0f0f12" stroke="{{t.cat}}" stroke-width="1.6" opacity="{{l.o}}"></polyline></sc-for></svg>''' % (size, size)
    return '''<sc-for list="{{%s}}" as="t" hint-placeholder-count="3"><div style="width: %dpx; display: flex; flex-direction: column; gap: 5px">%s%s</div></sc-for>''' % (var, size, body, name)

LOOKS = [
    ("print", "1 · Pattern print", "A cell for every 16th each part plays, a row per part in its mixer colour. Every style looks different because every rhythm is."),
    ("poster", "2 · Genre poster", "A flat colour per category, a bold monogram and one shape placed from the style's name. Reads from across the room."),
    ("glyph", "3 · Gradient + glyph", "A soft two-colour gradient from the category and tempo, with a simple glyph per category. Closest to Kontakt library tiles."),
    ("contour", "4 · Contour lines", "Stacked lines, one per part, rising where the part is busy. Calm, and still drawn from the rhythm."),
]
rows = "".join('''<div style="display: flex; gap: 22px; align-items: center; padding: 14px 0; border-top: 1px solid #26262b">
<div style="width: 250px; flex-shrink: 0; display: flex; flex-direction: column; gap: 6px"><span style="font-family: 'Archivo Narrow', sans-serif; font-size: 24px; font-weight: 700">%s</span><span style="font-size: 12px; color: #8d8d95; line-height: 1.4">%s</span></div>
<div style="display: flex; gap: 16px">%s</div>
<div style="width: 1px; align-self: stretch; background: #26262b"></div>
<div style="display: grid; grid-template-columns: repeat(4, 70px); gap: 10px 12px">%s</div>
</div>''' % (title, desc, art_tile(look, 140, "big"), art_tile(look, 70, "small")) for look, title, desc in LOOKS)

art_html = '''<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>B · Style artwork options</title>
<script src="./support.js"></script>
</head>
<body>
<x-dc>
<helmet>
<link href="https://fonts.googleapis.com/css2?family=Archivo:wght@400;500;600;700&amp;family=Archivo+Narrow:wght@500;600;700&amp;display=swap" rel="stylesheet">
<style>body{margin:0;background:#0e0e10}</style>
</helmet>
<div style="width: 1440px; height: 900px; box-sizing: border-box; background: #0e0e10; color: #f2f2f2; font-family: Archivo, sans-serif; display: flex; flex-direction: column; padding: 24px 32px">
<div style="display: flex; align-items: baseline; gap: 16px; padding-bottom: 10px"><span style="font-family: 'Archivo Narrow', sans-serif; font-size: 32px; font-weight: 700">Style artwork: four looks</span><span style="font-size: 13px; color: #8d8d95">Each is generated from the style file. Big: Home's art panel. Small: the browser list and the quick list.</span></div>
''' + rows + '''
</div>
</x-dc>
<script type="text/x-dc" data-dc-script data-props='{"$preview":{"width":1440,"height":900}}'>
class Component extends DCLogic {
  renderVals() {
    const COL = ['#ff5a5a','#ff9a3c','#ffd23f','#9be15d','#3fd6c6','#3fa9ff','#7a7dff','#c46bff','#ff6bd0','#ff8c8c','#5de0ff','#b8f05d'];
    const CAT = { 'Pop & Rock': COL[2], 'Ballad': COL[3], 'Dance': COL[4], 'R&B': COL[5], 'Swing & Jazz': COL[6], 'Latin': COL[7], 'Country': COL[8], 'Ballroom': COL[9] };
    const GLYPH = { 'Pop & Rock': 'M10 34 V22 M18 34 V14 M26 34 V18 M34 34 V10 M42 34 V24', 'Ballad': 'M8 30 C16 14 24 14 24 24 C24 34 32 34 40 18', 'Dance': 'M24 6 L30 20 L44 24 L30 28 L24 42 L18 28 L4 24 L18 20 Z', 'R&B': 'M14 12 A10 10 0 1 0 14 36 M34 12 A10 10 0 1 1 34 36', 'Swing & Jazz': 'M18 34 A6 5 0 1 1 18 33.9 M24 34 V8 L36 12', 'Latin': 'M16 18 A8 8 0 1 1 16 17.9 M32 30 A8 8 0 1 1 32 29.9 M20 24 L28 24', 'Country': 'M8 30 Q24 6 40 30 M16 30 V38 M32 30 V38', 'Ballroom': 'M12 36 L24 10 L36 36 M16 28 H32' };
    // Genre feel: steps per bar and how busy each of the 8 style parts is.
    const FEEL = { 'Pop & Rock': [16, [.5, .3, .8, .45, .35, .2, .15, .1]], 'Ballad': [16, [.25, .15, .4, .3, .3, .5, .1, .1]], 'Dance': [16, [.55, .3, .9, .6, .4, .3, .2, .2]], 'R&B': [16, [.4, .3, .6, .5, .4, .3, .2, .15]], 'Swing & Jazz': [12, [.3, .2, .6, .7, .3, .2, .25, .1]], 'Latin': [16, [.45, .55, .5, .5, .5, .3, .3, .2]], 'Country': [16, [.4, .3, .6, .5, .45, .2, .2, .1]], 'Ballroom': [12, [.3, .2, .3, .45, .4, .4, .15, .1]] };
    const rnd = seed => { let s = seed; return () => { s = (s * 9301 + 49297) % 233280; return s / 233280; }; };
    const style = (name, cat, seed, tempo) => {
      const r = rnd(seed), f = FEEL[cat], cols = f[0], cw = 160 / cols, ch = 20, print = [];
      const busy = [];
      for (let y = 0; y < 8; y++) { const row = []; for (let x = 0; x < cols; x++) { const on = (y === 0 && x % (cols / 4) === 0) || r() < f[1][y]; row.push(on ? 1 : 0); if (on) print.push({ x: (x * cw + 1).toFixed(1), y: (y * ch + 1).toFixed(1), w: (cw - 2).toFixed(1), h: (ch - 2).toFixed(1), col: COL[4 + y], o: (0.5 + r() * 0.5).toFixed(2) }); } busy.push(row); }
      const lines = busy.map((row, y) => { const base = 28 + y * 16; let pts = '0,' + base; row.forEach((on, x) => { const px = (x + 0.5) * 160 / cols; pts += ' ' + (px - 4).toFixed(1) + ',' + base + ' ' + px.toFixed(1) + ',' + (base - (on ? 8 + r() * 14 : r() * 3)).toFixed(1) + ' ' + (px + 4).toFixed(1) + ',' + base; }); pts += ' 160,' + base; return { pts, o: (0.55 + y * 0.05).toFixed(2) }; });
      const words = name.replace(/[^A-Za-z0-9 ]/g, '').split(' ');
      const mono = words.length > 1 ? words[0][0] + words[1][0] : name.slice(0, 2);
      const c = CAT[cat], c2 = COL[(seed * 7) % 12];
      return { name, cat: c, print, lines, mono, cx: (40 + r() * 80).toFixed(0), cy: (30 + r() * 50).toFixed(0), cr: (18 + r() * 26).toFixed(0), by: (60 + r() * 40).toFixed(0),
        grad: 'radial-gradient(circle at ' + Math.round(20 + (tempo % 60)) + '% 25%, ' + c + ' 0%, ' + c2 + ' 55%, #15151a 100%)', glyph: GLYPH[cat] };
    };
    const big = [style('Cool 8Beat', 'Pop & Rock', 7, 112), style('Jazz Ballad', 'Swing & Jazz', 19, 72), style('Salsa Classic', 'Latin', 31, 180)];
    const small = [style('Funky Pop', 'Dance', 11, 118), style('Country Pop', 'Country', 23, 104), style('Slow Rock', 'Ballad', 37, 68), style('Bossa Nova', 'Latin', 41, 132), style('Big Band', 'Swing & Jazz', 53, 170), style('Soul', 'R&B', 61, 96), style('Vienna Waltz', 'Ballroom', 67, 174), style('Disco Fever', 'Dance', 71, 124)];
    return { big, small };
  }
}
</script>
</body>
</html>
'''
open(os.path.join(D, "project", "BArtOptions.dc.html"), "w").write(art_html)
