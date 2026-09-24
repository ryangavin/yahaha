<!--
  The help footer: a fixed bar along the bottom of the window. Hover or tab to any
  control and its catalog entry shows here (title, what it does, the Genos name, the key
  and where it is on the Launchkey), so nothing pops up over the instrument while you
  play. On the right, a small screen shows the Launchkey control pressed last and what it
  does now.

  - Compact (default): two lines, always the same height, so the stage never shifts.
  - Help mode (`?`): the footer grows to the full entry (the whole description, every
    Launchkey place, the terminal keys) and keeps the last one pinned while you try the
    control. The stage gives up the height once, when help mode toggles.
  - "Pop-up": opt in to the old floating tips as well (remembered).

  Screen readers: the focused control points `aria-describedby` at a hidden plain-text
  copy of its own entry (not whatever the pointer is over).
-->
<script lang="ts">
  import { untrack } from 'svelte'
  import { TIPS, keyLabel, type Tip } from '../../help/tooltips'
  import { app } from '../store.svelte'
  import { lastControl, type LastControl } from './lastControl'
  import { tip, tips, TOOLTIP_ID } from './tip.svelte'

  const key = $derived(tips.shown)
  const t = $derived(key ? TIPS[key] : null)
  const keys = $derived(t ? (t.app_keys ?? t.keys) : [])
  const places = $derived(t?.launchkey ? t.launchkey.split('; ') : [])

  // Decode the last hardware message when it changes, against the state it came with.
  let last = $state<LastControl | null>(null)
  let seen = 0
  $effect(() => {
    const packed = app.state.io.lastControl
    if (packed === seen) return
    seen = packed
    const d = untrack(() => lastControl(packed, app.state))
    if (d) last = d
  })
  const connected = $derived(app.state.pads.connected)

  function plain(t: Tip): string {
    const k = t.app_keys ?? t.keys
    return [
      t.body,
      t.genos && t.genos !== t.title ? `Genos: ${t.genos}.` : '',
      k.length ? `Key: ${k.map(keyLabel).join(' or ')}.` : '',
      t.launchkey ? `Launchkey: ${t.launchkey}.` : '',
    ].filter(Boolean).join(' ')
  }
</script>

<footer class="help-footer mat-chassis" class:expanded={tips.help} aria-label="Help">
  <div class="entry">
    {#if t}
      <div class="head">
        <strong>{t.title}</strong>
        {#if t.genos && t.genos !== t.title}<span class="genos">Genos: {t.genos}</span>{/if}
      </div>
      <p class="body">{t.body}</p>
      <dl class="meta">
        <dt>Key</dt>
        <dd>
          {#if keys.length}
            {#each keys as k, i (k)}{#if i > 0}<span class="dim"> / </span>{/if}<kbd>{keyLabel(k)}</kbd>{/each}
            {#if tips.help && t.app_keys}<span class="dim"> (terminal: {t.keys.map(keyLabel).join(' / ')})</span>{/if}
          {:else}<span class="dim">none</span>{/if}
        </dd>
        <dt>Launchkey</dt>
        {#if tips.help}
          <dd>
            {#each places as place, i (place)}{#if i > 0}<br />{/if}{place}{:else}<span class="dim">not on the Launchkey</span>{/each}
          </dd>
        {:else}
          <dd class="clip">
            {#if places.length}{places[0]}{#if places.length > 1}<span class="dim"> +{places.length - 1} more</span>{/if}
            {:else}<span class="dim">not on it</span>{/if}
          </dd>
        {/if}
      </dl>
    {:else if tips.help}
      <p class="idle">
        <strong>Help mode.</strong> Hover over or tab to any control: its full entry stays here while
        you try it. Press <kbd>?</kbd> again to shrink the footer.
      </p>
    {:else}
      <p class="idle">Hover any control to learn what it does · <kbd>?</kbd> help mode</p>
    {/if}
  </div>

  <div class="lk">
    <span class="engraved lk-label">Launchkey</span>
    <div class="screen mat-screen">
      {#if last}
        <span class="where">{last.where}</span>
        <span class="what">{last.what || '—'}</span>
      {:else}
        <span class="what dim-screen">{connected ? 'press a pad or button' : 'not connected'}</span>
      {/if}
    </div>
  </div>

  <button
    type="button"
    class="float mat-raised"
    class:pressed={tips.floating}
    aria-pressed={tips.floating}
    use:tip={'app.floating_tips'}
    onclick={() => tips.setFloating(!tips.floating)}
  >
    Pop-up
  </button>

  <div id={TOOLTIP_ID} class="visually-hidden">{tips.focused ? plain(TIPS[tips.focused]) : ''}</div>
</footer>

<style>
  .help-footer {
    position: relative;
    /* Above a modal's scrim, so the browser's controls are explained too. */
    z-index: 45;
    flex: none;
    display: flex;
    align-items: stretch;
    gap: 0.9rem;
    height: var(--help-footer-h);
    padding: 0.35rem 0.5rem 0.35rem 0.8rem;
    border-radius: 8px;
    font-size: var(--fs-small);
    line-height: 1.3;
    overflow: hidden;
  }
  .help-footer.expanded {
    height: var(--help-footer-h-help);
    padding-block: 0.55rem;
  }

  .entry {
    flex: 1;
    min-width: 0;
    display: grid;
    grid-template-columns: minmax(7.5rem, 13rem) minmax(0, 1fr) minmax(9rem, 17rem);
    gap: 0 1.1rem;
    align-items: center;
  }
  .expanded .entry {
    grid-template-columns: minmax(8rem, 13rem) minmax(0, 1fr) minmax(11rem, 24rem);
    align-items: start;
    overflow: auto;
  }
  .expanded .head strong {
    font-size: 1.2rem;
    white-space: normal;
  }
  .expanded .genos {
    white-space: normal;
  }
  .expanded .body {
    display: block;
    max-width: 70ch;
  }
  .head {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .head strong,
  .idle strong {
    font-family: var(--font-display);
    font-size: 1.05rem;
    font-weight: 600;
    letter-spacing: 0.01em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .genos {
    color: var(--accent);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .body {
    margin: 0;
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
  }
  .meta {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 0.1rem 0.5rem;
    margin: 0;
    min-width: 0;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
    min-width: 0;
  }
  .clip {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .idle {
    grid-column: 1 / -1;
    margin: 0;
    color: var(--muted);
  }
  .expanded .idle {
    max-width: 70ch;
  }
  .dim {
    color: var(--muted);
  }
  kbd {
    font-family: var(--font-display);
    font-weight: 600;
    padding: 0 0.35em;
    border: 1px solid rgb(0 0 0 / 0.45);
    border-bottom-width: 2px;
    border-radius: 4px;
    background: linear-gradient(180deg, var(--raised-hi), var(--raised-lo));
    color: var(--ink);
  }

  /* The last Launchkey control, on a small screen like the hardware's own display. */
  .lk {
    flex: none;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding-left: 0.9rem;
    border-left: 1px solid var(--seam);
    box-shadow: inset 1px 0 0 rgb(255 255 255 / 0.04);
  }
  .lk-label {
    writing-mode: vertical-rl;
    transform: rotate(180deg);
    font-size: 0.6rem;
  }
  .screen {
    width: 11.5rem;
    height: 100%;
    max-height: 2.4rem;
    display: flex;
    flex-direction: column;
    justify-content: center;
    padding: 0 0.55rem;
    border-radius: 4px;
    font-family: var(--font-display);
    line-height: 1.15;
    letter-spacing: 0.03em;
  }
  .where {
    font-weight: 600;
    color: var(--screen-ink);
    text-shadow: 0 0 6px var(--screen-glow);
  }
  .what {
    color: var(--screen-dim);
  }
  .where,
  .what {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dim-screen {
    font-style: italic;
  }

  .float {
    flex: none;
    align-self: center;
    padding: 0.25rem 0.55rem;
    border-radius: 5px;
    color: var(--ink);
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.8rem;
    letter-spacing: 0.04em;
    cursor: pointer;
  }
  .float.pressed {
    color: var(--accent);
  }

  @media (max-width: 1100px) {
    .entry {
      grid-template-columns: minmax(7rem, 10rem) minmax(0, 1fr) minmax(8.5rem, 12rem);
      gap: 0 0.8rem;
    }
    .screen {
      width: 9rem;
    }
  }
</style>
