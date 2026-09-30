<!--
  The display above the Launchkey mirror: "where am I, what's next". A status line on top,
  as the Genos Home display's Style area: the style (click its name or Browse for the
  style browser, as touching the name does on the Genos), the chord the style follows,
  large, then the fingering type, transpose and time signature. Tempo, bar/beat and Sync
  are on the header's transport bar.

   ┌ status line (glass) ─────────────────────────────────────────────────────────────────┐
   │ Cool Pop [BROWSE]            C7  played Bb7            Fingered  Transpose +2·0  4/4 │
   ├ now ─────────┬ lane─────────┬ lane ──────────────────────────────────────────┬────────┬ next ─────────┐
   │ Playing      │ ┌1────────┬2────────┬3────────┬4────────┐        │        │ Next          │
   │ Main B       │ │ / / / / │ / / / / │ / /   / │ /  /  / │ cells  │[Charts]│ → Main C      │
   │ bar 3 of 4   │ └─────────┴─────────┴─────────┴─────────┘        │        │ at the bar    │
   │              │ ████████████████████▌─────────────────── progress │        │               │
   └──────────────┴──────────────────────────────────────────────────┴────────┴───────────────┘

  The lane shows the section playing as bar cells, one per bar with a slash per beat (lit
  as the beats pass), over a progress bar across the section. In chart mode (#89) the
  iReal chart renders into the same lane instead (ChartLane.svelte): two lines of bar
  cells with the chords, the bar playing ringed. The band gets its height from the shell,
  between 5.2em and 10em of `--u`.
-->
<script lang="ts">
  import { INTROS, MAINS, sectionLabel } from '../../lib/api/types'
  import { app, clock, ui } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import DrawerButton from '../../lib/ui/DrawerButton.svelte'
  import ChartLane from './ChartLane.svelte'

  const s = $derived(app.state)
  const t = $derived(s.transport)
  const c = $derived(s.chord)
  const signed = (n: number) => (n > 0 ? `+${n}` : n < 0 ? `−${-n}` : '0')
  /** Chart mode with a chart chosen: the lane shows the chart. */
  const chart = $derived(s.chart?.on && s.chart.song ? s.chart : null)
  /** Bars in the section's pattern; unknown (the engine doesn't send it yet): one cell. */
  const bars = $derived(Math.max(1, Math.min(32, t.sectionBars ?? 1)))
  const bpb = $derived(Math.max(1, t.beatsPerBar))
  /** Quarter notes into the section, run on from the engine's clock anchors every frame. */
  const pos = $derived(t.running ? Math.max(0, clock.pos) : 0)
  /** The bar playing within the pattern (0-based); it loops for a Main. */
  const cell = $derived(t.running ? Math.floor(pos / bpb) % bars : -1)
  /** Beats into the current bar (0 to bpb). */
  const inBar = $derived(t.running ? pos % bpb : 0)
  /** How far through the section (0–1): the progress bar under the cells. */
  const progress = $derived(t.running ? Math.min(1, (cell + inBar / bpb) / bars) : 0)
  /** The beat playing in the current bar (0-based). */
  const beatNow = $derived(Math.floor(inBar))

  /** Stopped: the Intro the band starts with (armed, or the chart's), and the first Main. */
  const intro = $derived(t.pendingIntro ?? chart?.intro ?? null)
  const firstMain = $derived(chart?.song?.bars[0]?.main ?? t.main)
  const now = $derived(
    t.running && t.section
      ? sectionLabel(t.section)
      : intro !== null
        ? sectionLabel(INTROS[intro])
        : sectionLabel(MAINS[firstMain] ?? 'Main A'),
  )
  const state = $derived(
    chart && t.running && chart.bar !== null
      ? `bar ${chart.bar + 1} of ${chart.song!.bars.length}${chart.overridden ? ' · your chord' : ''}`
      : t.running
      ? // Without the section's length the one cell loops every bar: count bars from the clock.
        t.sectionBars
        ? `bar ${cell + 1} of ${bars}`
        : `bar ${Math.floor(pos / bpb) + 1}`
      : t.syncStart
        ? 'waiting for your chord'
        : 'stopped',
  )
  const next = $derived(
    t.queued ? sectionLabel(t.queued) : t.running ? '' : intro !== null ? sectionLabel(MAINS[firstMain] ?? 'Main A') : '',
  )
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex (display items are focusable so their tooltips are reachable from the keyboard) -->
<section class="lead mat-chassis" aria-label="Lead sheet">
  <!-- Labelled as the mirror's status display was: it takes that display's place. -->
  <div class="status mat-screen" role="group" aria-label="Status display">
    <!-- Touching the style name opens the Style Selection display on the Genos: here, the style browser. -->
    <button type="button" class="style" use:tip={'browser.open'} onclick={() => (ui.browser = true)}>
      <span class="sname glow-text">{s.style.name || 'No style'}</span><span class="browse">Browse</span>
    </button>
    <span class="chord" tabindex="0" use:tip={'display.chord'}>
      <span class="cname glow-text">{c.name ?? '–'}</span>{#if c.fingered && c.fingered !== c.name}<span class="fingered">played {c.fingered}</span>{/if}
    </span>
    <span class="info">
      <span class="fingering" tabindex="0" use:tip={'fingering.select'}>{c.fingeringName}{c.manualBassActive ? ' · Manual Bass' : ''}</span>
      <span class="transpose" tabindex="0" use:tip={'transpose.display'} class:set={c.transposeKeyboard !== 0 || c.transposeMaster !== 0}>
        Transpose {signed(c.transposeKeyboard)} · {signed(c.transposeMaster)}
      </span>
      <span class="timesig" tabindex="0" use:tip={'display.timesig'}>{s.style.timeSignature[0]}/{s.style.timeSignature[1]}</span>
    </span>
  </div>

  <div class="now" tabindex="0" use:tip={'lead.section'}>
    <span class="engraved">{t.running ? 'Playing' : 'Starts with'}</span>
    <span class="name">{now}</span>
    <span class="sub">{state}</span>
  </div>

  <!-- The lane: the section's bars, or the chord chart in chart mode. -->
  {#if chart}
    <div class="lane" data-slot="chart" tabindex="0" use:tip={'lead.chart'} role="img" aria-label="Chord chart: {chart.song?.title}, {state}">
      <ChartLane {chart} running={t.running} beatsPerBar={t.beatsPerBar} />
    </div>
  {:else}
  <div class="lane" data-slot="chart" tabindex="0" use:tip={'lead.progress'} role="img" aria-label="Section progress: {state}">
    <div class="cells" style:--bars={bars}>
      {#each Array.from({ length: bars }, (_, i) => i) as i (i)}
        <div class="cell mat-screen" class:past={i < cell} class:current={i === cell}>
          <span class="num">{i + 1}</span>
          <!-- Beat slashes, as a lead sheet writes a bar with no new chord. -->
          <span class="beats" aria-hidden="true">
            {#each Array.from({ length: bpb }, (_, b) => b) as b (b)}<i class:on={i < cell || (i === cell && b <= beatNow)} class:now={i === cell && b === beatNow}>/</i>{/each}
          </span>
        </div>
      {/each}
    </div>
    <div class="track" aria-hidden="true"><span class="fill" style:transform="scaleX({progress})"></span></div>
  </div>
  {/if}

  <!-- The chord chart's own drawer, by the lane the chart plays in. -->
  <div class="charts"><DrawerButton tip="drawer.charts" open={ui.charts} onclick={() => ui.toggleDrawer('charts')}>Charts</DrawerButton></div>

  <div class="next" class:queued={!!t.queued} tabindex="0" use:tip={'lead.next'}>
    <span class="engraved">Next</span>
    <span class="name">{next ? `→ ${next}` : '–'}</span>
    <span class="sub">{t.queued ? (t.queued.startsWith('Fill') ? 'at the next beat' : 'at the bar line') : ''}</span>
  </div>
</section>

<style>
  .lead {
    display: grid;
    grid-template-columns: 13em minmax(0, 1fr) auto 12em;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 0.5em 1.2em;
    height: 100%;
    padding: 0.6em 1.5em 0.7em;
    border-radius: 1em;
    font-family: var(--font-display);
    /* On a short window the shell gives the display as little as 4em: the status line
       stays whole and the row under it is cut off, rather than spill over the mirror (the
       section also shows on the header's transport bar). */
    container: display / size;
    overflow: hidden;
  }
  @container display (height < 7em) {
    .status {
      padding-block: 0;
    }
    .cname {
      font-size: 2em;
    }
  }

  /* The status line: one row of glass across the display, style left, chord in the middle,
     fingering, transpose and time signature right. Each side shrinks (ellipsis) before the
     chord does. */
  .status {
    grid-column: 1 / -1;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
    align-items: center;
    gap: 1.2em;
    padding: 0.2em 0.9em;
    border-radius: 0.5em;
    white-space: nowrap;
  }
  .status span[tabindex] {
    border-radius: 3px;
  }
  .style {
    display: flex;
    align-items: baseline;
    gap: 0.6em;
    min-width: 0;
    max-width: 100%;
    justify-self: start;
    padding: 0;
    font: inherit;
    font-weight: 600;
    font-size: 1.35em;
    color: inherit;
    text-align: left;
    background: none;
    border: 0;
    cursor: pointer;
  }
  .sname {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* A soft-key label on the glass: the name is the button, as on the Genos. */
  .browse {
    flex: none;
    padding: 0.1em 0.45em;
    font-size: 0.55em;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--screen-dim);
    border: 1px solid color-mix(in srgb, var(--screen-dim) 55%, transparent);
    border-radius: 0.3em;
  }
  .style:hover .browse,
  .style:focus-visible .browse {
    color: var(--screen-ink);
    border-color: var(--screen-ink);
  }
  .chord {
    display: flex;
    align-items: baseline;
    gap: 0.5em;
  }
  /* Large enough to read from the keys. */
  .cname {
    font-size: 2.6em;
    font-weight: 700;
    line-height: 1.05;
    letter-spacing: 0.01em;
  }
  .fingered {
    font-size: 0.9em;
    color: var(--screen-dim);
  }
  .info {
    display: flex;
    align-items: baseline;
    justify-content: flex-end;
    gap: 1em;
    min-width: 0;
    overflow: hidden;
    color: var(--screen-dim);
  }
  .info > * {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .fingering {
    flex: 0 1 auto;
    color: var(--screen-ink);
    font-weight: 600;
    font-size: 1.1em;
  }
  .transpose {
    flex: 0 1 auto;
  }
  .transpose.set {
    color: var(--accent);
  }
  .timesig {
    flex: none;
    color: var(--screen-ink);
    font-weight: 600;
    font-size: 1.1em;
  }
  .lead :global(.engraved) {
    font-size: 0.78em;
  }
  .charts {
    display: flex;
    align-items: center;
  }
  .now,
  .next {
    display: flex;
    flex-direction: column;
    justify-content: center;
    min-width: 0;
    border-radius: 0.3em;
  }
  .next {
    text-align: right;
  }
  .name {
    font-weight: 700;
    font-size: 1.7em;
    line-height: 1.1;
    /* At the band's least height, overhang into the padding rather than clip the glyphs. */
    flex-shrink: 0;
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .next .name {
    color: var(--muted);
  }
  .next.queued .name {
    color: var(--accent);
  }
  .sub {
    min-height: 1.2em;
    font-size: 0.9em;
    color: var(--muted);
    white-space: nowrap;
  }

  .lane {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 0.45em;
    min-width: 0;
    min-height: 0;
    border-radius: 0.4em;
  }
  .cells {
    display: grid;
    grid-template-columns: repeat(var(--bars), minmax(0, 1fr));
    gap: 0.4em;
    flex: 0 1 5.5em;
    min-height: 2.4em;
  }
  .cell {
    position: relative;
    display: flex;
    align-items: center;
    border-radius: 0.35em;
    overflow: hidden;
  }
  .num {
    position: absolute;
    top: 0.25em;
    left: 0.45em;
    font-weight: 600;
    font-size: 0.75em;
    color: var(--screen-dim);
  }
  .beats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(0, 1fr));
    grid-auto-flow: column;
    width: 100%;
    padding: 0 0.6em 0 1.6em;
    font-weight: 600;
    font-size: 1.35em;
    color: color-mix(in srgb, var(--screen-dim) 45%, transparent);
  }
  .beats i {
    font-style: normal;
    text-align: center;
  }
  .beats .on {
    color: var(--screen-dim);
  }
  .beats .now {
    color: var(--screen-ink);
    text-shadow: 0 0 6px var(--screen-glow);
  }
  .current {
    box-shadow:
      inset 0 0 0 1px color-mix(in srgb, var(--screen-ink) 40%, transparent),
      inset 0 6px 18px rgb(0 0 0 / 0.6);
  }
  .current .num {
    color: var(--screen-ink);
  }
  /* The section progress bar: scaled with transform only, so it runs at 60 Hz on the compositor. */
  .track {
    position: relative;
    flex: none;
    height: 0.45em;
    border-radius: 0.25em;
    overflow: hidden;
    background: var(--well);
    box-shadow: inset 0 1px 2px rgb(0 0 0 / 0.6);
  }
  .fill {
    position: absolute;
    inset: 0;
    transform-origin: left center;
    background: linear-gradient(90deg, color-mix(in srgb, var(--accent) 55%, transparent), var(--accent));
    will-change: transform;
  }
</style>
