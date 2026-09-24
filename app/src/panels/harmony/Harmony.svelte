<!--
  The Harmony/Arpeggio drawer (right side, not modal: performance keys keep working).
  The Genos has one HARMONY/ARPEGGIO switch and one type: a Keyboard Harmony type or an
  arpeggio, never both. Here: the switch and the selected type up top, the type lists
  (Harmony types in Data List order, yahaha's arpeggio patterns by category), and the
  settings the selected type has (RM p.46-47; Multi Assign has none).

  State: `app.state.harmonyArp`; lists: `app.library.harmonyTypes` / `arpPatterns`.
  Commands: docs/app-api.md "Keyboard Harmony / Arpeggio". Every change applies at once.
-->
<script lang="ts">
  import type { ArpQuantize, ArpVelocityMode, HarmonyAssign, HarmonySpeed, HarmonyTypeInfo } from '../../lib/api/types'
  import { app, ui } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'
  import Overlay from '../../lib/ui/Overlay.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'
  import Choice from '../settings/Choice.svelte'
  import Field from '../settings/Field.svelte'
  import HSlider from '../settings/HSlider.svelte'

  const h = $derived(app.state.harmonyArp)
  const arp = $derived(h.mode === 'arpeggio')
  const types = $derived(app.library.harmonyTypes ?? [])
  const patterns = $derived(app.library.arpPatterns ?? [])
  const selected = $derived(arp ? h.arpPattern : h.harmonyType)
  const kind = $derived(arp ? 'arpeggio' : h.typeName === 'Multi Assign' ? 'multi' : h.category === 'Echo' ? 'echo' : 'harmony')

  /** The list shown, grouped by category in list order. */
  const groups = $derived.by(() => {
    const out: { category: string; items: { i: number; t: HarmonyTypeInfo }[] }[] = []
    ;(arp ? patterns : types).forEach((t, i) => {
      const last = out[out.length - 1]
      if (last && last.category === t.category) last.items.push({ i, t })
      else out.push({ category: t.category, items: [{ i, t }] })
    })
    return out
  })

  const pick = (i: number) => app.send(arp ? { type: 'setArpPattern', index: i } : { type: 'setHarmonyType', index: i })
  const mode = (m: 'harmony' | 'arpeggio') =>
    app.send(m === 'arpeggio' ? { type: 'setArpPattern', index: h.arpPattern } : { type: 'setHarmonyType', index: h.harmonyType })

  const SPEEDS: HarmonySpeed[] = ['1/4', '1/6', '1/8', '1/12', '1/16', '1/32']
  const ASSIGNS: { id: HarmonyAssign; label: string }[] = [
    { id: 'auto', label: 'Auto' },
    { id: 'multi', label: 'Multi' },
    { id: 'right1', label: 'Right 1' },
    { id: 'right2', label: 'Right 2' },
    { id: 'right3', label: 'Right 3' },
  ]
  const QUANTIZE: { id: ArpQuantize; label: string }[] = [
    { id: 'off', label: 'Off' },
    { id: 'eighth', label: '1/8' },
    { id: 'sixteenth', label: '1/16' },
  ]
  const VELOCITY: { id: ArpVelocityMode; label: string }[] = [
    { id: 'original', label: 'Pattern' },
    { id: 'thru', label: 'As played' },
    { id: 'fixed', label: 'Fixed' },
  ]
  const onOff = (on: boolean) => (on ? 'On' : 'Off')
</script>

<Overlay id="harmony" title="Harmony / Arpeggio" closeTip="harmony.close" onclose={() => (ui.harmony = false)}>
  <div class="harmony">
    <div class="head">
      <Toggle on={h.on} tip="harmony.switch" onclick={() => app.send({ type: 'toggleHarmonyArp' })}>{onOff(h.on)}</Toggle>
      <div class="now mat-screen" aria-live="polite">
        <span class="name glow-text">{h.typeName}</span>
        <span class="cat">{h.category}</span>
      </div>
      <HwButton tip="harmony.prev_type" label="Previous type" onclick={() => app.send({ type: 'stepHarmonyArpType', delta: -1 })}>◀</HwButton>
      <HwButton tip="harmony.next_type" label="Next type" onclick={() => app.send({ type: 'stepHarmonyArpType', delta: 1 })}>▶</HwButton>
    </div>

    <Choice
      label="Type list"
      value={h.mode}
      options={[
        { id: 'harmony', label: 'Harmony', tip: 'harmony.mode_harmony' },
        { id: 'arpeggio', label: 'Arpeggio', tip: 'harmony.mode_arpeggio' },
      ]}
      onselect={mode}
    />

    <div class="lists" role="listbox" aria-label={arp ? 'Arpeggio patterns' : 'Harmony types'}>
      {#each groups as g (g.category)}
        <div class="group">
          <span class="engraved">{g.category}</span>
          <div class="grid">
            {#each g.items as { i, t } (i)}
              <button
                type="button"
                role="option"
                class="type"
                class:mat-raised={i === selected}
                class:on={i === selected}
                aria-selected={i === selected}
                use:tip={arp ? 'harmony.pattern' : 'harmony.type'}
                onclick={() => pick(i)}
              >
                {t.name}
              </button>
            {/each}
          </div>
        </div>
      {/each}
    </div>

    <div class="details">
      {#if kind === 'multi'}
        <p class="note">Multi Assign sends each right-hand key to Right 1, 2 and 3 in turn. It has no settings.</p>
      {:else}
        <Field name="Volume" genos="Volume">
          <HSlider label="Volume" tip="harmony.volume" value={h.volume} onchange={(v) => app.send({ type: 'setHarmonyVolume', volume: v })} />
        </Field>
        {#if kind === 'echo'}
          <Field name="Speed" genos="Speed">
            <Choice
              label="Speed"
              value={h.speed}
              options={SPEEDS.map((s) => ({ id: s, label: s, tip: 'harmony.speed' as const }))}
              onselect={(s) => app.send({ type: 'setHarmonySpeed', speed: s })}
            />
          </Field>
        {/if}
        <Field name="Assign" genos="Assign">
          <Choice
            label="Assign"
            value={arp && h.assign === 'multi' ? 'auto' : h.assign}
            options={ASSIGNS.filter((a) => !(arp && a.id === 'multi')).map((a) => ({ ...a, tip: 'harmony.assign' as const }))}
            onselect={(a) => app.send({ type: 'setHarmonyAssign', assign: a })}
          />
        </Field>
        {#if kind === 'harmony'}
          <Field name="Chord Note Only" inline note="Only melody notes of the chord get a harmony.">
            <Toggle on={h.chordNoteOnly} tip="harmony.chord_note_only" onclick={() => app.send({ type: 'setChordNoteOnly', on: !h.chordNoteOnly })}>
              {onOff(h.chordNoteOnly)}
            </Toggle>
          </Field>
        {/if}
        {#if kind !== 'arpeggio'}
          <Field name="Touch Limit" genos="Minimum Velocity" note="The effect sounds for keys played at least this hard (1: every key).">
            <HSlider label="Touch Limit" tip="harmony.touch_limit" value={h.touchLimit} onchange={(v) => app.send({ type: 'setTouchLimit', velocity: Math.max(1, v) })} />
          </Field>
        {:else}
          <Field name="Quantize" genos="Arpeggio Quantize">
            <Choice
              label="Arpeggio Quantize"
              value={h.arp.quantize}
              options={QUANTIZE.map((q) => ({ ...q, tip: 'harmony.arp_quantize' as const }))}
              onselect={(q) => app.send({ type: 'setArpQuantize', quantize: q })}
            />
          </Field>
          <Field
            name="Hold"
            genos="Arpeggio Hold"
            inline
            note={h.arp.pedalHold ? 'The Arpeggio Hold pedal is holding the pattern too.' : 'The pattern plays on after you let go.'}
          >
            <Toggle on={h.arp.hold} tip="harmony.arp_hold" onclick={() => app.send({ type: 'toggleArpHold' })}>{onOff(h.arp.hold)}</Toggle>
          </Field>
          <Field name="Velocity">
            <Choice
              label="Arpeggio velocity"
              value={h.arp.velocity}
              options={VELOCITY.map((v) => ({ ...v, tip: 'harmony.arp_velocity' as const }))}
              onselect={(m) => app.send({ type: 'setArpVelocity', mode: m, velocity: h.arp.fixedVelocity })}
            />
          </Field>
          {#if h.arp.velocity === 'fixed'}
            <Field name="Fixed velocity">
              <HSlider
                label="Fixed velocity"
                tip="harmony.arp_fixed_velocity"
                value={h.arp.fixedVelocity}
                onchange={(v) => app.send({ type: 'setArpVelocity', mode: 'fixed', velocity: Math.max(1, v) })}
              />
            </Field>
          {/if}
          <Field name="Keep Key On" inline note="The pattern's clock runs on while no key is held.">
            <Toggle on={h.arp.keepKeyOn} tip="harmony.arp_keep_key_on" onclick={() => app.send({ type: 'setArpKeepKeyOn', on: !h.arp.keepKeyOn })}>
              {onOff(h.arp.keepKeyOn)}
            </Toggle>
          </Field>
        {/if}
      {/if}
    </div>
  </div>
</Overlay>

<style>
  .harmony {
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .now {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
    min-height: 2.2rem;
    padding: 0.2rem 0.7rem;
    border-radius: 5px;
  }
  .name {
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 1.15rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .cat {
    font-size: var(--fs-small);
    color: var(--screen-dim);
    white-space: nowrap;
  }
  .lists {
    display: grid;
    gap: 0.55rem;
  }
  .group {
    display: grid;
    gap: 0.3rem;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 3px;
  }
  .type {
    min-height: 2.2rem;
    padding: 0.2rem 0.4rem;
    border: 1px solid var(--seam);
    border-radius: 4px;
    background: var(--well);
    color: var(--screen-dim);
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.92rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .type:not(.on):hover {
    color: var(--screen-ink);
  }
  .type.on {
    background: linear-gradient(180deg, var(--raised-hi), var(--raised-lo));
    color: var(--ink);
    box-shadow:
      inset 0 1px 0 rgb(255 255 255 / 0.18),
      inset 0 -2px 0 var(--accent),
      0 1px 0 rgb(0 0 0 / 0.45);
  }
  .details {
    display: grid;
    gap: 1rem;
    padding-top: 0.8rem;
    border-top: 1px solid var(--line);
  }
  /* A slider cap's carrier reaches past its track at high values: keep it from giving
     the drawer a phantom horizontal scroll. */
  .details :global(.track) {
    overflow: clip;
  }
  .note {
    margin: 0;
    font-size: var(--fs-small);
    color: var(--muted);
  }
</style>
