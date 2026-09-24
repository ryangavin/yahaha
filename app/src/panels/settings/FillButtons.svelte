<!--
  The Genos assignable fill functions as four buttons: Fill Down, Fill Self, Fill Up,
  Fill Break. Fill Self and Fill Break only do something while the band plays.
-->
<script lang="ts">
  import type { TipKey } from '../../help/tooltips'
  import type { AppCmd } from '../../lib/api/types'
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'

  const running = $derived(app.state.transport.running)
  const buttons: { label: string; cmd: AppCmd; tip: TipKey; playing: boolean }[] = [
    { label: '◀ Fill Down', cmd: { type: 'fillDown' }, tip: 'transport.fill_down', playing: false },
    { label: 'Fill Self', cmd: { type: 'fillSelf' }, tip: 'transport.fill_self', playing: true },
    { label: 'Fill Up ▶', cmd: { type: 'fillUp' }, tip: 'transport.fill_up', playing: false },
    { label: 'Break', cmd: { type: 'fillBreak' }, tip: 'transport.fill_break', playing: true },
  ]
</script>

<div class="fills mat-well" role="group" aria-label="Fills">
  {#each buttons as b (b.tip)}
    {@const off = b.playing && !running}
    <button type="button" class="fill mat-raised" class:off aria-disabled={off || undefined} use:tip={b.tip} onclick={() => !off && app.send(b.cmd)}>
      {b.label}
    </button>
  {/each}
</div>

<style>
  .fills {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 3px;
    padding: 3px;
    border-radius: 6px;
  }
  .fill {
    min-height: 2.2rem;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    color: var(--ink);
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.85rem;
  }
  .fill.off {
    opacity: 0.45;
    cursor: default;
  }
</style>
