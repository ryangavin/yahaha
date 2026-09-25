<!--
  The transport section in the app bar (Header.svelte): the Genos STYLE CONTROL and TEMPO
  buttons in one row, always in reach whatever page the Launchkey is on.

  [▶ Start] [Sync Start] [Sync Stop] · Intro [I][II][III] · Ending [I][II][III] ·
  Tempo [−] 104 [+] [Tap] · 13.1 ●○○○ Main B → Fill B

  The Launchkey mirror's own Stop/Play and Tempo buttons stay as the hardware has them;
  these are the same commands. Every button lights from the engine's pad lamps
  (`transport.lamps`, pad page 1), so it flashes and pulses exactly like the pad, and
  takes that pad's catalog entry. Space is Start/Stop everywhere (lib/keys.ts).

  State: transport (running, syncStart, bar, beat, beatsPerBar, tempo, section, queued,
  pendingIntro, lamps). Commands: startStop, toggleSyncStart, toggleSyncStop, intro,
  ending, tempoDown, tempoUp, tapTempo.
-->
<script lang="ts">
  import type { TipKey } from '../../help/tooltips'
  import { tipFor } from '../../help/actions'
  import { sectionLabel, type AppCmd, type Pad } from '../../lib/api/types'
  import { formatTempo } from '../../lib/format'
  import { app, clock } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'

  const t = $derived(app.state.transport)
  const beats = $derived(clock.beats)
  const ROMAN = ['I', 'II', 'III']

  /** The page-1 pad that sends `cmd`: its light is this button's light. */
  function lamp(cmd: AppCmd): Pad | null {
    return (
      t.lamps.find((p) => p.action !== null && p.action.type === cmd.type && ('index' in cmd ? 'index' in p.action && p.action.index === cmd.index : true)) ??
      null
    )
  }

  const start: AppCmd = { type: 'startStop' }
  const syncStart: AppCmd = { type: 'toggleSyncStart' }
  const syncStop: AppCmd = { type: 'toggleSyncStop' }
  const tap: AppCmd = { type: 'tapTempo' }
  const intros: AppCmd[] = [0, 1, 2].map((index) => ({ type: 'intro', index }))
  const endings: AppCmd[] = [0, 1, 2].map((index) => ({ type: 'ending', index }))

  const state = $derived(t.running ? 'playing' : t.syncStart ? 'armed' : 'stopped')
  const now = $derived(
    t.section ? sectionLabel(t.section) : t.pendingIntro !== null ? `Intro ${ROMAN[t.pendingIntro]} armed` : '',
  )
  const send = (cmd: AppCmd) => app.send(cmd)
  const tipOf = (cmd: AppCmd): TipKey => tipFor(cmd)
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex (readouts are focusable so their tooltips are reachable from the keyboard) -->
<section class="tbar mat-chassis" aria-label="Transport" data-state={state}>
  <div class="group">
    <HwButton tip={tipOf(start)} led={lamp(start)} {beats} width="4.6em" label={t.running ? 'Stop' : 'Start'} onclick={() => send(start)}>
      <span class="icon">{t.running ? '■' : '▶'}</span>{t.running ? 'Stop' : 'Start'}
    </HwButton>
    <HwButton tip={tipOf(syncStart)} led={lamp(syncStart)} {beats} pressed={t.syncStart} onclick={() => send(syncStart)}>
      <span class="long">Sync Start</span><span class="short">Sync<br />Start</span>
    </HwButton>
    <HwButton tip={tipOf(syncStop)} led={lamp(syncStop)} {beats} pressed={t.syncStop} onclick={() => send(syncStop)}>
      <span class="long">Sync Stop</span><span class="short">Sync<br />Stop</span>
    </HwButton>
  </div>

  <div class="group" role="group" aria-label="Intro">
    <span class="engraved lbl">Intro</span>
    {#each intros as cmd, i (i)}
      <HwButton tip={tipOf(cmd)} led={lamp(cmd)} {beats} label="Intro {ROMAN[i]}" onclick={() => send(cmd)}>{ROMAN[i]}</HwButton>
    {/each}
  </div>

  <div class="group" role="group" aria-label="Ending">
    <span class="engraved lbl">Ending</span>
    {#each endings as cmd, i (i)}
      <HwButton tip={tipOf(cmd)} led={lamp(cmd)} {beats} label="Ending {ROMAN[i]}" onclick={() => send(cmd)}>{ROMAN[i]}</HwButton>
    {/each}
  </div>

  <div class="group" role="group" aria-label="Tempo">
    <span class="engraved lbl tempo-lbl">Tempo</span>
    <HwButton tip="tempo.down" label="Tempo −" onclick={() => send({ type: 'tempoDown' })}>−</HwButton>
    <span class="readout bpm mat-screen" tabindex="0" use:tip={'display.tempo'}>
      <span class="glow-text">{formatTempo(t.tempo)}</span><small>BPM</small>
    </span>
    <HwButton tip="tempo.up" label="Tempo +" onclick={() => send({ type: 'tempoUp' })}>+</HwButton>
    <HwButton tip={tipOf(tap)} led={lamp(tap)} {beats} onclick={() => send(tap)}>Tap</HwButton>
  </div>

  <span class="readout pos mat-screen" tabindex="0" use:tip={'display.position'}>
    <span class="bar glow-text">{t.running ? `${t.bar}.${t.beat}` : t.syncStart ? 'SYNC' : 'STOP'}</span>
    <span class="beats" aria-label="beat {t.beat} of {t.beatsPerBar}">
      {#each Array.from({ length: t.beatsPerBar }, (_, i) => i + 1) as b (b)}
        <span class="beat" class:on={t.running && b === t.beat} class:down={b === 1}></span>
      {/each}
    </span>
    <span class="now">{now}</span>
    <span class="next">{t.queued ? `→ ${sectionLabel(t.queued)}` : ''}</span>
  </span>
</section>

<style>
  .tbar {
    display: flex;
    align-items: center;
    column-gap: 1rem;
    padding: 0.3rem 0.7rem;
    border-radius: 6px;
    font-size: 13px;
    flex: 0 1 auto;
    min-width: 0;
    /* In the app bar: centred between the name and the app's own controls. */
    margin-inline: auto;
    overflow: hidden;
  }
  /* Large windows: a size up, so the transport reads as the app bar's main section. */
  @media (min-width: 1600px) {
    .tbar {
      font-size: 14.5px;
      column-gap: 1.3rem;
    }
  }
  .group {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    flex: none;
  }
  .lbl {
    white-space: nowrap;
    margin-right: 0.1rem;
  }
  .icon {
    font-size: 0.85em;
  }
  .short {
    display: none;
  }
  .readout {
    display: flex;
    align-items: center;
    height: 2.1rem;
    padding: 0 0.6rem;
    border-radius: 4px;
    font-family: var(--font-display);
    white-space: nowrap;
  }
  .bpm {
    gap: 0.2rem;
    justify-content: center;
    width: 4.4rem;
    font-size: 1.15rem;
    font-weight: 600;
  }
  .bpm small {
    font-size: 0.6rem;
    color: var(--screen-dim);
  }
  /* Bar.beat, the beat lights and the sections: fixed widths, so nothing moves as it plays. */
  .pos {
    flex: 0 1 24rem;
    min-width: 8.5rem;
    gap: 0.5rem;
    overflow: hidden;
  }
  .bar {
    font-size: 1.15rem;
    font-weight: 600;
    min-width: 3.2rem;
  }
  .beats {
    display: flex;
    gap: 0.25rem;
    flex: none;
  }
  .beat {
    width: 0.5rem;
    height: 0.5rem;
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
  .now,
  .next {
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 0.95rem;
  }
  .now {
    color: var(--screen-ink);
    flex: 0 1 auto;
    min-width: 0;
  }
  .next {
    color: var(--accent);
    flex: 1 1 0;
    min-width: 0;
  }
  @media (max-width: 1260px) {
    .tbar {
      column-gap: 0.5rem;
      padding: 0.3rem 0.5rem;
    }
    .long,
    .bpm small {
      display: none;
    }
    .short {
      display: inline-block;
      font-size: 0.68rem;
      line-height: 1;
      text-align: center;
    }
    .tempo-lbl {
      display: none;
    }
    .bpm {
      width: 3.2rem;
    }
  }
  /* Toward the 900px minimum: smaller print, Intro/Ending engraved upright beside their buttons,
     and bar.beat with the beat lights only (the lead-sheet band shows the sections). */
  @media (max-width: 1080px) {
    .tbar {
      font-size: 12px;
      column-gap: 0.4rem;
      padding: 0.3rem 0.45rem;
    }
    .short {
      font-size: 0.6rem;
    }
    .group {
      gap: 0.25rem;
    }
    .bpm {
      width: 2.8rem;
    }
    .lbl {
      writing-mode: vertical-rl;
      transform: rotate(180deg);
      font-size: 0.56rem;
      line-height: 1;
      margin-right: 0;
    }
    .pos {
      flex: none;
      min-width: 0;
      gap: 0.35rem;
      padding: 0 0.4rem;
    }
    .bar {
      min-width: 2.5rem;
    }
    .now,
    .next {
      display: none;
    }
  }
</style>
