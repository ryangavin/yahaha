<!--
  A small secondary button on the stage that opens one of the drawers (Mixer by the
  faders, Multi Pads by the pads, Charts by the lead-sheet lane…). Quieter than the
  mirrored hardware buttons (HwButton), so it never reads as a Launchkey control: an
  engraved face like the keyboard strip's size switch, underlined amber while its drawer
  is open. Sized in em of the surrounding panel print, with a 2.1em hit target.
-->
<script lang="ts">
  import type { Snippet } from 'svelte'
  import type { TipKey } from '../../help/tooltips'
  import { tip as tipAction } from '../tooltip/tip.svelte'

  let {
    tip,
    open,
    onclick,
    children,
    label,
  }: {
    tip: TipKey
    /** Its drawer is open. */
    open: boolean
    onclick: () => void
    children: Snippet
    /** Accessible name when the face is abbreviated. */
    label?: string
  } = $props()
</script>

<button type="button" class="drawer-btn" class:open aria-pressed={open} aria-label={label} use:tipAction={tip} {onclick}>
  {@render children()}
</button>

<style>
  .drawer-btn {
    flex: none;
    height: 2.5em;
    padding: 0 0.75em;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.84em;
    letter-spacing: 0.03em;
    white-space: nowrap;
    color: var(--engrave);
    background: linear-gradient(180deg, var(--raised-hi), var(--raised-lo));
    border: 1px solid rgb(0 0 0 / 0.45);
    border-radius: 0.35em;
    box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.14);
  }
  .drawer-btn:hover {
    color: var(--ink);
  }
  .drawer-btn:active {
    transform: translateY(1px);
  }
  .drawer-btn.open {
    color: var(--ink);
    box-shadow: inset 0 -2px 0 var(--accent), inset 0 1px 0 rgb(255 255 255 / 0.14);
  }
</style>
