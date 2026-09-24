<!--
  Genos Style Setting › Change Behavior: what choosing another style does to the tempo,
  the style parts you muted, and the Main.
-->
<script lang="ts">
  import type { ChangeRule } from '../../lib/api/types'
  import type { TipKey } from '../../help/tooltips'
  import { app } from '../../lib/store.svelte'
  import Choice from './Choice.svelte'
  import Field from './Field.svelte'

  const sc = $derived(app.state.styleChange)
  const rules = (tip: TipKey): { id: ChangeRule; label: string; tip: TipKey }[] => [
    { id: 'lock', label: 'Lock', tip },
    { id: 'hold', label: 'Hold', tip },
    { id: 'reset', label: 'Reset', tip },
  ]
  const OFF = -1
  const sections = [
    { id: OFF, label: 'Off', tip: 'settings.section_set' as TipKey },
    ...['A', 'B', 'C', 'D'].map((l, i) => ({ id: i, label: l, tip: 'settings.section_set' as TipKey })),
  ]
</script>

<div class="group">
  <h4 class="engraved">When you change style</h4>
  <Field name="Tempo" genos="Change Behavior: Tempo" note="Lock keeps it; Hold keeps it while playing; Reset takes the new style's.">
    <Choice label="Tempo on style change" value={sc.tempo} options={rules('settings.tempo_change')} onselect={(rule) => app.send({ type: 'setTempoChange', rule })} />
  </Field>
  <Field name="Part on/off" genos="Change Behavior: Part On/Off" note="Whether the style parts you muted stay muted.">
    <Choice label="Part on/off on style change" value={sc.parts} options={rules('settings.parts_change')} onselect={(rule) => app.send({ type: 'setPartsChange', rule })} />
  </Field>
  <Field name="Section" genos="Change Behavior: Section Set" note="The Main a style chosen while stopped starts on.">
    <Choice
      label="Section on style change"
      value={sc.sectionSet ?? OFF}
      options={sections}
      onselect={(id) => app.send({ type: 'setSectionSet', section: id === OFF ? null : id })}
    />
  </Field>
</div>

<style>
  .group {
    display: grid;
    gap: 1rem;
    padding-top: 1rem;
    border-top: 1px solid var(--line);
  }
  h4 {
    margin: 0;
    font-size: 0.8rem;
  }
</style>
