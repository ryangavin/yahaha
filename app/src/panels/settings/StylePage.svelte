<!--
  Style: Genos Menu › Style Setting. How the band starts, stops and fills, Section Change
  Timing, the Synchro Stop Window, Tap's Section Reset, Fade In/Out and Style Retrigger;
  OTS Link Timing is still to come (shown, disabled, marked "coming soon").
-->
<script lang="ts">
  import { app } from '../../lib/store.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'
  import Choice from './Choice.svelte'
  import Field from './Field.svelte'
  import HSlider from './HSlider.svelte'
  import { RETRIGGER_RATES, type FadeState, type IntroEndingTiming, type MainTiming } from '../../lib/api/types'

  const t = $derived(app.state.transport)
  const st = $derived(app.state.styleSettings)
  const onOff = (on: boolean) => (on ? 'On' : 'Off')
  /** Settings in ms, sliders in tenths of a second. */
  const tenths = (ms: number) => Math.round(ms / 100)
  const seconds = (v: number) => `${(v / 10).toFixed(1)} s`
  const FADE_LABEL: Record<FadeState, string> = { off: 'Off', armed: 'Armed', fadingIn: 'Fading in', fadingOut: 'Fading out', holding: 'Holding' }
  const FADE_NOTE: Record<FadeState, string> = {
    off: 'Stopped: arms a fade in for the next start. Playing: fades out and stops.',
    armed: 'The next start fades in.',
    fadingIn: 'Coming up from silence.',
    fadingOut: 'The band stops when it reaches silence.',
    holding: 'Silent for the hold time.',
  }
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

<Field
  name="Section change timing: to Main"
  genos="Section Change Timing – To Main"
  note={st.mainTiming === 'immediate' && t.autoFill ? 'Auto Fill is on, so Main changes still wait for the next bar.' : 'Also when a style loads while the band plays.'}
>
  <Choice
    label="Section change timing to Main"
    value={st.mainTiming}
    options={[
      { id: 'nextBar', label: 'Next Bar', tip: 'settings.section_timing' },
      { id: 'immediate', label: 'Immediate', tip: 'settings.section_timing' },
    ]}
    onselect={(timing: MainTiming) => app.send({ type: 'setMainTiming', timing })}
  />
</Field>

<Field name="Section change timing: inside Intro/Ending" genos="Section Change Timing – Inside Intro/Ending">
  <Choice
    label="Section change timing inside Intro and Ending"
    value={st.introEndingTiming}
    options={[
      { id: 'nextBar', label: 'Next Bar', tip: 'settings.intro_ending_timing' },
      { id: 'endOfSection', label: 'End of Section', tip: 'settings.intro_ending_timing' },
    ]}
    onselect={(timing: IntroEndingTiming) => app.send({ type: 'setIntroEndingTiming', timing })}
  />
</Field>

<Field name="Synchro Stop window" genos="Synchro Stop Window" note="Hold a chord longer than this and Sync Stop turns itself off.">
  <HSlider
    label="Synchro Stop window"
    tip="settings.synchro_stop_window"
    value={tenths(st.syncStopWindowMs)}
    max={50}
    format={(v) => (v === 0 ? 'Off' : seconds(v))}
    onchange={(v) => app.send({ type: 'setSyncStopWindow', ms: v * 100 })}
  />
</Field>

<Field name="Tap: Section Reset" genos="Tap Tempo › Style Section Reset" inline note="On: Tap while the band plays restarts the section. Off: Tap sets the tempo.">
  <Toggle on={st.sectionReset} tip="settings.section_reset" onclick={() => app.send({ type: 'setSectionReset', on: !st.sectionReset })}>{onOff(st.sectionReset)}</Toggle>
</Field>

<Field name="Fade In/Out" genos="Fade In/Out" inline note={FADE_NOTE[t.fade]}>
  <Toggle on={t.fade !== 'off'} tip="transport.fade" onclick={() => app.send({ type: 'toggleFade' })}>{FADE_LABEL[t.fade]}</Toggle>
</Field>

<Field name="Fade in time" genos="Fade In Time">
  <HSlider label="Fade in time" tip="settings.fade_in" value={tenths(st.fadeInMs)} max={200} format={seconds} onchange={(v) => app.send({ type: 'setFadeInTime', ms: v * 100 })} />
</Field>

<Field name="Fade out time" genos="Fade Out Time">
  <HSlider label="Fade out time" tip="settings.fade_out" value={tenths(st.fadeOutMs)} max={200} format={seconds} onchange={(v) => app.send({ type: 'setFadeOutTime', ms: v * 100 })} />
</Field>

<Field name="Fade out hold time" genos="Fade Out Hold Time">
  <HSlider label="Fade out hold time" tip="settings.fade_hold" value={tenths(st.fadeHoldMs)} max={50} format={seconds} onchange={(v) => app.send({ type: 'setFadeHoldTime', ms: v * 100 })} />
</Field>

<Field name="Retrigger" genos="Style Retrigger" inline note="On: each chord you play restarts the Main and loops its head.">
  <Toggle on={t.retrigger} tip="transport.retrigger" onclick={() => app.send({ type: 'toggleRetrigger' })}>{onOff(t.retrigger)}</Toggle>
</Field>

<Field name="Retrigger length" genos="Style Retrigger Rate">
  <Choice
    label="Retrigger length"
    value={st.retriggerRate}
    options={RETRIGGER_RATES.map((r) => ({ id: r as number, label: r === 1 ? '1' : `1/${r}`, tip: 'settings.retrigger_rate' as const }))}
    onselect={(rate: number) => app.send({ type: 'setRetriggerRate', rate })}
  />
</Field>

<div class="soon-group">
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
