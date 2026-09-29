<!--
  The Quick Racks drawer (right side, open while `ui.quick`; nav "Quick Racks", Alt+R): a
  small stand-in for Library › Racks, which comes later (docs/racks.md items 9–10).

  - Your racks: click one to load it (through the unsaved-changes guard).
  - Bank A–H: the eight buttons of the bank on view. A button loads its rack, or stores
    the live rack while Store is armed; Clear empties it.

  A question from a switch or a Store (RackPrompt) shows here while the drawer is open.

  State: racks, liveRack, quickRacks. Commands: loadRack, pressQuickRack,
  stepQuickRackBank, toggleQuickRackStore, clearQuickRack.
-->
<script lang="ts">
  import { QUICK } from '../../help/actions'
  import { QUICK_BANKS, bankLetter, quickLabel, quickLook, quickName } from '../../lib/api/quick-racks'
  import { app, clock, ui } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import Overlay from '../../lib/ui/Overlay.svelte'
  import RackPrompt from './RackPrompt.svelte'

  const q = $derived(app.state.quickRacks)
  const live = $derived(app.state.liveRack)
  const racks = $derived(app.state.racks)
  const beats = $derived(clock.beats)
  const armed = { rgb: [127, 0, 0] as [number, number, number], level: 'bright' as const, anim: 'flash' as const }

  /** View bank `b`: Bank −/+ as many times as it takes (they clamp, so order doesn't matter). */
  function viewBank(b: number) {
    const d = b - q.bank
    for (let i = 0; i < Math.abs(d); i++) app.send({ type: 'stepQuickRackBank', delta: Math.sign(d) })
  }
</script>

<Overlay id="quick" title="Quick Racks" closeTip="drawer.close" onclose={() => (ui.quick = false)}>
  <p class="note">Your racks and the Quick Rack buttons, until Library › Racks arrives.</p>

  <RackPrompt />

  <section class="block" aria-label="Your racks">
    <h3 class="engraved">Your racks</h3>
    {#if racks.length}
      <ul class="racks">
        {#each racks as r (r.id)}
          <li>
            <button type="button" class="rack mat-raised" class:live={r.id === live.id} use:tip={'quick.rack'} onclick={() => app.send({ type: 'loadRack', id: r.id })}>
              <span class="rname">{r.name}{r.id === live.id && live.modified ? ' *' : ''}</span>
              <span class="parts">{r.parts.filter((_, i) => r.on[i]).join(' + ')}</span>
            </button>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="note">No racks yet. Press Store, then a Quick Rack button, to save the live rack ({live.name}) and put it there.</p>
    {/if}
  </section>

  <section class="block" aria-label="Quick Racks bank">
    <div class="row">
      <h3 class="engraved">Bank</h3>
      <HwButton tip="quick.bank_prev" label="Previous bank" onclick={() => app.send({ type: 'stepQuickRackBank', delta: -1 })}>◀</HwButton>
      <div class="banks" role="group" aria-label="Banks A to H">
        {#each Array.from({ length: QUICK_BANKS }, (_, b) => b) as b (b)}
          <button type="button" class="bank" class:on={b === q.bank} aria-pressed={b === q.bank} use:tip={'quick.bank'} onclick={() => viewBank(b)}>{bankLetter(b)}</button>
        {/each}
      </div>
      <HwButton tip="quick.bank_next" label="Next bank" onclick={() => app.send({ type: 'stepQuickRackBank', delta: 1 })}>▶</HwButton>
      <HwButton tip="quick.store" led={q.store ? armed : null} {beats} pressed={q.store} onclick={() => app.send({ type: 'toggleQuickRackStore' })}>Store</HwButton>
    </div>
    {#if q.readOnly}<p class="note">Quick Racks can't be changed in this session.</p>{/if}
    <ol class="buttons">
      {#each q.buttons as b, i (i)}
        {@const label = quickLabel(q.bank, i)}
        <li class="button" class:current={b.loaded}>
          <HwButton tip={QUICK[i]} led={quickLook(q, i)} {beats} shape="square" label="Quick Rack {label}" onclick={() => app.send({ type: 'pressQuickRack', slot: i })}>{label}</HwButton>
          <span class="info" class:empty={!b.rack} class:missing={b.missing}>{b.missing ? 'missing (its rack is gone)' : quickName(q, i)}</span>
          {#if b.rack}<HwButton tip="quick.clear" label="Clear Quick Rack {label}" onclick={() => app.send({ type: 'clearQuickRack', bank: q.bank, slot: i })}>Clear</HwButton>{/if}
        </li>
      {/each}
    </ol>
  </section>
</Overlay>

<style>
  .block {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin: 0.8rem 0 1rem;
  }
  h3 {
    margin: 0;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.45rem;
  }
  .note {
    margin: 0 0 0.6rem;
    color: var(--muted);
    font-size: 0.85rem;
  }
  .racks,
  .buttons {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .rack {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.1rem;
    width: 100%;
    padding: 0.4rem 0.6rem;
    border-radius: 5px;
    color: var(--ink);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .rack.live {
    outline: 1px solid var(--accent);
  }
  .rack.live .rname {
    color: var(--accent);
  }
  .rname {
    font-weight: 600;
  }
  .parts {
    color: var(--muted);
    font-size: 0.8rem;
  }
  .banks {
    display: flex;
    gap: 0.2rem;
  }
  .bank {
    width: 1.7rem;
    height: 1.9rem;
    border: 1px solid var(--seam);
    border-radius: 4px;
    background: var(--well);
    color: var(--muted);
    font-family: var(--font-display);
    font-weight: 600;
    cursor: pointer;
  }
  .bank.on {
    border-color: var(--accent);
    color: var(--accent);
  }
  .button {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.25rem;
    border-radius: 5px;
  }
  .button.current {
    outline: 1px solid var(--accent);
  }
  .info {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .info.empty {
    color: var(--muted);
    font-style: italic;
  }
  .info.missing {
    color: var(--danger);
  }
</style>
