<!--
  One keyboard part as a channel strip: Edit (the part Voice −/+ changes), on/off, the
  voice (a small screen that opens the Sound Browser, #117: SoundFont presets, plugins
  and saved sounds in one list; a plugin part also gets its editor window here),
  octave shift, and the volume fader, which is fader 1–4 on the Launchkey's Panel page. Hovering or focusing the strip
  lights that fader on the mirror (lib/mirror).
-->
<script lang="ts">
  import type { KeyboardPart } from '../../lib/api/types'
  import type { VoiceEntry } from '../../lib/api/voices'
  import { mirror } from '../../lib/mirror.svelte'
  import { app, ui } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import Fader from '../../lib/ui/Fader.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'
  import { inProcessPending, launchkeyPlace, octaveLabel, onTip, pluginStatusLine, selectTip, volumeTip } from './parts'

  let {
    part,
    index,
    voices,
    hw = null,
    recalled = 0,
  }: {
    part: KeyboardPart
    index: number
    voices: VoiceEntry[]
    /** Where the Launchkey fader physically is, when known (for the pickup mark). */
    hw?: number | null
    /** Bumped on every OTS recall: the strip flashes once. */
    recalled?: number
  } = $props()

  const plugins = $derived(app.state.plugins)
  const plugin = $derived(part.plugin)
  const pluginLine = $derived(pluginStatusLine(plugin, plugins.available).replace(/ ▾$/, ''))
  // The part's plugin in the scan list: its "run in process" override (#141).
  const entry = $derived(plugin ? plugins.list.find((p) => p.id === plugin.id) : undefined)
  // Changed since the plugin loaded: it applies from the next load (#176).
  const pending = $derived(inProcessPending(plugin, entry))
  function toggleInProcess() {
    if (entry?.canRunInProcess) app.send({ type: 'setPluginInProcess', id: entry.id, inProcess: !entry.inProcess })
  }
  const own = $derived(voices.find((v) => v.program === part.program)?.name ?? `Program ${part.program + 1}`)
  const octave = (d: number) => app.send({ type: 'setPartOctave', part: index, octave: Math.max(-2, Math.min(2, part.octave + d)) })
  const link = (on: boolean) => (mirror.panelFader = on ? index : mirror.panelFader === index ? null : mirror.panelFader)
  // Closing the drawer under the pointer never fires pointerleave: let go of the mirror.
  $effect(() => () => link(false))
</script>

<div
  class="strip"
  class:off={!part.sounding}
  class:selected={part.selected}
  role="group"
  aria-label={part.name}
  onpointerenter={() => link(true)}
  onpointerleave={() => link(false)}
  onfocusin={() => link(true)}
  onfocusout={() => link(false)}
>
  {#key recalled}<span class="flash" class:go={recalled > 0} aria-hidden="true"></span>{/key}

  <div class="head">
    <span class="name">{part.name}</span>
    <span class="ch engraved">ch {part.channel}</span>
  </div>

  <button
    type="button"
    class="edit mat-raised"
    class:pressed={part.selected}
    aria-pressed={part.selected}
    use:tip={selectTip(index)}
    onclick={() => app.send({ type: 'selectPart', part: index })}
  >
    <span class="dot" aria-hidden="true"></span>{part.selected ? 'Editing' : 'Edit'}
  </button>

  <Toggle on={part.sounding} tip={onTip(index)} onclick={() => app.send({ type: 'togglePart', part: index })}>
    {part.playsBass ? 'Bass' : part.on ? 'On' : 'Off'}
  </Toggle>

  <button
    type="button"
    class="voice mat-screen"
    class:failed={plugin?.status === 'failed' || plugin?.status === 'muted'}
    aria-label="{part.name} sound: {part.voiceName}"
    use:tip={'part.voice'}
    onclick={() => (ui.soundBrowser = index)}
  >
    <span class="glow-text vname">{part.voiceName}</span>
    <span class="sub">
      {#if part.playsBass}<span class="badge">own: {own}</span>
      {:else if plugin}<span class="badge">{part.patch ? 'Saved' : 'Plugin'}</span> {pluginLine}
      {:else if part.patch}<span class="badge">Saved</span> ▾
      {:else}Sounds ▾{/if}
    </span>
  </button>
  {#if plugin}
    <div class="prow">
      <button type="button" class="mini mat-raised wide" aria-disabled={!plugin.editor} use:tip={'part.plugin_edit'} onclick={() => plugin.editor && app.pluginEditor(index, true)}>Edit…</button>
      <button type="button" class="mini mat-raised wide" class:on={!!entry?.inProcess} class:pending aria-pressed={!!entry?.inProcess} aria-disabled={!entry?.canRunInProcess} aria-label={pending ? 'In proc (applies on next load)' : undefined} use:tip={'part.plugin_in_process'} onclick={toggleInProcess}>In proc{pending ? ' ↻' : ''}</button>
    </div>
  {/if}

  <div class="octave">
    <button type="button" class="mini mat-raised" aria-label="{part.name} octave down" aria-disabled={part.octave <= -2} use:tip={'part.octave_down'} onclick={() => octave(-1)}>−</button>
    <span class="oval" aria-label="octave {octaveLabel(part.octave)}"><small class="engraved">Oct</small>{octaveLabel(part.octave)}</span>
    <button type="button" class="mini mat-raised" aria-label="{part.name} octave up" aria-disabled={part.octave >= 2} use:tip={'part.octave_up'} onclick={() => octave(1)}>+</button>
  </div>

  <div class="fader">
    <Fader
      value={part.volume}
      tip={volumeTip(index)}
      label="Vol"
      pickup={part.waiting}
      {hw}
      lit={part.sounding}
      onchange={(v) => app.send({ type: 'setPartVolume', part: index, volume: v })}
    />
  </div>
  <span class="lk engraved">{launchkeyPlace(index)}</span>
</div>

<style>
  .voice.failed .vname {
    color: var(--danger, #e66);
  }
  .prow {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.25rem;
  }
  .mini.wide {
    width: auto;
    font-size: 0.7rem;
  }
  /* In proc on: lit, since a crash there takes yahaha down. */
  .mini.wide.on {
    color: var(--accent);
  }
  /* Changed since the plugin loaded: dimmed until its next load. */
  .mini.wide.pending {
    opacity: 0.7;
    font-style: italic;
  }
  .strip {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0.45rem;
    min-width: 0;
    padding: 0.5rem 0.4rem 0.45rem;
    border-radius: 8px;
    border: 1px solid var(--seam);
    background: color-mix(in srgb, var(--well) 22%, transparent);
  }
  .strip.selected {
    border-color: var(--accent);
  }
  /* The OTS recall flash: an opacity-only wash that fades out once. */
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
  .head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.3rem;
  }
  .name {
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 1.02rem;
    letter-spacing: 0.02em;
    white-space: nowrap;
  }
  .ch {
    font-size: 0.68rem;
  }
  .edit {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.4em;
    min-height: 2.2rem;
    border-radius: 5px;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.88rem;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--ink);
  }
  .edit .dot {
    width: 0.5em;
    height: 0.5em;
    border-radius: 50%;
    background: var(--lamp-off);
  }
  .edit.pressed {
    outline: 1px solid var(--accent);
  }
  .edit.pressed .dot {
    background: var(--accent);
    box-shadow: 0 0 6px var(--accent);
  }
  .strip :global(.toggle) {
    justify-content: center;
    min-height: 2.2rem;
  }
  /* The voice screen: it opens the Sound Browser. */
  .voice {
    position: relative;
    display: flex;
    flex-direction: column;
    justify-content: center;
    min-height: 2.9rem;
    padding: 0.25rem 0.4rem;
    border: 0;
    border-radius: 5px;
    text-align: left;
    cursor: pointer;
  }
  .voice:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .vname {
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.98rem;
    line-height: 1.1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .off .vname {
    color: var(--screen-dim);
    text-shadow: none;
  }
  .sub {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 0.66rem;
    color: var(--screen-dim);
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }
  .badge {
    color: var(--accent);
  }
  .octave {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    gap: 0.25rem;
  }
  .mini {
    min-height: 2.2rem;
    border-radius: 5px;
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 1.05rem;
    color: var(--ink);
  }
  .mini[aria-disabled="true"] {
    opacity: 0.4;
  }
  .oval {
    display: flex;
    flex-direction: column;
    align-items: center;
    min-width: 2.1em;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 1rem;
    line-height: 1;
  }
  .oval small {
    font-size: 0.6rem;
  }
  .fader {
    height: 10.5rem;
    font-size: 14px;
  }
  .lk {
    text-align: center;
    font-size: 0.66rem;
  }
</style>
