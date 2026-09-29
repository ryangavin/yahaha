<!--
  The Master Compressor and Master EQ on the mixer's master strip (Genos Mixer › Master):
  each one's switch, and a ▸ button opening a small editor with each one's type, the
  compressor's Compression, Texture and Output and the EQ's eight bands (gain, frequency,
  Q, and shelf on the edge bands). The strip is narrow, so the types live in the editor. Both run on the whole mix, after the effects; off, the mix is
  untouched. The master column sits at the window's bottom, so the editor floats centred
  over the screen; ✕, Escape or a press outside closes it.
-->
<script lang="ts">
  import { COMP_PRESETS, EQ_PRESETS, MASTER_EQ_FREQ_RANGE, type CompParam, type CompPreset, type EqBand, type EqPreset } from '../../lib/api/types'
  import type { TipKey } from '../../help/tooltips'
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HSlider from '../settings/HSlider.svelte'
  import { dbText, hzText } from './eq'

  const master = $derived(app.state.effects.master)
  let open = $state(false)
  let root: HTMLDivElement | undefined = $state()
  let editor: HTMLDivElement | undefined = $state()

  // The editor floats over the screen (the strip sits at the bottom of the window): a
  // press outside it and outside this column's buttons, or Escape, closes it.
  function outside(e: PointerEvent) {
    if (!open || !(e.target instanceof Node)) return
    if (editor?.contains(e.target) || root?.contains(e.target)) return
    open = false
  }
  function escape(e: KeyboardEvent) {
    if (open && e.key === 'Escape') {
      e.preventDefault()
      e.stopPropagation()
      open = false
    }
  }

  const COMP_PARAMS: { param: CompParam; name: string; min: number; max: number; tip: TipKey; format: (v: number) => string }[] = [
    { param: 'compression', name: 'Compression', min: 0, max: 100, tip: 'fx.master_comp_compression', format: (v) => `${v}%` },
    { param: 'texture', name: 'Texture', min: 0, max: 100, tip: 'fx.master_comp_texture', format: (v) => `${v}%` },
    { param: 'output', name: 'Output', min: -12, max: 12, tip: 'fx.master_comp_output', format: (v) => `${dbText(v)} dB` },
  ]

  function setBand(i: number, change: Partial<EqBand>) {
    const b = { ...master.eq.bands[i], ...change }
    app.send({ type: 'setMasterEqBand', band: i, gain: b.gain, freq: b.freq, q: b.q, shelf: b.shelf })
  }
  /** A number field's value, or null while it isn't one. */
  const num = (e: Event) => {
    const v = Number((e.currentTarget as HTMLInputElement).value)
    return Number.isFinite(v) ? v : null
  }
</script>

<svelte:window onpointerdown={outside} onkeydowncapture={escape} />

<div class="master-fx" bind:this={root}>
  <div class="row">
    <button
      type="button"
      class="sw mat-raised"
      class:pressed={master.compressor.on}
      role="switch"
      aria-checked={master.compressor.on}
      use:tip={'fx.master_comp'}
      onclick={() => app.send({ type: 'setMasterCompressorOn', on: !master.compressor.on })}><span class="led" class:on={master.compressor.on}></span>Comp</button
    >
  </div>
  <div class="row">
    <button
      type="button"
      class="sw mat-raised"
      class:pressed={master.eq.on}
      role="switch"
      aria-checked={master.eq.on}
      use:tip={'fx.master_eq'}
      onclick={() => app.send({ type: 'setMasterEqOn', on: !master.eq.on })}><span class="led" class:on={master.eq.on}></span>EQ</button
    >
    <button type="button" class="expand mat-raised" aria-expanded={open} aria-label="Master settings" use:tip={'fx.master_edit'} onclick={() => (open = !open)}
      >{open ? '▾' : '▸'}</button
    >
  </div>

  {#if open}
    <div class="editor" role="group" aria-label="Master Compressor and EQ" bind:this={editor}>
      <div class="head">
        <span class="title">Master</span>
        <button type="button" class="close mat-raised" aria-label="Close master settings" use:tip={'fx.master_edit'} onclick={() => (open = false)}>✕</button>
      </div>
      <div class="param">
        <span class="engraved label">Compressor</span>
        <select
          class="field"
          aria-label="Master Compressor type"
          use:tip={'fx.master_comp_type'}
          value={master.compressor.preset}
          onchange={(e) => app.send({ type: 'setMasterCompressorPreset', preset: e.currentTarget.value as CompPreset })}
        >
          {#each COMP_PRESETS as p (p.preset)}
            <option value={p.preset}>{p.name}{master.compressor.edited && p.preset === master.compressor.preset ? ' (edited)' : ''}</option>
          {/each}
        </select>
      </div>
      {#each COMP_PARAMS as p (p.param)}
        <div class="param">
          <span class="label">{p.name}</span>
          <div class="slider">
            <HSlider
              value={master.compressor[p.param]}
              min={p.min}
              max={p.max}
              tip={p.tip}
              label="Master Compressor {p.name}"
              format={p.format}
              onchange={(v) => app.send({ type: 'setMasterCompressorParam', param: p.param, value: v })}
            />
          </div>
        </div>
      {/each}
      <div class="param">
        <span class="engraved label">EQ</span>
        <select
          class="field"
          aria-label="Master EQ type"
          use:tip={'fx.master_eq_type'}
          value={master.eq.preset}
          onchange={(e) => app.send({ type: 'setMasterEqPreset', preset: e.currentTarget.value as EqPreset })}
        >
          {#each EQ_PRESETS as p (p.preset)}
            <option value={p.preset}>{p.name}{master.eq.edited && p.preset === master.eq.preset ? ' (edited)' : ''}</option>
          {/each}
        </select>
      </div>
      <div class="bands">
        {#each master.eq.bands as b, i (i)}
          <div class="band" role="group" aria-label="EQ band {i + 1}">
            <span class="label">{i + 1} · {hzText(b.freq)}</span>
            <div class="slider">
              <HSlider value={b.gain} min={-12} max={12} unity={0} tip="fx.master_eq_gain" label="EQ band {i + 1} gain" format={(v) => `${dbText(v)} dB`} onchange={(v) => setBand(i, { gain: v })} />
            </div>
            <input
              class="field"
              type="number"
              aria-label="EQ band {i + 1} frequency (Hz)"
              use:tip={'fx.master_eq_freq'}
              min={MASTER_EQ_FREQ_RANGE[i][0]}
              max={MASTER_EQ_FREQ_RANGE[i][1]}
              value={b.freq}
              onchange={(e) => {
                const v = num(e)
                if (v !== null) setBand(i, { freq: v })
              }}
            />
            <input
              class="field q"
              type="number"
              aria-label="EQ band {i + 1} Q"
              use:tip={'fx.master_eq_q'}
              min={0.1}
              max={12}
              step={0.1}
              disabled={b.shelf}
              value={b.q / 10}
              onchange={(e) => {
                const v = num(e)
                if (v !== null) setBand(i, { q: Math.round(v * 10) })
              }}
            />
            {#if i === 0 || i === 7}
              <label class="shelf">
                <input type="checkbox" use:tip={'fx.master_eq_shelf'} checked={b.shelf} onchange={(e) => setBand(i, { shelf: e.currentTarget.checked })} />Shelf
              </label>
            {:else}
              <span class="shelf" aria-hidden="true"></span>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .master-fx {
    position: relative;
    display: grid;
    gap: 0.25rem;
    align-content: start;
    font-size: 0.72rem;
  }
  .row {
    display: flex;
    gap: 0.2rem;
    align-items: center;
  }
  .sw {
    flex: 1;
    display: inline-flex;
    align-items: center;
    gap: 0.3em;
    min-height: 1.6rem;
    padding: 0.1em 0.4em;
    border-radius: 4px;
    font-family: var(--font-display);
    font-weight: 600;
    color: var(--ink);
  }
  .led {
    width: 0.5em;
    height: 0.5em;
    border-radius: 50%;
    background: var(--lamp-off);
  }
  .led.on {
    background: var(--accent);
  }
  .expand {
    min-width: 1.4rem;
    min-height: 1.6rem;
    border-radius: 4px;
    color: var(--ink);
  }
  /* A floating panel centred in the window, above the drawers (z 40–50) and under the
     tooltips (z 100): the master column sits at the window's bottom, so the editor can't
     hang below it. It scrolls within the viewport when the window is short. */
  .editor {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 60;
    width: min(30rem, calc(100vw - 32px));
    max-height: calc(100dvh - 32px);
    overflow: auto;
    color: var(--ink);
    display: grid;
    gap: 0.35rem;
    padding: 0.5rem 0.6rem;
    border: 1px solid var(--seam);
    border-radius: 6px;
    background: var(--panel, var(--bg));
    box-shadow: 0 4px 16px rgb(0 0 0 / 0.4);
    font-size: 0.85rem;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .title {
    font-family: var(--font-display);
    font-weight: 600;
    letter-spacing: 0.03em;
  }
  .close {
    min-width: 1.8rem;
    min-height: 1.6rem;
    border-radius: 4px;
    color: var(--ink);
  }
  .param,
  .band {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .label {
    min-width: 5.5rem;
    color: var(--muted);
  }
  .slider {
    flex: 1;
    min-width: 7rem;
  }
  .bands {
    display: grid;
    gap: 0.25rem;
  }
  input.field {
    width: 4.2rem;
  }
  input.q {
    width: 3.4rem;
  }
  .shelf {
    min-width: 3.8rem;
    display: inline-flex;
    align-items: center;
    gap: 0.2em;
  }
</style>
