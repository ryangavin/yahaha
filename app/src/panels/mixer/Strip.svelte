<!--
  One compact strip of the always-visible mixer row (MixerRow.svelte). All 12 parts are
  equal: strip 0–3 the keyboard parts (Right 1–3, Left), 4–11 the Style parts. Top to
  bottom: the part's colour and name (selects the part), its voice, the reserved insert
  slot, Pan and the small REV and DLY sends, the fader (the channel's CC 7, with the
  soft-takeover mark and the Launchkey ghost) with its meter, then On and Solo.
  Everything comes from the engine's state; the strip only sends commands.
-->
<script lang="ts">
  import type { PartSend } from '../../lib/api/types'
  import { app, ui } from '../../lib/store.svelte'
  import { tipFor } from '../../help/actions'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import Fader from '../../lib/ui/Fader.svelte'
  import FxKnob from './FxKnob.svelte'
  import { isKeyboard, meterFill, PART_COLORS, stylePartOf } from './parts'
  import { partVoice, styleVoice, voiceName, voiceTitle } from './voice'

  let {
    part,
    level = null,
    hw = null,
  }: {
    /** Strip index: 0–3 the keyboard parts, 4–11 the Style parts (Rhythm 1 … Phrase 2). */
    part: number
    /** The channel's peak (linear), or null without meters: a static dim bar. */
    level?: number | null
    /** Where the Launchkey fader for this part physically is (0–127), only while the
     *  Launchkey's live fader page maps to it; null otherwise. */
    hw?: number | null
  } = $props()

  const keyboard = $derived(isKeyboard(part))
  const k = $derived(keyboard ? app.state.keyboardParts[part] : null)
  const sIdx = $derived(stylePartOf(part))
  const s = $derived(keyboard ? null : app.state.mixer.styleParts[sIdx])
  const mixer = $derived(app.state.mixer)

  const name = $derived(k?.name ?? s?.name ?? '')
  const voice = $derived(k ? partVoice(k) : styleVoice(s?.voice ?? null))
  const selected = $derived(ui.selectedPart === part)
  const colour = $derived(PART_COLORS[part])
  const isOn = $derived(k ? k.on : (s?.on ?? false))
  /** The lamp: a keyboard part lights while it sounds (Left playing Manual Bass too). */
  const lit = $derived(k ? k.sounding : !!s && s.on && !s.mutedByManualBass && (mixer.styleSolo === null || mixer.styleSolo === sIdx))
  const isSolo = $derived(k ? mixer.partSolo === part : mixer.styleSolo === sIdx)

  const panText = (v: number) => (v === 64 ? 'C' : v < 64 ? `L${64 - v}` : `R${v - 64}`)

  function select() {
    ui.selectedPart = part
    if (keyboard) app.send({ type: 'selectPart', part })
  }
  function send(which: PartSend, v: number) {
    if (keyboard) app.send({ type: 'setPartSend', part, send: which, value: v })
    else app.send({ type: 'setStylePartSend', part: sIdx, send: which, value: v })
  }
  function volume(v: number) {
    if (keyboard) app.send({ type: 'setPartVolume', part, volume: v })
    else app.send({ type: 'setStylePartVolume', part: sIdx, volume: v })
  }
  function toggle() {
    if (keyboard) app.send({ type: 'togglePart', part })
    else app.send({ type: 'toggleStylePart', part: sIdx })
  }
  function solo() {
    if (keyboard) app.send({ type: 'setPartSolo', part: isSolo ? null : part })
    else app.send({ type: 'setStyleSolo', part: isSolo ? null : sIdx })
  }
  /** A Style part's double-click on a send hands its sends back to the style. */
  const styleReset = $derived(keyboard ? null : () => app.send({ type: 'resetStylePartSends', part: sIdx }))
  const own = (which: PartSend) => !!s?.sendsSet.includes(which)
</script>

<div class="strip" class:selected style:--part={colour} data-part={part}>
  <button
    type="button"
    class="name"
    aria-pressed={selected}
    title={name}
    use:tip={'mixer.strip.select'}
    onclick={select}
  >
    <span class="colour" aria-hidden="true"></span>
    <span class="text">{name}</span>
  </button>

  {#if keyboard}
    <button
      type="button"
      class="voice"
      title={voiceTitle(voice)}
      aria-label="{name} voice: {voiceTitle(voice)}"
      use:tip={'mixer.strip.voice'}
      onclick={() => ui.openLibrary('sounds', part)}>{voiceName(voice)}</button
    >
  {:else}
    <span class="voice" title={voiceTitle(voice)} use:tip={'mixer.strip.voice'}>{voiceName(voice)}</span>
  {/if}

  <!-- The part's two insert slots (from the mixer-strips contract) go here: another lane fills it. -->
  <div class="inserts"></div>

  <!-- Pan and the send minis. Room is left in this row for more send minis later (Chorus…). -->
  <div class="knobs">
    {#if k}
      <FxKnob value={k.pan} tip="mixer.part.pan" label="{name} pan" caption="Pan" reset={64} centre format={panText} onchange={(v) => app.send({ type: 'setPartPan', part, pan: v })} />
      <!-- Dry by default: a keyboard part's sends are 0 until the player or data sets one. -->
      <FxKnob value={k.reverb} tip="mixer.part.reverb" label="{name} reverb" caption="Rev" reset={0} onchange={(v) => send('reverb', v)} />
      <FxKnob value={k.variation} tip="mixer.part.variation" label="{name} delay" caption="Dly" reset={0} onchange={(v) => send('variation', v)} />
    {:else if s}
      <!-- The API has no Style part pan: keep its place so the knobs line up. -->
      <span class="no-pan" aria-hidden="true"></span>
      <FxKnob value={s.reverb} tip="mixer.style.reverb" label="{name} reverb" caption="Rev" reset={40} onchange={(v) => send('reverb', v)} onreset={styleReset} own={own('reverb')} />
      <FxKnob value={s.variation} tip="mixer.style.variation" label="{name} delay" caption="Dly" reset={0} onchange={(v) => send('variation', v)} onreset={styleReset} own={own('variation')} />
    {/if}
  </div>

  <div class="level">
    <div class="fader">
      {#if k}
        <Fader value={k.volume} tip={tipFor({ type: 'setPartVolume', part, volume: 0 })} label={name} pickup={k.waiting} {hw} lit={k.sounding} onchange={volume} />
      {:else if s}
        <Fader value={s.volume} tip="mixer.style.volume" label={name} pickup={s.waiting} {hw} lit={lit} onchange={volume} />
      {/if}
    </div>
    <div class="meter" class:idle={level === null} data-testid="meter" aria-hidden="true">
      <div class="fill" style:transform="scaleY({level === null ? 0 : meterFill(level)})"></div>
    </div>
  </div>

  <div class="buttons">
    <button
      type="button"
      class="on"
      class:lit
      aria-pressed={isOn}
      aria-label="{name} {isOn ? 'on' : 'off'}"
      use:tip={k ? tipFor({ type: 'togglePart', part }) : 'mixer.style.mute'}
      onclick={toggle}
    >
      <span class="lamp" aria-hidden="true"></span>On
    </button>
    <button
      type="button"
      class="solo"
      class:lit={isSolo}
      aria-pressed={isSolo}
      aria-label="Solo {name}"
      use:tip={'mixer.solo'}
      onclick={solo}>S</button
    >
  </div>
</div>

<style>
  .strip {
    container-type: inline-size;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    min-width: 0;
    height: 100%;
    padding: 0 0.2rem 0.3rem;
    border-radius: 6px;
    background: var(--panel);
    box-shadow: inset 0 0 0 1px var(--line);
    overflow: hidden;
  }
  /* The selected part: a lit accent edge. */
  .strip.selected {
    box-shadow:
      inset 0 0 0 1px var(--accent),
      0 0 8px color-mix(in srgb, var(--accent) 45%, transparent);
  }

  button {
    font: inherit;
    color: inherit;
    border: none;
    background: none;
    padding: 0;
    cursor: pointer;
  }
  .name {
    display: grid;
    gap: 0.2rem;
    margin: 0 -0.2rem;
    min-width: 0;
    padding-bottom: 0.1rem;
    text-align: center;
  }
  .colour {
    display: block;
    height: 4px;
    background: var(--part);
  }
  .selected .colour {
    box-shadow: 0 0 6px var(--part);
  }
  .text,
  .voice {
    display: block;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .text {
    padding: 0 0.2rem;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.8rem;
    color: var(--ink);
  }
  .selected .text {
    color: var(--accent);
  }
  .voice {
    width: 100%;
    font-family: var(--font-display);
    font-size: 0.72rem;
    line-height: 1.2;
    text-align: center;
    color: var(--muted);
  }
  button.voice:hover {
    color: var(--ink);
  }
  /* Reserved for the two insert slots. */
  .inserts {
    flex: none;
    height: 1.1rem;
  }
  .knobs {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.05rem;
    align-items: start;
  }
  .no-pan {
    display: block;
  }
  .level {
    flex: 1 1 auto;
    /* Shorter with the row's details shown; the fader takes whatever height is left. */
    min-height: 4.5rem;
    display: flex;
    justify-content: center;
    gap: 0.2rem;
  }
  /* The fader scales with the strip (82 px wide: full size; 56 px: about two thirds). */
  .fader {
    min-width: 0;
    height: 100%;
    display: flex;
    justify-content: center;
    font-size: clamp(0.6rem, 16cqi, 0.85rem);
  }
  /* A shorter least track than the Fader's own, so a whole strip fits under the row's
     details; it still grows to the strip's full height without them. */
  .fader :global(.track) {
    min-height: 4.5em;
  }
  /* The name is on the strip's head already. */
  .fader :global(.name) {
    display: none;
  }
  .meter {
    position: relative;
    flex: none;
    width: 5px;
    margin: 1.6rem 0 0.4rem;
    border-radius: 2px;
    background: var(--lamp-off);
    overflow: hidden;
  }
  .meter.idle {
    opacity: 0.45;
  }
  .fill {
    position: absolute;
    inset: 0;
    transform-origin: bottom;
    background: linear-gradient(0deg, #3fd67a 0 70%, #ffd23f 70% 90%, #ff5a5a 90%);
    will-change: transform;
  }
  .buttons {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 0.2rem;
  }
  .on,
  .solo {
    height: 1.5rem;
    border-radius: 4px;
    background: var(--raised);
    box-shadow: inset 0 0 0 1px var(--line);
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.75rem;
    color: var(--muted);
  }
  .on {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.25rem;
    min-width: 0;
  }
  .lamp {
    width: 0.45rem;
    height: 0.45rem;
    border-radius: 50%;
    background: var(--lamp-off);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .on.lit {
    color: var(--ink);
  }
  .on.lit .lamp {
    background: var(--part);
    box-shadow: 0 0 6px var(--part);
  }
  .solo {
    min-width: 1.5rem;
    padding: 0 0.3rem;
  }
  /* The Genos lights a soloed channel purple. */
  .solo.lit {
    color: #fff;
    background: var(--solo);
    box-shadow: 0 0 8px var(--solo);
  }
</style>
