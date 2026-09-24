<!--
  Split: Genos Menu › Split & Fingering › Split Point. yahaha has one split point, the
  Genos's Style + Left point: drag it on the strip, or step it a key at a time.
-->
<script lang="ts">
  import { app } from '../../lib/store.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import Field from './Field.svelte'
  import SplitStrip from './SplitStrip.svelte'
  import { SPLIT_MAX, SPLIT_MIN } from './notes'

  const chord = $derived(app.state.chord)
</script>

<Field
  name="Split point"
  genos="Split Point (Style + Left)"
  note="Right 1–3 play above the split. Left and the chord section play at and below it (in Upper, the chords come from the right hand instead)."
>
  <div class="readout-row">
    <HwButton tip="split.down" label="Split down one key" onclick={() => app.send({ type: 'moveSplit', delta: -1 })}>−</HwButton>
    <div class="readout mat-screen" aria-live="polite">
      <span class="big glow-text">{chord.splitName}</span>
      <span class="midi">MIDI {chord.split}</span>
    </div>
    <HwButton tip="split.up" label="Split up one key" onclick={() => app.send({ type: 'moveSplit', delta: 1 })}>+</HwButton>
  </div>
  <SplitStrip split={chord.split} onchange={(note) => app.send({ type: 'setSplit', note })} />
</Field>

<p class="range">Range C0–C6 (MIDI {SPLIT_MIN}–{SPLIT_MAX}). Yamaha octave numbering: C3 is middle C (MIDI 60).</p>

<style>
  .readout-row {
    display: grid;
    grid-template-columns: 3.2rem 1fr 3.2rem;
    align-items: center;
    gap: 0.6rem;
    margin-bottom: 0.2rem;
  }
  .readout {
    display: flex;
    align-items: baseline;
    justify-content: center;
    gap: 0.6rem;
    height: 2.6rem;
    border-radius: 5px;
    font-family: var(--font-display);
  }
  .big {
    font-size: 1.6rem;
    font-weight: 700;
    line-height: 2.6rem;
    min-width: 3ch;
    text-align: right;
  }
  .midi {
    color: var(--screen-dim);
    font-size: 0.9rem;
    min-width: 6ch;
  }
  .range {
    margin: 0;
    font-size: var(--fs-small);
    color: var(--muted);
  }
</style>
