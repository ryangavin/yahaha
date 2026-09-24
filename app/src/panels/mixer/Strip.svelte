<!--
  One mixer channel strip, like a Genos Mixer channel: the MIDI out channel, a big fader
  that is the channel's CC 7 (with the soft-takeover mark and the ghost of where the
  Launchkey fader physically sits), On and Solo, and the voice. Everything comes from the
  engine's state; the strip only sends commands.
-->
<script lang="ts">
  import type { TipKey } from '../../help/tooltips'
  import type { Pad } from '../../lib/api/types'
  import { clock } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import Fader from '../../lib/ui/Fader.svelte'
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
  } = $props()
</script>

<div class="strip" class:unused>
  <div class="ch" use:tip={unused ? 'launchkey.fader_unused' : 'mixer.channel'}>
    {#if channel !== null}<span class="engraved">Ch</span> <b>{channel}</b>{:else}&nbsp;{/if}
  </div>

  <div class="fader">
    <Fader {value} tip={faderTip} label={name} pickup={waiting} {hw} {lit} disabled={unused} {onchange} />
  </div>

  <div class="buttons">
    {#if on}
      <HwButton tip={on.tip} led={on.led} beats={clock.beats} onclick={on.onclick} label="{name} {on.isOn ? 'on' : 'off'}">
        {on.isOn ? 'On' : 'Off'}
      </HwButton>
      <!-- Solo: the engine has none yet (#30). Disabled, but still hoverable for its tooltip. -->
      <button type="button" class="solo mat-raised" aria-disabled="true" aria-label="Solo {name} (not available yet)" use:tip={'mixer.solo'}>S</button>
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
    opacity: 0.5;
    cursor: not-allowed;
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
