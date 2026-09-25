<!--
  The layout shell: the app bar, the stage (lead-sheet band, Launchkey mirror, keyboard
  strip) and the panels that open around it. Each lives in its own folder under src/panels/.

  ┌ app bar: Parts & OTS · Mixer · Browse · Charts · Settings ······· ? · theme ┐
  │ ┌ stage ──────────────────────────────────────────────────────────────┐   │
  │ │ lead-sheet band (panels/leadsheet): now · bar cells / chart · next  │   │
  │ │ Launchkey mirror (panels/launchkey)                                 │ ┌ drawer ┐
  │ │ keyboard strip (panels/keystrip)                                    │ │ parts  │
  │ └─────────────────────────────────────────────────────────────────────┘ │ mixer  │
  │ Registration bar (by the keys): bank · 1–10 · Memory · Freeze · seq.    │settings│
  │ status line                                                             │        │
  │ help footer (lib/tooltip): the hovered control's entry · last Launchkey  └────────┘
  └──────────────────────────────────────────────────────────────────────────┘
  Browser: centred modal. Drawers and the browser end above the help footer
  (--help-footer-space), so it always explains what the pointer is on.

  Scaling: the app fills the window exactly (no page scroll). The stage is a size
  container; the stack inside sets its font size `--u` to the largest that fits both its
  width and height, and everything in it is sized in em. The mirror keeps its proportions;
  the lead-sheet band and the keyboard strip take the height left over (up to a limit).
  When the stage is taller than 1.45:1, the mirror switches to its stacked layout (faders
  under the pads), which is narrower and so can grow larger. The help footer has a fixed
  height (taller in help mode), so hovering never moves the stage.
-->
<script lang="ts">
  import { onDestroy } from 'svelte'
  import type { Session } from './lib/api/session'
  import { handleBlur, handleKey, handleKeyUp } from './lib/shortcuts'
  import { app, clock, ui } from './lib/store.svelte'
  import HelpFooter from './lib/tooltip/HelpFooter.svelte'
  import Tooltip from './lib/tooltip/Tooltip.svelte'
  import { tip, tips } from './lib/tooltip/tip.svelte'
  import Browser from './panels/browser/Browser.svelte'
  import Charts from './panels/charts/Charts.svelte'
  import Header from './panels/header/Header.svelte'
  import KeyStrip from './panels/keystrip/KeyStrip.svelte'
  import Launchkey from './panels/launchkey/Launchkey.svelte'
  import LeadSheet from './panels/leadsheet/LeadSheet.svelte'
  import Harmony from './panels/harmony/Harmony.svelte'
  import Mixer from './panels/mixer/Mixer.svelte'
  import Looper from './panels/looper/Looper.svelte'
  import MultiPad from './panels/multipad/MultiPad.svelte'
  import Parts from './panels/parts/Parts.svelte'
  import RegistBar from './panels/registration/RegistBar.svelte'
  import Registration from './panels/registration/Registration.svelte'
  import Settings from './panels/settings/Settings.svelte'
  import SoundLibrary from './panels/sound/SoundLibrary.svelte'
  import SoundBrowser from './panels/sounds/SoundBrowser.svelte'

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

  <main class="stage">
    <div class="stack">
      <div class="lead-slot"><LeadSheet /></div>
      <Launchkey />
      <div class="strip-slot"><KeyStrip /></div>
    </div>
  </main>

  <!-- Under the keys, as the Genos's Registration Memory buttons sit by the keyboard. -->
  <RegistBar />

  <!-- svelte-ignore a11y_no_noninteractive_tabindex (focusable so its tooltip is reachable from the keyboard) -->
  <footer class="status engraved" role="status" tabindex="0" use:tip={'display.status'}>
    <span class:error={message?.error}>{message?.text ?? ''}</span>
    <span>{unmapped}</span>
  </footer>

  <HelpFooter />
</div>

{#if ui.parts}<Parts />{/if}
{#if ui.mixer}<Mixer />{/if}
{#if ui.looper}<Looper />{/if}
{#if ui.multipad}<MultiPad />{/if}
{#if ui.sound}<SoundLibrary />{/if}
{#if ui.settings}<Settings />{/if}
{#if ui.charts}<Charts />{/if}
{#if ui.harmony}<Harmony />{/if}
{#if ui.regist}<Registration />{/if}
{#if ui.browser}<Browser />{/if}
{#if ui.soundPick !== null}<SoundBrowser pick={ui.soundPick} />{:else if ui.soundBrowser !== null}<SoundBrowser part={ui.soundBrowser} />{/if}
{#if tips.floating}<Tooltip />{/if}

<style>
  .app {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    height: 100vh;
    height: 100dvh;
    padding: 0.5rem 16px 0.6rem;
    overflow: hidden;
  }

  /* ── The stage: sizes in em of --u, the largest that fits both ways ──────────────────
     --w: the stack's width in em (the mirror's design width).
     --h: its least height in em: lead band min + mirror + strip min + 2 gaps.
     Measured from the rendered mirror: 26.00em tall wide, 46.17em stacked. */
  .stage {
    container: stage / size;
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .stack {
    --w: 96;
    --h: 39.9;
    --u: min(100cqw / var(--w), 100cqh / var(--h));
    font-size: var(--u);
    width: calc(var(--w) * 1em);
    height: 100%;
    max-height: calc((var(--h) + 11.5) * 1em);
    display: flex;
    flex-direction: column;
    gap: 0.7em;
  }
  .lead-slot {
    flex: 1 1 0;
    min-height: 5.2em;
    max-height: 10em;
  }
  .strip-slot {
    flex: 1.3 1 0;
    min-height: 7.2em;
    max-height: 14em;
  }
  @container stage (aspect-ratio < 1.45) {
    .stack {
      --w: 66;
      --h: 60;
    }
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
