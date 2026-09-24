<!--
  The layout shell. Each panel lives in its own folder under src/panels/ and is placed in
  a named grid area here; see app/CONTRIBUTING.md before adding one.

  ┌──────────────────────── header ────────────────────────┐
  │ sections                        │ launchkey             │
  │ parts (+ OTS)                   │ mixer                 │
  └─────────────────────── status ─────────────────────────┘
  Overlays: browser (center), settings (right). Help bar docks at the bottom in help mode.
-->
<script lang="ts">
  import { onDestroy } from 'svelte'
  import type { Session } from './lib/api/session'
  import { handleKey } from './lib/shortcuts'
  import { app, clock, ui } from './lib/store.svelte'
  import HelpBar from './lib/tooltip/HelpBar.svelte'
  import Tooltip from './lib/tooltip/Tooltip.svelte'
  import { tip, tips } from './lib/tooltip/tip.svelte'
  import Browser from './panels/browser/Browser.svelte'
  import Header from './panels/header/Header.svelte'
  import Launchkey from './panels/launchkey/Launchkey.svelte'
  import Mixer from './panels/mixer/Mixer.svelte'
  import Parts from './panels/parts/Parts.svelte'
  import Sections from './panels/sections/Sections.svelte'
  import Settings from './panels/settings/Settings.svelte'

  let { session }: { session: Session } = $props()

  $effect(() => {
    app.attach(session)
    clock.start()
    return () => {
      clock.stop()
      app.detach()
    }
  })
  onDestroy(() => session.dispose())

  $effect(() => {
    document.documentElement.dataset.theme = ui.theme
    document.documentElement.dataset.help = tips.help ? 'on' : 'off'
  })

  const message = $derived(app.state.message)
  const unmapped = $derived(app.state.io.unmapped)
</script>

<svelte:window onkeydown={handleKey} />

<div class="app">
  <div class="area header"><Header /></div>

  <main class="main">
    <div class="area sections"><Sections /></div>
    <div class="area launchkey"><Launchkey /></div>
    <div class="area parts"><Parts /></div>
    <div class="area mixer"><Mixer /></div>
  </main>

  <!-- svelte-ignore a11y_no_noninteractive_tabindex (focusable so its tooltip is reachable from the keyboard) -->
  <footer class="status" role="status" tabindex="0" use:tip={'display.status'}>
    <span class:error={message?.error}>{message?.text ?? ''}</span>
    <span class="unmapped">{unmapped}</span>
  </footer>

  <HelpBar />
</div>

{#if ui.browser}<Browser />{/if}
{#if ui.settings}<Settings />{/if}
<Tooltip />

<style>
  .app {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    max-width: 1680px;
    min-height: 100vh;
    margin: 0 auto;
    padding: 0.6rem 16px 0.75rem;
  }
  .main {
    display: grid;
    grid-template-columns: minmax(0, 7fr) minmax(0, 5fr);
    grid-template-areas:
      'sections launchkey'
      'parts mixer';
    gap: 0.6rem;
    align-items: stretch;
  }
  .area {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .area > :global(*) {
    flex: 1;
  }
  .sections {
    grid-area: sections;
  }
  .launchkey {
    grid-area: launchkey;
  }
  .parts {
    grid-area: parts;
  }
  .mixer {
    grid-area: mixer;
  }
  .status {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    min-height: 1.6rem;
    padding: 0.15rem 0.5rem;
    font-size: var(--fs-small);
    color: var(--muted);
    border-radius: 4px;
  }
  .error {
    color: var(--danger);
  }
  @media (max-width: 1100px) {
    .main {
      grid-template-columns: minmax(0, 1fr);
      grid-template-areas: 'sections' 'launchkey' 'parts' 'mixer';
    }
  }
</style>
