<!--
  BarBeat: the display's count, under the playing section. "Bar 3/4" in light numbers with the
  beat dots at the right (past beats filled grey, the current one in the section hue with its
  glow, beats to come a grey ring), and beneath them one line per bar of the section: past bars
  grey, the current bar filling in the hue, bars to come dark. The parent passes the moment;
  nothing here moves on its own. A readout, not a control: one spoken name, the parts hidden.
-->
<script lang="ts">
  type Hue = 'intro' | 'main' | 'ending' | 'brk' | 'fill'

  type Props = {
    /** The current bar of the section, 1-based. */
    bar: number
    /** Bars in the section: one line each. */
    bars: number
    /** The current beat, 1-based; 0 when stopped (every dot a ring, no bar filling). */
    beat?: number
    /** Beats in the bar: one dot each. */
    beats?: number
    /** The playing section's hue: the current beat and the current bar's fill. */
    hue?: Hue
    /** How far through the current bar, 0-1. Default: halfway through the current beat. */
    progress?: number
  }

  let { bar, bars, beat = 0, beats = 4, hue = 'main', progress }: Props = $props()

  const dots = $derived(Array.from({ length: Math.max(0, beats) }, (_, i) => i + 1))
  const lines = $derived(Array.from({ length: Math.max(0, bars) }, (_, i) => i + 1))
  const fill = $derived(
    beat === 0 ? 0 : Math.min(1, Math.max(0, progress ?? (beat - 0.5) / Math.max(1, beats))),
  )
  const spoken = $derived(`Bar ${bar} of ${bars}` + (beat ? `, beat ${beat} of ${beats}` : ', stopped'))

  function dotState(n: number): 'past' | 'current' | 'future' {
    if (n === beat) return 'current'
    return n < beat ? 'past' : 'future'
  }

  function lineState(n: number): 'past' | 'current' | 'future' {
    if (n === bar) return 'current'
    return n < bar ? 'past' : 'future'
  }
</script>

<div class="count" role="img" aria-label={spoken} style:--hue="var(--{hue})" data-hue={hue}>
  <div class="row" aria-hidden="true">
    <span class="word">Bar</span>
    <span class="bar">{bar}<span class="of">/{bars}</span></span>
    <span class="dots">
      {#each dots as n (n)}
        <span
          class="dot {dotState(n)}"
          style:--glow={dotState(n) === 'current' ? `var(--beat-glow-${hue})` : undefined}
          data-state={dotState(n)}
        ></span>
      {/each}
    </span>
  </div>
  <div class="lines" aria-hidden="true" style:--bars={Math.max(1, bars)}>
    {#each lines as n (n)}
      <span
        class="line {lineState(n)}"
        style:--fill="{lineState(n) === 'current' ? fill * 100 : 0}%"
        style:--glow={lineState(n) === 'current' && beat ? `var(--bar-glow-${hue})` : undefined}
        data-state={lineState(n)}
      ></span>
    {/each}
  </div>
</div>

<style>
  .count {
    display: flex;
    flex-direction: column;
    width: var(--now-section-width);
    font-family: var(--font-sans);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .row {
    height: var(--bar-row-height);
    display: flex;
    align-items: center;
    gap: var(--space-12);
  }
  .word {
    font-size: var(--text-14);
    font-weight: var(--weight-regular);
    color: var(--m);
  }
  .bar {
    font-size: var(--text-32);
    font-weight: var(--weight-light);
    line-height: var(--leading-24);
    color: var(--t);
  }
  .of {
    font-size: var(--text-18);
    color: var(--m);
  }
  .dots {
    margin-left: auto;
    display: flex;
    gap: var(--space-10);
  }
  .dot {
    box-sizing: border-box;
    width: var(--beat-dot);
    height: var(--beat-dot);
    border-radius: 50%;
  }
  .dot.past {
    background: var(--beat-dot-past);
  }
  .dot.current {
    background: var(--hue);
    box-shadow: var(--glow);
  }
  .dot.future {
    border: var(--line-width) solid var(--beat-dot-ring);
  }
  .lines {
    margin-top: var(--bar-segments-gap);
    height: var(--bar-segment-height);
    display: grid;
    grid-template-columns: repeat(var(--bars), minmax(0, 1fr));
    gap: var(--bar-segments-gap);
  }
  .line {
    display: flex;
    align-items: center;
  }
  .line::before {
    content: '';
    flex: 1;
    height: var(--bar-line);
    background: var(--bar-rest);
  }
  .line.past::before {
    background: var(--bar-past);
  }
  .line.current::before {
    background: linear-gradient(90deg, var(--hue) 0 var(--fill), var(--bar-rest) var(--fill) 100%);
    box-shadow: var(--glow);
  }
</style>
