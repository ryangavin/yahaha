<!--
  Header: the top bar (style, browse, app controls) and the performance strip (tempo,
  time signature, bar/beat, the chord, fingering, split, transpose).

  REFERENCE PANEL: copy its pattern for a new one (see app/CONTRIBUTING.md):
  - read slices of `app.state` with `$derived`; never keep your own copy of engine state
  - every action is `app.send({ type: … })`
  - every interactive element is a lib/ui component (or has `use:tip`) with a catalog key
  - fixed widths for anything that changes while playing, so nothing shifts
-->
<script lang="ts">
  import { sectionLabel } from '../../lib/api/types'
  import { app, ui } from '../../lib/store.svelte'
  import { tip, tips } from '../../lib/tooltip/tip.svelte'
  import Key from '../../lib/ui/Key.svelte'
  import Readout from '../../lib/ui/Readout.svelte'

  const s = $derived(app.state)
  const t = $derived(s.transport)
  const c = $derived(s.chord)
  const style = $derived(s.style)
  const entry = $derived(app.library.entries.find((e) => e.id === style.id))
  const file = $derived(style.path.split('/').pop() ?? '')
  const signed = (n: number) => (n > 0 ? `+${n}` : n < 0 ? `−${-n}` : '0')
</script>

<header class="header">
  <div class="bar">
    <span class="brand" aria-label="yahaha">yahaha</span>

    <div class="style">
      <Key tip="style.prev" size="s" hint={false} label="Previous style" onclick={() => app.send({ type: 'stepStyle', delta: -1 })}>◀</Key>
      <button type="button" class="style-name" use:tip={'style.name'} onclick={() => (ui.browser = true)}>
        <span class="name">{style.name || 'No style loaded'}</span>
        <span class="folder">{style.name ? [entry?.folder, file, style.format].filter(Boolean).join(' · ') : 'Open the browser to pick one'}</span>
      </button>
      <Key tip="style.next" size="s" hint={false} label="Next style" onclick={() => app.send({ type: 'stepStyle', delta: 1 })}>▶</Key>
      <Key tip="browser.open" size="s" onclick={() => (ui.browser = true)}>Browse</Key>
    </div>

    <div class="app-controls">
      <!-- svelte-ignore a11y_no_noninteractive_tabindex (focusable so its tooltip is reachable from the keyboard) -->
      <span class="lk" class:connected={s.pads.connected} role="status" tabindex="0" use:tip={'launchkey.status'}>
        <span class="dot" aria-hidden="true"></span>
        <span class="lk-name">{s.pads.connected ? 'Launchkey' : 'No Launchkey'}</span>
      </span>
      {#if app.kind === 'mock' || s.io.offline}<span class="mock" title="No live engine: mock or offline session">{app.kind === 'mock' ? 'mock' : 'offline'}</span>{/if}
      <Key tip="app.help" size="s" hint={false} pressed={tips.help} label="Help mode" onclick={() => tips.toggleHelp()}>?</Key>
      <Key tip="app.theme" size="s" hint={false} label="Light or dark theme" onclick={() => ui.setTheme(ui.theme === 'dark' ? 'light' : 'dark')}>
        {ui.theme === 'dark' ? '☾' : '☀'}
      </Key>
      <Key tip="settings.open" size="s" hint={false} onclick={() => (ui.settings = true)}>Settings</Key>
    </div>
  </div>

  <div class="strip">
    <Readout label="Tempo" tip="display.tempo" class="tempo">
      {Math.round(t.tempo)}<small> bpm</small>
      {#snippet controls()}
        <Key tip="tempo.down" size="s" hint={false} label="Tempo down" onclick={() => app.send({ type: 'tempoDown' })}>−</Key>
        <Key tip="tempo.up" size="s" hint={false} label="Tempo up" onclick={() => app.send({ type: 'tempoUp' })}>+</Key>
        <Key tip="tempo.tap" onclick={() => app.send({ type: 'tapTempo' })}>Tap</Key>
      {/snippet}
    </Readout>

    <Readout label="Time" tip="display.timesig" class="timesig">
      {style.timeSignature[0]}/{style.timeSignature[1]}
    </Readout>

    <Readout label={t.running ? 'Playing' : t.syncStart ? 'Waiting for a chord' : 'Stopped'} tip="display.position" class="position">
      <span class="barnum">Bar {t.running ? t.bar : '–'}</span>
      <span class="beats" aria-label="beat {t.beat} of {t.beatsPerBar}">
        {#each Array.from({ length: t.beatsPerBar }, (_, i) => i + 1) as b (b)}
          <span class="beat" class:on={t.running && b === t.beat} class:down={b === 1}></span>
        {/each}
      </span>
      <span class="sections">
        <span class="now">{t.section ? sectionLabel(t.section) : ''}</span>
        <span class="next">{t.queued ? `→ ${sectionLabel(t.queued)}` : ''}</span>
      </span>
    </Readout>

    <Readout label="Chord" tip="display.chord" class="chord">
      <span class="chord-name">{c.name ?? '—'}</span>
      <span class="played">{c.fingered && c.fingered !== c.name ? `fingered ${c.fingered}` : ''}</span>
    </Readout>

    <div class="detect">
      <Key tip="fingering.select" size="s" onclick={() => app.send({ type: 'nextFingering' })}>
        <span class="fixed fing">{c.fingeringName}</span>
      </Key>
      <Key tip="detection.upper" size="s" pressed={c.upper} onclick={() => app.send({ type: 'toggleUpper' })}>
        <span class="fixed ul">{c.upper ? 'Upper' : 'Lower'}</span>
      </Key>
    </div>

    <Readout label="Split" tip="split.display" class="split">
      <span class="fixed note">{c.splitName}</span>
      {#snippet controls()}
        <Key tip="split.down" size="s" hint={false} label="Split down" onclick={() => app.send({ type: 'moveSplit', delta: -1 })}>−</Key>
        <Key tip="split.up" size="s" hint={false} label="Split up" onclick={() => app.send({ type: 'moveSplit', delta: 1 })}>+</Key>
      {/snippet}
    </Readout>

    <Readout label="Transpose" tip="transpose.display" class="transpose">
      <span class="tr" class:set={c.transposeKeyboard !== 0}>K {signed(c.transposeKeyboard)}</span>
      <span class="tr" class:set={c.transposeMaster !== 0}>M {signed(c.transposeMaster)}</span>
      {#snippet controls()}
        <Key tip="transpose.keyboard_down" size="s" hint={false} label="Keyboard transpose down" onclick={() => app.send({ type: 'stepTranspose', keyboard: -1, master: 0 })}>−</Key>
        <Key tip="transpose.keyboard_up" size="s" hint={false} label="Keyboard transpose up" onclick={() => app.send({ type: 'stepTranspose', keyboard: 1, master: 0 })}>+</Key>
        <Key tip="transpose.reset" size="s" hint={false} onclick={() => app.send({ type: 'resetTranspose' })}>0</Key>
      {/snippet}
    </Readout>
  </div>
</header>

<style>
  .header {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 1rem;
    min-height: 2.75rem;
  }
  .brand {
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 1.35rem;
    letter-spacing: 0.04em;
    color: var(--accent);
  }
  .style {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    min-width: 0;
    flex: 1;
  }
  .style-name {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    min-width: 0;
    max-width: 30rem;
    padding: 0.1rem 0.5rem;
    background: none;
    border: 1px solid transparent;
    border-radius: var(--r-key);
    text-align: left;
  }
  .style-name:hover {
    border-color: var(--line);
  }
  .name {
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 1.35rem;
    line-height: 1.1;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .folder {
    font-size: var(--fs-small);
    color: var(--muted);
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .app-controls {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }
  .lk {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    font-size: var(--fs-small);
    color: var(--muted);
    margin-right: 0.4rem;
    white-space: nowrap;
  }
  .lk .dot {
    width: 0.55rem;
    height: 0.55rem;
    border-radius: 50%;
    background: var(--lamp-off);
    border: 1px solid var(--line-strong);
  }
  .lk.connected .dot {
    background: #3ecf6e;
    border-color: #3ecf6e;
  }
  .mock {
    font-size: 0.7rem;
    padding: 0.05rem 0.4rem;
    border: 1px dashed var(--line-strong);
    border-radius: 3px;
    color: var(--muted);
  }

  .strip {
    display: flex;
    align-items: stretch;
    gap: 0.9rem 1.4rem;
    flex-wrap: wrap;
    padding: 0.6rem 0.9rem;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--r-panel);
  }
  .strip :global(.tempo .content) {
    min-width: 4.6ch;
  }
  small {
    font-size: 0.9rem;
    color: var(--muted);
  }
  .strip :global(.position .content) {
    display: grid;
    grid-template-columns: 6.2ch auto;
    grid-template-rows: auto auto;
    column-gap: 0.6rem;
    align-items: center;
  }
  .barnum {
    font-size: 1.5rem;
  }
  .beats {
    display: flex;
    gap: 5px;
  }
  .beat {
    width: 0.8rem;
    height: 0.8rem;
    border-radius: 50%;
    background: var(--lamp-off);
    border: 1px solid var(--line-strong);
  }
  .beat.down {
    border-color: var(--muted);
  }
  .beat.on {
    background: var(--accent);
    border-color: var(--accent);
    box-shadow: 0 0 8px var(--accent);
  }
  .sections {
    grid-column: 1 / -1;
    display: flex;
    gap: 0.5rem;
    font-size: 0.95rem;
    font-weight: 500;
    width: 16ch;
  }
  .next {
    color: var(--accent);
  }

  /* The chord: the one thing you read from across the stage. */
  .strip :global(.chord) {
    flex: 1 1 12rem;
    justify-content: center;
  }
  .strip :global(.chord .value) {
    align-items: center;
  }
  .strip :global(.chord .content) {
    display: flex;
    flex-direction: column;
    align-items: center;
    line-height: 1;
  }
  .chord-name {
    font-size: clamp(2.75rem, 5vw, 4rem);
    font-weight: 700;
    letter-spacing: 0.01em;
    min-width: 5ch;
    text-align: center;
  }
  .played {
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--muted);
    min-height: 1.1em;
  }

  .detect {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 0.3rem;
  }
  .fixed {
    display: inline-block;
    text-align: left;
  }
  .fing {
    width: 8.5rem;
  }
  .ul {
    width: 3.6rem;
  }
  .note {
    min-width: 2.6ch;
  }
  .tr {
    font-size: 1.1rem;
    margin-right: 0.5rem;
    color: var(--muted);
  }
  .tr.set {
    color: var(--accent);
  }

  /* Narrow windows: the style row gets its own line, the Launchkey name shrinks to its dot. */
  @media (max-width: 760px) {
    .bar {
      flex-wrap: wrap;
      gap: 0.5rem;
    }
    .style {
      order: 3;
      flex-basis: 100%;
    }
    .app-controls {
      margin-left: auto;
    }
    .lk-name,
    .mock {
      display: none;
    }
  }
</style>
