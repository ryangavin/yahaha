<!--
  NowPlaying: the display's middle, in glance order. The chord on the left; on the right the
  playing section (44px in its hue), "next" and the next section in 28px muted text, and when the
  fill lands; under them the bar and beat with the section's bar lines, then the tempo with the
  Running light. A readout, not a control.
-->
<script lang="ts">
  import type { ComponentProps } from 'svelte'
  import BarBeat from '../BarBeat/BarBeat.svelte'
  import ChordReadout from '../ChordReadout/ChordReadout.svelte'
  import SectionName from '../SectionName/SectionName.svelte'
  import StatusDot from '../StatusDot/StatusDot.svelte'
  import TempoReadout from '../TempoReadout/TempoReadout.svelte'

  type Hue = 'intro' | 'main' | 'ending' | 'brk' | 'fill'

  type Props = {
    /** The chord readout: chord, extension, notes, fingering, held. */
    chord: ComponentProps<typeof ChordReadout>
    /** The playing section's shown name ("Main B"). */
    playing: string
    /** The playing section's hue (its name, the dot by "Section", the current beat and bar). */
    hue?: Hue
    /** The next section's shown name ("Main C"). Empty: no "next". */
    next?: string
    /** When the fill lands ("fill lands after bar 4"). Empty: nothing. */
    fill?: string
    /** The current bar of the section, 1-based. */
    bar: number
    /** Bars in the section. */
    bars: number
    /** The current beat, 1-based; 0 when stopped. */
    beat?: number
    /** Beats in the bar. */
    beats?: number
    /** How far through the current bar, 0-1. Default: halfway through the current beat. */
    progress?: number
    /** The tempo in BPM. */
    bpm: number
    /** The style is running: the green "Running" light. False: a hollow dot and "Stopped". */
    running?: boolean
  }

  let {
    chord,
    playing,
    hue = 'main',
    next = '',
    fill = '',
    bar,
    bars,
    beat = 0,
    beats = 4,
    progress,
    bpm,
    running = false,
  }: Props = $props()

  const hasNext = $derived(next.trim() !== '')
</script>

<div class="now">
  <ChordReadout {...chord} />
  <div class="column" role="group" aria-label="Section">
    <span class="label"><StatusDot {hue} />Section</span>
    <div class="sections">
      <SectionName label={playing} {hue} />
      {#if hasNext}
        <span class="word">next</span>
        <span class="next">{next}</span>
      {/if}
      {#if fill}<span class="fill">{fill}</span>{/if}
    </div>
    <div class="count"><BarBeat {bar} {bars} {beat} {beats} {hue} {progress} /></div>
    <div class="tempo">
      <TempoReadout {bpm} />
      <span class="state" class:running role="status">
        <StatusDot hue={running ? 'ok' : 'd'} hollow={!running} />{running ? 'Running' : 'Stopped'}
      </span>
    </div>
  </div>
</div>

<style>
  .now {
    display: grid;
    grid-template-columns: var(--now-chord-width) var(--now-section-width);
    gap: var(--space-24);
    width: var(--display-content-width);
    height: var(--now-height);
    font-family: var(--font-sans);
  }
  .column {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .label {
    height: var(--label-height);
    display: flex;
    align-items: center;
    gap: var(--space-8);
    font-size: var(--text-14);
    font-weight: var(--weight-regular);
    color: var(--m);
  }
  .sections {
    margin-top: var(--space-4);
    height: var(--leading-44);
    display: flex;
    align-items: baseline;
    gap: var(--space-16);
    white-space: nowrap;
  }
  .word {
    font-size: var(--text-13);
    font-weight: var(--weight-regular);
    color: var(--m);
  }
  .next {
    font-size: var(--text-28);
    font-weight: var(--weight-light);
    line-height: var(--leading-44);
    letter-spacing: var(--tracking-28);
    color: var(--m);
  }
  .fill {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: var(--text-14);
    font-weight: var(--weight-regular);
    color: var(--m);
  }
  .count {
    margin-top: var(--bar-row-gap);
  }
  .tempo {
    margin-top: var(--tempo-gap);
    height: var(--leading-40);
    display: flex;
    align-items: baseline;
    white-space: nowrap;
  }
  .state {
    margin-left: auto;
    align-self: center;
    display: flex;
    align-items: center;
    gap: var(--space-8);
    font-size: var(--text-14);
    font-weight: var(--weight-regular);
    color: var(--m);
  }
  .state.running {
    color: var(--ok);
  }
</style>
