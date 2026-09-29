<!--
  The 8 faders + master and the button under each, rendered from the surface state: what
  each fader controls on the active page, its level, the soft-takeover mark and where the
  hardware fader physically is; each button's function (and Shift function) and light. On
  the Panel page, faders 1–4 also name their part's sound (● edited, ⚠ plugin missing):
  clicking it opens the Sound Browser on that part.
-->
<script lang="ts">
  import type { ControlId, SurfaceState } from '../../lib/api/types'
  import { app, ui } from '../../lib/store.svelte'
  import { mirror } from '../../lib/mirror.svelte'
  import { faderCmd, faderTip } from '../../lib/surface'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import Fader from '../../lib/ui/Fader.svelte'
  import { isMissing, soundLabel } from '../rack/rack'
  import Control from './Control.svelte'

  let { surface }: { surface: SurfaceState } = $props()

  const panel = $derived(app.state.mixer.faderPage === 'panel')
  const parts = $derived(app.state.keyboardParts)

  const BUTTONS: ControlId[] = [
    'faderButton1', 'faderButton2', 'faderButton3', 'faderButton4',
    'faderButton5', 'faderButton6', 'faderButton7', 'faderButton8', 'masterButton',
  ]

  // A drawer is pointing at a keyboard part (lib/mirror): light its fader strip on the
  // Panel page, or the fader page button that gets you there from the Style page.
  const linked = $derived.by(() => {
    const f = mirror.panelFader
    if (f === null) return -1
    return app.state.mixer.faderPage === 'panel' ? f : 8
  })
</script>

<div class="bank" role="group" aria-label="Faders">
  {#each surface.faders as f, i (i)}
    <div class="strip" class:master={i === 8} class:linked={linked === i}>
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
      <!-- Panel page, faders 1–4: the part's sound (docs/racks.md "Screens"); a blank row elsewhere keeps the faders level. -->
      {#if panel && i < 4 && parts[i]}
        {@const p = parts[i]}
        <button
          type="button"
          class="sname"
          class:edited={p.soundEdited}
          class:missing={isMissing(p)}
          aria-label="{p.name} sound: {soundLabel(p)}"
          use:tip={'launchkey.fader_sound'}
          onclick={() => (ui.soundBrowser = i)}>{soundLabel(p)}</button
        >
      {:else}
        <span class="sname blank" aria-hidden="true"></span>
      {/if}
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
    grid-template-rows: 1fr auto auto;
    gap: 0.4em;
    min-width: 0;
  }
  /* The part's sound under its fader: click to change it. */
  .sname {
    height: 1.5em;
    min-width: 0;
    padding: 0 0.2em;
    border: 0;
    border-radius: 3px;
    background: none;
    color: var(--muted);
    font-family: var(--font-display);
    font-size: 0.72em;
    font-weight: 600;
    line-height: 1.5em;
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    cursor: pointer;
  }
  button.sname:hover,
  button.sname:focus-visible {
    color: var(--ink);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }
  .sname.edited {
    color: var(--accent);
  }
  .sname.missing {
    color: var(--danger, #e66);
  }
  .sname.blank {
    cursor: default;
  }
  /* Highlighted from a drawer (lib/mirror). Static: nothing here animates. */
  .strip {
    border-radius: 6px;
    outline: 2px solid transparent;
    outline-offset: 3px;
  }
  .linked {
    outline-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }
  .master {
    padding-left: 0.4em;
    border-left: 1px solid var(--seam);
  }
</style>
