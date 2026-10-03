<!--
  NowPlaying: the display's middle, in glance order. The chord on the left; on the right the
  playing section (44px in its hue) with the next one in its waiting face, and below them the
  tempo with the Running light. A readout, not a control.
-->
<script lang="ts">
  import type { ComponentProps } from 'svelte'
  import ChordReadout from '../ChordReadout/ChordReadout.svelte'
  import SectionName from '../SectionName/SectionName.svelte'
  import StatusDot from '../StatusDot/StatusDot.svelte'
  import TempoReadout from '../TempoReadout/TempoReadout.svelte'
  import WaitingChip from '../WaitingChip/WaitingChip.svelte'

  type Hue = 'intro' | 'main' | 'ending' | 'brk' | 'fill'

  type Props = {
    /** The chord readout: chord, extension, notes, fingering, held. */
    chord: ComponentProps<typeof ChordReadout>
    /** The playing section's shown name ("Main B"). */
    playing: string
    /** The playing section's hue (its name and the dot by "Section"). */
    hue?: Hue
    /** The next section's shown name ("Main C"). Empty: no "next", no chip. */
    next?: string
    /** The next section's hue. */
    nextHue?: Hue
    /** The tempo in BPM. */
    bpm: number
    /** The style is running: the green "Running" light. False: a hollow dot and "Stopped". */
    running?: boolean
  }

  let { chord, playing, hue = 'main', next = '', nextHue = 'main', bpm, running = false }: Props = $props()

  const hasNext = $derived(next.trim() !== '')
</script>

<div class="now">
  <ChordReadout {...chord} />
  <div class="column" role="group" aria-label="Section">
    <span class="label"><StatusDot {hue} />Section</span>
    <div class="sections">
      <SectionName label={playing} {hue} size="display" />
      {#if hasNext}
        <span class="word">next</span>
        <WaitingChip label={next} hue={nextHue} size="display" />
      {/if}
    </div>
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
    height: var(--leading-52);
    display: flex;
    align-items: center;
    gap: var(--space-16);
    white-space: nowrap;
  }
  .word {
    font-size: var(--text-14);
    font-weight: var(--weight-regular);
    color: var(--m);
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
