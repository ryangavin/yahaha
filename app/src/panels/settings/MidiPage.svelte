<!--
  MIDI: which sources play yahaha (all merged, or the ones you pick), the virtual
  "yahaha" output port, the Launchkey connection and its LED mode. Input selection and
  palette LEDs are mocked until the engine has `setMidiInputs` / `setPaletteLeds`.
-->
<script lang="ts">
  import { settings } from '../../lib/api/settings.svelte'
  import { app } from '../../lib/store.svelte'
  import { tip } from '../../lib/tooltip/tip.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'
  import Choice from './Choice.svelte'
  import Field from './Field.svelte'

  const io = $derived(app.state.io)
  const connected = $derived(app.state.pads.connected)
  const real = $derived(app.kind === 'tauri')
  const view = $derived(settings.view(app.state, real))
  // On the real engine a setting it lacks is badged and inert: it never pretends to work.
  const inputsInert = $derived(real && view.mocked.inputs)
  const ledsInert = $derived(real && view.mocked.paletteLeds)
  const inputsNote = $derived(
    inputsInert
      ? 'The inputs yahaha has open. Choosing them here needs an engine update; for now use --all-inputs or --input name at launch.'
      : view.allInputs
        ? 'Every source below plays yahaha, merged. Switch one off to pick sources instead.'
        : 'Only the sources switched on below play yahaha.',
  )

  /** Switching one source in "All" mode moves to "Selected" with every other source on. */
  function toggleSource(name: string) {
    if (inputsInert) return
    const on = view.sources.filter((s) => s.listening).map((s) => s.name)
    const names = on.includes(name) ? on.filter((n) => n !== name) : [...on, name]
    settings.send({ type: 'setMidiInputs', all: false, names })
  }
  function setMode(mode: 'all' | 'selected') {
    if (inputsInert) return
    const names = view.sources.filter((s) => s.listening).map((s) => s.name)
    settings.send({ type: 'setMidiInputs', all: mode === 'all', names })
  }
</script>

<Field
  name="Inputs"
  mock={inputsInert}
  note={inputsNote}
>
  <Choice
    label="MIDI inputs"
    disabled={inputsInert}
    value={view.allInputs === null ? null : view.allInputs ? 'all' : 'selected'}
    options={[
      { id: 'all', label: 'All, merged', tip: 'midi.merge_all' },
      { id: 'selected', label: 'Selected', tip: 'midi.merge_all' },
    ]}
    onselect={setMode}
  />
  <ul class="sources">
    {#each view.sources as s (s.name)}
      <li>
        <span class="src">
          <span class="sname">{s.name.replace(/ \(pads\)$/, '')}</span>
          {#if s.pads}<span class="tag engraved">pads + buttons</span>{/if}
        </span>
        <span class="gate" class:off={inputsInert}>
          <Toggle on={s.listening} tip="midi.input" onclick={() => toggleSource(s.name)}>
            {s.listening ? 'On' : 'Off'}
          </Toggle>
        </span>
      </li>
    {:else}
      <li class="none">No MIDI sources found.</li>
    {/each}
  </ul>
</Field>

<Field name="Output port" genos="MIDI Transmit" note="Pick it as a MIDI input in Ableton (or any app) to play yahaha on your own sounds.">
  <span class="port mat-screen" use:tip={'midi.output'}>
    <span class="glow-text">{io.outputPort || 'none'}</span>
    <span class="dim">{io.outputPort ? 'virtual MIDI port' : io.offline ? 'offline session' : 'not open'}</span>
  </span>
</Field>

<Field name="Launchkey" note={connected ? 'Its DAW port is open: pads, buttons and faders are arranger controls.' : 'Connect a Launchkey MK4 over USB; yahaha switches it to DAW mode.'}>
  <span class="lk" use:tip={'launchkey.status'}>
    <span class="lamp" class:on={connected} aria-hidden="true"></span>
    {connected ? 'Connected (DAW mode)' : 'Not connected'}
  </span>
</Field>

<Field
  name="Palette LEDs"
  inline
  mock={ledsInert}
  note={ledsInert
    ? 'Built-in palette colours and hardware flashing, instead of exact RGB. Switching it here needs an engine update; for now pass --palette-leds at launch.'
    : 'Built-in palette colours and hardware flashing, instead of exact RGB.'}
>
  <span class="gate" class:off={ledsInert}>
    <Toggle
      on={view.paletteLeds === true}
      tip="midi.palette_leds"
      onclick={() => !ledsInert && settings.send({ type: 'setPaletteLeds', on: !view.paletteLeds })}
    >
      {view.paletteLeds === null ? 'Set at launch' : view.paletteLeds ? 'On' : 'Off'}
    </Toggle>
  </span>
</Field>

<style>
  .gate.off {
    opacity: 0.5;
  }
  .sources {
    list-style: none;
    margin: 0.5rem 0 0;
    padding: 0;
    display: grid;
    gap: 0.35rem;
  }
  .sources li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.2rem 0 0.2rem 0.6rem;
    border-left: 2px solid var(--line);
  }
  .src {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.15rem 0.5rem;
    min-width: 0;
  }
  .sname {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tag {
    text-transform: uppercase;
  }
  .none {
    color: var(--muted);
    font-size: var(--fs-small);
  }
  .port {
    display: flex;
    align-items: baseline;
    gap: 0.7rem;
    padding: 0.45rem 0.8rem;
    border-radius: 5px;
    font-family: var(--font-display);
  }
  .port .glow-text {
    font-size: 1.2rem;
    font-weight: 700;
  }
  .dim {
    color: var(--screen-dim);
    font-size: 0.9rem;
  }
  .lk {
    display: inline-flex;
    align-items: center;
    gap: 0.55rem;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 1.02rem;
  }
  .lamp {
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
</style>
