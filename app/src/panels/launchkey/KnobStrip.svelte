<!--
  The Launchkey's eight encoders and its Knob Assign ▲/▼ buttons, clickable: the page
  steps with ◀ ▶, and each knob is a rotary Knob (drag, wheel or arrow keys). The knobs are relative, so
  this sends the same turnKnob a hardware turn does. Nothing here is hardware-only.
  Each knob has its label and readout stacked to its right, printed like the fader bank's.

  State: knobs. Commands: stepKnobPage, turnKnob, resetKnob (double-click).
-->
<script lang="ts">
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import Knob from '../../lib/ui/Knob.svelte'

  const k = $derived(app.state.knobs)
</script>

<div class="knobs mat-chassis" role="group" aria-label="Knobs: {k.pageName}">
  <div class="page">
    <button type="button" class="mini mat-raised" aria-label="Previous knob page" use:tip={'knobs.page'} onclick={() => app.send({ type: 'stepKnobPage', delta: -1 })}>◀</button>
    <span class="engraved page-name">Knobs {k.pageNumber}/{k.pageCount} · {k.pageName}</span>
    <button type="button" class="mini mat-raised" aria-label="Next knob page" use:tip={'knobs.page'} onclick={() => app.send({ type: 'stepKnobPage', delta: 1 })}>▶</button>
  </div>
  {#each k.knobs as knob, i (i)}
    {@const off = knob.function === 'none'}
    <div class="cell" class:off title={knob.name}>
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
</div>

<style>
  /* Each knob is as tall as its label + readout stack, and the bar is that row plus
     padding (app.css's --knob-size-bar and --knob-bar-h; App.svelte's --fixed counts on
     the bar's height plus its 4px margin). */
  .knobs {
    --knob-size: var(--knob-size-bar);
    display: grid;
    grid-template-columns: auto repeat(8, minmax(0, 1fr));
    column-gap: 0.6rem;
    box-sizing: border-box;
    height: var(--knob-bar-h);
    padding: 0.5rem 0.8rem;
    margin-top: 4px;
    border-radius: var(--r-panel);
    align-items: center;
  }
  .page {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding-right: 0.6rem;
    border-right: 1px solid var(--seam);
    box-shadow: 1px 0 0 rgb(255 255 255 / 0.04);
    align-self: stretch;
  }
  /* Two lines ("Knobs 1/6 ·" over the page name), so the knobs get the width. */
  .page-name {
    width: 4.6rem;
    text-align: center;
    line-height: 1.25;
    overflow-wrap: anywhere;
  }
  .mini {
    min-width: 1.6rem;
    height: 1.6rem;
    padding: 0;
    border-radius: 4px;
    color: var(--ink);
    font-size: 0.7rem;
  }
  .cell {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    min-width: 0;
  }
  .text {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0.3rem;
    min-width: 0;
    flex: 1;
  }
  .name,
  .readout {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* Line height and the readout's height add up to --knob-size-bar in app.css. */
  .name {
    line-height: 1.2;
  }
  /* The fader bank's readout: a small screen, lit text. */
  .readout {
    display: block;
    height: 1.45rem;
    padding: 0 0.3rem;
    border-radius: 4px;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.85rem;
    line-height: calc(1.45rem - 2px);
    text-align: center;
  }
  .off .name,
  .off .readout {
    opacity: 0.45;
  }
  .off .readout {
    color: var(--screen-dim);
  }
</style>
