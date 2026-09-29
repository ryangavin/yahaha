<!--
  The Sound Browser's Sounds tab (#117, docs/sound-browser.md "The Sounds tab"): the
  sounds a keyboard part can play (a modal over the mirror, open while `ui.soundBrowser`
  is a part). With `pick` it picks for something else instead, a program map rule: Enter
  or a click hands the sound over and closes.

  ┌ All sounds   ┐┌ Filter ─────────────────────────────── 142 of 1,219 ┐
  │ ★ Favourites ││ ☆ Grand Piano      SF  GeneralUser-GS · GM 1      ▶ │  virtualised
  │ ↺ Recent     ││ ☆ Silk Strings     Mine Sampler Deluxe            ▶ │
  │ ● My Sounds  ││ …                                                   │
  │ CATEGORIES   │├ [Silk Strings] [Strings ▾] Details Duplicate Delete… ┤  a library sound
  │ INSTRUMENTS  │└ Right 1 plays Sampler Deluxe · Silk Strings  edited  Save  Save as… ┘
  └──────────────┘

  Chips: All sounds is the GM map's resolved sounds (one row per program and the kit),
  every plugin sound and everything in My Sounds. Every other preset of a font, and a
  plugin's factory presets and .aupreset files, are under that instrument's chip.

  Two tabs (docs/sound-browser.md, D2): Sounds (this list) and Instruments
  (Instruments.svelte: each font and plugin with its presets, and the plugin housekeeping,
  category, Edit…, in process and Rescan, that used to sit in this footer).

  Keys (the filter keeps focus): ↑/↓ PgUp/PgDn Home/End move; Enter plays the sound on
  the part (the browser stays open, as the Genos Voice Selection does); Shift+Enter
  auditions (the band stopped); Ctrl/⌘+D stars; Ctrl/⌘+S saves; Ctrl/⌘+Shift+S saves
  as…; F2 renames the selected library sound; Ctrl/⌘+Delete deletes it (asked first); Esc
  closes. Tab reaches every chip and button.
-->
<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte'
  import { app, ui, type SoundPick } from '../../lib/store.svelte'
  import { tip, tips } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import Overlay from '../../lib/ui/Overlay.svelte'
  import { CATEGORY_LABELS } from '../../lib/api/sound-library'
  import type { PatchCategory } from '../../lib/api/types'
  import { moveCursor } from '../browser/model'
  import { pluginStatusLine } from '../parts/parts'
  import Instruments from './Instruments.svelte'
  import { browserNav, type BrowserTab } from './nav.svelte'
  import SoundEdit from './SoundEdit.svelte'
  import { SOURCE_BADGE, allSoundIds, categoryCounts, instrumentName, instruments, mapSlots, patchesById, playingId, presetFileName, visibleSounds, type SoundView } from './model'

  let { part = 0, pick = null }: { part?: number; pick?: SoundPick | null } = $props()

  const ROW = 36
  const OVERSCAN = 8

  const catalog = $derived(app.sounds)
  const entries = $derived(catalog.entries)
  const sl = $derived(app.state.soundLibrary)
  const ctx = $derived({ patches: sl.patches, gmMap: sl.gmMap ?? [] })
  let view = $state<SoundView>({ kind: 'all' })
  let query = $state('')
  const rows = $derived(visibleSounds(catalog, view, query, ctx))
  const listing = $derived(new Set(app.state.sounds?.listingPresets ?? []))
  const byId = $derived(patchesById(sl.patches))
  const slots = $derived(mapSlots(ctx.gmMap))
  const allIds = $derived(allSoundIds(ctx))
  const allCount = $derived(entries.reduce((n, e) => n + (allIds.has(e.id) ? 1 : 0), 0))
  const cats = $derived(categoryCounts(entries.filter((e) => allIds.has(e.id))))
  const insts = $derived(instruments(catalog))
  const favourites = $derived(entries.reduce((n, e) => n + (e.favourite ? 1 : 0), 0))
  const mine = $derived(entries.reduce((n, e) => n + (e.source === 'saved' ? 1 : 0), 0))

  const kp = $derived(pick ? undefined : app.state.keyboardParts[part])
  const plugins = $derived(app.state.plugins)
  const playing = $derived(pick ? (pick.value ? `saved:${pick.value}` : null) : kp ? playingId(kp, app.state.io.soundFontFile, ctx.gmMap) : null)
  const auditioning = $derived(app.state.sounds?.auditioning ?? null)
  const running = $derived(app.state.transport.running)

  // ── The Save flow (O3): Save over the part's own sound, Save as… a new one. ──────────
  // Save as… names the new sound; a plugin part can also keep it as an .aupreset that
  // Logic reads (asking before it replaces a file of that name, #307).
  // The Save as… waiting for its new sound: the sound added before it, and its name (a
  // Duplicate's "<name> copy", or a sound added before, is not it).
  let justSaved = $state<{ before: string | null; name: string } | null>(null)
  let saveForm = $state<{ name: string; aupreset: boolean; category: PatchCategory; replace: boolean } | null>(null)
  let saveName: HTMLInputElement | undefined = $state()
  function save() {
    if (kp) app.send({ type: 'saveSound', part })
  }
  function openSaveAs() {
    if (!kp) return
    const cur = kp.sound ? byId.get(kp.sound.id) : undefined
    saveForm = { name: kp.sound?.name ?? kp.voiceName, aupreset: false, category: cur?.category ?? 'synthLead', replace: false }
    void tick().then(() => (saveName?.focus(), saveName?.select()))
  }
  const closeSaveAs = () => ((saveForm = null), input?.focus())
  const canPreset = $derived(kp?.plugin?.status === 'playing')
  // A user preset of this plugin with that name exists: ask before replacing it, as Logic
  // does (the file is shared with Logic and MainStage).
  const clash = $derived.by(() => {
    if (!saveForm?.aupreset || !kp?.plugin) return false
    const parent = `au:${kp.plugin.id}`
    const n = presetFileName(saveForm.name).toLowerCase()
    return entries.some((e) => e.parent === parent && e.id.startsWith(`${parent}#u:`) && e.name.toLowerCase() === n)
  })
  function saveAs(overwrite = false) {
    if (!saveForm || !kp) return
    const name = saveForm.name.trim()
    if (!name) return
    // The sound added before this one (a send may add it at once).
    const before = sl.lastAdded
    if (saveForm.aupreset && canPreset) {
      if (clash && !overwrite) {
        saveForm.replace = true
        return
      }
      app.send({ type: 'savePartAsPluginPreset', part, name, category: saveForm.category, overwrite })
    }
    app.send({ type: 'saveSoundAs', part, name })
    justSaved = { before, name }
    closeSaveAs()
  }
  function saveFormKey(e: KeyboardEvent) {
    if (e.key !== 'Escape') return
    e.preventDefault()
    e.stopPropagation()
    closeSaveAs()
  }
  // The new sound shows in My Sounds, selected, once the catalog has it.
  $effect(() => {
    const id = sl.lastAdded
    if (!justSaved || !id || id === justSaved.before || !entries.some((e) => e.id === `saved:${id}`)) return
    const mine = byId.get(`saved:${id}`)?.name === justSaved.name
    justSaved = null
    if (!mine) return
    view = { kind: 'mine' }
    cursorId = `saved:${id}`
    void tick().then(() => ensureVisible(cursor, true))
  })

  // Instruments (O2) plays on a part: picking for a map rule shows Sounds only.
  const tab = $derived(pick ? 'sounds' : browserNav.tab)
  function showTab(t: BrowserTab) {
    browserNav.tab = t
    if (t === 'sounds') void tick().then(() => (input?.focus(), ensureVisible(cursor, true)))
  }

  let cursorId = $state<string | null>(null)
  const cursor = $derived(Math.max(0, cursorId === null ? rows.findIndex((i) => entries[i].id === playing) : rows.findIndex((i) => entries[i].id === cursorId)))
  const selected = $derived(rows[cursor] === undefined ? undefined : entries[rows[cursor]])
  // The selected library sound: rename, recategorise, delete (SoundEdit). A plugin's
  // category is filed on the Instruments tab (O2), with the rest of its housekeeping.
  const selPatch = $derived(!pick && selected?.source === 'saved' ? byId.get(selected.id) : undefined)
  let editor: SoundEdit | undefined = $state()

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
    // A plugin's factory presets need an instance: the engine lists them once, then
    // answers from its cache (a count from its .aupreset files alone says nothing).
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
    const mod = e.ctrlKey || e.metaKey
    if (e.key === 'Enter') {
      e.preventDefault()
      if (i !== undefined) (e.shiftKey ? audition : assign)(i)
    } else if (mod && e.key.toLowerCase() === 'd') {
      e.preventDefault()
      if (i !== undefined) star(i)
    } else if (mod && e.key.toLowerCase() === 's' && kp) {
      e.preventDefault()
      if (e.shiftKey) openSaveAs()
      else save()
    } else if (e.key === 'F2' && selPatch) {
      e.preventDefault()
      editor?.rename()
    } else if (mod && (e.key === 'Delete' || e.key === 'Backspace') && selPatch) {
      e.preventDefault()
      void editor?.askDelete()
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

  const is = (v: SoundView) => v.kind === view.kind && (v.kind !== 'category' || (view.kind === 'category' && view.id === v.id)) && (v.kind !== 'instrument' || (view.kind === 'instrument' && view.id === v.id))
  const slotText = (id: string) => {
    const s = slots.get(id)
    return s === undefined ? '' : s === 'drums' ? 'GM drums · ' : `GM ${s + 1} · `
  }
  const chipId = $derived(view.kind === 'instrument' ? view.id : null)
  const shownName = $derived(chipId ? (insts.find((x) => x.id === chipId)?.name ?? '') : '')
  const listingChip = $derived(!!chipId && listing.has(chipId))
  const chipError = $derived(chipId ? entries.find((e) => e.id === chipId)?.plugin?.presetsError : undefined)
  // "N of M": M is what the chip holds, before the filter (not the whole catalog, most of
  // which only its instrument's chip shows).
  const chipTotal = $derived(query.trim() ? visibleSounds(catalog, view, '', ctx).length : rows.length)
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
          oninput={() => void tick().then(() => ensureVisible(cursor, true))}
          onfocus={() => queueMicrotask(() => input && tips.hide(input))}
        />
        <span class="count engraved">{rows.length.toLocaleString()} of {chipTotal.toLocaleString()}{#if listingChip}&nbsp;· listing presets{:else if chipError}<span class="warn" title={chipError}>&nbsp;· presets not listed: {chipError}</span>{:else if app.state.sounds?.scanning}&nbsp;· scanning plugins{/if}</span>
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
                <span class="detail">{view.kind === 'all' || view.kind === 'category' ? slotText(e.id) : ''}{e.detail}{#if e.plugin?.lastError}<span class="warn" title={e.plugin.lastError}> ⚠ {e.plugin.lastError}</span>{/if}</span>
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
              {:else if view.kind === 'mine' && !query}Nothing in My Sounds yet: Save as… keeps what a part plays.
              {:else if chipId && !query}{listingChip ? `Listing ${shownName}'s presets…` : chipError ? `Could not list ${shownName}'s presets: ${chipError}` : `${shownName} has no presets.`}
              {:else}No sound matches “{query}”.{/if}
            </p>
          {/if}
        </div>
      </div>

      {#if selPatch}
        <SoundEdit bind:this={editor} patch={selPatch} onback={() => input?.focus()} />
      {/if}

      {#if saveForm && kp}
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions (Esc anywhere in the form closes it, not the browser) -->
        <form class="saveform" aria-label="Save as a new sound" onsubmit={(ev) => (ev.preventDefault(), saveAs())} onkeydown={saveFormKey}>
          <span class="engraved">Save as</span>
          <input bind:this={saveName} value={saveForm.name} oninput={(ev) => saveForm && ((saveForm.name = ev.currentTarget.value), (saveForm.replace = false))} class="mat-well" type="text" placeholder="Sound name" aria-label="New sound's name" spellcheck="false" use:tip={'sounds.save_as_name'} />
          {#if canPreset}
            <label class="check"><input type="checkbox" checked={saveForm.aupreset} onchange={(ev) => saveForm && ((saveForm.aupreset = ev.currentTarget.checked), (saveForm.replace = false))} use:tip={'sounds.save_preset'} /> .aupreset</label>
            {#if saveForm.aupreset}
              <select aria-label="Category of the preset" value={saveForm.category} onchange={(ev) => saveForm && (saveForm.category = ev.currentTarget.value as PatchCategory)} use:tip={'sounds.preset_category'}>
                {#each Object.entries(CATEGORY_LABELS) as [id, label] (id)}<option value={id}>{label}</option>{/each}
              </select>
            {/if}
          {/if}
          {#if saveForm.replace && clash}
            <span class="ask" role="alert">Replace ‘{presetFileName(saveForm.name)}’?</span>
            <HwButton tip="sounds.preset_replace" onclick={() => saveAs(true)}>Replace</HwButton>
            <HwButton tip="sounds.preset_replace_cancel" onclick={() => saveForm && ((saveForm.replace = false), saveName?.focus())}>Cancel</HwButton>
          {:else}
            <HwButton tip="sounds.save_as_confirm" onclick={() => saveAs()}>Save</HwButton>
            <HwButton tip="sounds.preset_cancel" onclick={closeSaveAs}>Cancel</HwButton>
          {/if}
        </form>
      {/if}

      <footer class="foot">
        {#if pick}<span class="now">Pick the sound for <b>{pick.title}</b></span>
        {:else if kp}
          <span class="now">{kp.name} plays <b>{instrumentName(kp, ctx, plugins.list, app.state.io.soundFontFile)}</b> · <b>{kp.sound?.name ?? kp.voiceName}</b>{#if kp.plugin && kp.plugin.status !== 'playing'}&nbsp;· {pluginStatusLine(kp.plugin, plugins.available).replace(/ ▾$/, '')}{/if}</span>
          {#if kp.soundEdited}<span class="edited" use:tip={'sounds.edited'}>edited</span>{/if}
        {/if}
        {#if auditioning}<HwButton tip="sounds.audition_stop" onclick={() => app.send({ type: 'stopSoundAudition' })}>■ Stop</HwButton>{/if}
        {#if kp}<HwButton tip="sounds.save_over" onclick={save}>Save</HwButton><HwButton tip="sounds.save" pressed={!!saveForm} onclick={() => (saveForm ? closeSaveAs() : openSaveAs())}>Save as…</HwButton>{/if}
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
  .saveform {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
  }
  .saveform input[type='text'] {
    flex: 1;
    min-width: 6rem;
    height: 2.2rem;
    padding: 0 0.6rem;
    border: 1px solid var(--well-edge);
    border-radius: 5px;
    color: var(--screen-ink);
  }
  .check {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    white-space: nowrap;
    font-size: var(--fs-small);
  }
  .ask {
    color: var(--accent);
    white-space: nowrap;
  }
  .saveform select {
    min-height: 2.2rem;
    padding: 0 0.45rem;
    border: 1px solid var(--well-edge);
    border-radius: 4px;
    background: var(--screen-bg);
    color: var(--screen-ink);
    font-size: 0.9rem;
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
  .edited {
    padding: 1px 6px;
    border: 1px solid color-mix(in srgb, var(--accent) 70%, transparent);
    border-radius: 3px;
    color: var(--accent);
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.72rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
</style>
