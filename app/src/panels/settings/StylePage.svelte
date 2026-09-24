<!--
  Style: Genos Menu › Style Setting. How the band starts, stops and fills, and the
  settings M5 adds (shown, disabled, marked "coming soon").
-->
<script lang="ts">
  import { app } from '../../lib/store.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'
  import Choice from './Choice.svelte'
  import Field from './Field.svelte'
  import HSlider from './HSlider.svelte'

  const t = $derived(app.state.transport)
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

<Field name="Stop Accompaniment" genos="STOP ACMP" inline note="With the band stopped and Sync Start off, the chord you hold sounds on the style's bass and pad voices.">
  <Toggle on={t.stopAcmp} tip="transport.stop_acmp" onclick={() => app.send({ type: 'toggleStopAcmp' })}>{onOff(t.stopAcmp)}</Toggle>
</Field>

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

  <Field name="OTS Link timing" genos="OTS Link Timing" soon="M5">
    <Choice
      label="OTS Link timing"
      disabled
      value={null}
      options={[
        { id: 'immediate', label: 'Immediate', tip: 'settings.ots_link_timing' },
        { id: 'section', label: 'At Main Section Change', tip: 'settings.ots_link_timing' },
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
