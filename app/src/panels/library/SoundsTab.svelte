<!--
  Library › Sounds (docs/racks.md, "Screens"; the wireframe's `browserSounds`): every
  sound a keyboard part can play, loading into the target part (`ui.libraryPart`).

  [Search name, instrument, tag…] (All)(Mine)(Factory)(SoundFont)(★) (Sampler Deluxe ✕)
  ┌ Category ┐┌ 42 sounds in Piano ───────────── click plays on Right 1 · ↑ ↓ steps ┐
  │ All   142 ││ ▶ Stage Grand · R1   GeneralUser-GS   SOUNDFONT  ☆                   │
  │ Piano  12 ││   Silk Strings       Sampler Deluxe   MINE       ★                   │
  │ …         │├ Sound details: name, category, used in, Duplicate / Copy to My Sounds ┤
  └───────────┘└──────────────────────────────────────────────────────────────────────┘

  A click plays the row on the part at once (`assignSound`), so you hear it in the rack;
  ↑ ↓ (PgUp/PgDn, Home/End) step and play; Enter plays the selected row; Ctrl/⌘+D stars.
  The list is virtualised: a font's presets (Instruments › Browse) run to a thousand rows.
-->
<script lang="ts">
  import { onMount, tick, untrack } from 'svelte'
  import { app, ui } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import { CATEGORY_LABELS } from '../../lib/api/sound-library'
  import type { GmMapRow } from '../../lib/api/types'
  import { moveCursor } from '../browser/model'
  import { inMySounds } from '../sounds/instruments'
  import { instrumentOf, patchesById } from '../sounds/model'
  import SoundEdit from '../sounds/SoundEdit.svelte'
  import { BADGE_LABEL, PART_SHORT, badgeOf, instrumentNames, libraryCategories, librarySounds, playingByPart, type SourceChip } from './model'
  import { libraryNav as f } from './nav.svelte'

  const ROW = 34
  const OVERSCAN = 8
  // One empty GM map, so an engine without one doesn't make a new array per state.
  const NO_ROWS: GmMapRow[] = []

  const part = $derived(ui.libraryPart)
  const catalog = $derived(app.sounds)
  const entries = $derived(catalog.entries)
  // The slices themselves, so a state that leaves them alone re-runs nothing below.
  const patches = $derived(app.state.soundLibrary.patches)
  const gmMap = $derived(app.state.soundLibrary.gmMap ?? NO_ROWS)
  const ctx = $derived.by(() => ({ patches, gmMap }))
  const filter = $derived.by(() => ({ source: f.source, favourites: f.favourites, instrument: f.instrument, category: f.category, query: f.query }))
  const rows = $derived(librarySounds(catalog, filter, ctx))
  const cats = $derived(libraryCategories(catalog, filter, ctx))
  const total = $derived(cats.reduce((n, c) => n + c.count, 0))
  const names = $derived(instrumentNames(catalog))
  const byId = $derived(patchesById(patches))
  const parts = $derived(app.state.keyboardParts)
  const playing = $derived(playingByPart(parts, ctx))
  const listingPresets = $derived(app.state.sounds?.listingPresets)
  const listing = $derived(new Set(listingPresets ?? []))

  const instName = (id: string | null) => (id ? (names.get(id) ?? id.replace(/^(sf|au):/, '').replace(/\.sf2$/i, '')) : '')
  const rowInst = (i: number) => instName(instrumentOf(entries[i], byId))
  const partsOn = (id: string) => playing.flatMap((p, k) => (p === id ? [k] : []))

  // The row moved to; null follows what the target part plays.
  let cursorId = $state<string | null>(null)
  const cursor = $derived(rows.findIndex((i) => entries[i].id === (cursorId ?? playing[part])))
  const selected = $derived(cursor >= 0 ? entries[rows[cursor]] : entries.find((e) => e.id === (cursorId ?? playing[part])))
  const selPatch = $derived(selected?.source === 'saved' ? byId.get(selected.id) : undefined)
  // Another target part: follow what it plays.
  $effect(() => {
    void part
    untrack(() => {
      cursorId = null
      void tick().then(() => ensureVisible(cursor, true))
    })
  })

  // Instruments › Browse on a plugin: its factory presets need an instance, listed once
  // (the engine answers from its cache after that). A failed listing isn't tried again.
  $effect(() => {
    const id = f.instrument
    if (!id?.startsWith('au:')) return
    untrack(() => {
      const e = entries.find((x) => x.id === id)
      if (e?.plugin && !e.plugin.lastError && !e.plugin.presetsError && !listing.has(id)) app.send({ type: 'listPluginPresets', id })
    })
  })

  let list: HTMLDivElement | undefined = $state()
  let box: HTMLDivElement | undefined = $state()
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

  /** Plays row `i` (an index into the entries) on the target part, at once. */
  function play(i: number) {
    const e = entries[i]
    if (!e) return
    cursorId = e.id
    app.send({ type: 'assignSound', part, id: e.id })
  }
  const star = (i: number) => entries[i] && app.send({ type: 'setSoundFavourite', id: entries[i].id, on: !entries[i].favourite })

  /** ↑ ↓ PgUp PgDn Home End step and play; Enter plays; Ctrl/⌘+D stars. Handled keys stay
   * here (the window's performance keys don't also get them). */
  function onkey(e: KeyboardEvent) {
    const m = moveCursor(e.key, cursor, rows.length, Math.max(1, Math.floor(height / ROW) - 1))
    const i = rows[cursor]
    if (m !== null) {
      e.preventDefault()
      e.stopPropagation()
      if (m !== cursor || cursor < 0) play(rows[m])
      ensureVisible(m)
    } else if (e.key === 'Enter') {
      e.preventDefault()
      e.stopPropagation()
      if (i !== undefined) play(i)
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'd') {
      e.preventDefault()
      e.stopPropagation()
      if (i !== undefined) star(i)
    }
  }

  function setSource(s: SourceChip) {
    f.source = s
    refocus()
  }
  function setCategory(c: typeof f.category) {
    f.category = c
    refocus()
  }
  function refocus() {
    void tick().then(() => ensureVisible(cursor, true))
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

  const CHIPS: { id: SourceChip; label: string; tip: 'library.src_all' | 'library.src_mine' | 'library.src_factory' | 'library.src_soundfont' }[] = [
    { id: 'all', label: 'All', tip: 'library.src_all' },
    { id: 'mine', label: 'Mine', tip: 'library.src_mine' },
    { id: 'factory', label: 'Factory', tip: 'library.src_factory' },
    { id: 'soundFont', label: 'SoundFont', tip: 'library.src_soundfont' },
  ]
  const usedIn = $derived(selected ? partsOn(selected.id).map((k) => parts[k].name) : [])
  const mine = $derived(!!selected && selected.source !== 'saved' && inMySounds(patches, selected.id))
  const listingNow = $derived(!!f.instrument && listing.has(f.instrument))
  const listError = $derived(f.instrument ? entries.find((e) => e.id === f.instrument)?.plugin?.presetsError : undefined)
</script>

<div class="sounds">
  <div class="filters">
    <input
      bind:this={input}
      bind:value={f.query}
      class="mat-well search"
      type="search"
      placeholder="Search name, instrument, category…"
      spellcheck="false"
      autocomplete="off"
      role="combobox"
      aria-expanded="true"
      aria-controls="library-sounds"
      aria-autocomplete="list"
      aria-activedescendant={rows[cursor] !== undefined ? `lib-sound-${rows[cursor]}` : undefined}
      aria-label="Search sounds"
      use:tip={'library.search'}
      onkeydown={onkey}
      oninput={refocus}
    />
    <div class="chips" role="group" aria-label="Source">
      {#each CHIPS as c (c.id)}
        <button type="button" class="chip" class:on={f.source === c.id} aria-pressed={f.source === c.id} use:tip={c.tip} onclick={() => setSource(c.id)}>{c.label}</button>
      {/each}
      <button type="button" class="chip" class:on={f.favourites} aria-pressed={f.favourites} aria-label="Starred only" use:tip={'library.src_starred'} onclick={() => ((f.favourites = !f.favourites), refocus())}>★</button>
      {#if f.instrument}
        <button type="button" class="chip inst" use:tip={'library.inst_clear'} aria-label="Show every instrument" onclick={() => ((f.instrument = null), refocus())}>{instName(f.instrument)} ✕</button>
      {/if}
    </div>
  </div>

  <div class="cols">
    <nav class="cats" aria-label="Category">
      <span class="lbl engraved">Category</span>
      <button type="button" class="cat" class:on={!f.category} aria-pressed={!f.category} use:tip={'library.category'} onclick={() => setCategory(null)}>
        <span>All</span><span class="n">{total.toLocaleString()}</span>
      </button>
      {#each cats as c (c.id)}
        <button type="button" class="cat" class:on={f.category === c.id} class:zero={!c.count} aria-pressed={f.category === c.id} use:tip={'library.category'} onclick={() => setCategory(c.id)}>
          <span>{c.label}</span><span class="n">{c.count.toLocaleString()}</span>
        </button>
      {/each}
    </nav>

    <section class="results" aria-label="Sounds">
      <div class="rhead engraved">
        <span>{rows.length.toLocaleString()} sound{rows.length === 1 ? '' : 's'}{f.category ? ` in ${CATEGORY_LABELS[f.category]}` : ''}{#if listingNow}&nbsp;· listing presets…{:else if listError}<span class="warn">&nbsp;· presets not listed: {listError}</span>{:else if app.state.sounds?.scanning}&nbsp;· scanning plugins{/if}</span>
        <span>click plays on {parts[part]?.name} · ↑ ↓ steps</span>
      </div>
      <div class="screen mat-screen">
        <div class="scroller" bind:this={list} onscroll={() => (scrollTop = list?.scrollTop ?? 0)}>
          <div class="rows" id="library-sounds" role="listbox" tabindex="0" aria-label="Sounds" style:height="{rows.length * ROW}px" bind:this={box} onkeydown={onkey} use:tip={'library.list'}>
            {#each slice as i, k (entries[i].id)}
              {@const e = entries[i]}
              {@const on = partsOn(e.id)}
              {@const b = badgeOf(e)}
              <!-- svelte-ignore a11y_click_events_have_key_events (the list and the search take ↑ ↓ and Enter) -->
              <div
                class="row"
                class:active={first + k === cursor}
                class:playing={on.includes(part)}
                role="option"
                tabindex="-1"
                id="lib-sound-{i}"
                aria-selected={first + k === cursor}
                style:transform="translateY({(first + k) * ROW}px)"
                use:tip={'library.row'}
                onclick={() => (play(i), box?.focus())}
              >
                <span class="mark" aria-hidden="true">{on.includes(part) ? '▶' : ''}</span>
                <span class="name">{e.name}{#if on.length}<span class="on"> · {on.map((x) => PART_SHORT[x]).join(', ')}</span>{/if}</span>
                <span class="in">{#if e.plugin?.lastError}<span class="warn" title={e.plugin.lastError}>⚠ </span>{/if}{rowInst(i)}</span>
                <span class="badge {b}">{BADGE_LABEL[b]}</span>
                <button type="button" class="star" class:on={e.favourite} tabindex="-1" aria-label={e.favourite ? 'Unstar' : 'Star'} aria-pressed={e.favourite} use:tip={'sounds.favourite'} onclick={(ev) => (ev.stopPropagation(), star(i))}>{e.favourite ? '★' : '☆'}</button>
              </div>
            {/each}
          </div>
          {#if rows.length === 0}
            <p class="empty">
              {#if entries.length === 0}No sounds yet: no SoundFonts, plugins or saved sounds.
              {:else if listingNow}Listing {instName(f.instrument)}'s presets…
              {:else if listError}Could not list {instName(f.instrument)}'s presets: {listError}
              {:else}No sounds match. Clear a filter, or make one from Instruments.{/if}
            </p>
          {/if}
        </div>
      </div>

      {#if selected}
        <div class="details">
          <h3 class="engraved">Sound details</h3>
          {#if selPatch}
            <SoundEdit patch={selPatch} onback={() => input?.focus()} />
          {:else}
            <div class="line">
              <b class="dname">{selected.name}</b>
              <span class="badge {badgeOf(selected)}">{BADGE_LABEL[badgeOf(selected)]}</span>
              <span class="k">{CATEGORY_LABELS[selected.category]}</span>
              <span class="grow"></span>
              <button type="button" class="act" class:done={mine} disabled={mine} use:tip={'library.copy'} onclick={() => app.send({ type: 'addToMySounds', id: selected.id })}>{mine ? '✓ In My Sounds' : 'Copy to My Sounds'}</button>
            </div>
          {/if}
          <div class="line meta">
            <span><span class="k">Instrument</span> {instName(instrumentOf(selected, byId)) || '—'}</span>
            <span><span class="k">Used in</span> {usedIn.length ? `${usedIn.join(', ')} of the live rack` : 'no part of the live rack'}</span>
          </div>
        </div>
      {/if}
    </section>
  </div>
</div>

<style>
  .sounds {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    height: 100%;
    min-height: 0;
  }
  .filters {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.6rem;
  }
  .search {
    flex: 1 1 14rem;
    min-width: 0;
    height: 2.3rem;
    padding: 0 0.8rem;
    border: 1px solid var(--well-edge);
    border-radius: 6px;
    color: var(--screen-ink);
    font-size: 1rem;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
  }
  .chip {
    min-height: 2rem;
    padding: 0 0.8rem;
    border: 1px solid var(--line-strong);
    border-radius: 1rem;
    background: none;
    color: var(--ink);
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.9rem;
  }
  .chip:hover {
    background: rgb(127 127 127 / 0.1);
  }
  .chip.on {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 22%, transparent);
  }
  .chip.inst {
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
    color: var(--accent);
  }
  .cols {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 10.5rem minmax(0, 1fr);
    gap: 0.8rem;
  }
  .cats {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-height: 0;
    overflow: auto;
  }
  .lbl {
    margin: 0.1rem 0.5rem 0.3rem;
    font-size: 0.72rem;
  }
  .cat {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
    min-height: 1.9rem;
    padding: 0 0.6rem;
    border: 1px solid transparent;
    border-radius: 5px;
    background: none;
    text-align: left;
    font-size: var(--fs-small);
    color: var(--ink);
  }
  .cat:hover {
    background: rgb(127 127 127 / 0.1);
  }
  .cat.on {
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
    background: color-mix(in srgb, var(--accent) 16%, transparent);
  }
  .cat.zero {
    color: var(--muted);
  }
  .n {
    color: var(--muted);
    font-size: 0.78rem;
  }
  .results {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    min-width: 0;
    min-height: 0;
  }
  .rhead {
    display: flex;
    justify-content: space-between;
    gap: 0.8rem;
    font-size: 0.72rem;
    white-space: nowrap;
    overflow: hidden;
  }
  .screen {
    --accent: #f3b843;
    --danger: #ff7a6b;
    flex: 1;
    min-height: 8rem;
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
    outline: none;
  }
  .rows:focus-visible {
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .row {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    height: 34px;
    display: grid;
    grid-template-columns: 1rem minmax(8rem, 1.5fr) minmax(0, 1fr) 5.4rem 1.6rem;
    align-items: center;
    column-gap: 0.6rem;
    padding: 0 0.4rem 0 0.6rem;
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
  .in {
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }
  .on,
  .in {
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
    font-size: 0.7rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .badge.mine {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--accent-ink);
  }
  .badge.factory {
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
  }
  .badge.soundFont {
    border-style: dashed;
  }
  button.star {
    border: 0;
    background: none;
    padding: 0;
    height: 34px;
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
  .details {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    padding: 0.5rem 0.2rem 0;
    border-top: 1px solid var(--line);
  }
  .details h3 {
    margin: 0;
    font-size: 0.72rem;
  }
  .details .badge {
    border-color: var(--line-strong);
  }
  .details .badge.mine {
    border-color: var(--accent);
  }
  .line {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    min-width: 0;
    flex-wrap: wrap;
  }
  .dname {
    font-family: var(--font-display);
    font-size: 1.05rem;
  }
  .grow {
    flex: 1;
  }
  .meta {
    font-size: var(--fs-small);
    gap: 1.2rem;
  }
  .k {
    color: var(--muted);
    font-size: var(--fs-small);
  }
  button.act {
    min-height: 2rem;
    border: 1px solid var(--line-strong);
    border-radius: 4px;
    background: linear-gradient(180deg, var(--raised-hi), var(--raised-lo));
    padding: 0 0.7rem;
    color: var(--ink);
    font-family: var(--font-display);
    font-weight: 600;
  }
  button.act.done {
    background: none;
    border-color: transparent;
    color: var(--accent);
  }
</style>
