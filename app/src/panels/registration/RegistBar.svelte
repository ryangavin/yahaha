<!--
  The Registration bar, in the keyboard strip's panel above the keys (the Genos has these
  buttons by the keys): the Genos REGISTRATION MEMORY section and the Playlist, always in
  reach while you play. It is on the stage, so every size is in em of `--u`.

  [◀ Bank ▶] [1]…[10] [Memory] [Freeze] [Regist − 3/6 +] [◀ Song ▶] [Panel]

  The ten buttons light like pad page 4 (red in use, blue stored, dark empty; flashing
  while Memory is armed). Everything here is a command the Launchkey page 4, the terminal
  keys and the engine share; the Registration panel (`Registration.svelte`) has the rest.

  State: registration, playlist. Commands: pressRegist, stepRegistBank, toggleRegistMemory,
  toggleFreeze, stepRegistSequence, stepPlaylist.
-->
<script lang="ts">
  import { app, clock, ui } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import { REGIST_TIPS, buttonLook, buttonName, sequenceText } from './regist'

  const r = $derived(app.state.registration)
  const pl = $derived(app.state.playlist)
  const beats = $derived(clock.beats)
  const amber = { rgb: [127, 90, 20] as [number, number, number], level: 'bright' as const, anim: 'solid' as const }
  const armed = { rgb: [127, 0, 0] as [number, number, number], level: 'bright' as const, anim: 'flash' as const }
  const seqOn = $derived(r.sequence.on && r.sequence.steps.length > 0)
  const song = $derived.by(() => {
    const row = pl.records.find((x) => x.index === pl.current)
    return row ? row.record.name : pl.records.length ? `${pl.records.length} songs` : 'empty'
  })

  function openPanel(tab: typeof ui.registTab) {
    if (ui.regist && ui.registTab === tab) ui.regist = false
    else {
      ui.registTab = tab
      if (!ui.regist) ui.toggleDrawer('regist')
    }
  }
</script>

<section class="rbar" aria-label="Registration">
  <div class="group bank">
    <span class="engraved lbl">Regist bank</span>
    <HwButton tip="regist.bank_prev" label="Previous bank" onclick={() => app.send({ type: 'stepRegistBank', delta: -1 })}>◀</HwButton>
    <button type="button" class="name mat-screen" use:tip={'regist.bank'} onclick={() => openPanel('bank')}>
      <span class="glow-text">{r.bank.name}{r.bank.dirty ? ' *' : ''}</span>
    </button>
    <HwButton tip="regist.bank_next" label="Next bank" onclick={() => app.send({ type: 'stepRegistBank', delta: 1 })}>▶</HwButton>
  </div>

  <div class="group buttons" role="group" aria-label="Registration Memory 1 to 10">
    {#each r.buttons as b, i (b.index)}
      <div class="slot" title={buttonName(r, i)}>
        <HwButton tip={REGIST_TIPS[i]} led={buttonLook(r, i)} {beats} shape="square" label="Registration {i + 1}" onclick={() => app.send({ type: 'pressRegist', index: i })}>{i + 1}</HwButton>
        <span class="bname engraved">{buttonName(r, i)}</span>
      </div>
    {/each}
  </div>

  <div class="group">
    <HwButton tip="regist.memory" led={r.memory ? armed : null} {beats} pressed={r.memory} onclick={() => app.send({ type: 'toggleRegistMemory' })}>Memory</HwButton>
    <HwButton tip="regist.freeze" led={r.freeze ? amber : null} pressed={r.freeze} onclick={() => app.send({ type: 'toggleFreeze' })}>Freeze</HwButton>
  </div>

  <div class="group" class:off={!seqOn}>
    <span class="engraved lbl">Sequence</span>
    <HwButton tip="regist.seq_prev" label="Regist −" onclick={() => app.send({ type: 'stepRegistSequence', delta: -1 })}>−</HwButton>
    <button type="button" class="pos mat-screen" use:tip={'regist.sequence_on'} onclick={() => openPanel('sequence')}>
      <span class="glow-text">{seqOn ? sequenceText(r) : 'off'}</span>
    </button>
    <HwButton tip="regist.seq_next" label="Regist +" onclick={() => app.send({ type: 'stepRegistSequence', delta: 1 })}>+</HwButton>
  </div>

  <div class="group song">
    <span class="engraved lbl">Playlist</span>
    <HwButton tip="playlist.prev" label="Previous song" onclick={() => app.send({ type: 'stepPlaylist', delta: -1 })}>◀</HwButton>
    <button type="button" class="name mat-screen" use:tip={'playlist.file'} onclick={() => openPanel('playlist')}>
      <span class="glow-text">{song}</span>
    </button>
    <HwButton tip="playlist.next" label="Next song" onclick={() => app.send({ type: 'stepPlaylist', delta: 1 })}>▶</HwButton>
  </div>

  <div class="group">
    <HwButton tip="regist.open" led={ui.regist ? amber : null} onclick={() => openPanel(ui.registTab)}>Panel</HwButton>
  </div>
</section>

<style>
  /* One row across the strip (93em wide); in the stacked layout (63em) it wraps to two. */
  .rbar {
    display: flex;
    flex-wrap: nowrap;
    align-items: flex-start;
    justify-content: space-between;
    column-gap: 0.9em;
    row-gap: 0.4em;
    min-width: 0;
  }
  /* Level with the middle of the square 1–10 buttons (2.6em of their 0.9em print). */
  .group {
    display: flex;
    align-items: center;
    gap: 0.35em;
    min-width: 0;
    min-height: 2.34em;
    flex: 0 1 auto;
  }
  .buttons {
    flex: none;
    align-items: flex-start;
    min-height: 0;
    gap: 0.3em;
  }
  .group.off {
    opacity: 0.55;
  }
  .lbl {
    white-space: nowrap;
  }
  .slot {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.15em;
    width: 3.1em;
  }
  .bname {
    width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: center;
    min-height: 1em;
  }
  .rbar .bname {
    font-size: 0.62em;
  }
  .name,
  .pos {
    min-width: 0;
    height: 2.2em;
    padding: 0 0.6em;
    border: 0;
    border-radius: 0.3em;
    font-family: var(--font-display);
    font-size: 0.95em;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .bank .name {
    flex: 0 1 8.5em;
    min-width: 4.5em;
  }
  .song .name {
    flex: 0 1 8.5em;
    min-width: 4.5em;
  }
  .pos {
    width: 4em;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }
  @container stage (aspect-ratio < 1.45) {
    .rbar {
      flex-wrap: wrap;
      justify-content: flex-start;
    }
  }
</style>
