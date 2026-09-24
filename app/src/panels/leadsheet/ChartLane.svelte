<!--
  The chord chart in the lead-sheet band's lane (#89): the iReal chart the band plays in
  chart mode, eight bars to a line, two lines (the one playing and the next).

   ┌A──────┬───────┬───────┬───────┬B──────┬───────┬───────┬───────┐
   │ Cmaj7 │ Dm7 G7│ ▣Em7  │ A7    │ Fmaj7 │ / / / │ ...          line playing, bar ringed
   ├───────┼───────┼───────┼───────┼───────┼───────┼───────┼───────┤
   │ ...                                                           │ next line
   └───────────────────────────────────────────────────────────────┘

  The bar playing is `state.chart.bar` (the engine's), never a local cursor; the beat
  within it runs on `clock.pos`, as the section progress bar does. A chord sits over its
  beat; a bar with no new chord shows beat slashes. The current bar's ring turns amber
  while your left hand has taken over (`chart.overridden`). Loop bars are tinted.
-->
<script lang="ts">
  import type { ChartState } from '../../lib/api/types'
  import { clock } from '../../lib/store.svelte'

  let { chart, running, beatsPerBar }: { chart: ChartState; running: boolean; beatsPerBar: number } = $props()

  const PER_LINE = 8
  const bars = $derived(chart.song?.bars ?? [])
  const cur = $derived(chart.bar)
  /** The line shown first: the one playing, else where the loop (or the song) starts. */
  const first = $derived(Math.floor((cur ?? chart.loop?.[0] ?? 0) / PER_LINE) * PER_LINE)
  const lines = $derived(
    [first, first + PER_LINE].filter((s) => s < bars.length).map((s) => Array.from({ length: Math.min(PER_LINE, bars.length - s) }, (_, k) => s + k)),
  )
  const inLoop = (i: number) => !!chart.loop && i >= chart.loop[0] && i < chart.loop[1]
  const bpb = $derived(Math.max(1, beatsPerBar))
  /** How far through the bar playing (0–1), for its progress line. */
  const progress = $derived(running && cur !== null ? (Math.max(0, clock.pos) % bpb) / bpb : 0)
  /** Grid columns for a chord on `beat` until the next chord (or the bar's end), 1-based. */
  const span = (beat: number, next: number | undefined, beats: number) => {
    const b = Math.min(beat, beats - 1)
    return `${b + 1} / ${Math.max(b + 1, Math.min(next ?? beats, beats)) + 1}`
  }
  /** Chorus marks: the first bar of each chorus after the first. */
  const chorusAt = (i: number) => (i > 0 && bars[i].chorus !== bars[i - 1].chorus ? bars[i].chorus : null)
</script>

<div class="lines" style:--per={PER_LINE}>
  {#each lines as line, l (l)}
    <div class="line">
      {#each line as i (i)}
        {@const b = bars[i]}
        <div
          class="cell mat-screen"
          class:current={i === cur}
          class:override={i === cur && chart.overridden}
          class:past={cur !== null && i < cur}
          class:loop={inLoop(i)}
          style:--beats={Math.max(1, b.time[0])}
        >
          {#if b.sectionStart && b.section}<span class="sec">{b.section}</span>{/if}
          {#if chorusAt(i)}<span class="chorus">×{chorusAt(i)}</span>{/if}
          <span class="num">{i + 1}</span>
          <span class="beats">
            {#if b.chords.length}
              {#each b.chords as c, k (c.beat)}<b style:grid-column={span(c.beat, b.chords[k + 1]?.beat, b.time[0])}>{c.name}</b>{/each}
            {:else}
              {#each Array.from({ length: b.time[0] }, (_, k) => k) as k (k)}<i style:grid-column={k + 1}>/</i>{/each}
            {/if}
          </span>
          {#if i === cur}<span class="bar-progress" aria-hidden="true"><span style:transform="scaleX({progress})"></span></span>{/if}
        </div>
      {/each}
    </div>
  {/each}
</div>

<style>
  .lines {
    display: flex;
    flex-direction: column;
    gap: 0.35em;
    flex: 1 1 auto;
    min-height: 0;
  }
  .line {
    display: grid;
    grid-template-columns: repeat(var(--per), minmax(0, 1fr));
    gap: 0.3em;
    flex: 1 1 0;
    min-height: 2.2em;
    max-height: 4em;
  }
  .cell {
    position: relative;
    display: flex;
    align-items: center;
    min-width: 0;
    border-radius: 0.3em;
    overflow: hidden;
  }
  .past {
    opacity: 0.55;
  }
  .loop {
    background-image: linear-gradient(0deg, color-mix(in srgb, var(--accent) 12%, transparent), transparent);
  }
  .current {
    box-shadow:
      inset 0 0 0 1.5px color-mix(in srgb, var(--screen-ink) 70%, transparent),
      inset 0 6px 18px rgb(0 0 0 / 0.6);
  }
  .override {
    box-shadow:
      inset 0 0 0 1.5px var(--accent),
      inset 0 6px 18px rgb(0 0 0 / 0.6);
  }
  .sec,
  .chorus,
  .num {
    position: absolute;
    top: 0.2em;
    font-weight: 700;
    font-size: 0.7em;
    line-height: 1;
  }
  .sec {
    left: 0.35em;
    padding: 0.05em 0.3em;
    border-radius: 0.2em;
    background: var(--accent);
    color: var(--accent-ink);
  }
  .chorus {
    left: 2em;
    color: var(--screen-dim);
  }
  .num {
    right: 0.4em;
    font-weight: 600;
    color: var(--screen-dim);
  }
  .beats {
    display: grid;
    grid-template-columns: repeat(var(--beats), minmax(0, 1fr));
    align-items: center;
    width: 100%;
    padding: 0.55em 0.4em 0;
    font-weight: 700;
    font-size: 1.15em;
    color: var(--screen-ink);
  }
  .beats b {
    font-weight: 700;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    grid-row: 1;
    min-width: 0;
  }
  .beats i {
    font-style: normal;
    text-align: center;
    color: color-mix(in srgb, var(--screen-dim) 60%, transparent);
  }
  .current .beats {
    text-shadow: 0 0 6px var(--screen-glow);
  }
  .bar-progress {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 0.2em;
    overflow: hidden;
  }
  .bar-progress span {
    position: absolute;
    inset: 0;
    transform-origin: left center;
    background: var(--accent);
    will-change: transform;
  }
</style>
