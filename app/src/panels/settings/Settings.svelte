<!--
  The Settings drawer (right side, not modal: performance keys keep working). Pages
  follow the Genos menus: Chord and Split (Split & Fingering), Transpose, Style (Style
  Setting), then yahaha's own Audio, MIDI and Library. Every change applies at once:
  there is no Save. Settings the engine has go through `app.send`; the few it doesn't
  yet (SoundFont, MIDI inputs, palette LEDs, library folders) go through the settings
  adapter in lib/api/settings.svelte.ts.

  All pages stay mounted (inactive ones `hidden`), so switching is instant and the
  tooltip coverage test sees every control.
-->
<script lang="ts">
  import { ui } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import Overlay from '../../lib/ui/Overlay.svelte'
  import AudioPage from './AudioPage.svelte'
  import ChordPage from './ChordPage.svelte'
  import LibraryPage from './LibraryPage.svelte'
  import MidiPage from './MidiPage.svelte'
  import { nav, TABS, type SettingsTab } from './nav.svelte'
  import SplitPage from './SplitPage.svelte'
  import StylePage from './StylePage.svelte'
  import TransposePage from './TransposePage.svelte'

  const current = $derived(TABS.find((t) => t.id === nav.tab) ?? TABS[0])
  let strip: HTMLDivElement | undefined = $state()

  /** ←/→ (and Home/End) move between tabs, as in any tab list. */
  function tabKey(e: KeyboardEvent) {
    const i = TABS.findIndex((t) => t.id === nav.tab)
    const to: Record<string, number> = { ArrowRight: i + 1, ArrowLeft: i - 1, Home: 0, End: TABS.length - 1 }
    if (!(e.key in to)) return
    e.preventDefault()
    e.stopPropagation()
    const next = TABS[(to[e.key] + TABS.length) % TABS.length]
    nav.tab = next.id
    strip?.querySelector<HTMLElement>(`#settings-tab-${next.id}`)?.focus()
  }
  const select = (id: SettingsTab) => (nav.tab = id)
</script>

<Overlay id="settings" title="Settings" closeTip="settings.close" onclose={() => (ui.settings = false)}>
  <div class="settings">
    <div class="tabs mat-well" role="tablist" aria-label="Settings pages" tabindex="-1" bind:this={strip} onkeydown={tabKey}>
      {#each TABS as t (t.id)}
        <button
          type="button"
          role="tab"
          id="settings-tab-{t.id}"
          class="tab"
          class:on={nav.tab === t.id}
          class:mat-raised={nav.tab === t.id}
          aria-selected={nav.tab === t.id}
          aria-controls="settings-page-{t.id}"
          tabindex={nav.tab === t.id ? 0 : -1}
          use:tip={t.tip}
          onclick={() => select(t.id)}
        >
          {t.label}
        </button>
      {/each}
    </div>

    <p class="crumb engraved" aria-hidden="true">
      {current.genos ? `Genos: Menu › ${current.genos}` : 'yahaha'} · changes apply at once
    </p>

    {#snippet page(id: SettingsTab, Body: typeof ChordPage)}
      <div class="page" id="settings-page-{id}" role="tabpanel" aria-labelledby="settings-tab-{id}" hidden={nav.tab !== id}>
        <Body />
      </div>
    {/snippet}
    {@render page('chord', ChordPage)}
    {@render page('split', SplitPage)}
    {@render page('transpose', TransposePage)}
    {@render page('style', StylePage)}
    {@render page('audio', AudioPage)}
    {@render page('midi', MidiPage)}
    {@render page('library', LibraryPage)}
  </div>
</Overlay>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .tabs {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    gap: 2px;
    padding: 3px;
    border-radius: 6px;
    outline: none;
  }
  .tab {
    min-height: 2.2rem;
    padding: 0 0.1rem;
    border: 1px solid transparent;
    border-radius: 4px;
    background: transparent;
    color: var(--screen-dim);
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.9rem;
    letter-spacing: 0.02em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tab:not(.on):hover {
    color: var(--screen-ink);
    background: rgb(255 255 255 / 0.04);
  }
  /* The well is dark in both themes: unselected text is screen-coloured, the selected
     tab is a raised face. */
  .tab.on {
    background: linear-gradient(180deg, var(--raised-hi), var(--raised-lo));
    color: var(--ink);
    box-shadow:
      inset 0 1px 0 rgb(255 255 255 / 0.18),
      inset 0 -2px 0 var(--accent),
      0 1px 0 rgb(0 0 0 / 0.45);
  }
  .crumb {
    margin: 0 0 0.2rem;
    text-transform: uppercase;
  }
  .page {
    display: grid;
    gap: 1.15rem;
  }
  .page[hidden] {
    display: none;
  }
</style>
