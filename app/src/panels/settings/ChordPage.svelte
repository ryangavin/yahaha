<!--
  Chord: Genos Menu › Split & Fingering. The fingering type (each explained in its
  tooltip, Single Finger's shapes drawn), the chord detection area and Manual Bass.
-->
<script lang="ts">
  import type { TipKey } from '../../help/tooltips'
  import { CHORD_SETTLE_MAX_MS, FINGERINGS, type Fingering } from '../../lib/api/types'
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'
  import Choice from './Choice.svelte'
  import Field from './Field.svelte'
  import HSlider from './HSlider.svelte'
  import SingleFingerChart from './SingleFingerChart.svelte'

  const chord = $derived(app.state.chord)

  const ABOUT: Record<Fingering, { tip: TipKey; line: string }> = {
    singleFinger: { tip: 'fingering.single_finger', line: 'One key; keys to its left add m, 7, m7' },
    fingered: { tip: 'fingering.fingered', line: 'Play the whole chord; the root is the bass' },
    fingeredOnBass: { tip: 'fingering.fingered_on_bass', line: 'Your lowest note is the bass: slash chords' },
    multiFinger: { tip: 'fingering.multi_finger', line: 'Single Finger or Fingered shapes, either one' },
    aiFingered: { tip: 'fingering.ai_fingered', line: 'Fewer than three keys still make a chord' },
    fullKeyboard: { tip: 'fingering.full_keyboard', line: 'Chords read across the whole keyboard' },
    aiFullKeyboard: { tip: 'fingering.ai_full_keyboard', line: 'Full Keyboard, guessing from fewer keys' },
  }

  const manualBassNote = $derived(
    !chord.upper
      ? 'Only in Upper: switch the detection area to Upper to use it.'
      : chord.manualBassActive
        ? 'On: the style\'s Bass is muted and your left hand plays its voice on Left.'
        : 'Off: the style\'s Bass plays the chords\' bass line.',
  )
</script>

<Field
  name="Fingering type"
  genos="Fingering Type"
  note={chord.upper ? `Upper reads your right hand as ${chord.fingeringName}. The type below applies in Lower.` : null}
>
  <div class="fingerings" role="radiogroup" aria-label="Fingering type">
    {#each FINGERINGS as f (f.id)}
      <button
        type="button"
        role="radio"
        class="fing"
        class:on={chord.fingering === f.id}
        class:mat-raised={chord.fingering === f.id}
        aria-checked={chord.fingering === f.id}
        use:tip={ABOUT[f.id].tip}
        onclick={() => chord.fingering !== f.id && app.send({ type: 'setFingering', fingering: f.id })}
      >
        <span class="led" aria-hidden="true"></span>
        <span class="fname">{f.name}</span>
        <span class="fline">{ABOUT[f.id].line}</span>
      </button>
    {/each}
  </div>
</Field>

<SingleFingerChart />

<Field
  name="Chord detection area"
  genos="Chord Detection Area"
  note={chord.upper ? 'Your right hand plays the chords; your left hand is free.' : 'Your left hand plays the chords, at and below the split.'}
>
  <Choice
    label="Chord detection area"
    value={chord.upper ? 'upper' : 'lower'}
    options={[
      { id: 'lower', label: 'Lower', tip: 'detection.upper' },
      { id: 'upper', label: 'Upper', tip: 'detection.upper' },
    ]}
    onselect={(id) => app.send({ type: 'setUpper', on: id === 'upper' })}
  />
</Field>

<Field name="Manual Bass" genos="Manual Bass" inline note={manualBassNote}>
  <span class="gate" class:off={!chord.upper}>
    <Toggle
      on={chord.upper && chord.manualBass}
      tip="detection.manual_bass"
      onclick={() => chord.upper && app.send({ type: 'setManualBass', on: !chord.manualBass })}
    >
      {chord.upper ? (chord.manualBass ? 'On' : 'Off') : 'Upper only'}
    </Toggle>
  </span>
</Field>

<Field
  name="Chord settle"
  note={chord.settleMs === 0
    ? 'Off: the style follows every chord change at once, even the passing chords of a roll.'
    : `${chord.settleMs} ms: a rolled chord is followed once. Chord parts wait at most ${chord.settleMs} ms (${3 * chord.settleMs} ms through a long roll) when you change chord on their beat.`}
>
  <HSlider
    label="Chord settle"
    tip="settings.chord_settle"
    value={chord.settleMs}
    max={CHORD_SETTLE_MAX_MS}
    onchange={(ms) => app.send({ type: 'setChordSettle', ms })}
  />
</Field>

<style>
  .fingerings {
    display: grid;
    gap: 3px;
    padding: 3px;
    border-radius: 6px;
    background: linear-gradient(180deg, var(--well-edge), var(--well) 30%);
    box-shadow: inset 0 2px 4px rgb(0 0 0 / 0.6);
  }
  .fing {
    display: grid;
    grid-template-columns: auto 8.2rem 1fr;
    align-items: center;
    gap: 0.6rem;
    min-height: 2.2rem;
    padding: 0.2rem 0.65rem;
    border: 1px solid transparent;
    border-radius: 4px;
    background: transparent;
    color: var(--screen-ink);
    text-align: left;
  }
  .fing:hover:not(.on) {
    background: rgb(255 255 255 / 0.05);
  }
  .fing.on {
    background: linear-gradient(180deg, var(--raised-hi), var(--raised-lo));
    color: var(--ink);
  }
  .led {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
    /* An unlit lamp in the dark well, in both themes. */
    background: #2a2e34;
    box-shadow: inset 0 1px 1px rgb(0 0 0 / 0.6);
  }
  .on .fline {
    color: var(--muted);
  }
  .on .led {
    background: var(--accent);
    box-shadow: 0 0 8px var(--accent);
  }
  .fname {
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 1rem;
    white-space: nowrap;
  }
  .fline {
    font-size: var(--fs-small);
    color: var(--screen-dim);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .gate.off {
    opacity: 0.5;
  }
  @media (max-width: 520px) {
    .fing {
      grid-template-columns: auto 1fr;
    }
    .fline {
      display: none;
    }
  }
</style>
