<!--
  The iReal Pro chart player drawer (#89): import playlists, pick a song, and set how the
  band plays it. Chart mode itself is also on `r`, the songs on `( )`.

  ┌ iReal Pro charts ───────────────────────────── Close ┐
  │ [Open playlist…]  [irealb://… paste      ] [Import]  │
  │ Chart mode ◉  · ◀ Song title (Style, key) ▶          │
  │ Choruses − 1 +  · Intro None A B C · Ending …        │
  │ Loop Off Song A B …  · Auto style ◉  [Suggested]     │
  │ Playlists            │ Songs                         │
  │  ▸ Demo playlist  ✕  │  ▶ Title   Style   Key  Tempo │
  └──────────────────────────────────────────────────────┘

  Everything shown comes from `state.chart`; the drawer keeps only which playlist's songs
  it lists (defaulting to the chosen song's) and the link being typed.
-->
<script lang="ts">
  import { app, ui } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import Overlay from '../../lib/ui/Overlay.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'
  import Choice from '../settings/Choice.svelte'
  import Field from '../settings/Field.svelte'

  const c = $derived(app.state.chart)
  const style = $derived(app.state.style)
  let viewed = $state<number | null>(null)
  /** The playlist whose songs are listed: the one picked here, else the chosen song's. */
  const list = $derived(viewed !== null && viewed < c.playlists.length ? viewed : (c.selected?.[0] ?? (c.playlists.length ? 0 : null)))
  const songs = $derived(list === null ? [] : c.playlists[list].songs)
  let link = $state('')
  let file: HTMLInputElement | undefined = $state()

  const suggested = $derived(c.suggestedStyle === null ? null : (app.library.entries.find((e) => e.id === c.suggestedStyle) ?? null))

  async function openFile() {
    const f = file?.files?.[0]
    if (!f) return
    app.send({ type: 'importCharts', text: await f.text() })
    file!.value = ''
  }
  function importLink() {
    const text = link.trim()
    if (!text) return
    app.send({ type: 'importCharts', text })
    link = ''
  }

  const LETTERS = ['A', 'B', 'C']
  const sectionOpts = (tipKey: 'chart.intro' | 'chart.ending') => [
    { id: -1, label: 'None', tip: tipKey },
    ...LETTERS.map((l, i) => ({ id: i, label: l, tip: tipKey })),
  ]
  /** Loop choices: off, the whole song, each section of the first chorus. */
  const loops = $derived.by(() => {
    const song = c.song
    const opts: { id: string; label: string; tip: 'chart.loop'; range: [number, number] | null }[] = [
      { id: 'off', label: 'Off', tip: 'chart.loop', range: null },
    ]
    if (!song) return opts
    opts.push({ id: 'song', label: 'Song', tip: 'chart.loop', range: [0, song.bars.length] })
    song.sections
      .filter((s) => s.chorus === 1)
      .forEach((s, i) => opts.push({ id: `s${i}`, label: `${s.label || 'Bars'} ${s.start + 1}–${s.start + s.bars}`, tip: 'chart.loop', range: [s.start, s.start + s.bars] }))
    return opts
  })
  const loopId = $derived(loops.find((o) => (o.range === null ? c.loop === null : !!c.loop && o.range[0] === c.loop[0] && o.range[1] === c.loop[1]))?.id ?? null)
</script>

<Overlay id="charts" title="iReal Pro charts" closeTip="drawer.close" onclose={() => (ui.charts = false)}>
  <div class="charts">
    <section class="import" aria-label="Import">
      <HwButton tip="chart.import_file" onclick={() => file?.click()}>Open playlist…</HwButton>
      <input class="file" type="file" accept=".html,.htm,.txt,text/html,text/plain" tabindex="-1" aria-hidden="true" use:tip={'chart.import_file'} bind:this={file} onchange={openFile} />
      <input
        class="link mat-well"
        type="text"
        placeholder="Paste an irealb:// link"
        aria-label="iReal Pro link"
        spellcheck="false"
        use:tip={'chart.link'}
        bind:value={link}
        onkeydown={(e) => e.key === 'Enter' && importLink()}
      />
      <HwButton tip="chart.import_link" onclick={importLink}>Import</HwButton>
    </section>

    <section class="player" aria-label="Player">
      <div class="song-row">
        <Toggle on={c.on} tip="chart.mode" onclick={() => app.send({ type: 'toggleChartMode' })}>Chart mode</Toggle>
        <HwButton tip="chart.prev" label="Previous song" onclick={() => app.send({ type: 'stepChart', delta: -1 })}>◀</HwButton>
        <div class="song-now">
          {#if c.song}
            <span class="title">{c.song.title}</span>
            <span class="meta">{c.song.style} · {c.song.key}{c.song.tempo ? ` · ${c.song.tempo} bpm` : ''} · {c.song.bars.length} bars</span>
          {:else}
            <span class="meta">No chart yet: open an iReal Pro playlist or paste a link.</span>
          {/if}
        </div>
        <HwButton tip="chart.next" label="Next song" onclick={() => app.send({ type: 'stepChart', delta: 1 })}>▶</HwButton>
      </div>

      <div class="grid">
        <Field name="Choruses" note="Times through the form before the Ending.">
          <div class="stepper">
            <HwButton tip="chart.choruses_down" label="Fewer choruses" onclick={() => app.send({ type: 'setChartChoruses', choruses: Math.max(1, c.choruses - 1) })}>−</HwButton>
            <span class="value">{c.choruses}</span>
            <HwButton tip="chart.choruses_up" label="More choruses" onclick={() => app.send({ type: 'setChartChoruses', choruses: Math.min(99, c.choruses + 1) })}>+</HwButton>
          </div>
        </Field>
        <Field name="Intro" genos="INTRO">
          <Choice label="Chart Intro" options={sectionOpts('chart.intro')} value={c.intro ?? -1} onselect={(i) => app.send({ type: 'setChartIntro', index: i < 0 ? null : i })} />
        </Field>
        <Field name="Ending" genos="ENDING/rit.">
          <Choice label="Chart Ending" options={sectionOpts('chart.ending')} value={c.ending ?? -1} onselect={(i) => app.send({ type: 'setChartEnding', index: i < 0 ? null : i })} />
        </Field>
        <Field name="Loop" note="Plays the song or one section over and over, until you stop or press an Ending.">
          <Choice label="Loop" columns={Math.min(3, loops.length)} options={loops} value={loopId} onselect={(id) => app.send({ type: 'setChartLoop', range: loops.find((o) => o.id === id)?.range ?? null })} />
        </Field>
        <Field name="Style" note={suggested ? `The chart's label suggests ${suggested.name}. Any style you pick in the browser overrides it.` : 'No library style matches the chart’s style label.'}>
          <div class="style-row">
            <Toggle on={c.autoStyle} tip="chart.auto_style" onclick={() => app.send({ type: 'setChartAutoStyle', on: !c.autoStyle })}>Auto style</Toggle>
            {#if suggested}
              <HwButton tip="chart.suggested" onclick={() => app.send({ type: 'loadStyle', id: suggested.id })} pressed={style.id === suggested.id}>{suggested.name}</HwButton>
            {/if}
          </div>
        </Field>
      </div>
    </section>

    <section class="lists" aria-label="Playlists and songs">
      <div class="playlists mat-well">
        <h3 class="engraved">Playlists</h3>
        {#each c.playlists as p, i (i)}
          <div class="pl-row" class:on={i === list}>
            <button type="button" class="pl" use:tip={'chart.playlist'} onclick={() => (viewed = i)}>
              <span class="name">{p.name}</span><span class="count">{p.songs.length}</span>
            </button>
            <button type="button" class="remove" aria-label="Remove {p.name}" use:tip={'chart.remove_playlist'} onclick={() => app.send({ type: 'removeChartPlaylist', playlist: i })}>✕</button>
          </div>
        {:else}
          <p class="empty">None imported.</p>
        {/each}
      </div>
      <div class="songs mat-well" role="listbox" aria-label="Songs">
        {#each songs as song, i (i)}
          {@const chosen = c.selected?.[0] === list && c.selected?.[1] === i}
          <button
            type="button"
            role="option"
            aria-selected={chosen}
            class="song"
            class:chosen
            use:tip={'chart.song'}
            onclick={() => list !== null && app.send({ type: 'selectChart', playlist: list, song: i })}
          >
            <span class="mark">{chosen ? '▶' : ''}</span>
            <span class="title">{song.title}</span>
            <span class="style">{song.style}</span>
            <span class="key">{song.key}</span>
            <span class="tempo">{song.tempo ?? ''}</span>
          </button>
        {/each}
      </div>
    </section>
  </div>
</Overlay>

<style>
  .charts {
    display: grid;
    gap: 1rem;
    min-width: 0;
  }
  .import {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .file {
    display: none;
  }
  .link {
    flex: 1;
    min-width: 0;
    min-height: 2.2rem;
    padding: 0 0.6rem;
    border: 1px solid var(--seam);
    border-radius: 5px;
    color: var(--screen-ink);
    font: inherit;
  }
  .player {
    display: grid;
    gap: 0.9rem;
  }
  .song-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .song-now {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }
  .song-now .title {
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 1.2rem;
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meta {
    font-size: 0.85rem;
    color: var(--muted);
  }
  .grid {
    display: grid;
    gap: 0.9rem;
  }
  .stepper,
  .style-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .value {
    min-width: 2ch;
    text-align: center;
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 1.2rem;
  }
  .lists {
    display: grid;
    gap: 0.6rem;
  }
  .playlists,
  .songs {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0.4rem;
    border-radius: 6px;
    max-height: 18rem;
    overflow: auto;
  }
  h3 {
    margin: 0 0 0.3rem;
    font-size: 0.75rem;
  }
  .pl-row {
    display: flex;
    align-items: stretch;
    border-radius: 4px;
  }
  .pl-row.on {
    background: rgb(255 255 255 / 0.06);
  }
  .pl,
  .remove,
  .song {
    min-height: 2.2rem;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--screen-ink);
    font: inherit;
    text-align: left;
  }
  .pl {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 0.4rem;
    min-width: 0;
    padding: 0 0.4rem;
  }
  .pl .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .count,
  .empty {
    color: var(--screen-dim);
    font-size: 0.85rem;
  }
  .remove {
    padding: 0 0.5rem;
    color: var(--screen-dim);
  }
  .remove:hover,
  .pl:hover,
  .song:hover {
    background: rgb(255 255 255 / 0.05);
  }
  .song {
    display: grid;
    grid-template-columns: 1.2em minmax(0, 1fr) minmax(0, 8em) 3em 3em;
    align-items: center;
    gap: 0.4rem;
    padding: 0 0.4rem;
  }
  .song span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .song .style,
  .song .key,
  .song .tempo {
    color: var(--screen-dim);
    font-size: 0.85rem;
  }
  .song .tempo {
    text-align: right;
  }
  .song.chosen {
    color: var(--accent);
  }
</style>
