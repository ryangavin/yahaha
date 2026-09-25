<!--
  A program map rule's sound (#117): a button naming the rule's patch (or its fallback)
  that opens the Sound Browser to pick any sound, whatever its source: a SoundFont preset
  or a plugin becomes a library patch the first time a rule names it. ✕ clears the rule.
  Sends nothing itself: `onpick` gets the catalog id, or null to clear.
-->
<script lang="ts">
  import type { TipKey } from '../../help/tooltips'
  import { app, ui } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import { patchName } from './nav.svelte'

  let {
    value,
    label,
    tipKey,
    none = '—',
    onpick,
  }: { value: string | null; label: string; tipKey: TipKey; none?: string; onpick: (id: string | null) => void } = $props()

  // A patch id, or a catalog id not yet in the library (the new override's pick).
  const name = $derived(value ? (app.sounds.entries.find((e) => e.id === value)?.name ?? patchName(app.state.soundLibrary, value)) : null)
  const open = () => (ui.soundPick = { title: label, value: value?.startsWith('sf:') || value?.startsWith('au:') ? null : (value?.replace(/^saved:/, '') ?? null), onpick: (id) => onpick(id) })
</script>

<span class="pick">
  <button type="button" class="val" class:set={!!value} aria-label="{label}: {name ?? none}" use:tip={tipKey} onclick={open}>
    <span class="n">{name ?? none}</span><span class="ar" aria-hidden="true">▾</span>
  </button>
  {#if value}
    <button type="button" class="clr" aria-label="Clear {label}" use:tip={'sound.rule_clear'} onclick={() => onpick(null)}>✕</button>
  {/if}
</span>

<style>
  .pick {
    display: flex;
    gap: 2px;
    width: 100%;
    min-width: 0;
  }
  .val,
  .clr {
    min-height: 2.2rem;
    border-radius: 4px;
    border: 1px solid var(--well-edge);
    background: var(--screen-bg);
    color: var(--screen-dim);
    font: inherit;
    font-size: 0.9rem;
  }
  .val {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0 0.4rem;
    text-align: left;
  }
  .val.set {
    color: var(--screen-ink);
  }
  .n {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .clr {
    flex: none;
    width: 2.2rem;
  }
</style>
