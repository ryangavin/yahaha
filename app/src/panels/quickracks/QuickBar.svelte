<!--
  The Quick Racks bar, in the keyboard strip's panel above the keys where the Genos has its
  Registration buttons (docs/racks.md): one press loads one of your racks. It is on the
  stage, so every size is in em of `--u`.

  [Quick Racks ◀ A ▶] [1]…[8] [Store]        (each button: its rack's name under it, ✕ clears)

  The buttons light like pad page 4 (red loaded, blue stored, dark empty; all flashing
  while Store is armed). When a switch or a Store asks something (unsaved changes, sound
  names, a rack to save first) the question takes the buttons' place (RackPrompt), unless
  the Quick Racks drawer is open and asks it there.

  State: quickRacks, liveRack. Commands: pressQuickRack, stepQuickRackBank,
  toggleQuickRackStore, clearQuickRack (and the guard's answers, guard.svelte.ts).
-->
<script lang="ts">
  import { QUICK } from '../../help/actions'
  import { bankLetter, quickLabel, quickLook, quickName } from '../../lib/api/quick-racks'
  import { app, clock, ui } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import { guard } from './guard.svelte'
  import RackPrompt, { asking } from './RackPrompt.svelte'

  const q = $derived(app.state.quickRacks)
  const beats = $derived(clock.beats)
  const armed = { rgb: [127, 0, 0] as [number, number, number], level: 'bright' as const, anim: 'flash' as const }
  const ask = $derived(asking(app.state) && !ui.quick)

  // A Save first that has gone through sends the switch it was for.
  $effect(() => guard.settle(app.state))
</script>

<section class="qbar" aria-label="Quick Racks">
  <div class="group">
    <span class="engraved lbl">Quick Racks</span>
    <HwButton tip="quick.bank_prev" label="Previous bank" onclick={() => app.send({ type: 'stepQuickRackBank', delta: -1 })}>◀</HwButton>
    <span class="letter mat-screen" use:tip={'quick.bank'}><span class="glow-text">{bankLetter(q.bank)}</span></span>
    <HwButton tip="quick.bank_next" label="Next bank" onclick={() => app.send({ type: 'stepQuickRackBank', delta: 1 })}>▶</HwButton>
  </div>

  {#if ask}
    <div class="ask"><RackPrompt /></div>
  {:else}
    <div class="group buttons" role="group" aria-label="Quick Racks 1 to 8">
      {#each q.buttons as b, i (i)}
        {@const label = quickLabel(q.bank, i)}
        <div class="slot" title="{label}: {quickName(q, i)}">
          <HwButton tip={QUICK[i]} led={quickLook(q, i)} {beats} shape="square" label="Quick Rack {label}" onclick={() => app.send({ type: 'pressQuickRack', slot: i })}>{i + 1}</HwButton>
          <span class="bname" class:empty={!b.rack} class:missing={b.missing} class:loaded={b.loaded}>
            <span class="txt">{quickName(q, i)}</span>
            {#if b.rack}
              <button type="button" class="clear" aria-label="Clear Quick Rack {label}" use:tip={'quick.clear'} onclick={() => app.send({ type: 'clearQuickRack', bank: q.bank, slot: i })}>✕</button>
            {/if}
          </span>
        </div>
      {/each}
    </div>

    <div class="group">
      <HwButton tip="quick.store" led={q.store ? armed : null} {beats} pressed={q.store} onclick={() => app.send({ type: 'toggleQuickRackStore' })}>Store</HwButton>
    </div>
  {/if}
</section>

<style>
  /* One row across the strip (93em wide); in the stacked layout (63em) it wraps to two. */
  .qbar {
    display: flex;
    flex-wrap: nowrap;
    align-items: flex-start;
    justify-content: flex-start;
    column-gap: 1.4em;
    row-gap: 0.4em;
    min-width: 0;
  }
  /* Level with the middle of the square 1–8 buttons (2.6em of their 0.9em print). */
  .group {
    display: flex;
    align-items: center;
    gap: 0.35em;
    min-width: 0;
    min-height: 2.34em;
    flex: 0 1 auto;
  }
  .buttons {
    flex: none;
    align-items: flex-start;
    min-height: 0;
    gap: 0.45em;
  }
  .ask {
    flex: 1 1 auto;
    min-width: 0;
    font-size: 0.9em;
  }
  .lbl {
    white-space: nowrap;
  }
  .slot {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.15em;
    width: 5.4em;
  }
  .bname {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.2em;
    width: 100%;
    min-height: 1em;
    font-size: 0.62em;
    line-height: 1;
    color: var(--ink);
  }
  .txt {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bname.empty {
    color: var(--muted);
    font-style: italic;
  }
  .bname.missing {
    color: var(--danger);
  }
  .bname.loaded {
    color: var(--accent);
  }
  .clear {
    flex: none;
    padding: 0 0.15em;
    border: 0;
    background: none;
    color: var(--muted);
    font: inherit;
    line-height: 1;
    cursor: pointer;
  }
  .clear:hover,
  .clear:focus-visible {
    color: var(--ink);
  }
  .letter {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 2em;
    height: 2.2em;
    border-radius: 0.3em;
    font-family: var(--font-display);
    font-size: 0.95em;
  }
  @container stage (aspect-ratio < 1.45) {
    .qbar {
      flex-wrap: wrap;
    }
  }
</style>
