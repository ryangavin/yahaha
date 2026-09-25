<!--
  The Registration bar, under the keyboard strip (the Genos has these buttons by the keys):
  the Genos REGISTRATION MEMORY section and the Playlist, always in reach while you play.

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

<section class="rbar mat-chassis" aria-label="Registration">
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

  <HwButton tip="regist.open" led={ui.regist ? amber : null} onclick={() => openPanel(ui.registTab)}>Panel</HwButton>
</section>

<style>
  .rbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    column-gap: 0.9rem;
    row-gap: 0.3rem;
    padding: 0.3rem 0.7rem;
    border-radius: 6px;
    font-size: 13px;
    overflow: hidden;
    flex-shrink: 0;
  }
  .group {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    min-width: 0;
    flex: 0 1 auto;
  }
  .buttons {
    flex: none;
  }
  .group.off {
    opacity: 0.55;
  }
  .lbl {
    white-space: nowrap;
  }
  .buttons {
    gap: 0.3rem;
  }
  .slot {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 3.4rem;
  }
  .bname {
    width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: center;
    font-size: 0.62rem;
    min-height: 0.9rem;
  }
  .name,
  .pos {
    min-width: 0;
    height: 2.1rem;
    padding: 0 0.6rem;
    border: 0;
    border-radius: 4px;
    font-family: var(--font-display);
    font-size: 0.95rem;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .bank .name {
    flex: 0 1 9rem;
    min-width: 4.5rem;
  }
  .song .name {
    flex: 0 1 10rem;
    min-width: 4.5rem;
  }
  .pos {
    width: 4.2rem;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }
  @media (max-width: 1400px) {
    .bname,
    .lbl {
      display: none;
    }
    .slot {
      width: auto;
    }
  }
  @media (max-width: 1150px) {
    .rbar {
      column-gap: 0.5rem;
    }
    .buttons {
      gap: 0.15rem;
    }
    .pos {
      width: 3.4rem;
    }
  }
</style>
