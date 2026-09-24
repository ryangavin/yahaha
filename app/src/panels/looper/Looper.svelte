<!--
  The Chord Looper drawer (Genos Menu › Chord Looper, RM p.14–19): record the chords you
  play, loop them, keep them in eight memories. docs/chord-looper.md.

  1. REC/STOP and ON/OFF, lit as on the Genos panel: REC red (flashing: waits for the bar
     line), ON/OFF orange (flashing: waits for the bar line), blue when there is a sequence
     but it isn't looping.
  2. The sequence, bar by bar, with the bar playing (or being recorded) lit.
  3. Memories 1–8: select one to loop it (while looping, from the next bar line). Memory /
     Clear, then a number, store or empty one; New bank empties them all.

  State: looper, transport.running. Commands: looperRec, looperOnOff, selectLooperMemory,
  storeLooperMemory, clearLooperMemory, newLooperBank. The engine owns every value;
  the Memory/Clear latch is the only UI state.
-->
<script lang="ts">
  import type { LoopChord, LooperMode } from '../../lib/api/types'
  import { app, clock, ui } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import Overlay from '../../lib/ui/Overlay.svelte'
  import { barsOf, beatText, looperLeds, modeText } from './looper'

  const lp = $derived(app.state.looper)
  const running = $derived(app.state.transport.running)
  const leds = $derived(looperLeds(lp))
  const bars = $derived(barsOf(lp))

  /** Memory / Clear pressed: the next memory number stores or clears. */
  let pick = $state<'store' | 'clear' | null>(null)

  function memory(i: number) {
    if (pick === 'store') app.send({ type: 'storeLooperMemory', index: i })
    else if (pick === 'clear') app.send({ type: 'clearLooperMemory', index: i })
    else app.send({ type: 'selectLooperMemory', index: i })
    pick = null
  }

  const summary = (chords: LoopChord[]) => chords.map((c) => c.chord).join(' · ')
  const recording = (m: LooperMode) => m === 'recording' || m === 'recArmed'
</script>

<Overlay id="looper" title="Chord Looper" closeTip="drawer.close" onclose={() => (ui.looper = false)}>
  <div class="looper">
    <section class="panel" aria-label="Chord Looper buttons">
      <div class="buttons">
        <HwButton tip="looper.rec" led={leds.rec} beats={clock.beats} onclick={() => app.send({ type: 'looperRec' })} width="7.5rem">
          REC/STOP
        </HwButton>
        <HwButton tip="looper.on_off" led={leds.onOff} beats={clock.beats} onclick={() => app.send({ type: 'looperOnOff' })} width="7.5rem">
          ON/OFF
        </HwButton>
      </div>
      <p class="status" aria-live="polite">
        <b>{modeText(lp.mode, running)}</b>
        {#if lp.bar !== null}
          <span class="pos">bar {lp.bar}{lp.mode === 'looping' ? ` of ${lp.bars}` : ''}</span>
        {/if}
      </p>
    </section>

    <section aria-label="The sequence">
      <h3 class="engraved">Sequence{lp.bars && lp.mode !== 'recording' ? ` · ${lp.bars} bar${lp.bars === 1 ? '' : 's'}` : ''}</h3>
      <div class="bars" use:tip={'looper.sequence'}>
        {#if lp.mode === 'recording'}
          <span class="empty">Recording… play the chords in time with the band.</span>
        {:else if !bars.length}
          <span class="empty">Nothing recorded yet. Press REC/STOP, then play a chord progression.</span>
        {:else}
          {#each bars as b (b.bar)}
            <div class="bar" class:now={lp.mode === 'looping' && lp.bar === b.bar}>
              <span class="n">{b.bar}</span>
              {#each b.chords as c (c.beat)}
                <span class="chord">{c.chord}{#if c.beat !== 1}<small>{beatText(c.beat)}</small>{/if}</span>
              {:else}
                <span class="hold">%</span>
              {/each}
            </div>
          {/each}
        {/if}
      </div>
    </section>

    <section aria-label="Memories">
      <h3 class="engraved">Memories</h3>
      <div class="memories">
        {#each lp.memories as m, i (i)}
          <button
            type="button"
            class="mem mat-raised"
            class:selected={lp.memory === i}
            class:pending={lp.pendingMemory === i}
            class:vacant={!m.name}
            class:picking={pick !== null}
            disabled={pick === null && recording(lp.mode)}
            use:tip={'looper.memory'}
            onclick={() => memory(i)}
          >
            <span class="num">{i + 1}</span>
            <span class="name">{m.name ?? 'empty'}{m.name ? ` · ${m.bars} bar${m.bars === 1 ? '' : 's'}` : ''}</span>
            <span class="sum">{summary(m.chords)}</span>
          </button>
        {/each}
      </div>
      <div class="ops">
        <HwButton tip="looper.store" pressed={pick === 'store'} onclick={() => (pick = pick === 'store' ? null : 'store')}>Memory</HwButton>
        <HwButton tip="looper.clear" pressed={pick === 'clear'} onclick={() => (pick = pick === 'clear' ? null : 'clear')}>Clear</HwButton>
        <HwButton tip="looper.new_bank" onclick={() => app.send({ type: 'newLooperBank' })}>New bank</HwButton>
        {#if pick}<span class="hint">{pick === 'store' ? 'Store the sequence in memory…' : 'Clear memory…'}</span>{/if}
      </div>
    </section>
  </div>
</Overlay>

<style>
  .looper {
    display: grid;
    gap: 1rem;
  }
  h3 {
    margin: 0 0 0.4rem;
    font-size: 0.8rem;
    font-weight: 600;
    letter-spacing: 0.06em;
  }
  .panel {
    display: grid;
    gap: 0.5rem;
  }
  .buttons {
    display: flex;
    gap: 0.6rem;
    font-size: 1rem;
  }
  .status {
    margin: 0;
    display: flex;
    gap: 0.6rem;
    align-items: baseline;
    font-size: var(--fs-small);
    color: var(--muted);
  }
  .status b {
    color: var(--ink);
    font-weight: 600;
  }
  .pos {
    font-family: var(--font-display);
    color: var(--ink);
  }
  .bars {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(6.5rem, 1fr));
    gap: 0.3rem;
    min-height: 2.6rem;
    padding: 0.4rem;
    border-radius: var(--r-key);
    background: rgb(0 0 0 / 0.12);
    box-shadow: inset 0 1px 3px rgb(0 0 0 / 0.35);
  }
  .empty {
    grid-column: 1 / -1;
    align-self: center;
    font-size: var(--fs-small);
    color: var(--muted);
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.35rem;
    padding: 0.3rem 0.45rem;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--seam);
  }
  .bar.now {
    box-shadow: inset 0 0 0 2px var(--accent);
  }
  .n {
    font-size: 0.7rem;
    color: var(--muted);
  }
  .chord {
    font-family: var(--font-display);
    font-weight: 600;
    color: var(--ink);
  }
  .chord small {
    margin-left: 0.15em;
    font-weight: 400;
    font-size: 0.7em;
    color: var(--muted);
  }
  .hold {
    color: var(--muted);
  }
  .memories {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.35rem;
  }
  .mem {
    display: grid;
    gap: 0.1rem;
    justify-items: start;
    min-width: 0;
    padding: 0.35rem 0.5rem;
    border-radius: 5px;
    text-align: left;
    color: var(--ink);
  }
  .mem.vacant {
    color: var(--muted);
  }
  .mem.selected {
    outline: 2px solid var(--accent);
  }
  .mem.pending {
    outline: 2px dashed var(--accent);
  }
  .mem.picking {
    outline: 1px dashed var(--line-strong);
  }
  .mem:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .num {
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 1.05rem;
  }
  .name,
  .sum {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.72rem;
  }
  .sum {
    color: var(--muted);
  }
  .ops {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.5rem;
    font-size: 0.9rem;
  }
  .hint {
    font-size: var(--fs-small);
    color: var(--accent);
  }
</style>
