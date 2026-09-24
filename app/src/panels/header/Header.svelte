<!--
  The app bar above the hardware view: the drawers around it (Keyboard parts + OTS,
  Mixer, Chord Looper, Browse, Settings) and the app's own controls (help mode, theme). Everything the
  hardware does lives on the Launchkey mirror, not here.

  REFERENCE for a small component: read `app.state` with `$derived`, act with
  `app.send(...)` or the `ui` store, and give every control a catalog `tip`.
-->
<script lang="ts">
  import { app, ui } from '../../lib/store.svelte'
  import { tips } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'

  const s = $derived(app.state)
  const amber = { rgb: [127, 90, 20] as [number, number, number], level: 'bright' as const, anim: 'solid' as const }
</script>

<header class="bar">
  <span class="brand" aria-label="yahaha">yahaha</span>
  <span class="engraved sub">software arranger</span>

  <nav class="drawers" aria-label="Panels">
    <HwButton tip="drawer.parts" led={ui.parts ? amber : null} onclick={() => ui.toggleDrawer('parts')}>Parts & OTS</HwButton>
    <HwButton tip="drawer.mixer" led={ui.mixer ? amber : null} onclick={() => ui.toggleDrawer('mixer')}>Mixer</HwButton>
    <HwButton tip="drawer.looper" led={ui.looper ? amber : null} onclick={() => ui.toggleDrawer('looper')}>Chord Looper</HwButton>
    <HwButton tip="drawer.multipad" led={ui.multipad ? amber : null} onclick={() => ui.toggleDrawer('multipad')}>Multi Pads</HwButton>
    <HwButton tip="drawer.harmony" led={ui.harmony ? amber : null} onclick={() => ui.toggleDrawer('harmony')}>Harmony/Arp</HwButton>
    <HwButton tip="browser.open" led={ui.browser ? amber : null} onclick={() => (ui.browser = true)}>Browse styles</HwButton>
    <HwButton tip="settings.open" led={ui.settings ? amber : null} onclick={() => ui.toggleDrawer('settings')}>Settings</HwButton>
  </nav>

  <div class="app-controls">
    {#if app.kind === 'mock' || s.io.offline}<span class="engraved badge">{app.kind === 'mock' ? 'mock session' : 'offline session'}</span>{/if}
    <HwButton tip="app.help" pressed={tips.help} led={tips.help ? amber : null} label="Help mode" onclick={() => tips.toggleHelp()}>?</HwButton>
    <HwButton tip="app.theme" label="Light or dark theme" onclick={() => ui.setTheme(ui.theme === 'dark' ? 'light' : 'dark')}>
      {ui.theme === 'dark' ? '☾' : '☀'}
    </HwButton>
  </div>
</header>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 1rem;
    min-height: 2.9rem;
    font-size: 15px;
  }
  .brand {
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 1.5rem;
    letter-spacing: 0.06em;
    color: var(--ink);
    text-shadow: 0 1px 0 rgb(0 0 0 / 0.5);
  }
  .sub {
    margin-left: -0.5rem;
  }
  .drawers {
    display: flex;
    gap: 0.5rem;
    margin-left: 1rem;
  }
  .app-controls {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-left: auto;
  }
  .badge {
    padding: 0.1rem 0.45rem;
    border: 1px dashed var(--line-strong);
    border-radius: 3px;
  }
  @media (max-width: 760px) {
    .bar {
      flex-wrap: wrap;
      gap: 0.5rem;
    }
    .sub,
    .badge {
      display: none;
    }
    .drawers {
      order: 3;
      flex-basis: 100%;
      margin-left: 0;
      flex-wrap: wrap;
    }
  }
</style>
