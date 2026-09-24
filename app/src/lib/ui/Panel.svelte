<!--
  A panel on the main screen: a titled region. `actions` go at the right of the title bar.
  Panels never change size with state (fixed rows/columns inside), so nothing shifts
  while you play.
-->
<script lang="ts">
  import type { Snippet } from 'svelte'

  let {
    id,
    title,
    actions,
    children,
    class: klass = '',
  }: { id: string; title: string; actions?: Snippet; children: Snippet; class?: string } = $props()
</script>

<section class="panel {klass}" aria-labelledby="{id}-title" data-panel={id}>
  <header>
    <h2 id="{id}-title">{title}</h2>
    {#if actions}<div class="actions">{@render actions()}</div>{/if}
  </header>
  <div class="body">{@render children()}</div>
</section>

<style>
  .panel {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--r-panel);
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    min-height: 2.5rem;
    padding: 0.35rem 0.75rem 0.25rem 0.9rem;
    border-bottom: 1px solid var(--line);
  }
  h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--fs-h);
    font-weight: 600;
    letter-spacing: 0.02em;
    color: var(--muted);
  }
  .actions {
    display: flex;
    gap: 0.4rem;
    align-items: center;
  }
  .body {
    padding: 0.75rem 0.9rem 0.9rem;
    flex: 1;
    min-height: 0;
  }
</style>
