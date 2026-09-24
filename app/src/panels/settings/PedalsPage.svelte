<!--
  Pedals and wheels: Genos Menu › Assignable (Foot Pedal) and Controller. Three pedals,
  each listening for a CC from the keyboards (the Launchkey's sustain jack is CC 64) and
  running an assignable function from the engine's table (assignable-functions.json);
  then, per keyboard part, which controllers reach it and its Pitch Bend Range.
  docs/controllers.md has the behaviour.
-->
<script lang="ts">
  import { functionGroups, functionInfo } from '../../lib/api/assignable'
  import { KEYBOARD_PART_NAMES, type BendRange, type ControlType, type PedalState } from '../../lib/api/types'
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import Choice from './Choice.svelte'
  import Field from './Field.svelte'

  const ctl = $derived(app.state.controllers)
  const groups = functionGroups()

  function setPedal(i: number, change: Partial<PedalState>) {
    const p = { ...ctl.pedals[i], ...change }
    app.send({ type: 'setPedal', pedal: i, cc: p.cc, function: p.function, controlType: p.controlType, reverse: p.reverse, range: p.range })
  }

  function ccInput(i: number, e: Event) {
    const v = (e.currentTarget as HTMLInputElement).value.trim()
    const n = Number(v)
    setPedal(i, { cc: v === '' || !Number.isInteger(n) ? null : Math.max(0, Math.min(127, n)) })
  }

  const learn = (i: number) => app.send({ type: 'learnPedal', pedal: ctl.learning === i ? null : i })

  const TYPES: { id: ControlType; label: string; tip: 'pedal.hold_a' | 'pedal.hold_b' | 'pedal.toggle' }[] = [
    { id: 'holdA', label: 'Hold A', tip: 'pedal.hold_a' },
    { id: 'holdB', label: 'Hold B', tip: 'pedal.hold_b' },
    { id: 'toggle', label: 'Toggle', tip: 'pedal.toggle' },
  ]
  const RANGES: { id: BendRange; label: string; tip: 'pedal.range_upper' | 'pedal.range_lower' | 'pedal.range_full' }[] = [
    { id: 'upper', label: 'Up', tip: 'pedal.range_upper' },
    { id: 'lower', label: 'Down', tip: 'pedal.range_lower' },
    { id: 'full', label: 'Both', tip: 'pedal.range_full' },
  ]

  const held = $derived(
    [ctl.sustain && 'Sustain', ctl.sostenuto && 'Sostenuto', ctl.soft && 'Soft'].filter(Boolean).join(' · ') || 'No pedal held',
  )

  function part(p: number, change: Partial<{ sustain: boolean; pitchBend: boolean; modulation: boolean }>) {
    const c = { ...ctl.parts[p], ...change }
    app.send({ type: 'setPartControllers', part: p, sustain: c.sustain, pitchBend: c.pitchBend, modulation: c.modulation })
  }
  const range = (p: number, d: number) =>
    app.send({ type: 'setBendRange', part: p, semitones: Math.max(0, Math.min(12, ctl.parts[p].bendRange + d)) })
</script>

<Field
  name="Pedals"
  genos="Assignable › Foot Pedal"
  note="Each pedal listens for one control change from your keyboard (the Launchkey's sustain jack sends CC 64) and runs one function."
>
  <p class="held mat-screen" aria-live="polite"><span class="glow-text" class:idle={!ctl.sustain && !ctl.sostenuto && !ctl.soft}>{held}</span></p>
  <ol class="pedals">
    {#each ctl.pedals as p, i (i)}
      {@const info = functionInfo(p.function)}
      <li class="pedal">
        <div class="row head">
          <span class="lamp" class:on={p.down} aria-hidden="true"></span>
          <span class="pname">Pedal {i + 1}</span>
          <label class="cc">
            <span class="engraved">CC</span>
            <input
              type="number"
              min="0"
              max="127"
              inputmode="numeric"
              placeholder="—"
              value={p.cc ?? ''}
              aria-label="Pedal {i + 1} CC"
              use:tip={'pedal.cc'}
              onchange={(e) => ccInput(i, e)}
            />
          </label>
          <button type="button" class="mini mat-raised" class:pressed={ctl.learning === i} aria-pressed={ctl.learning === i} use:tip={'pedal.learn'} onclick={() => learn(i)}>
            {ctl.learning === i ? 'Press the pedal…' : 'Learn'}
          </button>
        </div>
        <div class="row">
          <select
            class="fn"
            value={p.function}
            aria-label="Pedal {i + 1} function"
            use:tip={'pedal.function'}
            onchange={(e) => setPedal(i, { function: (e.currentTarget as HTMLSelectElement).value })}
          >
            {#each groups as g (g.category)}
              <optgroup label={g.name}>
                {#each g.functions as f (f.id)}
                  <option value={f.id} disabled={!f.available}>{f.name}{f.available ? '' : ' (later)'}</option>
                {/each}
              </optgroup>
            {/each}
          </select>
          <button
            type="button"
            class="mini mat-raised"
            disabled={p.function === 'none' || info?.kind === 'continuous' || info?.available === false}
            use:tip={'pedal.try'}
            onclick={() => app.send({ type: 'triggerFunction', function: p.function })}
          >
            Try
          </button>
        </div>
        {#if info?.kind === 'switch'}
          <Choice label="Pedal {i + 1} control type" options={TYPES} value={p.controlType} onselect={(controlType) => setPedal(i, { controlType })} />
        {:else if p.function === 'pitchBend'}
          <Choice label="Pedal {i + 1} bend range" options={RANGES} value={p.range} onselect={(range) => setPedal(i, { range })} />
        {/if}
        <label class="rev">
          <input type="checkbox" checked={p.reverse} use:tip={'pedal.reverse'} onchange={() => setPedal(i, { reverse: !p.reverse })} />
          Reverse polarity
        </label>
      </li>
    {/each}
  </ol>
</Field>

<Field
  name="Parts"
  genos="Controller › Pitch Bend Range"
  note="Which parts the sustain pedal, the pitch wheel and the modulation wheel reach. A part only takes them while it is on."
>
  <table class="parts">
    <thead>
      <tr><th></th><th class="engraved">Sustain</th><th class="engraved">Bend</th><th class="engraved">Mod</th><th class="engraved">Bend range</th></tr>
    </thead>
    <tbody>
      {#each ctl.parts as c, p (p)}
        <tr>
          <th scope="row">{KEYBOARD_PART_NAMES[p]}</th>
          <td>
            <button type="button" class="sw" role="switch" aria-checked={c.sustain} aria-label="{KEYBOARD_PART_NAMES[p]} sustain" use:tip={'pedal.part_sustain'} onclick={() => part(p, { sustain: !c.sustain })}>
              <span class="lamp" class:on={c.sustain} aria-hidden="true"></span>
            </button>
          </td>
          <td>
            <button type="button" class="sw" role="switch" aria-checked={c.pitchBend} aria-label="{KEYBOARD_PART_NAMES[p]} pitch bend" use:tip={'pedal.part_bend'} onclick={() => part(p, { pitchBend: !c.pitchBend })}>
              <span class="lamp" class:on={c.pitchBend} aria-hidden="true"></span>
            </button>
          </td>
          <td>
            <button type="button" class="sw" role="switch" aria-checked={c.modulation} aria-label="{KEYBOARD_PART_NAMES[p]} modulation" use:tip={'pedal.part_modulation'} onclick={() => part(p, { modulation: !c.modulation })}>
              <span class="lamp" class:on={c.modulation} aria-hidden="true"></span>
            </button>
          </td>
          <td class="range">
            <button type="button" class="mini mat-raised" aria-label="{KEYBOARD_PART_NAMES[p]} bend range down" aria-disabled={c.bendRange <= 0} use:tip={'pedal.bend_down'} onclick={() => range(p, -1)}>−</button>
            <span class="val">{c.bendRange}</span>
            <button type="button" class="mini mat-raised" aria-label="{KEYBOARD_PART_NAMES[p]} bend range up" aria-disabled={c.bendRange >= 12} use:tip={'pedal.bend_up'} onclick={() => range(p, 1)}>+</button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
</Field>

<style>
  .held {
    margin: 0 0 0.5rem;
    padding: 0.35rem 0.7rem;
    border-radius: 5px;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 1.02rem;
  }
  .held .idle {
    color: var(--screen-dim);
  }
  .pedals {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 0.7rem;
  }
  .pedal {
    display: grid;
    gap: 0.4rem;
    padding: 0.5rem 0.6rem;
    border: 1px solid var(--seam);
    border-radius: 7px;
    background: color-mix(in srgb, var(--well) 22%, transparent);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
  }
  .pname {
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 1.02rem;
    margin-right: auto;
  }
  .cc {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
  }
  .cc input {
    width: 3.6rem;
    min-height: 2.2rem;
    padding: 0 0.4rem;
    border: 1px solid var(--well-edge);
    border-radius: 4px;
    background: var(--screen-bg);
    color: var(--screen-ink);
    font-family: var(--font-display);
    font-size: 1rem;
    text-align: center;
  }
  .fn {
    flex: 1;
    min-width: 0;
    min-height: 2.2rem;
    padding: 0 0.5rem;
    border: 1px solid var(--well-edge);
    border-radius: 4px;
    background: var(--screen-bg);
    color: var(--screen-ink);
    font-family: var(--font-display);
    font-size: 1rem;
  }
  .mini {
    min-height: 2.2rem;
    min-width: 2.2rem;
    padding: 0 0.7rem;
    border-radius: 4px;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.95rem;
    color: var(--ink);
    white-space: nowrap;
  }
  .mini.pressed {
    box-shadow: inset 0 -2px 0 var(--accent);
    color: var(--accent);
  }
  .mini:disabled,
  .mini[aria-disabled='true'] {
    opacity: 0.45;
  }
  .rev {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    font-size: var(--fs-small);
    color: var(--muted);
  }
  .rev input {
    width: 1.1rem;
    height: 1.1rem;
    accent-color: var(--accent);
  }
  .lamp {
    display: inline-block;
    width: 0.65rem;
    height: 0.65rem;
    border-radius: 50%;
    background: var(--lamp-off);
    box-shadow: inset 0 1px 1px rgb(0 0 0 / 0.6);
  }
  .lamp.on {
    background: var(--accent);
    box-shadow: 0 0 8px var(--accent);
  }
  .parts {
    width: 100%;
    border-collapse: collapse;
  }
  .parts th,
  .parts td {
    padding: 0.15rem 0.2rem;
    text-align: center;
  }
  .parts th[scope='row'] {
    text-align: left;
    font-family: var(--font-display);
    font-weight: 600;
    white-space: nowrap;
  }
  .parts thead th {
    text-transform: uppercase;
    font-weight: 500;
  }
  .sw {
    display: inline-grid;
    place-items: center;
    width: 2.4rem;
    height: 2.2rem;
    border: 1px solid var(--seam);
    border-radius: 4px;
    background: var(--well);
  }
  .range {
    white-space: nowrap;
  }
  .range .mini {
    padding: 0;
  }
  .val {
    display: inline-block;
    min-width: 2ch;
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 1.05rem;
    font-variant-numeric: tabular-nums;
  }
</style>
