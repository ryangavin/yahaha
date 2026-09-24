<!--
  One Launchkey button, rendered from the surface state: its face is the legend printed on
  the hardware (▲, ▶, ■ …), its caption and tooltip are what it does now on the layer
  showing (Shift or not), and its light is the state's. Clicking sends its action.
-->
<script lang="ts">
  import type { ControlId, SurfaceState } from '../../lib/api/types'
  import { app, clock, ui } from '../../lib/store.svelte'
  import { controlTip, layer } from '../../lib/surface'
  import HwButton from '../../lib/ui/HwButton.svelte'

  let {
    surface,
    id,
    legend,
    caption,
    shape = 'rect',
    showLabel = false,
  }: {
    surface: SurfaceState
    id: ControlId
    /** What's printed on the hardware button. */
    legend: string
    /** Engraved text under it; defaults to the control's function when it has a Shift function showing. */
    caption?: string
    shape?: 'rect' | 'square'
    /** Show the function label on the face instead of the legend. */
    showLabel?: boolean
  } = $props()

  const c = $derived(surface.controls.find((x) => x.id === id)!)
  const shift = $derived(ui.shift || surface.shift)
  const now = $derived(layer(c, shift))
  const shifted = $derived(shift && (c.shiftLabel !== c.label || c.shiftAction !== c.action))
</script>

<HwButton
  tip={controlTip(c, shift)}
  led={c}
  beats={clock.beats}
  label={now.label || legend}
  {shape}
  caption={caption ?? (shifted ? now.label : undefined)}
  onclick={() => now.action && app.send(now.action)}
>
  {#if showLabel || shifted}<span class="fn" class:shift={shifted}>{now.label}</span>{:else}<span class="legend">{legend}</span>{/if}
</HwButton>

<style>
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
