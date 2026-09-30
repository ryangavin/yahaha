<!--
  The master column at the end of the mixer row: the Master Compressor and EQ
  (MasterFx: each on/off, ▸ for their types and parameters), the master fader (the
  synth's output level, `setMasterVolume`; Launchkey fader 9, with its soft-takeover mark
  and ghost), and the Details switch that shows the row's details (`ui.mixer`): the bar's
  extras and each strip's channel, Chorus, EQ, insert and CPU.
-->
<script lang="ts">
  import { app, ui } from '../../lib/store.svelte'
  import { surfaceOf } from '../../lib/surface'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import DrawerButton from '../../lib/ui/DrawerButton.svelte'
  import Fader from '../../lib/ui/Fader.svelte'
  import MasterFx from './MasterFx.svelte'

  const mixer = $derived(app.state.mixer)
  /** Launchkey fader 9 is the master. */
  const fader9 = $derived(surfaceOf(app.state, app.library).faders[8])
  const hw = $derived(fader9?.position ?? null)
  /** Its badge: M while the master fader moves the master (not without the synth). */
  const onFader = $derived(fader9?.set?.type === 'setMasterVolume')
</script>

<div class="master">
  <div class="head">
    <div class="name engraved">Master</div>
    {#if onFader}
      <span class="badge" use:tip={'stage.fader_badge'}>M</span>
    {/if}
  </div>
  <MasterFx />
  <div class="fader">
    <Fader
      value={mixer.master ?? 0}
      tip="mixer.master"
      label="Master"
      pickup={mixer.masterWaiting}
      {hw}
      disabled={mixer.master === null}
      onchange={(v) => app.send({ type: 'setMasterVolume', volume: v })}
    />
  </div>
  <div class="headroom" use:tip={'mixer.master'}>
    {#if mixer.master === null}Synth off{:else}100 = unity{/if}
  </div>
  <DrawerButton tip="drawer.mixer" open={ui.mixer} onclick={() => ui.toggleMixer()}>Details</DrawerButton>
</div>

<style>
  .master {
    display: grid;
    grid-template-rows: auto auto minmax(8rem, 1fr) auto auto;
    justify-items: center;
    gap: 0.3rem;
    flex: 1 1 auto;
    min-width: 4.5rem;
    height: 100%;
    padding: 0.3rem 0.25rem;
    border-left: 1px solid var(--seam);
    color: var(--ink);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }
  .name {
    font-family: var(--font-display);
    font-size: 0.8rem;
  }
  /* Launchkey fader 9, as the strips' F1–F8 badges. */
  .badge {
    padding: 0 0.22rem;
    border-radius: 3px;
    background: var(--raised);
    box-shadow: inset 0 0 0 1px var(--line);
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 0.6rem;
    line-height: 0.95rem;
    color: var(--ink);
  }
  .fader {
    height: 100%;
    min-height: 0;
    width: 100%;
    display: flex;
    justify-content: center;
    font-size: 0.9rem;
  }
  /* The fader's cap rides a full-height carrier moved by transform: clip it to the track. */
  .fader :global(.track) {
    overflow: clip;
  }
  .headroom {
    font-size: 0.66rem;
    color: var(--muted);
    white-space: nowrap;
  }
</style>
