<!--
  One keyboard part of the rack (docs/racks.md "Screens", the wireframe's `slot`):

  ┌ [RIGHT 1] [Silk Strings                              ] MINE ┐
  │ Sampler Deluxe  ● sound edited            [Edit] [Save sound] │
  │ ⚠ String Deluxe isn't installed, so this part is silent. [Replace…] │
  │ [On] Level ━━━━━━━━━━━━━━━━○━━ 100                            │
  │ Pan ━━○━ C     Rev ━○━━ 50     Cho ○━━━ 10                     │
  │ Oct − 0 +   Voice − +          (a plugin:) [Reload] [In proc] │
  └───────────────────────────────────────────────────────────────┘

  Clicking anywhere in it makes it the part you edit (selectPart), as the Launchkey's EDIT
  pads do. Hovering or focusing it lights its fader on the mirror (lib/mirror). Every
  control sends an existing part command; nothing here keeps state of its own.
-->
<script lang="ts">
  import type { KeyboardPart } from '../../lib/api/types'
  import { mirror } from '../../lib/mirror.svelte'
  import { app, ui } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'
  import HSlider from '../settings/HSlider.svelte'
  import { inProcessPending, octaveLabel, onTip, pluginStatusLine, volumeTip } from '../parts/parts'
  import { isMissing, panLabel, soundName, type SoundBadge } from './rack'

  let {
    part,
    index,
    instrument,
    badge,
    recalled = 0,
  }: {
    part: KeyboardPart
    index: number
    /** What plays the sound: the plugin's name, or the SoundFont's. */
    instrument: string
    badge: SoundBadge
    /** Bumped on every OTS recall: the slot flashes once. */
    recalled?: number
  } = $props()

  const plugins = $derived(app.state.plugins)
  const plugin = $derived(part.plugin)
  const missing = $derived(isMissing(part))
  const entry = $derived(plugin ? plugins.list.find((p) => p.id === plugin.id) : undefined)
  const pending = $derived(inProcessPending(plugin, entry))
  const status = $derived(plugin && !missing && plugin.status !== 'playing' ? pluginStatusLine(plugin, plugins.available).replace(/ ▾$/, '') : '')
  const canEdit = $derived(!!plugin?.editor)

  const select = () => {
    if (!part.selected) app.send({ type: 'selectPart', part: index })
  }
  const openSounds = () => (ui.soundBrowser = index)
  function stepVoice(delta: number) {
    app.send({ type: 'selectPart', part: index })
    app.send({ type: 'stepVoice', delta })
  }
  const octave = (d: number) => app.send({ type: 'setPartOctave', part: index, octave: Math.max(-2, Math.min(2, part.octave + d)) })
  function toggleInProcess() {
    if (entry?.canRunInProcess) app.send({ type: 'setPluginInProcess', id: entry.id, inProcess: !entry.inProcess })
  }
  const link = (on: boolean) => (mirror.panelFader = on ? index : mirror.panelFader === index ? null : mirror.panelFader)
  // Closing the drawer under the pointer never fires pointerleave: let go of the mirror.
  $effect(() => () => link(false))
</script>

<!-- Pointer-down anywhere selects the part (a shortcut); from the keyboard, the part name button does. -->
<div
  class="slot"
  class:focus={part.selected}
  class:off={!part.sounding}
  role="group"
  aria-label={part.name}
  onpointerdown={select}
  onpointerenter={() => link(true)}
  onpointerleave={() => link(false)}
  onfocusin={() => link(true)}
  onfocusout={() => link(false)}
>
  {#key recalled}<span class="flash" class:go={recalled > 0} aria-hidden="true"></span>{/key}

  <div class="head">
    <button type="button" class="pt mat-raised" class:pressed={part.selected} aria-pressed={part.selected} use:tip={'rack.part'} onclick={select}>
      <span class="dot" aria-hidden="true"></span>{part.name}
    </button>
    <button type="button" class="snbtn mat-screen" class:missing aria-label="{part.name} sound: {soundName(part)}" use:tip={'rack.sound'} onclick={openSounds}>
      <span class="glow-text">{soundName(part)}</span>
    </button>
    <span class="badge b-{badge.toLowerCase()}">{badge}</span>
  </div>

  <div class="acts">
    <small class="inst">{part.playsBass ? 'Style Bass under Manual Bass' : instrument}</small>
    {#if part.soundEdited}<span class="dotmark">● sound edited</span>{/if}
    {#if status}<span class="status">{status}</span>{/if}
    <span class="grow"></span>
    <button
      type="button"
      class="mini mat-raised"
      aria-disabled={!canEdit}
      aria-label={plugin ? `Edit ${part.name}'s plugin` : `${part.name}: SoundFont sounds have no editor`}
      use:tip={'rack.edit'}
      onclick={() => canEdit && app.pluginEditor(index, true)}>Edit</button
    >
    <button type="button" class="mini mat-raised" aria-disabled={!part.soundEdited} use:tip={'rack.save_sound'} onclick={() => part.soundEdited && app.send({ type: 'saveSound', part: index })}>Save sound</button>
  </div>

  {#if missing}
    <div class="warn" role="alert">
      <span>⚠ {plugin?.name || plugin?.id} isn't installed, so this part is silent.</span>
      <button type="button" class="mini mat-raised" use:tip={'rack.replace'} onclick={openSounds}>Replace…</button>
      <small>keeps level, pan, sends and octave</small>
    </div>
  {/if}

  <div class="levelrow">
    <Toggle on={part.sounding} tip={onTip(index)} onclick={() => app.send({ type: 'togglePart', part: index })}>{part.playsBass ? 'Bass' : 'On'}</Toggle>
    <span class="ctl"><span class="k">Level{#if part.waiting}<b class="pick" title="The Launchkey fader hasn't picked this level up yet">↕</b>{/if}</span>
      <HSlider value={part.volume} tip={volumeTip(index)} label="{part.name} level" onchange={(v) => app.send({ type: 'setPartVolume', part: index, volume: v })} />
    </span>
  </div>
  <div class="mix">
    <span class="ctl"><span class="k">Pan</span>
      <HSlider value={part.pan} tip="mixer.part.pan" label="{part.name} pan" unity={64} format={panLabel} onchange={(v) => app.send({ type: 'setPartPan', part: index, pan: v })} />
    </span>
    <span class="ctl"><span class="k">Rev</span>
      <HSlider value={part.reverb} tip="mixer.part.reverb" label="{part.name} reverb" onchange={(v) => app.send({ type: 'setPartSend', part: index, send: 'reverb', value: v })} />
    </span>
    <span class="ctl"><span class="k">Cho</span>
      <HSlider value={part.chorus} tip="mixer.part.chorus" label="{part.name} chorus" onchange={(v) => app.send({ type: 'setPartSend', part: index, send: 'chorus', value: v })} />
    </span>
  </div>

  <div class="more">
    <span class="k">Oct</span>
    <button type="button" class="mini sq mat-raised" aria-label="{part.name} octave down" aria-disabled={part.octave <= -2} use:tip={'part.octave_down'} onclick={() => octave(-1)}>−</button>
    <span class="num" aria-label="octave {octaveLabel(part.octave)}">{octaveLabel(part.octave)}</span>
    <button type="button" class="mini sq mat-raised" aria-label="{part.name} octave up" aria-disabled={part.octave >= 2} use:tip={'part.octave_up'} onclick={() => octave(1)}>+</button>
    <span class="sep"></span>
    <span class="k">Voice</span>
    <button type="button" class="mini sq mat-raised" aria-label="{part.name} previous voice" use:tip={'part.voice_down'} onclick={() => stepVoice(-1)}>−</button>
    <button type="button" class="mini sq mat-raised" aria-label="{part.name} next voice" use:tip={'part.voice_up'} onclick={() => stepVoice(1)}>+</button>
    {#if plugin}
      <span class="grow"></span>
      <button type="button" class="mini mat-raised" use:tip={'part.plugin_reload'} onclick={() => app.send({ type: 'reloadPartPlugin', part: index })}>Reload</button>
      <button
        type="button"
        class="mini mat-raised"
        class:lit={!!entry?.inProcess}
        class:pending
        aria-pressed={!!entry?.inProcess}
        aria-disabled={!entry?.canRunInProcess}
        aria-label={pending ? 'In proc (applies on next load)' : undefined}
        use:tip={'part.plugin_in_process'}
        onclick={toggleInProcess}>In proc{pending ? ' ↻' : ''}</button
      >
    {/if}
  </div>
</div>

<style>
  .slot {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    padding: 0.5rem 0.55rem;
    border-radius: 8px;
    border: 1px solid var(--seam);
    background: color-mix(in srgb, var(--well) 22%, transparent);
  }
  .slot.focus {
    border-color: var(--accent);
  }
  .slot.off .glow-text {
    color: var(--screen-dim);
    text-shadow: none;
  }
  .flash {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    opacity: 0;
    pointer-events: none;
  }
  .flash.go {
    animation: flash 0.9s ease-out;
  }
  @keyframes flash {
    from {
      opacity: 1;
    }
    to {
      opacity: 0;
    }
  }
  .head,
  .acts,
  .more {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    min-width: 0;
  }
  .grow {
    flex: 1;
  }
  .pt {
    display: inline-flex;
    align-items: center;
    gap: 0.35em;
    flex: none;
    min-height: 2rem;
    padding: 0 0.55em;
    border-radius: 5px;
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 0.8rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--ink);
  }
  .pt .dot {
    width: 0.5em;
    height: 0.5em;
    border-radius: 50%;
    background: var(--lamp-off);
  }
  .pt.pressed .dot {
    background: var(--accent);
    box-shadow: 0 0 6px var(--accent);
  }
  .snbtn {
    flex: 1;
    min-width: 0;
    min-height: 1.9rem;
    padding: 0.1rem 0.45rem;
    border: 0;
    border-radius: 5px;
    text-align: left;
    cursor: pointer;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.95rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .snbtn.missing .glow-text {
    color: var(--danger, #e66);
  }
  .snbtn:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .inst {
    min-width: 0;
    font-size: 0.72rem;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .badge {
    flex: none;
    padding: 0.05em 0.45em;
    border-radius: 4px;
    border: 1px solid var(--seam);
    font-family: var(--font-display);
    font-size: 0.66rem;
    font-weight: 700;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .badge.b-mine {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 50%, transparent);
  }
  .dotmark,
  .status {
    font-size: 0.72rem;
    color: var(--accent);
    white-space: nowrap;
  }
  .status {
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .warn {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4rem;
    padding: 0.35rem 0.45rem;
    border-radius: 5px;
    background: color-mix(in srgb, var(--danger, #e66) 15%, transparent);
    font-size: 0.78rem;
  }
  .warn small {
    color: var(--muted);
  }
  .levelrow {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 0.6rem;
  }
  .levelrow > :global(.toggle) {
    min-height: 2rem;
  }
  .mix {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    align-items: center;
    gap: 0.35rem 0.5rem;
  }
  .ctl {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 0.3rem;
    min-width: 0;
  }
  /* The drawer's sliders, compact: a narrower readout and gap leave room for the track. */
  .ctl :global(.hslider) {
    gap: 0.25rem;
  }
  .ctl :global(.track) {
    height: 1.9rem;
  }
  .ctl :global(.readout) {
    width: 2.2rem;
    height: 1.4rem;
    font-size: 0.8rem;
  }
  .sep {
    width: 0.6rem;
  }
  .k {
    font-family: var(--font-display);
    font-size: 0.68rem;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--engrave, var(--muted));
    white-space: nowrap;
  }
  .pick {
    margin-left: 0.2em;
    color: var(--accent);
  }
  .num {
    min-width: 1.6em;
    text-align: center;
    font-family: var(--font-display);
    font-weight: 600;
  }
  .mini {
    min-height: 1.9rem;
    padding: 0 0.6em;
    border-radius: 5px;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.78rem;
    color: var(--ink);
    white-space: nowrap;
  }
  .mini.sq {
    width: 1.9rem;
    padding: 0;
    font-size: 1rem;
  }
  .mini[aria-disabled='true'] {
    opacity: 0.4;
    cursor: default;
  }
  .mini.lit {
    color: var(--accent);
  }
  .mini.pending {
    opacity: 0.7;
    font-style: italic;
  }
</style>
