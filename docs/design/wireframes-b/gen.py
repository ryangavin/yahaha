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
<svg width="%d" height="%d" viewBox="0 0 %d %d" aria-hidden="true"><path d="{{k.track}}" stroke="#2d2d32" stroke-width="5" fill="none" stroke-linecap="round"></path><path d="{{k.arc}}" stroke="{{k.col}}" stroke-width="5" fill="none" stroke-linecap="round"></path></svg>
<span style="font-size: 13px; font-weight: 700">{{k.val}}</span>
<span class="cap" style="font-size: 10px">{{k.name}}</span>
</div>
</sc-for>'''
def krow(var, size=52, w=70):
    return KNOBROW % (var, w, size, size, size, size)

screens = {}

# ---------- Home ----------
# The sections are the Launchkey Sections pad page, drawn big: same order, colours and
# lights as the hardware. The Mains carry their real pattern (a row per drum/bass voice).
def home(artW=200, fxW=220, wide=False):
    art = '' if not artW else '''<aside aria-label="Style" style="width: %dpx; flex-shrink: 0; display: flex; flex-direction: column; gap: 6px; padding: 14px; border-right: 1px solid #26262b; min-height: 0">
<svg width="%d" height="%d" viewBox="0 0 160 160" aria-hidden="true" style="flex-shrink: 0; border-radius: 8px; background: #15151a"><sc-for list="{{print}}" as="c" hint-placeholder-count="40"><rect x="{{c.x}}" y="{{c.y}}" width="{{c.w}}" height="{{c.h}}" rx="1.5" fill="{{c.col}}" opacity="{{c.o}}"></rect></sc-for></svg>
<span class="cap" style="margin-top: 2px">Pop &amp; Rock · 4/4</span>
<div style="font-family: 'Archivo Narrow', sans-serif; font-size: %dpx; font-weight: 700; line-height: 1">Cool 8Beat</div>
<span style="font-size: 12px; color: #c9c9cf; white-space: nowrap; overflow: hidden; text-overflow: ellipsis"><b style="color: #ff7a2f">Regist 3</b> · Sunday Gig</span>
<span style="font-size: 12px; color: #c9c9cf; white-space: nowrap; overflow: hidden; text-overflow: ellipsis"><b style="color: #ff7a2f">OTS 2</b> · Piano &amp; Strings</span>
<div style="flex-grow: 1"></div>
<div style="display: flex; gap: 6px"><button class="chip" style="flex: 1">Browse</button><button class="chip" style="flex: 1">Edit…</button></div>
</aside>''' % (artW, artW - 28, artW - 28, 32 if wide else 26)
    # With no art column (narrow window) the style, registration and OTS move into the status line.
    mini = '' if artW else '''<div style="display: flex; gap: 10px; align-items: center; min-width: 0">
<svg width="46" height="46" viewBox="0 0 160 160" aria-hidden="true" style="flex-shrink: 0; border-radius: 6px; background: #15151a"><sc-for list="{{print}}" as="c" hint-placeholder-count="40"><rect x="{{c.x}}" y="{{c.y}}" width="{{c.w}}" height="{{c.h}}" fill="{{c.col}}" opacity="{{c.o}}"></rect></sc-for></svg>
<div style="display: flex; flex-direction: column; gap: 2px; min-width: 0"><span style="font-family: 'Archivo Narrow', sans-serif; font-size: 22px; font-weight: 700; line-height: 1; white-space: nowrap">Cool 8Beat</span><span style="font-size: 11px; color: #c9c9cf; white-space: nowrap"><b style="color: #ff7a2f">Regist 3</b> Sunday Gig · <b style="color: #ff7a2f">OTS 2</b></span></div>
</div>'''
    fx = '' if not fxW else '''<aside aria-label="Band effects" style="width: %dpx; flex-shrink: 0; display: flex; flex-direction: column; gap: 10px; padding: 14px; border-left: 1px solid #26262b">
<div style="display: flex; justify-content: space-between; align-items: baseline"><span class="cap">Band effects</span><button class="chip" style="height: 24px; padding: 0 8px; font-size: 11px">Effects ›</button></div>
<sc-for list="{{bandFx}}" as="f" hint-placeholder-count="3">
<div style="display: flex; flex-direction: column; gap: 5px">
<div style="display: flex; justify-content: space-between; align-items: center"><span style="font-size: 14px; font-weight: 700">{{f.name}}</span><span style="font-size: 11px; color: #8d8d95">{{f.type}}</span></div>
<div style="display: flex; align-items: center; gap: 8px"><div style="flex-grow: 1; height: 10px; border-radius: 5px; background: #26262b; position: relative"><div style="position: absolute; left: 0; top: 0; bottom: 0; width: {{f.pct}}%%; border-radius: 5px; background: #ff7a2f"></div><div style="position: absolute; top: -3px; left: {{f.pct}}%%; width: 4px; height: 16px; margin-left: -2px; border-radius: 2px; background: #f2f2f2"></div></div><span style="width: 36px; text-align: right; font-size: 12px; font-weight: 700">{{f.val}}</span></div>
</div>
</sc-for>
<div style="flex-grow: 1"></div>
<span style="font-size: 11px; color: #8d8d95; line-height: 1.35">How much the whole band feeds each effect. Each part's own send: the mixer's REV, CHO and DLY layers.</span>
</aside>''' % fxW
    return dict(TAB="Home", SEL="-1", DISPLAY=art + '''
<div style="flex-grow: 1; min-width: 0; display: flex; flex-direction: column; gap: 10px; padding: 12px 14px">
<div style="display: flex; align-items: flex-end; gap: 24px">''' + mini + '''
<div style="display: flex; flex-direction: column; gap: 2px"><span class="cap">Playing</span><span style="font-family: 'Archivo Narrow', sans-serif; font-size: 32px; font-weight: 700; line-height: 1; white-space: nowrap">Main B</span></div>
<div style="display: flex; flex-direction: column; gap: 2px"><span class="cap">Next</span><span style="font-family: 'Archivo Narrow', sans-serif; font-size: 32px; font-weight: 700; line-height: 1; color: #ff7a2f; white-space: nowrap">Fill B</span></div>
<div style="flex-grow: 1; min-width: 60px; display: flex; flex-direction: column; gap: 6px; padding-bottom: 4px"><span class="cap">Bar 3 of 4</span><div style="height: 8px; border-radius: 4px; background: #26262b; position: relative"><div style="position: absolute; left: 0; top: 0; bottom: 0; width: 62%; border-radius: 4px; background: #ff7a2f"></div></div></div>
<div style="display: flex; flex-direction: column; gap: 2px; align-items: flex-end"><span class="cap">Chord</span><span style="font-family: 'Archivo Narrow', sans-serif; font-size: 50px; font-weight: 700; line-height: .9; white-space: nowrap">Am<span style="font-size: 28px; color: #8d8d95">/G</span></span></div>
</div>
<div role="group" aria-label="Sections (the Launchkey pads)" style="flex-grow: 1; min-height: 0; display: grid; grid-template-columns: repeat(8, minmax(0, 1fr)); grid-template-rows: minmax(0, 1fr) minmax(0, 1.35fr); gap: 7px">
<sc-for list="{{homePads}}" as="d" hint-placeholder-count="16">
<button title="{{d.tip}}" style="position: relative; overflow: hidden; min-height: 0; border-radius: 8px; border: 1px solid {{d.border}}; outline: {{d.outline}}; outline-offset: 2px; background: {{d.bg}}; color: {{d.ink}}; box-shadow: {{d.glow}}; text-align: left; padding: 8px 10px; display: flex; flex-direction: column; gap: 4px">
<span style="font-family: 'Archivo Narrow', sans-serif; font-size: {{d.fs}}px; font-weight: 700; line-height: 1; white-space: nowrap">{{d.name}}</span>
<span style="font-size: 10px; font-weight: 600; opacity: .75">{{d.len}}</span>
<span style="flex-grow: 1"></span>
<svg width="100%" height="{{d.patH}}" viewBox="0 0 160 40" preserveAspectRatio="none" aria-hidden="true" style="display: {{d.patDisp}}"><sc-for list="{{d.pat}}" as="c" hint-placeholder-count="30"><rect x="{{c.x}}" y="{{c.y}}" width="8" height="8" rx="1.5" fill="{{d.patCol}}" opacity="{{c.o}}"></rect></sc-for></svg>
<span style="position: absolute; top: 7px; right: 7px; font-size: 9px; font-weight: 700; padding: 2px 5px; border-radius: 3px; background: #ffffff; color: #0e0e10; display: {{d.tagDisp}}">{{d.tag}}</span>
</button>
</sc-for>
</div>
<div style="display: flex; gap: 6px; align-items: center"><button class="chip on">OTS Link</button><button class="chip on">ACMP</button><button class="chip">Left Hold</button><button class="chip">Accent</button><div style="flex-grow: 1"></div><span class="cap" style="white-space: nowrap">AI Fingered · Split F#2</span></div>
</div>''' + fx, JS='''
    // Each Main's own pattern: kick, snare, hats, bass across its first bar (16ths).
    const PAT = { 8: ['x...x...x...x...', '....x.......x...', 'x.x.x.x.x.x.x.x.', 'x..x..x...x..x..'], 9: ['x.....x.x.......', '....x.......x..x', 'xxxxxxxxxxxxxxxx', 'x..x..x.x..x..x.'], 10: ['x..x..x...x.x...', '....x.......x...', 'x.xxx.xxx.xxx.xx', 'x...x.x...x...x.'], 11: ['x.x...x.x.x...x.', '....x..x....x..x', 'xxxxxxxxxxxxxxxx', 'x.xx..x.x.xx..x.'] };
    const TIP = ['Intro I: press to arm it for the start', '', '', 'Sync Start: the band starts on your first chord', 'Ending I', '', '', 'Auto Fill: a fill plays whenever you change Main', 'Main A: press while it plays for its fill', 'Main B: playing. Its fill is queued (it flashes on the Launchkey)', 'Main C', 'Main D', 'Break', 'Tap tempo', 'Sync Stop: the band stops when you let go', 'Start / Stop'];
    const homePads = secPads.map((d, i) => { const p = PAT[i]; const cells = [];
      if (p) p.forEach((row, y) => row.split('').forEach((c, x) => { if (c === 'x') cells.push({ x: x * 10 + 1, y: y * 10 + 1, o: y === 2 ? .55 : 1 }); }));
      return Object.assign({}, d, { tip: TIP[i] || d.name, fs: i >= 8 && i < 12 ? %d : %d, pat: cells, patDisp: p ? 'block' : 'none', patH: %d, patCol: d.ink, tag: i === 9 ? 'FILL ▸' : '', tagDisp: i === 9 ? 'block' : 'none' }); });
    const bandFx = [['Reverb', 'Hall 2', 100], ['Chorus', 'Chorus 1', 0], ['Delay', '1/8 dotted', 20]].map(f => ({ name: f[0], type: f[1], pct: Math.round(f[2] / 127 * 100), val: f[2] + '%%' }));
    const print = printFor(7, 160, 160, 16);
    const extra = { homePads, bandFx, print };''' % ((30, 20, 44) if wide else (24, 16, 34)), OVERLAY="", LAYER="VOL")

screens["Home"] = home()

# ---------- Channel ----------
screens["Channel"] = dict(TAB="Channel", SEL="7", DISPLAY='''
<div style="width: 280px; flex-shrink: 0; display: flex; flex-direction: column; gap: 10px; padding: 16px; background: linear-gradient(180deg, rgba(196,107,255,.22), rgba(196,107,255,0) 60%)">
<div style="display: flex; justify-content: space-between; align-items: center"><button class="chip" aria-label="Previous part">‹ Bass</button><button class="chip" aria-label="Next part">Chord 2 ›</button></div>
<span class="cap" style="color: #c46bff">Style part · ch 12</span>
<div style="font-family: 'Archivo Narrow', sans-serif; font-size: 40px; font-weight: 700; line-height: 1">Chord 1</div>
<button style="height: 44px; border-radius: 6px; border: 1px solid #c46bff; background: #1b1b20; color: #f2f2f2; font-size: 15px; font-weight: 700; text-align: left; padding: 0 12px">Steel Gtr ▾<span style="display: block; font-size: 10px; font-weight: 500; color: #8d8d95">SoundFont · from Yamaha 8/1/2</span></button>
<div style="display: flex; gap: 6px"><button class="chip on">On</button><button class="chip">Mute</button><button class="chip">Solo</button></div>
<div style="flex-grow: 1"></div>
<span style="font-size: 12px; color: #8d8d95">Select any strip below, or turn the Launchkey's page buttons, to edit that part here.</span>
</div>
<div style="flex-grow: 1; display: grid; grid-template-columns: 170px repeat(3, minmax(0, 1fr)); gap: 10px; padding: 14px">
<div style="display: flex; flex-direction: column; gap: 8px; padding: 12px; border-radius: 8px; background: #19191c"><span class="cap">Level</span>
<div style="flex-grow: 1; display: flex; gap: 14px; justify-content: center">
<div style="width: 8px; border-radius: 3px; background: #2d2d32; position: relative"><div style="position: absolute; left: 0; right: 0; bottom: 0; height: 58%; border-radius: 3px; background: #c46bff"></div></div>
<div style="width: 34px; position: relative"><div style="position: absolute; left: 16px; top: 0; bottom: 0; width: 3px; background: #2d2d32"></div><div style="position: absolute; left: 0; bottom: 64%; width: 34px; height: 16px; border-radius: 3px; background: #f2f2f2"></div></div>
</div>
<span style="font-size: 20px; font-weight: 700; text-align: center">82</span>
<div style="display: flex; justify-content: center">''' + krow("pan", 44, 60) + '''</div>
</div>
<div style="display: flex; flex-direction: column; gap: 8px; padding: 12px; border-radius: 8px; background: #19191c"><span class="cap">Effect sends</span>
<div style="display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; justify-items: center">''' + krow("sends") + '''</div>
<span style="font-size: 11px; color: #8d8d95">Band sends are scaled by Effects › Band.</span>
</div>
<div style="display: flex; flex-direction: column; gap: 8px; padding: 12px; border-radius: 8px; background: #19191c"><span class="cap">Tone · from OTS / voice</span>
<div style="display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 8px; justify-items: center">''' + krow("tone", 46, 64) + '''</div>
</div>
<div style="display: flex; flex-direction: column; gap: 10px; padding: 12px; border-radius: 8px; background: #19191c"><span class="cap">Play</span>
<div style="display: flex; align-items: center; justify-content: space-between"><span style="font-size: 13px">Octave</span><div style="display: flex; gap: 4px; align-items: center"><button class="chip">−</button><span style="width: 26px; text-align: center; font-weight: 700">0</span><button class="chip">+</button></div></div>
<div style="display: flex; align-items: center; justify-content: space-between"><span style="font-size: 13px">Mono / Poly</span><div style="display: flex; gap: 4px"><button class="chip">Mono</button><button class="chip on">Poly</button></div></div>
<div style="display: flex; align-items: center; justify-content: space-between"><span style="font-size: 13px">Bend range</span><div style="display: flex; gap: 4px; align-items: center"><button class="chip">−</button><span style="width: 26px; text-align: center; font-weight: 700">2</span><button class="chip">+</button></div></div>
<div style="display: flex; justify-content: center">''' + krow("porta", 44, 70) + '''</div>
</div>
</div>''', JS='''
    const C = '#c46bff';
    const K = (v, name, val) => Object.assign(knob(52, v, C), { name, val });
    const extra = { pan: [Object.assign(knob(44, .5, C), { name: 'Pan', val: 'C' })], sends: [K(.35, 'Reverb', '45'), K(.1, 'Chorus', '12'), K(0, 'Delay', '0'), K(.8, 'Dry', '100')], tone: [Object.assign(knob(46, .7, C), { name: 'Cutoff', val: '+10' }), Object.assign(knob(46, .5, C), { name: 'Reso', val: '0' }), Object.assign(knob(46, .45, C), { name: 'Attack', val: '−4' }), Object.assign(knob(46, .5, C), { name: 'Decay', val: '0' }), Object.assign(knob(46, .55, C), { name: 'Release', val: '+6' }), Object.assign(knob(46, .5, C), { name: 'Vibrato', val: '0' })], porta: [Object.assign(knob(44, 0, C), { name: 'Portamento', val: 'Off' })] };''', OVERLAY="")

# ---------- Effects ----------
def fxcard(name, var, types, extra_html=""):
    return '''<div style="flex: 1; display: flex; flex-direction: column; gap: 10px; padding: 12px; border-radius: 8px; background: #19191c">
<div style="display: flex; align-items: center; justify-content: space-between"><span style="font-family: 'Archivo Narrow', sans-serif; font-size: 24px; font-weight: 700">%s</span><div style="display: flex; gap: 4px"><button class="chip">From style: %s</button><button class="chip on">Mine</button></div></div>
<div style="display: flex; gap: 4px; flex-wrap: wrap">%s</div>
<div style="display: flex; justify-content: space-around">%s</div>
%s
<div style="display: flex; flex-direction: column; gap: 4px; margin-top: auto"><div style="display: flex; justify-content: space-between"><span class="cap">Band send</span><span style="font-size: 12px; font-weight: 700">{{%s.band}}</span></div><div style="height: 8px; border-radius: 4px; background: #26262b; position: relative"><div style="position: absolute; left: 0; top: 0; bottom: 0; width: {{%s.bandPct}}%%; border-radius: 4px; background: #ff7a2f"></div></div><span style="font-size: 11px; color: #8d8d95">How much the band's parts feed this effect. Your keyboard parts use their own sends.</span></div>
</div>''' % (name, types[0], "".join('<button class="chip%s">%s</button>' % (" on" if i == 0 else "", t) for i, t in enumerate(types[1:])), krow(var + ".knobs", 50, 64), extra_html, var, var)
screens["Effects"] = dict(TAB="Effects", SEL="-1", DISPLAY='''<div style="flex-grow: 1; display: flex; gap: 10px; padding: 14px">''' +
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
<div style="display: flex; flex-direction: column; gap: 4px; margin-top: 8px"><div style="display: flex; justify-content: space-between"><span class="cap">Pad level (fader 6)</span><span style="font-size: 12px; font-weight: 700">85%</span></div><div style="height: 8px; border-radius: 4px; background: #26262b; position: relative"><div style="position: absolute; left: 0; top: 0; bottom: 0; width: 67%; border-radius: 4px; background: #ff7a2f"></div></div></div>
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
<div style="display: flex; gap: 8px"><button class="hb" style="height: 48px; flex: 1; background: #ff5a5a; border-color: #ff5a5a; color: #0e0e10">● Rec / Stop</button><button class="hb" style="height: 48px; flex: 1; background: #ff7a2f; border-color: #ff7a2f; color: #1a0a00">Loop: On</button><button class="hb" style="height: 48px; flex: 1">Clear</button></div>
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
    const bars = CH.map((c, i) => ({ n: i + 1, ch: c, bg: i === 9 ? 'rgba(255,122,47,.18)' : (i % 8 === 0 ? '#1b1b20' : '#141417'), border: i === 9 ? A : '#26262b', ink: i === 9 ? A : '#f2f2f2' }));
    const extra = { mem, seq, bars };''', OVERLAY="")

# ---------- Voice quick list ----------
screens["VoiceList"] = dict(TAB="Home", SEL="7", DISPLAY=screens["Home"]["DISPLAY"], JS=screens["Home"]["JS"], OVERLAY='''
<div role="dialog" aria-label="Voices for Chord 1" style="position: absolute; left: 600px; top: 250px; width: 300px; padding: 8px; border-radius: 8px; background: #1f1f24; border: 1px solid #c46bff; box-shadow: 0 18px 40px rgba(0,0,0,.6); display: flex; flex-direction: column; gap: 2px">
<div style="display: flex; justify-content: space-between; padding: 4px 8px"><span class="cap" style="color: #c46bff">Chord 1 · Guitars</span><span class="cap">Esc</span></div>
<sc-for list="{{vl}}" as="v" hint-placeholder-count="8"><button style="height: 32px; border: 0; border-radius: 4px; background: {{v.bg}}; color: #f2f2f2; text-align: left; padding: 0 10px; font-size: 13px; font-weight: {{v.w}}; display: flex; justify-content: space-between; align-items: center">{{v.name}}<span style="font-size: 10px; color: #8d8d95">{{v.src}}</span></button></sc-for>
<div style="height: 1px; background: #2d2d32; margin: 4px 0"></div>
<button style="height: 34px; border: 0; border-radius: 4px; background: #26262b; color: #ff7a2f; text-align: left; padding: 0 10px; font-size: 13px; font-weight: 700">More in the Browser…</button>
</div>''')
screens["VoiceList"]["JS"] = screens["Home"]["JS"].replace("const extra = { homePads, bandFx, print };", "const vl = [['Steel Gtr','current'],['Nylon Gtr','SoundFont'],['Clean Gtr','SoundFont'],['Jazz Gtr','SoundFont'],['12-String','SoundFont'],['★ Ample Guitar M','plugin'],['★ My Strum Gtr','patch'],['Muted Gtr','SoundFont']].map((v, i) => ({ name: v[0], src: v[1], bg: i === 0 ? '#2c2536' : 'transparent', w: i === 0 ? 700 : 500 }));\n    const extra = { homePads, bandFx, print, vl };")

# ---------- Browser (full screen) ----------
screens["Browser"] = dict(TAB="Home", SEL="-1", DISPLAY=screens["Home"]["DISPLAY"], BROWSEBTN="background: #ff7a2f; color: #1a0a00; border-color: #ff7a2f", OVERLAY='''
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
<svg width="100%" height="110" viewBox="0 0 200 110" preserveAspectRatio="xMidYMid slice" aria-hidden="true" style="display: block; background: #15151a"><sc-for list="{{s.art.bars}}" as="b" hint-placeholder-count="10"><rect x="{{b.x}}" y="{{b.y}}" width="{{b.w}}" height="{{b.h}}" fill="{{b.col}}" opacity="{{b.o}}"></rect></sc-for><sc-for list="{{s.art.rings}}" as="c" hint-placeholder-count="6"><circle cx="{{c.x}}" cy="{{c.y}}" r="{{c.r}}" fill="none" stroke="{{c.col}}" stroke-width="{{c.sw}}" opacity="{{c.o}}"></circle></sc-for></svg>
<span style="padding: 8px 10px 2px; font-size: 14px; font-weight: 700">{{s.name}}</span>
<span style="padding: 0 10px 10px; font-size: 11px; color: #8d8d95">{{s.meta}}</span>
</button>
</sc-for>
</div>
</div>
<aside style="width: 300px; flex-shrink: 0; padding: 14px; display: flex; flex-direction: column; gap: 10px; border-left: 1px solid #26262b">
<svg width="272" height="180" viewBox="0 0 200 132" preserveAspectRatio="xMidYMid slice" aria-hidden="true" style="border-radius: 8px; background: #15151a"><sc-for list="{{sel.art.bars}}" as="b" hint-placeholder-count="10"><rect x="{{b.x}}" y="{{b.y}}" width="{{b.w}}" height="{{b.h}}" fill="{{b.col}}" opacity="{{b.o}}"></rect></sc-for><sc-for list="{{sel.art.rings}}" as="c" hint-placeholder-count="6"><circle cx="{{c.x}}" cy="{{c.y}}" r="{{c.r}}" fill="none" stroke="{{c.col}}" stroke-width="{{c.sw}}" opacity="{{c.o}}"></circle></sc-for></svg>
<div style="font-family: 'Archivo Narrow', sans-serif; font-size: 28px; font-weight: 700; line-height: 1">Funky Pop</div>
<span class="cap">Dance · 4/4 · 118 bpm · SFF2</span>
<span style="font-size: 12px; color: #8d8d95">Main A–D, 3 Intros, 3 Endings · 4 One Touch Settings · Chord guitars on Yamaha Mega Voices</span>
<div style="display: flex; gap: 6px"><button class="hb" style="flex: 1">▶ Preview</button><button class="hb" style="flex: 1; background: #ff7a2f; color: #1a0a00; border-color: #ff7a2f">Load</button></div>
<div style="display: flex; gap: 6px"><button class="chip" style="flex: 1">★ Favourite</button><button class="chip" style="flex: 1">Edit a copy…</button></div>
<div style="flex-grow: 1"></div>
<span style="font-size: 11px; color: #8d8d95">Tip: while the style plays, Load waits for the end of the bar (or the Ending) like the Genos.</span>
</aside>
</div>''', JS='''
    const CATS = [['All', 208], ['Favourites', 12], ['Pop & Rock', 46], ['Ballad', 19], ['Dance', 24], ['R&B', 17], ['Swing & Jazz', 21], ['Latin', 18], ['Country', 14], ['Ballroom', 11], ['Entertainer', 9], ['World', 9]];
    const cats = CATS.map((c, i) => ({ name: c[0], n: c[1], col: COL[i % 12], bg: i === 4 ? '#222228' : 'transparent', ink: i === 4 ? '#ffffff' : '#c9c9cf' }));
    const S = [['Funky Pop', 'Dance · 118'], ['Disco Fever', 'Dance · 124'], ['Synth Pop', 'Dance · 124'], ['Club House', 'Dance · 126'], ['Dance Pop 2', 'Dance · 120'], ['Euro Beat', 'Dance · 140'], ['Nu Disco', 'Dance · 116'], ['Future Bass', 'Dance · 150'], ['Tropical', 'Dance · 102'], ['Electro Swing', 'Dance · 128'], ['Deep House', 'Dance · 122'], ['80s Dance', 'Dance · 120'], ['Funky Finger', 'Dance · 112'], ['Trance Pop', 'Dance · 132'], ['Latin House', 'Dance · 124']];
    const cards = S.map((s, i) => ({ name: s[0], meta: s[1] + ' bpm', border: i === 0 ? A : 'transparent', art: artFor(11 + i * 5, 200, 110, 6) }));
    const extra = Object.assign(homeExtra, { cats, cards, sel: { art: artFor(11, 200, 132, 7) } });''')
screens["Browser"]["JS"] = screens["Home"]["JS"].replace("const extra =", "const homeExtra =") + screens["Browser"]["JS"]

# ---------- Home at other window sizes ----------
# The display stays about half the window. What gives way first: the Launchkey mirror,
# then the art column (it folds into the status line), then the band-effects column.
SIZES = {
    "1440": dict(W=1440, H=900, DISPH=400, MIRW=350, MIRDISP="flex", KEYH=104, NW=36, COMPACT=0, WIDE=0),
    "1280": dict(W=1280, H=800, DISPH=360, MIRW=270, MIRDISP="flex", KEYH=84, NW=36, COMPACT=0, WIDE=0),
    "1024": dict(W=1024, H=768, DISPH=350, MIRW=0, MIRDISP="none", KEYH=80, NW=29, COMPACT=1, WIDE=0),
    "1920": dict(W=1920, H=1080, DISPH=520, MIRW=440, MIRDISP="flex", KEYH=130, NW=52, COMPACT=0, WIDE=1),
}
screens["Home1280"] = dict(home(artW=180, fxW=200), SIZE="1280")
screens["Home1024"] = dict(home(artW=0, fxW=0), SIZE="1024")
screens["Home1920"] = dict(home(artW=300, fxW=300, wide=True), SIZE="1920")

TITLES = {"Home": "B · Home", "Channel": "B · Channel (selected track)", "Effects": "B · Effects", "MultiPads": "B · Multi Pads", "Looper": "B · Looper & Charts", "VoiceList": "B · Voice quick list", "Browser": "B · Browser",
          "Home1280": "B · Home at 1280×800", "Home1024": "B · Home at 1024×768", "Home1920": "B · Home at 1920×1080"}

os.makedirs(os.path.join(D, "project"), exist_ok=True)
for name in TITLES:
    sc = screens[name]
    size = SIZES[sc.get("SIZE", "1440")]
    out = shell
    for k, v in size.items():
        out = out.replace("%%" + k + "%%", str(v))
    out = out.replace("%%TITLE%%", TITLES[name]).replace("%%LAYER%%", sc.get("LAYER", "VOL"))
    out = out.replace("%%TAB%%", sc["TAB"]).replace("%%SEL%%", sc["SEL"]).replace("%%DISPLAY%%", sc["DISPLAY"]).replace("%%OVERLAY%%", sc.get("OVERLAY", "")).replace("%%BROWSEBTN%%", sc.get("BROWSEBTN", ""))
    out = out.replace("%%JS%%", sc["JS"])
    assert "%%" not in out, name
    open(os.path.join(D, "project", "B" + name + ".dc.html"), "w").write(out)

# ---------- Style artwork options (standalone sheet) ----------
exec(open(os.path.join(D, "art_options.py")).read())
print("ok")
