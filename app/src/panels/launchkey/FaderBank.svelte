<!--
  The 8 faders + master and the button under each, rendered from the surface state: what
  each fader controls on the active page, its level, the soft-takeover mark and where the
  hardware fader physically is; each button's function (and Shift function) and light.
-->
<script lang="ts">
  import type { ControlId, SurfaceState } from '../../lib/api/types'
  import { app } from '../../lib/store.svelte'
  import { faderCmd, faderTip } from '../../lib/surface'
  import Fader from '../../lib/ui/Fader.svelte'
  import Control from './Control.svelte'

  let { surface }: { surface: SurfaceState } = $props()

  const BUTTONS: ControlId[] = [
    'faderButton1', 'faderButton2', 'faderButton3', 'faderButton4',
    'faderButton5', 'faderButton6', 'faderButton7', 'faderButton8', 'masterButton',
  ]
</script>

<div class="bank" role="group" aria-label="Faders">
  {#each surface.faders as f, i (i)}
    <div class="strip" class:master={i === 8}>
      <Fader
        value={f.value ?? 0}
        tip={faderTip(f)}
        label={f.label || '—'}
        pickup={f.waiting}
        hw={f.position}
        lit={surface.controls.find((c) => c.id === BUTTONS[i])?.level === 'bright'}
        disabled={!f.set}
        onchange={(v) => {
          const cmd = faderCmd(f, v)
          if (cmd) app.send(cmd)
        }}
      />
      <Control {surface} id={BUTTONS[i]} legend="" showLabel={i === 8} />
    </div>
  {/each}
</div>

<style>
  .bank {
    display: grid;
    grid-template-columns: repeat(8, minmax(0, 1fr)) minmax(0, 1.15fr);
    gap: 0.4em;
    height: 100%;
  }
  .strip {
    display: grid;
    grid-template-rows: 1fr auto;
    gap: 0.55em;
    min-width: 0;
  }
  .master {
    padding-left: 0.4em;
    border-left: 1px solid var(--seam);
  }
</style>
