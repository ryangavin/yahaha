<script lang="ts">
  import type { Snippet } from 'svelte'

  /** Wraps a story in the toolbar's theme: the tokens apply under `data-theme`, not the page. */
  let { theme, children }: { theme: string; children?: Snippet } = $props()
</script>

<div class="theme-frame" data-theme={theme}>
  {@render children?.()}
</div>

<style>
  /* No box of its own, so the story's layout (centered, padded, fullscreen) is unchanged. */
  .theme-frame {
    display: contents;
    color: var(--t);
    font-family: var(--font-sans);
  }
</style>
