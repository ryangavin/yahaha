<!--
  CountRow: the middle of the section row, where the band's count lives. The beat blocks, "Bar
  3/4", the playing section → the next one in its waiting face, and when the fill lands, all at
  18px. It grows to fill the row and centres its content. A live status readout: one spoken
  sentence, the parts drawn hidden.
-->
<script lang="ts">
  import BeatBlocks from '../BeatBlocks/BeatBlocks.svelte'
  import SectionName from '../SectionName/SectionName.svelte'
  import WaitingChip from '../WaitingChip/WaitingChip.svelte'

  type Hue = 'intro' | 'main' | 'ending' | 'brk' | 'fill'

  type Props = {
    /** The current beat, 1-based; 0 when stopped. */
    beat: number
    /** Beats in the bar. */
    beats?: number
    /** The current bar of the section, 1-based. */
    bar: number
    /** Bars in the section. */
    bars: number
    /** The playing section's shown name ("Main B"). */
    playing: string
    /** The playing section's hue, also the current beat block's. */
    hue?: Hue
    /** The next section's shown name ("Main C"). Empty: no arrow, no chip. */
    next?: string
    /** The next section's hue. */
    nextHue?: Hue
    /** When the fill lands ("fill after bar 4"). Empty: nothing. */
    fill?: string
  }

  let { beat, beats = 4, bar, bars, playing, hue = 'main', next = '', nextHue = 'main', fill = '' }: Props = $props()

  const hasNext = $derived(next.trim() !== '')
  const spoken = $derived(
    `Beat ${beat} of ${beats}, bar ${bar} of ${bars}. ${playing} playing` +
      (hasNext ? `, ${next} next.` : '.') +
      (fill ? ` ${fill.charAt(0).toUpperCase()}${fill.slice(1)}` : ''),
  )
</script>

<div class="count" role="status" aria-label={spoken}>
  <BeatBlocks {beat} {beats} {hue} />
  <span class="bar" aria-hidden="true">Bar <span class="bar-value">{bar}/{bars}</span></span>
  <span class="sections" aria-hidden="true">
    <SectionName label={playing} {hue} size="count" />
    {#if hasNext}
      <span class="arrow">→</span>
      <WaitingChip label={next} hue={nextHue} size="count" />
    {/if}
  </span>
  {#if fill}<span class="fill" aria-hidden="true">{fill}</span>{/if}
</div>

<style>
  .count {
    flex: 1;
    min-width: 0;
    height: var(--control-height);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-20);
    font-family: var(--font-sans);
    font-size: var(--text-18);
    font-weight: var(--weight-light);
    font-variant-numeric: tabular-nums;
    line-height: var(--leading-24);
    white-space: nowrap;
  }
  .bar {
    color: var(--m);
    font-weight: var(--weight-regular);
  }
  .bar-value {
    color: var(--t);
    font-weight: var(--weight-light);
  }
  .sections {
    display: flex;
    align-items: center;
    gap: var(--space-8);
  }
  .arrow {
    color: var(--m);
  }
  .fill {
    color: var(--t);
  }
</style>
