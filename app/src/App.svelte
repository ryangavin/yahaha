<!--
  The layout shell: the app bar, the stage (lead-sheet band, Launchkey mirror, keyboard
  strip) and the panels that open around it. Each lives in its own folder under src/panels/.

  ┌ app bar: yahaha [transport: Start · Sync · Intro · Ending · Tempo · 13.1] ⚙ ? ☾ ┐
  │ ┌ stage ──────────────────────────────────────────────────────────────┐   │
  │ │ lead-sheet band (panels/leadsheet): now · bar cells / chart · next  │   │
  │ │ Launchkey mirror (panels/launchkey)                                 │ ┌ drawer ┐
  │ │ keyboard strip (panels/keystrip), one panel:                        │ │ rack   │
  │ │   Quick Racks bar: bank ◀ A ▶ · 1–8 · Store                         │ │ mixer  │
  │ │   chord tones · the keys                                            │ │settings│
  │ └─────────────────────────────────────────────────────────────────────┘ │        │
  │ status line                                                             │        │
  │ help footer (lib/tooltip): the hovered control's entry · last Launchkey  └────────┘
  └──────────────────────────────────────────────────────────────────────────┘
  Each drawer opens from a small button on the stage by what it details (lib/ui/DrawerButton):
  Rack, Library, Mixer on the fader head; Multi Pads by the pad-page tabs; Charts by
  the lead-sheet lane; Harmony/Arp and Chord Looper on the keyboard strip's cheek; the style
  name on the display opens the browser (as touching it does on the Genos).
  Library (panels/library, `ui.view`): a page in place of the stage, from the header's
  Stage | Library switch (Alt+B); drawers open over it too.
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
  import DropoutNotice from './lib/DropoutNotice.svelte'
  import { handleBlur, handleKey, handleKeyUp } from './lib/shortcuts'
  import { NAV } from './lib/nav'
  import { app, clock, ui } from './lib/store.svelte'
  import DrawerButton from './lib/ui/DrawerButton.svelte'
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
  import RackPanel from './panels/rack/RackPanel.svelte'
  import QuickBar from './panels/quickracks/QuickBar.svelte'
  import Library from './panels/library/Library.svelte'
  import Settings from './panels/settings/Settings.svelte'
  import SoundPicker from './panels/sounds/SoundPicker.svelte'

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
  <!-- Quick nav (lib/nav.ts): every panel and drawer, one click or Alt+letter away. -->
  <nav class="quick-nav" aria-label="Panels">
    {#each NAV.filter((n) => !n.hidden) as n (n.tip)}
      <DrawerButton tip={n.tip} open={n.open()} onclick={n.toggle}>{n.label}</DrawerButton>
    {/each}
  </nav>

  {#if ui.view === 'library'}
    <!-- Library replaces the stage (docs/racks.md, "Screens"); the band keeps playing. -->
    <main class="library-slot"><Library /></main>
  {:else}
    <main class="stage">
      <div class="stack">
        <div class="lead-slot"><LeadSheet /></div>
        <Launchkey />
        <!-- The Quick Racks bar sits in the keyboard strip's panel, above the keys. -->
        <div class="strip-slot"><KeyStrip><QuickBar /></KeyStrip></div>
      </div>
    </main>
  {/if}

  <!-- svelte-ignore a11y_no_noninteractive_tabindex (focusable so its tooltip is reachable from the keyboard) -->
  <footer class="status engraved" role="status" tabindex="0" use:tip={'display.status'}>
    <span class:error={message?.error}>{message?.text ?? ''}</span>
    <span>{unmapped}</span>
  </footer>

  <HelpFooter />
  <DropoutNotice />
</div>

{#if ui.rack}<RackPanel />{/if}
{#if ui.mixer}<Mixer />{/if}
{#if ui.looper}<Looper />{/if}
{#if ui.multipad}<MultiPad />{/if}
{#if ui.settings}<Settings />{/if}
{#if ui.charts}<Charts />{/if}
{#if ui.harmony}<Harmony />{/if}
{#if ui.browser}<Browser />{/if}
<!-- The Sound Browser only picks for a program map rule now (Style map); Library took over
     choosing a part's sound. -->
{#if ui.soundPick !== null}<SoundPicker pick={ui.soundPick} />{/if}
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
     --fixed: what doesn't scale: the mirror's knob row, sized in rem (80px + its 4px margin).
     --top: the Quick Racks bar's row in the strip, with its gap (one row wide, two stacked).
     Measured from the rendered mirror without its knob row: 26.00em tall wide, 46.17em
     stacked; the bar's row 4.0em wide, 6.7em stacked. Leaving --fixed out lets the stack
     overflow the stage, and the keyboard strip runs under the status line. */
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
    --h: 44.5;
    --fixed: 5.25rem;
    /* The Quick Racks bar's row at the top of the keyboard strip (+ its gap). */
    --top: 4.6em;
    --u: min(100cqw / var(--w), (100cqh - var(--fixed)) / var(--h));
    font-size: var(--u);
    width: calc(var(--w) * 1em);
    height: 100%;
    max-height: calc((var(--h) + 11.5) * 1em + var(--fixed));
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
    min-height: calc(7.2em + var(--top));
    max-height: calc(14em + var(--top));
  }
  @container stage (aspect-ratio < 1.45) {
    .stack {
      --w: 66;
      --h: 67.4;
      --top: 7.4em;
    }
  }
  .library-slot {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .quick-nav {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
    font-size: 13px;
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
