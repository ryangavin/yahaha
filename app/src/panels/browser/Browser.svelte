<!--
  The style browser (a modal over the Launchkey mirror, open while `ui.browser`).

  ┌ Categories ─┐┌ Filter ───────────────────────────────── 1,204 of 60,043 ┐
  │ All styles  ││ ☆ ▶ Name         Folder     Tempo Time  Sections  SFF    │
  │ ★ Favourites││ ☆   Sunday Drive Pop & Rock  104  4/4  ••• ••••  SFF2 ▶ │  virtualised:
  │ ↺ Recent    ││ …                                                         │  only the rows
  │ FOLDERS     ││                                                           │  in view exist
  │  Pop & Rock ││                                                           │
  └─────────────┘└ preview status · Preview on select · < Track · Track > ──┘

  - Order is the library's (folder, then name), the same order < Track / Track > step
    through, so the ‹ › marks and the mirror's Track buttons always agree with the list.
  - The filter field keeps focus: typing filters; ↑/↓ PgUp/PgDn Home/End move; Enter
    loads (the band keeps playing, like the terminal browser); Shift+Enter previews
    (stopped) or queues for the next bar (playing); Ctrl/⌘+D stars; Esc closes.
  - Preview and Load at next bar use the provisional `state.preview` + PreviewCmd
    (lib/api/types.ts). Without `state.preview` (an engine that can't audition yet) those
    controls don't show.
-->
<script lang="ts">
  import { onDestroy, onMount, tick, untrack } from 'svelte'
  import type { AppCmd } from '../../lib/api/types'
  import { css } from '../../lib/leds'
  import { app, ui } from '../../lib/store.svelte'
  import { surfaceOf } from '../../lib/surface'
  import { tip, tips } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import Overlay from '../../lib/ui/Overlay.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'
  import { folderTree, indexLibrary, keepCursor, moveCursor, visibleRows, type Category } from './model'
  import { prefs } from './prefs.svelte'
  import Row from './Row.svelte'
  import Sidebar from './Sidebar.svelte'

  /** Row height in px (the list is virtualised on it); at least the 2.2rem hit target. */
  const ROW = 36
  const OVERSCAN = 8
  /** How long a row must stay highlighted or hovered before Preview on select plays it. */
  const PREVIEW_DELAY_MS = 600

  // ── The list ────────────────────────────────────────────────────────────
  const entries = $derived(app.library.entries)
  const ix = $derived(indexLibrary(entries))
  const folders = $derived(folderTree(entries))
  /** The remembered category, unless its folder has gone from the library. */
  const category = $derived.by((): Category => {
    const c = prefs.category
    return c.kind === 'folder' && !folders.some((f) => f.path === c.path) ? { kind: 'all' } : c
  })
  let query = $state('')
  const rows = $derived(visibleRows(ix, category, query, prefs.favourites, prefs.recents))
  const favouriteCount = $derived(entries.reduce((n, e) => n + (prefs.favourites.has(e.path) ? 1 : 0), 0))
  const recentCount = $derived(prefs.recents.filter((p) => ix.byPath.has(p)).length)

  /** The highlighted style, by id, so it stays put while the list changes around it. */
  let cursorId = $state<number | null>(untrack(() => app.state.style.id))
  const cursor = $derived(keepCursor(rows, entries, cursorId))

  let list: HTMLDivElement | undefined = $state()
  let input: HTMLInputElement | undefined = $state()
  let scrollTop = $state(0)
  let height = $state(480)
  const first = $derived(Math.max(0, Math.floor(scrollTop / ROW) - OVERSCAN))
  const last = $derived(Math.min(rows.length, Math.ceil((scrollTop + height) / ROW) + OVERSCAN))
  const slice = $derived(rows.slice(first, last))

  // ── Engine state the rows show (primitives, so a 60 Hz state only touches what changed) ──
  const loadedId = $derived(app.state.style.id)
  const running = $derived(app.state.transport.running)
  const canPreview = $derived(app.state.preview !== undefined)
  const auditionId = $derived(app.state.preview?.audition?.id ?? null)
  const queuedId = $derived(app.state.preview?.queued ?? null)
  const rowAction = $derived(!canPreview ? null : running ? ('queue' as const) : ('preview' as const))
  // < Track / Track > neighbours: the engine's (`state.surface`), derived only while it doesn't send them.
  const near = $derived(surfaceOf(app.state, app.library))
  const prevId = $derived(near.trackPrev?.id ?? null)
  const nextId = $derived(near.trackNext?.id ?? null)
  const trackPrev = $derived(near.trackPrev?.name ?? '')
  const trackNext = $derived(near.trackNext?.name ?? '')

  // Section lamp colours: the section pads' own, from the engine.
  const lampRgb = (t: AppCmd['type'], fallback: string) => {
    const p = app.state.transport.lamps.find((l) => l.action?.type === t)
    return p ? css(p.rgb) : fallback
  }
  const cIntro = $derived(lampRgb('intro', 'rgb(90 200 255)'))
  const cMain = $derived(lampRgb('main', 'rgb(60 230 120)'))
  const cBreak = $derived(lampRgb('break', 'rgb(255 170 60)'))
  const cEnding = $derived(lampRgb('ending', 'rgb(255 90 90)'))
  const colours = $derived({ intro: cIntro, main: cMain, brk: cBreak, ending: cEnding })

  const auditionName = $derived(auditionId === null ? '' : (entries.find((e) => e.id === auditionId)?.name ?? ''))
  const queuedName = $derived(queuedId === null ? '' : (entries.find((e) => e.id === queuedId)?.name ?? ''))
  const audition = $derived(app.state.preview?.audition ?? null)
  const pending = $derived(app.state.library.pending)

  // ── Scrolling ───────────────────────────────────────────────────────────
  function ensureVisible(k: number, center = false) {
    if (!list || k < 0) return
    const h = list.clientHeight || height
    const top = k * ROW
    if (center) list.scrollTop = Math.max(0, top - h / 2 + ROW / 2)
    else if (top < list.scrollTop) list.scrollTop = top
    else if (top + ROW > list.scrollTop + h) list.scrollTop = top + ROW - h
    scrollTop = list.scrollTop
  }

  function follow(center = false) {
    void tick().then(() => ensureVisible(cursor, center))
  }

  // ── Actions ─────────────────────────────────────────────────────────────
  function load(i: number) {
    const e = entries[i]
    if (!e || e.status === 'error') return
    if (e.id !== loadedId) app.send({ type: 'loadStyle', id: e.id })
    ui.browser = false
  }

  /** Stopped: preview (or stop the preview of) this style. Playing: load it at the next bar. */
  function previewOrQueue(i: number) {
    const e = entries[i]
    if (!e || e.status === 'error' || !canPreview) return
    if (running) {
      if (e.id !== loadedId) app.send({ type: 'queueStyle', id: e.id })
    } else if (auditionId === e.id) app.send({ type: 'stopAudition' })
    else app.send({ type: 'auditionStyle', id: e.id })
  }

  let previewTimer: ReturnType<typeof setTimeout> | null = null
  function cancelAutoPreview() {
    if (previewTimer) clearTimeout(previewTimer)
    previewTimer = null
  }
  /** Preview on select: after a short pause on a row, preview it (stopped only). */
  function autoPreview(i: number) {
    cancelAutoPreview()
    const e = entries[i]
    if (!prefs.autoPreview || !canPreview || running || !e || e.status !== 'ok') return
    previewTimer = setTimeout(() => {
      previewTimer = null
      const p = app.state.preview
      if (p && !app.state.transport.running && ui.browser && p.audition?.id !== e.id) app.send({ type: 'auditionStyle', id: e.id })
    }, PREVIEW_DELAY_MS)
  }

  function setCursor(k: number) {
    const i = rows[k]
    if (i === undefined) return
    cursorId = entries[i].id
    ensureVisible(k)
    autoPreview(i)
  }

  function pick(c: Category) {
    prefs.setCategory(c)
    follow(true)
  }

  function onkey(e: KeyboardEvent) {
    const page = Math.max(1, Math.floor(height / ROW) - 1)
    const m = moveCursor(e.key, cursor, rows.length, page)
    if (m !== null) {
      e.preventDefault()
      setCursor(m)
      return
    }
    const i = rows[cursor]
    if (e.key === 'Enter') {
      e.preventDefault()
      if (i === undefined) return
      if (e.shiftKey) previewOrQueue(i)
      else load(i)
      return
    }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'd') {
      e.preventDefault()
      if (i !== undefined) prefs.toggleFavourite(entries[i].path)
    }
  }

  // < Track / Track > (or a pad, or a Launchkey) loaded another style: the cursor follows.
  let seenLoaded = untrack(() => app.state.style.id)
  $effect(() => {
    const id = loadedId
    untrack(() => {
      if (id === seenLoaded) return
      seenLoaded = id
      cursorId = id
      follow()
    })
  })

  // The list's height sets how many rows exist (no ResizeObserver in old webviews or jsdom).
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

  // The filter field holds focus the whole time the browser is open, so its tooltip
  // shows on hover only (`onfocus` hides it), not on every focus where it covers the list.
  onMount(() => {
    void tick().then(() => input?.focus())
  })

  // Open centred on the loaded style, as soon as the library has arrived.
  let centred = false
  $effect(() => {
    const i = rows[cursor]
    if (centred || i === undefined || entries[i].id !== cursorId) return
    centred = true
    follow(true)
  })

  onDestroy(() => {
    cancelAutoPreview()
    // Closing the browser ends a preview: nothing keeps sounding behind the performer.
    if (app.state.preview?.audition) app.send({ type: 'stopAudition' })
  })
</script>

<Overlay id="browser" title="Styles" side="center" modal closeTip="browser.close" onclose={() => (ui.browser = false)}>
  <div class="browser">
    <Sidebar {category} {folders} total={entries.length} favourites={favouriteCount} recents={recentCount} onpick={pick} />

    <section class="main">
      <div class="filter">
        <input
          bind:this={input}
          bind:value={query}
          class="mat-well"
          type="text"
          placeholder="Filter by name, file name or folder"
          spellcheck="false"
          autocomplete="off"
          role="combobox"
          aria-expanded="true"
          aria-controls="browser-list"
          aria-autocomplete="list"
          aria-activedescendant={rows[cursor] !== undefined ? `style-${entries[rows[cursor]].id}` : undefined}
          aria-label="Filter styles"
          use:tip={'browser.filter'}
          onkeydown={onkey}
          oninput={() => follow(true)}
          onfocus={() => queueMicrotask(() => input && tips.hide(input))}
        />
        <span class="count engraved">
          {rows.length.toLocaleString()} of {entries.length.toLocaleString()}{#if pending}&nbsp;· {pending.toLocaleString()} indexing{/if}
        </span>
      </div>

      <div class="screen mat-screen">
        <div class="head engraved" aria-hidden="true">
          <span></span><span></span><span>Name</span><span class="h-folder">Folder</span><span class="r">Tempo</span><span class="r">Time</span>
          <span>Intro · Main · Brk · End</span><span class="c">SFF</span><span></span>
        </div>
        <div
          class="scroller"
          bind:this={list}
          onscroll={() => (scrollTop = list?.scrollTop ?? 0)}
        >
          <div class="rows" id="browser-list" role="listbox" tabindex="-1" aria-label="Styles" style:height="{rows.length * ROW}px" onpointerleave={cancelAutoPreview}>
            {#each slice as i, k (entries[i].id)}
              {@const e = entries[i]}
              <Row
                entry={e}
                id="style-{e.id}"
                top={(first + k) * ROW}
                active={first + k === cursor}
                loaded={e.id === loadedId}
                mark={e.id === prevId ? '‹' : e.id === nextId ? '›' : ''}
                favourite={prefs.favourites.has(e.path)}
                {colours}
                action={rowAction}
                auditioning={e.id === auditionId}
                queued={e.id === queuedId}
                onload={() => load(i)}
                onfavourite={() => prefs.toggleFavourite(e.path)}
                onpreview={() => previewOrQueue(i)}
                onhover={() => autoPreview(i)}
              />
            {/each}
          </div>
          {#if rows.length === 0}
            <p class="empty">
              {#if entries.length === 0}No styles yet: the library is empty or still being read.
              {:else if category.kind === 'favourites' && !query}No favourites yet: star a style with ☆ (or Ctrl+D).
              {:else if category.kind === 'recents' && !query}Nothing loaded yet.
              {:else}No style matches “{query}”.{/if}
            </p>
          {/if}
        </div>
      </div>

      <footer class="foot">
        <div class="status" aria-live="polite">
          {#if audition}
            <span class="dot" aria-hidden="true"></span>
            <span class="what">Previewing <b>{auditionName}</b></span>
            <span class="bar">bar {audition.bar}/{audition.bars}{audition.chord ? ` · ${audition.chord}` : ''}</span>
            <HwButton tip="browser.preview_stop" onclick={() => app.send({ type: 'stopAudition' })}>■ Stop</HwButton>
          {:else if canPreview && running && queuedId !== null}
            <span class="what">Next bar: <b>{queuedName}</b></span>
          {:else if canPreview && running}
            <span class="hint">Band playing, so no previews. Shift+Enter or Next bar loads on the bar line.</span>
          {:else if canPreview}
            <span class="hint">Shift+Enter or ▶ previews a style while the band is stopped.</span>
          {/if}
        </div>
        {#if canPreview}
          <Toggle on={prefs.autoPreview} tip="browser.auto_preview" onclick={() => prefs.setAutoPreview(!prefs.autoPreview)}>Preview on select</Toggle>
        {/if}
        <div class="track">
          <HwButton tip="style.prev" label="Previous style" caption={trackPrev} width="7.5rem" onclick={() => app.send({ type: 'stepStyle', delta: -1 })}>◀ Track</HwButton>
          <HwButton tip="style.next" label="Next style" caption={trackNext} width="7.5rem" onclick={() => app.send({ type: 'stepStyle', delta: 1 })}>Track ▶</HwButton>
        </div>
      </footer>
    </section>
  </div>
</Overlay>

<style>
  .browser {
    --row: 36px;
    display: grid;
    grid-template-columns: 13.5rem minmax(0, 1fr);
    gap: 1rem;
    height: 100%;
    min-height: 0;
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
  .filter input::placeholder {
    color: var(--screen-dim);
  }
  .count {
    flex: none;
    white-space: nowrap;
  }
  /* The list is the instrument's display: dark glass in both themes, with the dark
     theme's amber so highlights read on it. */
  .screen {
    --accent: #f3b843;
    --danger: #ff7a6b;
    --cols: 1.8rem 1rem minmax(8rem, 2fr) minmax(0, 1.3fr) 2.6rem 2.4rem 7rem 2.8rem 4.6rem;
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-radius: 8px;
    overflow: hidden;
    container-type: inline-size;
  }
  .head {
    display: grid;
    grid-template-columns: var(--cols);
    column-gap: 0.6rem;
    align-items: center;
    height: 1.9rem;
    padding: 0 0.5rem 0 0.25rem;
    border-bottom: 1px solid rgb(255 255 255 / 0.08);
    color: var(--screen-dim);
    text-shadow: none;
    white-space: nowrap;
    overflow: hidden;
  }
  .r {
    text-align: right;
  }
  .c {
    text-align: center;
  }
  @container (max-width: 44rem) {
    .screen {
      --cols: 1.8rem 1rem minmax(8rem, 1fr) 0 2.6rem 2.4rem 7rem 2.8rem 4.6rem;
    }
    .h-folder {
      visibility: hidden;
    }
  }
  .scroller {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
  }
  .rows {
    position: relative;
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
    align-items: flex-start;
    gap: 1rem;
    min-height: 3.4rem;
  }
  .status {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 0.6rem;
    min-width: 0;
    min-height: 2.2rem;
    font-size: var(--fs-small);
    color: var(--muted);
  }
  .status :global(.hw) {
    flex: none;
  }
  .what,
  .hint {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  .what {
    color: var(--ink);
  }
  .bar {
    flex: none;
    font-family: var(--font-display);
    font-size: 0.95rem;
    color: var(--ink);
  }
  .dot {
    flex: none;
    width: 0.55rem;
    height: 0.55rem;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 8px var(--accent);
  }
  .track {
    display: flex;
    gap: 0.5rem;
    flex: none;
  }
  @media (max-width: 1100px) {
    .browser {
      grid-template-columns: 11rem minmax(0, 1fr);
    }
    .hint {
      display: none;
    }
  }
</style>
