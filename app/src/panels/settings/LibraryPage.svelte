<!--
  Library: the folders yahaha reads styles from (`library.roots`), what's indexed, and a
  rescan (`rescanLibrary`, `library.scanning`).
-->
<script lang="ts">
  import { settings } from '../../lib/api/settings.svelte'
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import Field from './Field.svelte'

  const lib = $derived(app.state.library)
  const view = $derived(settings.view(app.state))
  // A setting the engine lacks (an older engine) is badged and inert: it never pretends to work.
  const inert = $derived(view.mocked.library)
  const errors = $derived(app.library.entries.filter((e) => e.status === 'error').length)
  const categories = $derived(new Set(app.library.entries.map((e) => e.folder)).size)
</script>

<Field name="Style folders" mock={inert} note="Subfolders become the categories in the style browser. Set them with YAHAHA_STYLES or on the command line.">
  <ul class="roots mat-well" use:tip={'settings.style_folders'}>
    {#each view.roots as r (r)}
      <li><span class="icon" aria-hidden="true">▸</span>{r}</li>
    {:else}
      <li class="unknown">The engine doesn't report its style folders yet.</li>
    {/each}
  </ul>
</Field>

<div class="stats mat-screen" aria-live="polite">
  <div><b class="glow-text">{lib.count}</b><span>styles</span></div>
  <div><b class="glow-text">{categories}</b><span>categories</span></div>
  <div><b class="glow-text" class:warn={errors > 0}>{errors}</b><span>won't load</span></div>
  <div><b class="glow-text">{lib.pending}</b><span>indexing</span></div>
</div>

<div class="rescan" class:off={inert}>
  <HwButton
    tip="settings.rescan"
    led={view.scanning ? { rgb: [127, 90, 20], level: 'bright', anim: 'pulse' } : null}
    onclick={() => !inert && settings.send({ type: 'rescanLibrary' })}
  >
    {view.scanning ? 'Scanning…' : 'Rescan styles'}
  </HwButton>
  <span class="hint">{inert ? 'Needs an engine update; for now restart yahaha to pick up new files.' : 'The band keeps playing while it scans.'}</span>
</div>

<style>
  .rescan.off :global(.hw) {
    opacity: 0.5;
  }
  .unknown {
    color: #7d8792;
  }
  .roots {
    list-style: none;
    margin: 0;
    padding: 0.45rem 0.7rem;
    border-radius: 5px;
    display: grid;
    gap: 0.25rem;
    font-family: ui-monospace, 'SF Mono', Menlo, monospace;
    font-size: 0.82rem;
    color: #cfd6dd;
  }
  .roots li {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .icon {
    margin-right: 0.45rem;
    color: #7d8792;
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 0.5rem;
    padding: 0.55rem 0.7rem;
    border-radius: 5px;
    font-family: var(--font-display);
    text-align: center;
  }
  .stats div {
    display: flex;
    flex-direction: column;
  }
  .stats b {
    font-size: 1.35rem;
    font-weight: 700;
  }
  .stats b.warn {
    color: var(--danger);
  }
  .stats span {
    color: var(--screen-dim);
    font-size: 0.85rem;
  }
  .rescan {
    display: flex;
    align-items: center;
    gap: 0.8rem;
  }
  .rescan :global(.hw) {
    width: 9.5rem;
  }
  .hint {
    font-size: var(--fs-small);
    color: var(--muted);
  }
</style>
