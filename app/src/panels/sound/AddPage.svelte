<!--
  Add from SoundFont: the presets of a SoundFont in the SoundFont folder (read from its
  preset headers), searchable; audition one (the band stopped) and add it as a patch.

  State: io.soundFonts, soundLibrary.browse, soundLibrary.auditioning,
  transport.running. Commands: browseSoundFont, auditionPreset, stopPatchAudition,
  addPresetAsPatch.
-->
<script lang="ts">
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'

  const sl = $derived(app.state.soundLibrary)
  const fonts = $derived(app.state.io.soundFonts)
  const browse = $derived(sl.browse)
  const running = $derived(app.state.transport.running)
  let query = $state('')
  let playing = $state<string | null>(null)

  const shown = $derived.by(() => {
    const q = query.trim().toLowerCase()
    const all = browse?.presets ?? []
    return (q ? all.filter((p) => p.name.toLowerCase().includes(q)) : all).slice(0, 300)
  })
  const inLibrary = (bank: number, program: number) =>
    sl.patches.some((p) => p.source.kind === 'soundFont' && p.source.file === browse?.file && p.source.bank === bank && p.source.program === program)
  const key = (bank: number, program: number) => `${bank}:${program}`

  function audition(bank: number, program: number) {
    if (!browse) return
    if (sl.auditioning === 'preset' && playing === key(bank, program)) {
      app.send({ type: 'stopPatchAudition' })
      return
    }
    playing = key(bank, program)
    app.send({ type: 'auditionPreset', file: browse.file, bank, program })
  }
</script>

<div class="line">
  <select aria-label="SoundFont" value={browse?.file ?? ''} use:tip={'sound.soundfont'} onchange={(e) => app.send({ type: 'browseSoundFont', file: e.currentTarget.value || null })}>
    <option value="">Choose a SoundFont…</option>
    {#each fonts as f (f)}<option value={f}>{f.replace(/\.sf2$/i, '')}</option>{/each}
  </select>
  <input type="search" placeholder="Search presets" aria-label="Search presets" bind:value={query} use:tip={'sound.preset_search'} />
</div>

{#if fonts.length === 0}
  <p class="explain">No SoundFonts: put .sf2 files in the synth's SoundFont folder (soundfonts/).</p>
{:else if !browse}
  <p class="explain">Pick a SoundFont to list its presets. Patches from another SoundFont than the synth's load it alongside.</p>
{:else if browse.error}
  <p class="explain err">{browse.file}: {browse.error}</p>
{:else}
  <p class="explain">{browse.presets.length} presets{shown.length < browse.presets.length ? `, ${shown.length} shown` : ''}. {running ? 'Stop the band to audition.' : '▶ plays one on its own.'}</p>
  <div class="presets">
    {#each shown as p (key(p.bank, p.program))}
      {@const on = sl.auditioning === 'preset' && playing === key(p.bank, p.program)}
      <div class="preset" class:have={inLibrary(p.bank, p.program)}>
        <span class="bp">{p.bank >= 128 ? 'kit' : p.bank}:{p.program + 1}</span>
        <span class="pn">{p.name || '(no name)'}</span>
        <button type="button" class="mini mat-raised" class:on aria-label="{on ? 'Stop' : 'Audition'} {p.name}" aria-disabled={running} use:tip={on ? 'sound.audition_stop' : 'sound.preset'} onclick={() => audition(p.bank, p.program)}>{on ? '■' : '▶'}</button>
        <HwButton tip="sound.preset_add" label="Add {p.name} as a patch" onclick={() => browse && app.send({ type: 'addPresetAsPatch', file: browse.file, bank: p.bank, program: p.program, name: null })}>+</HwButton>
      </div>
    {/each}
  </div>
{/if}

<style>
  .line {
    display: flex;
    gap: 0.4rem;
  }
  select,
  input {
    flex: 1;
    min-width: 0;
    min-height: 2.2rem;
    padding: 0 0.45rem;
    border-radius: 4px;
    border: 1px solid var(--well-edge);
    background: var(--screen-bg);
    color: var(--screen-ink);
    font: inherit;
    font-size: 0.9rem;
  }
  .explain {
    margin: 0;
    font-size: var(--fs-small);
    color: var(--muted);
    line-height: 1.35;
  }
  .err {
    color: var(--danger);
  }
  .presets {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .preset {
    display: grid;
    grid-template-columns: 3.2rem 1fr 2.4rem auto;
    align-items: center;
    gap: 0.35rem;
    padding: 0 0.2rem;
    border-radius: 4px;
  }
  .preset:hover {
    background: color-mix(in srgb, var(--well) 30%, transparent);
  }
  .preset.have .pn::after {
    content: ' · in library';
    color: var(--accent);
    font-size: 0.75rem;
  }
  .bp {
    font-variant-numeric: tabular-nums;
    font-size: 0.8rem;
    color: var(--muted);
  }
  .pn {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .mini {
    min-height: 2.2rem;
    border-radius: 5px;
    color: var(--ink);
  }
  .mini.on {
    color: var(--accent);
  }
  .mini[aria-disabled='true'] {
    opacity: 0.45;
  }
</style>
