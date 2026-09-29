<!--
  The question a Quick Rack Store can ask, kept compact for the Quick Racks bar. The rack
  prompts (`liveRack.prompt`: unsaved changes, sound names) are asked in ONE place, the
  Rack panel (panels/rack), which the bar opens when one appears (QuickBar).

  - Store waiting (`quickRacks.storeWaiting`): the rack's name when it has never been
    saved → saveRack / saveRackAs; the engine then stores it on the waiting button

  Renders nothing when nothing is asked (`asking()`).
-->
<script lang="ts" module>
  import type { AppState } from '../../lib/api/types'

  /** Something waits for the player's answer. */
  export function asking(st: AppState): boolean {
    return st.quickRacks.storeWaiting !== null
  }
</script>

<script lang="ts">
  import { quickLabel } from '../../lib/api/quick-racks'
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'

  const live = $derived(app.state.liveRack)
  const q = $derived(app.state.quickRacks)

  /** The rack name typed for a Store waiting on a never-saved rack (null: the live name). */
  let rackName = $state<string | null>(null)

  function saveForStore() {
    const name = (rackName ?? live.name).trim()
    rackName = null
    app.send(live.id ? { type: 'saveRack' } : { type: 'saveRackAs', name })
  }
</script>

{#if q.storeWaiting !== null}
  <div class="prompt" role="group" aria-label="Save the rack to store it">
    {#if live.id}
      <span class="text">Save <b>{live.name}</b> to store it on {quickLabel(q.bank, q.storeWaiting)}.</span>
    {:else}
      <span class="text">Save the rack to store it on {quickLabel(q.bank, q.storeWaiting)}:</span>
      <input
        class="field"
        type="text"
        aria-label="Rack name"
        value={rackName ?? live.name}
        use:tip={'quick.save_name'}
        oninput={(e) => (rackName = e.currentTarget.value)}
        onkeydown={(e) => e.key === 'Enter' && saveForStore()}
      />
    {/if}
    <HwButton tip="quick.save" onclick={saveForStore}>Save rack</HwButton>
    <HwButton tip="quick.cancel_store" onclick={() => ((rackName = null), app.send({ type: 'toggleQuickRackStore' }))}>Cancel</HwButton>
  </div>
{/if}

<style>
  .prompt {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4em 0.5em;
    min-width: 0;
    padding: 0.3em 0.6em;
    border: 1px solid var(--accent);
    border-radius: 0.3em;
  }
  .text {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }
  .field {
    width: 10em;
    min-width: 0;
    height: 2em;
    padding: 0 0.4em;
    border: 1px solid var(--seam);
    border-radius: 0.25em;
    background: var(--well);
    color: var(--ink);
    font: inherit;
  }
</style>
