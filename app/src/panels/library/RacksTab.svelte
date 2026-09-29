<!--
  Library › Racks (docs/racks.md; the wireframe's `browserRacks` and `rackDetails`): the
  live rack (`AppState.liveRack`: what's under your hands, autosaved), your racks, and the
  saved racks the engine reports as needing attention (a part's plugin is missing:
  `plugins.needsAttention`), with a search and the "Needs attention" filter.

  ┌ [Search racks…] [⚠ Needs attention (1)]                          [+ New rack] ┐
  │ LOADED NOW  ▶ Ballad ●                                               Modified │
  │ MY RACKS      Ballad        Stage Grand + Silk Strings                   A1 │
  │               Evening       …                                               │
  ├ Rack details ─────────────────────────────────────────────────────────────────┤
  │ Name [Ballad        ]   Quick Racks A1   Split F#2   Harmony/Arp Off           │
  │ [Load] [Duplicate] [Delete…]   → Delete Ballad? Quick Rack A1 will be empty.   │
  └───────────────────────────────────────────────────────────────────────────────┘

  A click on one of your racks selects it and shows its details; a double-click, or Load in
  the details, loads it (through the unsaved-changes guard, asked in the docked Rack panel).
  The details rename it (`renameRack`), duplicate it (`duplicateRack`) and delete it
  (`deleteRack`, after an inline confirm; refused for the loaded rack). + New rack sends
  `newRack`, with the same guard. With nothing selected, the details are the loaded rack's.

  Style racks (docs/racks.md "Styles and OTS", journey 4): the loaded style's OTS buttons
  1–4 (`ots.settings`, `ots.racks`), each with a select, "Style's own" or one of your racks
  (`setOtsRack` / `clearOtsRack`, kept per style), and Load (`recallOts`).

  │ STYLE RACKS: Soul Ballad (OTS buttons 1–4)                                    │
  │ ▶ OTS 1 · Soul Ballad's own   Tine EP · Strings…   [Style's own ▾] [Load] Style │
  │   OTS 2 · Ballad Pad          Grand + Pad          [Ballad Pad  ▾] [Load] Mine  │

  Split, Harmony/Arp and the transpose are only in the state for the live rack, so they show
  for the loaded rack; Quick Racks state carries only the bank on view, so the labels (A1)
  are that bank's.
-->
<script lang="ts">
  import { noteName } from '../../lib/api/constants'
  import { bankLetter } from '../../lib/api/quick-racks'
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import { quickButtonsOf, searchRacks } from './model'
  import { libraryNav } from './nav.svelte'

  const s = $derived(app.state)
  const live = $derived(s.liveRack)
  const parts = $derived(s.keyboardParts)
  const attention = $derived(s.plugins.needsAttention)
  // The filter, while there is anything to filter to.
  const filtered = $derived(libraryNav.attention && attention.length > 0)
  const query = $derived(libraryNav.rackQuery.trim())
  const PARTS = ['Right 1', 'Right 2', 'Right 3', 'Left']
  const partSound = (i: number) => {
    const p = parts[i]
    if (!p?.on && !p?.playsBass) return '—'
    return `${p.plugin?.missing ? '⚠ ' : ''}${p.sound?.name ?? p.voiceName}`
  }
  const missingParts = $derived(parts.flatMap((p, i) => (p.plugin?.missing ? [PARTS[i]] : [])))
  const shown = $derived(searchRacks(s.racks, libraryNav.rackQuery))
  const shownAttention = $derived(query ? attention.filter((r) => r.name.toLowerCase().includes(query.toLowerCase())) : attention)

  // The rack whose details show: the one clicked, else the loaded one (if it is saved).
  const selId = $derived(libraryNav.rack ?? live.id)
  const sel = $derived(s.racks.find((r) => r.id === selId) ?? null)
  const loaded = $derived(!!sel && sel.id === live.id)
  const buttons = $derived(sel ? quickButtonsOf(s.quickRacks, sel.id) : [])
  const bank = $derived(bankLetter(s.quickRacks.bank))

  // A rack that was deleted elsewhere (a Quick Rack, the engine) lets its selection go.
  $effect(() => {
    if (libraryNav.rack && !s.racks.some((r) => r.id === libraryNav.rack)) libraryNav.rack = null
  })

  // Delete…: the inline confirm, for the rack it was asked about.
  let confirming = $state<string | null>(null)
  const asking = $derived(!!sel && confirming === sel.id)

  // A rack that became the loaded one can't be deleted: its confirm goes.
  $effect(() => {
    if (confirming && confirming === live.id) confirming = null
  })

  function select(id: string | null) {
    libraryNav.rack = id
    confirming = null
  }
  const load = (id: string) => app.send({ type: 'loadRack', id })

  // Style racks: what the loaded style's OTS buttons load (docs/racks.md "Styles and OTS").
  const ots = $derived(s.ots)
  /** The OTS rack select: '' is the style's own. */
  function pickOtsRack(i: number, id: string) {
    app.send(id ? { type: 'setOtsRack', index: i, id } : { type: 'clearOtsRack', index: i })
  }

  // The name as typed, until the state has the new name (or refused it: a name another
  // rack has); null shows the state's.
  let draft = $state<string | null>(null)
  let sentAt = -1
  function rename() {
    const name = draft?.trim()
    if (!sel || !name || name === sel.name) {
      draft = null
      return
    }
    sentAt = s.message?.seq ?? 0
    app.send({ type: 'renameRack', id: sel.id, name })
  }
  $effect(() => {
    void sel?.id
    void sel?.name
    draft = null
  })
  $effect(() => {
    const m = s.message
    if (m?.error && m.seq > sentAt && sentAt >= 0) {
      sentAt = -1
      draft = null
    }
  })
  function nameKey(e: KeyboardEvent) {
    const el = e.currentTarget as HTMLInputElement
    if (e.key === 'Enter') {
      e.preventDefault()
      el.blur()
    } else if (e.key === 'Escape') {
      // Esc puts the name back; it doesn't leave Library.
      e.preventDefault()
      e.stopPropagation()
      draft = null
      el.blur()
    }
  }
  function askDelete() {
    if (sel && !loaded) confirming = sel.id
  }
  function loadSelected() {
    if (sel) load(sel.id)
  }

  // Duplicate selects the copy: the id that wasn't there before.
  let before: Set<string> | null = null
  function duplicate() {
    if (!sel) return
    before = new Set(s.racks.map((r) => r.id))
    const at = s.message?.seq ?? 0
    app.send({ type: 'duplicateRack', id: sel.id })
    dupAt = at
  }
  // A refused duplicate stops waiting for the copy.
  let dupAt = -1
  $effect(() => {
    const m = s.message
    if (m?.error && dupAt >= 0 && m.seq > dupAt) {
      dupAt = -1
      before = null
    }
  })
  $effect(() => {
    const racks = s.racks
    if (!before) return
    const copy = racks.find((r) => !before!.has(r.id))
    if (copy) {
      before = null
      select(copy.id)
    }
  })

  function remove() {
    if (!sel || loaded) return
    app.send({ type: 'deleteRack', id: sel.id })
    confirming = null
    libraryNav.rack = null
  }
  function searchKey(e: KeyboardEvent) {
    if (e.key === 'Escape' && libraryNav.rackQuery) {
      e.preventDefault()
      e.stopPropagation()
      libraryNav.rackQuery = ''
    }
  }
</script>

<div class="racks">
  <div class="filters">
    <input
      bind:value={libraryNav.rackQuery}
      class="mat-well search"
      type="search"
      placeholder="Search racks…"
      spellcheck="false"
      autocomplete="off"
      aria-label="Search racks"
      use:tip={'library.racks_search'}
      onkeydown={searchKey}
    />
    {#if attention.length}
      <button type="button" class="chip" class:on={libraryNav.attention} aria-pressed={libraryNav.attention} use:tip={'library.racks_attention'} onclick={() => (libraryNav.attention = !libraryNav.attention)}>⚠ Needs attention ({attention.length})</button>
    {/if}
    <span class="grow"></span>
    <button type="button" class="act mat-raised" use:tip={'library.rack_new'} onclick={() => app.send({ type: 'newRack' })}>+ New rack</button>
  </div>

  <div class="screen mat-screen">
    <div class="scroller">
      {#if !filtered && !query}
        <div class="rhead"><span>Loaded now</span><span>autosaved; comes back on boot</span></div>
        <div role="listbox" aria-label="Live rack">
          <div class="row live" class:sel={!sel} role="option" tabindex="0" aria-selected={!sel} use:tip={'library.rack_live'} onclick={() => select(null)} onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); select(null) } }}>
            <span class="mark" aria-hidden="true">▶</span>
            <span class="name">{live.name}{#if live.modified}<span class="mod" title="Modified"> ●</span>{/if}<span class="sub">{PARTS.map((_, i) => partSound(i)).join(' · ')}</span></span>
            <span class="badge" class:warn={missingParts.length > 0}>{missingParts.length ? '⚠ fix' : live.modified ? 'Modified' : 'Live'}</span>
          </div>
        </div>
      {/if}

      <div class="rhead"><span>{filtered ? 'Needs attention' : 'My racks'}</span><span>{attention.length ? 'a part plays a plugin that is missing' : 'click for details · double-click loads'}</span></div>
      {#if shownAttention.length}
        <div role="listbox" aria-label="Racks that need attention">
          {#each shownAttention as r (r.id)}
            <div class="row" role="option" tabindex="-1" aria-selected="false" use:tip={'library.rack_attention'}>
              <span class="mark" aria-hidden="true"></span>
              <span class="name">{r.name}<span class="sub">⚠ {r.parts.map((i) => PARTS[i]).join(', ')} {r.parts.length === 1 ? 'needs' : 'need'} a new sound</span></span>
              <span class="badge warn">⚠ fix</span>
            </div>
          {/each}
        </div>
      {/if}
      {#if !filtered}
        {#if shown.length}
          <div role="listbox" aria-label="My racks">
            {#each shown as r (r.id)}
              {@const on = quickButtonsOf(s.quickRacks, r.id).join(' ')}
              <button
                type="button"
                class="row pick"
                role="option"
                aria-selected={r.id === sel?.id}
                class:sel={r.id === sel?.id}
                use:tip={'library.rack_row'}
                onclick={() => select(r.id)}
                ondblclick={() => load(r.id)}
              >
                <span class="mark" aria-hidden="true">{r.id === live.id ? '▶' : ''}</span>
                <span class="name">{r.name}{#if r.id === live.id && live.modified}<span class="mod" title="Modified"> ●</span>{/if}<span class="sub">{r.parts.filter((_, i) => r.on[i]).join(' + ')}</span></span>
                <span class="badge" class:warn={r.needsAttention}>{r.needsAttention ? '⚠ fix' : on}</span>
              </button>
            {/each}
          </div>
        {:else if query}
          <p class="coming">No racks match “{query}”.</p>
        {:else}
          <p class="coming">No racks yet. Press Store, then a Quick Rack button, to save the live rack ({live.name}) and put it there.</p>
        {/if}
        {#if !query && ots.settings.length}
          <div class="rhead"><span>Style racks: {s.style.name} (OTS buttons 1–4)</span><span>swap in your own per style</span></div>
          <div role="list" aria-label="Style racks">
            {#each ots.settings.slice(0, 4) as o, i (i)}
              {@const sr = ots.racks[i]}
              {@const mine = sr && sr.rack !== null && !sr.missing ? s.racks.find((r) => r.id === sr.rack) : undefined}
              <div class="row slot" role="listitem" class:sel={ots.applied === i + 1}>
                <span class="mark" aria-hidden="true">{ots.applied === i + 1 ? '▶' : ''}</span>
                <span class="name"
                  ><b class="ots">OTS {i + 1}</b> · {mine ? mine.name : `${s.style.name}'s own`}<span class="sub"
                    >{#if sr?.missing}⚠ the rack chosen is gone: the style's own plays ·
                    {/if}{mine ? mine.parts.filter((_, j) => mine.on[j]).join(' + ') : o.parts.map((p) => (p.on ? p.voiceName : '—')).join(' · ')}</span
                  ></span
                >
                <select class="slotsel" aria-label="OTS {i + 1} rack" value={mine ? mine.id : ''} disabled={ots.racksReadOnly} use:tip={'ots.rack'} onchange={(e) => pickOtsRack(i, e.currentTarget.value)}>
                  <option value="">Style's own</option>
                  {#each s.racks as r (r.id)}<option value={r.id}>{r.name}</option>{/each}
                </select>
                <button type="button" class="act mat-raised" use:tip={'library.style_rack_load'} onclick={() => app.send({ type: 'recallOts', index: i })}>Load</button>
                <span class="badge" class:mine={!!mine}>{mine ? 'Mine' : 'Style'}</span>
              </div>
            {/each}
          </div>
          {#if ots.racksReadOnly}<p class="muted">Style racks can't be changed now (see the message at start).</p>{/if}
        {/if}
      {/if}
    </div>
  </div>

  <div class="details" aria-label="Rack details" role="group">
    <h3 class="engraved">Rack details</h3>
    {#if sel}
      <div class="line">
        <label class="namefield"><span class="k">Name</span>
          <input class="mat-well" type="text" value={draft ?? sel.name} spellcheck="false" autocomplete="off" aria-label="Rack name" use:tip={'library.rack_name'} oninput={(e) => (draft = e.currentTarget.value)} onchange={rename} onkeydown={nameKey} />
        </label>
        <span><span class="k">Quick Racks</span> {buttons.length ? buttons.join(', ') : `not in bank ${bank}`}</span>
        {#if loaded}
          <span><span class="k">Split</span> {noteName(s.chord.split)}</span>
          <span><span class="k">Harmony/Arp</span> {s.harmonyArp.on ? s.harmonyArp.typeName : 'Off'}</span>
        {:else}
          <span class="muted"><span class="k">Split, Harmony/Arp</span> show once it's loaded</span>
        {/if}
      </div>
      <div class="line"><span><span class="k">Parts</span> {sel.parts.map((p, i) => `${PARTS[i]}: ${sel.on[i] ? p : '—'}`).join(' · ')}</span></div>
      {#if asking}
        <div class="confirm" role="alert">
          <span>Delete <b>{sel.name}</b>? {buttons.length ? `Quick Rack${buttons.length === 1 ? '' : 's'} ${buttons.join(', ')} will be emptied, and any other button holding it.` : 'Any Quick Rack button holding it will be emptied.'}</span>
          <button type="button" class="act mat-raised danger" use:tip={'library.rack_delete_confirm'} onclick={remove}>Delete</button>
          <button type="button" class="act mat-raised" use:tip={'library.rack_delete_cancel'} onclick={() => (confirming = null)}>Cancel</button>
        </div>
      {:else}
        <div class="acts">
          {#if !loaded}<button type="button" class="act mat-raised primary" use:tip={'library.rack_load'} onclick={loadSelected}>Load</button>{/if}
          <button type="button" class="act mat-raised" use:tip={'library.rack_duplicate'} onclick={duplicate}>Duplicate</button>
          <button type="button" class="act mat-raised" aria-disabled={loaded} use:tip={'library.rack_delete'} onclick={askDelete}>Delete…</button>
          {#if loaded}<small class="muted">It's loaded: load another rack to delete it.</small>{/if}
        </div>
      {/if}
    {:else}
      <div class="line">
        <span><span class="k">Name</span> {live.name}</span>
        <span><span class="k">Split</span> {noteName(s.chord.split)}</span>
        <span><span class="k">Harmony/Arp</span> {s.harmonyArp.on ? s.harmonyArp.typeName : 'Off'}</span>
        <span><span class="k">Transpose</span> {s.chord.transposeKeyboard > 0 ? '+' : ''}{s.chord.transposeKeyboard}</span>
        <span><span class="k">State</span> {live.modified ? 'modified since loaded' : 'as loaded'}</span>
      </div>
      <p class="muted">Never saved: Save rack in the Rack panel makes it one of your racks, to rename, duplicate or put on a Quick Rack button.</p>
    {/if}
    {#if missingParts.length}<p class="warn">⚠ {missingParts.join(', ')} {missingParts.length === 1 ? 'is' : 'are'} silent: {missingParts.length === 1 ? 'its' : 'their'} plugin is missing. Pick a new sound in Sounds; the part keeps its mix.</p>{/if}
  </div>
</div>

<style>
  .racks {
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
    gap: 0.4rem;
  }
  .search {
    flex: 0 1 18rem;
    min-width: 8rem;
    height: 2.2rem;
    padding: 0 0.8rem;
    border: 1px solid var(--well-edge);
    border-radius: 6px;
    color: var(--screen-ink);
    font-size: 1rem;
  }
  .grow {
    flex: 1;
  }
  .chip {
    min-height: 2rem;
    padding: 0 0.8rem;
    border: 1px solid color-mix(in srgb, var(--danger) 60%, transparent);
    border-radius: 1rem;
    background: none;
    color: var(--ink);
    font-family: var(--font-display);
    font-weight: 600;
  }
  .chip.on {
    border-color: var(--danger);
    background: color-mix(in srgb, var(--danger) 20%, transparent);
  }
  .act {
    min-height: 2rem;
    padding: 0 0.75rem;
    border-radius: 5px;
    color: var(--ink);
    font-family: var(--font-display);
    font-weight: 600;
    white-space: nowrap;
  }
  .act.primary {
    color: var(--accent);
  }
  .act.danger {
    color: var(--danger);
  }
  .act[aria-disabled='true'] {
    opacity: 0.4;
    cursor: default;
  }
  .screen {
    --accent: #f3b843;
    --danger: #ff7a6b;
    flex: 1;
    min-height: 6rem;
    display: flex;
    border-radius: 8px;
    overflow: hidden;
  }
  .scroller {
    flex: 1;
    overflow-y: auto;
    color: var(--screen-ink);
  }
  .rhead {
    display: flex;
    justify-content: space-between;
    gap: 0.8rem;
    padding: 0.45rem 0.7rem 0.3rem;
    color: var(--screen-dim);
    font-family: var(--font-display);
    font-size: 0.72rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .row {
    display: grid;
    grid-template-columns: 1rem minmax(0, 1fr) auto;
    align-items: center;
    gap: 0.6rem;
    padding: 0.4rem 0.7rem;
    border-top: 1px solid rgb(255 255 255 / 0.05);
    outline: none;
  }
  button.row.pick {
    width: 100%;
    border: none;
    border-top: 1px solid rgb(255 255 255 / 0.05);
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .badge:empty {
    display: none;
  }
  .row.slot {
    grid-template-columns: 1rem minmax(0, 1fr) auto auto auto;
  }
  .ots {
    font-family: var(--font-display);
    font-size: 0.85em;
  }
  .slotsel {
    max-width: 11rem;
    min-height: 1.9rem;
    font: inherit;
    font-size: 0.85rem;
  }
  .badge.mine {
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
    color: var(--accent);
  }
  .row.sel {
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .row.live:not(.sel) {
    background: color-mix(in srgb, var(--accent) 6%, transparent);
  }
  .row:focus-visible {
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .mark,
  .mod {
    color: var(--accent);
  }
  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
  }
  .sub {
    display: block;
    color: var(--screen-dim);
    font-size: var(--fs-small);
    font-weight: 400;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .badge {
    padding: 1px 4px;
    border: 1px solid rgb(223 244 255 / 0.25);
    border-radius: 3px;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.7rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .badge.warn {
    border: 2px solid var(--danger);
  }
  .warn {
    color: var(--danger);
  }
  .coming {
    margin: 0.6rem 0.7rem;
    color: var(--screen-dim);
    font-size: var(--fs-small);
  }
  .details {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    padding-top: 0.5rem;
    border-top: 1px solid var(--line);
  }
  .details h3 {
    margin: 0;
    font-size: 0.72rem;
  }
  .details p {
    margin: 0;
    font-size: var(--fs-small);
  }
  .line {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4rem 1.2rem;
    font-size: var(--fs-small);
  }
  .namefield {
    display: flex;
    align-items: center;
    gap: 0.45rem;
  }
  .namefield input {
    width: 14rem;
    max-width: 40vw;
    height: 2rem;
    padding: 0 0.6rem;
    border: 1px solid var(--well-edge);
    border-radius: 5px;
    color: var(--screen-ink);
    font: inherit;
    font-weight: 600;
  }
  .acts,
  .confirm {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.45rem;
  }
  .confirm {
    padding: 0.4rem 0.55rem;
    border-radius: 6px;
    background: color-mix(in srgb, var(--danger) 12%, transparent);
    font-size: var(--fs-small);
  }
  .confirm span {
    flex: 1 1 16rem;
  }
  .k {
    color: var(--muted);
  }
  .muted {
    color: var(--muted);
  }
</style>
