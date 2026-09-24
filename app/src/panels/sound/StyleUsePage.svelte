<!--
  This style: every program the current style sends its parts (its setup and every
  section), what each plays now and by which rule, and a picker to remap it right here
  (an override for that program, or the drum rule for a drum part), for every style or
  for this style only.

  State: soundLibrary (usage, styleKey), style. Commands: setProgramOverride,
  setDrumRule.
-->
<script lang="ts">
  import type { ProgramUse } from '../../lib/api/types'
  import { app } from '../../lib/store.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'
  import PatchSelect from './PatchSelect.svelte'
  import { nav } from './nav.svelte'

  const sl = $derived(app.state.soundLibrary)
  const style = $derived(nav.styleScope)
  const mapped = $derived(sl.usage.filter((u) => u.patch).length)

  const RULE: Record<ProgramUse['rule'], string> = { drums: 'drum rule', override: 'override', family: 'family', fallback: 'style voice' }
  function remap(u: ProgramUse, patch: string | null) {
    if (u.drums) app.send({ type: 'setDrumRule', patch, style })
    else app.send({ type: 'setProgramOverride', program: u.gmProgram, patch, style })
  }
</script>

<div class="scope">
  <Toggle on={!style} tip="sound.scope" onclick={() => (nav.styleScope = false)}>Every style</Toggle>
  <Toggle on={style} tip="sound.scope" onclick={() => (nav.styleScope = true)}>This style</Toggle>
</div>
<p class="explain">
  {app.state.style.name}: {sl.usage.length} program{sl.usage.length === 1 ? '' : 's'} on the band's parts, {mapped} playing your patches.
  A remap here is {style ? 'this style\'s own override' : 'an override for every style'} (drum parts: the drum rule).
</p>

<div class="table" role="table" aria-label="Programs this style sends">
  <div class="tr head" role="row">
    <span role="columnheader">Part</span><span role="columnheader">Style voice</span><span role="columnheader">Plays</span>
  </div>
  {#each sl.usage as u, i (i)}
    <div class="tr" role="row" class:mapped={!!u.patch}>
      <span class="part" role="cell">{u.part}<small>ch {u.channel}</small></span>
      <span class="voice" role="cell">{u.voice}</span>
      <span class="plays" role="cell">
        <PatchSelect value={u.patch} label="{u.part}: {u.voice}" tipKey="sound.remap" none="— style voice" onpick={(p) => remap(u, p)} />
        <small class:own={u.fromStyle}>{RULE[u.rule]}{u.fromStyle ? ' · this style' : ''}</small>
      </span>
    </div>
  {/each}
</div>

<style>
  .scope {
    display: flex;
    gap: 0.4rem;
  }
  .explain {
    margin: 0;
    font-size: var(--fs-small);
    color: var(--muted);
    line-height: 1.35;
  }
  .table {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .tr {
    display: grid;
    grid-template-columns: 5.2rem 1fr 11rem;
    align-items: center;
    gap: 0.4rem;
    padding: 0.2rem 0.3rem;
    border-radius: 5px;
    background: color-mix(in srgb, var(--well) 25%, transparent);
  }
  .tr.head {
    background: none;
    font-family: var(--font-display);
    font-size: 0.72rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .part {
    display: flex;
    flex-direction: column;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.9rem;
  }
  .part small,
  .plays small {
    font-weight: 400;
    font-size: 0.72rem;
    color: var(--muted);
  }
  .plays small.own {
    color: var(--accent);
  }
  .voice {
    font-size: 0.8rem;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tr.mapped .voice {
    text-decoration: line-through;
    text-decoration-color: color-mix(in srgb, var(--muted) 60%, transparent);
  }
  .plays {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }
</style>
