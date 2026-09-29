<!--
  The layout shell: the app bar, the stage (display, mixer line, registration row,
  keyboard) and the panels that open around it. Each lives in its own folder under
  src/panels/.

  ┌ app bar: yahaha [transport: Start · Sync · Intro · Ending · Tempo · 13.1] ⚙ ? ☾ ┐
  │ quick nav (lib/nav.ts)                                                   │
  │ ┌ stage ──────────────────────────────────────────────────────────────┐   │
  │ │ display (panels/leadsheet): now · bar cells / chart · next          │   │
  │ │   takes the height the rows below leave (about half the window)    │ ┌ drawer ┐
  │ ├ mixer line ─────────────────────────────────────────┬──────────────┤ │ rack   │
  │ │ mixer row (panels/mixer/MixerRow): 12 strips ·master│ Launchkey    │ │effects │
  │ │   (its details below the strips while ui.mixer)     │ mirror       │ │settings│
  │ ├─────────────────────────────────────────────────────┴──────────────┤ │  …     │
  │ │ knobs + Quick Racks (panels/knobracks), one panel, 8 shared columns:│ │        │
  │ │   ◀ Knobs ▶ · knobs 1–8 / ◀ Bank A ▶ · Quick Racks 1–8 · Store      │ │        │
  │ │ keyboard strip (panels/keystrip): chord tones · the keys            │ │        │
  │ └─────────────────────────────────────────────────────────────────────┘ │        │
  │ status line                                                             │        │
  │ help footer (lib/tooltip): the hovered control's entry · last Launchkey  └────────┘
  └──────────────────────────────────────────────────────────────────────────┘
  Each drawer opens from a small button on the stage by what it details (lib/ui/DrawerButton):
  Rack and Library on the fader head (its Mixer button shows the mixer row's details, as
  Alt+M does); Multi Pads by the pad-page tabs; Charts by the lead-sheet lane;
  Harmony/Arp and Chord Looper on the keyboard strip's cheek; Effects from the quick nav
  and the mixer row; the style name on the display opens the browser (as touching it
  does on the Genos).
  Library (panels/library, `ui.view`): a page in place of the whole stage, from the
  header's Stage | Library switch (Alt+B); drawers open over it too.
  Browser: centred modal. Drawers and the browser end above the help footer
  (--help-footer-space), so it always explains what the pointer is on.

  Scaling, CSS only: the app fills the window exactly (no page scroll). The stage is a
  size container (`stage`); the stack inside sets its font size to --u, the smaller of
  its width / 96 and its height / 64 (7–15px), and the registration row and the keyboard
  strip are sized in em of it. The mixer line has a set height that MixerRow fills
  (220–280px; 62% of the stage while its details show); the display takes what's left.
  Across the mixer line the mirror gives way first: the row asks for 66rem and the mirror
  for 18rem (growing into any spare width), and the mirror shrinks 50× faster, down to
  13rem; only then do the strips narrow. The mirror's slot is its own size container
  (`mirror`): the mirror sets its --u to the largest that fits the slot, in its wide
  layout (96em × 26.2em) or, when the slot is narrower than 2.09:1, its stacked one
  (66em × 46.4em, faders under the pads). The help footer has a fixed height (taller in
  help mode), so hovering never moves the stage.
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
  import MixerRow from './panels/mixer/MixerRow.svelte'
  import Effects from './panels/effects/Effects.svelte'
  import ChannelView from './panels/channel/ChannelView.svelte'
  import { channelNav } from './panels/channel/nav.svelte'
  import Looper from './panels/looper/Looper.svelte'
  import MultiPad from './panels/multipad/MultiPad.svelte'
  import RackPanel from './panels/rack/RackPanel.svelte'
  import KnobRackPanel from './panels/knobracks/KnobRackPanel.svelte'
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

  /** Esc closes what's open over the stage first (handleKey), then the Channel view. */
  function onKey(e: KeyboardEvent) {
    handleKey(e)
    if (e.key === 'Escape' && !e.defaultPrevented && channelNav.escape()) e.preventDefault()
  }
</script>

<svelte:window onkeydown={onKey} onkeyup={handleKeyUp} onblur={handleBlur} />

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
        <!-- The mixer's details take the display's place, straight above the strips. -->
        <!-- A selected strip's Channel view (panels/channel) takes the display until closed. -->
        <div class="display-slot" class:hidden={ui.mixer}>
          {#if channelNav.open}
            <ChannelView part={ui.selectedPart} onpart={(p) => channelNav.show(p)} onclose={() => channelNav.close()} />
          {:else}
            <LeadSheet />
          {/if}
        </div>
        <!-- The mixer row, always shown, with the Launchkey mirror beside it. A strip's
             name opens or closes its Channel view (seen before the strip handles it). -->
        <div class="mixer-line" class:details={ui.mixer}>
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (the strip's own button takes the keys; Enter on it clicks) -->
          <div class="mixer-slot" onclickcapture={(e) => channelNav.stripClick(e)}><MixerRow /></div>
          <div class="mirror-slot"><div class="mirror"><Launchkey /></div></div>
        </div>
        <!-- The knobs over the Quick Racks, knob n above rack n. -->
        <KnobRackPanel />
        <div class="strip-slot"><KeyStrip /></div>
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
{#if ui.effects}<Effects />{/if}
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

  /* ── The stage ───────────────────────────────────────────────────────────────────────
     The stack's font size --u: the registration row (panels/knobracks, 7.37em tall) and
     the keyboard strip (7.6em) are sized in it. Width / 96 keeps the knob columns in their
     old proportions; height / 64 keeps those two rows to about a quarter of the stage, so
     the display gets the rest after the mixer line. */
  .stage {
    container: stage / size;
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .stack {
    --u: clamp(7px, min(100cqw / 96, 100cqh / 64), 15px);
    font-size: var(--u);
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 0.7em;
  }
  .display-slot {
    flex: 1 1 0;
    min-height: 0;
  }
  /* The mixer row and the mirror beside it, in rem (not --u). A set height that MixerRow
     fills (it lays its strips out in whatever height it gets): 268–280px, the least a
     strip needs whole. With the details shown (ui.mixer) the display steps aside and the
     line takes its place (the bar and each strip's details sit straight above the strips);
     on a short window the strips scroll inside the row. The mirror fits into the same
     height. */
  .mixer-line {
    flex: none;
    height: clamp(16.75rem, 38cqh, 17.5rem);
    display: flex;
    gap: 0.6rem;
    font-size: 1rem;
  }
  .mixer-line.details {
    flex: 1 1 0;
    height: auto;
    min-height: 16.75rem;
  }
  .display-slot.hidden {
    display: none;
  }
  /* The mirror gives way first here too: with the details shown the row takes the whole
     line, so its bar fits in fewer lines. */
  .mixer-line.details .mirror-slot {
    display: none;
  }
  .mixer-line.details .mixer-slot {
    flex: 1 1 auto;
  }
  /* The row asks for 12 strips of ~80px plus the master; it only narrows once the mirror
     is at its least width (flex-shrink 1 against the mirror's 50). */
  .mixer-slot {
    flex: 0 1 66rem;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .mirror-slot {
    container: mirror / size;
    flex: 1 50 18rem;
    min-width: 13rem;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  /* The mirror's own --u: the largest that fits the slot at the surface's proportions.
     Measured from the rendered mirror: 26.00em tall wide, 46.17em stacked (66em wide);
     --h has a little headroom so it never spills out of the slot. */
  .mirror {
    --w: 96;
    --h: 26.2;
    --u: min(100cqw / var(--w), 100cqh / var(--h));
    font-size: var(--u);
    width: calc(var(--w) * 1em);
    flex: none;
  }
  @container mirror (aspect-ratio < 2.09) {
    .mirror {
      --w: 66;
      --h: 46.4;
    }
  }
  .strip-slot {
    flex: none;
    height: 7.6em;
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
