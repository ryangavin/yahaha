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
  const hw = $derived(surfaceOf(app.state, app.library).faders[8]?.position ?? null)
</script>

<div class="master">
  <div class="name engraved">Master</div>
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
  <DrawerButton tip="drawer.mixer" open={ui.mixer} onclick={() => (ui.mixer = !ui.mixer)}>Details</DrawerButton>
</div>

<style>
  .master {
    display: grid;
    grid-template-rows: auto auto minmax(8rem, 1fr) auto auto;
    justify-items: center;
    gap: 0.3rem;
    min-width: 4.5rem;
    height: 100%;
    padding: 0.3rem 0.25rem;
    border-left: 1px solid var(--seam);
    color: var(--ink);
  }
  .name {
    font-family: var(--font-display);
    font-size: 0.8rem;
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
