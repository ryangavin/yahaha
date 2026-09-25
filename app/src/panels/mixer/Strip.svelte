<!--
  One mixer channel strip, like a Genos Mixer channel: the MIDI out channel, a big fader
  that is the channel's CC 7 (with the soft-takeover mark and the ghost of where the
  Launchkey fader physically sits), On and Solo, and the voice. A keyboard part's strip
  also has Pan, Reverb and Chorus knobs (CC 10, 91, 93) above the fader; `fxRow` keeps
  that row's space on a strip without them, so the faders line up. Everything comes from the
  engine's state; the strip only sends commands.
-->
<script lang="ts">
  import type { TipKey } from '../../help/tooltips'
  import type { Pad } from '../../lib/api/types'
  import { clock } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import Fader from '../../lib/ui/Fader.svelte'
  import FxKnob from './FxKnob.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import type { VoiceLines } from './voice'

  let {
    name,
    channel = null,
    value,
    waiting = false,
    hw = null,
    faderTip,
    onchange,
    lit = true,
    unused = false,
    on = null,
    button = null,
    voice = null,
    badge = null,
    solo = null,
    fx = null,
    fxRow = false,
  }: {
    name: string
    /** 1-based MIDI channel at yahaha's output; null for an unused strip. */
    channel?: number | null
    value: number
    waiting?: boolean
    /** Where the Launchkey fader physically is (0–127), when the engine says. */
    hw?: number | null
    faderTip: TipKey
    onchange: (v: number) => void
    lit?: boolean
    /** An unused Launchkey fader (Panel page, 5–8): drawn dim so the page reads 1:1. */
    unused?: boolean
    /** The on/off button: its light (the Launchkey fader button's), tooltip and command. */
    on?: { led: Pick<Pad, 'rgb' | 'level' | 'anim'> | null; isOn: boolean; tip: TipKey; onclick: () => void } | null
    /** Another function on the Launchkey fader button (an unused strip's: Panel button 5 is
     *  HARMONY/ARPEGGIO), drawn in place of On/Solo. */
    button?: { led: Pick<Pad, 'rgb' | 'level' | 'anim'> | null; text: string; label: string; tip: TipKey; onclick: () => void } | null
    voice?: VoiceLines | null
    /** A note under the name, e.g. "Manual Bass" when the engine mutes the Bass part. */
    badge?: { text: string; tip: TipKey } | null
    /** Solo: whether this part is the one soloed, and the command that toggles it. */
    solo?: { isSolo: boolean; onclick: () => void } | null
    /** Pan and the reverb/chorus sends (0–127), and what moving them sends. */
    fx?: {
      pan: number
      reverb: number
      chorus: number
      onpan: (v: number) => void
      onsend: (send: 'reverb' | 'chorus', v: number) => void
    } | null
    /** Keep the knob row's space when there are no knobs. */
    fxRow?: boolean
  } = $props()

  const panText = (v: number) => (v === 64 ? 'C' : v < 64 ? `L${64 - v}` : `R${v - 64}`)
</script>

<div class="strip" class:unused class:knobs={fx !== null || fxRow}>
  <div class="ch" use:tip={unused ? 'launchkey.fader_unused' : 'mixer.channel'}>
    {#if channel !== null}<span class="engraved">Ch</span> <b>{channel}</b>{:else}&nbsp;{/if}
  </div>

  {#if fx}
    <div class="fx">
      <FxKnob value={fx.pan} tip="mixer.part.pan" label="{name} pan" caption="Pan" reset={64} centre format={panText} onchange={fx.onpan} />
      <FxKnob value={fx.reverb} tip="mixer.part.reverb" label="{name} reverb" caption="Rev" reset={40} onchange={(v) => fx.onsend('reverb', v)} />
      <FxKnob value={fx.chorus} tip="mixer.part.chorus" label="{name} chorus" caption="Cho" reset={0} onchange={(v) => fx.onsend('chorus', v)} />
    </div>
  {:else if fxRow}
    <div class="fx" aria-hidden="true"></div>
  {/if}

  <div class="fader">
    <Fader {value} tip={faderTip} label={name} pickup={waiting} {hw} {lit} disabled={unused} {onchange} />
  </div>

  <div class="buttons">
    {#if on}
      <HwButton tip={on.tip} led={on.led} beats={clock.beats} onclick={on.onclick} label="{name} {on.isOn ? 'on' : 'off'}">
        {on.isOn ? 'On' : 'Off'}
      </HwButton>
      <button
        type="button"
        class="solo mat-raised"
        class:on={solo?.isSolo}
        aria-pressed={solo?.isSolo ?? false}
        aria-label="Solo {name}"
        use:tip={'mixer.solo'}
        onclick={() => solo?.onclick()}>S</button
      >
    {:else if button}
      <HwButton tip={button.tip} led={button.led} beats={clock.beats} onclick={button.onclick} label={button.label}>
        {button.text}
      </HwButton>
    {/if}
  </div>

  <div class="voice" use:tip={unused ? 'launchkey.fader_unused' : 'mixer.voice'}>
    {#if voice}
      <span class="plays">{voice.plays}</span>
      <span class="for">{voice.writtenFor}</span>
    {/if}
  </div>

  <div class="badge">
    {#if badge}<span class="tag" use:tip={badge.tip}>{badge.text}</span>{/if}
  </div>
</div>

<style>
  .strip {
    display: grid;
    grid-template-rows: auto minmax(13rem, 1fr) auto auto auto;
    justify-items: center;
    gap: 0.45rem;
    min-width: 0;
    padding: 0.5rem 0.25rem 0.4rem;
    border-radius: var(--r-key);
    background: linear-gradient(180deg, rgb(255 255 255 / 0.025), transparent 40%);
    box-shadow: inset 0 0 0 1px rgb(0 0 0 / 0.18);
  }
  /* The knob row sits between the channel and the fader. */
  .strip.knobs {
    grid-template-rows: auto var(--fx-h, 3.4rem) minmax(13rem, 1fr) auto auto auto;
  }
  .fx {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.1rem;
    width: 100%;
    height: var(--fx-h, 3.4rem);
    align-items: start;
  }
  .unused {
    opacity: 0.55;
  }
  .ch {
    font-family: var(--font-display);
    font-size: 0.85rem;
    color: var(--ink);
    white-space: nowrap;
  }
  .ch b {
    font-weight: 700;
  }
  .fader {
    height: 100%;
    min-height: 0;
    width: 100%;
    display: flex;
    justify-content: center;
    font-size: 0.95rem;
  }
  .buttons {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 0.25rem;
    width: 100%;
    min-height: 2.1rem;
    font-size: 0.9rem;
  }
  .buttons :global(.btn) {
    min-width: 0;
  }
  .solo {
    height: 2.1em;
    min-width: 1.9em;
    padding: 0 0.35em;
    border-radius: 5px;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.9em;
    color: var(--muted);
  }
  /* The Genos lights a soloed channel purple. */
  .solo.on {
    color: #fff;
    background: var(--solo);
    box-shadow: 0 0 8px var(--solo);
  }
  .voice {
    display: grid;
    width: 100%;
    min-height: 2.3rem;
    text-align: center;
    line-height: 1.15;
  }
  .plays,
  .for {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .plays {
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.85rem;
    color: var(--ink);
  }
  .for {
    font-family: var(--font-display);
    font-size: 0.75rem;
    color: var(--muted);
  }
  .badge {
    min-height: 1.1rem;
  }
  .tag {
    display: inline-block;
    font-family: var(--font-display);
    font-size: 0.7rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    padding: 0 0.35em;
    border-radius: 3px;
    color: var(--accent-ink);
    background: var(--accent);
  }
</style>
