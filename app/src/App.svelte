<!--
  The layout shell: the app bar, the stage (display, hand surface, mixer row, Quick Racks,
  keyboard) and the panels that open around it. Each lives in its own folder under
  src/panels/.

  ┌ app bar: yahaha [transport: Start · Sync · Intro · Ending · Tempo · 13.1] ⚙ ? ☾ ┐
  │ quick nav (lib/nav.ts)                                                   │
  │ ┌ stage, five rows, each the full width ──────────────────────────────┐   │
  │ │ display (panels/leadsheet, or panels/channel's Channel view):       │   │
  │ │   now · bar cells / chart · next; takes the height the rows below   │ ┌ drawer ┐
  │ │   leave (about half the window on a tall one)                       │ │ rack   │
  │ ├ hand surface ───────────────────────────────────────────────────────┤ │effects │
  │ │ the Launchkey mirror (panels/launchkey), one flat row: 8 knobs over │ │settings│
  │ │   the 16 pads, with the side buttons, as under your hands           │ │  …     │
  │ ├ mixer row ──────────────────────────────────────────────────────────┤ │        │
  │ │ mixer row (panels/mixer/MixerRow): its bar · 12 strips · master     │ │        │
  │ │   (with ui.mixer its details show too, in the display's place)      │ │        │
  │ ├ Quick Racks ────────────────────────────────────────────────────────┤ │        │
  │ │ Quick Racks row (panels/knobracks): ◀ Bank A ▶ · racks 1–8 · Store  │ │        │
  │ ├─────────────────────────────────────────────────────────────────────┤ │        │
  │ │ keyboard strip (panels/keystrip): chord tones · the keys            │ │        │
  │ └─────────────────────────────────────────────────────────────────────┘ │        │
  │ status line                                                             │        │
  │ help footer (lib/tooltip): the hovered control's entry · last Launchkey  └────────┘
  └──────────────────────────────────────────────────────────────────────────┘
  Each drawer opens from a small button on the stage by what it details (lib/ui/DrawerButton):
  Rack and Library on the mixer bar, beside the loaded rack's name (its Mixer button shows
  the mixer row's details, as Alt+M does); Multi Pads by the pad-page tabs; Charts by the
  lead-sheet lane; Harmony/Arp and Chord Looper on the keyboard strip's cheek; Effects from
  the quick nav and the mixer row; Styles in the quick nav opens the browser.
  Library (panels/library, `ui.view`): a page in place of the whole stage, from the
  header's Stage | Library switch (Alt+B); drawers open over it too.
  Browser: centred modal. Drawers and the browser end above the help footer
  (--help-footer-space), so it always explains what the pointer is on.

  Scaling, CSS only: the app fills the window exactly (no page scroll). The stage is a
  size container (`stage`); the stack inside sets its font size to --u, the smaller of
  its width / 96 and its height / 64 (7–15px; height / 72 on a stage under 560px), and
  the hand surface's slot, the Quick Racks row and the keyboard strip are sized in em of
  it. The mixer row has a set height that MixerRow fills (236–280px; with its details
  shown it takes the display's place instead); the display takes what's left, at least
  110px, and on a short window the hand surface gives way first, down to 80px. Under
  760px of window height the app's gaps close up, so 1024×700 fits every row whole. The hand surface's slot is its own size container
  (`hand`): the mirror sets its --u to the largest that fits the slot at the surface's
  proportions (76em × 18em, centred). The help footer has a fixed height (taller in help
  mode), so hovering never moves the stage.
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
        <!-- The hand surface: the Launchkey mirror, knobs over pads, in one flat row. -->
        <div class="hand-slot" class:details={ui.mixer} use:tip={'stage.hand_surface'}><div class="mirror"><Launchkey /></div></div>
        <!-- The mixer row, always shown. A strip's name opens or closes its Channel view
             (seen before the strip handles it). -->
        <div class="mixer-slot" class:details={ui.mixer} onclickcapture={(e) => channelNav.stripClick(e)}><MixerRow /></div>
        <!-- The Quick Racks row. -->
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
     The stack's font size --u: the hand surface's slot (13em, at least 7em), the Quick
     Racks row (panels/knobracks) and the keyboard strip (7.6em) are sized in it. Width / 96
     keeps the rack columns in their old proportions; height / 64 keeps those rows to about
     half the stage with the mixer row, so the display gets the rest. */
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
  /* A short stage (under 560px, so a window under about 760px tall): the stack's --u
     follows height / 72, so the Quick Racks row and the keyboard strip keep their
     proportions a little smaller, and the gaps close up. With the app's own tighter
     spacing below, 1024×700 fits the display's 110px, the hand surface's 80px and a whole
     mixer row. */
  @container stage (height < 560px) {
    .stack {
      --u: clamp(7px, min(100cqw / 96, 100cqh / 72), 15px);
      gap: 0.4em;
    }
  }
  @media (max-height: 760px) {
    .app {
      gap: 0.3rem;
      padding-top: 0.3rem;
      padding-bottom: 0.3rem;
    }
  }
  /* The display takes what the other rows leave, and keeps at least 110px of it (its
     status line plus the Now/Next row with the bar cells): on a short window the hand
     surface gives up its height for it. */
  .display-slot {
    flex: 1 1 0;
    min-height: 6.875rem;
  }
  .display-slot.hidden {
    display: none;
  }
  /* The hand surface: a full-width slot, 13em of the stack's --u tall (195px on a big
     window); on a short one it is the row that shrinks, down to 80px, the least at which
     the mirror's print shows. It is its own size container, so the mirror sizes itself in
     em of it. */
  .hand-slot {
    container: hand / size;
    flex: 0 1 13em;
    min-height: 5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }
  /* With the mixer's details shown, the hand surface keeps only its least height and the
     mixer row takes the rest. */
  .hand-slot.details {
    flex-basis: max(5rem, 7em);
  }
  /* The mirror's own --u: the largest that fits the slot at the surface's proportions,
     105em × 10em: one flat row with the 8 knobs beside the 16 pads, Shift, Pad Bank, Track
     and Rotary on the left and Scene/Function, Stop/Play on the right (no fader bank, no
     status display). If a row is wider than 105:10 the mirror is limited by height and
     centred. */
  .mirror {
    --w: 105;
    --h: 10;
    --u: min(100cqw / var(--w), 100cqh / var(--h));
    font-size: var(--u);
    width: calc(var(--w) * 1em);
    flex: none;
  }
  /* The mixer row, in rem (not --u). A set height that MixerRow fills (it lays its strips
     out in whatever height it gets): at least 235px, a whole strip (200px) plus the bar's
     one line and the gap, growing to 280px on a tall window. With the
     details shown (ui.mixer) the display steps aside and the row takes its place (the bar
     and each strip's details sit straight above the strips); on a short window the strips
     scroll inside the row. */
  .mixer-slot {
    flex: none;
    height: clamp(14.75rem, 38cqh, 17.5rem);
    min-height: 0;
    display: flex;
    flex-direction: column;
    font-size: 1rem;
  }
  .mixer-slot.details {
    flex: 1 1 0;
    height: auto;
    min-height: 16.75rem;
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
