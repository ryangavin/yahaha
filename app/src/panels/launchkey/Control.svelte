<!--
  One Launchkey button, rendered from the surface state: its face is the legend printed on
  the hardware (▲, ▶, ■ …), and its tooltip is what it does now on the layer showing
  (Shift or not); its light is the state's. Clicking sends its action.

  What it does now shows in one of two places: engraved under it (`showFunction`, for the
  buttons with room below: Scene, Function, Stop, Play), or, where the mirror has no room
  (Pad Bank, Track), on the face in place of the legend while Shift gives it another job.
-->
<script lang="ts">
  import type { ControlId, SurfaceState } from '../../lib/api/types'
  import { app, clock, ui } from '../../lib/store.svelte'
  import { controlTip, hasShiftFunction, layer } from '../../lib/surface'
  import HwButton from '../../lib/ui/HwButton.svelte'

  let {
    surface,
    id,
    legend,
    caption,
    shape = 'rect',
    showFunction = false,
    pressed,
  }: {
    surface: SurfaceState
    id: ControlId
    /** What's printed on the hardware button. */
    legend: string
    /** Engraved text under it (e.g. the neighbouring style's name). */
    caption?: string
    shape?: 'rect' | 'square'
    /** Engrave the control's current function under it, and keep the legend on the face. */
    showFunction?: boolean
    /** A held button that is down now (Sound, while the state's layer says so). */
    pressed?: boolean
  } = $props()

  const c = $derived(surface.controls.find((x) => x.id === id)!)
  const shift = $derived(ui.shift || surface.shift)
  const now = $derived(layer(c, shift))
  const shifted = $derived(shift && hasShiftFunction(c))
</script>

<HwButton
  tip={controlTip(c, shift)}
  led={c}
  beats={clock.beats}
  label={now.label || legend}
  {shape}
  {pressed}
  caption={showFunction ? now.label : caption}
  onclick={() => now.action && app.send(now.action)}
>
  {#if shifted && !showFunction}<span class="fn shift">{now.label}</span><span class="legend hidden" aria-hidden="true">{legend}</span>
  {:else}<span class="legend">{legend}</span>{/if}
</HwButton>

<style>
  .hidden {
    display: none;
  }
  .legend {
    font-size: 1.05em;
    line-height: 1;
  }
  .fn {
    font-size: 0.72em;
  }
  .shift {
    color: var(--accent);
  }
</style>
