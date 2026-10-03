<!--
  TempoReadout: the display's tempo, "104 BPM": the number in 32px light text, the unit muted
  beside it on the same baseline. A readout, not a control.
-->
<script lang="ts">
  type Props = {
    /** Beats per minute. */
    bpm: number
    /** The unit after the number. */
    unit?: string
  }

  let { bpm, unit = 'BPM' }: Props = $props()
</script>

<span class="tempo" role="img" aria-label="Tempo {bpm} {unit}">
  <span class="bpm" aria-hidden="true">{bpm}</span>
  <span class="unit" aria-hidden="true">{unit}</span>
</span>

<style>
  .tempo {
    display: inline-flex;
    align-items: baseline;
    gap: var(--space-6);
    height: var(--leading-40);
    font-family: var(--font-sans);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .bpm {
    font-size: var(--text-32);
    font-weight: var(--weight-light);
    line-height: var(--leading-40);
    letter-spacing: var(--tracking-32);
    color: var(--t);
  }
  .unit {
    font-size: var(--text-14);
    font-weight: var(--weight-regular);
    color: var(--m);
  }
</style>
