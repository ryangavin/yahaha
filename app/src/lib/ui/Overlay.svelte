<!--
  A modal panel over the main screen (style browser, settings). The shell closes it on
  Esc (`ui.escape()`); the close button needs its own catalog tip. Playing keeps working
  while it's open: MIDI and the Launchkey don't go through the UI.
-->
<script lang="ts">
  import type { Snippet } from 'svelte'
  import type { TipKey } from '../../help/tooltips'
  import Key from './Key.svelte'

  let {
    id,
    title,
    closeTip,
    onclose,
    children,
    side = 'center',
  }: { id: string; title: string; closeTip: TipKey; onclose: () => void; children: Snippet; side?: 'center' | 'right' } = $props()

  let box: HTMLDivElement | undefined = $state()
  $effect(() => {
    // Take focus so keys go to the overlay, and give it back on close.
    const prev = document.activeElement as HTMLElement | null
    box?.focus()
    return () => prev?.focus?.()
  })
</script>

<div class="scrim" role="presentation" onclick={onclose}></div>
<div class="overlay {side}" role="dialog" aria-modal="true" aria-labelledby="{id}-title" tabindex="-1" bind:this={box} data-overlay={id}>
  <header>
    <h2 id="{id}-title">{title}</h2>
    <Key tip={closeTip} size="s" onclick={onclose}>Close</Key>
  </header>
  <div class="body">{@render children()}</div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 40;
    background: rgb(0 0 0 / 0.45);
  }
  .overlay {
    position: fixed;
    z-index: 50;
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border: 1px solid var(--line-strong);
    border-radius: var(--r-panel);
    box-shadow: 0 24px 60px -12px rgb(0 0 0 / 0.6);
    outline: none;
  }
  .center {
    inset: 5vh max(16px, calc(50vw - 36rem));
  }
  .right {
    top: 16px;
    right: 16px;
    bottom: 16px;
    width: min(30rem, calc(100vw - 32px));
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.6rem 0.75rem 0.6rem 1rem;
    border-bottom: 1px solid var(--line);
  }
  h2 {
    margin: 0;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 1.35rem;
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 1rem;
  }
</style>
