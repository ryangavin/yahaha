<!--
  The Sound Browser (#117): one list of every sound for a keyboard part, whatever it
  comes from: SoundFont presets, instrument plugins, saved sounds (a modal over the
  mirror, open while `ui.soundBrowser` is a part). With `pick` it picks for something else
  instead, a program map rule: Enter or a click hands the sound over and closes.

  ┌ All sounds   ┐┌ Filter ─────────────────────────── 1,219 of 1,219 ┐
  │ ★ Favourites ││ ☆ Grand Piano      SF  GeneralUser-GS.sf2      ▶ │  virtualised
  │ ↺ Recent     ││ ☆ Serum            AU  Xfer Records  ⚠         ▶ │
  │ CATEGORIES   ││ …                                                 │
  └──────────────┘└ Right 1 plays: Grand Piano · Edit… · Rescan ──────┘

  Keys (the filter keeps focus): ↑/↓ PgUp/PgDn Home/End move; Enter plays the sound on
  the part (the browser stays open, as the Genos Voice Selection does); Shift+Enter
  auditions (the band stopped); Ctrl/⌘+D stars; Esc closes.
-->
<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte'
  import { app, ui, type SoundPick } from '../../lib/store.svelte'
  import { tip, tips } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import Overlay from '../../lib/ui/Overlay.svelte'
  import { moveCursor } from '../browser/model'
  import { pluginStatusLine } from '../parts/parts'
  import { SOURCE_BADGE, categoryCounts, playingId, visibleSounds, type SoundView } from './model'

  let { part = 0, pick = null }: { part?: number; pick?: SoundPick | null } = $props()

  const ROW = 36
  const OVERSCAN = 8

  const catalog = $derived(app.sounds)
  const entries = $derived(catalog.entries)
  let view = $state<SoundView>({ kind: 'all' })
  let query = $state('')
  const rows = $derived(visibleSounds(catalog, view, query))
  const cats = $derived(categoryCounts(entries))
  const favourites = $derived(entries.reduce((n, e) => n + (e.favourite ? 1 : 0), 0))

  const kp = $derived(pick ? undefined : app.state.keyboardParts[part])
  const playing = $derived(pick ? (pick.value ? `saved:${pick.value}` : null) : kp ? playingId(kp, app.state.io.soundFontFile) : null)
  const auditioning = $derived(app.state.sounds?.auditioning ?? null)
  const running = $derived(app.state.transport.running)
  const plugins = $derived(app.state.plugins)

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
                <span class="name">{e.name}</span>
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
              {:else}No sound matches “{query}”.{/if}
            </p>
          {/if}
        </div>
      </div>

      <footer class="foot">
        {#if pick}<span class="now">Pick the sound for <b>{pick.title}</b></span>{:else}<span class="now">{kp?.name} plays <b>{kp?.voiceName}</b>{#if kp?.plugin}&nbsp;· {pluginStatusLine(kp.plugin, plugins.available).replace(/ ▾$/, '')}{#if kp.plugin.status === 'playing'}&nbsp;· CPU {Math.round(kp.plugin.cpu * 100)}%{/if}{/if}</span>{/if}
        {#if auditioning}<HwButton tip="sounds.audition_stop" onclick={() => app.send({ type: 'stopSoundAudition' })}>■ Stop</HwButton>{/if}
        {#if kp?.plugin?.editor}<HwButton tip="part.plugin_edit" onclick={() => app.pluginEditor(part, true)}>Edit…</HwButton>{/if}
        {#if plugins.available}
          <HwButton tip="part.plugin_rescan" onclick={() => !plugins.scanning && app.send({ type: 'rescanPlugins' })}>{plugins.scanning ? 'Scanning' : 'Rescan'}</HwButton>
        {/if}
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
