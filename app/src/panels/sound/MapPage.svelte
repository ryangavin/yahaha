<!--
  Program Map: which patch the programs a style sends play. Every style's map, or this
  style's own rules (which win over it; unset ones fall through). The drum rule, a grid of
  the 16 GM families with a patch picker each, and the program overrides.

  State: soundLibrary (map, styleMap, styleKey, families). Commands: setDrumRule,
  setFamilyRule, setProgramOverride, clearStyleMap.
-->
<script lang="ts">
  import { GM } from '../../lib/api/mock'
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'
  import PatchSelect from './PatchSelect.svelte'
  import { nav, patchName } from './nav.svelte'

  const sl = $derived(app.state.soundLibrary)
  const style = $derived(nav.styleScope)
  const map = $derived(style ? sl.styleMap : sl.map)
  let newProgram = $state(4)
  let newPatch = $state<string | null>(null)
  const styleRules = $derived(sl.styleMap.families.filter(Boolean).length + sl.styleMap.overrides.length + (sl.styleMap.drums ? 1 : 0))

  /** What an unset rule falls back to, for the picker's "none" line. */
  const fallFamily = (f: number) => (style && sl.map.families[f] ? `↳ ${patchName(sl, sl.map.families[f])}` : '— style voice')
  const fallDrums = $derived(style && sl.map.drums ? `↳ ${patchName(sl, sl.map.drums)}` : '— style kit')
</script>

<div class="scope">
  <Toggle on={!style} tip="sound.scope" onclick={() => (nav.styleScope = false)}>Every style</Toggle>
  <Toggle on={style} tip="sound.scope" onclick={() => (nav.styleScope = true)}>This style</Toggle>
  <span class="key engraved">{style ? sl.styleKey || 'no style' : 'global map'}</span>
</div>
<p class="explain">
  {#if style}
    Rules for {sl.styleKey} only. They win over the global map; what you leave blank (↳) follows it.
  {:else}
    Every style's program changes go through these rules: the drum rule, then a program's override, then its family. Bank variations count as their program. Anything blank plays the style's own voice.
  {/if}
</p>

<section class="block" aria-labelledby="map-drums">
  <h3 id="map-drums" class="engraved">Drums · Rhythm 1–2 and drum kit banks</h3>
  <PatchSelect value={map.drums} label="Drum rule" tipKey="sound.drums" none={fallDrums} onpick={(patch) => app.send({ type: 'setDrumRule', patch, style })} />
</section>

<section class="block" aria-labelledby="map-families">
  <h3 id="map-families" class="engraved">GM families</h3>
  <div class="grid">
    {#each sl.families as name, f (f)}
      <div class="fam" class:set={!!map.families[f]}>
        <span class="fname">{name}<small>{f * 8 + 1}–{f * 8 + 8}</small></span>
        <PatchSelect value={map.families[f]} label="{name} family" tipKey="sound.family" none={fallFamily(f)} onpick={(patch) => app.send({ type: 'setFamilyRule', family: f, patch, style })} />
      </div>
    {/each}
  </div>
</section>

<section class="block" aria-labelledby="map-overrides">
  <h3 id="map-overrides" class="engraved">Program overrides</h3>
  {#if map.overrides.length === 0}<p class="explain">None. Add one where a family is too coarse, e.g. E.Piano 1 inside Piano.</p>{/if}
  {#each map.overrides as o (o.program)}
    <div class="ov">
      <span class="prog">{o.program + 1} · {GM[o.program]}</span>
      <PatchSelect value={o.patch} label="Override for {GM[o.program]}" tipKey="sound.override_patch" onpick={(patch) => app.send({ type: 'setProgramOverride', program: o.program, patch, style })} />
      <HwButton tip="sound.override_remove" label="Remove override for {GM[o.program]}" onclick={() => app.send({ type: 'setProgramOverride', program: o.program, patch: null, style })}>✕</HwButton>
    </div>
  {/each}
  <div class="ov add">
    <select aria-label="Program" bind:value={newProgram} use:tip={'sound.override_program'}>
      {#each GM as g, p (p)}<option value={p}>{p + 1} · {g}</option>{/each}
    </select>
    <PatchSelect value={newPatch} label="Patch for the new override" tipKey="sound.override_patch" onpick={(id) => (newPatch = id)} />
    <HwButton tip="sound.override_add" onclick={() => newPatch && app.send({ type: 'setProgramOverride', program: newProgram, patch: newPatch, style })}>Add</HwButton>
  </div>
</section>

{#if style && styleRules > 0}
  <div><HwButton tip="sound.clear_style_map" onclick={() => app.send({ type: 'clearStyleMap' })}>Clear this style's rules</HwButton></div>
{/if}

<style>
  .scope {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .key {
    margin-left: auto;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .explain {
    margin: 0;
    font-size: var(--fs-small);
    color: var(--muted);
    line-height: 1.35;
  }
  .block {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  h3 {
    margin: 0;
    font-size: 0.78rem;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.35rem 0.6rem;
  }
  .fam {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .fname {
    display: flex;
    justify-content: space-between;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.85rem;
    color: var(--muted);
  }
  .fam.set .fname {
    color: var(--ink);
  }
  .fname small {
    color: var(--muted);
    font-weight: 400;
  }
  .ov {
    display: grid;
    grid-template-columns: 9rem 1fr auto;
    align-items: center;
    gap: 0.35rem;
  }
  .prog {
    font-size: 0.85rem;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  select {
    min-width: 0;
    min-height: 2.2rem;
    padding: 0 0.4rem;
    border-radius: 4px;
    border: 1px solid var(--seam);
    background: var(--well);
    color: var(--ink);
    font: inherit;
    font-size: 0.85rem;
  }
</style>
