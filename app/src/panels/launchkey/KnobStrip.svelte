<!--
  The Launchkey's eight encoders and its Knob Assign ▲/▼ buttons, clickable: the page
  steps with ◀ ▶, and each knob turns one step with − / +. The knobs are relative, so
  this sends the same turnKnob a hardware turn does. Nothing here is hardware-only.

  State: knobs. Commands: stepKnobPage, turnKnob.
-->
<script lang="ts">
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'

  const k = $derived(app.state.knobs)
</script>

<div class="knobs mat-well" role="group" aria-label="Knobs: {k.pageName}">
  <div class="page">
    <button type="button" class="mini mat-raised" aria-label="Previous knob page" use:tip={'knobs.page'} onclick={() => app.send({ type: 'stepKnobPage', delta: -1 })}>◀</button>
    <span class="engraved">Knobs {k.pageNumber}/{k.pageCount} · {k.pageName}</span>
    <button type="button" class="mini mat-raised" aria-label="Next knob page" use:tip={'knobs.page'} onclick={() => app.send({ type: 'stepKnobPage', delta: 1 })}>▶</button>
  </div>
  {#each k.knobs as knob, i (i)}
    <div class="knob" title={knob.name}>
      <span class="engraved">{knob.short}</span>
      <span class="val">{knob.value}</span>
      <span class="turn">
        <button type="button" class="mini mat-raised" aria-label="{knob.name} down" aria-disabled={knob.function === 'none' || undefined} use:tip={'knobs.knob'} onclick={() => app.send({ type: 'turnKnob', knob: i, delta: -1 })}>−</button>
        <button type="button" class="mini mat-raised" aria-label="{knob.name} up" aria-disabled={knob.function === 'none' || undefined} use:tip={'knobs.knob'} onclick={() => app.send({ type: 'turnKnob', knob: i, delta: 1 })}>+</button>
      </span>
    </div>
  {/each}
</div>

<style>
  .knobs {
    display: grid;
    grid-template-columns: auto repeat(8, minmax(0, 1fr));
    gap: 3px;
    padding: 3px 6px;
    margin-top: 4px;
    border-radius: 6px;
    align-items: center;
    font-size: 0.75rem;
  }
  .page,
  .knob {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    min-width: 0;
  }
  .page {
    flex-direction: row;
  }
  .val {
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }
  .turn {
    display: flex;
    gap: 2px;
  }
  .mini {
    min-width: 1.6rem;
    border-radius: 4px;
    color: var(--ink);
  }
</style>
