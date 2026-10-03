<!--
  SectionRow: the toolbar between the app bar and the display. Accomp at the left, the count row
  in the middle, the helpers at the right: Metronome joined with its ▾ settings caret, Unison,
  Panic and help mode's ?. Every switch is controlled: a press only calls back.
-->
<script lang="ts">
  import type { ComponentProps } from 'svelte'
  import type { Action } from 'svelte/action'
  import Button from '../Button/Button.svelte'
  import CountRow from '../CountRow/CountRow.svelte'
  import LampButton from '../LampButton/LampButton.svelte'

  type Props = {
    /** The count row's beat, bar and sections. */
    count: ComponentProps<typeof CountRow>
    /** Accompaniment (ACMP) on. */
    accomp?: boolean
    /** The metronome on. */
    metronome?: boolean
    /** The metronome's settings popover is open (the caret's ▾ turns `--t`). */
    metronomeOpen?: boolean
    /** The id of the metronome's settings popover, for `aria-controls`. */
    metronomeControls?: string
    /** Unison on. */
    unison?: boolean
    /** Help mode on: the ? is lit. */
    help?: boolean
    /** The app's tooltip action (`use:tip`), applied to every control. */
    tipAction?: Action<HTMLElement, string>
    /** Called with the state asked for when Accomp is pressed. */
    onaccomp?: (on: boolean) => void
    /** Called with the state asked for when Metronome is pressed. */
    onmetronome?: (on: boolean) => void
    /** Called when the metronome's ▾ caret is pressed (opens or closes its settings). */
    onmetronomesettings?: () => void
    /** Called with the state asked for when Unison is pressed. */
    onunison?: (on: boolean) => void
    /** Called when Panic is pressed (all notes off). */
    onpanic?: () => void
    /** Called with the state asked for when ? is pressed. */
    onhelp?: (on: boolean) => void
  }

  let {
    count,
    accomp = false,
    metronome = false,
    metronomeOpen = false,
    metronomeControls,
    unison = false,
    help = false,
    tipAction,
    onaccomp,
    onmetronome,
    onmetronomesettings,
    onunison,
    onpanic,
    onhelp,
  }: Props = $props()
</script>

<div class="row" role="toolbar" aria-label="Switches and helpers">
  <LampButton label="Accomp" code="ACMP" on={accomp} tip="transport.acmp" {tipAction} ontoggle={onaccomp} />
  <CountRow {...count} />
  <span class="helpers">
    <span class="joined" role="group" aria-label="Metronome">
      <LampButton label="Metronome" on={metronome} join="start" tip="metronome.on" {tipAction} ontoggle={onmetronome} />
      <Button
        symbol="caret"
        size="caret"
        join="end"
        popup="dialog"
        expanded={metronomeOpen}
        controls={metronomeControls}
        name="Metronome settings: on/off, volume, bell on beat 1"
        {tipAction}
        onpress={onmetronomesettings}
      />
    </span>
    <LampButton label="Unison" on={unison} tip="transport.unison" {tipAction} ontoggle={onunison} />
    <Button label="Panic" name="Panic: all notes off" tip="transport.panic" {tipAction} onpress={onpanic} />
    <Button
      label="?"
      size="icon"
      on={help}
      pressed={help}
      name="Help mode: point at any control to learn what it does"
      {tipAction}
      onpress={() => onhelp?.(!help)}
    />
  </span>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: var(--space-24);
    width: var(--stage-row-width);
    height: var(--control-height);
    white-space: nowrap;
  }
  .helpers {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: var(--space-8);
  }
  .joined {
    display: flex;
    gap: var(--line-width);
  }
</style>
