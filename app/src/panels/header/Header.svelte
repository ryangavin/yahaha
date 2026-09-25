<!--
  The app bar above the hardware view, one row: the brand, the transport section
  (TransportBar.svelte: Start/Stop, Sync, Intro, Ending, Tempo, Tap, bar.beat and the
  section) and the app's own controls (Settings, help mode, theme).

  [yahaha] [▶ Start][Sync Start][Sync Stop] Intro[I][II][III] Ending[I][II][III]
           Tempo[−]104[+][Tap] [13.1 ●○○○ Main B → Fill B]      [Settings] ? ☾

  Everything else the hardware does lives on the Launchkey mirror, and each drawer opens from a
  small button next to the controls it details (Mixer by the faders, Multi Pads by the
  pads, Charts by the lead-sheet lane, …: lib/ui/DrawerButton.svelte), not from here.

  REFERENCE for a small component: read `app.state` with `$derived`, act with
  `app.send(...)` or the `ui` store, and give every control a catalog `tip`.
-->
<script lang="ts">
  import { app, ui } from '../../lib/store.svelte'
  import { tips } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import TransportBar from './TransportBar.svelte'

  const s = $derived(app.state)
  const amber = { rgb: [127, 90, 20] as [number, number, number], level: 'bright' as const, anim: 'solid' as const }
</script>

<header class="bar">
  <span class="brand" aria-label="yahaha">yahaha</span>
  <span class="tag">
    <span class="engraved sub">software arranger</span>
    {#if app.kind === 'mock' || s.io.offline}<span class="engraved badge"><span class="long">{app.kind === 'mock' ? 'mock session' : 'offline session'}</span><span class="short">{app.kind === 'mock' ? 'mock' : 'offline'}</span></span>{/if}
  </span>

  <TransportBar />

  <div class="app-controls">
    <HwButton tip="settings.open" led={ui.settings ? amber : null} label="Settings" onclick={() => ui.toggleDrawer('settings')}>
      <span class="long">Settings</span><span class="short icon">⚙</span>
    </HwButton>
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
  /* Under the name's tagline: which session this is, when it isn't the engine's. */
  .tag {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.15rem;
    margin-left: -0.5rem;
    flex: none;
  }
  .sub {
    line-height: 1.1;
    white-space: nowrap;
  }
  .app-controls {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    /* The transport between takes the free space as its margins (centred). */
    margin-left: auto;
    flex: none;
  }
  .badge {
    padding: 0 0.35rem;
    line-height: 1.3;
    border: 1px dashed var(--line-strong);
    border-radius: 3px;
    white-space: nowrap;
  }
  .short {
    display: none;
  }
  .icon {
    font-size: 1.1em;
    line-height: 1;
  }
  /* Up to a laptop: the transport takes the room; the app's own controls go short. */
  @media (max-width: 1360px) {
    .bar {
      gap: 0.6rem;
    }
    .sub {
      display: none;
    }
    .tag {
      margin-left: 0;
    }
    .app-controls {
      gap: 0.3rem;
    }
    .long {
      display: none;
    }
    .short {
      display: inline;
    }
  }
  @media (max-width: 1080px) {
    .bar {
      gap: 0.4rem;
    }
    .brand {
      font-size: 1.25rem;
      letter-spacing: 0.02em;
    }
  }
</style>
