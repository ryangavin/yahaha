<!--
  The Rack panel (docs/racks.md "Screens"; the owner's wireframe v5, `rackEditor`): what's
  under your hands, as one rack. It replaces Parts & OTS. On Stage it is the right-hand
  drawer (open while `ui.rack`, Alt+O); with `docked` it renders without the drawer frame,
  for Library's dock.

  ┌ RACK  Soul Ballad  Mine  ● modified          [Save rack] [Save as…] [Revert] ┐
  │ YOUR HANDS                                                                    │
  │ Split − F#2 +   Harmony/Arp [Duet ▾] (on)   Transpose − 0 +                   │
  │ Manual Bass  Left Hold                                                        │
  │ ▸ Controller map   (faders 1–4, knobs 1–8: what each does for this rack)      │
  │ KEYBOARD PARTS                                                                │
  │ four RackSlots: R1, R2, R3, L                                                 │
  │ ONE TOUCH SETTINGS · <style>                                                  │
  │ OTS Link · timing · OTS 1–4 cards                                             │
  │ AS WRITTEN FOR (the band's voices per channel)                                │
  └───────────────────────────────────────────────────────────────────────────────┘

  State: liveRack, keyboardParts, chord, harmonyArp, ots, mixer.styleParts, plugins,
  soundLibrary. Commands: all existing ones (moveSplit, stepTranspose, setHarmonyType,
  setArpPattern, setHarmonyArpOn, toggleManualBass, toggleLeftHold, recallOts,
  toggleOtsLink and the part commands in RackSlot). Save rack, Save as… and Revert wait
  for the rack commands (docs/racks.md, order of work item 5) and are shown disabled.
-->
<script lang="ts">
  import { untrack } from 'svelte'
  import { app, ui } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import Overlay from '../../lib/ui/Overlay.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'
  import { PART_SHORT, linkedMain, otsLine, otsTip } from '../parts/parts'
  import { instrumentName, playingId } from '../sounds/model'
  import { rackName, signed, soundBadge, targetLabel } from './rack'
  import RackSlot from './RackSlot.svelte'

  let { docked = false }: { docked?: boolean } = $props()

  const s = $derived(app.state)
  const rack = $derived(s.liveRack)
  const parts = $derived(s.keyboardParts)
  const chord = $derived(s.chord)
  const ots = $derived(s.ots)
  const h = $derived(s.harmonyArp)
  const sl = $derived(s.soundLibrary)
  const ctx = $derived({ patches: sl.patches, gmMap: sl.gmMap ?? [] })
  const types = $derived(app.library.harmonyTypes ?? [])
  const patterns = $derived(app.library.arpPatterns ?? [])
  const waiting = $derived(parts.flatMap((p, i) => (p.waiting ? [i] : [])))
  const panelPage = $derived(s.mixer.faderPage === 'panel')

  let showMap = $state(false)

  /** The Harmony/Arp select's value: `h<index>` or `a<index>`. */
  const harmonyValue = $derived(h.mode === 'arpeggio' ? `a${h.arpPattern}` : `h${h.harmonyType}`)
  function pickHarmony(v: string) {
    const i = Number(v.slice(1))
    app.send(v[0] === 'a' ? { type: 'setArpPattern', index: i } : { type: 'setHarmonyType', index: i })
  }

  // Flash the slots once on each OTS recall (UI only: the engine owns the values).
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

{#snippet body()}
  <div class="rack" class:docked>
    <!-- The head: which rack, whether it changed, and saving it. ──────────────────── -->
    <div class="rackhead">
      <span class="k">Rack</span>
      <span class="rn">{rackName(rack)}</span>
      {#if rack.id !== null}<span class="badge mine">Mine</span>{/if}
      {#if rack.modified}<span class="dot">● modified</span>{/if}
      <span class="grow"></span>
      <button type="button" class="mini mat-raised" aria-disabled="true" use:tip={'rack.save'}>Save rack</button>
      <button type="button" class="mini mat-raised" aria-disabled="true" use:tip={'rack.save_as'}>Save as…</button>
      {#if rack.modified}<button type="button" class="mini mat-raised" aria-disabled="true" use:tip={'rack.revert'}>Revert</button>{/if}
    </div>

    <!-- Your hands ─────────────────────────────────────────────────────────────── -->
    <section aria-labelledby="rack-hands">
      <h3 id="rack-hands" class="engraved">Your hands</h3>
      <div class="row">
        <span class="k">Split</span>
        <HwButton tip="split.down" label="Split point down" onclick={() => app.send({ type: 'moveSplit', delta: -1 })}>−</HwButton>
        <!-- svelte-ignore a11y_no_noninteractive_tabindex (focusable so its tooltip is reachable from the keyboard) -->
        <b class="val" tabindex="0" use:tip={'split.display'}>{chord.splitName}</b>
        <HwButton tip="split.up" label="Split point up" onclick={() => app.send({ type: 'moveSplit', delta: 1 })}>+</HwButton>
        <span class="gap"></span>
        <span class="k">Transpose</span>
        <HwButton tip="transpose.keyboard_down" label="Keyboard transpose down" onclick={() => app.send({ type: 'stepTranspose', keyboard: -1, master: 0 })}>−</HwButton>
        <!-- svelte-ignore a11y_no_noninteractive_tabindex (focusable so its tooltip is reachable from the keyboard) -->
        <b class="val" tabindex="0" use:tip={'transpose.display'}>{signed(chord.transposeKeyboard)}</b>
        <HwButton tip="transpose.keyboard_up" label="Keyboard transpose up" onclick={() => app.send({ type: 'stepTranspose', keyboard: 1, master: 0 })}>+</HwButton>
      </div>
      <div class="row">
        <label for="rack-harm" class="k">Harmony / Arp</label>
        <select id="rack-harm" value={harmonyValue} onchange={(e) => pickHarmony(e.currentTarget.value)} use:tip={'rack.harmony_type'}>
          <optgroup label="Harmony">
            {#each types as t, i (i)}<option value="h{i}">{t.name}</option>{/each}
          </optgroup>
          <optgroup label="Arpeggio">
            {#each patterns as t, i (i)}<option value="a{i}">{t.name}</option>{/each}
          </optgroup>
        </select>
        <Toggle on={h.on} tip="harmony.switch" onclick={() => app.send({ type: 'setHarmonyArpOn', on: !h.on })}>On</Toggle>
      </div>
      <div class="row">
        <!-- Lit only when in effect: the engine ignores it in Lower, where the Launchkey pad is dark too. -->
        <Toggle on={chord.manualBassActive} tip="detection.manual_bass" onclick={() => app.send({ type: 'toggleManualBass' })}>Manual Bass</Toggle>
        <Toggle on={chord.leftHold} tip="detection.left_hold" onclick={() => app.send({ type: 'toggleLeftHold' })}>Left Hold</Toggle>
        <span class="note">
          {#if chord.manualBassActive}Left plays the style's Bass
          {:else if chord.upper}Manual Bass off
          {:else}Manual Bass: Upper only{/if}
        </span>
      </div>
      <div class="row">
        <button type="button" class="disc" aria-expanded={showMap} use:tip={'rack.map'} onclick={() => (showMap = !showMap)}>{showMap ? '▾' : '▸'} Controller map</button>
        <span class="note">What the Launchkey faders and knobs do on the Rack knob page</span>
      </div>
      {#if showMap}
        <table class="map">
          <thead><tr><th>Launchkey</th><th>Controls</th></tr></thead>
          <tbody>
            {#each rack.controls.faders as t, i (i)}<tr><td>Fader {i + 1}</td><td>{targetLabel(t)}</td></tr>{/each}
            {#each rack.controls.knobs as t, i (i)}<tr><td>Knob {i + 1}</td><td>{targetLabel(t)}</td></tr>{/each}
          </tbody>
        </table>
      {/if}
    </section>

    <!-- Keyboard parts ─────────────────────────────────────────────────────────── -->
    <section aria-labelledby="rack-parts">
      <h3 id="rack-parts" class="engraved">Keyboard parts</h3>
      {#each parts as p, i (i)}
        <RackSlot
          part={p}
          index={i}
          instrument={instrumentName(p, ctx, s.plugins.list, s.io.soundFontFile)}
          badge={soundBadge(p, playingId(p, ctx))}
          {recalled}
        />
      {/each}
      <p class="pickup" class:on={waiting.length > 0} use:tip={'mixer.pickup'}>
        <span class="mark">↕</span>
        {#if waiting.length}
          Launchkey {waiting.length > 1 ? 'faders' : 'fader'} {waiting.map((i) => i + 1).join(', ')} waiting: move {waiting.length > 1 ? 'them' : 'it'} to the level to pick up{panelPage ? '' : ' (on the Panel fader page)'}.
        {:else}
          Launchkey faders 1–4 are in step with these levels.
        {/if}
      </p>
    </section>

    <!-- One Touch Settings ─────────────────────────────────────────────────────── -->
    <section aria-labelledby="rack-ots">
      <h3 id="rack-ots" class="engraved">One Touch Settings · {s.style.name}</h3>
      <div class="row">
        <Toggle on={ots.link} tip="ots.link" onclick={() => app.send({ type: 'toggleOtsLink' })}>OTS Link</Toggle>
        <!-- svelte-ignore a11y_no_noninteractive_tabindex (focusable so its tooltip is reachable from the keyboard) -->
        <span class="timing" tabindex="0" use:tip={'ots.link_timing'}>
          {ots.linkTiming === 'immediate' ? 'Immediate · on press' : 'At Main Section Change'}
        </span>
        <span class="note">{ots.link ? 'Main A–D recall OTS 1–4' : 'Mains leave your sounds alone'}</span>
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
              <span class="badge">Style</span>
              {#if applied}<span class="tag">last recalled</span>{/if}
              {#if ots.link}<span class="main">{linkedMain(i)}</span>{/if}
            </span>
            {#if o}
              <span class="clines">
                {#each o.parts as op, j (j)}
                  <span class="cline" class:off={!op.on}><b>{PART_SHORT[j]}</b><span class="ctext">{op.on ? otsLine(op) : `off · ${op.voiceName}`}</span></span>
                {/each}
              </span>
            {:else}
              <span class="none">Not in this style</span>
            {/if}
          </button>
        {/each}
      </div>
      <p class="note">Not in the rack: the style, tempo and Multi Pad bank.</p>
    </section>

    <!-- As written for ─────────────────────────────────────────────────────────── -->
    <section aria-labelledby="rack-written">
      <h3 id="rack-written" class="engraved">As written for</h3>
      <p class="note">The band's voices, per channel on the <code>yahaha</code> port: what to load in Ableton.</p>
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
{/snippet}

{#if docked}
  <aside class="dock mat-chassis" aria-label="Rack">{@render body()}</aside>
{:else}
  <Overlay id="rack" title="Rack" closeTip="drawer.close" onclose={() => (ui.rack = false)}>{@render body()}</Overlay>
{/if}

<style>
  .dock {
    height: 100%;
    overflow: auto;
    padding: 0.8rem;
    border-radius: var(--r-panel);
    font-size: 15px;
  }
  .rack {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  h3 {
    margin: 0;
    font-size: 0.78rem;
  }
  .rackhead {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.45rem;
  }
  .rn {
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 1.15rem;
    letter-spacing: 0.02em;
  }
  .grow {
    flex: 1;
  }
  .gap {
    width: 0.8rem;
  }
  .dot {
    font-size: 0.78rem;
    color: var(--accent);
    white-space: nowrap;
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
  .badge.mine {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 50%, transparent);
  }
  .k {
    font-family: var(--font-display);
    font-size: 0.7rem;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--engrave, var(--muted));
    white-space: nowrap;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4rem;
  }
  .row :global(.hw) {
    width: 2.2rem;
  }
  .row :global(.btn) {
    height: 2rem;
    font-size: 1rem;
  }
  .val {
    min-width: 2.4rem;
    text-align: center;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 1rem;
    border-radius: 4px;
  }
  select {
    flex: 1;
    min-width: 8rem;
    max-width: 14rem;
    min-height: 2rem;
    font: inherit;
    font-size: 0.85rem;
  }
  .note {
    margin: 0;
    font-size: var(--fs-small);
    color: var(--muted);
  }
  .mini {
    min-height: 1.9rem;
    padding: 0 0.65em;
    border-radius: 5px;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.8rem;
    color: var(--ink);
    white-space: nowrap;
  }
  .mini[aria-disabled='true'] {
    opacity: 0.45;
    cursor: default;
  }
  .disc {
    padding: 0.2rem 0.3rem;
    border: 0;
    background: none;
    color: var(--ink);
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.85rem;
    cursor: pointer;
  }
  .map {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--fs-small);
  }
  .map th,
  .map td {
    padding: 0.2rem 0.5rem;
    text-align: left;
    border-bottom: 1px solid color-mix(in srgb, var(--seam) 60%, transparent);
  }
  .map th {
    color: var(--muted);
    font-weight: 600;
  }
  .pickup {
    display: flex;
    align-items: baseline;
    gap: 0.4rem;
    margin: 0;
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
  .timing {
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.85rem;
    border-radius: 4px;
  }

  /* ── OTS ── */
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
    min-height: 6.4rem;
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
    font-size: 1rem;
    letter-spacing: 0.03em;
  }
  .tag,
  .main {
    font-family: var(--font-display);
    font-size: 0.66rem;
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
    font-size: 0.76rem;
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
