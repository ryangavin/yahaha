<!--
  The bar across the top of the mixer row: everything the old Mixer drawer had above its
  strips, in one or two wrapping lines.

  - Page tabs Panel/Style: the tab IS the Launchkey fader page (`state.mixer.faderPage`),
    so switching sends `setFaderPage` and the Launchkey's page button switches the tab.
    All twelve strips stay visible either way; the page only says which ones the eight
    Launchkey faders move. The lamp beside it is the Launchkey's page button light.
  - The fader layer (VOL/PAN/REV/CHO/DLY): what the Launchkey faders move.
  - MIDI out port; CPU of every track together and the plugin instances (#340, #407),
    from the meters the row reads.
  - Metronome (on, bell, its own volume: the built-in synth's click, never on the port).
  - Style Track Mute (a Genos Live Control knob, A/B order) and "Sends: style" (every
    Style part's sends back to the style's, #268).
  - Style volume (#199) and Multi Pad volume (#196): Panel faders 5 and 6 on the
    Launchkey, scales on the Style parts' and the pads' CC 7. ↕ while the level waits for
    its Launchkey fader.
  - Effects…: opens the Effects screen, with what each shared block plays.
  The Harmony/Arpeggio switch the drawer drew under Panel fader 5 lives on the Harmony
  drawer (and J, and the Launchkey's fader button 5).
-->
<script lang="ts">
  import { FADER_LAYERS, type FaderLayer, type FaderPage, type Meters, type TrackMuteOrder } from '../../lib/api/types'
  import { app, ui } from '../../lib/store.svelte'
  import { css } from '../../lib/leds'
  import { surfaceOf } from '../../lib/surface'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import DrawerButton from '../../lib/ui/DrawerButton.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'
  import HSlider from '../settings/HSlider.svelte'
  import { CPU_WARN, pct } from './cpu'

  let { meters }: { meters: Meters | null } = $props()

  const LAYER_NAMES: Record<FaderLayer, string> = { volume: 'VOL', pan: 'PAN', reverb: 'REV', chorus: 'CHO', delay: 'DLY' }
  const TABS: { id: FaderPage; name: string }[] = [
    { id: 'panel', name: 'Panel' },
    { id: 'style', name: 'Style' },
  ]

  const mixer = $derived(app.state.mixer)
  const page = $derived(mixer.faderPage)
  const metronome = $derived(app.state.metronome)
  const effects = $derived(app.state.effects.blocks)
  const outPort = $derived(app.state.io.outputPort)
  const surface = $derived(surfaceOf(app.state, app.library))
  const pageLed = $derived(surface.controls.find((c) => c.id === 'masterButton') ?? null)
  const cpuTotal = $derived(meters && meters.channels.length > 0 ? meters.cpu : null)

  // Style Track Mute is a knob: the engine keeps only the parts' switches it sets, so the
  // knob's position and order are this bar's. Choosing an order sends nothing, so parts
  // switched off by hand stay off.
  let muteOrder = $state<TrackMuteOrder>('a')
  let muteValue = $state(127)
  function trackMute(v: number) {
    muteValue = v
    app.send({ type: 'styleTrackMute', order: muteOrder, value: v })
  }

  function setPage(p: FaderPage) {
    if (p !== page) app.send({ type: 'setFaderPage', page: p })
  }
  function tabKey(e: KeyboardEvent) {
    if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') return
    e.preventDefault()
    e.stopPropagation()
    const next = page === 'panel' ? 'style' : 'panel'
    setPage(next)
    document.getElementById(`mixer-tab-${next}`)?.focus()
  }
</script>

<div class="bar" role="group" aria-label="Mixer">
  <div class="tabs" role="tablist" aria-label="Mixer page (the Launchkey fader page)">
    {#each TABS as t (t.id)}
      <button
        type="button"
        role="tab"
        id="mixer-tab-{t.id}"
        class="tab mat-raised"
        class:pressed={page === t.id}
        aria-selected={page === t.id}
        aria-controls="mixer-strips"
        tabindex={page === t.id ? 0 : -1}
        use:tip={'mixer.page'}
        onclick={() => setPage(t.id)}
        onkeydown={tabKey}
      >
        <span class="dot" class:on={page === t.id} aria-hidden="true"></span>{t.name}
      </button>
    {/each}
  </div>
  <span class="follows engraved" use:tip={'mixer.page'}>
    <span class="lamp" aria-hidden="true" style:--led={pageLed ? css(pageLed.rgb) : 'transparent'}></span>
    Launchkey faders: {page === 'panel' ? 'Panel' : 'Style'}
  </span>
  <div class="group layers" role="group" aria-label="Fader layer">
    {#each FADER_LAYERS as l (l)}
      <Toggle on={mixer.faderLayer === l} tip="mixer.layer" onclick={() => app.send({ type: 'setFaderLayer', layer: l })}>{LAYER_NAMES[l]}</Toggle>
    {/each}
  </div>

  <div class="group level">
    <span class="engraved">Style</span>
    <div class="slider">
      <HSlider value={mixer.styleVolume} tip="mixer.style_level" label="Style volume" onchange={(v) => app.send({ type: 'setStyleVolume', volume: v })} />
    </div>
    {#if mixer.styleVolumeWaiting}<span class="wait" data-testid="style-waiting" use:tip={'mixer.style_level'}>↕</span>{/if}
  </div>
  <div class="group level">
    <span class="engraved">M.Pad</span>
    <div class="slider">
      <HSlider value={mixer.multiPadVolume} tip="mixer.pad_level" label="Multi Pad volume" onchange={(v) => app.send({ type: 'setMultiPadVolume', volume: v })} />
    </div>
    {#if mixer.multiPadVolumeWaiting}<span class="wait" data-testid="pad-waiting" use:tip={'mixer.pad_level'}>↕</span>{/if}
  </div>

  <div class="group trackmute">
    <span class="engraved">Track Mute</span>
    {#each ['a', 'b'] as const as o (o)}
      <button
        type="button"
        class="order mat-raised"
        class:pressed={muteOrder === o}
        aria-pressed={muteOrder === o}
        use:tip={'mixer.track_mute_order'}
        onclick={() => (muteOrder = o)}>{o.toUpperCase()}</button
      >
    {/each}
    <div class="slider">
      <HSlider value={muteValue} tip="mixer.track_mute" label="Style Track Mute" onchange={trackMute} />
    </div>
    <button
      type="button"
      class="order mat-raised"
      disabled={mixer.styleParts.every((p) => p.sendsSet.length === 0)}
      use:tip={'mixer.style.reset_sends'}
      onclick={() => app.send({ type: 'resetStylePartSends', part: null })}>Sends: style</button
    >
  </div>

  <div class="group metronome">
    <Toggle on={metronome.on} tip="metronome.on" onclick={() => app.send({ type: 'toggleMetronome' })}>Metronome</Toggle>
    <Toggle on={metronome.bell} tip="metronome.bell" onclick={() => app.send({ type: 'setMetronomeBell', on: !metronome.bell })}>Bell</Toggle>
    <div class="slider">
      <HSlider
        value={metronome.volume}
        tip="metronome.volume"
        label="Metronome volume"
        disabled={!metronome.audible}
        onchange={(v) => app.send({ type: 'setMetronomeVolume', volume: v })}
      />
    </div>
    {#if !metronome.audible}<span class="note">Synth off: no click</span>{/if}
  </div>

  <!-- The effect blocks and the style's inserts live on the Effects screen: here, what each block plays and the way there. -->
  <div class="group effects" role="group" aria-label="Effects">
    <DrawerButton tip="drawer.effects" open={ui.effects} onclick={() => ui.toggleDrawer('effects')}>Effects…</DrawerButton>
    {#each effects as b (b.block)}
      <span class="fx-now"><span class="engraved">{b.block === 'variation' ? 'Delay' : b.name}</span> {b.effectName}</span>
    {/each}
  </div>

  <div class="group load" data-testid="cpu-total">
    <span use:tip={'mixer.cpu_total'} class:warn={cpuTotal !== null && cpuTotal.peak > CPU_WARN}>
      <span class="engraved">CPU</span>
      {#if cpuTotal}
        <b>{pct(cpuTotal.total)}</b> · pk {pct(cpuTotal.peak)}{cpuTotal.bufferUs > 0 ? ` of a ${(cpuTotal.bufferUs / 1000).toFixed(1)} ms buffer` : ''}
      {:else}
        <b>—</b> (synth off)
      {/if}
    </span>
    <span use:tip={'part.plugin_instances'}>
      <span class="engraved">Plugins</span> <b>{app.state.plugins.instances ?? 0}</b>
    </span>
    <span class="out" use:tip={'mixer.channel'}>
      <span class="engraved">MIDI out</span> <b>{outPort || '—'}</b>
    </span>
  </div>

  <span class="info" use:tip={'mixer.info'}><b>A fader is its channel’s CC 7</b>, no hidden gain</span>
</div>

<style>
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.3rem 0.9rem;
    padding: 0.25rem 0.4rem;
    font-size: 0.8rem;
    color: var(--ink);
  }
  .group {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }
  .bar :global(.toggle) {
    min-height: 1.9em;
    padding: 0.1em 0.55em;
    font-size: 0.9em;
  }
  .bar :global(.drawer-btn) {
    height: 2.2em;
  }
  .tabs {
    display: flex;
    gap: 0.2rem;
  }
  .tab {
    display: inline-flex;
    align-items: center;
    gap: 0.4em;
    min-height: 1.9rem;
    padding: 0 0.7em;
    border-radius: 5px;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.9rem;
    letter-spacing: 0.03em;
    color: var(--ink);
  }
  .tab.pressed {
    outline: 1px solid var(--accent);
  }
  /* Only the selected tab takes focus (roving tabindex), so the focus ring must beat the
     selected outline above or keyboard focus is invisible. */
  .tab:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .dot {
    width: 0.5em;
    height: 0.5em;
    border-radius: 50%;
    background: var(--lamp-off);
    box-shadow: inset 0 1px 1px rgb(0 0 0 / 0.6);
  }
  .dot.on {
    background: var(--accent);
    box-shadow: 0 0 8px var(--accent);
  }
  .follows {
    display: inline-flex;
    align-items: center;
    gap: 0.4em;
    white-space: nowrap;
  }
  .lamp {
    width: 0.9em;
    height: 0.45em;
    border-radius: 2px;
    background: var(--led);
    box-shadow: 0 0 6px var(--led);
  }
  .slider {
    width: 6.5rem;
  }
  .wait {
    font-weight: 700;
    color: var(--accent-ink);
    background: var(--accent);
    border-radius: 3px;
    padding: 0 0.2em;
  }
  .order {
    min-width: 1.8rem;
    min-height: 1.8rem;
    padding: 0 0.4em;
    border-radius: 5px;
    font-family: var(--font-display);
    font-weight: 600;
    color: var(--ink);
    white-space: nowrap;
  }
  .order.pressed {
    outline: 1px solid var(--accent);
  }
  .note {
    font-size: var(--fs-small);
    color: var(--muted);
  }
  .fx-now,
  .load,
  .out {
    font-family: var(--font-display);
    white-space: nowrap;
  }
  .load {
    gap: 0.9rem;
    font-variant-numeric: tabular-nums;
  }
  .load .warn b {
    color: var(--danger);
  }
  .info {
    font-size: var(--fs-small);
    color: var(--muted);
  }
  .info b {
    color: var(--ink);
    font-weight: 600;
  }
</style>
