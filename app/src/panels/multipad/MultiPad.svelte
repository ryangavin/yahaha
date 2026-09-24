<!--
  Multi Pads (right-side drawer, open while `ui.multipad`), laid out like the Genos MULTI
  PAD CONTROL: four pads, STOP, and per pad what the panel's STOP + pad and SELECT + pad
  do, then the pad bank and Multi Pad Synchro Stop.

  - A pad press plays at once when the band is stopped, at the next bar line while it
    plays (the pad flashes amber until then). Lamps come from the engine
    (`multiPad.pads[].lamp`) and animate on the beat clock.
  - Repeat and Chord Match come from the bank file; the switches override them until
    another bank loads.
  - The bank list is every .pad file in the style folders (`multiPad.banks`).

  State: multiPad, transport.running. Commands: triggerMultiPad, stopMultiPad,
  stopAllMultiPads, armMultiPad, setMultiPadRepeat, setMultiPadChordMatch, loadMultiPad,
  clearMultiPad, setMultiPadSynchroStop.
-->
<script lang="ts">
  import { app, clock, ui } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import Overlay from '../../lib/ui/Overlay.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'
  import PadButton from './PadButton.svelte'
  import { byFolder, PAD_KEYS, synchroStopText } from './multipad'

  const mp = $derived(app.state.multiPad)
  const running = $derived(app.state.transport.running)
  const groups = $derived(byFolder(mp.banks))
  const busy = $derived(mp.pads.some((p) => p.lamp === 'playing' || p.lamp === 'queued' || p.lamp === 'armed'))
  const red = { rgb: [127, 12, 8] as [number, number, number], level: 'bright' as const, anim: 'solid' as const }
  const armedLed = { rgb: [127, 12, 8] as [number, number, number], level: 'bright' as const, anim: 'flash' as const }
</script>

<Overlay id="multipad" title="Multi Pads" closeTip="drawer.close" onclose={() => (ui.multipad = false)}>
  <div class="drawer">
    <!-- 1. The pads ───────────────────────────────────────────────────── -->
    <section aria-labelledby="mp-pads">
      <h3 id="mp-pads" class="engraved">
        Pads · {mp.bank ? mp.bank.name : 'no bank'}{#if mp.loading}<span class="loading"> · loading…</span>{/if}
      </h3>
      <p class="explain">
        {running ? 'The band is playing: a pad starts at the next bar line.' : 'The band is stopped: a pad starts at once.'}
      </p>
      <div class="pads">
        {#each mp.pads as p (p.index)}
          <div class="col">
            <PadButton pad={p} beats={clock.beats} keyLabel={PAD_KEYS[p.index]} onpress={() => app.send({ type: 'triggerMultiPad', pad: p.index })} />
            <span class="ch engraved">ch {p.channel}</span>
            {#if p.lamp !== 'empty'}
              <div class="row">
                <HwButton tip="multipad.stop" label="Stop pad {p.index + 1}" onclick={() => app.send({ type: 'stopMultiPad', pad: p.index })}>■</HwButton>
                <HwButton tip="multipad.arm" label="Synchro Start pad {p.index + 1}" led={p.lamp === 'armed' ? armedLed : null} beats={clock.beats} pressed={p.lamp === 'armed'} onclick={() => app.send({ type: 'armMultiPad', pad: p.index })}>Sync</HwButton>
              </div>
              <Toggle on={p.repeat} tip="multipad.repeat" onclick={() => app.send({ type: 'setMultiPadRepeat', pad: p.index, on: !p.repeat })}>Repeat</Toggle>
              <Toggle on={p.chordMatch} tip="multipad.chord_match" onclick={() => app.send({ type: 'setMultiPadChordMatch', pad: p.index, on: !p.chordMatch })}>Chord</Toggle>
            {/if}
          </div>
        {/each}
      </div>
      <div class="stoprow">
        <HwButton tip="multipad.stop_all" led={busy ? red : null} onclick={() => app.send({ type: 'stopAllMultiPads' })}>STOP</HwButton>
        <span class="note engraved">Shift+Z X C V play the pads · Shift+B stops them</span>
      </div>
    </section>

    <!-- 2. The bank ───────────────────────────────────────────────────── -->
    <section aria-labelledby="mp-bank">
      <h3 id="mp-bank" class="engraved">Pad bank</h3>
      {#if mp.banks.length === 0}
        <p class="explain">No .pad files in the style folders. Put Multi Pad banks there and rescan (Settings), or try the synthetic one: <code>yahaha pad --demo</code>.</p>
      {:else}
        <div class="banks" role="listbox" aria-label="Multi Pad banks">
          {#each groups as g, gi (gi)}
            {#if g.folder}<span class="folder engraved">{g.folder}</span>{/if}
            {#each g.banks as b (b.id)}
              <button
                type="button"
                class="bank mat-raised"
                role="option"
                aria-selected={mp.bank?.id === b.id}
                class:current={mp.bank?.id === b.id}
                use:tip={'multipad.bank'}
                onclick={() => app.send({ type: 'loadMultiPad', id: b.id })}
              >
                <span class="dot" aria-hidden="true"></span>{b.name}
              </button>
            {/each}
          {/each}
        </div>
      {/if}
      {#if mp.bank}
        <div><HwButton tip="multipad.clear" onclick={() => app.send({ type: 'clearMultiPad' })}>No bank</HwButton></div>
      {/if}
    </section>

    <!-- 3. Synchro Stop ───────────────────────────────────────────────── -->
    <section aria-labelledby="mp-sync">
      <h3 id="mp-sync" class="engraved">Multi Pad Synchro Stop</h3>
      <div class="sync">
        <Toggle
          on={mp.synchroStop.styleStop}
          tip="multipad.synchro_style_stop"
          onclick={() => app.send({ type: 'setMultiPadSynchroStop', styleStop: !mp.synchroStop.styleStop, ending: mp.synchroStop.ending })}
        >Style Stop</Toggle>
        <Toggle
          on={mp.synchroStop.ending}
          tip="multipad.synchro_ending"
          onclick={() => app.send({ type: 'setMultiPadSynchroStop', styleStop: mp.synchroStop.styleStop, ending: !mp.synchroStop.ending })}
        >Style Ending</Toggle>
      </div>
      <p class="explain">{synchroStopText(mp.synchroStop)}</p>
    </section>
  </div>
</Overlay>

<style>
  .drawer {
    display: flex;
    flex-direction: column;
    gap: 1.15rem;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
  }
  h3 {
    margin: 0;
    font-size: 0.78rem;
  }
  .loading {
    color: var(--muted);
  }
  .explain {
    margin: 0;
    min-height: 1.35em;
    font-size: var(--fs-small);
    color: var(--muted);
    line-height: 1.35;
  }
  .pads {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.7rem;
  }
  .col {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0.35rem;
    min-width: 0;
  }
  .ch {
    text-align: center;
  }
  .row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.3rem;
  }
  .stoprow {
    display: flex;
    align-items: center;
    gap: 0.8rem;
  }
  .note {
    color: var(--muted);
  }
  .banks {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    max-height: 16rem;
    overflow-y: auto;
    padding: 0.1rem;
  }
  .folder {
    margin-top: 0.3rem;
  }
  .bank {
    display: flex;
    align-items: center;
    gap: 0.5em;
    min-height: 2.2rem;
    padding: 0 0.7em;
    border-radius: 5px;
    font-family: var(--font-display);
    font-weight: 600;
    color: var(--ink);
    text-align: left;
  }
  .dot {
    width: 0.5em;
    height: 0.5em;
    border-radius: 50%;
    background: var(--lamp-off);
  }
  .bank.current .dot {
    background: var(--accent);
  }
  .bank.current {
    outline: 1px solid var(--accent);
  }
  .sync {
    display: flex;
    gap: 0.6rem;
    flex-wrap: wrap;
  }
</style>
