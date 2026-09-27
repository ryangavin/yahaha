<!--
  The Sound Browser (#117): one list of every sound for a keyboard part, whatever it
  comes from: SoundFont presets, instrument plugins, saved sounds (a modal over the
  mirror, open while `ui.soundBrowser` is a part). With `pick` it picks for something else
  instead, a program map rule: Enter or a click hands the sound over and closes.

  ┌ All sounds   ┐┌ Filter ─────────────────────────── 1,219 of 1,219 ┐
  │ ★ Favourites ││ ☆ Grand Piano      SF  GeneralUser-GS.sf2      ▶ │  virtualised
  │ ↺ Recent     ││ ☆ Serum            AU  Xfer Records  ⚠         ▶ │
  │ CATEGORIES   ││ …                                                 │
  └──────────────┘└ Right 1 plays: Grand Piano ──────── Save as sound ┘

  Two tabs (docs/sound-browser.md, D2): Sounds (this list) and Instruments
  (Instruments.svelte: each font and plugin with its presets, and the plugin housekeeping,
  category, Edit…, in process and Rescan, that used to sit in this footer).

  Keys (the filter keeps focus): ↑/↓ PgUp/PgDn Home/End move; Enter plays the sound on
  the part (the browser stays open, as the Genos Voice Selection does); Shift+Enter
  auditions (the band stopped); Ctrl/⌘+D stars; Esc closes; → / ← open and close a
  plugin's presets.

  AU presets: a plugin row's ▸ lists its presets under it (factory presets and the
  `.aupreset` files in ~/Library/Audio/Presets, as Logic shows them). A preset is a sound
  of its own: each part that picks one gets an instance of the plugin with that preset.
  "Save as preset…" keeps what the part's plugin plays now (a Kontakt instrument loaded
  in its editor, say) as an `.aupreset`, filed under a category.
-->
<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte'
  import { SvelteSet } from 'svelte/reactivity'
  import { app, ui, type SoundPick } from '../../lib/store.svelte'
  import { tip, tips } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import Overlay from '../../lib/ui/Overlay.svelte'
  import { CATEGORY_LABELS } from '../../lib/api/sound-library'
  import type { PatchCategory } from '../../lib/api/types'
  import { moveCursor } from '../browser/model'
  import Instruments from './Instruments.svelte'
  import { browserNav, type BrowserTab } from './nav.svelte'
  import { SOURCE_BADGE, categoryCounts, expandable, playingId, visibleSounds, type SoundView } from './model'

  let { part = 0, pick = null }: { part?: number; pick?: SoundPick | null } = $props()

  const ROW = 36
  const OVERSCAN = 8

  const catalog = $derived(app.sounds)
  const entries = $derived(catalog.entries)
  let view = $state<SoundView>({ kind: 'all' })
  let query = $state('')
  // Plugins whose presets are open (All sounds, unfiltered).
  const expanded = new SvelteSet<string>()
  const rows = $derived(visibleSounds(catalog, view, query, expanded))
  const listing = $derived(new Set(app.state.sounds?.listingPresets ?? []))
  function toggle(id: string, open?: boolean) {
    const e = entries.find((x) => x.id === id)
    const on = open ?? !expanded.has(id)
    if (on === expanded.has(id)) return
    if (on) {
      expanded.add(id)
      // Factory presets need a plugin instance: listed once, then cached.
      if (e?.plugin && e.plugin.presets == null) app.send({ type: 'listPluginPresets', id })
    } else expanded.delete(id)
  }
  const cats = $derived(categoryCounts(entries))
  const favourites = $derived(entries.reduce((n, e) => n + (e.favourite ? 1 : 0), 0))
  const saved = $derived(entries.reduce((n, e) => n + (e.source === 'saved' ? 1 : 0), 0))
  // Save as… (#117, O3): what the part plays, with its volume and octave, as a new sound
  // the part then plays. It shows under Saved, highlighted, once the catalog has it.
  let justSaved = $state(false)
  function saveAsSound() {
    if (!kp) return
    app.send({ type: 'saveSoundAs', part, name: null })
    justSaved = true
  }
  // Save (O3): over the part's own sound (a factory preset or file is saved as a new one).
  function saveSound() {
    if (kp) app.send({ type: 'saveSound', part })
  }
  $effect(() => {
    const id = app.state.soundLibrary.lastAdded
    if (!justSaved || !id || !entries.some((e) => e.id === `saved:${id}`)) return
    justSaved = false
    view = { kind: 'saved' }
    cursorId = `saved:${id}`
    void tick().then(() => ensureVisible(cursor, true))
  })

  const kp = $derived(pick ? undefined : app.state.keyboardParts[part])
  const playing = $derived(pick ? (pick.value ? `saved:${pick.value}` : null) : kp ? playingId(kp, app.state.io.soundFontFile) : null)
  const auditioning = $derived(app.state.sounds?.auditioning ?? null)
  const running = $derived(app.state.transport.running)
  // Instruments (O2) plays on a part: picking for a map rule shows Sounds only.
  const tab = $derived(pick ? 'sounds' : browserNav.tab)
  function showTab(t: BrowserTab) {
    browserNav.tab = t
    if (t === 'sounds') void tick().then(() => (input?.focus(), ensureVisible(cursor, true)))
  }

  // Save as preset (AU presets): the part's plugin as it plays now, as an .aupreset.
  let presetForm = $state<{ name: string; category: PatchCategory; replace: boolean } | null>(null)
  let presetName: HTMLInputElement | undefined = $state()
  function openPresetForm() {
    if (!kp?.plugin) return
    presetForm = { replace: false, name: kp.plugin.preset ?? '', category: entries.find((e) => e.id === `au:${kp.plugin?.id}`)?.category ?? 'synthLead' }
    void tick().then(() => presetName?.focus())
  }
  // The file name a preset name saves as (as `presets::safe_name`).
  const fileName = (n: string) => n.replace(/[/:\\]/g, '-').trim().replace(/^\.+/, '').trim() || 'Untitled'
  // A user preset of this plugin with that name exists: ask before replacing it, as Logic
  // does (the file is shared with Logic and MainStage).
  const clash = $derived.by(() => {
    if (!presetForm || !kp?.plugin) return false
    const parent = `au:${kp.plugin.id}`
    const n = fileName(presetForm.name).toLowerCase()
    return entries.some((e) => e.parent === parent && e.id.startsWith(`${parent}#u:`) && e.name.toLowerCase() === n)
  })
  function savePreset(overwrite = false) {
    if (!presetForm || !presetForm.name.trim()) return
    if (clash && !overwrite) {
      presetForm.replace = true
      return
    }
    app.send({ type: 'savePartAsPluginPreset', part, name: presetForm.name.trim(), category: presetForm.category, overwrite })
    const id = kp?.plugin ? `au:${kp.plugin.id}` : null
    if (id && view.kind === 'all' && !query) toggle(id, true)
    presetForm = null
    input?.focus()
  }

  let cursorId = $state<string | null>(null)
  const cursor = $derived(Math.max(0, cursorId === null ? rows.findIndex((i) => entries[i].id === playing) : rows.findIndex((i) => entries[i].id === cursorId)))

  let list: HTMLDivElement | undefined = $state()
  let input: HTMLInputElement | undefined = $state()
  let scrollTop = $state(0)
  let height = $state(480)
  const first = $derived(Math.max(0, Math.floor(scrollTop / ROW) - OVERSCAN))
  const last = $derived(Math.min(rows.length, Math.ceil((scrollTop + height) / ROW) + OVERSCAN))
  const slice = $derived(rows.slice(first, last))

  function ensureVisible(k: number, center = false) {
    if (!list || k < 0) return
    const h = list.clientHeight || height
    const top = k * ROW
    if (center) list.scrollTop = Math.max(0, top - h / 2 + ROW / 2)
    else if (top < list.scrollTop) list.scrollTop = top
    else if (top + ROW > list.scrollTop + h) list.scrollTop = top + ROW - h
    scrollTop = list.scrollTop
  }

  function assign(i: number) {
    const e = entries[i]
    if (!e) return
    if (!pick) return app.send({ type: 'assignSound', part, id: e.id })
    pick.onpick(e.id)
    ui.soundPick = null
  }
  const close = () => (pick ? (ui.soundPick = null) : (ui.soundBrowser = null))
  function audition(i: number) {
    const e = entries[i]
    if (!e || running) return
    app.send(auditioning === e.id ? { type: 'stopSoundAudition' } : { type: 'auditionSound', id: e.id })
  }
  const star = (i: number) => entries[i] && app.send({ type: 'setSoundFavourite', id: entries[i].id, on: !entries[i].favourite })

  function show(v: SoundView) {
    view = v
    void tick().then(() => ensureVisible(cursor, true))
  }

  function onkey(e: KeyboardEvent) {
    const m = moveCursor(e.key, cursor, rows.length, Math.max(1, Math.floor(height / ROW) - 1))
    if (m !== null) {
      e.preventDefault()
      cursorId = entries[rows[m]].id
      ensureVisible(m)
      return
    }
    const i = rows[cursor]
    const cur = i === undefined ? undefined : entries[i]
    if ((e.key === 'ArrowRight' || e.key === 'ArrowLeft') && cur && !query && view.kind === 'all') {
      if (cur.parent && e.key === 'ArrowLeft') {
        e.preventDefault()
        cursorId = cur.parent
        toggle(cur.parent, false)
        return
      }
      if (expandable(cur)) {
        e.preventDefault()
        toggle(cur.id, e.key === 'ArrowRight')
        return
      }
    }
    if (e.key === 'Enter') {
      e.preventDefault()
      if (i !== undefined) (e.shiftKey ? audition : assign)(i)
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'd') {
      e.preventDefault()
      if (i !== undefined) star(i)
    }
  }

  $effect(() => {
    const el = list
    if (!el) return
    const measure = () => (height = el.clientHeight || height)
    measure()
    if (typeof ResizeObserver === 'undefined') return
    const ro = new ResizeObserver(measure)
    ro.observe(el)
    return () => ro.disconnect()
  })

  onMount(() => {
    void tick().then(() => {
      input?.focus()
      ensureVisible(cursor, true)
    })
  })
  // Closing the browser ends an audition.
  onDestroy(() => {
    if (app.state.sounds?.auditioning) app.send({ type: 'stopSoundAudition' })
  })

  const is = (v: SoundView) => v.kind === view.kind && (v.kind !== 'category' || (view.kind === 'category' && view.id === v.id))
</script>

<Overlay id="sounds" title="Sounds · {pick ? pick.title : (kp?.name ?? '')}" side="center" modal closeTip="sounds.close" onclose={close}>
  <div class="wrap">
  {#if !pick}
    <div class="tabs" role="tablist" aria-label="Sound Browser">
      <button type="button" role="tab" class="tab" class:on={tab === 'sounds'} aria-selected={tab === 'sounds'} use:tip={'sounds.tab_sounds'} onclick={() => showTab('sounds')}>Sounds</button>
      <button type="button" role="tab" class="tab" class:on={tab === 'instruments'} aria-selected={tab === 'instruments'} use:tip={'sounds.tab_instruments'} onclick={() => showTab('instruments')}>Instruments</button>
    </div>
  {/if}
  {#if tab === 'instruments'}
    <Instruments {part} />
  {:else}
  <div class="sb">
    <nav class="side" aria-label="Sound categories">
      <button type="button" class="cat" class:on={is({ kind: 'all' })} aria-pressed={is({ kind: 'all' })} use:tip={'sounds.all'} onclick={() => show({ kind: 'all' })}>
        <span>All sounds</span><span class="n">{entries.length.toLocaleString()}</span>
      </button>
      <button type="button" class="cat" class:on={is({ kind: 'favourites' })} aria-pressed={is({ kind: 'favourites' })} use:tip={'sounds.favourites'} onclick={() => show({ kind: 'favourites' })}>
        <span><span class="ico" aria-hidden="true">★</span> Favourites</span><span class="n">{favourites}</span>
      </button>
      <button type="button" class="cat" class:on={is({ kind: 'recents' })} aria-pressed={is({ kind: 'recents' })} use:tip={'sounds.recents'} onclick={() => show({ kind: 'recents' })}>
        <span><span class="ico" aria-hidden="true">↺</span> Recent</span><span class="n">{catalog.recents.length}</span>
      </button>
      <button type="button" class="cat" class:on={is({ kind: 'saved' })} aria-pressed={is({ kind: 'saved' })} use:tip={'sounds.saved'} onclick={() => show({ kind: 'saved' })}>
        <span><span class="ico" aria-hidden="true">●</span> Saved</span><span class="n">{saved}</span>
      </button>
      <h3 class="engraved">Categories</h3>
      {#each cats as c (c.id)}
        {@const v: SoundView = { kind: 'category', id: c.id }}
        <button type="button" class="cat sub" class:on={is(v)} aria-pressed={is(v)} use:tip={'sounds.category'} onclick={() => show(v)}>
          <span>{c.label}</span><span class="n">{c.count.toLocaleString()}</span>
        </button>
      {/each}
    </nav>

    <section class="main">
      <div class="filter">
        <input
          bind:this={input}
          bind:value={query}
          class="mat-well"
          type="text"
          placeholder="Filter by name, file or maker"
          spellcheck="false"
          autocomplete="off"
          role="combobox"
          aria-expanded="true"
          aria-controls="sounds-list"
          aria-autocomplete="list"
          aria-activedescendant={rows[cursor] !== undefined ? `sound-${rows[cursor]}` : undefined}
          aria-label="Filter sounds"
          use:tip={'sounds.filter'}
          onkeydown={onkey}
          oninput={() => void tick().then(() => ensureVisible(cursor, true))}
          onfocus={() => queueMicrotask(() => input && tips.hide(input))}
        />
        <span class="count engraved">{rows.length.toLocaleString()} of {entries.length.toLocaleString()}{#if app.state.sounds?.scanning}&nbsp;· scanning plugins{/if}</span>
      </div>

      <div class="screen mat-screen">
        <div class="scroller" bind:this={list} onscroll={() => (scrollTop = list?.scrollTop ?? 0)}>
          <div class="rows" id="sounds-list" role="listbox" tabindex="-1" aria-label="Sounds" style:height="{rows.length * ROW}px">
            {#each slice as i, k (entries[i].id)}
              {@const e = entries[i]}
              <!-- svelte-ignore a11y_click_events_have_key_events (the filter field drives the list: ↑/↓, Enter) -->
              <div
                class="row"
                class:active={first + k === cursor}
                class:playing={e.id === playing}
                role="option"
                tabindex="-1"
                id="sound-{i}"
                aria-selected={first + k === cursor}
                style:transform="translateY({(first + k) * ROW}px)"
                use:tip={e.plugin?.lastError ? 'sounds.row_failed' : 'sounds.row'}
                onclick={() => ((cursorId = e.id), assign(i))}
              >
                <button type="button" class="star" class:on={e.favourite} tabindex="-1" aria-label={e.favourite ? 'Unstar' : 'Star'} aria-pressed={e.favourite} use:tip={'sounds.favourite'} onclick={(ev) => (ev.stopPropagation(), star(i))}>{e.favourite ? '★' : '☆'}</button>
                <span class="mark" aria-hidden="true">{e.id === playing ? '▶' : ''}</span>
                <span class="name" class:child={!!e.parent && view.kind === 'all' && !query}>
                  {#if expandable(e) && view.kind === 'all' && !query}<button type="button" class="tw" tabindex="-1" aria-expanded={expanded.has(e.id)} aria-label={expanded.has(e.id) ? `Hide ${e.name}'s presets` : `Show ${e.name}'s presets`} use:tip={'sounds.presets'} onclick={(ev) => (ev.stopPropagation(), (cursorId = e.id), toggle(e.id))}>{expanded.has(e.id) ? '▾' : '▸'}</button>{/if}{e.name}{#if expandable(e) && e.plugin?.presets}<span class="np"> · {e.plugin.presets} presets</span>{:else if listing.has(e.id)}<span class="np"> · listing presets…</span>{/if}
                </span>
                <span class="badge {e.source}">{SOURCE_BADGE[e.source]}</span>
                <span class="detail">{e.detail}{#if e.plugin?.lastError}<span class="warn" title={e.plugin.lastError}> ⚠ {e.plugin.lastError}</span>{/if}</span>
                <span class="act">
                  {#if !running}
                    <button type="button" class="pv" class:on={auditioning === e.id} tabindex="-1" aria-label={auditioning === e.id ? 'Stop audition' : 'Audition'} use:tip={auditioning === e.id ? 'sounds.audition_stop' : 'sounds.audition'} onclick={(ev) => (ev.stopPropagation(), audition(i))}>{auditioning === e.id ? '■' : '▶'}</button>
                  {/if}
                </span>
              </div>
            {/each}
          </div>
          {#if rows.length === 0}
            <p class="empty">
              {#if entries.length === 0}No sounds yet: no SoundFonts, plugins or saved sounds.
              {:else if view.kind === 'favourites' && !query}No favourites yet: star a sound with ☆ (or Ctrl+D).
              {:else if view.kind === 'recents' && !query}Nothing picked yet.
              {:else if view.kind === 'saved' && !query}No saved sounds yet: Save as… keeps what a part plays.
              {:else}No sound matches “{query}”.{/if}
            </p>
          {/if}
        </div>
      </div>

      {#if presetForm && kp?.plugin}
        <form class="presetform" onsubmit={(ev) => (ev.preventDefault(), savePreset())}>
          <span class="engraved">{kp.plugin.name} preset</span>
          <input bind:this={presetName} bind:value={presetForm.name} oninput={() => presetForm && (presetForm.replace = false)} class="mat-well" type="text" placeholder="Preset name" aria-label="Preset name" spellcheck="false" use:tip={'sounds.preset_name'} onkeydown={(ev) => ev.key === 'Escape' && (ev.stopPropagation(), (presetForm = null), input?.focus())} />
          <select aria-label="Category of the preset" bind:value={presetForm.category} use:tip={'sounds.preset_category'}>
            {#each Object.entries(CATEGORY_LABELS) as [id, label] (id)}<option value={id}>{label}</option>{/each}
          </select>
          {#if presetForm.replace && clash}
            <span class="ask" role="alert">Replace ‘{fileName(presetForm.name)}’?</span>
            <HwButton tip="sounds.preset_replace" onclick={() => savePreset(true)}>Replace</HwButton>
            <HwButton tip="sounds.preset_replace_cancel" onclick={() => presetForm && ((presetForm.replace = false), presetName?.focus())}>Cancel</HwButton>
          {:else}
            <HwButton tip="sounds.preset_save" onclick={() => savePreset()}>Save</HwButton>
            <HwButton tip="sounds.preset_cancel" onclick={() => ((presetForm = null), input?.focus())}>Cancel</HwButton>
          {/if}
        </form>
      {/if}

      <footer class="foot">
        {#if pick}<span class="now">Pick the sound for <b>{pick.title}</b></span>{:else}<span class="now">{kp?.name} plays <b>{kp?.voiceName}</b></span>{/if}
        {#if auditioning}<HwButton tip="sounds.audition_stop" onclick={() => app.send({ type: 'stopSoundAudition' })}>■ Stop</HwButton>{/if}
        {#if kp}<HwButton tip="sounds.save_over" onclick={saveSound}>Save</HwButton><HwButton tip="sounds.save" onclick={saveAsSound}>Save as…</HwButton>{/if}
        {#if kp?.plugin?.status === 'playing'}<HwButton tip="sounds.save_preset" onclick={openPresetForm}>Save as preset…</HwButton>{/if}
      </footer>
    </section>
  </div>
  {/if}
  </div>
</Overlay>

<style>
  .wrap {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    height: 100%;
    min-height: 0;
  }
  .wrap > :global(*:last-child) {
    flex: 1;
    min-height: 0;
  }
  .tabs {
    display: flex;
    gap: 0.3rem;
  }
  .tab {
    min-height: 2.1rem;
    padding: 0 0.9rem;
    border: 1px solid transparent;
    border-radius: 5px;
    background: none;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.95rem;
    color: var(--ink);
  }
  .tab:hover {
    background: rgb(127 127 127 / 0.1);
  }
  .tab.on {
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
    background: color-mix(in srgb, var(--accent) 16%, transparent);
  }
  .sb {
    display: grid;
    grid-template-columns: 12rem minmax(0, 1fr);
    gap: 1rem;
    height: 100%;
    min-height: 0;
  }
  .side {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-height: 0;
    overflow: auto;
  }
  h3 {
    margin: 0.8rem 0.5rem 0.3rem;
    font-size: 0.72rem;
  }
  .cat {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
    min-height: 2.2rem;
    padding: 0 0.6rem;
    border: 1px solid transparent;
    border-radius: 5px;
    background: none;
    text-align: left;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.95rem;
    color: var(--ink);
  }
  .cat.sub {
    font-family: var(--font-body);
    font-weight: 500;
    font-size: var(--fs-small);
  }
  .cat:hover {
    background: rgb(127 127 127 / 0.1);
  }
  .cat.on {
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
    background: color-mix(in srgb, var(--accent) 16%, transparent);
  }
  .ico {
    color: var(--accent);
  }
  .n {
    color: var(--muted);
    font-size: 0.8rem;
  }
  .main {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    min-width: 0;
    min-height: 0;
  }
  .filter {
    display: flex;
    align-items: center;
    gap: 0.8rem;
  }
  .filter input {
    flex: 1;
    min-width: 0;
    height: 2.4rem;
    padding: 0 0.8rem;
    border: 1px solid var(--well-edge);
    border-radius: 6px;
    color: var(--screen-ink);
    font-size: 1rem;
  }
  .count {
    white-space: nowrap;
  }
  .screen {
    --accent: #f3b843;
    --danger: #ff7a6b;
    flex: 1;
    min-height: 0;
    display: flex;
    border-radius: 8px;
    overflow: hidden;
  }
  .scroller {
    position: relative;
    flex: 1;
    overflow-y: auto;
    overscroll-behavior: contain;
  }
  .rows {
    position: relative;
  }
  .row {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    height: 36px;
    display: grid;
    grid-template-columns: 1.8rem 1rem minmax(8rem, 1.4fr) 3.2rem minmax(0, 1fr) 2.4rem;
    align-items: center;
    column-gap: 0.6rem;
    padding: 0 0.5rem 0 0.25rem;
    color: var(--screen-ink);
    cursor: pointer;
    white-space: nowrap;
    contain: layout paint;
  }
  .row:hover {
    background: rgb(255 255 255 / 0.045);
  }
  .row.active {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .row.playing .name,
  .mark {
    color: var(--accent);
  }
  .name,
  .detail {
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }
  .detail {
    color: var(--screen-dim);
    font-size: var(--fs-small);
  }
  .warn {
    color: var(--danger);
  }
  .name.child {
    padding-left: 1.4rem;
  }
  button.tw {
    border: 0;
    background: none;
    padding: 0 0.35rem 0 0;
    color: var(--screen-dim);
    font-size: 0.85rem;
  }
  .np {
    color: var(--screen-dim);
    font-size: var(--fs-small);
  }
  .presetform {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
  }
  .presetform input {
    flex: 1;
    min-width: 6rem;
    height: 2.2rem;
    padding: 0 0.6rem;
    border: 1px solid var(--well-edge);
    border-radius: 5px;
    color: var(--screen-ink);
  }
  .ask {
    color: var(--accent);
    white-space: nowrap;
  }
  .presetform select {
    min-height: 2.2rem;
    padding: 0 0.45rem;
    border: 1px solid var(--well-edge);
    border-radius: 4px;
    background: var(--screen-bg);
    color: var(--screen-ink);
  }
  .badge {
    justify-self: start;
    padding: 1px 4px;
    border: 1px solid rgb(223 244 255 / 0.25);
    border-radius: 3px;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.72rem;
    letter-spacing: 0.06em;
  }
  .badge.plugin {
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
  }
  .badge.saved {
    color: var(--accent);
  }
  button.star,
  button.pv {
    border: 0;
    background: none;
    padding: 0;
    height: 36px;
    color: var(--screen-dim);
  }
  .star.on,
  .pv.on {
    color: var(--accent);
  }
  .act {
    display: flex;
    justify-content: flex-end;
  }
  .empty {
    position: absolute;
    inset: 1.5rem 1rem auto;
    margin: 0;
    text-align: center;
    color: var(--screen-dim);
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    min-height: 2.6rem;
  }
  .now {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-small);
    color: var(--muted);
  }
  .now b {
    color: var(--ink);
  }
</style>
