<!--
  The hardware mirror: the Launchkey 49/61 MK4 control surface, laid out like the real one
  so a glance maps 1:1 onto the controls under your hands (MK4 User Guide, "The surface in
  DAW mode"; CCs in src/launchkey.rs).

   ┌────────────────────────────────────────────────────────────────────────┬──────────────┐
   │ [ status display: style · tempo · bar/beat │ CHORD │ fingering · split ] │ 8 faders   M │
   │ (pad page tabs)                                                         │              │
   │ [Shift] [◀ Track ▶]  ▲ [ 8 pads, top row    ] Scene›  Stop              │              │
   │                      ▼ [ 8 pads, bottom row ] Func    Play              │ 8 buttons  M │
   └────────────────────────────────────────────────────────────────────────┴──────────────┘

  Every element shows its function on the current pad/fader page and Shift layer, has a
  tooltip from the catalog, and clicking it sends exactly what the hardware sends. The
  whole surface scales with the window (sizes are in em of a width-derived font size).
-->
<script lang="ts">
  import { tipFor } from '../../help/actions'
  import { PAD_PAGES, type Pad, type PadPage, type Rgb } from '../../lib/api/types'
  import { app, clock, ui } from '../../lib/store.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import FaderBank from './FaderBank.svelte'
  import HwPad from './HwPad.svelte'
  import { neighbours } from './neighbours'
  import StatusDisplay from './StatusDisplay.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'

  const s = $derived(app.state)
  const beats = $derived(clock.beats)
  const pads = $derived(s.pads.pads)
  const top = $derived(pads.filter((p) => p.note < 112))
  const bottom = $derived(pads.filter((p) => p.note >= 112))
  const pageIndex = $derived(PAD_PAGES.findIndex((p) => p.id === s.pads.page))
  const near = $derived(neighbours(app.library, s.library.position))

  /** Page identity colours (src/launchkey.rs: white, cyan, magenta). */
  const PAGE_RGB: Record<PadPage, Rgb> = { sections: [100, 100, 100], chordSetup: [0, 100, 127], otsParts: [127, 0, 70] }
  const WHITE: Rgb = [100, 100, 100]
  const lit = (rgb: Rgb, on: boolean) => ({ rgb, level: on ? ('bright' as const) : ('off' as const), anim: 'solid' as const })

  const press = (p: Pad) => p.action && app.send(p.action)
  const pageRgb = $derived(PAGE_RGB[s.pads.page])
  const left = $derived(s.keyboardParts[3])
</script>

<section class="wrap" aria-label="Launchkey">
  <div class="device mat-chassis">
    <span class="screw tl" aria-hidden="true"></span>
    <span class="screw tr" aria-hidden="true"></span>

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
            style:--page={`rgb(${PAGE_RGB[p.id].map((c) => Math.round((c / 127) * 255)).join(' ')})`}
            use:tip={p.id === 'sections' ? 'padpage.sections' : p.id === 'chordSetup' ? 'padpage.chord_setup' : 'padpage.ots_parts'}
            onclick={() => app.send({ type: 'setPadPage', page: p.id })}
          >
            <span class="num">{i + 1}</span>{p.name}
          </button>
        {/each}
      </div>
      <span class="engraved lk" class:on={s.pads.connected}>{s.pads.connected ? 'Launchkey connected' : 'No Launchkey'}</span>
    </div>

    <div class="left-controls">
      <HwButton tip="launchkey.shift" pressed={ui.shift} caption="Shift" onclick={() => (ui.shiftLatched = !ui.shiftLatched)}>
        <span class="icon">⇧</span>
      </HwButton>
      <div class="track">
        <HwButton tip="style.prev" led={lit(WHITE, !!near.prev)} {beats} label="Previous style" caption={near.prev?.name ?? ''} onclick={() => app.send({ type: 'stepStyle', delta: -1 })}>
          <span class="icon">◀</span>
        </HwButton>
        <HwButton tip="style.next" led={lit(WHITE, !!near.next)} {beats} label="Next style" caption={near.next?.name ?? ''} onclick={() => app.send({ type: 'stepStyle', delta: 1 })}>
          <span class="icon">▶</span>
        </HwButton>
      </div>
      <span class="engraved track-label">Track</span>
    </div>

    <div class="padbank" role="group" aria-label="Pad Bank">
      {#if ui.shift}
        <HwButton tip="part.left.on" led={lit(pageRgb, left?.on ?? false)} {beats} label="Shift + Pad Bank up: Left on/off" shape="square" onclick={() => app.send({ type: 'togglePart', part: 3 })}>
          <span class="shift-fn">Left</span>
        </HwButton>
        <HwButton tip="ots.link" led={lit(pageRgb, s.ots.link)} {beats} label="Shift + Pad Bank down: OTS Link" shape="square" onclick={() => app.send({ type: 'toggleOtsLink' })}>
          <span class="shift-fn">Link</span>
        </HwButton>
      {:else}
        <HwButton tip="padpage.prev" led={lit(pageRgb, pageIndex > 0)} {beats} label="Pad Bank up" shape="square" onclick={() => pageIndex > 0 && app.send({ type: 'setPadPage', page: PAD_PAGES[pageIndex - 1].id })}>
          <span class="icon">▲</span>
        </HwButton>
        <HwButton tip="padpage.next" led={lit(pageRgb, pageIndex < 2)} {beats} label="Pad Bank down" shape="square" onclick={() => pageIndex < 2 && app.send({ type: 'setPadPage', page: PAD_PAGES[pageIndex + 1].id })}>
          <span class="icon">▼</span>
        </HwButton>
      {/if}
      <span class="engraved page-num" style:color={`rgb(${pageRgb.map((c) => Math.round((c / 127) * 255)).join(' ')})`}>{pageIndex + 1}</span>
    </div>

    <div class="pads mat-well" role="group" aria-label="Pads: page {pageIndex + 1}, {s.pads.pageName}">
      {#each top as p (p.note)}<HwPad pad={p} {beats} onpress={press} />{/each}
      {#each bottom as p (p.note)}<HwPad pad={p} {beats} onpress={press} />{/each}
    </div>

    <div class="side" role="group" aria-label="Tempo">
      <HwButton tip={tipFor({ type: 'tempoUp' })} label="Scene Launch: tempo up" shape="square" onclick={() => app.send({ type: 'tempoUp' })}>
        <span class="icon">+</span>
      </HwButton>
      <HwButton tip={tipFor({ type: 'tempoDown' })} label="Function: tempo down" shape="square" onclick={() => app.send({ type: 'tempoDown' })}>
        <span class="icon">−</span>
      </HwButton>
      <span class="engraved">Tempo</span>
    </div>

    <div class="transport" role="group" aria-label="Transport">
      <HwButton tip="transport.stop" label="Stop" shape="square" onclick={() => app.send({ type: 'stop' })}>
        <span class="icon">■</span>
      </HwButton>
      <HwButton tip="transport.start_stop" label="Play" shape="square" led={lit([0, 127, 0], s.transport.running)} {beats} onclick={() => app.send({ type: 'startStop' })}>
        <span class="icon">▶</span>
      </HwButton>
      <span class="engraved">Transport</span>
    </div>

    <div class="faders">
      <div class="fader-head">
        <span class="engraved">Faders · {s.mixer.faderPage === 'panel' ? 'Panel: your parts' : 'Style: the band'}</span>
      </div>
      <div class="fader-body"><FaderBank /></div>
    </div>
  </div>
</section>

<style>
  .wrap {
    container-type: inline-size;
  }
  /* The surface scales with the window: every size below is in em of this font size. */
  .device {
    --u: clamp(10.5px, calc(100cqw / 96), 19px);
    font-size: var(--u);
    position: relative;
    display: grid;
    grid-template-columns: 11.5em 3.3em minmax(0, 1fr) 3.6em 3.6em 30em;
    grid-template-rows: 9.5em auto auto;
    grid-template-areas:
      'screen screen screen screen screen faders'
      'pagebar pagebar pagebar pagebar pagebar faders'
      'left padbank pads side transport faders';
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
  .lk.on {
    color: #5fd68a;
  }
  .left-controls {
    grid-area: left;
    display: grid;
    grid-template-columns: 3em 1fr;
    grid-template-rows: auto auto 1fr;
    column-gap: 0.7em;
    align-content: start;
    padding-top: 0.35em;
  }
  .track {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.4em;
    min-width: 0;
  }
  .track-label {
    grid-column: 2;
    text-align: center;
  }
  .padbank,
  .side,
  .transport {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0.7em;
    padding-top: 0.35em;
  }
  .padbank {
    grid-area: padbank;
  }
  .side {
    grid-area: side;
  }
  .transport {
    grid-area: transport;
  }
  .padbank .engraved,
  .side .engraved,
  .transport .engraved {
    text-align: center;
  }
  .page-num {
    font-size: 1.1em;
    font-weight: 700;
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
    padding-left: 1em;
    border-left: 1px solid var(--seam);
    box-shadow: inset 1px 0 0 rgb(255 255 255 / 0.04);
    min-width: 0;
  }
  .fader-body {
    min-height: 0;
  }
  .icon {
    font-size: 1.05em;
    line-height: 1;
  }
  .shift-fn {
    font-size: 0.72em;
    color: var(--accent);
  }

  /* Narrow windows: the fader bank moves under the pads. */
  @container (max-width: 820px) {
    .device {
      --u: clamp(9px, calc(100cqw / 62), 15px);
      grid-template-columns: 3.3em minmax(0, 1fr) 3.6em;
      grid-template-rows: auto auto auto auto auto auto;
      grid-template-areas:
        'screen screen screen'
        'pagebar pagebar pagebar'
        'padbank pads side'
        'left left left'
        'transport transport transport'
        'faders faders faders';
    }
    .screen-area {
      height: 11em;
    }
    .pagebar {
      flex-wrap: wrap;
    }
    .transport {
      flex-direction: row;
      justify-content: center;
      align-items: center;
    }
    .transport :global(.hw) {
      width: 4em;
    }
    .faders {
      padding-left: 0;
      border-left: none;
      height: 20em;
    }
  }
</style>
