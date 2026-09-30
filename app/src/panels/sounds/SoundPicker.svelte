<!--
  Picks a sound for a program map rule (Library › Style map, `ui.soundPick`): a modal list
  of every sound, with the Sounds tab's chips and filter. Enter or a click hands the sound
  over and closes. It is what is left of the old Sound Browser (#117), which Library
  replaced (docs/racks.md, item 13).

  ┌ All sounds   ┐┌ Filter ─────────────────────────────── 142 of 1,219 ┐
  │ ★ Favourites ││ ☆ Grand Piano      SF  GeneralUser-GS · GM 1        │  virtualised
  │ ↺ Recent     ││ ☆ Silk Strings     Mine Sampler Deluxe              │
  │ ● My Sounds  ││ …                                                   │
  │ CATEGORIES   │├ Pick the sound for Piano family ────────────────────┤
  │ INSTRUMENTS  │└─────────────────────────────────────────────────────┘
  └──────────────┘

  Keys (the filter keeps focus): ↑/↓ PgUp/PgDn Home/End move; Enter picks; Ctrl/⌘+D
  stars; Esc closes. Tab reaches every chip.
-->
<script lang="ts">
  import { onMount, tick } from 'svelte'
  import type { GmMapRow } from '../../lib/api/types'
  import { app, ui, type SoundPick } from '../../lib/store.svelte'
  import { tip, tips } from '../../lib/tooltip/tip.svelte'
  import Overlay from '../../lib/ui/Overlay.svelte'
  import { moveCursor } from '../browser/model'
  import { SOURCE_BADGE, allSoundIds, categoryCounts, instruments, mapSlots, patchesById, soundNumber, visibleSounds, type SoundView } from './model'

  let { pick }: { pick: SoundPick } = $props()

  const ROW = 36
  const OVERSCAN = 8
  // One empty GM map, so an engine without one doesn't make a new array per state.
  const NO_ROWS: GmMapRow[] = []

  const catalog = $derived(app.sounds)
  const entries = $derived(catalog.entries)
  // The slices themselves, so a state that leaves them alone re-runs nothing below.
  const patches = $derived(app.state.soundLibrary.patches)
  const gmMap = $derived(app.state.soundLibrary.gmMap ?? NO_ROWS)
  const ctx = $derived.by(() => ({ patches, gmMap }))
  let view = $state<SoundView>({ kind: 'all' })
  let query = $state('')
  const rows = $derived(visibleSounds(catalog, view, query, ctx))
  const listingPresets = $derived(app.state.sounds?.listingPresets)
  const listing = $derived(new Set(listingPresets ?? []))
  const slots = $derived(mapSlots(gmMap))
  const byId = $derived(patchesById(patches))
  const allIds = $derived(allSoundIds(ctx))
  const allCount = $derived(entries.reduce((n, e) => n + (allIds.has(e.id) ? 1 : 0), 0))
  const cats = $derived(categoryCounts(entries.filter((e) => allIds.has(e.id))))
  const insts = $derived(instruments(catalog))
  const favourites = $derived(entries.reduce((n, e) => n + (e.favourite ? 1 : 0), 0))
  const mine = $derived(entries.reduce((n, e) => n + (e.source === 'saved' ? 1 : 0), 0))
  // The library patch the rule names now.
  const playing = $derived(pick.value ? `saved:${pick.value}` : null)

  let cursorId = $state<string | null>(null)
  // No row is active while the rule's sound (or the one moved to) isn't in the list. ↓
  // then starts at the top.
  const cursor = $derived(rows.findIndex((i) => entries[i].id === (cursorId ?? playing)))
  // Typing a filter that hides the row moved to moves to its first match, so Enter picks it.
  function onfilter() {
    if (cursor < 0 && rows.length) cursorId = entries[rows[0]].id
    ensureVisible(cursor, true)
  }

  let list: HTMLDivElement | undefined = $state()
  let input: HTMLInputElement | undefined = $state()
  let scrollTop = $state(0)
  let height = $state(480)
  const first = $derived(Math.max(0, Math.floor(scrollTop / ROW) - OVERSCAN))
  const last = $derived(Math.min(rows.length, Math.ceil((scrollTop + height) / ROW) + OVERSCAN))
  const slice = $derived(rows.slice(first, last))

  function ensureVisible(k: number, center = false) {
    if (!list) return
    const h = list.clientHeight || height
    const top = Math.max(0, k) * ROW
    if (center) list.scrollTop = Math.max(0, top - h / 2 + ROW / 2)
    else if (top < list.scrollTop) list.scrollTop = top
    else if (top + ROW > list.scrollTop + h) list.scrollTop = top + ROW - h
    scrollTop = list.scrollTop
  }

  function choose(i: number) {
    const e = entries[i]
    if (!e) return
    pick.onpick(e.id)
    ui.soundPick = null
  }
  const close = () => (ui.soundPick = null)
  const star = (i: number) => entries[i] && app.send({ type: 'setSoundFavourite', id: entries[i].id, on: !entries[i].favourite })

  function show(v: SoundView) {
    view = v
    // A plugin's factory presets need an instance: the engine lists them once, then
    // answers from its cache.
    if (v.kind === 'instrument' && v.id.startsWith('au:')) {
      const e = entries.find((x) => x.id === v.id)
      // A failed listing is not tried again (until the next scan): it says why instead.
      if (e?.plugin && !e.plugin.lastError && !e.plugin.presetsError && !listing.has(v.id)) app.send({ type: 'listPluginPresets', id: v.id })
    }
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
    if (e.key === 'Enter') {
      e.preventDefault()
      if (i !== undefined) choose(i)
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

  const is = (v: SoundView) => v.kind === view.kind && (v.kind !== 'category' || (view.kind === 'category' && view.id === v.id)) && (v.kind !== 'instrument' || (view.kind === 'instrument' && view.id === v.id))
  const slotText = (id: string) => {
    const s = slots.get(id)
    return s === undefined ? '' : s === 'drums' ? 'GM drums · ' : `GM ${s + 1} · `
  }
  const chipId = $derived(view.kind === 'instrument' ? view.id : null)
  const shownName = $derived(chipId ? (insts.find((x) => x.id === chipId)?.name ?? '') : '')
  const listingChip = $derived(!!chipId && listing.has(chipId))
  const chipError = $derived(chipId ? entries.find((e) => e.id === chipId)?.plugin?.presetsError : undefined)
  // "N of M": M is what the chip holds, before the filter.
  const chipTotal = $derived(query.trim() ? visibleSounds(catalog, view, '', ctx).length : rows.length)
</script>

<Overlay id="sounds" title="Sounds · {pick.title}" side="center" modal closeTip="sounds.close" onclose={close}>
  <div class="sb">
    <nav class="side" aria-label="Sound chips">
      <button type="button" class="cat" class:on={is({ kind: 'all' })} aria-pressed={is({ kind: 'all' })} use:tip={'sounds.all'} onclick={() => show({ kind: 'all' })}>
        <span>All sounds</span><span class="n">{allCount.toLocaleString()}</span>
      </button>
      <button type="button" class="cat" class:on={is({ kind: 'favourites' })} aria-pressed={is({ kind: 'favourites' })} use:tip={'sounds.favourites'} onclick={() => show({ kind: 'favourites' })}>
        <span><span class="ico" aria-hidden="true">★</span> Favourites</span><span class="n">{favourites}</span>
      </button>
      <button type="button" class="cat" class:on={is({ kind: 'recents' })} aria-pressed={is({ kind: 'recents' })} use:tip={'sounds.recents'} onclick={() => show({ kind: 'recents' })}>
        <span><span class="ico" aria-hidden="true">↺</span> Recent</span><span class="n">{catalog.recents.length}</span>
      </button>
      <button type="button" class="cat" class:on={is({ kind: 'mine' })} aria-pressed={is({ kind: 'mine' })} use:tip={'sounds.saved'} onclick={() => show({ kind: 'mine' })}>
        <span><span class="ico" aria-hidden="true">●</span> My Sounds</span><span class="n">{mine}</span>
      </button>
      <h3 class="engraved">Categories</h3>
      {#each cats as c (c.id)}
        {@const v: SoundView = { kind: 'category', id: c.id }}
        <button type="button" class="cat sub" class:on={is(v)} aria-pressed={is(v)} use:tip={'sounds.category'} onclick={() => show(v)}>
          <span>{c.label}</span><span class="n">{c.count.toLocaleString()}</span>
        </button>
      {/each}
      {#if insts.length}
        <h3 class="engraved">Instruments</h3>
        {#each insts as ins (ins.id)}
          {@const v: SoundView = { kind: 'instrument', id: ins.id }}
          <button type="button" class="cat sub inst" class:on={is(v)} aria-pressed={is(v)} use:tip={'sounds.instrument'} onclick={() => show(v)}>
            <span class="iname">{ins.name}</span><span class="n">{ins.kind === 'soundFont' ? 'SF' : 'AU'}</span>
          </button>
        {/each}
      {/if}
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
          oninput={() => void tick().then(onfilter)}
          onfocus={() => queueMicrotask(() => input && tips.hide(input))}
        />
        <span class="count engraved">{rows.length.toLocaleString()} of {chipTotal.toLocaleString()}{#if listingChip}&nbsp;· listing presets{:else if chipError}<span class="warn" title={chipError}>&nbsp;· presets not listed: {chipError}</span>{:else if app.state.sounds?.scanning}&nbsp;· scanning plugins{/if}</span>
      </div>

      <div class="screen mat-screen">
        <div class="scroller" bind:this={list} onscroll={() => (scrollTop = list?.scrollTop ?? 0)}>
          <div class="rows" id="sounds-list" role="listbox" tabindex="-1" aria-label="Sounds" style:height="{rows.length * ROW}px">
            {#each slice as i, k (entries[i].id)}
              {@const e = entries[i]}
              {@const n = soundNumber(e, byId)}
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
                use:tip={e.plugin?.lastError ? 'sounds.row_failed' : 'sounds.row_pick'}
                onclick={() => ((cursorId = e.id), choose(i))}
              >
                <button type="button" class="star" class:on={e.favourite} tabindex="-1" aria-label={e.favourite ? 'Unstar' : 'Star'} aria-pressed={e.favourite} use:tip={'sounds.favourite'} onclick={(ev) => (ev.stopPropagation(), star(i))}>{e.favourite ? '★' : '☆'}</button>
                <span class="mark" aria-hidden="true">{e.id === playing ? '▶' : ''}</span>
                {#if n !== null}<span class="num" use:tip={'sound.number'}>{n}</span>{:else}<span class="num"></span>{/if}
                <span class="name">{e.name}</span>
                <span class="badge {e.source}">{SOURCE_BADGE[e.source]}</span>
                <span class="detail">{view.kind === 'all' || view.kind === 'category' ? slotText(e.id) : ''}{e.detail}{#if e.plugin?.lastError}<span class="warn" title={e.plugin.lastError}> ⚠ {e.plugin.lastError}</span>{/if}</span>
              </div>
            {/each}
          </div>
          {#if rows.length === 0}
            <p class="empty">
              {#if entries.length === 0}No sounds yet: no SoundFonts, plugins or saved sounds.
              {:else if view.kind === 'favourites' && !query}No favourites yet: star a sound with ☆ (or Ctrl+D).
              {:else if view.kind === 'recents' && !query}Nothing picked yet.
              {:else if view.kind === 'mine' && !query}Nothing in My Sounds yet.
              {:else if chipId && !query}{listingChip ? `Listing ${shownName}'s presets…` : chipError ? `Could not list ${shownName}'s presets: ${chipError}` : `${shownName} has no presets.`}
              {:else}No sound matches “{query}”.{/if}
            </p>
          {/if}
        </div>
      </div>

      <footer class="foot">
        <span class="now">Pick the sound for <b>{pick.title}</b></span>
      </footer>
    </section>
  </div>
</Overlay>

<style>
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
  .iname {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
    grid-template-columns: 1.8rem 1rem 2.2rem minmax(8rem, 1.4fr) 3.2rem minmax(0, 1fr);
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
  .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: var(--screen-dim);
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
  button.star {
    border: 0;
    background: none;
    padding: 0;
    height: 36px;
    color: var(--screen-dim);
  }
  .star.on {
    color: var(--accent);
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
