<!--
  Launchkey: the pad page order (docs/eyes-free.md). Page 1 (Sections) is fixed; pages 2-5
  (Racks, Chord, Multi Pads, Setup) are the player's to reorder, and any of them can be
  left out. Pad Bank ▲/▼ and Tab walk the order. Every change sends `setPadPageOrder` with
  the whole new order; the list shows `settings.padPages` as the engine has it, so a
  refused order leaves the list as it was. Kept in settings.json.
-->
<script lang="ts">
  import { DEFAULT_PAD_PAGES, PAD_PAGES, type PadPage } from '../../lib/api/types'
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import Field from './Field.svelte'

  const order = $derived(app.state.settings.padPages)
  const left = $derived(DEFAULT_PAD_PAGES.filter((p) => !order.includes(p)))
  const name = (id: PadPage) => PAD_PAGES.find((p) => p.id === id)?.name ?? id
  const isDefault = $derived(order.length === DEFAULT_PAD_PAGES.length && order.every((p, i) => p === DEFAULT_PAD_PAGES[i]))

  const send = (pages: PadPage[]) => app.send({ type: 'setPadPageOrder', pages })

  function move(i: number, by: -1 | 1) {
    const j = i + by
    if (j < 0 || j >= order.length) return
    const pages = [...order]
    ;[pages[i], pages[j]] = [pages[j], pages[i]]
    send(pages)
  }
  const remove = (i: number) => send(order.filter((_, k) => k !== i))
  const add = (p: PadPage) => send([...order, p])
</script>

<p class="intro">
  The pages Pad Bank ▲/▼ (and Tab) step through. Sections is always page 1; put the others in the order you reach for
  them, and leave out any you don't use. Hold Sound still shows Racks, in the order or not. Kept in your settings.
</p>

<Field name="Pad page order" note="Changes apply at once, on the Launchkey too.">
  <ol class="pages" aria-label="Pad page order">
    <li class="row fixed">
      <span class="num">1</span>
      <span class="pname">Sections</span>
      <span class="tag">Fixed</span>
    </li>
    {#each order as p, i (p)}
      <li class="row" data-page={p}>
        <span class="num">{i + 2}</span>
        <span class="pname">{name(p)}</span>
        <span class="acts">
          <button
            type="button"
            class="act"
            aria-label="Move {name(p)} up"
            disabled={i === 0}
            use:tip={'settings.pad_pages.up'}
            onclick={() => move(i, -1)}>▲</button
          >
          <button
            type="button"
            class="act"
            aria-label="Move {name(p)} down"
            disabled={i === order.length - 1}
            use:tip={'settings.pad_pages.down'}
            onclick={() => move(i, 1)}>▼</button
          >
          <button type="button" class="act" aria-label="Leave out {name(p)}" use:tip={'settings.pad_pages.remove'} onclick={() => remove(i)}
            >✕</button
          >
        </span>
      </li>
    {/each}
  </ol>
</Field>

{#if left.length}
  <Field name="Left out" note="Pad Bank skips these. Add one back to put it last.">
    <ul class="pages" aria-label="Pages left out">
      {#each left as p (p)}
        <li class="row out" data-page={p}>
          <span class="pname">{name(p)}</span>
          <span class="acts">
            <button type="button" class="act wide" aria-label="Add {name(p)} back" use:tip={'settings.pad_pages.add'} onclick={() => add(p)}
              >Add</button
            >
          </span>
        </li>
      {/each}
    </ul>
  </Field>
{/if}

<div class="reset">
  <button type="button" class="act wide" disabled={isDefault} use:tip={'settings.pad_pages.reset'} onclick={() => send([...DEFAULT_PAD_PAGES])}
    >Default order</button
  >
</div>

<style>
  .intro {
    margin: 0;
    color: var(--muted);
    font-size: 0.9rem;
    line-height: 1.4;
  }
  .pages {
    display: grid;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .row {
    display: grid;
    grid-template-columns: 1.6rem 1fr auto;
    align-items: center;
    gap: 0.5rem;
    min-height: 2.4rem;
    padding: 0 0.5rem;
    border: 1px solid var(--line);
    border-radius: 5px;
  }
  .row.out {
    grid-template-columns: 1fr auto;
    border-style: dashed;
    color: var(--muted);
  }
  .num {
    font-family: var(--font-display);
    font-weight: 600;
    color: var(--muted);
    text-align: center;
  }
  .pname {
    font-family: var(--font-display);
    font-weight: 600;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .fixed .pname {
    color: var(--muted);
  }
  .tag {
    font-family: var(--font-display);
    font-size: 0.72rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .acts {
    display: flex;
    gap: 4px;
  }
  .act {
    min-width: 2.2rem;
    min-height: 2rem;
    border: 1px solid var(--line-strong);
    border-radius: 4px;
    background: transparent;
    color: var(--ink);
    font-family: var(--font-display);
    font-weight: 600;
  }
  .act.wide {
    padding: 0 0.7rem;
  }
  .act:not(:disabled):hover {
    background: rgb(255 255 255 / 0.06);
  }
  .act:disabled {
    opacity: 0.35;
  }
  .reset {
    display: flex;
    justify-content: flex-end;
  }
</style>
