<!--
  The layout shell: the app bar, the Launchkey mirror (the main screen), and the panels
  that open around it. Each lives in its own folder under src/panels/.

  ┌ app bar: Parts & OTS · Mixer · Browse · Settings ·············· ? · theme ┐
  │ ┌ Launchkey mirror (panels/launchkey) ────────────────────────────────┐   │
  │ │ status display · pads · Pad Bank · tempo · transport · faders      │ ┌ drawer ┐
  │ └─────────────────────────────────────────────────────────────────────┘ │ parts  │
  │ status line                                                             │ mixer  │
  └──────────────────────────────────────────────────────────────────────── │settings│
  Browser: centred modal. Help bar: docks at the bottom in help mode.
-->
<script lang="ts">
  import { onDestroy } from 'svelte'
  import type { Session } from './lib/api/session'
  import { handleBlur, handleKey, handleKeyUp } from './lib/shortcuts'
  import { app, clock, ui } from './lib/store.svelte'
  import HelpBar from './lib/tooltip/HelpBar.svelte'
  import Tooltip from './lib/tooltip/Tooltip.svelte'
  import { tip, tips } from './lib/tooltip/tip.svelte'
  import Browser from './panels/browser/Browser.svelte'
  import Header from './panels/header/Header.svelte'
  import Launchkey from './panels/launchkey/Launchkey.svelte'
  import Mixer from './panels/mixer/Mixer.svelte'
  import Parts from './panels/parts/Parts.svelte'
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

<svelte:window onkeydown={handleKey} onkeyup={handleKeyUp} onblur={handleBlur} />

<div class="app">
  <Header />

  <main class="main">
    <Launchkey />
  </main>

  <!-- svelte-ignore a11y_no_noninteractive_tabindex (focusable so its tooltip is reachable from the keyboard) -->
  <footer class="status engraved" role="status" tabindex="0" use:tip={'display.status'}>
    <span class:error={message?.error}>{message?.text ?? ''}</span>
    <span>{unmapped}</span>
  </footer>

  <HelpBar />
</div>

{#if ui.parts}<Parts />{/if}
{#if ui.mixer}<Mixer />{/if}
{#if ui.settings}<Settings />{/if}
{#if ui.browser}<Browser />{/if}
<Tooltip />

<style>
  .app {
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
    max-width: 1760px;
    min-height: 100vh;
    margin: 0 auto;
    padding: 0.7rem 16px 0.9rem;
  }
  .main {
    display: flex;
    flex-direction: column;
    justify-content: center;
    flex: 1;
  }
  .status {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    min-height: 1.4rem;
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
    font-size: 0.8rem;
  }
  .error {
    color: var(--danger);
  }
</style>
