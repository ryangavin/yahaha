# Home, built in the mixer's language: dark panels (#19191c, 1px edge, 8px radius), a colour
# bar on top of each pad like each strip, caps labels, small knobs with caps names. The style's
# artwork lives in the now-playing band at the top. Run by gen.py (exec), which provides
# SONGMAP_SVG. Tokens like @FXW@ are filled with str.replace, so the markup keeps its {{holes}}.

PANEL = "border-radius: 8px; border: 1px solid #2d2d32; background: #19191c"

KNOBS = '''<sc-for list="{{@VAR@}}" as="k" hint-placeholder-count="8"><div title="{{k.tip}}" style="display: flex; flex-direction: column; align-items: center; gap: 2px; opacity: {{k.op}}"><svg width="28" height="28" viewBox="0 0 22 22" aria-hidden="true"><path d="{{k.track}}" stroke="#2d2d32" stroke-width="2.6" fill="none" stroke-linecap="round"></path><path d="{{k.arc}}" stroke="#f2f2f2" stroke-width="2.6" fill="none" stroke-linecap="round"></path></svg><span style="font-size: 8px; font-weight: 700; letter-spacing: .05em; color: #a9a9b1; white-space: nowrap">{{k.name}}</span></div></sc-for>'''

CHIP = 'height: 15px; padding: 0 5px; border-radius: 3px; font-size: 8px; font-weight: 700'

RIGHT = '''<div style="width: @FXW@px; flex-shrink: 0; display: flex; flex-direction: column; gap: 8px">
<section aria-label="Launchkey knobs" style="@PANEL@; padding: 22px 10px 10px; position: relative; outline: 2px solid #ffffff; outline-offset: -1px">
<div style="position: absolute; top: 5px; left: 10px; right: 10px; display: flex; align-items: center; gap: 3px"><span class="cap" style="font-size: 9px; color: #f2f2f2; margin-right: auto; white-space: nowrap">Knobs</span><sc-for list="{{knobPages}}" as="g" hint-placeholder-count="4"><button title="{{g.name}}" style="@CHIP@; border: 1px solid {{g.border}}; background: {{g.bg}}; color: {{g.ink}}">{{g.short}}</button></sc-for></div>
<div style="display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 6px 2px; justify-items: center">''' + KNOBS.replace("@VAR@", "lkKnobs") + '''</div>
</section>
<section aria-label="Master bus" style="@PANEL@; flex-grow: 1; min-height: 0; padding: 8px 10px; display: flex; flex-direction: column; gap: 6px">
<div style="display: flex; align-items: center"><span class="cap" style="font-size: 9px; margin-right: auto">Master bus</span><button class="hb" style="height: 20px; padding: 0 8px; font-size: 10px; background: #ffffff; color: #0e0e10; border-color: #ffffff">ON</button></div>
<div style="flex-grow: 1; display: grid; grid-template-columns: repeat(6, minmax(0, 1fr)); gap: 4px 0; justify-items: center; align-content: center">''' + KNOBS.replace("@VAR@", "master") + '''</div>
</section>
</div>'''

MAIN = '''
<div style="flex-grow: 1; min-width: 0; display: flex; flex-direction: column; gap: 8px; padding: 8px">
<section aria-label="Now playing" style="height: @BANDH@px; flex-shrink: 0; position: relative; overflow: hidden; @PANEL@; display: flex; align-items: center; gap: 28px; padding: 0 16px">
<svg width="100%" height="100%" viewBox="0 0 640 160" preserveAspectRatio="none" aria-hidden="true" style="position: absolute; inset: 0">@ART@</svg>
<div aria-hidden="true" style="position: absolute; inset: 0; background: linear-gradient(90deg, rgba(14,14,16,.85) 0%, rgba(14,14,16,.6) 45%, rgba(14,14,16,.25) 100%)"></div>
<div style="position: relative; display: flex; flex-direction: column; gap: 3px; min-width: 0">
<span class="cap" style="color: #c9c9cf">Pop &amp; Rock · 4/4</span>
<div style="display: flex; align-items: baseline; gap: 8px"><span style="font-family: 'Archivo Narrow', sans-serif; font-size: @NAMEFS@px; font-weight: 700; line-height: 1; white-space: nowrap">Cool 8Beat</span><button class="hb" style="height: 20px; padding: 0 8px; font-size: 10px">Edit…</button></div>
<span style="font-size: 11px; color: #c9c9cf; white-space: nowrap">Bank A · Snapshot 3 · OTS 2 Piano &amp; Strings</span>
</div>
<div style="position: relative; display: flex; flex-direction: column; gap: 2px"><span class="cap">Playing</span><span style="font-family: 'Archivo Narrow', sans-serif; font-size: 26px; font-weight: 700; line-height: 1; white-space: nowrap">Main B</span></div>
<div style="position: relative; display: flex; flex-direction: column; gap: 2px"><span class="cap">Next</span><span style="font-family: 'Archivo Narrow', sans-serif; font-size: 26px; font-weight: 700; line-height: 1; white-space: nowrap">Fill B</span></div>
<div aria-label="Bar 3, beat 2 of a 4-bar Main" style="position: relative; flex-grow: 1; min-width: 60px; display: flex; flex-direction: column; gap: 4px"><span class="cap">Bar 3 · Beat 2 <span style="color: #8d8d95">of 4 bars</span></span>
<div style="display: flex; gap: 10px; align-items: flex-end"><sc-for list="{{barsRow}}" as="b" hint-placeholder-count="4"><div style="flex: 1; display: flex; flex-direction: column; gap: 3px"><div style="display: flex; gap: 3px; align-items: flex-end"><sc-for list="{{b.beats}}" as="t" hint-placeholder-count="4"><span style="flex: 1; height: {{t.h}}px; border-radius: 2px; background: {{t.bg}}; box-shadow: {{t.glow}}"></span></sc-for></div><span style="font-size: 8px; font-weight: 700; color: {{b.ink}}">{{b.n}}</span></div></sc-for></div>
</div>
<div style="position: relative; display: flex; flex-direction: column; gap: 2px; align-items: flex-end"><span class="cap">Chord</span><span style="font-family: 'Archivo Narrow', sans-serif; font-size: @CHORDFS@px; font-weight: 700; line-height: .9; white-space: nowrap">Am<span style="font-size: 60%; color: #c9c9cf">/G</span></span></div>
</section>
<div style="flex-grow: 1; min-height: 0; display: flex; gap: 8px">
<section aria-label="The Launchkey: knobs over pads" style="flex-grow: 1; min-width: 0; @PANEL@; padding: 8px; position: relative; outline: 2px solid #ffffff; outline-offset: -1px; display: flex; flex-direction: column; gap: 7px">

<div style="flex-grow: 1; min-height: 0; display: grid; grid-template-columns: 62px repeat(8, minmax(0, 1fr)); grid-template-rows: 58px minmax(0, 1fr) minmax(0, 1fr); gap: 5px">
<div style="grid-row: 1; grid-column: 1; display: flex; flex-direction: column; align-items: stretch; justify-content: center; gap: 4px"><span class="cap" style="font-size: 8px; text-align: center">Knobs</span><div style="display: flex; gap: 3px"><button aria-label="Knob page up" title="Knob page ▲" style="flex: 1; height: 20px; padding: 0; border-radius: 3px; border: 1px solid #3a3a40; background: #26262b; color: #f2f2f2; font-size: 8px">▲</button><button aria-label="Knob page down" title="Knob page ▼" style="flex: 1; height: 20px; padding: 0; border-radius: 3px; border: 1px solid #3a3a40; background: #26262b; color: #f2f2f2; font-size: 8px">▼</button></div><button title="Pick a page" style="height: 22px; padding: 0 4px; border-radius: 3px; border: 1px solid #ffffff; background: #ffffff; color: #0e0e10; font-size: 9px; font-weight: 800; white-space: nowrap; overflow: hidden">STYLE ▾</button></div>
<div style="grid-row: 2 / span 2; grid-column: 1; display: flex; flex-direction: column; align-items: stretch; justify-content: center; gap: 4px"><span class="cap" style="font-size: 8px; text-align: center">Pads</span><div style="display: flex; gap: 3px"><button aria-label="Pad Bank up" title="Pad Bank ▲" style="flex: 1; height: 20px; padding: 0; border-radius: 3px; border: 1px solid #3a3a40; background: #26262b; color: #f2f2f2; font-size: 8px">▲</button><button aria-label="Pad Bank down" title="Pad Bank ▼" style="flex: 1; height: 20px; padding: 0; border-radius: 3px; border: 1px solid #3a3a40; background: #26262b; color: #f2f2f2; font-size: 8px">▼</button></div><button title="Pick a page" style="height: 22px; padding: 0 4px; border-radius: 3px; border: 1px solid #ffffff; background: #ffffff; color: #0e0e10; font-size: 9px; font-weight: 800; white-space: nowrap; overflow: hidden">SECTIONS ▾</button></div>
<sc-for list="{{lkKnobs}}" as="k" hint-placeholder-count="8"><div title="{{k.tip}}" style="display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 3px; opacity: {{k.op}}"><svg width="38" height="38" viewBox="0 0 22 22" aria-hidden="true"><path d="{{k.track}}" stroke="#2d2d32" stroke-width="2.6" fill="none" stroke-linecap="round"></path><path d="{{k.arc}}" stroke="#f2f2f2" stroke-width="2.6" fill="none" stroke-linecap="round"></path></svg><span style="font-size: 8px; font-weight: 700; letter-spacing: .05em; color: #a9a9b1; white-space: nowrap">{{k.name}}</span></div></sc-for>
<sc-for list="{{homePads}}" as="d" hint-placeholder-count="16">
<button title="{{d.tip}}" style="position: relative; overflow: hidden; min-height: 0; border-radius: 4px; border: 1px solid {{d.border}}; outline: {{d.outline}}; outline-offset: 1px; background: {{d.fill}}; color: {{d.ink}}; box-shadow: {{d.glow}}; text-align: left; padding: 7px 7px 5px; display: flex; flex-direction: column; gap: 2px">
<span style="display: flex; align-items: center; gap: 4px"><span style="font-size: {{d.fs}}px; font-weight: 700; line-height: 1.05; white-space: nowrap">{{d.name}}</span><span style="flex-grow: 1"></span><span title="{{d.otsTip}}" style="display: {{d.otsDisp}}; font-size: 8px; font-weight: 800; padding: 1px 4px; border-radius: 3px; border: 1px solid currentColor; background: {{d.otsBg}}; color: {{d.otsInk}}">{{d.ots}}</span></span>
<span style="font-size: 8px; font-weight: 700; letter-spacing: .05em; text-transform: uppercase; color: {{d.sub}}; white-space: nowrap">{{d.len}}</span>
<span style="flex-grow: 1"></span>
<span aria-label="Parts playing" style="display: {{d.partsDisp}}; height: 40%; min-height: 12px; align-items: flex-end; gap: 2px; padding: 2px; border-radius: 2px; background: {{d.well}}"><sc-for list="{{d.parts}}" as="q" hint-placeholder-count="8"><span title="{{q.tip}}" style="flex: 1; height: {{q.h}}%; border-radius: 1px; background: {{q.c}}; opacity: {{q.o}}"></span></sc-for></span>
</button>
</sc-for>
</div>
<div aria-label="Style parts" style="display: flex; align-items: center; gap: 12px; overflow: hidden"><span class="cap" style="font-size: 9px; white-space: nowrap">In this style</span><sc-for list="{{legend}}" as="q" hint-placeholder-count="8"><span style="display: flex; align-items: center; gap: 4px; white-space: nowrap; font-size: 10px; color: #c9c9cf"><span style="width: 8px; height: 8px; border-radius: 2px; background: {{q.c}}"></span>{{q.voice}}</span></sc-for></div>
</section>
@RIGHT@
</div>
</div>'''

HOME_JS = '''
    // What each section plays: the 8 style parts (Rhythm 1 … Phrase 2) in their mixer colours,
    // bar height = how busy the part is there. OTS Link: Main A–D recall OTS 1–4.
    const FSM = @FSM@, FSO = @FSO@;
    const ACT = { 0: [1, .4, .6, .5, .3, .8, 0, 0], 1: [1, .5, .8, .7, .5, .6, .3, 0], 2: [.6, 0, .5, .8, 0, .5, 0, 0], 3: [1, .3, .7, .6, .4, .6, 0, 0], 4: [1, .5, .8, .7, .5, .7, .5, 0], 5: [.5, 0, .4, .5, 0, .8, 0, 0],
      8: [1, .3, .7, .6, 0, .5, 0, 0], 9: [1, .5, .8, .7, .5, .6, .3, 0], 10: [1, .7, .9, .8, .6, .7, .6, .3], 11: [1, .9, 1, .9, .8, .8, .9, .7], 12: [.3, .6, 0, 0, 0, .4, 0, 0] };
    const SP = P.slice(4);
    const TIP = ['Intro I: press to arm it for the start', '', '', 'Ending I', '', '', 'Sync Start: the band starts on your first chord', 'Auto Fill: a fill plays whenever you change Main', 'Main A: press while it plays for its fill', 'Main B: playing. Its fill is queued (it flashes on the Launchkey)', 'Main C', 'Main D', 'Break', 'Tap tempo', 'Sync Stop: the band stops when you let go', 'Start / Stop'];
    // A section's pad is filled edge to edge with its parts smeared together: each part a band in
    // its mixer colour, as wide as it is loud there, blended into one gradient. Playing = full strength.
    // A section's pad is filled edge to edge with its parts smeared together: each part a band in
    // its mixer colour, as wide as it is loud there, pulled toward the pad's own colour (Intro yellow,
    // Main green, Ending red, Break violet) so the pad still reads as its section. Playing = full strength.
    const mix = (a, b, t) => { const p = h => [1, 3, 5].map(i => parseInt(h.slice(i, i + 2), 16)); const x = p(a), y = p(b); return '#' + x.map((v, i) => Math.round(v + (y[i] - v) * t).toString(16).padStart(2, '0')).join(''); };
    const smear = (act, lit, padCol) => { const tot = act.reduce((a, v) => a + v, 0); let x = 0; const stops = [];
      act.forEach((v, k) => { if (!v) return; stops.push(mix(COL[4 + k], padCol, .5) + ' ' + (x + v / tot * 50).toFixed(0) + '%'); x += v / tot * 100; });
      const dim = lit ? 0 : .4;
      return 'linear-gradient(180deg, rgba(14,14,16,.5) 0%, rgba(14,14,16,0) 55%), linear-gradient(rgba(14,14,16,' + dim + '), rgba(14,14,16,' + dim + ')), linear-gradient(90deg, ' + stops.join(', ') + ')'; };
    const homePads = secPads.map((d, i) => { const act = ACT[i], isMain = i >= 8 && i < 12, lit = d.ink === '#0e0e10';
      return Object.assign({}, d, { tip: TIP[i] || d.name, fs: isMain ? FSM : FSO, len: i === 9 ? d.len + ' · fill next' : d.len, well: lit ? 'rgba(14,14,16,.55)' : 'transparent',
        partsDisp: 'none', fill: act ? smear(act, lit, d.bar) : d.bg, ink: act ? '#ffffff' : d.ink, sub: act ? 'rgba(255,255,255,.75)' : d.sub, border: act ? (lit ? '#ffffff' : hexa(d.bar, .45)) : d.border, glow: act && lit ? '0 0 14px rgba(255,255,255,.35)' : d.glow, parts: (act || []).map((v, k) => ({ h: Math.max(8, Math.round(v * 100)), c: COL[4 + k], o: v ? 1 : .18, tip: SP[k][0] + ' · ' + SP[k][1] })),
        ots: isMain ? 'OTS ' + (i - 7) : '', otsDisp: isMain ? 'inline' : 'none', otsTip: 'OTS Link: Main ' + 'ABCD'[i - 8] + ' recalls One Touch ' + (i - 7),
        otsBg: i === 9 ? '#0e0e10' : 'transparent', otsInk: i === 9 ? '#ffffff' : (lit ? '#0e0e10' : '#a9a9b1') }); });
    const legend = SP.map((p, k) => ({ c: COL[4 + k], voice: p[1] }));
    const mk = (v, name, tip) => ({ track: arcPath(11, 11, 7.5, 1), arc: arcPath(11, 11, 7.5, Math.max(0.01, v)), name, tip, op: name ? 1 : .35 });
    const master = [[.5, 'LOW', 'Low 0 dB'], [.45, 'MID', 'Mid −1 dB'], [.58, 'HIGH', 'High +2 dB'], [.35, 'GLUE', 'Glue 2:1'], [.2, 'ROOM', 'Room 12%'], [.8, 'LEVEL', 'Level −2 dB']].map(m => mk(m[0], m[1], m[2]));
    const lkKnobs = knobs.map((k, i) => mk([.5, .3, 0, 1, 1, 0, 0, .45][i], k.name === '—' ? '' : k.name.toUpperCase(), k.name));
    // Bar and beat of the section playing: 4 bars of 4 beats, now at bar 3 beat 2.
    const NOWB = 2, NOWT = 1;
    const barsRow = [0, 1, 2, 3].map(b => ({ n: b + 1, ink: b === NOWB ? '#ffffff' : '#8d8d95', beats: [0, 1, 2, 3].map(t => { const past = b < NOWB || (b === NOWB && t < NOWT), now = b === NOWB && t === NOWT;
      return { h: t === 0 ? 14 : 9, bg: now ? '#ffffff' : (past ? 'rgba(255,255,255,.55)' : 'rgba(255,255,255,.16)'), glow: now ? '0 0 8px rgba(255,255,255,.8)' : 'none' }; }) }));
    const extra = { homePads, master, legend, lkKnobs, barsRow };'''


def home(fxW=260, wide=False):
    right = ""  # knobs sit over the pads; the master bus lives in the mixer
    display = MAIN.replace("@RIGHT@", right).replace("@ART@", SONGMAP_SVG % {"v": "bgArt"})
    for k, v in {"@PANEL@": PANEL, "@CHIP@": CHIP, "@BANDH@": "96" if wide else "76", "@NAMEFS@": "40" if wide else "32", "@CHORDFS@": "64" if wide else "50"}.items():
        display = display.replace(k, v)
    js = HOME_JS.replace("@FSM@", "18" if wide else "15").replace("@FSO@", "15" if wide else "12")
    return dict(TAB="Home", SEL="-1", PADRING="1", DISPLAY=display, JS=js, OVERLAY="", LAYER="VOL")
