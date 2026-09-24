<!--
  One keyboard part as a channel strip: Edit (the part Voice −/+ changes), on/off, the
  voice (a small screen with the voice picker over it), octave shift, and the volume
  fader, which is fader 1–4 on the Launchkey's Panel page. Hovering or focusing the strip
  lights that fader on the mirror (lib/mirror).
-->
<script lang="ts">
  import type { KeyboardPart } from '../../lib/api/types'
  import { voiceGroups, type VoiceEntry } from '../../lib/api/voices'
  import { mirror } from '../../lib/mirror.svelte'
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import Fader from '../../lib/ui/Fader.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'
  import { byCategory } from '../sound/nav.svelte'
  import { launchkeyPlace, octaveLabel, onTip, selectTip, volumeTip } from './parts'

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

  const groups = $derived(voiceGroups(voices))
  // The sound library's patches come first (the Library tab of the voice list, #103).
  const library = $derived(byCategory(app.state.soundLibrary.patches))
  const pickValue = $derived(part.patch ? `patch:${part.patch}` : `gm:${part.program}`)
  const own = $derived(voices.find((v) => v.program === part.program)?.name ?? `Program ${part.program + 1}`)

  function pick(e: Event & { currentTarget: HTMLSelectElement }) {
    const [kind, v] = e.currentTarget.value.split(/:(.*)/s)
    if (kind === 'patch') app.send({ type: 'setPartPatch', part: index, id: v })
    else app.send({ type: 'setPartVoice', part: index, program: Number(v) })
    // Give the performance keys back (a focused select keeps them).
    e.currentTarget.blur()
  }
  // A focused select swallows keys (the app's shortcuts skip form fields) and type-ahead
  // would turn a performance key into a voice change: `b` picks Bagpipe, `t` Taiko. The
  // picker is still focused after a mouse pick of the same voice or a dismissed list, so
  // it only keeps the keys that move through the list; any other key lets go of it and
  // goes to the performance shortcuts as if nothing had been focused.
  const LIST_KEYS = new Set(['ArrowUp', 'ArrowDown', 'Home', 'End', 'Tab', 'Shift', 'Control', 'Alt', 'Meta'])
  function pickerKey(e: KeyboardEvent & { currentTarget: HTMLSelectElement }) {
    if (LIST_KEYS.has(e.key) || e.ctrlKey || e.altKey || e.metaKey) return
    e.preventDefault()
    e.stopPropagation()
    e.currentTarget.blur()
    const { key, code, shiftKey, repeat } = e
    window.dispatchEvent(new KeyboardEvent('keydown', { key, code, shiftKey, repeat, cancelable: true }))
  }
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

  <label class="voice mat-screen">
    <span class="glow-text vname">{part.voiceName}</span>
    <span class="sub">
      {#if part.playsBass}<span class="badge">own: {own}</span>{:else if part.patch}<span class="badge">Library</span> ▾{:else}Voice ▾{/if}
    </span>
    <select value={pickValue} aria-label="{part.name} voice" use:tip={part.patch ? 'part.library' : 'part.voice'} onchange={pick} onkeydown={pickerKey}>
      {#each library as g (g.category)}
        <optgroup label="Library · {g.label}">
          {#each g.patches as p (p.id)}<option value="patch:{p.id}">{p.name}</option>{/each}
        </optgroup>
      {/each}
      {#each groups as g (g.family)}
        <optgroup label={g.family}>
          {#each g.voices as v (v.program)}<option value="gm:{v.program}">{v.name}</option>{/each}
        </optgroup>
      {/each}
    </select>
  </label>

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
  /* The voice screen, with a transparent native picker laid over it. */
  .voice {
    position: relative;
    display: flex;
    flex-direction: column;
    justify-content: center;
    min-height: 2.9rem;
    padding: 0.25rem 0.4rem;
    border-radius: 5px;
    cursor: pointer;
  }
  .voice:has(select:focus-visible) {
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
  select {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    opacity: 0;
    cursor: pointer;
    font-size: 0.9rem;
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
