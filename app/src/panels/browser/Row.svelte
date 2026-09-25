<!--
  One style in the browser list: favourite star, loaded/Track marks, name, folder, tempo,
  time signature, section lamps, SFF version and the preview / load-at-next-bar button.
  Fixed height (the list is virtualised), and every column has a fixed width so nothing
  shifts while the band plays.
-->
<script lang="ts">
  import type { LibraryEntry } from '../../lib/api/types'
  import { formatTempo } from '../../lib/format'
  import { brightness } from '../../lib/leds'
  import { clock } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import Lamps from './Lamps.svelte'

  let {
    entry,
    id,
    top,
    active,
    loaded,
    mark,
    favourite,
    colours,
    action,
    auditioning,
    queued,
    onload,
    onfavourite,
    onpreview,
    onhover,
  }: {
    entry: LibraryEntry
    /** DOM id, for the filter's aria-activedescendant. */
    id: string
    top: number
    active: boolean
    loaded: boolean
    /** ‹ / ›: < Track / Track > would load this one. */
    mark: '' | '‹' | '›'
    favourite: boolean
    colours: { intro: string; main: string; brk: string; ending: string }
    /** What the row's button does now: preview (stopped), queue (playing), or none. */
    action: 'preview' | 'queue' | null
    auditioning: boolean
    queued: boolean
    onload: () => void
    onfavourite: () => void
    onpreview: () => void
    onhover: () => void
  } = $props()

  const error = $derived(entry.status === 'error')
  const pending = $derived(entry.status === 'pending')
  const flash = $derived(queued ? brightness({ level: 'bright', anim: 'flash' }, clock.beats) : 1)
</script>

<!-- svelte-ignore a11y_click_events_have_key_events (the filter field drives the list from the keyboard: ↑/↓, Enter) -->
<div
  class="row"
  class:active
  class:loaded
  class:error
  role="option"
  tabindex="-1"
  aria-selected={active}
  {id}
  style:transform="translateY({top}px)"
  use:tip={error ? 'browser.row_error' : 'browser.row'}
  onclick={onload}
  onpointerenter={onhover}
>
  <button
    type="button"
    class="star"
    class:on={favourite}
    tabindex="-1"
    aria-label={favourite ? 'Unstar' : 'Star'}
    aria-pressed={favourite}
    use:tip={'browser.favourite'}
    onclick={(e) => (e.stopPropagation(), onfavourite())}>{favourite ? '★' : '☆'}</button
  >
  <span class="mark" aria-hidden="true">{loaded ? '▶' : mark}</span>
  <span class="name">{entry.name}</span>
  {#if error}
    <span class="why">{entry.error ?? 'unreadable'}</span>
  {:else}
    <span class="folder">{entry.folder}</span>
    <span class="num tempo">{pending ? '…' : formatTempo(entry.tempo)}</span>
    <span class="num">{pending ? '' : entry.timeSignature ? `${entry.timeSignature[0]}/${entry.timeSignature[1]}` : ''}</span>
    <span class="secs">{#if !pending}<Lamps summary={entry.sections} {colours} />{/if}</span>
    <span class="sff">{#if entry.format}<b>{entry.format}</b>{:else}<i>{pending ? '…' : '—'}</i>{/if}</span>
  {/if}
  <span class="act">
    {#if action === 'preview' && !error}
      <button
        type="button"
        class="pv"
        class:on={auditioning}
        tabindex="-1"
        aria-label={auditioning ? 'Stop preview' : 'Preview'}
        use:tip={auditioning ? 'browser.preview_stop' : 'browser.preview'}
        onclick={(e) => (e.stopPropagation(), onpreview())}>{auditioning ? '■' : '▶'}</button
      >
    {:else if action === 'queue' && !error && !loaded}
      <button
        type="button"
        class="q"
        class:on={queued}
        tabindex="-1"
        use:tip={'browser.queue'}
        style:--f={flash}
        onclick={(e) => (e.stopPropagation(), onpreview())}>{queued ? 'Queued' : 'Next bar'}</button
      >
    {/if}
  </span>
</div>

<style>
  .row {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    height: var(--row);
    display: grid;
    grid-template-columns: var(--cols);
    align-items: center;
    column-gap: 0.6rem;
    padding: 0 0.5rem 0 0.25rem;
    border-bottom: 1px solid rgb(255 255 255 / 0.035);
    color: var(--screen-ink);
    cursor: pointer;
    white-space: nowrap;
    outline: none;
    contain: layout paint;
  }
  .row:hover {
    background: rgb(255 255 255 / 0.045);
  }
  .row.active {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .name,
  .folder,
  .why {
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }
  .name {
    font-weight: 500;
  }
  .loaded .name,
  .mark {
    color: var(--accent);
  }
  .loaded .name {
    text-shadow: 0 0 6px color-mix(in srgb, var(--accent) 45%, transparent);
  }
  .mark {
    text-align: center;
    font-size: 0.8rem;
  }
  .folder {
    color: var(--screen-dim);
    font-size: var(--fs-small);
  }
  .error .name {
    color: var(--danger);
  }
  .why {
    grid-column: 4 / 9;
    color: var(--danger);
    font-size: var(--fs-small);
    opacity: 0.85;
  }
  .num {
    text-align: right;
    /* Fixed-width column: clip rather than spill into the next one. */
    overflow: hidden;
    min-width: 0;
    font-family: var(--font-display);
    font-size: 0.95rem;
  }
  .sff {
    text-align: center;
  }
  .sff b,
  .sff i {
    font-family: var(--font-display);
    font-weight: 600;
    font-style: normal;
    font-size: 0.72rem;
    letter-spacing: 0.06em;
  }
  .sff b {
    padding: 1px 4px;
    border: 1px solid rgb(223 244 255 / 0.25);
    border-radius: 3px;
    color: var(--screen-ink);
  }
  .sff i {
    color: var(--screen-dim);
  }
  button {
    border: 0;
    background: none;
    padding: 0;
    color: inherit;
    height: var(--row);
  }
  .star {
    width: 100%;
    color: var(--screen-dim);
    font-size: 1rem;
    opacity: 0.55;
  }
  .row:hover .star,
  .row.active .star,
  .star.on {
    opacity: 1;
  }
  .star.on {
    color: var(--accent);
  }
  .act {
    display: flex;
    justify-content: flex-end;
  }
  .pv {
    width: 2.2rem;
    color: var(--screen-dim);
    font-size: 0.8rem;
  }
  .pv:hover,
  .pv.on {
    color: var(--screen-ink);
  }
  .pv.on {
    color: var(--accent);
  }
  .q {
    padding: 0 0.3rem;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.78rem;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--screen-dim);
    opacity: 0;
  }
  .row:hover .q,
  .row.active .q {
    opacity: 1;
  }
  .q:hover {
    color: var(--screen-ink);
  }
  .q.on {
    opacity: var(--f);
    color: var(--accent);
  }
</style>
