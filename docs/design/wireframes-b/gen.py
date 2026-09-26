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
screens["Home"] = dict(TAB="Home", SEL="-1", DISPLAY=ART + '''
<div style="flex-grow: 1; display: flex; flex-direction: column; gap: 12px; padding: 14px 16px">
<div style="display: flex; align-items: flex-end; gap: 28px">
<div style="display: flex; flex-direction: column; gap: 2px"><span class="cap">Playing</span><span style="font-family: 'Archivo Narrow', sans-serif; font-size: 38px; font-weight: 700; line-height: 1">Main B</span></div>
<div style="display: flex; flex-direction: column; gap: 2px"><span class="cap">Next</span><span style="font-family: 'Archivo Narrow', sans-serif; font-size: 38px; font-weight: 700; line-height: 1; color: #ff7a2f">Fill B</span></div>
<div style="flex-grow: 1; display: flex; flex-direction: column; gap: 6px; padding-bottom: 4px"><span class="cap">Bar 3 of 4</span><div style="height: 8px; border-radius: 4px; background: #26262b; position: relative"><div style="position: absolute; left: 0; top: 0; bottom: 0; width: 62%; border-radius: 4px; background: #ff7a2f"></div></div></div>
<div style="display: flex; flex-direction: column; gap: 2px; align-items: flex-end"><span class="cap">Chord</span><span style="font-family: 'Archivo Narrow', sans-serif; font-size: 58px; font-weight: 700; line-height: .9">Am<span style="font-size: 32px; color: #8d8d95">/G</span></span></div>
</div>
<div style="flex-grow: 1; display: flex; gap: 12px; min-height: 0">
<div style="display: flex; flex-direction: column; gap: 6px; width: 120px"><span class="cap">Intro</span>
<sc-for list="{{intros}}" as="s" hint-placeholder-count="3"><button style="flex: 1; border-radius: 6px; border: 1px solid {{s.border}}; background: {{s.bg}}; color: {{s.ink}}; font-size: 15px; font-weight: 700; display: flex; align-items: center; justify-content: space-between; padding: 0 12px">{{s.label}}<span style="font-size: 10px; font-weight: 600; opacity: .7">{{s.len}}</span></button></sc-for>
</div>
<div style="flex-grow: 1; display: flex; flex-direction: column; gap: 6px"><span class="cap">Main · tap again for its fill</span>
<div style="flex-grow: 1; display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 8px">
<sc-for list="{{mains}}" as="m" hint-placeholder-count="4">
<div style="display: flex; flex-direction: column; gap: 6px">
<button style="flex-grow: 1; position: relative; overflow: hidden; border-radius: 8px; border: 2px solid {{m.border}}; background: {{m.bg}}; color: {{m.ink}}; text-align: left; padding: 10px 12px; display: flex; flex-direction: column; justify-content: space-between; box-shadow: {{m.glow}}">
<span style="font-family: 'Archivo Narrow', sans-serif; font-size: 40px; font-weight: 700; line-height: 1">{{m.label}}</span>
<span style="display: flex; gap: 2px; align-items: flex-end; height: 34px"><sc-for list="{{m.thumb}}" as="t" hint-placeholder-count="12"><span style="flex: 1; height: {{t}}%; border-radius: 1px; background: {{m.thumbCol}}"></span></sc-for></span>
<span style="font-size: 11px; font-weight: 600; opacity: .8">{{m.len}}</span>
</button>
<button style="height: 34px; border-radius: 6px; border: 1px solid {{m.fBorder}}; background: {{m.fBg}}; color: {{m.fInk}}; font-size: 12px; font-weight: 700">Fill {{m.label}}</button>
</div>
</sc-for>
</div>
</div>
<div style="display: flex; flex-direction: column; gap: 6px; width: 110px"><span class="cap">Break</span><button style="flex-grow: 1; border-radius: 8px; border: 1px solid #2d2d32; background: #1b1b20; color: #f2f2f2; font-size: 16px; font-weight: 700">Break<br><span style="font-size: 10px; opacity: .7">1 bar</span></button></div>
<div style="display: flex; flex-direction: column; gap: 6px; width: 120px"><span class="cap">Ending</span>
<sc-for list="{{endings}}" as="s" hint-placeholder-count="3"><button style="flex: 1; border-radius: 6px; border: 1px solid {{s.border}}; background: {{s.bg}}; color: {{s.ink}}; font-size: 15px; font-weight: 700; display: flex; align-items: center; justify-content: space-between; padding: 0 12px">{{s.label}}<span style="font-size: 10px; font-weight: 600; opacity: .7">{{s.len}}</span></button></sc-for>
</div>
</div>
<div style="display: flex; gap: 6px"><button class="chip on">Auto Fill</button><button class="chip on">OTS Link</button><button class="chip">Sync Start</button><button class="chip on">ACMP</button><button class="chip">Left Hold</button><button class="chip">Accent</button><div style="flex-grow: 1"></div><span class="cap" style="align-self: center">Fingering: AI Fingered · Split: F#2</span></div>
</div>''', JS='''
    const sec = (label, len, st) => ({ label, len, bg: st === 'on' ? A : '#1b1b20', ink: st === 'on' ? AI : '#f2f2f2', border: st === 'on' ? A : '#2d2d32' });
    const intros = [sec('I', '4 bars'), sec('II', '8 bars'), sec('III', '2 bars')];
    const endings = [sec('I', '2 bars'), sec('II', '4 bars'), sec('III', '4 bars')];
    let s2 = 3; const r2 = () => { s2 = (s2 * 9301 + 49297) % 233280; return s2 / 233280; };
    const mains = ['A', 'B', 'C', 'D'].map((l, i) => { const on = i === 1; const c = COL[i * 3]; return { label: l, len: ['4 bars', '4 bars', '8 bars', '8 bars'][i], bg: on ? A : '#1b1b20', ink: on ? AI : '#f2f2f2', border: on ? A : c, glow: on ? '0 0 22px rgba(255,122,47,.45)' : 'none', thumbCol: on ? 'rgba(26,10,0,.55)' : c, thumb: Array.from({ length: 16 }, () => Math.round(20 + r2() * 80)), fBg: on ? 'rgba(255,122,47,.14)' : '#141417', fInk: on ? A : '#8d8d95', fBorder: on ? A : '#2d2d32' }; });
    const extra = { intros, mains, endings };''', OVERLAY="")

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
  '</div>', JS='''
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
screens["VoiceList"]["JS"] = screens["Home"]["JS"].replace("const extra = { intros, mains, endings };", "const vl = [['Steel Gtr','current'],['Nylon Gtr','SoundFont'],['Clean Gtr','SoundFont'],['Jazz Gtr','SoundFont'],['12-String','SoundFont'],['★ Ample Guitar M','plugin'],['★ My Strum Gtr','patch'],['Muted Gtr','SoundFont']].map((v, i) => ({ name: v[0], src: v[1], bg: i === 0 ? '#2c2536' : 'transparent', w: i === 0 ? 700 : 500 }));\n    const extra = { intros, mains, endings, vl };")

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
    const extra = { cats, cards, sel: { art: artFor(11, 200, 132, 7) } };''')

os.makedirs(os.path.join(D, "project"), exist_ok=True)
order = ["Home", "Channel", "Effects", "MultiPads", "Looper", "VoiceList", "Browser"]
for name in order:
    sc = screens[name]
    out = shell
    out = out.replace("%%TITLE%%", {"Home": "B · Home", "Channel": "B · Channel (selected track)", "Effects": "B · Effects", "MultiPads": "B · Multi Pads", "Looper": "B · Looper & Charts", "VoiceList": "B · Voice quick list", "Browser": "B · Browser"}[name])
    out = out.replace("%%TAB%%", sc["TAB"]).replace("%%SEL%%", sc["SEL"]).replace("%%DISPLAY%%", sc["DISPLAY"]).replace("%%OVERLAY%%", sc.get("OVERLAY", "")).replace("%%BROWSEBTN%%", sc.get("BROWSEBTN", ""))
    out = out.replace("%%JS%%", sc["JS"])
    assert "%%" not in out.replace("%;", "").replace("100%", ""), name
    open(os.path.join(D, "project", "B" + name + ".dc.html"), "w").write(out)
print("ok")
