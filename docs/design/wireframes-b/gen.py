import os
D = os.path.dirname(os.path.abspath(__file__))
shell = open(os.path.join(D, "shell.html")).read()

ART = '''<div style="width: 280px; flex-shrink: 0; position: relative; overflow: hidden; background: #15151a">
<svg width="280" height="366" viewBox="0 0 300 366" preserveAspectRatio="xMidYMid slice" aria-hidden="true" style="position: absolute; inset: 0">
<sc-for list="{{art.bars}}" as="b" hint-placeholder-count="12"><rect x="{{b.x}}" y="{{b.y}}" width="{{b.w}}" height="{{b.h}}" fill="{{b.col}}" opacity="{{b.o}}" rx="2"></rect></sc-for>
<sc-for list="{{art.rings}}" as="c" hint-placeholder-count="9"><circle cx="{{c.x}}" cy="{{c.y}}" r="{{c.r}}" fill="none" stroke="{{c.col}}" stroke-width="{{c.sw}}" opacity="{{c.o}}"></circle></sc-for>
<defs><linearGradient id="fade" x1="0" y1="0" x2="0" y2="1"><stop offset="0.4" stop-color="#121317" stop-opacity="0"></stop><stop offset="1" stop-color="#121317" stop-opacity="0.96"></stop></linearGradient></defs>
<rect width="300" height="366" fill="url(#fade)"></rect>
</svg>
<div style="position: absolute; left: 18px; right: 18px; bottom: 16px; display: flex; flex-direction: column; gap: 4px">
<span class="cap">Pop &amp; Rock · 4/4</span>
<div style="font-family: 'Archivo Narrow', sans-serif; font-size: 34px; font-weight: 700; line-height: 1">Cool 8Beat</div>
<div style="display: flex; gap: 6px; margin-top: 6px"><button class="chip">Browse</button><button class="chip">Edit…</button></div>
</div>
</div>'''

KNOBROW = '''<sc-for list="{{%s}}" as="k" hint-placeholder-count="4">
<div style="display: flex; flex-direction: column; align-items: center; gap: 4px; width: %dpx">
<svg width="%d" height="%d" viewBox="0 0 %d %d" aria-hidden="true"><path d="{{k.track}}" stroke="#2d2d32" stroke-width="4" fill="none" stroke-linecap="round"></path><path d="{{k.arc}}" stroke="{{k.col}}" stroke-width="4" fill="none" stroke-linecap="round"></path></svg>
<span style="font-size: 8px; font-weight: 700; letter-spacing: .05em; text-transform: uppercase; color: #a9a9b1">{{k.name}}</span>
<span style="font-size: 10px; font-weight: 600; color: #c9c9cf; min-height: 12px">{{k.val}}</span>
</div>
</sc-for>'''
def krow(var, size=52, w=70):
    return KNOBROW % (var, w, size, size, size, size)


# The style artwork: an abstract swirl of colour from a few of the style's attributes. It is
# the display background (full bleed) and the style's art everywhere. It doesn't try to encode
# the structure: the category picks the palette, the tempo sets how tight the swirl turns, the
# energy sets how bright it is, and the seed (the file) makes every style its own.
# (The function keeps its old name and arguments so every caller stays the same.)
SONGMAP_JS = r"""
    const PAL = { pop: ['#ff8a3d', '#ff4f6d', '#ffc94d', '#b83dff'], ballad: ['#8f7bff', '#3fc6c6', '#ff8fb3', '#2b3a8f'],
      latin: ['#ff6b2b', '#ff3d8b', '#ffd23f', '#1fb8a0'], dance: ['#3de0ff', '#8b5cff', '#ff4fd8', '#1f5cff'] };
    const songMap = (seed, cat, hot, form, gid, W, H) => {
      let s = seed * 7 + 3; const r = () => { s = (s * 9301 + 49297) % 233280; return s / 233280; };
      const pal = PAL[form].concat([cat, hot]), tempo = 70 + r() * 110, energy = 0.55 + r() * 0.45;
      const turn = 1.6 + (tempo - 70) / 110 * 2.2, m = Math.min(W, H), big = Math.max(W, H), blobs = [];
      const start = r() * 6.28, n = 7;
      for (let i = 0; i < n; i++) { const t = i / (n - 1), ang = start + t * turn * 3.14, rad = (0.08 + t * 0.42) * big;
        blobs.push({ cx: (W / 2 + Math.cos(ang) * rad).toFixed(1), cy: (H / 2 + Math.sin(ang) * rad * (H / W + 0.35)).toFixed(1), rx: ((0.22 + r() * 0.28) * big).toFixed(1), ry: ((0.12 + r() * 0.2) * big).toFixed(1),
          rot: ((ang * 57.3 + 90) % 360).toFixed(0), c: pal[Math.floor(r() * pal.length)], o: (0.55 + r() * 0.4 * energy).toFixed(2) }); }
      return { gid, url: 'url(#' + gid + ')', blobs, blur: (m * 0.16).toFixed(1), bg: pal[3], dim: (0.55 - energy * 0.35).toFixed(2), H, W };
    };
"""
SONGMAP_SVG = """<defs><filter id="{{%(v)s.gid}}" x="-50%%" y="-50%%" width="200%%" height="200%%"><feGaussianBlur stdDeviation="{{%(v)s.blur}}"></feGaussianBlur></filter></defs><rect width="100%%" height="100%%" fill="{{%(v)s.bg}}"></rect><g filter="{{%(v)s.url}}"><sc-for list="{{%(v)s.blobs}}" as="b" hint-placeholder-count="7"><ellipse cx="{{b.cx}}" cy="{{b.cy}}" rx="{{b.rx}}" ry="{{b.ry}}" fill="{{b.c}}" opacity="{{b.o}}" transform="rotate({{b.rot}} {{b.cx}} {{b.cy}})"></ellipse></sc-for></g><rect width="100%%" height="100%%" fill="#0e0e10" opacity="{{%(v)s.dim}}"></rect>"""

screens = {}

# ---------- Home ----------
# The sections are the Launchkey Sections pad page, drawn big: same order, colours and
# lights as the hardware. The Mains carry their real pattern (a row per drum/bass voice).
exec(open(os.path.join(D, "home.py")).read())
screens["Home"] = home()

# ---------- Channel ----------
screens["Channel"] = dict(TAB="Channel", SEL="7", DISPLAY='''
<div style="width: 280px; flex-shrink: 0; display: flex; flex-direction: column; gap: 10px; padding: 16px; margin: 8px 0 8px 8px; border-radius: 8px; border: 1px solid #2d2d32; background: #19191c; box-shadow: inset 0 4px 0 #c46bff">
<div style="display: flex; justify-content: space-between; align-items: center"><button class="chip" aria-label="Previous part">‹ Bass</button><button class="chip" aria-label="Next part">Chord 2 ›</button></div>
<span class="cap" style="color: #c46bff">Style part · ch 12</span>
<div style="font-family: 'Archivo Narrow', sans-serif; font-size: 40px; font-weight: 700; line-height: 1">Chord 1</div>
<button style="height: 44px; border-radius: 6px; border: 1px solid #c46bff; background: #1b1b20; color: #f2f2f2; font-size: 15px; font-weight: 700; text-align: left; padding: 0 12px">Steel Gtr ▾<span style="display: block; font-size: 10px; font-weight: 500; color: #8d8d95">SoundFont · from Yamaha 8/1/2</span></button>
<div style="display: flex; gap: 6px"><button class="chip on">On</button><button class="chip">Mute</button><button class="chip">Solo</button></div>
<div style="flex-grow: 1"></div>
<span style="font-size: 11px; color: #8d8d95">Click any strip below to edit it here.</span>
</div>
<div style="flex-grow: 1; display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 8px; padding: 8px">
<div style="display: flex; flex-direction: column; gap: 8px; padding: 12px; border-radius: 8px; border: 1px solid #2d2d32; background: #19191c"><span class="cap">Mix · pan and sends (level is on the strip)</span>
<div style="display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 10px; justify-items: center">''' + krow("sends") + '''</div>
<span style="font-size: 11px; color: #8d8d95">Band sends are scaled by Effects › Band.</span>
</div>
<div style="display: flex; flex-direction: column; gap: 8px; padding: 12px; border-radius: 8px; border: 1px solid #2d2d32; background: #19191c"><span class="cap">Tone · from OTS / voice</span>
<div style="display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 8px; justify-items: center">''' + krow("tone", 46, 64) + '''</div>
</div>
<div style="display: flex; flex-direction: column; gap: 10px; padding: 12px; border-radius: 8px; border: 1px solid #2d2d32; background: #19191c"><span class="cap">Play</span>
<div style="display: flex; align-items: center; justify-content: space-between"><span style="font-size: 13px">Octave</span><div style="display: flex; gap: 4px; align-items: center"><button class="chip">−</button><span style="width: 26px; text-align: center; font-weight: 700">0</span><button class="chip">+</button></div></div>
<div style="display: flex; align-items: center; justify-content: space-between"><span style="font-size: 13px">Mono / Poly</span><div style="display: flex; gap: 4px"><button class="chip">Mono</button><button class="chip on">Poly</button></div></div>
<div style="display: flex; align-items: center; justify-content: space-between"><span style="font-size: 13px">Bend range</span><div style="display: flex; gap: 4px; align-items: center"><button class="chip">−</button><span style="width: 26px; text-align: center; font-weight: 700">2</span><button class="chip">+</button></div></div>
<div style="display: flex; justify-content: center">''' + krow("porta", 44, 70) + '''</div>
</div>
</div>''', JS='''
    const C = '#c46bff';
    const K = (v, name, val) => Object.assign(knob(52, v, C), { name, val });
    const extra = { pan: [Object.assign(knob(44, .5, C), { name: 'Pan', val: 'C' })], sends: [K(.5, 'Pan', 'C'), K(.35, 'Reverb', ''), K(.1, 'Chorus', ''), K(0, 'Delay', ''), K(.8, 'Dry', '')], tone: [Object.assign(knob(46, .7, C), { name: 'Cutoff', val: '' }), Object.assign(knob(46, .5, C), { name: 'Reso', val: '' }), Object.assign(knob(46, .45, C), { name: 'Attack', val: '' }), Object.assign(knob(46, .5, C), { name: 'Decay', val: '' }), Object.assign(knob(46, .55, C), { name: 'Release', val: '' }), Object.assign(knob(46, .5, C), { name: 'Vibrato', val: '' })], porta: [Object.assign(knob(44, 0, C), { name: 'Portamento', val: 'Off' })] };''', OVERLAY="")

# ---------- Effects ----------
def fxcard(name, var, types, extra_html=""):
    return '''<div style="flex: 1; display: flex; flex-direction: column; gap: 10px; padding: 12px; border-radius: 8px; border: 1px solid #2d2d32; background: #19191c">
<div style="display: flex; align-items: center; justify-content: space-between"><span style="font-family: 'Archivo Narrow', sans-serif; font-size: 24px; font-weight: 700">%s</span><div style="display: flex; gap: 4px"><button class="chip">From style: %s</button><button class="chip on">Mine</button></div></div>
<div style="display: flex; gap: 4px; flex-wrap: wrap">%s</div>
<div style="display: flex; justify-content: space-around">%s</div>
%s
<div style="display: flex; flex-direction: column; gap: 4px; margin-top: auto"><div style="display: flex; justify-content: space-between"><span class="cap">Band send</span><span style="font-size: 12px; font-weight: 700">{{%s.band}}</span></div><div style="height: 8px; border-radius: 4px; background: #26262b; position: relative"><div style="position: absolute; left: 0; top: 0; bottom: 0; width: {{%s.bandPct}}%%; border-radius: 4px; background: #ffffff"></div></div><span style="font-size: 11px; color: #8d8d95">How much the band's parts feed this effect. Your keyboard parts use their own sends.</span></div>
</div>''' % (name, types[0], "".join('<button class="chip%s">%s</button>' % (" on" if i == 0 else "", t) for i, t in enumerate(types[1:])), krow(var + ".knobs", 50, 64), extra_html, var, var)
screens["Effects"] = dict(TAB="Effects", SEL="-1", DISPLAY='''<div style="flex-grow: 1; display: flex; gap: 8px; padding: 8px">''' +
  fxcard("Reverb", "rev", ["Hall 2", "Hall", "Room", "Stage", "Plate"]) +
  fxcard("Chorus", "cho", ["Chorus 1", "Chorus", "Celeste", "Flanger"]) +
  fxcard("Delay", "dly", ["Tempo Echo", "1/16", "1/8T", "1/8", "1/8.", "1/4", "1/4T", "1/4.", "1/2"], '<div style="display: flex; gap: 6px"><button class="chip on">Tempo sync</button><button class="chip">Ping-pong</button></div>') +
  '</div>', LAYER="REV", JS='''
    const K = (v, name, val, c) => Object.assign(knob(50, v, c || '#f2f2f2'), { name, val });
    const extra = { rev: { knobs: [K(.55, 'Return', '0 dB', A), K(.4, 'Time', '2.4 s'), K(.2, 'Pre-delay', '20 ms'), K(.6, 'Tone', 'Warm')], band: '100%', bandPct: 100 }, cho: { knobs: [K(.5, 'Return', '0 dB', A), K(.3, 'Rate', '0.8 Hz'), K(.45, 'Depth', '45%')], band: '0%', bandPct: 0 }, dly: { knobs: [K(.4, 'Return', '−3 dB', A), K(.35, 'Feedback', '35%'), K(.6, 'Tone', '6 kHz')], band: '0%', bandPct: 0 } };''', OVERLAY="")

# ---------- Multi Pads ----------
screens["MultiPads"] = dict(TAB="Multi Pads", SEL="-1", DISPLAY='''
<div style="width: 280px; flex-shrink: 0; display: flex; flex-direction: column; gap: 10px; padding: 16px; border-right: 1px solid #26262b">
<span class="cap">Multi Pad bank</span>
<div style="font-family: 'Archivo Narrow', sans-serif; font-size: 30px; font-weight: 700; line-height: 1">Pop Hits 2</div>
<div style="display: flex; gap: 6px"><button class="chip">Browse banks</button><button class="chip">‹</button><button class="chip">›</button></div>
<div style="display: flex; flex-direction: column; gap: 4px; margin-top: 8px"><div style="display: flex; justify-content: space-between"><span class="cap">Pad level (fader 6)</span><span style="font-size: 12px; font-weight: 700">85%</span></div><div style="height: 8px; border-radius: 4px; background: #26262b; position: relative"><div style="position: absolute; left: 0; top: 0; bottom: 0; width: 67%; border-radius: 4px; background: #ffffff"></div></div></div>
<div style="flex-grow: 1"></div>
<button class="chip" style="height: 40px">■ Stop all pads</button>
</div>
<div style="flex-grow: 1; display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 12px; padding: 16px">
<sc-for list="{{mp}}" as="p" hint-placeholder-count="4">
<button style="border-radius: 10px; border: 2px solid {{p.col}}; background: {{p.bg}}; color: {{p.ink}}; text-align: left; padding: 14px; display: flex; flex-direction: column; gap: 8px; box-shadow: {{p.glow}}">
<span style="font-family: 'Archivo Narrow', sans-serif; font-size: 44px; font-weight: 700; line-height: 1">{{p.n}}</span>
<span style="font-size: 16px; font-weight: 700">{{p.name}}</span>
<span style="font-size: 12px; font-weight: 600; opacity: .8">{{p.state}}</span>
<span style="flex-grow: 1"></span>
<span style="display: flex; gap: 6px"><span style="font-size: 10px; font-weight: 700; padding: 3px 6px; border-radius: 3px; border: 1px solid currentColor; opacity: {{p.syncOp}}">SYNC</span><span style="font-size: 10px; font-weight: 700; padding: 3px 6px; border-radius: 3px; border: 1px solid currentColor; opacity: {{p.repOp}}">REPEAT</span><span style="font-size: 10px; font-weight: 700; padding: 3px 6px; border-radius: 3px; border: 1px solid currentColor; opacity: {{p.chOp}}">CHORD</span></span>
</button>
</sc-for>
</div>''', JS='''
    const MP = [['Guitar Riff', 'Playing · bar 2 of 4', 1, 1, 1, 'play'], ['Horn Stab', 'Armed: starts at the next bar', 1, 0, 1, 'arm'], ['Shaker Loop', 'Ready', 0, 1, 0, ''], ['Crash + FX', 'Ready', 0, 0, 0, '']];
    const extra = { mp: MP.map((p, i) => { const c = COL[i * 3 + 1]; const st = p[5]; return { n: i + 1, name: p[0], state: p[1], col: c, bg: st === 'play' ? c : (st === 'arm' ? '#1b1b20' : '#141417'), ink: st === 'play' ? '#0e0e10' : '#f2f2f2', glow: st === 'play' ? '0 0 24px ' + c : 'none', syncOp: p[2] ? 1 : .3, repOp: p[3] ? 1 : .3, chOp: p[4] ? 1 : .3 }; }) };''', OVERLAY="")

# ---------- Looper & Charts ----------
screens["Looper"] = dict(TAB="Looper & Charts", SEL="-1", DISPLAY='''
<div style="width: 640px; flex-shrink: 0; display: flex; flex-direction: column; gap: 12px; padding: 16px; border-right: 1px solid #26262b">
<div style="display: flex; align-items: center; gap: 8px"><span style="font-family: 'Archivo Narrow', sans-serif; font-size: 24px; font-weight: 700">Chord Looper</span><div style="flex-grow: 1"></div><button class="chip">Bank: Ballad Set ▾</button><button class="chip">Save</button></div>
<div style="display: flex; gap: 6px"><sc-for list="{{mem}}" as="m" hint-placeholder-count="8"><button style="flex: 1; height: 40px; border-radius: 6px; border: 1px solid {{m.border}}; background: {{m.bg}}; color: {{m.ink}}; font-weight: 700">{{m.n}}</button></sc-for></div>
<div style="display: flex; gap: 6px; flex-wrap: wrap"><sc-for list="{{seq}}" as="c" hint-placeholder-count="8"><span style="min-width: 62px; height: 46px; border-radius: 6px; display: flex; align-items: center; justify-content: center; font-size: 18px; font-weight: 700; background: {{c.bg}}; color: {{c.ink}}; border: 1px solid {{c.border}}">{{c.ch}}</span></sc-for></div>
<div style="flex-grow: 1"></div>
<div style="display: flex; gap: 8px"><button class="hb" style="height: 48px; flex: 1; background: #ff5a5a; border-color: #ff5a5a; color: #0e0e10">● Rec / Stop</button><button class="hb" style="height: 48px; flex: 1; background: #ffffff; border-color: #ffffff; color: #0e0e10">Loop: On</button><button class="hb" style="height: 48px; flex: 1">Clear</button></div>
</div>
<div style="flex-grow: 1; display: flex; flex-direction: column; gap: 10px; padding: 16px">
<div style="display: flex; align-items: center; gap: 8px"><span style="font-family: 'Archivo Narrow', sans-serif; font-size: 24px; font-weight: 700">Autumn Leaves</span><span class="cap">Chart · 2 choruses</span><div style="flex-grow: 1"></div><button class="chip">Songs ▾</button><button class="chip on">Follow</button></div>
<div style="flex-grow: 1; display: grid; grid-template-columns: repeat(8, minmax(0, 1fr)); gap: 4px">
<sc-for list="{{bars}}" as="b" hint-placeholder-count="32"><div style="border-radius: 4px; padding: 6px; background: {{b.bg}}; border: 1px solid {{b.border}}; display: flex; flex-direction: column; justify-content: space-between"><span style="font-size: 9px; color: #8d8d95">{{b.n}}</span><span style="font-size: 15px; font-weight: 700; color: {{b.ink}}">{{b.ch}}</span></div></sc-for>
</div>
</div>''', JS='''
    const mem = Array.from({ length: 8 }, (_, i) => ({ n: i + 1, bg: i === 1 ? A : '#1b1b20', ink: i === 1 ? AI : (i < 4 ? '#f2f2f2' : '#55555c'), border: i === 1 ? A : '#2d2d32' }));
    const seq = ['C', 'Am', 'F', 'G7', 'C', 'Am', 'Dm7', 'G7'].map((c, i) => ({ ch: c, bg: i === 5 ? '#26262e' : '#141417', ink: '#f2f2f2', border: i === 5 ? A : '#2d2d32' }));
    const CH = ['Cm7','F7','BbM7','EbM7','Am7b5','D7','Gm','Gm','Am7b5','D7','Gm','Gm','Cm7','F7','BbM7','EbM7','Am7b5','D7','Gm','G7','Cm7','F7','Bb','Eb','Am7b5','D7','Gm7','C7','Fm7','Bb7','EbM7','D7'];
    const bars = CH.map((c, i) => ({ n: i + 1, ch: c, bg: i === 9 ? 'rgba(255,255,255,.18)' : (i % 8 === 0 ? '#1b1b20' : '#141417'), border: i === 9 ? A : '#26262b', ink: i === 9 ? A : '#f2f2f2' }));
    const extra = { mem, seq, bars };''', OVERLAY="")

# ---------- Pads & Loops (Multi Pads, Chord Looper and the chart in one tab) ----------
screens["PadsLoops"] = dict(TAB="Pads & Loops", SEL="-1", PADPAGE="multi", DISPLAY='''
<div style="width: 330px; flex-shrink: 0; display: flex; flex-direction: column; gap: 8px; padding: 12px; margin: 8px 0 8px 8px; border-radius: 8px; border: 1px solid #2d2d32; background: #19191c">
<div style="display: flex; align-items: center; gap: 6px"><span style="font-family: 'Archivo Narrow', sans-serif; font-size: 20px; font-weight: 700">Multi Pads</span><div style="flex-grow: 1"></div><button class="chip" style="height: 24px">Pop Hits 2 ▾</button></div>
<div style="flex-grow: 1; display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px">
<sc-for list="{{mp}}" as="p" hint-placeholder-count="4"><button style="border-radius: 4px; border: 1px solid {{p.border}}; outline: {{p.outline}}; outline-offset: 1px; background: {{p.bg}}; color: {{p.ink}}; text-align: left; padding: 9px 10px 8px; display: flex; flex-direction: column; gap: 2px; box-shadow: {{p.glow}}"><span style="font-size: 14px; font-weight: 800">{{p.n}} · {{p.name}}</span><span style="font-size: 11px; font-weight: 600; opacity: .8">{{p.state}}</span><span style="flex-grow: 1"></span><span style="font-size: 9px; font-weight: 700; letter-spacing: .05em; opacity: .75">{{p.flags}}</span></button></sc-for>
</div>
<div style="display: flex; align-items: center; gap: 8px"><span class="cap">Level</span><div style="flex-grow: 1; height: 8px; border-radius: 4px; background: #26262b; position: relative"><div style="position: absolute; left: 0; top: 0; bottom: 0; width: 67%; border-radius: 4px; background: #ffffff"></div></div><button class="chip" style="height: 24px">■ Stop all</button></div>
</div>
<div style="width: 470px; flex-shrink: 0; display: flex; flex-direction: column; gap: 8px; padding: 12px; margin: 8px 0 8px 8px; border-radius: 8px; border: 1px solid #2d2d32; background: #19191c">
<div style="display: flex; align-items: center; gap: 6px"><span style="font-family: 'Archivo Narrow', sans-serif; font-size: 20px; font-weight: 700">Chord Looper</span><div style="flex-grow: 1"></div><button class="chip" style="height: 24px">Ballad Set ▾</button><button class="chip" style="height: 24px">Save</button></div>
<div style="display: flex; gap: 4px"><sc-for list="{{mem}}" as="m" hint-placeholder-count="8"><button style="flex: 1; height: 30px; border-radius: 5px; border: 1px solid {{m.border}}; background: {{m.bg}}; color: {{m.ink}}; font-weight: 700">{{m.n}}</button></sc-for></div>
<div style="display: flex; gap: 4px; flex-wrap: wrap"><sc-for list="{{seq}}" as="c" hint-placeholder-count="8"><span style="min-width: 50px; height: 36px; border-radius: 5px; display: flex; align-items: center; justify-content: center; font-size: 15px; font-weight: 700; background: {{c.bg}}; color: {{c.ink}}; border: 1px solid {{c.border}}">{{c.ch}}</span></sc-for></div>
<div style="flex-grow: 1"></div>
<div style="display: flex; gap: 6px"><button class="hb" style="height: 38px; flex: 1; background: #ff5a5a; border-color: #ff5a5a; color: #0e0e10">● Rec / Stop</button><button class="hb" style="height: 38px; flex: 1; background: #ffffff; border-color: #ffffff; color: #0e0e10">Loop: On</button><button class="hb" style="height: 38px; flex: 1">Clear</button></div>
</div>
<div style="flex-grow: 1; min-width: 0; display: flex; flex-direction: column; gap: 8px; padding: 12px; margin: 8px; border-radius: 8px; border: 1px solid #2d2d32; background: #19191c">
<div style="display: flex; align-items: center; gap: 6px"><span style="font-family: 'Archivo Narrow', sans-serif; font-size: 20px; font-weight: 700">Autumn Leaves</span><span class="cap">Chart</span><div style="flex-grow: 1"></div><button class="chip" style="height: 24px">Songs ▾</button><button class="chip on" style="height: 24px">Follow</button></div>
<div style="flex-grow: 1; display: grid; grid-template-columns: repeat(8, minmax(0, 1fr)); grid-auto-rows: minmax(0, 1fr); gap: 3px">
<sc-for list="{{bars}}" as="b" hint-placeholder-count="32"><div style="border-radius: 3px; padding: 3px 4px; background: {{b.bg}}; border: 1px solid {{b.border}}; display: flex; flex-direction: column; justify-content: space-between; min-height: 0"><span style="font-size: 8px; color: #8d8d95">{{b.n}}</span><span style="font-size: 12px; font-weight: 700; color: {{b.ink}}">{{b.ch}}</span></div></sc-for>
</div>
</div>''', JS='''
    const MP = [['Guitar Riff', 'Playing · bar 2 of 4', 'SYNC · REPEAT · CHORD', 'play'], ['Horn Stab', 'Armed: starts at the next bar', 'SYNC · CHORD', 'arm'], ['Shaker Loop', 'Ready', 'REPEAT', ''], ['Crash + FX', 'Ready', '', '']];
    const mp = MP.map((p, i) => { const st = p[3], c = st ? COL[0] : COL[5]; return { n: i + 1, name: p[0], state: p[1], flags: p[2], col: c, border: st === 'play' ? c : '#2d2d32', outline: st === 'arm' ? '2px dashed #ffffff' : 'none', bg: st === 'play' ? c : '#1b1b20', ink: st === 'play' ? '#0e0e10' : '#f2f2f2', glow: st === 'play' ? '0 0 12px ' + c + '80' : 'inset 0 3px 0 ' + c }; });
    const mem = Array.from({ length: 8 }, (_, i) => ({ n: i + 1, bg: i === 1 ? A : '#1b1b20', ink: i === 1 ? AI : (i < 4 ? '#f2f2f2' : '#55555c'), border: i === 1 ? A : '#2d2d32' }));
    const seq = ['C', 'Am', 'F', 'G7', 'C', 'Am', 'Dm7', 'G7'].map((c, i) => ({ ch: c, bg: i === 5 ? '#26262e' : '#141417', ink: '#f2f2f2', border: i === 5 ? A : '#2d2d32' }));
    const CH = ['Cm7','F7','BbM7','EbM7','Am7b5','D7','Gm','Gm','Am7b5','D7','Gm','Gm','Cm7','F7','BbM7','EbM7','Am7b5','D7','Gm','G7','Cm7','F7','Bb','Eb','Am7b5','D7','Gm7','C7','Fm7','Bb7','EbM7','D7'];
    const bars = CH.map((c, i) => ({ n: i + 1, ch: c, bg: i === 9 ? 'rgba(255,255,255,.16)' : (i % 8 === 0 ? '#1b1b20' : '#141417'), border: i === 9 ? A : '#26262b', ink: '#f2f2f2' }));
    const extra = { mp, mem, seq, bars };''', OVERLAY="")

# ---------- Voice quick list ----------
screens["VoiceList"] = dict(TAB="Home", SEL="7", PADRING="1", DISPLAY=screens["Home"]["DISPLAY"], JS=screens["Home"]["JS"], OVERLAY='''
<div role="dialog" aria-label="Voices for Chord 1" style="position: absolute; left: 800px; top: 250px; width: 300px; padding: 8px; border-radius: 8px; background: #1f1f24; border: 1px solid #c46bff; box-shadow: 0 18px 40px rgba(0,0,0,.6); display: flex; flex-direction: column; gap: 2px">
<div style="display: flex; justify-content: space-between; padding: 4px 8px"><span class="cap" style="color: #c46bff">Chord 1 · Guitars</span><span class="cap">Esc</span></div>
<sc-for list="{{vl}}" as="v" hint-placeholder-count="8"><button style="height: 32px; border: 0; border-radius: 4px; background: {{v.bg}}; color: #f2f2f2; text-align: left; padding: 0 10px; font-size: 13px; font-weight: {{v.w}}; display: flex; justify-content: space-between; align-items: center">{{v.name}}<span style="font-size: 10px; color: #8d8d95">{{v.src}}</span></button></sc-for>
<div style="height: 1px; background: #2d2d32; margin: 4px 0"></div>
<button style="height: 34px; border: 0; border-radius: 4px; background: #26262b; color: #ffffff; text-align: left; padding: 0 10px; font-size: 13px; font-weight: 700">More in the Browser…</button>
</div>''')
screens["VoiceList"]["JS"] = screens["Home"]["JS"].replace("const extra = { homePads, master, legend, lkKnobs, barsRow };", "const vl = [['Steel Gtr','current'],['Nylon Gtr','SoundFont'],['Clean Gtr','SoundFont'],['Jazz Gtr','SoundFont'],['12-String','SoundFont'],['★ Ample Guitar M','plugin'],['★ My Strum Gtr','patch'],['Muted Gtr','SoundFont']].map((v, i) => ({ name: v[0], src: v[1], bg: i === 0 ? '#2c2536' : 'transparent', w: i === 0 ? 700 : 500 }));\n    const extra = { homePads, master, legend, lkKnobs, barsRow, vl };")

# ---------- Browser (full screen) ----------
screens["Browser"] = dict(TAB="Home", SEL="-1", PADRING="1", DISPLAY=screens["Home"]["DISPLAY"], BROWSEBTN="background: #ffffff; color: #0e0e10; border-color: #ffffff", OVERLAY='''
<div role="dialog" aria-label="Browser" style="position: absolute; left: 0; right: 0; top: 52px; bottom: 0; background: #0e0e10; display: flex; gap: 0">
<nav style="width: 220px; flex-shrink: 0; padding: 14px; display: flex; flex-direction: column; gap: 4px; border-right: 1px solid #26262b">
<div style="display: flex; gap: 4px; margin-bottom: 10px"><button class="chip on">Styles</button><button class="chip">Voices</button><button class="chip">Pads</button><button class="chip">Songs</button></div>
<sc-for list="{{cats}}" as="c" hint-placeholder-count="10"><button style="height: 34px; border: 0; border-radius: 4px; background: {{c.bg}}; color: {{c.ink}}; text-align: left; padding: 0 10px; font-size: 13px; font-weight: 600; display: flex; justify-content: space-between; align-items: center"><span style="display: flex; gap: 8px; align-items: center"><span style="width: 8px; height: 8px; border-radius: 2px; background: {{c.col}}"></span>{{c.name}}</span><span style="font-size: 11px; color: #8d8d95">{{c.n}}</span></button></sc-for>
</nav>
<div style="flex-grow: 1; display: flex; flex-direction: column; gap: 12px; padding: 14px; min-width: 0">
<div style="display: flex; gap: 8px"><input aria-label="Search" value="" placeholder="Search styles, tempo, song title…" style="flex-grow: 1; height: 38px; padding: 0 12px; border-radius: 6px; border: 1px solid #2d2d32; background: #19191c; color: #f2f2f2; font-size: 14px"><button class="chip">★ Favourites</button><button class="chip">My Styles</button><button class="chip">Drafts</button><button class="hb" aria-label="Close browser">✕ Close</button></div>
<div style="flex-grow: 1; display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 12px; align-content: start">
<sc-for list="{{cards}}" as="s" hint-placeholder-count="15">
<button style="padding: 0; border-radius: 8px; overflow: hidden; border: 2px solid {{s.border}}; background: #19191c; color: #f2f2f2; text-align: left; display: flex; flex-direction: column">
<svg width="100%" height="110" viewBox="0 0 200 110" preserveAspectRatio="none" aria-hidden="true" style="display: block">''' + SONGMAP_SVG % {'v': 's.m'} + '''</svg>
<span style="padding: 8px 10px 2px; font-size: 14px; font-weight: 700">{{s.name}}</span>
<span style="padding: 0 10px 10px; font-size: 11px; color: #8d8d95">{{s.meta}}</span>
</button>
</sc-for>
</div>
</div>
<aside style="width: 300px; flex-shrink: 0; padding: 14px; display: flex; flex-direction: column; gap: 10px; border-left: 1px solid #26262b">
<svg width="272" height="180" viewBox="0 0 272 180" preserveAspectRatio="none" aria-hidden="true" style="border-radius: 8px">''' + SONGMAP_SVG % {'v': 'sel.m'} + '''</svg>
<div style="font-family: 'Archivo Narrow', sans-serif; font-size: 28px; font-weight: 700; line-height: 1">Funky Pop</div>
<span class="cap">Dance · 4/4 · 118 bpm · SFF2</span>
<span style="font-size: 12px; color: #8d8d95">Main A–D, 3 Intros, 3 Endings · 4 One Touch Settings · Chord guitars on Yamaha Mega Voices</span>
<div style="display: flex; gap: 6px"><button class="hb" style="flex: 1">▶ Preview</button><button class="hb" style="flex: 1; background: #ffffff; color: #0e0e10; border-color: #ffffff">Load</button></div>
<div style="display: flex; gap: 6px"><button class="chip" style="flex: 1">★ Favourite</button><button class="chip" style="flex: 1">Edit a copy…</button></div>
<div style="flex-grow: 1"></div>
<span style="font-size: 11px; color: #8d8d95">Tip: while the style plays, Load waits for the end of the bar (or the Ending) like the Genos.</span>
</aside>
</div>''', JS='''
    const CATS = [['All', 208], ['Favourites', 12], ['Pop & Rock', 46], ['Ballad', 19], ['Dance', 24], ['R&B', 17], ['Swing & Jazz', 21], ['Latin', 18], ['Country', 14], ['Ballroom', 11], ['Entertainer', 9], ['World', 9]];
    const cats = CATS.map((c, i) => ({ name: c[0], n: c[1], col: COL[i % 12], bg: i === 4 ? '#222228' : 'transparent', ink: i === 4 ? '#ffffff' : '#c9c9cf' }));
    const S = [['Funky Pop', 'Dance · 118'], ['Disco Fever', 'Dance · 124'], ['Synth Pop', 'Dance · 124'], ['Club House', 'Dance · 126'], ['Dance Pop 2', 'Dance · 120'], ['Euro Beat', 'Dance · 140'], ['Nu Disco', 'Dance · 116'], ['Future Bass', 'Dance · 150'], ['Tropical', 'Dance · 102'], ['Electro Swing', 'Dance · 128'], ['Deep House', 'Dance · 122'], ['80s Dance', 'Dance · 120'], ['Funky Finger', 'Dance · 112'], ['Trance Pop', 'Dance · 132'], ['Latin House', 'Dance · 124']];
    const FORMS = ['dance', 'pop', 'latin', 'ballad'];
    const cards = S.map((s, i) => ({ name: s[0], meta: s[1] + ' bpm', border: i === 0 ? A : 'transparent', m: songMap(11 + i * 5, COL[4], ['#ff5a3c', '#ff6bd0', '#ffd23f', '#ff9a3c'][i % 4], FORMS[i % 4], 'bc' + i, 200, 110, -1) }));
    const extra = Object.assign(homeExtra, { cats, cards, sel: { m: songMap(11, COL[4], '#ff5a3c', 'dance', 'bsel', 272, 180, -1) } });''')
screens["Browser"]["JS"] = screens["Home"]["JS"].replace("const extra =", "const homeExtra =") + screens["Browser"]["JS"]

# ---------- Home at other window sizes ----------
# The display stays about half the window. What gives way first: the Launchkey mirror,
# then the art column (it folds into the status line), then the band-effects column.
SIZES = {
    "1440": dict(W=1440, H=900, DISPH=400, KEYH=104, NW=36, COMPACT=0, WIDE=0),
    "1280": dict(W=1280, H=800, DISPH=360, KEYH=84, NW=36, COMPACT=0, WIDE=0),
    "1024": dict(W=1024, H=768, DISPH=350, KEYH=80, NW=29, COMPACT=1, WIDE=0),
    "1920": dict(W=1920, H=1080, DISPH=520, KEYH=130, NW=52, COMPACT=0, WIDE=1),
}
screens["Home1280"] = dict(home(fxW=240), SIZE="1280")
screens["Home1024"] = dict(home(fxW=0), SIZE="1024")
screens["Home1920"] = dict(home(fxW=300, wide=True), SIZE="1920")

for _n in ("VoiceList", "Browser"):
    screens[_n].update(STRIP=screens["Home"]["STRIP"], STRIPH=screens["Home"]["STRIPH"])

TITLES = {"Home": "B · Home", "Channel": "B · Channel (selected track)", "Effects": "B · Effects", "PadsLoops": "B · Pads & Loops", "VoiceList": "B · Voice quick list", "Browser": "B · Browser",
          "Home1280": "B · Home at 1280×800", "Home1024": "B · Home at 1024×768", "Home1920": "B · Home at 1920×1080"}

os.makedirs(os.path.join(D, "project"), exist_ok=True)
for name in TITLES:
    sc = screens[name]
    size = SIZES[sc.get("SIZE", "1440")]
    out = shell
    for k, v in size.items():
        out = out.replace("%%" + k + "%%", str(v))
    out = out.replace("%%TITLE%%", TITLES[name]).replace("%%LAYER%%", sc.get("LAYER", "VOL")).replace("%%PADRING%%", sc.get("PADRING", "0")).replace("%%PADPAGE%%", sc.get("PADPAGE", "sections")).replace("%%STRIP%%", sc.get("STRIP", "")).replace("%%STRIPH%%", sc.get("STRIPH", "50"))
    out = out.replace("%%TAB%%", sc["TAB"]).replace("%%SEL%%", sc["SEL"]).replace("%%DISPLAY%%", sc["DISPLAY"]).replace("%%OVERLAY%%", sc.get("OVERLAY", "")).replace("%%BROWSEBTN%%", sc.get("BROWSEBTN", ""))
    out = out.replace("%%JS%%", sc["JS"]).replace("%%SONGMAP%%", SONGMAP_JS).replace("%%BGSVG%%", SONGMAP_SVG % {"v": "bgArt"})
    assert "%%" not in out, name
    open(os.path.join(D, "project", "B" + name + ".dc.html"), "w").write(out)

# ---------- Style artwork options (standalone sheet) ----------
exec(open(os.path.join(D, "art_options.py")).read())
print("ok")
