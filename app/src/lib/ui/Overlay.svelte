<!--
  A panel around the hardware view, in the same brushed-metal material.
  - `side="right"`: a drawer (Keyboard parts + OTS, Mixer, Settings). Not modal: the
    mirror stays usable and performance keys keep working.
  - `side="center"` + `modal`: the style browser. A scrim, focus moves in, and it owns
    the keyboard until closed.
  The shell closes the topmost one on Esc (`ui.escape()`); the close button needs a
  catalog tip.
-->
<script lang="ts">
  import type { Snippet } from 'svelte'
  import type { TipKey } from '../../help/tooltips'
  import HwButton from './HwButton.svelte'

  let {
    id,
    title,
    closeTip,
    onclose,
    children,
    side = 'right',
    modal = false,
  }: { id: string; title: string; closeTip: TipKey; onclose: () => void; children: Snippet; side?: 'center' | 'right'; modal?: boolean } = $props()

  let box: HTMLDivElement | undefined = $state()
  $effect(() => {
    if (!modal) return
    // Take focus so keys go to the overlay, and give it back on close.
    const prev = document.activeElement as HTMLElement | null
    box?.focus()
    return () => prev?.focus?.()
  })
</script>

{#if modal}<div class="scrim" role="presentation" onclick={onclose}></div>{/if}
<div
  class="overlay mat-chassis {side}"
  role={modal ? 'dialog' : 'complementary'}
  aria-modal={modal || undefined}
  aria-labelledby="{id}-title"
  tabindex="-1"
  bind:this={box}
  data-overlay={id}
>
  <header>
    <h2 id="{id}-title" class="engraved">{title}</h2>
    <HwButton tip={closeTip} onclick={onclose}>Close</HwButton>
  </header>
  <div class="body">{@render children()}</div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 40;
    background: rgb(0 0 0 / 0.5);
  }
  .overlay {
    position: fixed;
    z-index: 50;
    display: flex;
    flex-direction: column;
    border-radius: var(--r-panel);
    outline: none;
    font-size: 15px;
  }
  /* Both end above the help footer, which explains their controls. */
  .center {
    inset: 5vh max(16px, calc(50vw - 36rem)) max(5vh, var(--help-footer-space, 16px));
  }
  .right {
    top: 3.9rem;
    right: 16px;
    bottom: var(--help-footer-space, 16px);
    width: min(30rem, calc(100vw - 32px));
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.6rem 0.75rem 0.6rem 1rem;
    border-bottom: 1px solid var(--seam);
    box-shadow: 0 1px 0 rgb(255 255 255 / 0.04);
  }
  h2 {
    margin: 0;
    font-size: 0.95rem;
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 1rem;
  }
</style>
