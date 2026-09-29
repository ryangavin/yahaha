<!--
  The GM map page (docs/sound-browser.md, O5): the drums and all 128 GM programs, grouped
  by the 16 families. Each family shows its rule; each program its override, the sound it
  resolves to and the layer that decided it (override, family or auto), as a badge, so an
  auto slot stands out. The switch picks the map the rule pickers edit: the global map, or
  the current style's own (unset rules fall through to the global one, ↳). A rule is set by
  picking from Sounds (PatchSelect opens the Sound Browser), and cleared with ✕.

  "Resolves to" is what plays now, for the style playing: a style's own rule shows a
  "style" mark even while the global map is being edited. Rows the style's parts use are
  marked with those parts (this replaces the old "This style" tab).

  State: soundLibrary (gmMap, map, styleMap, styleKey, usage, families). Commands:
  setDrumRule, setFamilyRule, setProgramOverride, clearStyleMap.
-->
<script lang="ts">
  import { GM } from '../../lib/api/constants'
  import type { GmMapRow } from '../../lib/api/types'
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'
  import PatchSelect from './PatchSelect.svelte'
  import { layerLabel, nav, patchName, soundName } from './nav.svelte'

  const sl = $derived(app.state.soundLibrary)
  const style = $derived(nav.styleScope)
  const map = $derived(style ? sl.styleMap : sl.map)
  const rows = $derived(sl.gmMap)
  const drums = $derived(rows.find((r) => r.program === null))
  const autoCount = $derived(rows.filter((r) => r.resolved.layer === 'auto').length)
  const noneCount = $derived(rows.filter((r) => r.resolved.layer === 'none').length)
  const styleRules = $derived(sl.styleMap.families.filter(Boolean).length + sl.styleMap.overrides.length + (sl.styleMap.drums ? 1 : 0))

  /** The style's parts on a program (or the drums). */
  const usedBy = (program: number | null) =>
    [...new Set(sl.usage.filter((u) => (program === null ? u.drums : !u.drums && u.gmProgram === program)).map((u) => u.part))].join(', ')
  const override = (p: number) => map.overrides.find((o) => o.program === p)?.patch ?? null
  const globalOverride = (p: number) => sl.map.overrides.find((o) => o.program === p)?.patch ?? null
  /** What an unset rule in the style's map falls through to. */
  const fall = (global: string | null) => (style && global ? `↳ ${patchName(sl, global)}` : '—')
  const resolvedName = (r: GmMapRow) => soundName(sl, app.sounds.entries, r.resolved.sound, r.resolved.font)
  const familyRows = (f: number) => rows.filter((r) => r.family === f)
</script>

{#snippet badge(r: GmMapRow)}
  <span class="badge {r.resolved.layer}" use:tip={'sound.map_layer'} aria-label="Decided by: {layerLabel(r.resolved.layer)}{r.resolved.fromStyle ? ', this style' : ''}"
    >{layerLabel(r.resolved.layer)}{#if r.resolved.fromStyle}<small>style</small>{/if}</span
  >
{/snippet}

<div class="scope" role="group" aria-label="Which map the rules edit">
  <Toggle on={!style} tip="sound.scope" onclick={() => (nav.styleScope = false)}>Global</Toggle>
  <Toggle on={style} tip="sound.scope" onclick={() => (nav.styleScope = true)}>This style</Toggle>
  <span class="key engraved">{style ? sl.styleKey || 'no style' : 'every style'}</span>
</div>
<p class="explain">
  {#if style}
    Rules for {sl.styleKey || 'this style'} only. They win over the global map; what you leave blank (↳) follows it.
  {:else}
    Most specific first: the drum rule, a program's override, its family's rule, then auto (the best preset in your SoundFonts).
  {/if}
  <span class="count" class:warn={autoCount > 0}>{autoCount} auto</span>{#if noneCount}, <span class="count warn">{noneCount} unset</span>{/if}.
</p>

<div class="map">
  {#if drums}
    <section class="fam" aria-labelledby="gm-drums">
      <div class="fhead">
        <h3 id="gm-drums" class="engraved">Drums <small>Rhythm 1–2, kit banks</small></h3>
        <span class="rule">
          <PatchSelect value={map.drums} label="Drum rule" tipKey="sound.drums" none={fall(sl.map.drums)} onpick={(patch) => app.send({ type: 'setDrumRule', patch, style })} />
        </span>
      </div>
      <div class="row" class:auto={drums.resolved.layer === 'auto'} class:none={drums.resolved.layer === 'none'}>
        <span class="prog">Kit{#if usedBy(null)}<small class="used">{usedBy(null)}</small>{/if}</span>
        <span class="res">{resolvedName(drums)}</span>
        <span>{@render badge(drums)}</span>
      </div>
    </section>
  {/if}

  {#each sl.families as name, f (f)}
    <section class="fam" aria-labelledby="gm-fam-{f}">
      <div class="fhead">
        <h3 id="gm-fam-{f}" class="engraved">{name} <small>{f * 8 + 1}–{f * 8 + 8}</small></h3>
        <span class="rule">
          <PatchSelect value={map.families[f]} label="{name} family" tipKey="sound.family" none={fall(sl.map.families[f])} onpick={(patch) => app.send({ type: 'setFamilyRule', family: f, patch, style })} />
        </span>
      </div>
      {#each familyRows(f) as r (r.program)}
        {@const p = r.program as number}
        <div class="row" class:auto={r.resolved.layer === 'auto'} class:none={r.resolved.layer === 'none'}>
          <span class="prog">{p + 1} · {GM[p]}{#if usedBy(p)}<small class="used">{usedBy(p)}</small>{/if}</span>
          <span class="res">
            <PatchSelect value={override(p)} label="Override for {GM[p]}" tipKey="sound.override_patch" none={fall(globalOverride(p))} onpick={(patch) => app.send({ type: 'setProgramOverride', program: p, patch, style })} />
            <small class="plays">{resolvedName(r)}</small>
          </span>
          <span>{@render badge(r)}</span>
        </div>
      {/each}
    </section>
  {/each}
</div>

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
  .count.warn {
    color: var(--accent);
    font-weight: 600;
  }
  .map {
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
  }
  .fam {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .fhead {
    display: grid;
    grid-template-columns: 9rem 1fr;
    align-items: center;
    gap: 0.4rem;
  }
  h3 {
    margin: 0;
    font-size: 0.8rem;
  }
  h3 small {
    font-weight: 400;
    color: var(--muted);
  }
  .row {
    display: grid;
    grid-template-columns: 9rem 1fr 4.6rem;
    align-items: center;
    gap: 0.4rem;
    padding: 0.15rem 0.3rem;
    border-radius: 5px;
    background: color-mix(in srgb, var(--well) 25%, transparent);
  }
  .row.auto {
    background: color-mix(in srgb, var(--accent) 8%, transparent);
  }
  .prog {
    display: flex;
    flex-direction: column;
    font-size: 0.85rem;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .used {
    font-size: 0.7rem;
    color: var(--accent);
  }
  .res {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    min-width: 0;
    font-size: 0.85rem;
  }
  .plays {
    font-size: 0.72rem;
    color: var(--muted);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .badge {
    display: inline-flex;
    flex-direction: column;
    align-items: center;
    padding: 0.1rem 0.3rem;
    border-radius: 4px;
    border: 1px solid var(--well-edge);
    font-family: var(--font-display);
    font-size: 0.68rem;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--ink);
  }
  .badge small {
    font-size: 0.85em;
    font-weight: 400;
    color: var(--accent);
  }
  .badge.auto {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    color: var(--accent);
  }
  .badge.none {
    border-style: dashed;
    color: var(--muted);
  }
  .badge.family {
    color: var(--muted);
  }
</style>
