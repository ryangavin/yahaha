<!--
  The hand surface: the Launchkey 49/61 MK4's knobs and pads, laid out like the real one
  so a glance maps 1:1 onto the controls under your hands (MK4 User Guide pp. 9–11,
  "Launchkey 49 hardware overview": the eight encoders over the sixteen pads, the pad and
  track buttons left of them, Scene Launch / Function right of the pads; CCs in
  crates/yahaha-engine/src/launchkey.rs). One flat row, the stage's full width.

   ┌ cheek ──────────────────────┬ 8 knob columns ──────────────────┬ right cheek ──────────────┐
   │ KNOBS 1/6                   │ (◯) NAME  (◯) NAME  …  (◯) NAME  │ [1 Sections] [2 Racks] …  │
   │ [◀][  Style   ][▶]          │     [val]     [val]        [val]  │ [4 Multi Pads] [5 Setup]  │
   │ [Shift][Sound] [▲][▼] Page 2│ ┌ pads, top row ───────────────┐ │ [›] TEMPO+ [■] STOP [Multi Pads]
   │ [Rotary] [◀]     [▶]        │ └ pads, bottom row ────────────┘ │ [•] TEMPO- [▶] PLAY ● connected
   │          prev     next      │                                  │                           │
   └─────────────────────────────┴──────────────────────────────────┴───────────────────────────┘

  The knobs are relative, like the hardware's: a Knob sends the same turnKnob a hardware
  turn does (drag, wheel or arrow keys; double-click resets), with its label over its lit
  readout, and the ◀ ▶ pager steps the knob page (stepKnobPage); the page shown is the
  hardware's. The pad-page tabs sit over the right cheek, one per page in the player's
  order and names (`pads.pages`: Sections, then Settings › Launchkey's order; a page left
  out has no tab), and the Multi Pads drawer button
  (lib/ui/DrawerButton: small and quieter, not hardware) beside Stop/Play. The faders and
  the Launchkey's screen are not here: the mixer row's strips are what the faders move,
  and the header and the display show what the screen did.

  The held layers (`surface.layer`, docs/eyes-free.md) show as the hardware's do. Sound
  (Panel fader button 6 on the hardware, here beside Shift since the strips are the
  faders) lights while held, the Racks tab is marked, and the pads are the state's, which
  are already the Racks page's with the hold's actions (a capture pad sends storeRack). On
  screen Sound is held with `setLayer`: a click latches it, the next click lets go, and a
  press held past 350 ms is momentary. In swap mode (a part's fader button held, a knob
  turned; on screen a long press on a keyboard strip's On, Strip.svelte) the knob pager
  and knobs are the state's, the part's: knob 1 its sound by number and name, which a turn
  steps with swapSound as the hardware's knob 1 does; knobs 2-8 its mix (turnSwapKnob).

  Every element shows its function on the current pad page and Shift layer, has a tooltip
  from the catalog, and clicking it sends exactly what the hardware sends. Every size is
  in em: the shell (App.svelte) sets the font size (`--u`) so the surface fills its slot
  at the proportions below (105em × 10em); on a very short slot the engraved print
  (unreadable there) steps aside and the tooltips carry it.
-->
<script lang="ts">
  import type { Layer, Pad, PadPage, Rgb } from '../../lib/api/types'
  import { app, clock, ui } from '../../lib/store.svelte'
  import { layer, surfaceOf } from '../../lib/surface'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import DrawerButton from '../../lib/ui/DrawerButton.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import Knob from '../../lib/ui/Knob.svelte'
  import Control from './Control.svelte'
  import HwPad from './HwPad.svelte'

  const s = $derived(app.state)
  const k = $derived(s.knobs)
  const surface = $derived(surfaceOf(s, app.library))
  const shift = $derived(ui.shift || surface.shift)
  const beats = $derived(clock.beats)
  // The same array while the pads don't change (the store shares equal parts of each
  // state), so the rows below aren't re-filtered on every state.
  const pads = $derived(s.pads.pads)
  const top = $derived(pads.filter((p) => p.note < 112))
  const bottom = $derived(pads.filter((p) => p.note >= 112))
  // The held control's layer (docs/eyes-free.md): while Sound is held the state's pads are
  // already the Racks page's (and their actions the hold's, storeRack on a capture pad);
  // while a part's button is held with a knob turned, the knobs are that part's.
  const held: Layer = $derived(surface.layer ?? { type: 'none' })
  const soundHeld = $derived(held.type === 'sound')
  const swapPart = $derived(held.type === 'swap' ? held.part : null)
  /** A knob turned: in swap mode knob 1 steps the part's sound, as the hardware's knob 1
   *  does (`swapSound`), and knobs 2-8 are the part's mix (`turnSwapKnob`); otherwise the
   *  knob page's (`turnKnob`). */
  function turn(i: number, delta: number) {
    if (swapPart === null) app.send({ type: 'turnKnob', knob: i, delta })
    else if (i === 0) app.send({ type: 'swapSound', part: swapPart, step: delta })
    else app.send({ type: 'turnSwapKnob', part: swapPart, knob: i, delta })
  }

  // Sound (Panel fader button 6): held as the hardware's is, with `setLayer`. A click
  // latches it (the pointer is then free for the pads) and the next click lets go; a press
  // held past HOLD_MS is momentary, let go on release (a second finger plays the pads). While
  // Shift gives the button another job (Style page: mute the Pad part) it does that instead,
  // but a held Sound always lets go.
  const HOLD_MS = 350
  const b6 = $derived(surface.controls.find((c) => c.id === 'faderButton6'))
  const isSound = $derived(soundHeld || (!shift && b6?.label === 'SOUND'))
  let soundDown: { at: number; release: boolean } | null = null
  const setSound = (on: boolean) => app.send({ type: 'setLayer', layer: on ? { type: 'sound' } : { type: 'none' } })
  function soundOther() {
    const a = b6 && layer(b6, shift).action
    if (a) app.send(a)
  }
  function soundHold(down: boolean) {
    if (down) {
      soundDown = null
      if (!isSound) return soundOther()
      soundDown = { at: performance.now(), release: soundHeld }
      if (!soundHeld) setSound(true)
    } else if (soundDown) {
      if (soundDown.release || performance.now() - soundDown.at >= HOLD_MS) setSound(false)
      soundDown = null
    }
  }
  /** Enter or Space: latch or let go. */
  const soundPress = () => (isSound ? setSound(!soundHeld) : soundOther())

  /** Page identity colours for the tabs (src/launchkey.rs: white, cyan, magenta, orange). */
  const PAGE_RGB: Record<PadPage, Rgb> = { sections: [100, 100, 100], racks: [127, 60, 0], chord: [0, 100, 127], multiPads: [127, 127, 0], setup: [127, 0, 70] }
  const PAGE_TIP = { sections: 'padpage.sections', racks: 'padpage.racks', chord: 'padpage.chord', multiPads: 'padpage.multi_pads', setup: 'padpage.setup' } as const
  const cssRgb = (c: Rgb) => `rgb(${c.map((x) => Math.round((x / 127) * 255)).join(' ')})`
  const press = (p: Pad) => p.action && app.send(p.action)
</script>

<section class="wrap" aria-label="Launchkey">
  <div class="device mat-chassis">
    <span class="screw tl" aria-hidden="true"></span>
    <span class="screw tr" aria-hidden="true"></span>

    <!-- The knob page, left of the knobs. -->
    <div class="kpager" role="group" aria-label="Knob page">
      <span class="engraved lbl">Knobs {k.pageNumber}/{k.pageCount}</span>
      <div class="pg">
        <HwButton tip="knobs.page" label="Previous knob page" onclick={() => app.send({ type: 'stepKnobPage', delta: -1 })}>◀</HwButton>
        <span class="readout mat-screen page-name" use:tip={'knobs.page'}><span class="glow-text">{k.pageName}</span></span>
        <HwButton tip="knobs.page" label="Next knob page" onclick={() => app.send({ type: 'stepKnobPage', delta: 1 })}>▶</HwButton>
      </div>
    </div>

    <!-- The eight knobs, each over its column of pads. -->
    <div class="knobs" class:swap={swapPart !== null} role="group" aria-label="Knobs" data-layer={held.type}>
      {#each k.knobs as knob, i (i)}
        {@const off = knob.function === 'none'}
        {@const sound = swapPart !== null && i === 0}
        <div class="cell" class:off class:swapsound={sound} title={knob.name} data-col={i}>
          <Knob
            label={knob.name}
            level={knob.level}
            disabled={off}
            tipKey={swapPart !== null ? 'part.swap' : 'knobs.knob'}
            onturn={(delta) => turn(i, delta)}
            onreset={() => swapPart === null && app.send({ type: 'resetKnob', knob: i })}
          />
          <div class="text">
            <span class="name engraved">{knob.short}</span>
            <span class="readout mat-screen"><span class="glow-text">{knob.value}</span></span>
          </div>
        </div>
      {/each}
    </div>

    <!-- The pad pages, over the right cheek: every page named, in two rows. -->
    <div class="tabs" role="tablist" aria-label="Pad page">
      {#each s.pads.pages as p, i (p.page)}
        <button
          type="button"
          role="tab"
          class="pagetab"
          class:held={soundHeld && p.page === 'racks'}
          aria-selected={p.page === s.pads.page}
          style:--page={cssRgb(PAGE_RGB[p.page])}
          use:tip={PAGE_TIP[p.page]}
          onclick={() => app.send({ type: 'setPadPage', page: p.page })}
        >
          <span class="num">{i + 1}</span><span class="pname">{p.name}</span>
        </button>
      {/each}
    </div>

    <!-- Shift, Pad Bank ▲ ▼ over Rotary, Track ◀ ▶: left of the pads, as on the hardware. -->
    <div class="nav">
      <HwButton tip="launchkey.shift" pressed={shift} label="Shift" onclick={() => (ui.shiftLatched = !ui.shiftLatched)}>
        <span class="icon">⇧</span><span class="word">Shift</span>
      </HwButton>
      <!-- Sound: Panel fader button 6 on the hardware, here beside Shift since the strips
           are the faders. Lit (and pressed) while it is held or latched (setLayer). -->
      <div class="sound" data-held={soundHeld}>
        <Control
          {surface}
          id="faderButton6"
          legend="Sound"
          pressed={soundHeld}
          tip={soundHeld ? 'launchkey.sound' : undefined}
          onhold={soundHold}
          onpress={soundPress}
        />
      </div>
      <div class="padbank" role="group" aria-label="Pad Bank">
        <Control {surface} id="padBankUp" legend="▲" />
        <Control {surface} id="padBankDown" legend="▼" />
        <span class="engraved page-num">Page <b style:color={cssRgb(PAGE_RGB[s.pads.page])}>{s.pads.pageNumber}</b></span>
      </div>
      <!-- The organ rotary speaker's Slow/Fast: Shift + the encoder page ▲ on the hardware. -->
      <HwButton tip="launchkey.rotary_fast" pressed={s.effects.rotaryFast} label="Rotary Fast" onclick={() => app.send({ type: 'toggleRotaryFast' })}>Rotary</HwButton>
      <div class="track" role="group" aria-label="Track">
        <Control {surface} id="trackPrev" legend="◀" caption={surface.trackPrev?.name ?? ''} />
        <Control {surface} id="trackNext" legend="▶" caption={surface.trackNext?.name ?? ''} />
      </div>
    </div>

    <div class="pads mat-well" role="group" aria-label="Pads: {soundHeld ? 'Sound held, Racks' : `page ${s.pads.pageNumber}, ${s.pads.pageName}`}" data-layer={held.type}>
      {#each top as p (p.note)}<HwPad pad={p} {beats} paletteLeds={s.pads.paletteLeds} onpress={press} />{/each}
      {#each bottom as p (p.note)}<HwPad pad={p} {beats} paletteLeds={s.pads.paletteLeds} onpress={press} />{/each}
    </div>

    <!-- One button per pad row, its function engraved beside it. -->
    <div class="side" role="group" aria-label="Scene Launch and Function">
      <Control {surface} id="scene" legend="›" shape="square" showFunction />
      <Control {surface} id="function" legend="•" shape="square" showFunction />
    </div>

    <div class="transport" role="group" aria-label="Transport">
      <Control {surface} id="stop" legend="■" shape="square" showFunction />
      <Control {surface} id="play" legend="▶" shape="square" showFunction />
    </div>

    <div class="pagebar">
      <DrawerButton tip="drawer.multipad" open={ui.multipad} onclick={() => ui.toggleDrawer('multipad')}>Multi Pads</DrawerButton>
      <span class="engraved lk" class:on={s.pads.connected} use:tip={'launchkey.status'}
        ><span class="lk-dot" aria-hidden="true"></span><span class="lk-text">{s.pads.connected ? 'Launchkey connected' : 'No Launchkey'}</span></span
      >
    </div>
  </div>
</section>

<style>
  /* The surface scales with the window: every size below is in em of `--u`, which the
     shell derives from its slot (App.svelte). 16px when shown on its own. The height is
     the shell's --h (10em): 0.6em padding, the knob row (2.8em), a 0.45em gap and the pad
     rows. The width is the shell's --w (105em): the fixed columns take 48.5em with the
     gaps and padding, and the eight knob/pad columns the rest, so a narrower slot narrows
     the pads, not the controls beside them. */
  .device {
    font-size: var(--u, 16px);
    position: relative;
    box-sizing: border-box;
    height: 10em;
    display: grid;
    grid-template-columns: 19.1em minmax(0, 1fr) 6.6em 6.6em 8.6em;
    grid-template-rows: 2.8em minmax(0, 1fr);
    grid-template-areas:
      'kpager knobs tabs tabs tabs'
      'nav pads side transport pagebar';
    column-gap: 0.9em;
    row-gap: 0.45em;
    padding: 0.6em 1.2em;
    border-radius: 0.8em;
  }
  .screw {
    position: absolute;
    top: 0.35em;
  }
  .screw.tl {
    left: 0.35em;
  }
  .screw.tr {
    right: 0.35em;
  }
  /* Panel print scales with the surface (the global .engraved is in rem, for drawers). */
  .device :global(.engraved) {
    font-size: 0.78em;
    line-height: 1.2;
  }

  /* ── The knob pager: a label over ◀ [page] ▶, laid out like a knob's text ── */
  .kpager {
    grid-area: kpager;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 0.2em;
    min-width: 0;
  }
  .lbl {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .pg {
    display: flex;
    gap: 0.3em;
    height: 1.55em;
    min-width: 0;
  }
  .pg :global(.hw) {
    flex: none;
    width: 1.9em;
    height: 100%;
  }
  .pg :global(.btn) {
    min-width: 0;
    height: 100%;
    padding: 0;
    font-size: 0.7em;
  }
  .pg .readout {
    flex: 1;
  }

  /* ── The knobs: eight columns over the pads' eight, each as tall as its label + readout ── */
  .knobs {
    grid-area: knobs;
    display: grid;
    grid-template-columns: repeat(8, minmax(0, 1fr));
    column-gap: 0.6em;
    padding: 0 0.35em;
    min-width: 0;
  }
  .cell {
    --knob-size: 2.6em;
    display: flex;
    align-items: center;
    gap: 0.4em;
    min-width: 0;
  }
  .text {
    display: flex;
    flex-direction: column;
    gap: 0.2em;
    min-width: 0;
    flex: 1;
  }
  .name,
  .readout {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* The lit readout: knob values and the knob page alike. */
  .readout {
    display: block;
    box-sizing: border-box;
    height: 1.55em;
    padding: 0 0.3em;
    border-radius: var(--r-key);
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 1em;
    line-height: calc(1.55em - 2px);
    text-align: center;
  }
  .readout > span {
    font-size: 0.9em;
  }
  .off .name,
  .off .readout {
    opacity: 0.45;
  }
  .off .readout {
    color: var(--screen-dim);
  }
  /* Swap mode: knob 1 is the held part's sound (its number and name), marked in the
     accent so the row reads as the part's, not the knob page's. */
  .swapsound .name {
    color: var(--accent);
  }
  .swapsound .readout {
    outline: 1px solid var(--accent);
  }

  /* ── The pad-page tabs: two rows over the right cheek ── */
  .tabs {
    grid-area: tabs;
    display: flex;
    flex-wrap: wrap;
    align-content: space-between;
    gap: 0.25em 0.3em;
    min-width: 0;
  }
  .pagetab {
    display: inline-flex;
    align-items: center;
    gap: 0.35em;
    height: 1.6em;
    padding: 0 0.55em 0 0.3em;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.8em;
    white-space: nowrap;
    color: var(--engrave);
    background: transparent;
    border: 1px solid var(--seam);
    border-radius: 0.35em;
  }
  .pagetab .num {
    font-size: 0.85em;
    width: 1.3em;
    height: 1.3em;
    display: inline-grid;
    place-items: center;
    border-radius: 50%;
    border: 1px solid currentColor;
  }
  /* Sound held: the pads show Racks, whatever page is on view. */
  .pagetab.held {
    color: var(--ink);
    border-style: dashed;
    border-color: var(--page);
  }
  .pagetab[aria-selected='true'] {
    color: var(--ink);
    border-color: var(--page);
    box-shadow: inset 0 -2px 0 var(--page), 0 0 10px -4px var(--page);
  }

  /* ── Left of the pads: [Shift] [Sound] [▲ ▼ page] over [Rotary] [◀ ▶ + the
     neighbouring styles' names] ── */
  .nav {
    grid-area: nav;
    display: grid;
    grid-template-columns: 3.6em 3.6em minmax(0, 1fr);
    grid-template-rows: auto auto;
    column-gap: 0.5em;
    align-content: space-between;
    align-items: start;
    padding-top: 0.35em;
    min-width: 0;
  }
  .nav :global(.btn) {
    height: 2em;
    min-width: 0;
    padding: 0 0.3em;
  }
  .padbank,
  .track {
    display: grid;
    grid-template-columns: 1fr 1fr;
    column-gap: 0.4em;
    align-items: start;
    min-width: 0;
  }
  .padbank {
    grid-template-columns: 1fr 1fr 3.2em;
  }
  /* Track ◀ ▶ take the width under Sound and Pad Bank: their captions are style names. */
  .track {
    grid-column: 2 / 4;
  }
  .sound {
    min-width: 0;
  }
  .sound :global(.legend) {
    font-size: 0.8em;
  }
  .page-num {
    align-self: center;
    text-align: center;
    white-space: nowrap;
  }
  .page-num b {
    font-weight: 700;
  }
  .icon {
    font-size: 1.05em;
    line-height: 1;
  }
  .word {
    font-size: 0.8em;
  }
  .track :global(.hw) {
    gap: 0.15em;
  }

  /* ── The pads: two rows of eight, each pad as wide as its knob's column ── */
  .pads {
    grid-area: pads;
    display: grid;
    grid-template-columns: repeat(8, minmax(0, 1fr));
    grid-template-rows: repeat(2, minmax(0, 1fr));
    gap: 0.35em 0.6em;
    padding: 0.35em;
    border-radius: 0.6em;
    min-height: 0;
  }
  .pads :global(.pad) {
    aspect-ratio: auto;
    height: 100%;
  }

  /* ── Right of the pads: one button per pad row, its function engraved beside it ── */
  .side,
  .transport {
    display: grid;
    grid-template-rows: repeat(2, minmax(0, 1fr));
    row-gap: 0.35em;
    padding: 0.35em 0;
    min-width: 0;
    min-height: 0;
  }
  .side {
    grid-area: side;
  }
  .transport {
    grid-area: transport;
  }
  .side :global(.hw),
  .transport :global(.hw) {
    flex-direction: row;
    align-items: stretch;
    gap: 0.4em;
    min-width: 0;
    min-height: 0;
  }
  .side :global(.btn),
  .transport :global(.btn) {
    flex: none;
    width: 2.6em;
    min-width: 0;
    height: 100%;
    padding: 0;
  }
  .side :global(.caption),
  .transport :global(.caption) {
    align-self: center;
    text-align: left;
    min-width: 0;
  }

  .pagebar {
    grid-area: pagebar;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    align-items: stretch;
    padding: 0.35em 0;
    min-width: 0;
  }
  .pagebar :global(.drawer-btn) {
    height: 2.1em;
  }
  .lk {
    display: flex;
    align-items: center;
    gap: 0.4em;
    min-width: 0;
    white-space: nowrap;
  }
  .lk-text {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .lk-dot {
    flex: none;
    width: 0.6em;
    height: 0.6em;
    border-radius: 50%;
    background: var(--seam);
  }
  .lk.on {
    color: #5fd68a;
  }
  .lk.on .lk-dot {
    background: currentColor;
    box-shadow: 0 0 5px currentColor;
  }

  /* A very short slot (under 80px: a 1024×700 window, where the mixer row keeps its
     least height): the engraved print is too small to read there, so it steps aside
     (visually only: the tooltips and the accessible names keep it), and the pads keep
     their labels. Nothing moves, so the surface keeps its proportions. */
  @container hand (height < 80px) {
    .device :global(.engraved),
    .device :global(.caption),
    .pads :global(.key) {
      visibility: hidden;
    }
    .screw {
      display: none;
    }
  }
</style>
