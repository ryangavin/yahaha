<!--
  The hardware mirror: the Launchkey 49/61 MK4 control surface, laid out like the real one
  so a glance maps 1:1 onto the controls under your hands (MK4 User Guide pp. 9–11,
  "Launchkey 49 hardware overview": faders left of the screen, transport right of the
  pads; CCs in src/launchkey.rs).

   ┌──────────────┬────────────────────────────────────────────────────────────────────────┐
   │ 8 faders   M │ [ status display: style · tempo · bar/beat │ CHORD │ fingering · split ] │
   │              │ (pad page tabs)                                          [Multi Pads]  │
   │              │ [Shift] [▲ ▼]    [ 8 pads, top row    ] Scene›  Stop                   │
   │ 8 buttons  M │  Track  [◀ ▶]    [ 8 pads, bottom row ] Func    Play                   │
   └──────────────┴────────────────────────────────────────────────────────────────────────┘

  The fader head carries the Parts & OTS, Sounds and Mixer drawer buttons, and the pad-page
  row the Multi Pads one (lib/ui/DrawerButton: small and quieter, not hardware).

  Every element shows its function on the current pad/fader page and Shift layer, has a
  tooltip from the catalog, and clicking it sends exactly what the hardware sends. Every
  size is in em: the shell (App.svelte) sets the font size (`--u`) so the surface fills
  the window by width and height at the hardware's proportions (96em wide; 66em in the
  stacked layout that tall windows get).
-->
<script lang="ts">
  import { PAD_PAGES, type Pad, type PadPage, type Rgb } from '../../lib/api/types'
  import { app, clock, ui } from '../../lib/store.svelte'
  import { surfaceOf } from '../../lib/surface'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import DrawerButton from '../../lib/ui/DrawerButton.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import Control from './Control.svelte'
  import FaderBank from './FaderBank.svelte'
  import HwPad from './HwPad.svelte'
  import StatusDisplay from './StatusDisplay.svelte'

  const s = $derived(app.state)
  const surface = $derived(surfaceOf(s, app.library))
  const shift = $derived(ui.shift || surface.shift)
  const beats = $derived(clock.beats)
  const top = $derived(s.pads.pads.filter((p) => p.note < 112))
  const bottom = $derived(s.pads.pads.filter((p) => p.note >= 112))
  const pageIndex = $derived(PAD_PAGES.findIndex((p) => p.id === s.pads.page))

  /** Page identity colours for the tabs (src/launchkey.rs: white, cyan, magenta, orange). */
  const PAGE_RGB: Record<PadPage, Rgb> = { sections: [100, 100, 100], chordSetup: [0, 100, 127], otsParts: [127, 0, 70], registration: [127, 60, 0], multiPads: [127, 127, 0] }
  const PAGE_TIP = { sections: 'padpage.sections', chordSetup: 'padpage.chord_setup', otsParts: 'padpage.ots_parts', registration: 'padpage.registration', multiPads: 'padpage.multi_pads' } as const
  const cssRgb = (c: Rgb) => `rgb(${c.map((x) => Math.round((x / 127) * 255)).join(' ')})`
  const press = (p: Pad) => p.action && app.send(p.action)
</script>

<section class="wrap" aria-label="Launchkey">
  <div class="device mat-chassis">
    <span class="screw tl" aria-hidden="true"></span>
    <span class="screw tr" aria-hidden="true"></span>

    <!-- Left of the screen, as on the hardware (and first in tab order). -->
    <div class="faders">
      <div class="fader-head">
        <span class="engraved">Faders · {s.mixer.faderPage === 'panel' ? 'Panel: your parts' : 'Style: the band'}</span>
        <!-- The drawers that detail what the faders play: your parts, their sounds, the mix. -->
        <nav class="drawers" aria-label="Part panels">
          <DrawerButton tip="drawer.parts" open={ui.parts} onclick={() => ui.toggleDrawer('parts')}>Parts & OTS</DrawerButton>
          <DrawerButton tip="drawer.sound" open={ui.sound} onclick={() => ui.toggleDrawer('sound')}>Sounds</DrawerButton>
          <DrawerButton tip="drawer.mixer" open={ui.mixer} onclick={() => ui.toggleDrawer('mixer')}>Mixer</DrawerButton>
        </nav>
      </div>
      <div class="fader-body"><FaderBank {surface} /></div>
    </div>

    <div class="screen-area"><StatusDisplay /></div>

    <div class="pagebar">
      <span class="engraved">Pad page</span>
      <div class="tabs" role="tablist" aria-label="Pad page">
        {#each PAD_PAGES as p, i (p.id)}
          <button
            type="button"
            role="tab"
            class="pagetab"
            aria-selected={p.id === s.pads.page}
            style:--page={cssRgb(PAGE_RGB[p.id])}
            use:tip={PAGE_TIP[p.id]}
            onclick={() => app.send({ type: 'setPadPage', page: p.id })}
          >
            <span class="num">{i + 1}</span>{p.name}
          </button>
        {/each}
      </div>
      <span class="engraved lk" class:on={s.pads.connected}>{s.pads.connected ? 'Launchkey connected' : 'No Launchkey'}</span>
      <DrawerButton tip="drawer.multipad" open={ui.multipad} onclick={() => ui.toggleDrawer('multipad')}>Multi Pads</DrawerButton>
    </div>

    <!-- Shift, Pad Bank ▲ ▼ and Track ◀ ▶, stacked in two rows beside the pads. -->
    <div class="nav">
      <HwButton tip="launchkey.shift" pressed={shift} shape="square" caption="Shift" label="Shift" onclick={() => (ui.shiftLatched = !ui.shiftLatched)}>
        <span class="icon">⇧</span>
      </HwButton>
      <div class="padbank" role="group" aria-label="Pad Bank">
        <Control {surface} id="padBankUp" legend="▲" shape="square" />
        <Control {surface} id="padBankDown" legend="▼" shape="square" />
        <span class="engraved page-num">Page <b style:color={cssRgb(PAGE_RGB[s.pads.page])}>{pageIndex + 1}</b></span>
      </div>
      <span class="engraved track-label">Track</span>
      <div class="track" role="group" aria-label="Track">
        <Control {surface} id="trackPrev" legend="◀" caption={surface.trackPrev?.name ?? ''} />
        <Control {surface} id="trackNext" legend="▶" caption={surface.trackNext?.name ?? ''} />
      </div>
    </div>

    <div class="pads mat-well" role="group" aria-label="Pads: page {pageIndex + 1}, {s.pads.pageName}">
      {#each top as p (p.note)}<HwPad pad={p} {beats} paletteLeds={s.pads.paletteLeds} onpress={press} />{/each}
      {#each bottom as p (p.note)}<HwPad pad={p} {beats} paletteLeds={s.pads.paletteLeds} onpress={press} />{/each}
    </div>

    <div class="side" role="group" aria-label="Scene Launch and Function">
      <Control {surface} id="scene" legend="›" shape="square" caption={surface.controls.find((c) => c.id === 'scene')?.label} />
      <Control {surface} id="function" legend="•" shape="square" caption={surface.controls.find((c) => c.id === 'function')?.label} />
    </div>

    <div class="transport" role="group" aria-label="Transport">
      <Control {surface} id="stop" legend="■" shape="square" caption={surface.controls.find((c) => c.id === 'stop')?.label} />
      <Control {surface} id="play" legend="▶" shape="square" caption={surface.controls.find((c) => c.id === 'play')?.label} />
    </div>
  </div>
</section>

<style>
  /* The surface scales with the window: every size below is in em of `--u`, which the
     shell derives from the space it has (App.svelte). 16px when shown on its own. */
  .device {
    font-size: var(--u, 16px);
    position: relative;
    display: grid;
    grid-template-columns: 30em 11.5em minmax(0, 1fr) 4.2em 4.2em;
    grid-template-rows: 9.5em auto auto;
    grid-template-areas:
      'faders screen screen screen screen'
      'faders pagebar pagebar pagebar pagebar'
      'faders nav pads side transport';
    column-gap: 1em;
    row-gap: 0.9em;
    padding: 1.3em 1.5em 1.2em;
    border-radius: 1em;
  }
  .screw {
    position: absolute;
    top: 0.5em;
  }
  .screw.tl {
    left: 0.5em;
  }
  .screw.tr {
    right: 0.5em;
  }
  .screen-area {
    grid-area: screen;
    min-width: 0;
  }
  .pagebar {
    grid-area: pagebar;
    display: flex;
    align-items: flex-end;
    gap: 0.8em;
    padding-bottom: 0.1em;
    min-width: 0;
  }
  .tabs {
    display: flex;
    gap: 0.35em;
  }
  .pagetab {
    display: inline-flex;
    align-items: center;
    gap: 0.4em;
    padding: 0.3em 0.7em;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.85em;
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
  .pagetab[aria-selected='true'] {
    color: var(--ink);
    border-color: var(--page);
    box-shadow: inset 0 -2px 0 var(--page), 0 0 10px -4px var(--page);
  }
  .lk {
    margin-left: auto;
  }
  /* A touch taller than the tabs: let it rise into the row gap, so the mirror's height
     (and so --h in App.svelte) stays as measured. */
  .pagebar :global(.drawer-btn) {
    margin-top: -0.5em;
  }
  .lk.on {
    color: #5fd68a;
  }
  /* Two rows beside the pads: [Shift] [▲ ▼ page] over [Track] [◀ ▶ + the neighbouring
     styles' names]. One narrow block, so the pads get the width. */
  .nav {
    grid-area: nav;
    display: grid;
    grid-template-columns: 3em minmax(0, 1fr);
    grid-template-rows: auto auto;
    column-gap: 0.5em;
    row-gap: 0.5em;
    align-content: space-between;
    align-items: start;
    padding-top: 0.35em;
    min-width: 0;
  }
  .padbank,
  .track {
    display: grid;
    grid-template-columns: 1fr 1fr;
    column-gap: 0.4em;
    min-width: 0;
  }
  .page-num {
    grid-column: 1 / -1;
    margin-top: 0.3em;
    text-align: center;
  }
  .page-num b {
    font-weight: 700;
  }
  /* The neighbouring styles' names: wrap onto two lines rather than cut off. */
  .track :global(.caption) {
    white-space: normal;
    overflow-wrap: anywhere;
    line-height: 1.1;
    min-height: 2.2em;
  }
  /* Level with the middle of ◀ ▶ (2.1em buttons). */
  .track-label {
    align-self: start;
    margin-top: 0.75em;
    text-align: center;
  }
  .side,
  .transport {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0.7em;
    padding-top: 0.35em;
  }
  .side {
    grid-area: side;
  }
  .transport {
    grid-area: transport;
  }
  .pads {
    grid-area: pads;
    display: grid;
    grid-template-columns: repeat(8, minmax(0, 1fr));
    gap: 0.6em;
    padding: 0.6em;
    border-radius: 0.8em;
    align-self: start;
  }
  .faders {
    grid-area: faders;
    display: grid;
    grid-template-rows: auto 1fr;
    gap: 0.4em;
    padding-right: 1em;
    border-right: 1px solid var(--seam);
    box-shadow: 1px 0 0 rgb(255 255 255 / 0.04);
    min-width: 0;
  }
  .fader-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.8em;
    min-width: 0;
  }
  .drawers {
    display: flex;
    gap: 0.35em;
  }
  .fader-body {
    min-height: 0;
  }
  .icon {
    font-size: 1.05em;
    line-height: 1;
  }
  /* Panel print scales with the surface (the global .engraved is in rem, for drawers). */
  .device :global(.engraved) {
    font-size: 0.78em;
  }

  /* Tall windows (the shell's stage narrower than 1.45:1): the fader bank moves under the
     pads, so the surface is 66em wide and can grow larger. */
  @container stage (aspect-ratio < 1.45) {
    .device {
      grid-template-columns: 11.5em minmax(0, 1fr) 4.2em 4.2em;
      grid-template-rows: 9.5em auto auto 19em;
      grid-template-areas:
        'screen screen screen screen'
        'pagebar pagebar pagebar pagebar'
        'nav pads side transport'
        'faders faders faders faders';
    }
    .faders {
      padding: 0.7em 0 0;
      border-right: none;
      border-top: 1px solid var(--seam);
      box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.04);
    }
  }
</style>
