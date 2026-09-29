<!--
  Library › Racks (docs/racks.md; the wireframe's `browserRacks`): the racks there are
  today, read-only. The live rack (`AppState.liveRack`: what's under your hands, autosaved)
  and the saved racks the engine reports as needing attention (a part's plugin is
  missing: `plugins.needsAttention`), with the "Needs attention" filter.

  Loading, saving and listing every saved rack come with the rack commands (docs/racks.md,
  "Order of work" item 5); until then their buttons say so rather than pretend.
-->
<script lang="ts">
  import { noteName } from '../../lib/api/mock'
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import { libraryNav } from './nav.svelte'

  const s = $derived(app.state)
  const live = $derived(s.liveRack)
  const parts = $derived(s.keyboardParts)
  const attention = $derived(s.plugins.needsAttention)
  // The filter, while there is anything to filter to.
  const filtered = $derived(libraryNav.attention && attention.length > 0)
  const PARTS = ['Right 1', 'Right 2', 'Right 3', 'Left']
  const partSound = (i: number) => {
    const p = parts[i]
    if (!p?.on && !p?.playsBass) return '—'
    return `${p.plugin?.missing ? '⚠ ' : ''}${p.sound?.name ?? p.voiceName}`
  }
  const missingParts = $derived(parts.flatMap((p, i) => (p.plugin?.missing ? [PARTS[i]] : [])))
  const COMING = ['+ New rack', 'Save rack', 'Save as…', 'Revert']
</script>

<div class="racks">
  <div class="filters">
    {#if attention.length}
      <button type="button" class="chip" class:on={libraryNav.attention} aria-pressed={libraryNav.attention} use:tip={'library.racks_attention'} onclick={() => (libraryNav.attention = !libraryNav.attention)}>⚠ Needs attention ({attention.length})</button>
    {/if}
    <span class="grow"></span>
    {#each COMING as label (label)}
      <button type="button" class="act" disabled use:tip={'library.rack_coming'}>{label}</button>
    {/each}
  </div>

  <div class="screen mat-screen">
    <div class="scroller">
      {#if !filtered}
        <div class="rhead"><span>Loaded now</span><span>autosaved; comes back on boot</span></div>
        <div role="listbox" aria-label="Live rack">
          <div class="row sel" role="option" tabindex="0" aria-selected="true" use:tip={'library.rack_live'}>
            <span class="mark" aria-hidden="true">▶</span>
            <span class="name">{live.name}{#if live.modified}<span class="mod" title="Modified"> ●</span>{/if}<span class="sub">{PARTS.map((_, i) => partSound(i)).join(' · ')}</span></span>
            <span class="badge" class:warn={missingParts.length > 0}>{missingParts.length ? '⚠ fix' : live.modified ? 'Modified' : 'Live'}</span>
          </div>
        </div>
      {/if}

      <div class="rhead"><span>{filtered ? 'Needs attention' : 'My racks'}</span><span>{attention.length ? 'a part plays a plugin that is missing' : ''}</span></div>
      {#if attention.length}
        <div role="listbox" aria-label="Racks that need attention">
          {#each attention as r (r.id)}
            <div class="row" role="option" tabindex="-1" aria-selected="false" use:tip={'library.rack_attention'}>
              <span class="mark" aria-hidden="true"></span>
              <span class="name">{r.name}<span class="sub">⚠ {r.parts.map((i) => PARTS[i]).join(', ')} {r.parts.length === 1 ? 'needs' : 'need'} a new sound</span></span>
              <span class="badge warn">⚠ fix</span>
            </div>
          {/each}
        </div>
      {/if}
      {#if !filtered}
        <p class="coming">Your saved racks list here once racks can be saved and loaded (coming next). Until then the live rack keeps everything: it autosaves, and comes back when yahaha starts.</p>
      {/if}
    </div>
  </div>

  <div class="details">
    <h3 class="engraved">Rack details</h3>
    <div class="line">
      <span><span class="k">Name</span> {live.name}</span>
      <span><span class="k">Split</span> {noteName(s.chord.split)}</span>
      <span><span class="k">Harmony/Arp</span> {s.harmonyArp.on ? s.harmonyArp.typeName : 'Off'}</span>
      <span><span class="k">Transpose</span> {s.chord.transposeKeyboard > 0 ? '+' : ''}{s.chord.transposeKeyboard}</span>
      <span><span class="k">State</span> {live.modified ? 'modified since loaded' : 'as loaded'}</span>
    </div>
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
  button.act {
    min-height: 2rem;
    border: 1px dashed var(--line-strong);
    border-radius: 4px;
    background: none;
    padding: 0 0.65rem;
    color: var(--muted);
    font-family: var(--font-display);
    font-weight: 600;
    cursor: not-allowed;
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
  .row.sel {
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    box-shadow: inset 2px 0 0 var(--accent);
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
    gap: 0.35rem;
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
    gap: 0.4rem 1.2rem;
    font-size: var(--fs-small);
  }
  .k {
    color: var(--muted);
  }
</style>
