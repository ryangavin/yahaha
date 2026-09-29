<!--
  The questions a rack switch or a Quick Rack Store can ask, kept compact for the Quick
  Racks bar (the full Rack panel comes later, docs/racks.md item 11). One at a time:

  - unsaved changes (`liveRack.prompt`): [Save first] [Discard and switch] [Keep editing]
  - sound names (`liveRack.prompt`): a name per edited preset → the same save again
  - Store waiting (`quickRacks.storeWaiting`): the rack's name when it has never been
    saved → saveRack / saveRackAs; the engine then stores it on the waiting button

  Renders nothing when nothing is asked (`asking()`).
-->
<script lang="ts" module>
  import type { AppState } from '../../lib/api/types'

  /** Something waits for the player's answer. */
  export function asking(st: AppState): boolean {
    return st.liveRack.prompt !== null || st.quickRacks.storeWaiting !== null
  }
</script>

<script lang="ts">
  import { quickLabel } from '../../lib/api/quick-racks'
  import { KEYBOARD_PART_NAMES } from '../../lib/api/types'
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import { guard } from './guard.svelte'

  const live = $derived(app.state.liveRack)
  const q = $derived(app.state.quickRacks)
  const prompt = $derived(live.prompt)

  /** Names typed for the new sounds, by part; untouched ones keep the suggestion. */
  let soundNames = $state<Record<number, string>>({})
  /** The rack name typed for a Store waiting on a never-saved rack (null: the live name). */
  let rackName = $state<string | null>(null)

  function saveWithNames() {
    if (prompt?.kind !== 'soundNames') return
    const names = Object.fromEntries(prompt.parts.map((p) => [p.part, (soundNames[p.part] ?? p.suggested).trim()]))
    soundNames = {}
    app.send(prompt.saveAs === null ? { type: 'saveRack', soundNames: names } : { type: 'saveRackAs', name: prompt.saveAs, soundNames: names })
  }

  function cancelNames() {
    soundNames = {}
    guard.keepEditing()
  }

  function saveForStore() {
    const name = (rackName ?? live.name).trim()
    rackName = null
    app.send(live.id ? { type: 'saveRack' } : { type: 'saveRackAs', name })
  }
</script>

{#if prompt?.kind === 'unsavedChanges'}
  {@const then = prompt.then}
  <div class="prompt" role="group" aria-label="Unsaved changes">
    <span class="text">
      <b>{live.name}</b> has unsaved changes. {then.kind === 'load' ? `Switch to ${then.name}?` : 'Start a new rack?'}
    </span>
    <HwButton tip="quick.save_first" onclick={() => guard.saveFirst(then)}>Save first</HwButton>
    <HwButton tip="quick.discard" onclick={() => guard.discard(then)}>Discard and switch</HwButton>
    <HwButton tip="quick.keep_editing" onclick={() => guard.keepEditing()}>Keep editing</HwButton>
  </div>
{:else if prompt?.kind === 'soundNames'}
  <div class="prompt" role="group" aria-label="Name the new sounds">
    <span class="text">Name the new sounds:</span>
    {#each prompt.parts as p (p.part)}
      <label class="named">
        <span class="engraved">{KEYBOARD_PART_NAMES[p.part]}</span>
        <input
          class="field"
          type="text"
          aria-label="New sound name for {KEYBOARD_PART_NAMES[p.part]}"
          value={soundNames[p.part] ?? p.suggested}
          use:tip={'quick.sound_name'}
          oninput={(e) => (soundNames[p.part] = e.currentTarget.value)}
        />
      </label>
    {/each}
    <HwButton tip="quick.sound_names_save" onclick={saveWithNames}>Save</HwButton>
    <HwButton tip="quick.sound_names_cancel" onclick={cancelNames}>Cancel</HwButton>
  </div>
{:else if q.storeWaiting !== null}
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
  .named {
    display: inline-flex;
    align-items: center;
    gap: 0.3em;
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
