<!--
  "Audio dropouts detected": a small notice over the status line, not modal, when the
  engine's dropout count keeps rising (lib/dropouts.svelte.ts decides when). Its button
  opens Settings › Audio, where the buffer size is; × puts it away for ten minutes. A new
  buffer size clears it.
-->
<script lang="ts">
  import { nav } from '../panels/settings/nav.svelte'
  import { dropouts } from './dropouts.svelte'
  import { app, ui } from './store.svelte'
  import { tip } from './tooltip/tip.svelte'

  $effect(() => {
    const s = app.state.io.synth
    dropouts.observe(s ? { dropouts: s.dropouts ?? 0, bufferFrames: s.bufferFrames } : null, Date.now())
  })

  const openAudio = () => {
    nav.tab = 'audio'
    ui.settings = true
  }
</script>

{#if dropouts.show}
  <div class="notice mat-raised" role="alert">
    <span>Audio dropouts detected — consider increasing the buffer size.</span>
    <button type="button" class="go" use:tip={'audio.dropouts'} onclick={openAudio}>Buffer size…</button>
    <button type="button" class="close" aria-label="Dismiss" use:tip={'audio.dropouts_dismiss'} onclick={() => dropouts.dismiss(Date.now())}>×</button>
  </div>
{/if}

<style>
  .notice {
    position: fixed;
    left: 16px;
    bottom: calc(var(--help-footer-space, 16px) + 2.2rem);
    z-index: 30;
    display: flex;
    align-items: center;
    gap: 0.6rem;
    max-width: calc(100vw - 32px);
    padding: 0.4rem 0.5rem 0.4rem 0.8rem;
    border-radius: 6px;
    border-left: 3px solid var(--danger);
    font-size: 0.85rem;
  }
  button {
    font: inherit;
    cursor: pointer;
    border: 0;
    border-radius: 4px;
    padding: 0.2rem 0.5rem;
    background: transparent;
    color: inherit;
  }
  .go {
    text-decoration: underline;
    white-space: nowrap;
  }
  .close {
    font-size: 1.1rem;
    line-height: 1;
  }
  button:hover,
  button:focus-visible {
    background: color-mix(in srgb, currentColor 12%, transparent);
  }
</style>
