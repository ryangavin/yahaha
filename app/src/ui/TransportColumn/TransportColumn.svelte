<!--
  TransportColumn: the band's 88px right column. Transport: Start / Stop (the same control as pad
  16, with its green running bar), Stop, Reset, Fade (full rows, as Round 2 draws them), and the
  Fills pair. Tempo: the + and − pair under the Tempo header (repeat while held) and Style tempo,
  so the column fits the band's 368px. Holds no state and no timers: the parent
  repeats Tempo ± between the two `onhold` calls.
-->
<script lang="ts">
  import type { Action } from 'svelte/action'
  import Button from '../Button/Button.svelte'
  import GroupHeader from '../GroupHeader/GroupHeader.svelte'

  type Props = {
    /** The band is running: Start / Stop's green bar and aria-pressed. */
    running?: boolean
    /** A fade in or out is under way: Fade's lamp face. */
    fading?: boolean
    /** The app's `use:tip` action, passed to every button. */
    tipAction?: Action<HTMLElement, string>
    /** Start / Stop pressed. */
    onstartstop?: () => void
    /** Stop pressed. */
    onstop?: () => void
    /** Stop held: stop with a fade. */
    onstoplong?: () => void
    /** Section reset pressed. */
    onreset?: () => void
    /** Fade pressed. */
    onfade?: () => void
    /** Fill ▲ pressed. */
    onfillup?: () => void
    /** Fill ▼ pressed. */
    onfilldown?: () => void
    /** Tempo + held (`true`) and released (`false`); from the keyboard, a press calls with `true` then `false`. */
    ontempoup?: (down: boolean) => void
    /** Tempo − held and released, as `ontempoup`. */
    ontempodown?: (down: boolean) => void
    /** Style tempo pressed: back to the style's own tempo. */
    onstyletempo?: () => void
  }

  let {
    running = false,
    fading = false,
    tipAction,
    onstartstop,
    onstop,
    onstoplong,
    onreset,
    onfade,
    onfillup,
    onfilldown,
    ontempoup,
    ontempodown,
    onstyletempo,
  }: Props = $props()

  /** A keyboard press on a hold button: one step, as a hold that ends at once. */
  const tap = (fn?: (down: boolean) => void) => () => {
    fn?.(true)
    fn?.(false)
  }
</script>

<section class="column" aria-label="Transport and tempo">
  <GroupHeader title="Transport" />
  <div class="buttons">
    <Button
      label="Start / Stop"
      size="band"
      compact
      strong
      bar={running}
      pressed={running}
      name={running ? 'Start / Stop, running (Play). The same control as pad 16' : 'Start / Stop, stopped (Play). The same control as pad 16'}
      {tipAction}
      onpress={onstartstop}
    />
    <Button label="Stop" size="band" name="Stop (fade with hold)" tip="transport.stop" {tipAction} onpress={onstop} onlongpress={onstoplong} />
    <Button label="Reset" size="band" name="Section reset: restart the section from its first bar" {tipAction} onpress={onreset} />
    <Button label="Fade" size="band" on={fading} pressed={fading} name="Fade in/out" tip="transport.fade" {tipAction} onpress={onfade} />
    <div class="pair" role="group" aria-label="Fills">
      <Button label="Fill" symbol="up" size="pair" name="Fill Up: a fill, then the next Main up (at Main D, its own fill)" {tipAction} onpress={onfillup} />
      <Button label="Fill" symbol="down" size="pair" name="Fill Down: a fill, then the next Main down (at Main A, its own fill)" {tipAction} onpress={onfilldown} />
    </div>
  </div>

  <div class="tempo">
    <GroupHeader title="Tempo" />
  </div>
  <div class="buttons">
    <div class="pair" role="group" aria-label="Tempo up and down">
      <Button label="" symbol="plus" size="pair" hold name="Tempo up (Scene Launch)" tip="tempo.up" {tipAction} onhold={ontempoup} onpress={tap(ontempoup)} />
      <Button label="" symbol="minus" size="pair" hold name="Tempo down (Function)" tip="tempo.down" {tipAction} onhold={ontempodown} onpress={tap(ontempodown)} />
    </div>
    <Button
      label="Style tempo"
      size="band"
      compact
      name="Style tempo: back to the tempo the style came with (Scene Launch and Function together)"
      tip="tempo.reset"
      {tipAction}
      onpress={onstyletempo}
    />
  </div>
</section>

<style>
  .column {
    display: flex;
    flex-direction: column;
    width: var(--band-transport-width);
    font-family: var(--font-sans);
  }
  .buttons {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    margin-top: var(--band-body-gap);
  }
  .pair {
    display: grid;
    grid-template-columns: var(--button-pair-width) var(--button-pair-width);
    gap: var(--space-6);
  }
  .tempo {
    margin-top: var(--band-tempo-gap);
  }
</style>
