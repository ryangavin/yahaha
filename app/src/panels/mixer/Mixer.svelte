<!--
  The Mixer drawer, laid out like the Genos Mixer's Panel and Style tabs.

  - The tab IS the Launchkey fader page (`state.mixer.faderPage`): switching tabs sends
    `setFaderPage`, so the hardware follows, and the Launchkey's page button switches the
    tab. There is no local copy of the page.
  - Panel: Right 1–3 and Left on faders 1–4, the Style volume on fader 5 (#199: a scale
    on every Style part's CC 7, like a fade; the button under it is HARMONY/ARPEGGIO, as on
    the Launchkey), the Multi Pad volume on fader 6 (#196, the same kind of scale on the
    pads' CC 7); 7–8 are unused on the hardware, drawn dim.
    Style: the eight band parts, Rhythm 1 … Phrase 2, on faders 1–8. Master on the right.
  - A fader is the channel's CC 7 (0–127) and nothing else: no hidden gain. Loading a
    style sets the Style faders to the style's own levels.
  - Soft takeover: ↕ while a level waits for its Launchkey fader, and a dashed ghost cap
    where that fader physically sits (from the provisional `state.surface`).
  - Panel strips have Pan, Reverb, Chorus and Delay knobs (the part's CC 10, 91, 93, 94:
    `setPartPan`, `setPartSend`, #198/#204); double-click one to put it back to its default.
  - Effects (#204): the shared effect bus's Reverb, Chorus and Variation (tempo delay)
    blocks, each with its type and return level (`setEffectType`, `setEffectReturn`). Every
    part, Panel and Style, SoundFont and plugin, feeds them through its sends. Each block's
    Band send (#236, `setBandSend`) scales every Style part's send to it, in percent: the
    band's reverb as written, its chorus and delay off until turned up.
  - Solo (S): only that part plays, even if it is off; the Style tab solos a band part,
    the Panel tab a keyboard part (`setStyleSolo` / `setPartSolo`, #30). Press again to end.
  - The metronome (on/off, bell, its own volume) sits above the strips: it is the
    built-in synth's click voice, never on the MIDI port. The Style tab adds Style Track
    Mute (a Genos Live Control knob, A/B order).
  - No level meters yet.
-->
<script lang="ts">
  import type { FaderPage, FxBlock, FxType, KeyboardPart, PartSend, StylePart, TrackMuteOrder } from '../../lib/api/types'
  import type { TipKey } from '../../help/tooltips'
  import { app, ui } from '../../lib/store.svelte'
  import { tipFor } from '../../help/actions'
  import { css } from '../../lib/leds'
  import { surfaceOf } from '../../lib/surface'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import Fader from '../../lib/ui/Fader.svelte'
  import Overlay from '../../lib/ui/Overlay.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'
  import HSlider from '../settings/HSlider.svelte'
  import Strip from './Strip.svelte'
  import { partVoice, pluginBadge, pluginTip, styleVoice } from './voice'

  const mixer = $derived(app.state.mixer)
  const page = $derived(mixer.faderPage)
  const parts = $derived(app.state.keyboardParts)
  const surface = $derived(surfaceOf(app.state, app.library))
  const outPort = $derived(app.state.io.outputPort)
  const metronome = $derived(app.state.metronome)
  const effects = $derived(app.state.effects.blocks)
  const FX_TIPS: Record<FxBlock, [TipKey, TipKey, TipKey]> = {
    reverb: ['fx.reverb_type', 'fx.reverb_return', 'fx.reverb_band'],
    chorus: ['fx.chorus_type', 'fx.chorus_return', 'fx.chorus_band'],
    variation: ['fx.variation_type', 'fx.variation_return', 'fx.variation_band'],
  }
  /** A return level as the Genos shows it: 64 = 0 dB, 127 = +6 dB, 0 = off. */
  const returnText = (v: number) => (v === 0 ? 'Off' : `${v >= 64 ? '+' : ''}${(20 * Math.log10(v / 64)).toFixed(1)} dB`)

  // Style Track Mute is a knob: the engine keeps only the parts' switches it sets, so the
  // knob's position and order are this drawer's. Choosing an order only chooses what the
  // knob does next: it sends nothing, so parts switched off by hand stay off.
  let muteOrder = $state<TrackMuteOrder>('a')
  let muteValue = $state(127)
  function trackMute(v: number) {
    muteValue = v
    app.send({ type: 'styleTrackMute', order: muteOrder, value: v })
  }
  function setMuteOrder(o: TrackMuteOrder) {
    muteOrder = o
  }

  /** Where Launchkey fader `i` (0–7, 8 = master) physically is, when the engine says. */
  const hwAt = (i: number): number | null => surface.faders[i]?.position ?? null
  /** The light of the button under Launchkey fader `i` (0-based), as the engine reports it. */
  const ledAt = (i: number) => surface.controls.find((c) => c.id === `faderButton${i + 1}`) ?? null
  const pageLed = $derived(surface.controls.find((c) => c.id === 'masterButton') ?? null)

  /** Panel fader 5 (0-based 4): the Style volume. */
  const STYLE_SLOT = 4
  /** Panel fader 6: the Multi Pad volume. */
  const PAD_SLOT = 5
  /** Panel page faders after the Multi Pad volume (7–8): unused on the Launchkey too. */
  const unusedSlots = $derived(Array.from({ length: Math.max(0, 8 - PAD_SLOT - 1) }, (_, k) => PAD_SLOT + 1 + k))

  /** The Panel page's button under fader 5 is HARMONY/ARPEGGIO, on the Launchkey and here. */
  const HARM_ARP_SLOT = 4
  const harmonyOn = $derived(app.state.harmonyArp.on)
  const slotButton = (n: number) =>
    n === HARM_ARP_SLOT
      ? {
          led: ledAt(n),
          text: 'Harm/Arp',
          label: `Harmony/Arpeggio ${harmonyOn ? 'on' : 'off'}`,
          tip: tipFor({ type: 'toggleHarmonyArp' }),
          onclick: () => app.send({ type: 'toggleHarmonyArp' }),
        }
      : null

  const TABS: { id: FaderPage; name: string }[] = [
    { id: 'panel', name: 'Panel' },
    { id: 'style', name: 'Style' },
  ]

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

  // Launchkey fader order on the Panel page: Right 1, Right 2, Right 3, Left.
  const panelStrip = (p: KeyboardPart, i: number) => ({
    name: p.name,
    channel: p.channel,
    value: p.volume,
    waiting: p.waiting,
    hw: hwAt(i),
    faderTip: tipFor({ type: 'setPartVolume', part: i, volume: 0 }),
    onchange: (v: number) => app.send({ type: 'setPartVolume', part: i, volume: v }),
    fx: {
      pan: p.pan,
      reverb: p.reverb,
      chorus: p.chorus,
      variation: p.variation,
      reverbDefault: i === 3 ? 40 : 50,
      onpan: (v: number) => app.send({ type: 'setPartPan', part: i, pan: v }),
      onsend: (send: PartSend, v: number) => app.send({ type: 'setPartSend', part: i, send, value: v }),
    },
    lit: p.sounding,
    on: {
      led: ledAt(i),
      isOn: p.on,
      tip: tipFor({ type: 'togglePart', part: i }),
      onclick: () => app.send({ type: 'togglePart', part: i }),
    },
    voice: partVoice(p),
    badge: p.playsBass
      ? { text: 'Plays bass', tip: 'detection.manual_bass' as const }
      : p.plugin
        ? { text: pluginBadge(p.plugin), tip: pluginTip(p.plugin) }
        : null,
    solo: {
      isSolo: mixer.partSolo === i,
      onclick: () => app.send({ type: 'setPartSolo', part: mixer.partSolo === i ? null : i }),
    },
  })

  const styleStrip = (p: StylePart, i: number) => ({
    name: p.name,
    channel: p.channel,
    value: p.volume,
    waiting: p.waiting,
    hw: hwAt(i),
    faderTip: 'mixer.style.volume' as const,
    onchange: (v: number) => app.send({ type: 'setStylePartVolume', part: i, volume: v }),
    lit: mixer.styleSolo === null ? p.on : mixer.styleSolo === i,
    on: {
      led: ledAt(i),
      isOn: p.on,
      tip: 'mixer.style.mute' as const,
      onclick: () => app.send({ type: 'toggleStylePart', part: i }),
    },
    voice: styleVoice(p.voice),
    badge: p.mutedByManualBass ? { text: 'Manual Bass', tip: 'detection.manual_bass' as const } : null,
    solo: {
      isSolo: mixer.styleSolo === i,
      onclick: () => app.send({ type: 'setStyleSolo', part: mixer.styleSolo === i ? null : i }),
    },
  })
</script>

<Overlay id="mixer" title="Mixer" closeTip="drawer.close" onclose={() => (ui.mixer = false)}>
  <div class="mixer">
    <div class="top">
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
      <div class="follows engraved" use:tip={'mixer.page'}>
        <span
          class="lamp"
          aria-hidden="true"
          style:--led={pageLed ? css(pageLed.rgb) : 'transparent'}
        ></span>
        Launchkey faders: {page === 'panel' ? 'Panel' : 'Style'}
      </div>
      <div class="out" use:tip={'mixer.channel'}>
        <span class="engraved">MIDI out</span> <b>{outPort || '—'}</b>
      </div>
    </div>

    <div class="extras">
      <div class="metronome">
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
      {#if page === 'style'}
        <div class="trackmute">
          <span class="engraved">Track Mute</span>
          {#each ['a', 'b'] as const as o (o)}
            <button
              type="button"
              class="order mat-raised"
              class:pressed={muteOrder === o}
              aria-pressed={muteOrder === o}
              use:tip={'mixer.track_mute_order'}
              onclick={() => setMuteOrder(o)}>{o.toUpperCase()}</button
            >
          {/each}
          <div class="slider">
            <HSlider value={muteValue} tip="mixer.track_mute" label="Style Track Mute" onchange={trackMute} />
          </div>
        </div>
      {/if}
    </div>

    <div class="effects">
      {#each effects as b (b.block)}
        <div class="block">
          <span class="engraved">{b.block === 'variation' ? 'Delay' : b.name}</span>
          <select
            class="field"
            aria-label="{b.name} type"
            use:tip={FX_TIPS[b.block][0]}
            value={b.effect}
            onchange={(e) => app.send({ type: 'setEffectType', block: b.block, effect: e.currentTarget.value as FxType })}
          >
            {#each b.types as t (t.effect)}
              <option value={t.effect}>{t.name}</option>
            {/each}
          </select>
          <div class="slider return">
            <HSlider
              value={b.returnLevel}
              tip={FX_TIPS[b.block][1]}
              label="{b.name} return"
              unity={64}
              format={returnText}
              onchange={(v) => app.send({ type: 'setEffectReturn', block: b.block, level: v })}
            />
          </div>
          <span class="band-label">Band</span>
          <div class="slider return">
            <HSlider
              value={b.bandSend}
              tip={FX_TIPS[b.block][2]}
              label="{b.name} band send"
              unity={100}
              format={(v) => `${v}%`}
              onchange={(v) => app.send({ type: 'setBandSend', block: b.block, level: v })}
            />
          </div>
        </div>
      {/each}
    </div>

    <p class="info" use:tip={'mixer.info'}>
      <b>A fader is its channel’s CC 7</b>, 0–127, with no hidden gain. Loading a style sets the Style faders to the
      style’s own levels. <span class="wait">↕</span> waits for the Launchkey fader; the dashed cap is where it sits.
    </p>

    <div class="strips" id="mixer-strips" role="tabpanel" aria-labelledby="mixer-tab-{page}">
      {#if page === 'panel'}
        {#each parts as p, i (i)}
          <Strip {...panelStrip(p, i)} />
        {/each}
        <Strip
          name="Style"
          value={mixer.styleVolume}
          waiting={mixer.styleVolumeWaiting}
          hw={hwAt(STYLE_SLOT)}
          faderTip="mixer.style_level"
          onchange={(v) => app.send({ type: 'setStyleVolume', volume: v })}
          fxRow
          button={slotButton(STYLE_SLOT)}
        />
        <Strip
          name="M.Pad"
          value={mixer.multiPadVolume}
          waiting={mixer.multiPadVolumeWaiting}
          hw={hwAt(PAD_SLOT)}
          faderTip="mixer.pad_level"
          onchange={(v) => app.send({ type: 'setMultiPadVolume', volume: v })}
          fxRow
          button={slotButton(PAD_SLOT)}
        />
        {#each unusedSlots as n (n)}
          <Strip name="—" value={0} faderTip="launchkey.fader_unused" onchange={() => {}} unused fxRow button={slotButton(n)} />
        {/each}
      {:else}
        {#each mixer.styleParts as p, i (i)}
          <Strip {...styleStrip(p, i)} />
        {/each}
      {/if}

      <div class="master" class:knobs={page === 'panel'}>
        <div class="ch engraved">Synth</div>
        {#if page === 'panel'}<div class="fx-space" aria-hidden="true"></div>{/if}
        <div class="fader">
          <Fader
            value={mixer.master ?? 0}
            tip="mixer.master"
            label="Master"
            pickup={mixer.masterWaiting}
            hw={hwAt(8)}
            disabled={mixer.master === null}
            onchange={(v) => app.send({ type: 'setMasterVolume', volume: v })}
          />
        </div>
        <div class="headroom" use:tip={'mixer.master'}>
          {#if mixer.master === null}
            <span>Synth off</span><span>&nbsp;</span>
          {:else}
            <span>100 = unity</span><span>soft clip above −1 dBFS</span>
          {/if}
        </div>
      </div>
    </div>
  </div>
</Overlay>

<style>
  /* Wider than the other drawers: a Genos-style bank of eight strips and the master. */
  :global(.overlay.right[data-overlay='mixer']) {
    width: min(48rem, calc(100vw - 32px));
  }
  .mixer {
    display: grid;
    grid-template-rows: auto auto auto auto 1fr;
    gap: 0.7rem;
    height: 100%;
    min-height: 27rem;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 0.9rem;
    flex-wrap: wrap;
  }
  .tabs {
    display: flex;
    gap: 0.3rem;
  }
  .tab {
    display: inline-flex;
    align-items: center;
    gap: 0.5em;
    min-height: 2.2rem;
    min-width: 5.5rem;
    padding: 0 0.9em;
    border-radius: 5px;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 1rem;
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
    gap: 0.45em;
    white-space: nowrap;
  }
  .lamp {
    width: 0.9em;
    height: 0.45em;
    border-radius: 2px;
    background: var(--led);
    box-shadow: 0 0 6px var(--led);
  }
  .out {
    margin-left: auto;
    font-family: var(--font-display);
    font-size: 0.9rem;
    white-space: nowrap;
  }
  .extras {
    display: flex;
    flex-wrap: wrap;
    gap: 0.6rem 1.4rem;
    align-items: center;
    font-size: 0.9rem;
  }
  .effects {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 1.2rem;
    align-items: center;
    font-size: 0.9rem;
  }
  .block {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .field {
    min-height: 2rem;
    padding: 0 0.4rem;
    border: 1px solid var(--seam);
    border-radius: 4px;
    background: var(--well);
    color: var(--ink);
    font: inherit;
  }
  .slider.return {
    width: 7.5rem;
  }
  .band-label {
    font-size: var(--fs-small);
    color: var(--muted);
  }
  .metronome,
  .trackmute {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .slider {
    width: 9rem;
  }
  .note {
    font-size: var(--fs-small);
    color: var(--muted);
  }
  .order {
    min-width: 2rem;
    min-height: 2rem;
    border-radius: 5px;
    font-family: var(--font-display);
    font-weight: 600;
    color: var(--ink);
  }
  .order.pressed {
    outline: 1px solid var(--accent);
  }
  .info {
    margin: 0;
    font-size: var(--fs-small);
    color: var(--muted);
    line-height: 1.35;
  }
  .info b {
    color: var(--ink);
    font-weight: 600;
  }
  .wait {
    font-weight: 700;
    color: var(--accent-ink);
    background: var(--accent);
    border-radius: 3px;
    padding: 0 0.2em;
  }
  .strips {
    display: grid;
    grid-template-columns: repeat(8, minmax(0, 1fr)) minmax(0, 1.2fr);
    gap: 0.3rem;
    min-height: 0;
    padding: 0.5rem;
    border-radius: var(--r-key);
    background: rgb(0 0 0 / 0.12);
    box-shadow: inset 0 1px 3px rgb(0 0 0 / 0.35);
  }
  /* A Fader's cap rides a full-height carrier moved by transform, so at a low value the
     (invisible) carrier hangs up to a whole track below it and makes the drawer scroll
     into empty space. The cap and ghost always sit inside the track: clip there. */
  .strips :global(.track) {
    overflow: clip;
  }
  .master {
    display: grid;
    grid-template-rows: auto minmax(13rem, 1fr) auto;
    justify-items: center;
    gap: 0.45rem;
    padding: 0.5rem 0.25rem 0.4rem 0.5rem;
    margin-left: 0.2rem;
    border-left: 1px solid var(--seam);
  }
  /* The row the Panel strips' knobs take, so the master fader lines up with theirs. */
  .master.knobs {
    grid-template-rows: auto var(--fx-h, 3.4rem) minmax(13rem, 1fr) auto;
  }
  .master .ch {
    font-size: 0.8rem;
  }
  .master .fader {
    height: 100%;
    width: 100%;
    display: flex;
    justify-content: center;
    font-size: 0.95rem;
  }
  .headroom {
    display: grid;
    text-align: center;
    font-size: 0.7rem;
    line-height: 1.2;
    color: var(--muted);
    align-content: start;
    /* The height of a strip's buttons, voice and badge, so the master fader lines up. */
    min-height: 6.4rem;
    padding-top: 0.2rem;
  }
</style>
