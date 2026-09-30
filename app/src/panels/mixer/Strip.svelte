<!--
  One compact strip of the always-visible mixer row (MixerRow.svelte). All 12 parts are
  equal: strip 0–3 the keyboard parts (Right 1–3, Left), 4–11 the Style parts. Top to
  bottom: the part's colour and name (selects the part), its voice (a keyboard part's with
  ⚠ plugin missing and ● sound edited, as the Launchkey fader bank showed), the reserved insert
  slot, the layer tag and the hardware fader badge, the fader with its meter, then On and
  Solo. Everything comes from the engine's state; the strip only sends commands.
  The fader shows the mixer's fader layer (`mixer.faderLayer`, stepped from the mixer bar
  or the Launchkey): VOL is the channel's CC 7 as always (no tag); PAN, REV, CHO and DLY
  move the part's pan or send instead, tagged small above the fader, with the value in the
  readout (pan as L/C/R). A Style part has no pan: its fader is unused in PAN. In PAN and
  the send layers a double-click puts the value back (pan to centre, a keyboard part's
  send to 0, a Style part's sends back to the style's). The soft-takeover mark and the
  Launchkey ghost follow the layer too.
-->
<script lang="ts">
  import type { FaderLayer, PartSend } from '../../lib/api/types'
  import { app, ui } from '../../lib/store.svelte'
  import { tipFor } from '../../help/actions'
  import type { TipKey } from '../../help/tooltips'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import Fader from '../../lib/ui/Fader.svelte'
  import { isMissing, soundLabel } from '../rack/rack'
  import { isKeyboard, meterFill, PART_COLORS, stylePartOf } from './parts'
  import { partVoice, styleVoice, voiceName, voiceTitle } from './voice'

  let {
    part,
    level = null,
    hw = null,
    badge = null,
  }: {
    /** Strip index: 0–3 the keyboard parts, 4–11 the Style parts (Rhythm 1 … Phrase 2). */
    part: number
    /** The channel's peak (linear), or null without meters: a static dim bar. */
    level?: number | null
    /** Where the Launchkey fader for this part physically is (0–127), only while the
     *  Launchkey's live fader page maps to it; null otherwise. */
    hw?: number | null
    /** The Launchkey fader this part is on, on the current page ("F1"–"F8"); null: none. */
    badge?: string | null
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
  /** The sound's marks (keyboard parts): its plugin is missing, its sound is edited. */
  const missing = $derived(!!k && isMissing(k))
  const marked = $derived(!!k && (missing || !!k.soundEdited))
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

  const LAYER_TAG: Record<FaderLayer, string> = { volume: 'VOL', pan: 'PAN', reverb: 'REV', chorus: 'CHO', delay: 'DLY' }
  const LAYER_SEND: Record<FaderLayer, PartSend | null> = { volume: null, pan: null, reverb: 'reverb', chorus: 'chorus', delay: 'variation' }
  const LAYER_WORD: Record<FaderLayer, string> = { volume: '', pan: ' pan', reverb: ' reverb', chorus: ' chorus', delay: ' delay' }

  const layer = $derived<FaderLayer>(mixer.faderLayer ?? 'volume')
  /** In PAN and the send layers: this part's fader hasn't reached the value yet. */
  const sendWaiting = $derived(k ? (mixer.sendWaiting & (1 << part)) !== 0 : (mixer.styleSendWaiting & (1 << sIdx)) !== 0)

  /** What the fader shows and moves in a layer other than VOL; null in VOL. */
  const alt = $derived.by((): {
    value: number
    tip: TipKey
    /** The readout's text in place of the number (pan as L/C/R). */
    text: string | null
    disabled: boolean
    own: boolean
    set: (v: number) => void
    reset: () => void
  } | null => {
    if (layer === 'volume') return null
    if (layer === 'pan') {
      if (k) return { value: k.pan, tip: 'mixer.part.pan', text: panText(k.pan), disabled: false, own: false, set: (v) => app.send({ type: 'setPartPan', part, pan: v }), reset: () => app.send({ type: 'setPartPan', part, pan: 64 }) }
      // The API has no Style part pan: the fader is unused here, as on the Launchkey.
      return { value: 64, tip: 'mixer.layer', text: '—', disabled: true, own: false, set: () => {}, reset: () => {} }
    }
    const which = LAYER_SEND[layer]!
    if (k) return { value: k[which], tip: tipFor({ type: 'setPartSend', part, send: which, value: 0 }), text: null, disabled: false, own: false, set: (v) => send(which, v), reset: () => send(which, 0) }
    if (s) return { value: s[which], tip: `mixer.style.${which}`, text: null, disabled: false, own: own(which), set: (v) => send(which, v), reset: () => styleReset?.() }
    return null
  })
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

  {#if k}
    <!-- A keyboard part's sound, with the marks the Launchkey fader bank showed: ⚠ its
         plugin is missing, ● its sound is edited from the rack. -->
    <button
      type="button"
      class="voice"
      class:marked
      class:edited={k.soundEdited}
      class:missing
      title={voiceTitle(voice)}
      aria-label="{name} sound: {soundLabel(k)}"
      use:tip={marked ? 'launchkey.fader_sound' : 'mixer.strip.voice'}
      onclick={() => ui.openLibrary('sounds', part)}
      >{#if missing}<span class="mark" data-mark="missing" aria-hidden="true">⚠</span>{/if}<span class="vname">{voiceName(voice)}</span
      >{#if k.soundEdited}<span class="mark" data-mark="edited" aria-hidden="true">●</span>{/if}</button
    >
  {:else}
    <span class="voice" title={voiceTitle(voice)} use:tip={'mixer.strip.voice'}>{voiceName(voice)}</span>
  {/if}

  <!-- The part's two insert slots (from the mixer-strips contract) go here: another lane fills it. -->
  <div class="inserts"></div>

  <!-- The fader layer showing (none in VOL) and the Launchkey fader this part is on. -->
  <div class="tags">
    {#if alt}
      <span class="layer" class:own={alt.own} data-layer={layer}>{LAYER_TAG[layer]}</span>
    {/if}
    {#if badge}
      <span class="badge" use:tip={'stage.fader_badge'}>{badge}</span>
    {/if}
  </div>

  <div class="level">
    {#if alt}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="fader" class:text={alt.text !== null} ondblclick={alt.reset}>
        <Fader value={alt.value} tip={alt.tip} label="{name}{LAYER_WORD[layer]}" pickup={sendWaiting} {hw} lit={k ? k.sounding : lit} disabled={alt.disabled} onchange={alt.set} />
        {#if alt.text !== null}
          <span class="fmt glow-text" class:dim={!(k ? k.sounding : lit) || alt.disabled} aria-hidden="true">{alt.text}</span>
        {/if}
      </div>
    {:else}
      <div class="fader">
        {#if k}
          <Fader value={k.volume} tip={tipFor({ type: 'setPartVolume', part, volume: 0 })} label={name} pickup={k.waiting} {hw} lit={k.sounding} onchange={volume} />
        {:else if s}
          <Fader value={s.volume} tip="mixer.style.volume" label={name} pickup={s.waiting} {hw} lit={lit} onchange={volume} />
        {/if}
      </div>
    {/if}
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
  /* Only the fader gives way to a short row: the rows above and below keep their height. */
  .name,
  .voice,
  .buttons {
    flex: none;
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
  /* With a mark, only the name is cut short: the marks always show. */
  .voice.marked {
    display: flex;
    justify-content: center;
    gap: 0.15rem;
  }
  .vname {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .mark {
    flex: none;
  }
  .voice.edited {
    color: var(--accent);
  }
  .voice.missing {
    color: var(--danger, #e66);
  }
  /* Reserved for the two insert slots. */
  .inserts {
    flex: none;
    height: 1.1rem;
  }
  /* The layer tag (left) and the hardware fader badge (right): small, one line. */
  .tags {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.2rem;
    height: 0.95rem;
    min-width: 0;
    padding: 0 0.1rem;
  }
  .layer {
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 0.62rem;
    letter-spacing: 0.04em;
    color: var(--muted);
  }
  /* A Style part's own send (#268): not the style's. */
  .layer.own {
    color: var(--accent);
  }
  .layer.own::after {
    content: '•';
  }
  .badge {
    margin-left: auto;
    padding: 0 0.22rem;
    border-radius: 3px;
    background: var(--raised);
    box-shadow: inset 0 0 0 1px var(--line);
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 0.6rem;
    line-height: 0.95rem;
    color: var(--ink);
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
    position: relative;
    min-width: 0;
    height: 100%;
    display: flex;
    justify-content: center;
    font-size: clamp(0.6rem, 16cqi, 0.85rem);
  }
  /* PAN: the readout's text (L/C/R) laid over the Fader's own number, in the same box
     (the Fader's readout: 0.95em type, 1.45em high, at the top). */
  .fader.text :global(.readout .glow-text) {
    visibility: hidden;
  }
  .fmt {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 1.45em;
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.95em;
    color: var(--screen-ink);
    pointer-events: none;
  }
  .fmt.dim {
    color: var(--screen-dim);
  }
  /* A shorter least track than the Fader's own, so the readout and the track fit the
     level's least height (4.5rem) and a whole strip stands in 200 px; it still grows to
     the strip's full height when there's room. */
  .fader :global(.track) {
    min-height: 3.6em;
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
