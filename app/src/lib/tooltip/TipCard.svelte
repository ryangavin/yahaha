<script lang="ts">
  import { TIPS, keyLabel, type TipKey } from '../../help/tooltips'

  let { key, wide = false }: { key: TipKey; wide?: boolean } = $props()
  const t = $derived(TIPS[key])
  const keys = $derived(t.app_keys ?? t.keys)
</script>

<div class="card" class:wide>
  <div class="head">
    <strong>{t.title}</strong>
    {#if t.genos && t.genos !== t.title}<span class="genos">Genos: {t.genos}</span>{/if}
  </div>
  <p>{t.body}</p>
  <dl>
    <dt>Key</dt>
    <dd>
      {#if keys.length}
        {#each keys as k, i (k)}{#if i > 0}<span class="or"> or </span>{/if}<kbd>{keyLabel(k)}</kbd>{/each}
        {#if t.app_keys}<span class="or"> (terminal: {t.keys.map(keyLabel).join(' / ')})</span>{/if}
      {:else}
        <span class="none">none</span>
      {/if}
    </dd>
    <dt>Launchkey</dt>
    <dd>
      {#if t.launchkey}
        {#each t.launchkey.split('; ') as place, i (place)}{#if i > 0}<br />{/if}{place}{/each}
      {:else}
        <span class="none">not on the Launchkey</span>
      {/if}
    </dd>
  </dl>
</div>

<style>
  .card {
    max-width: 22rem;
    font-size: var(--fs-small);
    line-height: 1.4;
  }
  .card.wide {
    max-width: none;
    display: grid;
    grid-template-columns: minmax(12rem, 18rem) 1fr auto;
    gap: 0.25rem 1.5rem;
    align-items: start;
  }
  .head {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }
  strong {
    font-family: var(--font-display);
    font-size: 1.15rem;
    font-weight: 600;
    letter-spacing: 0.01em;
  }
  .genos {
    color: var(--accent);
    font-size: var(--fs-small);
  }
  p {
    margin: 0.35rem 0 0.5rem;
  }
  .wide p {
    margin: 0;
    max-width: 60ch;
  }
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.2rem 0.6rem;
    margin: 0;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
  }
  kbd {
    font-family: var(--font-display);
    font-weight: 600;
    padding: 0 0.35em;
    border: 1px solid rgb(0 0 0 / 0.45);
    border-bottom-width: 2px;
    border-radius: 4px;
    background: linear-gradient(180deg, var(--raised-hi), var(--raised-lo));
  }
  .or,
  .none {
    color: var(--muted);
  }
</style>
