<!--
  Placeholder body for a panel that hasn't been built yet: says what belongs there, so
  the layout shell shows every panel's place and size. Delete it from your panel when
  you build the real thing.
-->
<script lang="ts">
  let { items }: { items: string[] } = $props()
</script>

<div class="slot">
  <p>Not built yet. This panel will show:</p>
  <ul>
    {#each items as it (it)}<li>{it}</li>{/each}
  </ul>
</div>

<style>
  .slot {
    height: 100%;
    min-height: 8rem;
    padding: 0.75rem 1rem;
    border: 1px dashed var(--line-strong);
    border-radius: var(--r-key);
    color: var(--muted);
    font-size: var(--fs-small);
  }
  p {
    margin: 0 0 0.4rem;
  }
  ul {
    margin: 0;
    padding-left: 1.1rem;
  }
</style>
