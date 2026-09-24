<!--
  The status display, where the Launchkey's screen is: style, tempo, bar/beat, the
  section playing and the one queued (left); the big chord (centre); fingering, chord
  detection, split and transpose (right). A recessed glass (`.mat-screen`) with glowing
  text. Every item has a tooltip and can be reached with Tab.
-->
<script lang="ts">
  import { sectionLabel } from '../../lib/api/types'
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'

  const s = $derived(app.state)
  const t = $derived(s.transport)
  const c = $derived(s.chord)
  const signed = (n: number) => (n > 0 ? `+${n}` : n < 0 ? `−${-n}` : '0')
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex (display items are focusable so their tooltips are reachable from the keyboard) -->
<div class="screen mat-screen" role="group" aria-label="Status display">
  <div class="col left">
    <span class="style glow-text" tabindex="0" use:tip={'style.name'}>{s.style.name || 'No style'}</span>
    <span class="tempo" tabindex="0" use:tip={'display.tempo'}>
      <span class="glow-text">{Math.round(t.tempo)}</span><small> BPM</small>
      <small class="ts">{s.style.timeSignature[0]}/{s.style.timeSignature[1]}</small>
    </span>
    <span class="pos" tabindex="0" use:tip={'display.position'}>
      <span class="bar glow-text">{t.running ? `${t.bar}.${t.beat}` : t.syncStart ? 'SYNC' : 'STOP'}</span>
      <span class="beats" aria-label="beat {t.beat} of {t.beatsPerBar}">
        {#each Array.from({ length: t.beatsPerBar }, (_, i) => i + 1) as b (b)}
          <span class="beat" class:on={t.running && b === t.beat} class:down={b === 1}></span>
        {/each}
      </span>
    </span>
    <span class="sections">
      <span class="now">{t.section ? sectionLabel(t.section) : t.pendingIntro !== null ? `Intro ${['I', 'II', 'III'][t.pendingIntro]} armed` : ''}</span>
      <span class="next">{t.queued ? `→ ${sectionLabel(t.queued)}` : ''}</span>
    </span>
  </div>

  <span class="chord" tabindex="0" use:tip={'display.chord'}>
    <span class="name glow-text">{c.name ?? '–'}</span>
    <span class="fingered">{c.fingered && c.fingered !== c.name ? `played ${c.fingered}` : ''}</span>
  </span>

  <div class="col right">
    <span tabindex="0" use:tip={'fingering.select'}>{c.fingeringName}</span>
    <span tabindex="0" use:tip={'detection.upper'}>{c.upper ? 'Upper' : 'Lower'}{c.manualBassActive ? ' · Manual Bass' : ''}</span>
    <span tabindex="0" use:tip={'split.display'}>Split {c.splitName}</span>
    <span tabindex="0" use:tip={'transpose.display'} class:set={c.transposeKeyboard !== 0 || c.transposeMaster !== 0}>
      Transpose {signed(c.transposeKeyboard)} · {signed(c.transposeMaster)}
    </span>
  </div>
</div>

<style>
  .screen {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1.25fr) minmax(0, 1fr);
    align-items: center;
    gap: 1em;
    height: 100%;
    padding: 0.7em 1.1em;
    border-radius: 0.5em;
    font-family: var(--font-display);
    overflow: hidden;
  }
  .screen span[tabindex] {
    border-radius: 3px;
  }
  .col {
    display: flex;
    flex-direction: column;
    gap: 0.25em;
    min-width: 0;
  }
  .col > * {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .style {
    font-weight: 600;
    font-size: 1.2em;
  }
  .tempo {
    font-weight: 600;
    font-size: 1.05em;
  }
  small {
    font-size: 0.72em;
    color: var(--screen-dim);
  }
  .ts {
    margin-left: 0.6em;
  }
  .pos {
    display: flex;
    align-items: center;
    gap: 0.5em;
  }
  .bar {
    font-size: 1.45em;
    font-weight: 600;
    min-width: 3.4ch;
  }
  .beats {
    display: flex;
    gap: 0.3em;
  }
  .beat {
    width: 0.55em;
    height: 0.55em;
    border-radius: 50%;
    background: #1d2a30;
  }
  .beat.down {
    box-shadow: inset 0 0 0 1px var(--screen-dim);
  }
  .beat.on {
    background: var(--screen-ink);
    box-shadow: 0 0 6px var(--screen-glow);
  }
  .sections {
    display: flex;
    gap: 0.6em;
    font-size: 0.95em;
    min-height: 1.3em;
  }
  .next {
    color: var(--accent);
  }
  .chord {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-width: 0;
  }
  .name {
    font-size: 4.4em;
    font-weight: 700;
    line-height: 1;
    letter-spacing: 0.01em;
    white-space: nowrap;
  }
  .fingered {
    font-size: 0.8em;
    color: var(--screen-dim);
    min-height: 1.2em;
  }
  .right {
    align-items: flex-end;
    text-align: right;
    font-size: 0.95em;
    color: var(--screen-dim);
  }
  .right > :first-child {
    color: var(--screen-ink);
    font-size: 1.1em;
    font-weight: 600;
  }
  .right .set {
    color: var(--accent);
  }
</style>
