<!--
  Help mode's docked bar: the full catalog entry for the last control you hovered or
  tabbed to stays here, so you can read it while you try the control.
-->
<script lang="ts">
  import TipCard from './TipCard.svelte'
  import { tips } from './tip.svelte'

  const key = $derived(tips.key ?? tips.pinned)
</script>

{#if tips.help}
  <aside class="helpbar" aria-live="polite" aria-label="Help">
    {#if key}
      <TipCard {key} wide />
    {:else}
      <p>Help mode is on. Hover over or tab to any control to see what it does, its key and where it is on the Launchkey. Press <kbd>?</kbd> to turn help off.</p>
    {/if}
  </aside>
{/if}

<style>
  .helpbar {
    position: sticky;
    bottom: 0;
    z-index: 30;
    min-height: 5.5rem;
    padding: 0.8rem 1.1rem;
    background: var(--tip-bg);
    border: 1px solid var(--accent);
    border-radius: var(--r-panel);
    box-shadow: 0 -8px 24px -12px rgb(0 0 0 / 0.5);
  }
  p {
    margin: 0;
    color: var(--muted);
  }
  kbd {
    font-family: var(--font-display);
    font-weight: 600;
    padding: 0 0.35em;
    border: 1px solid var(--line-strong);
    border-radius: 4px;
  }
</style>
