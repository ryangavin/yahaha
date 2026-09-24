<!--
  The browser's categories: All, Favourites, Recent, then every folder of the library
  (the folder is the category), indented under its parent, with how many styles it holds.
-->
<script lang="ts">
  import { tip } from '../../lib/tooltip/tip.svelte'
  import type { Category, FolderNode } from './model'

  let {
    category,
    folders,
    total,
    favourites,
    recents,
    onpick,
  }: {
    category: Category
    folders: FolderNode[]
    total: number
    favourites: number
    recents: number
    onpick: (c: Category) => void
  } = $props()

  const is = (c: Category) => c.kind === category.kind && (c.kind !== 'folder' || (category.kind === 'folder' && category.path === c.path))
</script>

<nav class="side" aria-label="Categories">
  <button type="button" class="cat" class:on={is({ kind: 'all' })} aria-pressed={is({ kind: 'all' })} use:tip={'browser.all'} onclick={() => onpick({ kind: 'all' })}>
    <span>All styles</span><span class="n">{total.toLocaleString()}</span>
  </button>
  <button
    type="button"
    class="cat"
    class:on={is({ kind: 'favourites' })}
    aria-pressed={is({ kind: 'favourites' })}
    use:tip={'browser.favourites'}
    onclick={() => onpick({ kind: 'favourites' })}
  >
    <span><span class="ico" aria-hidden="true">★</span> Favourites</span><span class="n">{favourites}</span>
  </button>
  <button
    type="button"
    class="cat"
    class:on={is({ kind: 'recents' })}
    aria-pressed={is({ kind: 'recents' })}
    use:tip={'browser.recents'}
    onclick={() => onpick({ kind: 'recents' })}
  >
    <span><span class="ico" aria-hidden="true">↺</span> Recent</span><span class="n">{recents}</span>
  </button>

  <h3 class="engraved">Folders</h3>
  <div class="folders">
    {#each folders as f (f.path)}
      {@const c: Category = { kind: 'folder', path: f.path }}
      <button
        type="button"
        class="cat folder"
        class:on={is(c)}
        aria-pressed={is(c)}
        style:--depth={f.depth}
        use:tip={'browser.folder'}
        onclick={() => onpick(c)}
      >
        <span class="fname">{f.path === '' ? '(top level)' : f.name}</span><span class="n">{f.count.toLocaleString()}</span>
      </button>
    {/each}
  </div>
</nav>

<style>
  .side {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-height: 0;
    overflow: auto;
    padding-right: 0.25rem;
  }
  h3 {
    margin: 0.8rem 0.5rem 0.3rem;
    font-size: 0.72rem;
  }
  .folders {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .cat {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
    min-height: 2.2rem;
    padding: 0 0.6rem 0 calc(0.6rem + var(--depth, 0) * 0.85rem);
    border: 1px solid transparent;
    border-radius: 5px;
    background: none;
    text-align: left;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.95rem;
    letter-spacing: 0.02em;
    color: var(--ink);
  }
  .cat:hover {
    background: rgb(127 127 127 / 0.1);
  }
  .cat.on {
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
    background: color-mix(in srgb, var(--accent) 16%, transparent);
  }
  .folder {
    font-family: var(--font-body);
    font-weight: 500;
    font-size: var(--fs-small);
    min-height: 2.2rem;
  }
  .fname {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  .ico {
    color: var(--accent);
  }
  .n {
    flex: none;
    color: var(--muted);
    font-family: var(--font-display);
    font-weight: 500;
    font-size: 0.8rem;
  }
</style>
