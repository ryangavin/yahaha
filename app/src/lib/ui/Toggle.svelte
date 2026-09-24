<!--
  An on/off setting that isn't a lamp key: a compact switch with a small indicator, lit in
  the accent colour while on. Use LampKey for anything the Launchkey lights, so the
  colours match the pads.
-->
<script lang="ts">
  import type { Snippet } from 'svelte'
  import type { TipKey } from '../../help/tooltips'
  import { tip as tipAction } from '../tooltip/tip.svelte'

  let { on, tip, onclick, children }: { on: boolean; tip: TipKey; onclick: () => void; children: Snippet } = $props()
</script>

<button type="button" class="toggle" role="switch" aria-checked={on} use:tipAction={tip} {onclick}>
  <span class="dot" class:on aria-hidden="true"></span>
  {@render children()}
</button>

<style>
  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    min-height: 2.25rem;
    padding: 0.2rem 0.7rem;
    background: var(--raised);
    border: 1px solid var(--line);
    border-radius: var(--r-key);
    font-family: var(--font-display);
    font-weight: 600;
    font-size: var(--fs-label);
  }
  .toggle:hover {
    border-color: var(--line-strong);
  }
  .dot {
    width: 0.6rem;
    height: 0.6rem;
    border-radius: 50%;
    background: var(--lamp-off);
    border: 1px solid var(--line-strong);
  }
  .dot.on {
    background: var(--accent);
    border-color: var(--accent);
    box-shadow: 0 0 8px var(--accent);
  }
</style>
