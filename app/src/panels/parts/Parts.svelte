<!--
  Keyboard parts + One Touch Settings (right-side drawer, open while `ui.parts`).

  1. Your hands: a keyboard map (who plays left and right of the split, and where the
     chords are read), the split point, Manual Bass.
  2. The four parts as strips: Right 1–3 under a layer bar (the Right parts that sound
     together), then Left. Each strip is Launchkey fader 1–4; hovering one lights it on
     the mirror.
  3. OTS 1–4: what each sets, the last recalled, OTS Link and its timing. A recall
     flashes the strips, and faders it moved show the soft-takeover mark.
  4. As written for: the voice each style part was written for, per channel.

  State: keyboardParts, chord (split, upper, manualBass), ots, mixer.styleParts,
  surface.faders (hardware positions). Commands: togglePart, selectPart, setPartVoice,
  setPartVolume, setPartOctave, moveSplit, toggleManualBass, recallOts, toggleOtsLink.
-->
<script lang="ts">
  import { untrack } from 'svelte'
  import { noteName } from '../../lib/api/mock'
  import { voiceList } from '../../lib/api/voices'
  import { app, ui } from '../../lib/store.svelte'
  import { surfaceOf } from '../../lib/surface'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import Overlay from '../../lib/ui/Overlay.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'
  import PartStrip from './PartStrip.svelte'
  import { PART_SHORT, chordsWhere, layerOf, layerText, leftZone, linkedMain, otsLine, otsTip } from './parts'

  const s = $derived(app.state)
  const parts = $derived(s.keyboardParts)
  const chord = $derived(s.chord)
  const ots = $derived(s.ots)
  const voices = $derived(voiceList(s))
  const layer = $derived(layerOf(parts))
  const zone = $derived(leftZone(s))
  const chords = $derived(chordsWhere(s))
  const surface = $derived(surfaceOf(s, app.library))
  const panelPage = $derived(s.mixer.faderPage === 'panel')
  const waiting = $derived(parts.flatMap((p, i) => (p.waiting ? [i] : [])))

  // The keyboard map spans the Launchkey 61's keys (C1–C6 in Yamaha numbering).
  const LOW = 36
  const HIGH = 96
  const splitAt = $derived(Math.max(0, Math.min(1, (chord.split + 1 - LOW) / (HIGH - LOW + 1))) * 100)
  const aboveSplit = $derived(noteName(Math.min(127, chord.split + 1)))

  // Flash the strips once on each OTS recall (UI only: the engine owns the values).
  let recalled = $state(0)
  let lastApplied = -1
  let lastStyle = -1
  $effect(() => {
    const a = ots.applied
    const style = s.style.id
    if (lastApplied >= 0 && style === lastStyle && a !== lastApplied && a > 0) untrack(() => recalled++)
    lastApplied = a
    lastStyle = style
  })
  function recall(i: number) {
    app.send({ type: 'recallOts', index: i })
    recalled++
  }
</script>

<Overlay id="parts" title="Keyboard parts and OTS" closeTip="drawer.close" onclose={() => (ui.parts = false)}>
  <div class="drawer">
    <!-- 1. The keyboard map ───────────────────────────────────────────── -->
    <section aria-labelledby="parts-hands">
      <h3 id="parts-hands" class="engraved">Your hands</h3>
      <div class="map" style:--split="{splitAt}%">
        <div class="keys mat-well" aria-hidden="true"></div>
        <div class="zone left who-{zone.who}" use:tip={'part.left_zone'}>
          <span class="zname">{zone.who === 'bass' ? 'Left · Bass' : zone.who === 'left' ? 'Left' : zone.who === 'chords' ? 'Chords' : 'Right'}</span>
          <span class="zvoice">{zone.text}</span>
          {#if chords === 'left'}<span class="ztag">chords read here</span>{/if}
        </div>
        <div class="zone right" use:tip={'part.layer'}>
          <span class="zname">{layer.length ? layer.map((i) => PART_SHORT[i]).join(' + ') : 'Right'}</span>
          <span class="zvoice">{layer.length ? layer.map((i) => parts[i].voiceName).join(' + ') : '—'}</span>
          {#if chords === 'right'}<span class="ztag">chords read here (Upper)</span>{/if}
        </div>
        <div class="split" aria-hidden="true"></div>
      </div>
      <div class="controls">
        <div class="splitctl">
          <HwButton tip="split.down" label="Split point down" onclick={() => app.send({ type: 'moveSplit', delta: -1 })}>−</HwButton>
          <!-- svelte-ignore a11y_no_noninteractive_tabindex (focusable so its tooltip is reachable from the keyboard) -->
          <span class="splitval" tabindex="0" use:tip={'split.display'}>
            <small class="engraved">Split</small>{chord.splitName}
          </span>
          <HwButton tip="split.up" label="Split point up" onclick={() => app.send({ type: 'moveSplit', delta: 1 })}>+</HwButton>
        </div>
        <div class="mb">
          <!-- Lit only when in effect: the engine ignores it in Lower, where the Launchkey pad is dark too. -->
          <Toggle on={chord.manualBassActive} tip="detection.manual_bass" onclick={() => app.send({ type: 'toggleManualBass' })}>Manual Bass</Toggle>
          <span class="note engraved">
            {#if chord.manualBassActive}Left plays the style's Bass
            {:else if chord.upper}off
            {:else}Upper only{/if}
          </span>
        </div>
      </div>
    </section>

    <!-- 2. The parts ──────────────────────────────────────────────────── -->
    <section aria-labelledby="parts-parts">
      <h3 id="parts-parts" class="engraved">Keyboard parts</h3>
      <div class="hands">
        <div class="hand right" use:tip={'part.layer'}>
          <span class="engraved">Right hand · {aboveSplit} and up</span>
          <div class="bus" aria-hidden="true">
            {#each [0, 1, 2] as i (i)}
              <span class="node" class:on={parts[i].sounding}></span>
            {/each}
            {#if layer.length > 1}
              <span class="wire" style:left="{(layer[0] * 100) / 3 + 100 / 6}%" style:right="{100 - ((layer[layer.length - 1] * 100) / 3 + 100 / 6)}%"></span>
            {/if}
          </div>
          <span class="layertext" class:layered={layer.length > 1}>{layerText(parts)}</span>
        </div>
        <div class="hand leftc" use:tip={'part.left_zone'}>
          <span class="engraved">Left · {chord.splitName} ↓</span>
          <div class="bus" aria-hidden="true"><span class="node" class:on={parts[3].sounding} class:bass={parts[3].playsBass}></span></div>
          <span class="layertext">{parts[3].playsBass ? 'plays Bass' : parts[3].sounding ? 'on' : 'off'}</span>
        </div>
      </div>
      <div class="strips">
        {#each parts as p, i (i)}
          <PartStrip
            part={p}
            index={i}
            {voices}
            hw={panelPage ? (surface.faders[i]?.position ?? null) : null}
            {recalled}
          />
        {/each}
      </div>
      <p class="pickup" class:on={waiting.length > 0} use:tip={'mixer.pickup'}>
        <span class="mark">↕</span>
        {#if waiting.length}
          Launchkey {waiting.length > 1 ? 'faders' : 'fader'} {waiting.map((i) => i + 1).join(', ')} waiting: move {waiting.length > 1 ? 'them' : 'it'} to the dashed mark to pick up{panelPage ? '' : ' (on the Panel fader page)'}.
        {:else}
          Launchkey faders 1–4 are in step with these levels.
        {/if}
      </p>
    </section>

    <!-- 3. One Touch Settings ─────────────────────────────────────────── -->
    <section aria-labelledby="parts-ots">
      <h3 id="parts-ots" class="engraved">One Touch Settings</h3>
      <p class="explain">
        Four sound setups for <em>your hands</em> that come with the style. Pressing one sets Right 1–3 and
        Left: voice, on/off, volume and octave. The band doesn't change.
      </p>
      <div class="linkrow">
        <Toggle on={ots.link} tip="ots.link" onclick={() => app.send({ type: 'toggleOtsLink' })}>OTS Link</Toggle>
        <!-- svelte-ignore a11y_no_noninteractive_tabindex (focusable so its tooltip is reachable from the keyboard) -->
        <span class="timing" tabindex="0" use:tip={'ots.link_timing'}>
          <small class="engraved">Timing</small>
          Real Time · on press
        </span>
        <span class="linknote">{ots.link ? 'Main A–D recall OTS 1–4' : 'Mains leave your sounds alone'}</span>
      </div>
      <div class="cards">
        {#each [0, 1, 2, 3] as i (i)}
          {@const o = ots.settings[i]}
          {@const applied = ots.applied === i + 1}
          <button
            type="button"
            class="card mat-raised"
            class:applied
            class:missing={!o}
            class:linked={ots.link && s.transport.main === i}
            aria-pressed={applied}
            aria-disabled={!o}
            use:tip={otsTip(i)}
            onclick={() => o && recall(i)}
          >
            <span class="chead">
              <span class="lamp" aria-hidden="true"></span>
              <span class="cname">{o?.name ?? `OTS ${i + 1}`}</span>
              {#if applied}<span class="tag">last recalled</span>{/if}
              {#if ots.link}<span class="main">{linkedMain(i)}</span>{/if}
            </span>
            {#if o}
              <span class="clines">
                {#each o.parts as op, j (j)}
                  <span class="cline" class:off={!op.on}>
                    <b>{PART_SHORT[j]}</b><span class="ctext">{op.on ? otsLine(op) : `off · ${op.voiceName}`}</span>
                  </span>
                {/each}
              </span>
            {:else}
              <span class="none">Not in this style</span>
            {/if}
          </button>
        {/each}
      </div>
    </section>

    <!-- 4. As written for ───────────────────────────────────────────────── -->
    <section aria-labelledby="parts-written">
      <h3 id="parts-written" class="engraved">As written for</h3>
      <p class="explain">The band's voices, per channel on the <code>yahaha</code> port: what to load in Ableton.</p>
      <ul class="written">
        {#each s.mixer.styleParts as sp (sp.channel)}
          <li use:tip={'part.written_for'} class:muted={sp.mutedByManualBass}>
            <span class="wch">ch {sp.channel}</span>
            <span class="wname">{sp.name}</span>
            <span class="wvoice">{sp.voice?.label ?? '—'}</span>
            {#if sp.mutedByManualBass}<span class="wtag">your left hand</span>{/if}
          </li>
        {/each}
      </ul>
    </section>
  </div>
</Overlay>

<style>
  .drawer {
    display: flex;
    flex-direction: column;
    gap: 1.15rem;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
  }
  h3 {
    margin: 0;
    font-size: 0.78rem;
  }
  .explain {
    margin: 0;
    font-size: var(--fs-small);
    color: var(--muted);
    line-height: 1.35;
  }
  .explain em {
    color: var(--ink);
    font-style: normal;
    font-weight: 600;
  }

  /* ── Keyboard map ── */
  .map {
    position: relative;
    display: grid;
    grid-template-columns: var(--split) 1fr;
    height: 4.2rem;
    border-radius: 6px;
    overflow: hidden;
  }
  .keys {
    position: absolute;
    inset: 0;
    border-radius: 6px;
    /* A key every 1/61 of the width: thin seams, no bitmaps. */
    background:
      repeating-linear-gradient(90deg, transparent 0 calc(100% / 61 - 1px), rgb(255 255 255 / 0.06) calc(100% / 61 - 1px) calc(100% / 61)),
      var(--well);
  }
  .zone {
    position: relative;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 0.05rem;
    min-width: 0;
    padding: 0 0.55rem;
    color: var(--screen-ink);
  }
  .zone.left {
    background: color-mix(in srgb, var(--screen-glow) 45%, transparent);
  }
  .zone.left.who-chords,
  .zone.left.who-right {
    background: transparent;
  }
  .zone.left.who-bass {
    background: color-mix(in srgb, var(--accent) 30%, transparent);
  }
  .zone.right {
    background: color-mix(in srgb, var(--screen-glow) 25%, transparent);
  }
  .zname {
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 0.82rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    white-space: nowrap;
  }
  .zvoice,
  .ztag {
    font-size: 0.74rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .ztag {
    color: var(--screen-dim);
  }
  .split {
    position: absolute;
    top: 0;
    bottom: 0;
    left: var(--split);
    width: 2px;
    margin-left: -1px;
    background: var(--accent);
    box-shadow: 0 0 6px var(--accent);
  }
  .controls {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.8rem;
    flex-wrap: wrap;
  }
  .splitctl {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }
  .splitctl :global(.hw) {
    width: 2.6rem;
  }
  .splitctl :global(.btn) {
    height: 2.2rem;
    font-size: 1.05rem;
  }
  .splitval,
  .timing {
    display: inline-flex;
    flex-direction: column;
    align-items: center;
    min-width: 3.2rem;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 1.05rem;
    line-height: 1.05;
    border-radius: 4px;
  }
  .splitval small,
  .timing small {
    font-size: 0.6rem;
  }
  .mb {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .note {
    font-size: 0.68rem;
  }

  /* ── Hands and the layer bus ── */
  .hands {
    display: grid;
    grid-template-columns: 3fr 1fr;
    gap: 0.4rem;
  }
  .hand {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.2rem;
    min-width: 0;
    padding: 0.3rem 0.3rem 0.35rem;
    border-radius: 6px;
    border: 1px solid var(--seam);
  }
  .hand .engraved {
    font-size: 0.66rem;
    white-space: nowrap;
  }
  .bus {
    position: relative;
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: 1fr;
    width: 100%;
    height: 0.9rem;
    align-items: center;
    justify-items: center;
  }
  .node {
    position: relative;
    z-index: 1;
    width: 0.7rem;
    height: 0.7rem;
    border-radius: 50%;
    background: var(--lamp-off);
    box-shadow: inset 0 1px 1px rgb(0 0 0 / 0.6);
  }
  .node.on {
    background: var(--accent);
    box-shadow: 0 0 7px var(--accent);
  }
  .wire {
    position: absolute;
    top: 50%;
    height: 3px;
    margin-top: -1.5px;
    border-radius: 2px;
    background: var(--accent);
  }
  .layertext {
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.85rem;
    color: var(--muted);
    white-space: nowrap;
  }
  .layertext.layered {
    color: var(--accent);
  }
  .strips {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr)) minmax(0, 1fr);
    gap: 0.4rem;
  }
  .strips > :global(:nth-child(4)) {
    margin-left: 0.2rem;
  }
  .pickup {
    display: flex;
    align-items: baseline;
    gap: 0.4rem;
    margin: 0;
    min-height: 2.4em;
    font-size: var(--fs-small);
    color: var(--muted);
    line-height: 1.3;
  }
  .mark {
    flex: none;
    font-weight: 700;
    font-size: 0.75rem;
    color: var(--accent-ink);
    background: var(--lamp-off);
    border-radius: 3px;
    padding: 0 0.25em;
  }
  .pickup.on {
    color: var(--ink);
  }
  .pickup.on .mark {
    background: var(--accent);
  }

  /* ── OTS ── */
  .linkrow {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    flex-wrap: wrap;
  }
  .timing {
    font-size: 0.9rem;
  }
  .linknote {
    font-size: var(--fs-small);
    color: var(--muted);
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.45rem;
  }
  .card {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0.35rem;
    min-height: 7.2rem;
    padding: 0.5rem 0.55rem;
    border-radius: 7px;
    text-align: left;
    color: var(--ink);
    font-family: var(--font-body);
  }
  .card.applied {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
  }
  .card.linked:not(.applied) {
    outline: 1px dashed var(--accent);
    outline-offset: -1px;
  }
  .card.missing {
    opacity: 0.5;
    cursor: default;
  }
  .chead {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    min-width: 0;
  }
  .lamp {
    flex: none;
    width: 0.6rem;
    height: 0.6rem;
    border-radius: 50%;
    background: var(--lamp-off);
    box-shadow: inset 0 1px 1px rgb(0 0 0 / 0.6);
  }
  .applied .lamp {
    background: var(--accent);
    box-shadow: 0 0 8px var(--accent);
  }
  .cname {
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 1.02rem;
    letter-spacing: 0.03em;
  }
  .tag,
  .main {
    font-family: var(--font-display);
    font-size: 0.68rem;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    white-space: nowrap;
  }
  .tag {
    color: var(--accent);
  }
  .main {
    margin-left: auto;
    color: var(--muted);
  }
  .clines {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    font-size: 0.78rem;
  }
  .cline {
    display: grid;
    grid-template-columns: 1.6rem minmax(0, 1fr);
    min-width: 0;
  }
  .cline b {
    font-family: var(--font-display);
    font-weight: 700;
    color: var(--engrave);
  }
  .ctext {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .cline.off {
    color: var(--muted);
    opacity: 0.7;
  }
  .none {
    font-size: var(--fs-small);
    color: var(--muted);
  }

  /* ── As written for ── */
  code {
    font-size: 0.95em;
  }
  .written {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    border-radius: 6px;
    border: 1px solid var(--seam);
    overflow: hidden;
  }
  .written li {
    display: grid;
    grid-template-columns: 2.6rem 4.4rem minmax(0, 1fr) auto;
    align-items: baseline;
    gap: 0.4rem;
    padding: 0.28rem 0.5rem;
    font-size: var(--fs-small);
  }
  .written li + li {
    border-top: 1px solid color-mix(in srgb, var(--seam) 60%, transparent);
  }
  .wch {
    color: var(--muted);
  }
  .wname {
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.92rem;
  }
  .wvoice {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .muted .wvoice {
    color: var(--muted);
  }
  .wtag {
    font-size: 0.68rem;
    color: var(--accent);
    white-space: nowrap;
  }
</style>
