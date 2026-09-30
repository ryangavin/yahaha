<!--
  The stage's Quick Racks row, between the mixer row and the keyboard strip
  (docs/racks.md): the bank pager on the left cheek, the eight Quick Rack buttons in
  eight columns, Store on the right cheek. The columns are the grid the knobs used to
  share; the knobs now live in the Launchkey mirror (panels/launchkey).

   ┌ cheek ────────────┬ col 1 ──────────┬ col 2 ─ … ─ col 8 ┬ cheek ─┐
   │ QUICK RACKS       │ ┌─────────────┐ │                   │ ┌─────┐│
   │ [◀][   A    ][▶]  │ │ 1  Ballad  ✕│ │   …               │ │Store││
   └───────────────────┴─┴─────────────┴─┴───────────────────┴─┴─────┴┘

  One press loads a rack; the buttons light like pad page 4 (red loaded, blue stored,
  dark empty; all flashing while Store is armed), the rack's name printed on the button,
  ✕ clears it. Storing, as on the Launchkey: Store then a button (armed, the button sends
  pressQuickRack as the Store pad then a rack pad do), or in one go, a long press or
  right-click on a button (storeRack, as hold Sound + tap a Racks pad; quickracks/storeHold). A Store waiting for a save asks in the buttons' place (RackPrompt), and a
  rack prompt that appears opens the Rack drawer (openRackDrawerOnPrompt). Library has
  its own one-row bar of the same buttons (panels/quickracks/QuickBar). Nothing here is
  hardware-only.

  Every size is in em of the stage's `--u` (App.svelte). The row is --row tall: one
  label line, a small gap and a readout, so the pager's label sits over ◀ A ▶ and the
  buttons are as tall as the two; with the padding the panel is 3.99em.

  State: quickRacks, liveRack. Commands: pressQuickRack, stepQuickRackBank,
  toggleQuickRackStore, storeRack, clearQuickRack.
-->
<script lang="ts">
  import { QUICK } from '../../help/actions'
  import { bankLetter, quickLabel, quickLook, quickName } from '../../lib/api/quick-racks'
  import { app, clock } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import { openRackDrawerOnPrompt } from '../quickracks/rackPromptDrawer.svelte'
  import RackPrompt, { asking } from '../quickracks/RackPrompt.svelte'
  import { storeHold } from '../quickracks/storeHold'

  const q = $derived(app.state.quickRacks)
  const beats = $derived(clock.beats)
  const armed = { rgb: [127, 0, 0] as [number, number, number], level: 'bright' as const, anim: 'flash' as const }
  const ask = $derived(asking(app.state))

  openRackDrawerOnPrompt()

  /** The grid column of rack button `i`: column 1 is the left cheek. */
  const col = (i: number) => String(i + 2)
</script>

<section class="panel mat-chassis" aria-label="Quick Racks">
  <span class="seam left" aria-hidden="true"></span>
  <!-- A waiting Store's prompt runs across the right cheek, so its seam goes. -->
  {#if !ask}<span class="seam right" aria-hidden="true"></span>{/if}

  <div class="pager" role="group" aria-label="Quick Racks bank">
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
        <div class="hold" use:storeHold={() => app.send({ type: 'storeRack', slot: i })}>
          <HwButton tip={QUICK[i]} led={quickLook(q, i)} {beats} label="Quick Rack {label}: {quickName(q, i)}" onclick={() => app.send({ type: 'pressQuickRack', slot: i })}>
            <span class="num">{i + 1}</span>
            <span class="bname" class:empty={!b.rack} class:missing={b.missing}>{quickName(q, i)}</span>
          </HwButton>
        </div>
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
  /* Columns: the left cheek (the bank pager), the eight rack columns, the right cheek
     (Store). One row, --row tall: one label line (0.78em print, line height 1.2), a 0.2em
     gap and a 1.55em readout. With 0.65em of padding above and below, the panel is
     2.686em + 1.3em = 3.99em of the stage's --u. The stage is always at least 96em wide
     (App.svelte's --u), so the cheeks fit beside the columns on every window. */
  .panel {
    --row: calc(0.78em * 1.2 + 0.2em + 1.55em);
    --gap: 1em;
    position: relative;
    flex: none;
    display: grid;
    grid-template-columns: 9.6em repeat(8, minmax(0, 1fr)) 5.2em;
    grid-template-rows: var(--row);
    column-gap: var(--gap);
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
    grid-row: 1;
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

  /* ── The bank pager: a label over ◀ [letter] ▶ ── */
  .pager {
    grid-column: 1;
    grid-row: 1;
    display: flex;
    flex-direction: column;
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
  /* The lit bank letter. */
  .readout {
    display: block;
    flex: 1;
    box-sizing: border-box;
    height: 1.55em;
    padding: 0 0.3em;
    border-radius: var(--r-key);
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 1em;
    line-height: calc(1.55em - 2px);
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
  }
  .readout > span {
    font-size: 0.9em;
  }

  /* ── The Quick Rack buttons: the rack's name on the button ── */
  .slot {
    grid-row: 1;
    position: relative;
    min-width: 0;
  }
  /* The long-press wrapper takes no box: the button sizes to the slot. No callout or
     selection on a touch long press. */
  .hold {
    display: contents;
  }
  .slot {
    -webkit-touch-callout: none;
    user-select: none;
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
  /* ✕ clears: a small badge on the button's corner, in the panel's padding. */
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
    grid-row: 1;
    min-width: 0;
  }
  /* A Store waiting for a save asks across the rack columns (Store's own cheek included:
     Cancel is in the prompt), in the row's height. */
  .ask {
    grid-column: 2 / -1;
    grid-row: 1;
    align-self: center;
    min-width: 0;
    font-size: 0.9em;
  }
</style>
