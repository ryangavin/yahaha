<!--
  An on/off setting for the drawers (Settings, Parts): a physical rocker-style switch
  with a small amber indicator LED. Use HwButton with an engine LED look for anything the
  Launchkey lights, so colours match the pads.
-->
<script lang="ts">
  import type { Snippet } from 'svelte'
  import type { TipKey } from '../../help/tooltips'
  import { tip as tipAction } from '../tooltip/tip.svelte'

  let { on, tip, onclick, children }: { on: boolean; tip: TipKey; onclick: () => void; children: Snippet } = $props()
</script>

<button type="button" class="toggle mat-raised" class:pressed={on} role="switch" aria-checked={on} use:tipAction={tip} {onclick}>
  <span class="led" class:on aria-hidden="true"></span>
  {@render children()}
</button>

<style>
  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 0.55em;
    min-height: 2.2em;
    padding: 0.2em 0.8em;
    border-radius: 5px;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.95em;
    color: var(--ink);
  }
  .led {
    width: 0.55em;
    height: 0.55em;
    border-radius: 50%;
    background: var(--lamp-off);
    box-shadow: inset 0 1px 1px rgb(0 0 0 / 0.6);
  }
  .led.on {
    background: var(--accent);
    box-shadow: 0 0 8px var(--accent), inset 0 -1px 1px rgb(0 0 0 / 0.3);
  }
</style>
