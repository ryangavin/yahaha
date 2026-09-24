<!--
  Transpose: Genos Menu › Transpose. Keyboard (your keys and the chord the style
  follows) and Master (everything that sounds, drums excepted), −12..+12 semitones.
-->
<script lang="ts">
  import { app } from '../../lib/store.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import Field from './Field.svelte'
  import { signed } from './notes'

  const chord = $derived(app.state.chord)
  const kb = $derived(chord.transposeKeyboard)
  const master = $derived(chord.transposeMaster)
  const follows = $derived(
    chord.fingered && chord.name && chord.fingered !== chord.name ? `You play ${chord.fingered}; the style follows ${chord.name}.` : null,
  )
</script>

<Field name="Keyboard transpose" genos="Keyboard" note={follows ?? 'Moves your keys and the chord the style follows.'}>
  <div class="stepper">
    <HwButton tip="transpose.keyboard_down" label="Keyboard transpose down" onclick={() => app.send({ type: 'stepTranspose', keyboard: -1, master: 0 })}>−</HwButton>
    <div class="readout mat-screen" class:moved={kb !== 0}><span class="glow-text">{signed(kb)}</span><span class="unit">semitones</span></div>
    <HwButton tip="transpose.keyboard_up" label="Keyboard transpose up" onclick={() => app.send({ type: 'stepTranspose', keyboard: 1, master: 0 })}>+</HwButton>
  </div>
</Field>

<Field name="Master transpose" genos="Master" note="Moves everything that sounds, the band included. The drums stay put.">
  <div class="stepper">
    <HwButton tip="transpose.master_down" label="Master transpose down" onclick={() => app.send({ type: 'stepTranspose', keyboard: 0, master: -1 })}>−</HwButton>
    <div class="readout mat-screen" class:moved={master !== 0}><span class="glow-text">{signed(master)}</span><span class="unit">semitones</span></div>
    <HwButton tip="transpose.master_up" label="Master transpose up" onclick={() => app.send({ type: 'stepTranspose', keyboard: 0, master: 1 })}>+</HwButton>
  </div>
</Field>

<div class="reset">
  <HwButton tip="transpose.reset" onclick={() => app.send({ type: 'resetTranspose' })}>Reset both to 0</HwButton>
</div>

<style>
  .stepper {
    display: grid;
    grid-template-columns: 3.2rem 1fr 3.2rem;
    align-items: center;
    gap: 0.6rem;
  }
  .readout {
    display: flex;
    align-items: baseline;
    justify-content: center;
    gap: 0.5rem;
    height: 2.6rem;
    border-radius: 5px;
    font-family: var(--font-display);
    font-size: 1.6rem;
    font-weight: 700;
    line-height: 2.6rem;
  }
  .readout .glow-text {
    min-width: 2.5ch;
    text-align: right;
  }
  .unit {
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--screen-dim);
  }
  .readout:not(.moved) .glow-text {
    color: var(--screen-dim);
  }
  .reset {
    display: flex;
    justify-content: flex-start;
  }
</style>
