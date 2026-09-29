<!--
  The stage's control panel between the Launchkey mirror and the keyboard strip: the
  Launchkey's eight knobs over the eight Quick Rack buttons, one column each, so knob n
  sits directly above rack n. Both pagers share the left cheek; Store is on the right.

   ┌ cheek ────────────┬ col 1 ──────────┬ col 2 ─ … ─ col 8 ┬ cheek ─┐
   │ KNOBS 1/6         │ (◯) NAME        │                   │        │
   │ [◀][ Style  ][▶]  │     [ value ]   │   …               │        │
   │ QUICK RACKS       │ ┌─────────────┐ │                   │ [Store]│
   │ [◀][   A    ][▶]  │ │ 1  Ballad  ✕│ │   …               │        │
   └───────────────────┴─┴─────────────┴─┴───────────────────┴────────┘

  Knobs: each a rotary Knob (drag, wheel or arrow keys; double-click resets) with its
  label over its lit readout; the knobs are relative, so this sends the same turnKnob a
  hardware turn does. Quick Racks (docs/racks.md): one press loads a rack; the buttons
  light like pad page 4 (red loaded, blue stored, dark empty; all flashing while Store is
  armed), the rack's name printed on the button, ✕ clears it. A Store waiting for a save
  asks in the rack row's place (RackPrompt), and a rack prompt that appears opens the
  Rack drawer (openRackDrawerOnPrompt). Nothing here is hardware-only.

  Every size is in em of the stage's `--u` (App.svelte), like the mirror and the strip.
  Both rows are --row tall: one label line, a small gap and a readout, so a knob is as
  tall as the text beside it, and a pager is laid out like a knob's text.

  State: knobs, quickRacks, liveRack. Commands: stepKnobPage, turnKnob, resetKnob,
  pressQuickRack, stepQuickRackBank, toggleQuickRackStore, clearQuickRack.
-->
<script lang="ts">
  import { QUICK } from '../../help/actions'
  import { bankLetter, quickLabel, quickLook, quickName } from '../../lib/api/quick-racks'
  import { app, clock } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import Knob from '../../lib/ui/Knob.svelte'
  import { openRackDrawerOnPrompt } from '../quickracks/rackPromptDrawer.svelte'
  import RackPrompt, { asking } from '../quickracks/RackPrompt.svelte'

  const k = $derived(app.state.knobs)
  const q = $derived(app.state.quickRacks)
  const beats = $derived(clock.beats)
  const armed = { rgb: [127, 0, 0] as [number, number, number], level: 'bright' as const, anim: 'flash' as const }
  const ask = $derived(asking(app.state))

  openRackDrawerOnPrompt()

  /** The grid column of knob / rack button `i`: column 1 is the left cheek. */
  const col = (i: number) => String(i + 2)
</script>

<section class="panel mat-chassis" aria-label="Knobs and Quick Racks">
  <span class="seam left" aria-hidden="true"></span>
  <!-- A waiting Store's prompt runs across the right cheek, so its seam goes. -->
  {#if !ask}<span class="seam right" aria-hidden="true"></span>{/if}

  <!-- Row 1: the knobs. -->
  <div class="pager knobs-pager" role="group" aria-label="Knob page">
    <span class="engraved lbl">Knobs {k.pageNumber}/{k.pageCount}</span>
    <div class="pg">
      <HwButton tip="knobs.page" label="Previous knob page" onclick={() => app.send({ type: 'stepKnobPage', delta: -1 })}>◀</HwButton>
      <span class="readout mat-screen page-name" use:tip={'knobs.page'}><span class="glow-text">{k.pageName}</span></span>
      <HwButton tip="knobs.page" label="Next knob page" onclick={() => app.send({ type: 'stepKnobPage', delta: 1 })}>▶</HwButton>
    </div>
  </div>

  {#each k.knobs as knob, i (i)}
    {@const off = knob.function === 'none'}
    <div class="cell" class:off title={knob.name} style:grid-column={col(i)} data-col={i}>
      <Knob
        label={knob.name}
        level={knob.level}
        disabled={off}
        tipKey="knobs.knob"
        onturn={(delta) => app.send({ type: 'turnKnob', knob: i, delta })}
        onreset={() => app.send({ type: 'resetKnob', knob: i })}
      />
      <div class="text">
        <span class="name engraved">{knob.short}</span>
        <span class="readout mat-screen"><span class="glow-text">{knob.value}</span></span>
      </div>
    </div>
  {/each}

  <!-- Row 2: the Quick Racks, each under its knob. -->
  <div class="pager bank-pager" role="group" aria-label="Quick Racks bank">
    <span class="engraved lbl">Quick Racks</span>
    <div class="pg">
      <HwButton tip="quick.bank_prev" label="Previous bank" onclick={() => app.send({ type: 'stepQuickRackBank', delta: -1 })}>◀</HwButton>
      <span class="readout mat-screen letter" use:tip={'quick.bank'}><span class="glow-text">{bankLetter(q.bank)}</span></span>
      <HwButton tip="quick.bank_next" label="Next bank" onclick={() => app.send({ type: 'stepQuickRackBank', delta: 1 })}>▶</HwButton>
    </div>
  </div>

  {#if ask}
    <div class="ask"><RackPrompt /></div>
  {:else}
    {#each q.buttons as b, i (i)}
      {@const label = quickLabel(q.bank, i)}
      <div class="slot" title="{label}: {quickName(q, i)}" style:grid-column={col(i)} data-col={i}>
        <HwButton tip={QUICK[i]} led={quickLook(q, i)} {beats} label="Quick Rack {label}: {quickName(q, i)}" onclick={() => app.send({ type: 'pressQuickRack', slot: i })}>
          <span class="num">{i + 1}</span>
          <span class="bname" class:empty={!b.rack} class:missing={b.missing}>{quickName(q, i)}</span>
        </HwButton>
        {#if b.rack}
          <button type="button" class="clear" aria-label="Clear Quick Rack {label}" use:tip={'quick.clear'} onclick={() => app.send({ type: 'clearQuickRack', bank: q.bank, slot: i })}>✕</button>
        {/if}
      </div>
    {/each}

    <div class="store">
      <HwButton tip="quick.store" led={q.store ? armed : null} {beats} pressed={q.store} onclick={() => app.send({ type: 'toggleQuickRackStore' })}>Store</HwButton>
    </div>
  {/if}
</section>

<style>
  /* Columns: the left cheek (pagers), the eight knob/rack columns, the right cheek
     (Store). Rows: --row each, one label line (0.78em print, line height 1.2), a 0.2em
     gap and a 1.55em readout. App.svelte's --h counts this panel as --panel-h. */
  .panel {
    --row: calc(0.78em * 1.2 + 0.2em + 1.55em);
    --gap: 1em;
    position: relative;
    display: grid;
    grid-template-columns: 9.6em repeat(8, minmax(0, 1fr)) 5.2em;
    grid-template-rows: var(--row) var(--row);
    column-gap: var(--gap);
    row-gap: 0.55em;
    box-sizing: border-box;
    padding: 0.65em 1.5em;
    border-radius: 1em;
  }
  /* One print size for every label on the panel, as on the mirror. */
  .panel :global(.engraved) {
    font-size: 0.78em;
    line-height: 1.2;
  }

  /* The cheeks' seams run the panel's height, in the middle of the column gap. */
  .seam {
    grid-row: 1 / -1;
    width: 0;
    border-left: 1px solid var(--seam);
    box-shadow: 1px 0 0 rgb(255 255 255 / 0.04);
  }
  .seam.left {
    grid-column: 1;
    justify-self: end;
    margin-right: calc(var(--gap) / -2);
  }
  .seam.right {
    grid-column: 10;
    justify-self: start;
    margin-left: calc(var(--gap) / -2);
  }

  /* ── Pagers: a label over ◀ [readout] ▶, laid out like a knob's text ── */
  .pager {
    grid-column: 1;
    display: flex;
    flex-direction: column;
    gap: 0.2em;
    min-width: 0;
  }
  .knobs-pager {
    grid-row: 1;
  }
  .bank-pager {
    grid-row: 2;
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

  /* ── The knobs: each as tall as its label + readout ── */
  .cell {
    --knob-size: var(--row);
    grid-row: 1;
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
  /* The lit readout: knob values, the knob page and the bank letter alike. */
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

  /* ── The Quick Rack buttons: the rack's name on the button ── */
  .slot {
    grid-row: 2;
    position: relative;
    min-width: 0;
  }
  .slot :global(.hw),
  .store :global(.hw) {
    height: 100%;
  }
  .slot :global(.btn),
  .store :global(.btn) {
    height: 100%;
    min-width: 0;
  }
  .slot :global(.face) {
    max-width: 100%;
    gap: 0.45em;
  }
  .num {
    flex: none;
    font-size: 1.05em;
  }
  .bname {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: var(--font-body);
    font-weight: 500;
    font-size: 0.85em;
    letter-spacing: 0;
  }
  .bname.empty {
    color: var(--muted);
    font-style: italic;
  }
  .bname.missing {
    color: var(--danger);
  }
  /* ✕ clears: a small badge on the button's corner, in the row gap. */
  .clear {
    position: absolute;
    top: -0.45em;
    right: -0.45em;
    z-index: 1;
    display: grid;
    place-items: center;
    width: 1.25em;
    height: 1.25em;
    padding: 0;
    border: 1px solid var(--seam);
    border-radius: 50%;
    background: var(--well);
    color: var(--muted);
    font-size: 0.8em;
    line-height: 1;
    opacity: 0.75;
  }
  .slot:hover .clear,
  .clear:focus-visible {
    opacity: 1;
    color: var(--ink);
  }
  .store {
    grid-column: 10;
    grid-row: 2;
    min-width: 0;
  }
  /* A Store waiting for a save asks across the rack row (Store's own cheek included:
     Cancel is in the prompt). */
  .ask {
    grid-column: 2 / -1;
    grid-row: 2;
    align-self: center;
    min-width: 0;
    font-size: 0.9em;
  }

  /* Tall windows: the stage is 66em wide, too narrow for the cheeks beside the eight
     columns. The cheek columns fold to nothing and their contents become a header row
     (pagers with their labels beside them, then Store), so the columns keep the width;
     the columns stay the same grid lines, so knob n is still over rack n. */
  @container stage (aspect-ratio < 1.45) {
    .panel {
      --gap: 0.7em;
      grid-template-columns: 0 repeat(8, minmax(0, 1fr)) 0;
      grid-template-rows: 1.55em var(--row) var(--row);
      padding: 0.65em 0.8em;
    }
    .seam {
      display: none;
    }
    .pager {
      grid-row: 1;
      flex-direction: row;
      align-items: center;
      gap: 0.6em;
    }
    .knobs-pager {
      grid-column: 2 / 6;
    }
    .bank-pager {
      grid-column: 6 / 9;
    }
    .lbl {
      flex: none;
    }
    .pg {
      flex: 0 1 11em;
    }
    .cell {
      grid-row: 2;
      gap: 0.3em;
    }
    .slot,
    .ask {
      grid-row: 3;
    }
    .store {
      grid-row: 1;
      grid-column: 9;
    }
    .store :global(.btn) {
      font-size: 0.8em;
    }
    .slot :global(.face) {
      gap: 0.3em;
    }
  }
</style>
