<!--
  Style: Genos Menu › Style Setting. How the band starts, stops and fills, Stop
  Accompaniment, OTS Link timing, Change Behavior, and the settings M5 still adds (shown,
  disabled, marked "coming soon").
-->
<script lang="ts">
  import { app } from '../../lib/store.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'
  import ChangeBehavior from './ChangeBehavior.svelte'
  import Choice from './Choice.svelte'
  import Field from './Field.svelte'
  import FillButtons from './FillButtons.svelte'
  import HSlider from './HSlider.svelte'

  const t = $derived(app.state.transport)
  const ots = $derived(app.state.ots)
  const onOff = (on: boolean) => (on ? 'On' : 'Off')
</script>

<Field name="Sync Start" genos="SYNC START" inline note="Armed, the first chord you play starts the style.">
  <Toggle on={t.syncStart} tip="transport.sync_start" onclick={() => app.send({ type: 'toggleSyncStart' })}>{onOff(t.syncStart)}</Toggle>
</Field>

<Field
  name="Sync Stop"
  genos="SYNC STOP"
  inline
  note={t.syncStopAvailable ? 'Release every chord key and the style stops; play again and it restarts.' : 'Not available with the Full Keyboard fingering types in Lower.'}
>
  <span class="gate" class:off={!t.syncStopAvailable}>
    <Toggle on={t.syncStop} tip="transport.sync_stop" onclick={() => t.syncStopAvailable && app.send({ type: 'toggleSyncStop' })}>{onOff(t.syncStop)}</Toggle>
  </span>
</Field>

<Field name="Auto Fill" genos="AUTO FILL IN" inline note="Changing Main plays a fill first.">
  <Toggle on={t.autoFill} tip="transport.auto_fill" onclick={() => app.send({ type: 'toggleAutoFill' })}>{onOff(t.autoFill)}</Toggle>
</Field>

<Field name="Half Bar Fill" genos="Half Bar Fill In" inline note="A Main pressed on the first beat of a bar: a fill from the middle of that bar, then the Main.">
  <Toggle on={t.halfBarFill} tip="transport.half_bar_fill" onclick={() => app.send({ type: 'toggleHalfBarFill' })}>{onOff(t.halfBarFill)}</Toggle>
</Field>

<Field name="Fills" genos="Fill Down / Self / Up / Break" note="A fill, then the Main to the left or right; the Main's own fill; the Break.">
  <FillButtons />
</Field>

<Field name="Stop Accompaniment" genos="Stop ACMP" note="With the band stopped and Sync Start off, what the chord you hold sounds on.">
  <Choice
    label="Stop Accompaniment"
    value={t.stopAcmpMode}
    options={[
      { id: 'off', label: 'Off', tip: 'settings.stop_acmp_off' },
      { id: 'style', label: 'Style', tip: 'settings.stop_acmp_style' },
      { id: 'fixed', label: 'Fixed', tip: 'settings.stop_acmp_fixed' },
    ]}
    onselect={(mode) => app.send({ type: 'setStopAcmp', mode })}
  />
</Field>

<Field name="OTS Link timing" genos="OTS Link Timing" note="With OTS Link on and the band playing: swap your sounds as you press a Main, or when that Main starts.">
  <Choice
    label="OTS Link timing"
    value={ots.linkTiming}
    options={[
      { id: 'immediate', label: 'Immediate', tip: 'settings.ots_link_timing' },
      { id: 'mainChange', label: 'At Main Section Change', tip: 'settings.ots_link_timing' },
    ]}
    onselect={(timing) => app.send({ type: 'setOtsLinkTiming', timing })}
  />
</Field>

<ChangeBehavior />

<div class="soon-group">
  <Field name="Section change timing" genos="Section Change Timing" soon="M5">
    <Choice
      label="Section change timing"
      disabled
      value={null}
      options={[
        { id: 'immediate', label: 'Immediate', tip: 'settings.section_timing' },
        { id: 'bar', label: 'Next Bar', tip: 'settings.section_timing' },
      ]}
      onselect={() => {}}
    />
  </Field>

  <Field name="Synchro Stop window" genos="Synchro Stop Window" soon="M5">
    <HSlider label="Synchro Stop window" tip="settings.synchro_stop_window" value={0} disabled onchange={() => {}} />
  </Field>
</div>

<style>
  .gate.off {
    opacity: 0.5;
  }
  .soon-group {
    display: grid;
    gap: 1rem;
    padding-top: 1rem;
    border-top: 1px dashed var(--line);
  }
</style>
