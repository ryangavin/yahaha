<!--
  The Channel view: one part's full channel strip (the mixer rework's StripState), filling
  the display. `part` is the strip 0–11: 0–3 the keyboard parts (Right 1, Right 2, Right 3,
  Left), 4–11 the Style parts (Rhythm 1 … Phrase 2). ‹ › step through all twelve.

  A row of section cards that scrolls sideways when narrow:
  - Level: the part's CC 7 (`setPartVolume` / `setStylePartVolume`), its meter (read from
    `app.meters()` every 100 ms while shown) and, on a keyboard part only, its pan
    (`setPartPan`). Style parts have no pan.
  - EQ: low and high shelf gain and frequency (`setStripEq`, the whole EQ).
  - Compressor: on/off, type, threshold/ratio/attack/release/make-up (`setStripCompressor*`);
    "edited" when the parameters differ from the type's.
  - Inserts: two slots (`setStripInsertKind`, `On`, `Setting`), a knob per setting of the
    slot's kind. On a Style part, slot 1 is the style's insert.
  - Sends: the strip's level to each send effect there is (`setStripSend`).
  - Play (keyboard parts): octave (`setPartOctave`) and Pitch Bend Range (`setBendRange`).
  Everything comes from the state; the view only sends commands.
-->
<script lang="ts">
  import type { TipKey } from '../../help/tooltips'
  import { COMP_PRESETS, type CompPreset, type InsertType, type PartCompParam, type PartCompState, type PartEq } from '../../lib/api/types'
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import Fader from '../../lib/ui/Fader.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'
  import FxKnob from '../mixer/FxKnob.svelte'
  import { dbText, GAIN_KNOB_MAX, gainKnob, HIGH_STEPS, hzText, knobGain, LOW_STEPS, stepOf, withEq } from '../mixer/eq'
  import { PART_COLORS } from '../mixer/parts'
  import { octaveLabel } from '../parts/parts'
  import { INSERT_KINDS, INSERT_SETTING_TIPS, meterFrac, panText, STRIP_COUNT } from './channel'

  let { part, onpart, onclose }: { part: number; onpart: (p: number) => void; onclose: () => void } = $props()

  const kb = $derived(part < 4 ? (app.state.keyboardParts[part] ?? null) : null)
  const sp = $derived(part >= 4 ? (app.state.mixer.styleParts[part - 4] ?? null) : null)
  const info = $derived(kb ?? sp)
  const strip = $derived(info?.strip ?? null)
  const sends = $derived(app.state.effects.sends)
  const bend = $derived(kb ? (app.state.controllers.parts[part]?.bendRange ?? null) : null)

  // The meter: the loudest since the last read, falling back slowly between reads.
  let level = $state(0)
  $effect(() => {
    const ch = info?.channel ?? null
    let live = true
    level = 0
    const read = () =>
      app.meters().then((m) => {
        if (!live) return
        const peak = m?.channels.find((c) => c.channel === ch)?.peak ?? 0
        level = Math.max(meterFrac(peak), level * 0.8)
      })
    read()
    const t = setInterval(read, 100)
    return () => {
      live = false
      clearInterval(t)
    }
  })

  const setVolume = (v: number) =>
    kb ? app.send({ type: 'setPartVolume', part, volume: v }) : app.send({ type: 'setStylePartVolume', part: part - 4, volume: v })
  const setEq = (eq: PartEq, change: Partial<PartEq>) => app.send({ type: 'setStripEq', strip: part, eq: withEq(eq, change) })

  const COMP_KNOBS: { param: PartCompParam; caption: string; min: number; max: number; tip: TipKey; format: (v: number) => string }[] = [
    { param: 'threshold', caption: 'Threshold', min: -48, max: 0, tip: 'mixer.strip.comp_threshold', format: (v) => `${v < 0 ? '−' + -v : v} dB` },
    { param: 'ratio', caption: 'Ratio', min: 10, max: 200, tip: 'mixer.strip.comp_ratio', format: (v) => `${(v / 10).toFixed(1)}:1` },
    { param: 'attack', caption: 'Attack', min: 1, max: 100, tip: 'mixer.strip.comp_attack', format: (v) => `${v} ms` },
    { param: 'release', caption: 'Release', min: 10, max: 1000, tip: 'mixer.strip.comp_release', format: (v) => `${v} ms` },
    { param: 'makeup', caption: 'Make-up', min: 0, max: 24, tip: 'mixer.strip.comp_makeup', format: (v) => `${v} dB` },
  ]

  const slotTitle = (i: number) => (sp && i === 0 ? 'Style insert' : `Insert ${i + 1}`)
  /** The kinds the picker offers, with the slot's own if this build doesn't know it. */
  const kindsFor = (kind: InsertType, name: string) => (INSERT_KINDS.some((k) => k.kind === kind) ? INSERT_KINDS : [...INSERT_KINDS, { kind, name }])
</script>

{#if info && strip}
  {@const name = info.name}
  <section class="channel" aria-label="{name} channel" class:style={sp !== null}>
    <header class="head">
      <button type="button" class="nav mat-raised" aria-label="Previous part" use:tip={'mixer.channel.prev'} onclick={() => onpart((part + STRIP_COUNT - 1) % STRIP_COUNT)}>‹</button>
      <div class="title">
        <span class="swatch" aria-hidden="true" style:--part={PART_COLORS[part]}></span>
        <h2>{name}</h2>
        <span class="sub">{kb ? 'Keyboard' : 'Style'} · Ch {info.channel}</span>
      </div>
      <button type="button" class="nav mat-raised" aria-label="Next part" use:tip={'mixer.channel.next'} onclick={() => onpart((part + 1) % STRIP_COUNT)}>›</button>
      <button type="button" class="nav close mat-raised" aria-label="Close channel view" use:tip={'mixer.channel.close'} onclick={onclose}>×</button>
    </header>

    <div class="sections">
      <section class="card level" aria-label="Level">
        <h3>Level</h3>
        <div class="level-row">
          <div class="fader">
            <Fader value={info.volume} tip="mixer.channel.level" label="{name} level" pickup={info.waiting} onchange={setVolume} />
          </div>
          <div
            class="meter mat-well"
            role="meter"
            aria-label="{name} meter"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={Math.round(level * 100)}
          >
            <div class="meter-fill" style:transform="scaleY({level})"></div>
          </div>
        </div>
        {#if kb}
          <div class="knobs">
            <FxKnob value={kb.pan} tip="mixer.channel.pan" label="{name} pan" caption="Pan" reset={64} centre format={panText} onchange={(v) => app.send({ type: 'setPartPan', part, pan: v })} />
          </div>
        {/if}
      </section>

      <section class="card" aria-label="EQ">
        <h3>EQ</h3>
        {#snippet eqKnobs(e: PartEq)}
          <div class="knobs grid2">
            <FxKnob value={gainKnob(e.lowGain)} max={GAIN_KNOB_MAX} centre tip="mixer.strip.eq_low_gain" label="{name} EQ low" caption="Low" reset={gainKnob(0)} format={(v) => `${dbText(knobGain(v))} dB`} onchange={(v) => setEq(e, { lowGain: knobGain(v) })} />
            <FxKnob value={stepOf(LOW_STEPS, e.lowFreq)} max={LOW_STEPS.length - 1} tip="mixer.strip.eq_low_freq" label="{name} EQ low frequency" caption="Low Hz" reset={stepOf(LOW_STEPS, 80)} format={(v) => hzText(LOW_STEPS[v])} onchange={(v) => setEq(e, { lowFreq: LOW_STEPS[v] })} />
            <FxKnob value={gainKnob(e.highGain)} max={GAIN_KNOB_MAX} centre tip="mixer.strip.eq_high_gain" label="{name} EQ high" caption="High" reset={gainKnob(0)} format={(v) => `${dbText(knobGain(v))} dB`} onchange={(v) => setEq(e, { highGain: knobGain(v) })} />
            <FxKnob value={stepOf(HIGH_STEPS, e.highFreq)} max={HIGH_STEPS.length - 1} tip="mixer.strip.eq_high_freq" label="{name} EQ high frequency" caption="High Hz" reset={stepOf(HIGH_STEPS, 10000)} format={(v) => hzText(HIGH_STEPS[v])} onchange={(v) => setEq(e, { highFreq: HIGH_STEPS[v] })} />
          </div>
        {/snippet}
        {@render eqKnobs(strip.eq)}
      </section>

      <section class="card comp" aria-label="Compressor">
        {#snippet compBody(c: PartCompState)}
        <header>
          <h3>Compressor</h3>
          {#if c.edited}<span class="edited" title="The parameters differ from the type's">edited</span>{/if}
        </header>
        <Toggle on={c.on} tip="mixer.strip.comp" onclick={() => app.send({ type: 'setStripCompressorOn', strip: part, on: !c.on })}>On</Toggle>
        <select
          class="pick"
          aria-label="{name} compressor type"
          value={c.preset}
          use:tip={'mixer.strip.comp_type'}
          onchange={(e) => app.send({ type: 'setStripCompressorPreset', strip: part, preset: e.currentTarget.value as CompPreset })}
        >
          {#each COMP_PRESETS as p (p.preset)}<option value={p.preset}>{p.name}</option>{/each}
        </select>
        <div class="knobs" class:off={!c.on}>
          {#each COMP_KNOBS as k (k.param)}
            <FxKnob
              value={c[k.param]}
              min={k.min}
              max={k.max}
              tip={k.tip}
              label="{name} compressor {k.caption}"
              caption={k.caption}
              reset={c[k.param]}
              format={k.format}
              onchange={(v) => app.send({ type: 'setStripCompressorParam', strip: part, param: k.param, value: v })}
            />
          {/each}
        </div>
        {/snippet}
        {@render compBody(strip.comp)}
      </section>

      {#each strip.inserts as slot, i (i)}
        <section class="card insert" aria-label={slotTitle(i)}>
          <h3>{slotTitle(i)}</h3>
          <Toggle on={slot.on} tip="mixer.strip.insert_on" onclick={() => app.send({ type: 'setStripInsertOn', strip: part, slot: i, on: !slot.on })}>On</Toggle>
          <select
            class="pick"
            aria-label="{name} {slotTitle(i).toLowerCase()} type"
            value={slot.kind}
            use:tip={'mixer.strip.insert_kind'}
            onchange={(e) => app.send({ type: 'setStripInsertKind', strip: part, slot: i, kind: e.currentTarget.value })}
          >
            {#each kindsFor(slot.kind, slot.name) as k (k.kind)}<option value={k.kind}>{k.name}</option>{/each}
          </select>
          {#if slot.settings.length > 0}
            <div class="knobs grid2" class:off={!slot.on}>
              {#each slot.settings as s, j (j)}
                <FxKnob
                  value={s.value}
                  min={s.min}
                  max={s.max}
                  tip={INSERT_SETTING_TIPS[j] ?? INSERT_SETTING_TIPS[3]}
                  label="{name} {slotTitle(i).toLowerCase()} {s.name}"
                  caption={s.name}
                  reset={s.default}
                  format={(v) => (v === s.value ? s.display : String(v))}
                  onchange={(v) => app.send({ type: 'setStripInsertSetting', strip: part, slot: i, setting: j, value: v })}
                />
              {/each}
            </div>
          {:else}
            <p class="none">Empty slot</p>
          {/if}
        </section>
      {/each}

      <section class="card" aria-label="Sends">
        <h3>Sends</h3>
        <div class="knobs grid2">
          {#each sends as s (s.send)}
            <FxKnob
              value={strip.sends[s.send] ?? 0}
              tip="mixer.strip.send"
              label="{name} send {s.send + 1} {s.name}"
              caption="{s.send + 1} {s.name}"
              reset={0}
              onchange={(v) => app.send({ type: 'setStripSend', strip: part, send: s.send, level: v })}
            />
          {/each}
        </div>
      </section>

      {#if kb}
        <section class="card play" aria-label="Play">
          <h3>Play</h3>
          <div class="stepper">
            <span class="engraved">Octave</span>
            <button type="button" class="mini mat-raised" aria-label="{name} octave down" aria-disabled={kb.octave <= -2} use:tip={'part.octave_down'} onclick={() => kb.octave > -2 && app.send({ type: 'setPartOctave', part, octave: kb.octave - 1 })}>−</button>
            <span class="val">{octaveLabel(kb.octave)}</span>
            <button type="button" class="mini mat-raised" aria-label="{name} octave up" aria-disabled={kb.octave >= 2} use:tip={'part.octave_up'} onclick={() => kb.octave < 2 && app.send({ type: 'setPartOctave', part, octave: kb.octave + 1 })}>+</button>
          </div>
          {#if bend !== null}
            <div class="stepper">
              <span class="engraved">Bend</span>
              <button type="button" class="mini mat-raised" aria-label="{name} bend range down" aria-disabled={bend <= 0} use:tip={'pedal.bend_down'} onclick={() => bend > 0 && app.send({ type: 'setBendRange', part, semitones: bend - 1 })}>−</button>
              <span class="val">{bend}</span>
              <button type="button" class="mini mat-raised" aria-label="{name} bend range up" aria-disabled={bend >= 12} use:tip={'pedal.bend_up'} onclick={() => bend < 12 && app.send({ type: 'setBendRange', part, semitones: bend + 1 })}>+</button>
            </div>
          {/if}
        </section>
      {/if}
    </div>
  </section>
{/if}

<style>
  .channel {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    height: 100%;
    min-height: 0;
    min-width: 0;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .title {
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
    flex: 1;
    min-width: 0;
  }
  .swatch {
    align-self: center;
    width: 0.35rem;
    height: 1.5rem;
    border-radius: 2px;
    background: var(--part, var(--accent));
    box-shadow: 0 0 6px var(--part, var(--accent));
  }
  h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 1.6rem;
    font-weight: 700;
    line-height: 1;
    color: var(--ink);
    white-space: nowrap;
  }
  .sub {
    font-family: var(--font-display);
    font-size: 0.85rem;
    color: var(--muted);
    white-space: nowrap;
  }
  .nav {
    min-width: 2.2rem;
    height: 2.2rem;
    border-radius: 5px;
    font-family: var(--font-display);
    font-size: 1.2rem;
    font-weight: 700;
    color: var(--ink);
  }
  .nav:focus-visible,
  .mini:focus-visible,
  .pick:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  /* The sections: a row of cards that scrolls sideways, never the page. */
  .sections {
    display: flex;
    gap: 0.6rem;
    flex: 1;
    min-height: 0;
    overflow-x: auto;
    overflow-y: hidden;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    flex: none;
    min-width: 9.5rem;
    padding: 0.7rem;
    border: 1px solid var(--seam);
    border-radius: var(--r-key);
    background: rgb(0 0 0 / 0.12);
    box-shadow: inset 0 1px 3px rgb(0 0 0 / 0.35);
    overflow-y: auto;
  }
  .card header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.5rem;
  }
  h3 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 1.1rem;
    font-weight: 700;
    line-height: 1;
    color: var(--ink);
    white-space: nowrap;
  }
  .edited {
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
  .level-row {
    display: flex;
    gap: 0.5rem;
    flex: 1;
    min-height: 10rem;
  }
  .fader {
    height: 100%;
    display: flex;
  }
  .meter {
    position: relative;
    width: 0.6rem;
    margin: 2rem 0 1.6rem;
    border-radius: 3px;
    overflow: hidden;
  }
  .meter-fill {
    position: absolute;
    inset: 0;
    transform-origin: bottom;
    background: linear-gradient(0deg, var(--accent) 0 70%, var(--danger) 100%);
    will-change: transform;
  }
  .knobs {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-around;
    gap: 0.5rem 0.3rem;
  }
  .knobs.grid2 {
    display: grid;
    grid-template-columns: repeat(2, minmax(3.8rem, 1fr));
  }
  .comp .knobs {
    display: grid;
    grid-template-columns: repeat(3, minmax(3.8rem, 1fr));
  }
  .knobs.off {
    opacity: 0.6;
  }
  .knobs > :global(*) {
    min-width: 3.5rem;
  }
  .knobs :global(.knob svg) {
    width: 2.7rem;
  }
  .knobs :global(.knob .caption) {
    font-size: 0.72rem;
  }
  .knobs :global(.knob .value) {
    font-size: 0.85rem;
  }
  .pick {
    min-height: 2rem;
    font: inherit;
    font-size: 0.9rem;
  }
  .none {
    margin: 0;
    font-size: var(--fs-small);
    color: var(--muted);
  }
  .stepper {
    display: grid;
    grid-template-columns: 4rem auto 2.4rem auto;
    align-items: center;
    gap: 0.4rem;
  }
  .mini {
    width: 2rem;
    height: 2rem;
    border-radius: 5px;
    font-family: var(--font-display);
    font-weight: 700;
    color: var(--ink);
  }
  .mini[aria-disabled='true'] {
    opacity: 0.45;
  }
  .val {
    text-align: center;
    font-family: var(--font-display);
    font-weight: 600;
    color: var(--ink);
  }
</style>
